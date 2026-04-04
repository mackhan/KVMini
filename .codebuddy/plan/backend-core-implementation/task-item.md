# 实施计划：KVMini 后端核心功能完整实现

- [ ] 1. 实现 macOS CGEvent Tap 键鼠捕获（`capturer.rs`）
   - 在专用线程中创建 `CFRunLoop`，安装 CGEvent Tap 监听全局键鼠事件
   - 实现鼠标移动 1000Hz 节流（`std::time::Instant` 计时）
   - 实现 `set_intercept(bool)` 控制事件是否透传给本机
   - 实现 tap 被系统禁用后自动重新启用（最多重试 3 次）
   - 权限未授权时返回 `CapturerError::PermissionDenied` 并通过 `emit_all("capturer-error")` 通知前端
   - 使用 `CGPreflightListenEventAccess()` 检测权限，避免 TCC 缓存延迟
   - _需求：1.1、1.2、1.3、1.4、1.5、1.6、1.7、1.8、1.9、1.10_

- [ ] 2. 实现 macOS CGEvent 键鼠注入（`injector.rs`）
   - 使用 `CGWarpMouseCursorPosition` 实现鼠标移动注入，处理 Retina 逻辑像素转换
   - 使用 `CGEvent::new_mouse_event` 实现鼠标按键注入
   - 使用 `CGEvent::new_scroll_event` 实现滚轮注入
   - 使用 `CGEvent::new_keyboard_event` 实现键盘注入
   - 收到 `KvmFrame::SwitchControl` 时将光标定位到对应边缘位置
   - _需求：2.1、2.2、2.3、2.4、2.5、2.6、2.7_

- [ ] 3. 实现 Windows SetWindowsHookEx 键鼠捕获（`capturer.rs`）
   - 在专用线程中安装 `WH_MOUSE_LL` 和 `WH_KEYBOARD_LL` 低级钩子
   - 钩子回调中**仅做 `try_send()`**，不执行任何耗时操作（确保 300ms 内返回）
   - 实现 `set_intercept(bool)` 控制钩子回调是否返回非零值拦截事件
   - 实现 `stop()` 调用 `UnhookWindowsHookEx` 并退出消息循环
   - 启用 `Cargo.toml` 中 winapi 依赖（`winuser`、`winbase` features）
   - _需求：4.1、4.2、4.3、4.4、4.5、4.6_

- [ ] 4. 实现 Windows SendInput 键鼠注入（`injector.rs`）
   - 使用 `SendInput` + `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` 实现鼠标移动，处理 DPI 缩放
   - 调用 `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` 确保 DPI 感知
   - 使用 `GetSystemMetrics(SM_CXVIRTUALSCREEN)` 获取虚拟屏幕尺寸，坐标映射到 0-65535
   - 使用 `SendInput` + `KEYBDINPUT` 实现键盘注入（使用映射后的 VirtualKey）
   - 收到 `KvmFrame::SwitchControl` 时使用 `SetCursorPos` 定位光标到边缘
   - _需求：3.1、3.2、3.3、3.4、3.5、3.6、3.7_

- [ ] 5. 实现跨平台键码映射表（`keycode_map.rs`）
   - 新建 `keycode_map.rs`，实现 `CGKeyCode → VirtualKey` 和 `VirtualKey → CGKeyCode` 双向映射表
   - 覆盖字母 A-Z、数字 0-9、功能键 F1-F12、方向键、修饰键（Ctrl/Alt/Shift/Win/Cmd）等常用键
   - `Command` 键默认映射为 `Win` 键，支持通过设置切换为 `Ctrl`
   - `KvmFrame::KeyEvent` 协议中携带平台标识（`platform: "macos"` / `"windows"`）
   - 映射表中不存在的键码丢弃并记录 warn 日志，不崩溃
   - _需求：10.1、10.2、10.3、10.4、10.5、10.6_

- [ ] 6. 串联事件循环（捕获 → TCP 传输 → 注入）
   - 在 `connection_manager.rs` 的连接建立回调中，根据角色启动捕获器（主控端）或注入器（被控端）
   - 主控端：从捕获器 channel 读取 `KvmFrame`，经键码映射后通过 TCP 连接发送
   - 被控端：从 TCP 接收 `KvmFrame`，经键码映射后调用注入器执行
   - 边缘检测触发切换时发送 `KvmFrame::SwitchControl`，同步更新捕获器拦截状态
   - TCP 断开时停止捕获器并恢复本机输入控制权
   - _需求：5.1、5.2、5.3、5.4、5.5_

- [ ] 7. 实现默认快捷键 `Ctrl+Alt+S` 及前端显示
   - 后端 `get_settings()` 返回空 hotkey 时填充默认值 `"Ctrl+Alt+S"`
   - 快捷键触发时通过 `emit_all("hotkey-switch")` 通知前端切换控制权状态
   - 前端设置页快捷键输入框显示默认值 `Ctrl+Alt+S`，保存时重新注册快捷键
   - 快捷键为空时禁用功能（不注册）
   - _需求：6.1、6.2、6.3、6.4、6.5_

- [ ] 8. 实现快捷键冲突检测（后端 + 前端）
   - 后端新增 `check_hotkey_conflict(hotkey: String) -> ConflictResult` Tauri 命令
   - macOS 平台：内置系统保留快捷键列表（`Cmd+Space`、`Cmd+Tab`、`Ctrl+Space` 等）
   - Windows 平台：内置系统保留快捷键列表（`Win+L`、`Ctrl+Alt+Del`、`Alt+F4` 等）
   - 前端快捷键输入框 `change` 事件触发冲突检测，在输入框下方显示警告文字
   - 用户保存有冲突的快捷键时弹出确认对话框
   - _需求：8.1、8.2、8.3、8.4、8.5、8.6、8.7_

