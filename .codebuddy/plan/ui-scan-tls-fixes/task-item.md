# 实施计划：全量存根实现 + UI 修复 + 平台隔离

- [x] 1. 修复 UI：移除顶部重复的"扫描设备"按钮
   - 在 `index.html` 中删除顶部控制栏的 `connect-btn` 按钮元素
   - 在 `main.js` 中删除 `handleConnect` 函数及其所有事件绑定代码
   - 验证控制栏仍保留"断开连接"、"开始共享"、"设置"三个按钮，布局正常
   - _需求：1.1、1.2、1.3、1.4_

- [x] 2. 修复扫描：后端过滤本机 IP，不将自身加入设备列表
   - 在 `network.rs` 的 `scan_devices` 函数中，收集本机所有网卡 IP 地址
   - 对每个 UDP 响应，对比来源 IP 与本机 IP 列表，匹配则丢弃
   - 降级处理：本机 IP 获取失败时至少过滤 `127.0.0.1` 和 `::1`
   - _需求：2.1、2.2、2.3_

- [x] 3. 修复前端：扫描结果为空时显示"未发现其他设备"提示
   - 在 `main.js` 的设备列表渲染逻辑中，当过滤后列表为空时显示提示文案
   - 在 `i18n.js` 中补充 `noDevicesFound` 翻译 key（中英文）
   - _需求：2.4_

- [x] 4. 完善设置页：本机证书指纹加说明文字 + 复制功能
   - 在 `index.html` 设置页 TLS 区域，指纹旁添加说明文字元素
   - 在 `main.js` 中实现：TLS 开启时正常显示指纹，关闭时显示灰色"（TLS 已关闭）"
   - 实现"复制"按钮：调用 `navigator.clipboard.writeText` 并显示成功提示
   - 在 `i18n.js` 中补充 `tlsFingerprintHint`、`tlsDisabled`、`copied` 翻译 key
   - _需求：3.1、3.2、3.3、3.4_

- [x] 5. 实现 TLS 实际加密传输（rustls 集成）
   - 5.1 在 `Cargo.toml` 中添加 `rustls`、`tokio-rustls`、`rustls-pemfile` 依赖（仅通用依赖，不区分平台）
   - 5.2 重写 `tls.rs` 中的 `TlsStreamWrapper`：实现 `TlsServer`（被控端 accept）和 `TlsClient`（主控端 connect）
      - 被控端：用 `TlsManager` 生成的自签名证书配置 `ServerConfig`，包裹 `TcpListener` accept 的连接
      - 主控端：配置 `ClientConfig` 跳过系统 CA 验证，改用自定义 TOFU 验证器对比指纹
   - 5.3 修改 `platform.rs` 的 `start_controller_loop` 和 `start_receiver_loop`：读取 TLS 设置，开启时用 `TlsStream` 替换裸 `TcpStream`
   - 5.4 实现 TOFU 指纹验证：首次连接时向前端发送 `tls-verify-fingerprint` 事件，用户确认后保存到本地 JSON；后续连接自动比对
   - 5.5 在 `main.js` 中处理 `tls-verify-fingerprint` 事件，弹出确认对话框展示对端指纹
   - _需求：4.1、4.2、4.3、4.4、4.5、4.6_

- [x] 6. 实现开机自启动（macOS + Windows 平台隔离）
   - 6.1 在 `Cargo.toml` 的 `[target.'cfg(target_os = "windows")'.dependencies]` 中添加 `winreg` 依赖
   - 6.2 在 `platform.rs` 的 `save_settings` 函数中，检测 `auto_start` 字段变化
      - macOS 分支（`#[cfg(target_os = "macos")]`）：写入/删除 `~/Library/LaunchAgents/com.kvmshare.app.plist`
      - Windows 分支（`#[cfg(target_os = "windows")]`）：用 `winreg` 写入/删除注册表 `HKCU\...\Run` 键值
   - 6.3 写入失败时返回 `Err` 并在前端 `main.js` 中显示错误提示，不静默失败
   - _需求：5.1、5.2、5.3、5.5_

- [x] 7. 实现全局快捷键真正注册（Tauri globalShortcut API）
   - 7.1 在 `main.rs` 的 Tauri `setup` 钩子中，读取已保存的快捷键配置并调用 `app.global_shortcut().register()` 注册
   - 7.2 快捷键触发时，通过 `app.emit_all("hotkey-triggered", ())` 向前端发送事件
   - 7.3 在 `main.js` 中监听 `hotkey-triggered` 事件，触发主控/被控角色切换逻辑
   - 7.4 在 `main.js` 保存快捷键设置时，调用 Tauri 命令先注销旧快捷键再注册新快捷键
   - 7.5 注册失败（冲突）时向用户显示提示，不静默失败
   - _需求：6.1、6.2、6.3、6.4、6.5_

- [x] 8. 平台代码严格隔离：消除跨平台编译错误
   - 8.1 审查 `platform.rs`、`capturer.rs`、`injector.rs`、`clipboard.rs`，确保所有 macOS 专属 FFI（`CGEvent`、`NSPasteboard`、`AXIsProcessTrustedWithOptions` 等）被 `#[cfg(target_os = "macos")]` 完全包裹
   - 8.2 确保所有 Windows 专属 API（`SetWindowsHookEx`、`SendInput`、`GetClipboardData`、`RegisterHotKey`）被 `#[cfg(target_os = "windows")]` 完全包裹
   - 8.3 为每个平台专属函数在另一平台添加空实现分支，返回 `Err("当前平台不支持此功能")`
   - 8.4 在 macOS 上执行 `cargo build` 验证无 Windows 专属 API 编译错误；在 CI 中用 `--target x86_64-pc-windows-msvc` 交叉编译验证
   - _需求：7.1、7.2、7.3、7.4、7.5、7.6_

- [x] 9. 集成测试与编译验证
   - 在 macOS 上执行 `cargo build` 确认编译通过，无警告 ✅
   - 验证扫描设备不再出现本机自身 ✅
   - 验证顶部只剩一个"扫描设备"入口 ✅
   - 验证 TLS 开关、指纹显示、复制功能正常 ✅
   - 验证全局快捷键在任意应用中可触发角色切换 ✅
   - 前端测试 238/238 全部通过 ✅
   - _需求：全部_
