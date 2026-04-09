# KVMini 全面重构需求文档

## 引言

KVMini 是一款跨平台 KVM（键鼠共享）工具，允许 Windows 和 macOS 设备共享同一套键盘和鼠标。

### 当前架构问题

项目目前存在 **4 套后端代码并存**的混乱状态：

| 项目 | 类型 | 问题 |
|------|------|------|
| `kvm-core` | 核心库 | ✅ 正确，包含完整的网络/协议/状态机，应保留 |
| `kvm-win` | 命令行程序（无 GUI） | ⚠️ 接入了 kvm-core，但没有 GUI，用户无法使用 |
| `kvm-mac` | 命令行程序（无 GUI） | ⚠️ 接入了 kvm-core，但没有 GUI，用户无法使用 |
| `src-tauri` | Tauri GUI（仅 Windows） | ❌ 自己实现了一套 `connection/transport.rs`，完全没有接入 `kvm-core`；Mac 端没有 GUI |

**核心问题：**
- `src-tauri` 自己写了一套 `connection/transport.rs`（使用 `std::thread` + 同步 IO），而 `kvm-core` 已有完整的异步网络实现（`tokio` + `ConnectionManager`），两套代码完全割裂
- `kvm-win` 和 `kvm-mac` 有完整的平台适配器（输入采集/注入/快捷键/屏幕/剪贴板），但 `src-tauri` 没有使用它们
- `src-tauri` 版本号 `0.1.5` 与 workspace 版本 `0.2.0` 不一致，且 `DEFAULT_PEER_IP` 硬编码为 `30.19.140.18`
- Mac 端没有 GUI 应用，用户只能用命令行

### 目标架构

```
kvm-core/          ← 唯一核心库（保留，零平台依赖）
src-tauri/         ← 唯一 GUI 应用（Windows + macOS 双端）
                      ├── 接入 kvm-core（删掉自己的 connection/transport.rs）
                      ├── platform/windows.rs（从 kvm-win 迁移平台适配器）
                      └── platform/macos.rs（从 kvm-mac 迁移平台适配器）

kvm-win/           ← 删除（平台代码迁移到 src-tauri/platform/windows.rs）
kvm-mac/           ← 删除（平台代码迁移到 src-tauri/platform/macos.rs）
```

---

## 需求

### 需求 1：删除冗余项目，统一项目结构

**用户故事：** 作为开发者，我希望项目只有 `kvm-core`（核心库）和 `src-tauri`（GUI 应用）两个 Rust 项目，以便消除重复代码、降低维护成本。

#### 验收标准

1. WHEN 重构完成 THEN `kvm-win/` 目录 SHALL 被完全删除
2. WHEN 重构完成 THEN `kvm-mac/` 目录 SHALL 被完全删除
3. WHEN 查看根目录 `Cargo.toml` THEN `[workspace] members` SHALL 只包含 `["kvm-core", "src-tauri"]`
4. WHEN 查看 `src-tauri/src/connection/` THEN 自己实现的 `transport.rs`、`error.rs` SHALL 被删除，改为使用 `kvm-core::network`
5. WHEN 查看 `src-tauri/src/` THEN 不得存在与 `kvm-core` 重复的网络/协议/状态机实现

---

### 需求 2：`src-tauri` 接入 `kvm-core`，删除自造轮子

**用户故事：** 作为开发者，我希望 `src-tauri` 的网络功能完全来自 `kvm-core`，以便两端使用同一套协议实现，消除不一致风险。

#### 验收标准

