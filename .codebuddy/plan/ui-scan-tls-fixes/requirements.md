# 需求文档：全量存根实现 + UI 修复 + 平台隔离

## 引言

经过全面代码扫描，项目中存在以下**真实存根**（接口存在但功能未实现）和 **UI 问题**，本次需求要求全部一次性实现完毕，不再遗漏：

### 已确认的存根清单

| 存根位置 | 问题描述 |
|---|---|
| `tls.rs` `TlsStreamWrapper` | 注释明确写"待 rustls 依赖添加后实现"，TCP 连接未包裹 TLS 加密层 |
| `platform.rs` `register_global_hotkey` | 注释写"实际注册在前端通过 Tauri API 完成"，只保存配置，未真正注册 |
| `platform.rs` `save_settings(auto_start)` | `auto_start` 字段只保存到 JSON，未调用 macOS LaunchAgent / Windows 注册表 |

### 已确认的 UI 问题

| 问题 | 描述 |
|---|---|
| 两个"扫描设备"按钮 | 顶部控制栏和设备面板各有一个，功能完全重复 |
| 扫描到本机自身 | UDP 广播时本机也在监听并回复，导致自己出现在设备列表 |
| 本机指纹说明缺失 | 用户不知道"本机证书指纹"是什么用途 |

### 已确认正常的功能（无需实现）

- macOS CGEvent Tap 键鼠捕获 ✅（`capturer.rs` `MacOSCapturer`）
- Windows SetWindowsHookEx 键鼠捕获 ✅（`capturer.rs` `WindowsCapturer`）
- macOS CGEvent 键鼠注入 ✅（`injector.rs` `MacOSInjector`）
- Windows SendInput 键鼠注入 ✅（`injector.rs` `WindowsInjector`）
- 跨平台键码映射 ✅（`keycode_map.rs`）
- 事件循环串联 ✅（`platform.rs` `start_controller_loop` / `start_receiver_loop`）
- 剪贴板共享 ✅（`clipboard.rs` + `start_clipboard_sync`）
- 多显示器支持 ✅（`monitor.rs`）
- UDP 广播扫描 ✅（`network.rs`）
- 真实延迟 Ping/Pong ✅（`platform.rs` `LAST_RTT_MS`）
- TLS 证书生成和指纹计算 ✅（`tls.rs` `TlsManager`）

---

## 需求

### 需求 1：移除顶部重复的"扫描设备"按钮

**用户故事：** 作为用户，我希望界面上只有一个"扫描设备"入口，以便操作清晰不混乱。

#### 验收标准

1. WHEN 用户打开主界面 THEN 顶部控制栏 SHALL 不再显示"扫描设备"按钮（`connect-btn`）
2. WHEN 顶部按钮被移除 THEN 设备面板内的"扫描设备"按钮（`scan-btn`）SHALL 作为唯一扫描入口
3. IF 顶部按钮被移除 THEN 原有的 `handleConnect` 事件绑定 SHALL 一并清理，不留死代码
4. WHEN 顶部按钮移除后 THEN 控制栏 SHALL 仍保留"断开连接"、"开始共享"、"设置"三个按钮，布局不破坏

---

### 需求 2：扫描时过滤本机 IP，不将自身显示在设备列表

**用户故事：** 作为用户，我希望扫描结果只显示其他设备，以便不会误连接到自己。

#### 验收标准

1. WHEN UDP 广播扫描收到响应 THEN 系统 SHALL 对比响应来源 IP 与本机所有 IP 地址
2. IF 响应来源 IP 与本机任意一个 IP 相同 THEN 系统 SHALL 丢弃该响应，不加入设备列表
3. WHEN 本机 IP 获取失败 THEN 系统 SHALL 降级为仅过滤 `127.0.0.1` 和 `::1`
4. WHEN 过滤后设备列表为空 THEN 前端 SHALL 显示"未发现其他设备"提示，而非空白

---

### 需求 3：本机证书指纹展示加说明文字

**用户故事：** 作为用户，我希望知道"本机证书指纹"是什么、有什么用，以便理解安全机制。

#### 背景说明

本机指纹是 TLS TOFU（Trust On First Use）机制的核心：
- 本机生成自签名证书，指纹为该证书的 SHA-256 哈希
- 对端首次连接时展示本机指纹，用户确认后保存信任记录
- 后续连接自动验证指纹，指纹变化时拒绝连接（防中间人攻击）
- 类似 SSH 首次连接时的 `Are you sure you want to continue connecting?`

#### 验收标准

1. WHEN 用户打开设置页安全与加密区域 THEN 本机证书指纹旁 SHALL 显示说明文字："对端首次连接时需核对此指纹，确认一致后方可信任连接"
2. WHEN TLS 功能已启用 THEN 本机证书指纹 SHALL 正常显示（调用 `get_local_tls_fingerprint` 命令）
3. IF TLS 功能未启用 THEN 本机证书指纹行 SHALL 显示为灰色并标注"（TLS 已关闭）"
4. WHEN 用户点击"复制"按钮 THEN 系统 SHALL 将完整指纹复制到剪贴板并显示成功提示

---

### 需求 4：实现 TLS 实际加密传输（rustls 集成）

**用户故事：** 作为用户，我希望主控端与被控端之间的数据传输经过 TLS 1.3 加密，以便防止网络监听和中间人攻击。

