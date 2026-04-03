# 需求文档：辅助功能权限引导 & 连接真实性验证

## 引言

本需求文档涵盖两个紧密相关的问题修复，以及平台隔离设计规范：

1. **辅助功能权限（Accessibility）真实检测与引导**：当前 `check_accessibility_macos()` 硬编码返回 `Denied`，未调用系统 API，也无自签名引导流程。参考 OneNoteCopy 项目，需实现真实的权限检测、自签名证书配置，以及引导用户授权的完整 UI 流程。

2. **连接真实性验证（"Win 已关闭但还能连上"问题）**：当前 `connect_by_ip` 仅完成 TCP 三次握手即返回成功，未执行 KVMini 握手协议验证，导致任何监听 8889 端口的服务都会被误判为"连接成功"。连接后 stream 也未被保存，是一个假连接。

3. **macOS 与 Windows 平台隔离**：平台特定代码（辅助功能权限、CGEvent、SendInput 等）必须严格隔离在各自平台模块中，通过统一 trait 接口对上层暴露，跨平台公共逻辑不得包含任何平台特定代码，确保编译时隔离、测试时可 Mock。

---

## 需求

### 需求 1：macOS 辅助功能权限真实检测

**用户故事：** 作为 KVMini 用户，我希望应用能真实检测辅助功能权限状态，以便在未授权时得到准确提示，而不是每次都显示"未授权"。

#### 验收标准

1. WHEN 应用在 macOS 上运行 THEN 系统 SHALL 调用 `AXIsProcessTrustedWithOptions` 真实检测权限状态，而非硬编码返回 `Denied`
2. WHEN 权限已授权 THEN `check_accessibility` 命令 SHALL 返回 `Granted`
3. WHEN 权限未授权 THEN `check_accessibility` 命令 SHALL 返回 `Denied`
4. IF 运行平台为 Windows THEN 系统 SHALL 始终返回 `Granted`（Windows 不需要此权限）

---

### 需求 2：自签名证书配置（参考 OneNoteCopy）

**用户故事：** 作为开发者，我希望应用使用自签名证书进行代码签名，以便 macOS 辅助功能权限系统能稳定识别应用身份，避免每次重新编译后权限失效。

#### 验收标准

1. WHEN 应用以开发模式运行 THEN 系统 SHALL 使用自签名证书对可执行文件进行签名，使 TCC 数据库能稳定记录权限
2. WHEN 应用签名后 THEN 辅助功能权限 SHALL 在重新编译后不需要重新授权（只要签名 identity 不变）
3. IF 自签名证书不存在 THEN 系统 SHALL 提供创建证书的脚本或文档说明

---

### 需求 3：辅助功能权限引导 UI

**用户故事：** 作为 KVMini 用户，我希望在权限未授权时看到清晰的引导界面，以便知道如何在系统设置中授权，而不是看到一个无法操作的错误弹窗。

#### 验收标准

1. WHEN 应用启动且辅助功能权限未授权 THEN 系统 SHALL 在主界面顶部显示权限引导横幅（Banner）
2. WHEN 用户点击"前往授权"按钮 THEN 系统 SHALL 打开 macOS 系统设置 → 隐私与安全性 → 辅助功能页面
3. WHEN 用户在系统设置中完成授权并返回应用 THEN 系统 SHALL 自动重新检测权限状态并隐藏引导横幅
4. WHEN 权限已授权 THEN 系统 SHALL 不显示任何权限相关提示
5. IF 用户关闭引导横幅 THEN 系统 SHALL 在本次会话内不再显示，但下次启动时重新检测

---

### 需求 4：连接真实性验证（KVMini 握手协议验证）

**用户故事：** 作为 KVMini 用户，我希望"连接成功"意味着对端确实运行着 KVMini 服务，以便避免误连到其他程序的端口而产生混淆。

#### 验收标准

