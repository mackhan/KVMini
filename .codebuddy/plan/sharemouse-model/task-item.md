# 实施计划：KVMini 仿 ShareMouse 重构

- [ ] 1. 搭建 Cargo Workspace 三层架构
   - 修改根目录 `Cargo.toml`，配置 workspace members 包含 `kvm-core`、`kvm-mac`、`kvm-win`、`src-tauri`
   - 创建 `kvm-core/` 目录及 `Cargo.toml`（纯逻辑，无平台依赖）
   - 在 `kvm-core/src/lib.rs` 中声明所有子模块
   - 在 `kvm-core/src/platform.rs` 中定义 `ScreenProvider`、`InputCapture`、`InputExecutor`、`InputBlocker`、`HotkeyManager`、`ClipboardProvider`、`CursorController` 七个 trait 接口
   - _需求：1.1、1.2、1.3、1.4_

- [ ] 2. 实现 kvm-core 核心模块
   - 2.1 实现 `error.rs`：定义统一 `KvmError` 枚举，涵盖网络、协议、权限、配置等所有错误场景，每个变体提供用户友好中文描述
     - _需求：18.1、18.2、18.3、18.4_
   - 2.2 实现 `protocol.rs`：定义事件帧结构（鼠标移动/点击/滚轮、键盘按下/释放、控制权切换、心跳、握手），跨平台统一键码表，长度前缀 + bincode 序列化，事件帧含时间戳
     - _需求：10.1、10.2、10.3、10.4、10.6、10.7、10.8_
   - 2.3 实现 `state.rs`：定义 `ControlState`（Idle / LocalControlling / RemoteControlling）状态机，状态转换逻辑，状态变化回调接口
     - _需求：3.1、3.2、3.3、3.4、3.5、3.6_
   - 2.4 实现 `config.rs`：定义统一配置数据结构（屏幕布局、连接密码、对端 IP、剪贴板开关、自启动开关、日志级别），serde 序列化
     - _需求：16.3_
   - _需求：2.1、2.2_

- [ ] 3. 实现 kvm-core 网络与发现模块（`network.rs`）
   - 实现 TCP 长连接建立/断开/重连（重连间隔 3s，最多 5 次），任意一端均可监听也可主动连接
   - 实现 UDP 广播发现（端口 8888），发送 `KVM_DISCOVERY` 广播，收集响应（设备名、OS、TCP 端口），等待 5 秒返回列表
   - 实现握手协议：交换协议版本号、屏幕布局信息、DPI 缩放信息；版本不一致时拒绝连接
   - 实现心跳保活（每 5 秒一次，连续 3 次无响应判定断开），系统唤醒/锁屏时触发重连或回 Idle
   - _需求：2.1、2.2、2.3、2.4、2.5、2.6、2.7、2.8、2.9、2.10、2.11、2.12、2.13_

- [ ] 4. 实现 kvm-core 边缘触发与光标映射模块（`edge_logic.rs` + `cursor_mapping.rs`）
   - `edge_logic.rs`：实现基于虚拟桌面整体外边缘的触发判定，50ms 防抖，支持四个方向配置
   - `cursor_mapping.rs`：实现触发边缘坐标按比例映射到对方屏幕（左右布局映射 Y，上下布局映射 X），控制权归还时反向映射
   - _需求：4.2、4.5、4.6、4.7、4.8、4.9、5.1、5.2、5.3、5.4、6.1、6.2_

- [ ] 5. 实现 kvm-core 剪贴板与修饰键同步（`clipboard.rs` + `state.rs` 扩展）
   - `clipboard.rs`：实现控制权归还时自动同步剪贴板，支持纯文本，超过 10MB 跳过，可通过配置关闭
   - `state.rs` 扩展：控制权切换时检查并发送修饰键释放事件，事件帧中包含修饰键状态位图，被控方校验并自动修正
   - _需求：11.1、11.2、11.3、11.4、12.1、12.2、12.3、12.4、12.5、13.1、13.2、13.3、13.4、13.5_

