# 实施计划：KVMini 完整实现

- [ ] 1. 修复 UI Bug（连接错误、i18n、按钮语义、默认快捷键）
   - 修复前端连接失败时显示 `undefined` 的问题，增加错误兜底文案
   - 补全 `manualIpPlaceholder` 等缺失的中英文 i18n 翻译 key
   - 将顶部"连接"按钮改为"扫描并连接"或移除，消除与设备列表按钮的语义重复
   - 设置页快捷键输入框在 `hotkey` 为空时自动填充默认值 `Ctrl+Alt+S`
   - _需求：1.1、1.2、1.4、2.1、2.2、2.4、3.1、3.3、4.1、4.2_

- [ ] 2. 实现 macOS CGEvent Tap 键鼠捕获
   - 在专用子线程中启动 CGEvent Tap，同线程调用 `CFRunLoopRun()` 驱动事件循环
   - 使用 `CGPreflightListenEventAccess()` 检测权限，无权限时返回 `CapturerError::PermissionDenied` 并通知前端
   - 捕获鼠标移动（1000Hz 节流）、按键、滚轮事件，转换为 `KvmFrame` 发送到 channel
   - 实现 `set_intercept(true/false)` 控制事件是否传递给本机
   - CGEvent Tap 被系统禁用时自动重启，最多重试 3 次
   - _需求：5.1、5.2、5.3、5.4、5.5、5.6、5.7、5.8_

- [ ] 3. 实现 macOS CGEvent 键鼠注入
   - 收到 `MouseMove` 时调用 `CGWarpMouseCursorPosition`，处理 Retina 逻辑像素转换
   - 收到 `MouseButton`、`MouseScroll`、`KeyEvent` 时分别调用对应 `CGEvent::new_*` 注入
   - 收到 `SwitchControl` 时将光标定位到对应边缘位置
   - _需求：6.1、6.2、6.3、6.4、6.5_

- [ ] 4. 实现 Windows SetWindowsHookEx 键鼠捕获
   - 在专用线程中安装 `WH_MOUSE_LL` 和 `WH_KEYBOARD_LL` 低级钩子
   - 钩子回调中仅做 `try_send()`，确保 300ms 内返回，不执行耗时操作
   - 实现 `set_intercept(true/false)`，拦截时回调返回非零值
   - `stop()` 时调用 `UnhookWindowsHookEx` 并退出消息循环
   - 在 `Cargo.toml` 中为 Windows 目标启用 `winapi` 相关 features
   - _需求：7.1、7.2、7.3、7.4、7.5_

- [ ] 5. 实现 Windows SendInput 键鼠注入
   - 收到 `MouseMove` 时调用 `SendInput` 以 `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` 移动光标
   - 收到 `MouseButton`、`MouseScroll`、`KeyEvent` 时分别调用 `SendInput` 注入对应事件
   - 进程启动时调用 `SetProcessDpiAwarenessContext` 确保 DPI 感知，坐标映射到 0-65535 范围
   - _需求：8.1、8.2、8.3、8.4、8.5、8.6_

- [ ] 6. 实现跨平台键码映射表
   - 在 `KvmFrame::KeyEvent` 协议中增加 `platform` 字段（`"macos"` / `"windows"`）
   - 实现 `CGKeyCode → VirtualKey` 和 `VirtualKey → CGKeyCode` 双向映射表
   - 覆盖 A-Z、0-9、F1-F12、方向键、修饰键（含 Command → Win 映射）
   - 映射表中不存在的键码丢弃并记录警告日志，不崩溃
   - _需求：9.1、9.2、9.3、9.4、9.5、9.6_

- [ ] 7. 串联事件循环（捕获 → TCP → 注入）
   - TCP 连接建立后根据角色自动启动捕获器（主控端）或注入器（被控端）
   - 主控端：捕获事件 → 键码映射 → TCP 发送
   - 被控端：TCP 接收 → 键码映射 → 注入器执行
   - 边缘检测触发切换时发送 `SwitchControl` 帧，主控端停止拦截，被控端开始接管
   - TCP 断开时停止捕获器，恢复本机输入控制权
   - _需求：10.1、10.2、10.3、10.4、10.5_

- [ ] 8. 实现多显示器支持
   - 新增 Tauri 命令 `get_monitors()`，macOS 用 `CGGetActiveDisplayList`，Windows 用 `EnumDisplayMonitors`
   - 边缘检测基于虚拟桌面总尺寸（所有显示器包围盒）进行坐标归一化
   - 鼠标到达任意显示器边缘时根据布局配置判断是否触发跨设备切换
   - 显示器配置变化时通过 Tauri 事件 `monitors-changed` 通知前端
   - _需求：11.1、11.2、11.3、11.4、11.5、11.6_

- [ ] 9. 实现剪贴板共享（Mac ↔ Windows 双向同步）
   - macOS 用 200ms 轮询 `NSPasteboard.changeCount` 检测剪贴板变化
   - Windows 用 `AddClipboardFormatListener` 注册窗口消息监听
   - 变化时将文本通过 `KvmFrame::Clipboard { text }` 发送给对端，超过 1MB 截断
   - 收到远端剪贴板时写入本机，使用标志位防止循环同步
   - 在设置中支持开关剪贴板共享功能
   - _需求：12.1、12.2、12.3、12.4、12.5、12.6、12.7、12.8_

- [ ] 10. 实现 TLS 1.3 加密传输
   - 首次启动时用 `rcgen` 自动生成 RSA-2048 自签名证书，保存到应用数据目录
   - 连接时使用 `rustls` 建立 TLS 1.3 加密通道，采用 TOFU 模式验证证书指纹
   - 证书指纹不匹配时拒绝连接并弹出安全警告；TLS 握手失败时通过 Tauri 事件通知前端
   - 在设置中支持关闭加密（降级为明文，显示安全警告）
   - _需求：13.1、13.2、13.3、13.4、13.5、13.6、13.7_