1. WHEN 用户点击"连接"按钮 THEN 系统 SHALL 在 TCP 连接成功后，执行 KVMini 握手协议（发送 `Handshake` 帧，等待 `HandshakeAck` 响应）
2. WHEN 握手协议验证通过 THEN 系统 SHALL 显示"已成功连接到 IP:Port"
3. WHEN TCP 连接成功但握手失败（对端非 KVMini 服务）THEN 系统 SHALL 显示"连接失败：对端不是 KVMini 服务"
4. WHEN 握手协议版本不匹配 THEN 系统 SHALL 显示"版本不兼容：请确保两端使用相同版本的 KVM Share"
5. WHEN 握手超时（3 秒内无响应）THEN 系统 SHALL 显示"握手超时：对端可能不是 KVMini 服务"
6. WHEN 连接成功且握手通过 THEN 系统 SHALL 保存 TCP stream 供后续键鼠事件传输使用

---

### 需求 5：连接状态持久化与断线检测

**用户故事：** 作为 KVMini 用户，我希望应用能实时反映连接状态，以便在对端断开时立即得到通知。

#### 验收标准

1. WHEN 连接建立后对端主动断开 THEN 系统 SHALL 在 UI 上显示"连接已断开"并清除连接状态
2. WHEN 连接建立后网络中断 THEN 系统 SHALL 在 30 秒心跳超时后显示"连接超时，已断开"
3. WHEN 用户点击"断开连接"THEN 系统 SHALL 关闭 TCP 连接并清除所有连接状态
4. IF 设置了自动重连 THEN 系统 SHALL 在断线后按指数退避策略自动重连

---

### 需求 6：Windows 平台适配

**用户故事：** 作为 Windows 用户，我希望连接验证功能在 Windows 上同样工作，以便跨平台使用 KVMini。

#### 验收标准

1. WHEN Windows 端作为被控端运行 THEN 系统 SHALL 正确响应 macOS 主控端的握手请求
2. WHEN Windows 端作为主控端运行 THEN 系统 SHALL 能向 macOS 被控端发起握手并验证
3. IF Windows 防火墙阻止连接 THEN 系统 SHALL 在连接失败时提示用户检查防火墙设置

---

### 需求 7：macOS 与 Windows 平台代码隔离

**用户故事：** 作为开发者，我希望 macOS 和 Windows 的平台特定代码严格隔离，以便两个平台可以独立编译、独立测试，互不干扰。

#### 验收标准

1. WHEN 编写平台特定代码 THEN 开发者 SHALL 将 macOS 实现放在 `#[cfg(target_os = "macos")]` 条件编译块或独立的 `platform/macos/` 子模块中，Windows 实现放在 `#[cfg(target_os = "windows")]` 或 `platform/windows/` 子模块中
2. WHEN 上层模块（server、client、role_manager 等）需要调用平台功能 THEN 系统 SHALL 通过统一的 trait 接口（如 `EventInjector`、`EventCapturer`、`AccessibilityChecker`）调用，不得直接调用平台 API
3. WHEN 在 macOS 上编译 THEN 系统 SHALL 不包含任何 Windows 专用 API 调用（如 `SendInput`、`SetWindowsHookEx`），反之亦然，确保编译时零平台交叉依赖
4. WHEN 编写单元测试 THEN 开发者 SHALL 使用 `MockInjector`、`MockCapturer` 等 Mock 实现替代真实平台实现，使测试可在任意平台运行
5. WHEN 新增平台特定功能 THEN 开发者 SHALL 先在 trait 接口中定义方法，再分别在 macOS 和 Windows 子模块中实现，不得在公共模块中写 `if cfg!(windows)` 分支逻辑
6. IF 某功能仅 macOS 需要（如辅助功能权限检测）THEN 系统 SHALL 在 Windows 实现中提供空实现或直接返回成功，保持接口一致性
7. WHEN 运行 `cargo test` THEN 所有测试 SHALL 在 macOS 和 Windows 上均能通过，不依赖特定平台环境