1. WHEN 查看 `src-tauri/Cargo.toml` THEN `[dependencies]` SHALL 包含 `kvm-core = { path = "../kvm-core" }`
2. WHEN `src-tauri` 建立连接 THEN SHALL 使用 `kvm_core::network::ConnectionManager`，不得使用自己的 `transport.rs`
3. WHEN `src-tauri` 处理握手 THEN SHALL 使用 `kvm_core::network::HandshakeInfo` 和 `kvm_core::protocol::PROTOCOL_VERSION`
4. WHEN `src-tauri` 处理心跳 THEN SHALL 使用 `kvm_core::network` 中定义的心跳常量（`HEARTBEAT_INTERVAL_SECS`、`HEARTBEAT_TIMEOUT_COUNT`）
5. WHEN `src-tauri` 处理连接状态 THEN SHALL 使用 `kvm_core::network::ConnectionState` 和 `NetworkEvent`
6. WHEN `src-tauri` 处理自动重连 THEN SHALL 使用 `kvm_core::config::KvmConfig` 中的重连配置

---

### 需求 3：`src-tauri` 支持 macOS，实现双端 GUI

**用户故事：** 作为 macOS 用户，我希望能运行与 Windows 端功能完全一致的 GUI 应用，以便获得相同的使用体验，而不是只能用命令行。

#### 验收标准

1. WHEN 在 macOS 上编译 `src-tauri` THEN SHALL 编译成功，无平台相关错误
2. WHEN 在 macOS 上启动 THEN SHALL 显示与 Windows 端完全相同的主界面
3. WHEN 在 macOS 上启动 THEN SHALL 检查辅助功能权限，未授权时弹出引导对话框（复用 `kvm-mac/src/platform/permission.rs` 的逻辑）
4. WHEN 在 macOS 上运行 THEN SHALL 使用 macOS 平台适配器（输入采集/注入/快捷键/屏幕/剪贴板）
5. WHEN 在 Windows 上运行 THEN SHALL 使用 Windows 平台适配器
6. WHEN 查看 `src-tauri/src/platform/` THEN SHALL 包含 `mod.rs`（trait 定义）、`windows.rs`（Windows 实现）、`macos.rs`（macOS 实现）
7. WHEN 平台特定代码 THEN SHALL 只出现在 `src-tauri/src/platform/` 目录下，通过 `#[cfg(target_os)]` 条件编译隔离

---

### 需求 4：版本号统一，两端显示一致

**用户故事：** 作为用户，我希望在两端的界面看到完全相同的版本号，以便确认两端版本匹配，避免协议不兼容。

#### 验收标准

1. WHEN 查看根目录 `Cargo.toml` THEN `[workspace.package] version` SHALL 是唯一的版本号来源（当前应为 `0.2.0`）
2. WHEN 查看 `src-tauri/Cargo.toml` THEN `version` SHALL 使用 `version.workspace = true`，不得硬编码
3. WHEN 查看 `src-tauri/tauri.conf.json` THEN `package.version` SHALL 与 workspace version 保持一致
4. WHEN 前端显示版本号 THEN SHALL 通过 `invoke('get_app_version')` 从后端读取（后端返回 `env!("CARGO_PKG_VERSION")`），不得硬编码
5. WHEN 握手协议中的协议版本号 THEN SHALL 使用 `kvm_core::protocol::PROTOCOL_VERSION` 常量，两端共用

---

### 需求 5：默认 IP 不硬编码，改为从配置文件读取

**用户故事：** 作为用户，我希望应用记住上次连接的对端 IP，以便下次启动时无需重新输入，且不同用户的 IP 不会互相干扰。

#### 验收标准

1. WHEN 查看 `src-tauri/src/connection/mod.rs` THEN `DEFAULT_PEER_IP` 硬编码常量 SHALL 被删除
2. WHEN 首次启动 THEN 对端 IP 输入框 SHALL 显示空白（不预填任何 IP）
3. WHEN 用户输入 IP 并保存设置 THEN SHALL 将 IP 持久化到本地配置文件（`~/.config/kvmini/config.json` 或系统对应路径）
4. WHEN 重启应用 THEN SHALL 自动从配置文件读取并填入上次保存的对端 IP
5. WHEN 后端 Tauri 命令 `get_settings` THEN SHALL 返回从配置文件读取的 IP，而非硬编码值
6. WHEN 后端 Tauri 命令 `save_settings` THEN SHALL 将包含 IP 的配置持久化到文件