#### 背景说明

当前 `tls.rs` 中 `TlsStreamWrapper` 是接口占位符，注释明确写"待 rustls 依赖添加后实现"。需要：
1. 在 `Cargo.toml` 中添加 `rustls` 和 `tokio-rustls` 依赖
2. 将 `platform.rs` 中的 `TcpStream` 包裹为 `TlsStream`
3. 握手时执行 TOFU 指纹验证

#### 验收标准

1. WHEN 用户在设置中开启 TLS 加密 THEN 系统 SHALL 在建立 TCP 连接后进行 TLS 握手，将 `TcpStream` 包裹为加密流
2. WHEN TLS 握手成功 THEN 系统 SHALL 使用加密流收发所有键鼠事件和剪贴板数据
3. WHEN 对端证书指纹与已保存记录不匹配 THEN 系统 SHALL 拒绝连接并向用户显示警告
4. WHEN 对端为首次连接 THEN 系统 SHALL 弹出确认对话框展示对端指纹，用户确认后保存信任记录
5. IF TLS 加密关闭 THEN 系统 SHALL 回退到明文 TCP 传输，不影响功能
6. WHEN TLS 握手失败 THEN 系统 SHALL 向用户显示具体错误原因（证书过期/版本不兼容/握手超时）

---

### 需求 5：实现开机自启动（macOS + Windows）

**用户故事：** 作为用户，我希望开启"开机自启动"后系统重启时应用自动运行，以便不需要手动启动。

#### 背景说明

当前 `platform.rs` 中 `save_settings` 只将 `auto_start: true` 保存到 JSON 文件，没有任何系统级注册逻辑。需要在 `save_settings` 中检测 `auto_start` 变化并调用平台 API。

#### 验收标准

1. WHEN 用户在设置中开启"开机自启动" THEN 系统 SHALL 在 macOS 上创建 `~/Library/LaunchAgents/com.kvmshare.app.plist` 文件
2. WHEN 用户在设置中开启"开机自启动" THEN 系统 SHALL 在 Windows 上向 `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` 注册表写入启动项
3. WHEN 用户关闭"开机自启动" THEN 系统 SHALL 删除对应的 plist 文件或注册表项
4. WHEN 系统重启后 THEN 应用 SHALL 自动启动并最小化到系统托盘
5. IF 写入 plist 或注册表失败 THEN 系统 SHALL 向用户显示错误提示，不静默失败

---

### 需求 6：实现全局快捷键真正注册（macOS + Windows）

**用户故事：** 作为用户，我希望设置的全局快捷键能真正生效，以便在任意应用中按下快捷键时触发主控/被控切换。

#### 背景说明

当前 `platform.rs` 中 `register_global_hotkey` 注释写"实际注册在前端通过 Tauri API 完成"，但前端也没有调用 Tauri globalShortcut API。需要在 Tauri 的 `setup` 钩子中注册全局快捷键，并在快捷键触发时向前端发送事件。

#### 验收标准

1. WHEN 用户保存快捷键设置 THEN 系统 SHALL 调用 Tauri `globalShortcut.register()` 注册全局快捷键
2. WHEN 用户在任意应用中按下已注册的快捷键 THEN 系统 SHALL 向前端发送 `hotkey-triggered` 事件，触发主控/被控角色切换
3. WHEN 快捷键与其他应用冲突导致注册失败 THEN 系统 SHALL 向用户显示冲突提示，不静默失败
4. WHEN 应用退出时 THEN 系统 SHALL 注销已注册的全局快捷键，释放系统资源
5. WHEN 用户修改快捷键 THEN 系统 SHALL 先注销旧快捷键，再注册新快捷键

---

### 需求 7：Windows / macOS 平台代码严格隔离，消除跨平台编译错误

**用户故事：** 作为开发者，我希望在 Windows 上编译时不出现 macOS 专属 API 的编译错误，反之亦然，以便两端都能独立构建。

#### 验收标准

1. WHEN 在 Windows 上执行 `cargo build` THEN 编译 SHALL 成功，不出现 macOS 专属 API（如 `CGEvent`、`CGKeyCode`、`NSPasteboard`、`scutil`、`AXIsProcessTrustedWithOptions`）的编译错误
2. WHEN 在 macOS 上执行 `cargo build` THEN 编译 SHALL 成功，不出现 Windows 专属 API（如 `SetWindowsHookEx`、`SendInput`、`GetClipboardData`、`RegisterHotKey`、`winreg`）的编译错误
3. WHEN 平台专属代码块使用 `#[cfg(target_os = "macos")]` 或 `#[cfg(target_os = "windows")]` THEN 另一平台 SHALL 有对应的分支提供空实现或编译占位
4. WHEN 公共接口函数在不支持的平台被调用 THEN 系统 SHALL 返回 `Err("当前平台不支持此功能")` 而非编译失败
5. WHEN Windows 编译时 THEN 所有 macOS 专属 FFI 声明（如 `AXIsProcessTrustedWithOptions`）SHALL 被 `#[cfg(target_os = "macos")]` 完全包裹，不泄漏到 Windows 编译单元
6. WHEN 开机自启动功能在 Windows 上编译时 THEN `winreg` crate 依赖 SHALL 仅在 `[target.'cfg(target_os = "windows")'.dependencies]` 中声明，不影响 macOS 编译