- [ ] 9. 实现托盘菜单国际化与连接状态联动
   - 后端新增 `update_tray_menu(lang: &str, connected: bool)` 函数，根据语言重建托盘菜单文字
   - 前端语言切换时调用 `invoke("update_tray_language")` 通知后端更新菜单文字
   - 连接/断开时更新托盘菜单项启用状态（已连接时"连接设备"置灰）
   - 托盘菜单"连接设备"/"断开连接"点击时通过 `emit_all("tray-connect"/"tray-disconnect")` 通知前端
   - _需求：7.1、7.2、7.3、7.4、7.5_

- [ ] 10. 实现剪贴板共享（`clipboard.rs`）
   - 新建 `clipboard.rs`，定义 `ClipboardMonitor` trait，macOS/Windows 分别实现
   - macOS：每 200ms 轮询 `NSPasteboard.changeCount`，变化时读取纯文本内容
   - Windows：使用 `AddClipboardFormatListener` 注册窗口消息监听，收到 `WM_CLIPBOARDUPDATE` 时读取文本
   - 内容超过 1MB 时截断并记录警告；非文本内容忽略
   - 发送时使用 `KvmFrame::Clipboard { text }`，收到时写入本机剪贴板
   - 使用标志位防止循环同步（收到远端剪贴板写入本机时，暂时屏蔽本机变化监听）
   - 在设置中新增 `clipboard_sync: bool` 开关，关闭时停止监听
   - _需求：11.1、11.2、11.3、11.4、11.5、11.6、11.7、11.8_

- [ ] 11. 实现 TLS 1.3 加密传输（`tls.rs`）
   - 新建 `tls.rs`，使用 `rustls` + `rcgen` crate 实现 TLS 加密层
   - 首次启动时用 `rcgen` 自动生成 RSA-2048 自签名证书，保存到应用数据目录
   - 实现 TOFU 指纹验证：首次连接展示对端证书指纹，用户确认后保存到 `trusted_peers.json`
   - 后续连接自动验证指纹，不匹配时拒绝连接并通过 `emit_all("tls-fingerprint-mismatch")` 通知前端
   - TLS 握手失败时返回具体错误原因并通过 Tauri 事件通知前端
   - 在设置中新增 `enable_tls: bool` 开关，关闭时降级为明文连接并显示安全警告
   - 将现有 `TcpStream` 包装为 `TlsStream<TcpStream>`，对上层透明
   - _需求：12.1、12.2、12.3、12.4、12.5、12.6、12.7_

- [ ] 12. 实现多显示器支持（`monitor.rs`）
   - 新建 `monitor.rs`，定义 `MonitorInfo { id, width, height, dpi, offset_x, offset_y }` 结构体
   - macOS：使用 `CGGetActiveDisplayList` 枚举所有活跃显示器
   - Windows：使用 `EnumDisplayMonitors` 枚举所有显示器
   - 新增 Tauri 命令 `get_monitors() -> Vec<MonitorInfo>`，供前端布局配置界面调用
   - 监听显示器插拔事件，变化时通过 `emit_all("monitors-changed")` 通知前端
   - 更新 `EdgeDetector`：基于虚拟桌面总尺寸（所有显示器包围盒）进行坐标归一化
   - 边缘检测时遍历所有显示器边缘，而非仅检测主显示器
   - _需求：13.1、13.2、13.3、13.4、13.5、13.6、13.7、13.8_

- [ ] 13. 补充后端单元测试与集成测试
   - 为 macOS 捕获器编写测试：使用 MockCapturer 验证事件节流逻辑（1000Hz 限速）
   - 为 macOS/Windows 注入器编写测试：验证归一化坐标转换、DPI 缩放坐标转换
   - 为键码映射编写测试：验证 CGKeyCode ↔ VirtualKey 双向转换正确性（含 Command→Win 映射）
   - 为快捷键冲突检测编写测试：验证 macOS/Windows 各自的系统保留快捷键列表
   - 为剪贴板共享编写测试：验证循环同步防护、1MB 截断、非文本内容忽略
   - 为 TLS 编写测试：验证证书生成、指纹计算、TOFU 验证流程（使用内存 TLS 流）
   - 为多显示器编写测试：验证虚拟桌面包围盒计算、坐标归一化
   - 为事件循环串联编写集成测试：MockCapturer → channel → MockTcpSink → MockInjector 端到端验证
   - _需求：9.1、9.2、9.3、9.4、9.5、9.6、9.7、9.8、9.9_

- [ ] 14. 补充前端测试用例（Jest）
   - 测试快捷键输入框默认值显示 `Ctrl+Alt+S`（需求 6.2）
   - 测试快捷键冲突警告文字显示与确认对话框逻辑（需求 8.6、8.7）
   - 测试托盘菜单语言切换后前端 `invoke("update_tray_language")` 被调用（需求 7.1、7.2）
   - 测试连接/断开时托盘菜单状态更新（需求 7.5）
   - 测试 TLS 指纹确认对话框展示与保存逻辑（需求 12.2、12.3）
   - 测试多显示器布局配置界面渲染与偏移量输入（需求 13.1、13.2）
   - _需求：6.2、7.1、7.2、7.5、8.6、8.7、12.2、12.3、13.1、13.2_
