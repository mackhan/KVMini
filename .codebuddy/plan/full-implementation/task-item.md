# 实施计划

- [ ] 1. 修复 UI Bug（连接错误提示、i18n、按钮语义、默认快捷键）
   - 修复连接失败显示 `undefined`，增加兜底文案和具体错误原因
   - 补全 `manualIpPlaceholder` 等缺失的 i18n 翻译 key
   - 顶部"连接"按钮改为"扫描设备"，消除语义重复
   - 设置页快捷键输入框空值时自动填充默认值 `Ctrl+Alt+S`
   - _需求：1、2、3、4_

- [ ] 2. 实现 UDP 广播扫描——被控端响应广播
   - 在 `platform.rs` 中为被控端启动时开启 UDP 广播监听线程（端口 8888）
   - 收到 `KVM_DISCOVERY` 包后立即回复 `KVM_RESPONSE`（含设备名、IP、TCP 端口）
   - 主控端扫描固定等待 5 秒，超时后返回设备列表
   - 前端扫描中显示"扫描中..."动画，结束后展示结果或"未发现设备"提示
   - 被控端监听线程崩溃时自动重启，最多重试 3 次
   - _需求：5_

- [ ] 3. 实现真实延迟 Ping/Pong 检测
   - 在 `protocol.rs` 中添加 `KvmFrame::Ping` / `KvmFrame::Pong` 帧类型
   - TCP 连接建立后每 2 秒发送一次 `Ping`，记录发送时间戳
   - 收到 `Pong` 时计算 RTT，通过 Tauri 事件 `latency-update` 推送前端
   - 前端延迟超 50ms 显示黄色，超 100ms 显示红色，断开后重置为"-"
   - _需求：6_

- [ ] 4. 实现 macOS CGEvent Tap 键鼠捕获
   - 在 `capturer.rs` 中实现 `MacosCapturer`，在专用子线程启动 CGEvent Tap
   - 使用 `CGPreflightListenEventAccess()` 检测权限，无权限时返回 `PermissionDenied` 并通知前端
   - 鼠标移动事件以最高 1000Hz 节流，发送归一化坐标的 `KvmFrame::MouseMove`
   - `set_intercept(true)` 时拦截事件不传递给本机；CGEvent Tap 被禁用时自动重启（最多 3 次）
   - 在同一子线程调用 `CFRunLoopRun()` 驱动事件循环
   - _需求：7_

- [ ] 5. 实现 macOS CGEvent 键鼠注入
   - 在 `injector.rs` 中实现 `MacosInjector`
   - `MouseMove` → `CGWarpMouseCursorPosition`（处理 Retina 逻辑像素转换）
   - `MouseButton` → `CGEvent::new_mouse_event`；`MouseScroll` → `CGEvent::new_scroll_event`
   - `KeyEvent` → `CGEvent::new_keyboard_event`；`SwitchControl` → 光标定位到对应边缘
   - _需求：8_

- [ ] 6. 实现 Windows SetWindowsHookEx 捕获 + SendInput 注入
   - `capturer.rs` Windows 分支：专用线程安装 `WH_MOUSE_LL` + `WH_KEYBOARD_LL` 低级钩子
   - 钩子回调中仅做 `try_send()`，确保 300ms 内返回；`stop()` 时调用 `UnhookWindowsHookEx`
   - `injector.rs` Windows 分支：`MouseMove` → `SendInput MOUSEEVENTF_MOVE|ABSOLUTE`
   - `KeyEvent` → `SendInput KEYBDINPUT`；处理 DPI 缩放坐标映射（0-65535 范围）
   - 进程启动时调用 `SetProcessDpiAwarenessContext` 确保 DPI 感知
   - _需求：9、10_

- [ ] 7. 实现跨平台键码映射表
   - 在 `keycode_map.rs` 中建立 `CGKeyCode ↔ VirtualKey` 双向映射表
   - 覆盖字母 A-Z、数字 0-9、F1-F12、方向键、修饰键（Ctrl/Alt/Shift/Win/Cmd）
   - `Command` 键默认映射为 Windows `Win` 键
   - 映射表中不存在的键码丢弃并记录警告日志，不崩溃
   - `KvmFrame::KeyEvent` 协议层携带平台标识（`platform: "macos"` 或 `"windows"`）
   - _需求：11_

- [ ] 8. 串联事件循环（捕获 → TCP → 注入）
   - TCP 连接建立成功后，根据角色自动启动捕获器（主控端）或注入器（被控端）
   - 主控端捕获到事件 → 键码映射 → TCP 发送；被控端收到帧 → 键码映射 → 注入器执行
   - 边缘检测触发切换时发送 `KvmFrame::SwitchControl`，主控端停止拦截，被控端接管
   - TCP 断开时停止捕获器，恢复本机输入控制权
   - _需求：12_

- [ ] 9. 多显示器支持（前端 UI + 后端坐标归一化）
   - 设置页调用 `get_monitors()` 获取显示器列表，以可视化方式展示布局
   - 支持用户拖拽调整显示器相对位置，保存布局配置
   - 鼠标坐标归一化基于虚拟桌面总尺寸（所有显示器包围盒）
   - 鼠标到达任意显示器边缘时根据布局判断是否触发跨设备切换
   - 显示器配置变化时通过 Tauri 事件 `monitors-changed` 通知前端更新
   - _需求：13_

- [ ] 10. 剪贴板共享 + TLS 1.3 加密传输
   - 10.1 剪贴板共享：macOS 200ms 轮询 `NSPasteboard.changeCount`，Windows 使用 `AddClipboardFormatListener`
   - 复制后 500ms 内通过 TCP 发送 `KvmFrame::Clipboard { text }`，超 1MB 截断
   - 收到远端剪贴板时写入本机，使用标志位防止循环同步；设置中可关闭
   - 10.2 TLS 加密：使用 `rcgen` 生成 RSA-2048 自签名证书，`rustls` 实现 TLS 1.3
   - TOFU 模式：首次连接展示证书指纹，用户确认后保存；指纹不匹配时拒绝连接
   - 设置中可降级为明文连接（显示安全警告）
   - _需求：14、15_
