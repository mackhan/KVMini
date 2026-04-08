# 实施计划

- [ ] 1. 建立共享通信协议模块
   - 在 `src-tauri/src/shared/` 下创建协议结构体：`KvmMessage`、`InputEvent`（mouse_move/mouse_down/mouse_up/key_down/key_up/scroll）、`HandshakePacket`、`HeartbeatPacket`
   - 所有结构体使用 `serde` 序列化，不含任何平台特定依赖
   - _需求：2.1、2.2、6.3_

- [ ] 2. 开发 Mac 端被控端程序框架
   - 在项目根目录新建 `kvm-agent/` 作为独立 Rust 可执行程序（`cargo new kvm-agent`）
   - 实现 UDP 广播监听（端口 8888），收到 `KVM_DISCOVER` 后回复 `KVM_RESPONSE`（含主机名、OS 类型）
   - 实现 TCP 服务端监听（端口 9999），接受 Windows 端连接并建立持久会话
   - 所有 macOS 特定代码使用 `#[cfg(target_os = "macos")]` 隔离
   - _需求：1.1、1.2、6.1、6.2_

- [ ] 3. 实现 Mac 端辅助功能权限检测与输入事件执行
   - 使用 `core-graphics` crate 调用 `CGEvent` API 实现鼠标移动、点击、滚轮模拟
   - 实现 Windows keycode → macOS 虚拟键码的映射表，模拟键盘按键
   - 程序启动时检测辅助功能权限，未授权则输出引导文案（适配 macOS 13+ "系统设置 → 隐私与安全性 → 辅助功能"路径）
   - _需求：4.1、4.2、4.3、4.4、4.5、6.5_

- [ ] 4. 实现 Mac 端系统托盘与连接状态显示
   - 使用 `tray-icon` 或系统 API 在菜单栏显示托盘图标
   - 托盘菜单支持：显示当前连接的 Windows 端 IP、启动/停止监听
   - _需求：1.3、1.4_

- [ ] 5. 实现心跳机制与断线重连
   - 双端每 2 秒发送心跳包，超过 6 秒未收到则判定断开
   - Windows 端断线后自动重连，最多重试 3 次，重试间隔递增
   - Mac 端断线后独立恢复到待机监听状态，不影响托盘程序运行
   - _需求：2.3、2.4、6.7_

- [ ] 6. 实现 Windows 端全局输入钩子与事件转发
   - 使用 `SetWindowsHookEx`（Win32 API）注册全局鼠标和键盘钩子，代码用 `#[cfg(target_os = "windows")]` 隔离
   - 检测鼠标到达屏幕右边缘时锁定鼠标并进入转发模式，将相对坐标/keycode/滚轮增量通过 TCP 发送给 Mac 端
   - 监听 `Ctrl+Alt+Tab` 快捷键切换回本地控制模式；连接断开时立即释放鼠标锁定
   - 兼容 Windows 10/11，不使用仅 Windows 11 独有 API
   - _需求：3.1、3.2、3.3、3.4、3.5、6.1、6.6_

- [ ] 7. 优化 Windows 端设备发现与连接 UI
   - 修复设备列表中 `undefined` 字段，正确显示 Mac 主机名、IP、OS 类型（对接需求 1.1 的响应包字段）
   - 连接按钮点击后显示进度状态（发现 → 握手 → 已连接）
   - 连接成功后状态面板显示设备名称和延迟（ms）；连接失败显示具体错误原因
   - _需求：5.1、5.2、5.3、5.4_