---

### 需求 6：前端 Tauri 命令与后端完全对齐

**用户故事：** 作为开发者，我希望前端调用的所有 `invoke` 命令在后端都有对应实现，以便功能正常运行，不出现"命令未找到"错误。

#### 验收标准

1. WHEN 前端调用 `invoke('get_settings')` THEN 后端 SHALL 有 `#[tauri::command] fn get_settings()` 实现，返回完整配置（含 peer_ip、port、快捷键、自动重连等）
2. WHEN 前端调用 `invoke('save_settings')` THEN 后端 SHALL 有对应实现，接收并持久化配置
3. WHEN 前端调用 `invoke('get_app_version')` THEN 后端 SHALL 返回 `env!("CARGO_PKG_VERSION")`
4. WHEN 前端调用 `invoke('get_local_ip')` THEN 后端 SHALL 返回本机所有局域网 IP 列表
5. WHEN 前端调用 `invoke('connect_to_peer')` THEN 后端 SHALL 使用 `kvm_core::network::ConnectionManager` 建立连接
6. WHEN 后端注册 Tauri 命令 THEN `invoke_handler` 中注册的命令列表 SHALL 与前端 `platform.js` 中调用的命令完全对应，无遗漏

---

### 需求 7：两端功能完全对齐

**用户故事：** 作为用户，我希望 Windows 端和 macOS 端拥有完全相同的功能集，以便在任意一端都能正常使用所有功能。

#### 验收标准

1. WHEN 两端都启动 THEN SHALL 都能发起连接（客户端模式）和接受连接（服务端模式）
2. WHEN 两端都启动 THEN SHALL 都有系统托盘图标，支持显示/隐藏窗口、连接/断开、退出
3. WHEN 两端都启动 THEN SHALL 都支持 UDP 局域网设备发现（广播发现对端）
4. WHEN 两端都启动 THEN SHALL 都支持手动输入 IP 直连（跨网段）
5. WHEN 两端都启动 THEN SHALL 都支持全局快捷键（`Ctrl+Alt+S` 切换控制权）
6. WHEN 两端都启动 THEN SHALL 都支持屏幕边缘触发切换（鼠标移到屏幕边缘自动切换控制权）
7. WHEN 两端都启动 THEN SHALL 都支持剪贴板共享
8. WHEN 两端都启动 THEN SHALL 都支持开机自启动配置
9. IF 某个功能在当前平台不支持 THEN SHALL 显示灰色禁用状态并说明原因，不得直接隐藏

---

### 需求 8：平台隔离规范，条件编译清晰

**用户故事：** 作为开发者，我希望平台特定代码有清晰的隔离边界，以便在不同平台上独立编译和维护，不出现跨平台编译错误。

#### 验收标准

1. WHEN 查看 `kvm-core/` THEN SHALL 不包含任何 `#[cfg(target_os)]` 条件编译（纯逻辑库，零平台依赖）
2. WHEN 查看 `src-tauri/src/platform/` THEN SHALL 包含 `mod.rs`（定义 `PlatformAdapter` trait）、`windows.rs`（Windows 实现）、`macos.rs`（macOS 实现）
3. WHEN 其他模块需要平台功能 THEN SHALL 通过 `platform::PlatformAdapter` trait 调用，不得直接调用平台 API
4. WHEN 编译 Windows 版本 THEN SHALL 只链接 Windows 平台依赖（`winapi`、`winreg` 等）
5. WHEN 编译 macOS 版本 THEN SHALL 只链接 macOS 平台依赖（`core-graphics`、`core-foundation`、`objc` 等）
6. WHEN 查看 `src-tauri/Cargo.toml` THEN 平台依赖 SHALL 使用 `[target.'cfg(target_os = "windows")'.dependencies]` 和 `[target.'cfg(target_os = "macos")'.dependencies]` 分组声明