- [ ] 6. 实现 Windows 平台适配层（`kvm-win/src/platform/`）
   - `screen.rs`：Win32 `EnumDisplayMonitors` 枚举所有显示器（分辨率、位置、DPI），实现 `ScreenProvider` trait，设置 DPI Awareness Per-Monitor V2
   - `input_capture.rs`：`SetWindowsHookEx` 全局键鼠 Hook，实现 `InputCapture` trait
   - `input_inject.rs`：`SendInput` 模拟键鼠，实现 `InputExecutor` trait，本地键码与统一键码双向转换
   - `input_block.rs`：Hook + `BlockInput` 屏蔽/恢复键鼠，实现 `InputBlocker` trait
   - `hotkey.rs`：`RegisterHotKey` 全局快捷键，实现 `HotkeyManager` trait
   - `clipboard.rs`：Win32 Clipboard API，实现 `ClipboardProvider` trait
   - `cursor.rs`：`ShowCursor` + `SetCursorPos`，实现 `CursorController` trait
   - _需求：1.4、7.3、7.4、7.5、9.1、9.2、9.3、9.4_

- [ ] 7. 实现 Mac 平台适配层（`kvm-mac/src/platform/`）
   - `screen.rs`：CoreGraphics 枚举显示器，实现 `ScreenProvider` trait
   - `input_capture.rs`：`CGEventTap` 键鼠采集，实现 `InputCapture` trait
   - `input_inject.rs`：`CGEvent` 键鼠注入，实现 `InputExecutor` trait，本地键码与统一键码双向转换
   - `input_block.rs`：`CGEventTap` 屏蔽键鼠，实现 `InputBlocker` trait
   - `hotkey.rs`：`NSEvent` 全局快捷键，实现 `HotkeyManager` trait
   - `clipboard.rs`：`NSPasteboard` 剪贴板读写，实现 `ClipboardProvider` trait
   - `cursor.rs`：`NSCursor` 光标显示/隐藏/位置，实现 `CursorController` trait
   - `permission.rs`：辅助功能权限检测，未授权时禁用键鼠功能并引导用户授权
   - _需求：1.4、15.1、15.2、15.3、15.4、15.5_

- [ ] 8. 实现两端应用入口与配置持久化（`kvm-win/src/main.rs` + `kvm-mac/src/main.rs`）
   - 两端 `main.rs`：组装 kvm-core + 平台适配层，启动网络监听，注册快捷键，启动心跳，注册 panic hook 确保键鼠恢复
   - `config_store.rs`（两端各自）：Win 端存 `%APPDATA%/KVMini/config.json`，Mac 端存 `~/Library/Application Support/KVMini/config.json`，配置损坏时使用默认值
   - `autostart.rs`（两端各自）：Win 端注册表实现，Mac 端 LaunchAgent plist 实现，自启动时以托盘模式启动
   - 优雅退出：按顺序恢复键鼠 → 恢复光标 → 通知对端 → 关闭连接 → 取消快捷键 → 停止剪贴板监听
   - _需求：16.1、16.2、16.3、16.4、16.5、16.6、17.1、17.2、17.3、17.4、17.5、19.1、19.2、19.3、19.4、19.5_

- [ ] 9. 重构 Windows 端 UI（`kvm-win/src/ui/` + Tauri 前端）
   - 移除"当前角色"标签、角色名称、"切换主控/被控"按钮
   - 连接状态展示：未连接 / 已连接·各自独立 / 正在控制对方 / 对方正在控制本机
   - 连接入口：扫描设备（UDP 发现，等待 5 秒）+ 手动输入 IP
   - 设置面板：屏幕布局（本机在左/右/上/下）、连接密码、剪贴板共享开关、开机自启动开关、日志级别
   - 多显示器布局预览：按真实物理排列等比例渲染本机多显示器，支持拖拽调整本机与远端的相对位置
   - 托盘图标颜色：绿色=Idle，蓝色=LocalControlling，橙色=RemoteControlling
   - _需求：21.1、21.2、21.3、21.4、21.5、21.6、21.7、21.9_

- [ ] 10. 实现 Mac 端托盘 UI（`kvm-mac/src/tray.rs`）
   - `NSStatusBar` 托盘菜单：显示连接状态、控制状态、权限状态
   - 菜单项：连接/断开、扫描设备、手动输入 IP、设置、退出
   - 不显示任何"主控端/被控端"角色相关内容
   - _需求：21.1、21.2、21.3、21.5、21.6、21.8_

- [ ] 11. 实现日志系统与安全认证（`kvm-core/src/log.rs` + `auth.rs`）
   - `log.rs`：定义统一日志 trait，各端接入日志后端，输出时间戳 + 级别 + 模块名 + 消息，支持 debug/info/warn/error 级别配置
   - `auth.rs`：连接密码验证逻辑，TLS 加密配置，密码错误时拒绝连接，修改密码时断开重连
   - _需求：14.1、14.2、14.3、14.4、20.1、20.2、20.3、20.4_
