# 需求文档：KVMini 完整实现（UI Bug 修复 + 核心功能）

## 引言

KVMini 是一款基于 Tauri（Rust + WebView）的跨平台键鼠共享软件，目标是让 Mac 和 Windows 两台设备共享同一套键盘鼠标。

本文档整合了所有历史规划中**尚未实现**的需求，目标是让 KVMini 从"能打开"变成"真正能用"——即 Mac 和 Windows 两台设备可以通过局域网共享键鼠，并修复当前界面中存在的 Bug。

### 当前已实现的功能（不在本文档范围内）

- ✅ TCP 连接（握手、心跳、断开）
- ✅ 协议编解码（KvmFrame 二进制协议）
- ✅ 边缘检测逻辑（EdgeDetector、SwitchManager）
- ✅ 设置持久化（settings.json、history.json）
- ✅ 网卡枚举、监听地址配置
- ✅ 辅助功能权限检测（macOS）
- ✅ MockCapturer / MockInjector（测试用）
- ✅ HotkeyDetector（键码检测逻辑）

### 本文档目标（全部未实现）

- ❌ UI Bug 修复（连接失败 undefined、i18n 占位符、按钮语义、默认快捷键显示）
- ❌ macOS CGEvent Tap 真实捕获与注入
- ❌ Windows SetWindowsHookEx 真实捕获与 SendInput 注入
- ❌ 跨平台键码映射（CGKeyCode ↔ VirtualKey）
- ❌ 事件循环串联（捕获 → TCP → 注入）
- ❌ 多显示器支持
- ❌ 剪贴板共享（Mac ↔ Windows）
- ❌ TLS 1.3 加密传输

---

## 需求

### 需求 1：修复"连接失败: undefined"错误

**用户故事：** 作为用户，我希望连接失败时能看到有意义的错误信息，以便知道原因并知道如何解决。

#### 验收标准

1. WHEN 用户点击"连接"按钮且连接失败 THEN 系统 SHALL 显示具体错误原因（如"目标设备不在线"、"端口被拒绝"、"网络不可达"），而非 `undefined`
2. WHEN 后端返回的错误对象为 `null` 或 `undefined` 时 THEN 前端 SHALL 显示兜底文案"连接失败，请检查目标设备是否在线"
3. WHEN 连接超时时 THEN 系统 SHALL 显示"连接超时，请检查 IP 地址和端口是否正确"
4. WHEN 手动连接输入框为空时点击连接 THEN 系统 SHALL 显示"请输入目标设备 IP 地址"，不发起连接请求

### 需求 2：修复 i18n 占位符未翻译问题

**用户故事：** 作为用户，我希望界面上所有文字都显示正确的语言，而不是 i18n key。

#### 验收标准

1. WHEN 应用语言为中文时 THEN 手动连接输入框 SHALL 显示"请输入目标设备 IP 地址"作为占位符
2. WHEN 应用语言为英文时 THEN 手动连接输入框 SHALL 显示"Enter target device IP address"作为占位符
3. WHEN 任何 i18n key 在翻译文件中缺失时 THEN 系统 SHALL 在控制台输出警告，并显示 key 本身
4. IF 翻译文件中存在 `manualIpPlaceholder` key THEN 系统 SHALL 确保中英文翻译均已填写

### 需求 3：澄清两个"连接"按钮的语义

**用户故事：** 作为用户，我希望界面上的按钮语义清晰，知道每个"连接"按钮的作用。

#### 验收标准

1. WHEN 用户查看主界面时 THEN 顶部"连接"按钮 SHALL 改为"扫描并连接"或移除（功能与下方设备列表重复）
2. WHEN 设备列表中有可用设备时 THEN 每个设备旁的"连接"按钮 SHALL 点击后直接连接该设备
3. WHEN 手动连接区域的"连接"按钮被点击时 THEN 系统 SHALL 使用输入框中的 IP 和端口发起连接
4. WHEN 已有连接时 THEN 顶部状态区域 SHALL 清晰显示"已连接到 [设备名/IP]"

### 需求 4：默认快捷键在设置页正确显示

**用户故事：** 作为用户，我希望打开设置页时能看到当前生效的快捷键（包括默认值），以便知道如何切换控制权。

#### 验收标准

1. WHEN 用户首次打开设置页且未设置过快捷键 THEN 快捷键输入框 SHALL 显示默认值 `Ctrl+Alt+S`
2. WHEN `get_settings()` 返回的 `hotkey` 字段为空字符串或 `null` 时 THEN 前端 SHALL 自动填充默认值 `Ctrl+Alt+S`
3. WHEN 用户清空快捷键输入框并保存 THEN 系统 SHALL 提示"快捷键已清空，切换控制权功能将不可用"
4. WHEN 用户修改快捷键后保存 THEN 系统 SHALL 立即注册新快捷键，并在输入框中显示新值

### 需求 5：macOS CGEvent Tap 键鼠捕获实现

**用户故事：** 作为 Mac 主控端用户，我希望软件能捕获我的键盘和鼠标输入并转发给被控端，以便实现跨设备控制。

#### 验收标准

1. WHEN 本机角色为"主控端"且连接已建立 THEN 系统 SHALL 在专用子线程中启动 CGEvent Tap，捕获全局键鼠事件
2. WHEN CGEvent Tap 启动时辅助功能权限未授权 THEN 系统 SHALL 返回 `CapturerError::PermissionDenied` 并通过 Tauri 事件通知前端
3. WHEN 捕获到鼠标移动事件 THEN 系统 SHALL 以最高 1000Hz 节流后发送 `KvmFrame::MouseMove`（归一化坐标）
4. WHEN 捕获到键盘/鼠标按键/滚轮事件 THEN 系统 SHALL 分别发送对应的 `KvmFrame`
5. WHEN `set_intercept(true)` 被调用 THEN 系统 SHALL 拦截事件不传递给本机
6. WHEN CGEvent Tap 被系统自动禁用时 THEN 系统 SHALL 自动重新启用，最多重试 3 次
7. WHEN 权限检测时 THEN 系统 SHALL 使用 `CGPreflightListenEventAccess()` 避免 TCC 缓存延迟
8. WHEN CGEvent Tap 在子线程中运行时 THEN 系统 SHALL 在同一子线程调用 `CFRunLoopRun()` 阻塞驱动事件循环（不能在主线程）

### 需求 6：macOS CGEvent 键鼠注入实现

**用户故事：** 作为 Mac 被控端用户，我希望软件能将收到的键鼠事件注入到本机，以便实现远程控制。

#### 验收标准

1. WHEN 收到 `KvmFrame::MouseMove` THEN 系统 SHALL 调用 `CGWarpMouseCursorPosition` 移动光标（处理 Retina 逻辑像素转换）
2. WHEN 收到 `KvmFrame::MouseButton` THEN 系统 SHALL 调用 `CGEvent::new_mouse_event` 注入鼠标按键
3. WHEN 收到 `KvmFrame::MouseScroll` THEN 系统 SHALL 调用 `CGEvent::new_scroll_event` 注入滚轮
4. WHEN 收到 `KvmFrame::KeyEvent` THEN 系统 SHALL 调用 `CGEvent::new_keyboard_event` 注入键盘事件
5. WHEN 收到 `KvmFrame::SwitchControl` THEN 系统 SHALL 将光标定位到对应边缘位置

### 需求 7：Windows SetWindowsHookEx 键鼠捕获实现

**用户故事：** 作为 Windows 主控端用户，我希望软件能捕获键鼠输入并转发给 Mac 被控端。

#### 验收标准

1. WHEN 启动捕获 THEN 系统 SHALL 在专用线程中安装 `WH_MOUSE_LL` 和 `WH_KEYBOARD_LL` 低级钩子
2. WHEN 捕获到事件 THEN 系统 SHALL 通过 channel 发送对应的 `KvmFrame`（钩子回调中仅做 `try_send()`，不执行耗时操作，确保 300ms 内返回）
3. WHEN `set_intercept(true)` 被调用 THEN 系统 SHALL 在钩子回调中返回非零值拦截事件
4. WHEN `stop()` 被调用 THEN 系统 SHALL 调用 `UnhookWindowsHookEx` 并退出消息循环
5. WHEN 启用 winapi 依赖时 THEN 系统 SHALL 在 `Cargo.toml` 中启用 `winapi = { version = "0.3", features = ["winuser", "winbase"] }`

### 需求 8：Windows SendInput 键鼠注入实现

**用户故事：** 作为 Windows 被控端用户，我希望软件能将收到的键鼠事件注入到本机。

#### 验收标准

1. WHEN 收到 `KvmFrame::MouseMove` THEN 系统 SHALL 调用 `SendInput` 以 `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` 移动光标
2. WHEN 收到 `KvmFrame::MouseButton` THEN 系统 SHALL 调用 `SendInput` 注入鼠标按键事件
3. WHEN 收到 `KvmFrame::MouseScroll` THEN 系统 SHALL 调用 `SendInput` 以 `MOUSEEVENTF_WHEEL` 注入滚轮
4. WHEN 收到 `KvmFrame::KeyEvent` THEN 系统 SHALL 调用 `SendInput` 以 `KEYBDINPUT` 注入键盘事件（使用映射后的 VirtualKey）
5. WHEN Windows DPI 缩放不为 100% 时 THEN 系统 SHALL 使用 `GetSystemMetrics(SM_CXVIRTUALSCREEN)` 正确处理坐标映射（0-65535 范围）
6. WHEN 进程启动时 THEN 系统 SHALL 调用 `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` 确保 DPI 感知

### 需求 9：跨平台键码映射

**用户故事：** 作为用户，我希望在 Mac 上按下的键能在 Windows 上正确重现，以便跨平台键盘操作无误。

#### 验收标准

1. WHEN macOS 发送 `KvmFrame::KeyEvent` 时 THEN 协议层 SHALL 携带平台标识（`platform: "macos"` 或 `"windows"`）
2. WHEN Windows 收到来自 macOS 的 `KeyEvent` 时 THEN 系统 SHALL 通过映射表将 `CGKeyCode` 转换为 `VirtualKey`
3. WHEN macOS 收到来自 Windows 的 `KeyEvent` 时 THEN 系统 SHALL 通过映射表将 `VirtualKey` 转换为 `CGKeyCode`
4. WHEN 映射表中不存在对应键码时 THEN 系统 SHALL 丢弃该事件并记录警告日志，不崩溃
5. WHEN 映射常用键时 THEN 系统 SHALL 正确映射字母 A-Z、数字 0-9、F1-F12、方向键、修饰键（Ctrl/Alt/Shift/Win/Cmd）
6. WHEN macOS 的 `Command` 键被按下时 THEN 系统 SHALL 默认映射为 Windows 的 `Win` 键

### 需求 10：事件循环串联（捕获 → TCP 传输 → 注入）

**用户故事：** 作为用户，我希望连接建立后主控端的键鼠操作能实时传输并在被控端执行，以便真正实现跨设备控制。

#### 验收标准

1. WHEN TCP 连接建立成功 THEN 系统 SHALL 根据本机角色自动启动捕获器（主控端）或注入器（被控端）
2. WHEN 主控端捕获到事件 THEN 系统 SHALL 将 `KvmFrame` 经键码映射后通过 TCP 发送给被控端
3. WHEN 被控端收到 `KvmFrame` THEN 系统 SHALL 经键码映射后调用注入器执行
4. WHEN 边缘检测触发切换 THEN 系统 SHALL 发送 `KvmFrame::SwitchControl`，主控端停止拦截，被控端开始接管
5. WHEN TCP 连接断开 THEN 系统 SHALL 停止捕获器，恢复本机输入控制权

### 需求 11：多显示器支持

**用户故事：** 作为使用多台显示器的用户，我希望鼠标能在多个屏幕之间自然地跨设备移动，而不仅限于主显示器边缘。

#### 验收标准

1. WHEN 新增 Tauri 命令 `get_monitors()` THEN 系统 SHALL 返回 `Vec<MonitorInfo>` 供前端布局配置界面调用
2. WHEN macOS 枚举显示器时 THEN 系统 SHALL 使用 `CGGetActiveDisplayList` 获取所有活跃显示器
3. WHEN Windows 枚举显示器时 THEN 系统 SHALL 使用 `EnumDisplayMonitors` 获取所有显示器
4. WHEN 鼠标到达任意显示器边缘时 THEN 系统 SHALL 根据布局配置判断是否触发跨设备切换（而非仅检测主显示器）
5. WHEN 鼠标坐标归一化时 THEN 系统 SHALL 基于虚拟桌面总尺寸（所有显示器包围盒）进行归一化
6. WHEN 显示器配置变化时 THEN 系统 SHALL 通过 Tauri 事件 `monitors-changed` 通知前端更新布局

### 需求 12：剪贴板共享（Mac ↔ Windows 双向同步）

**用户故事：** 作为用户，我希望在 Mac 上复制的内容能直接粘贴到 Windows 上（反之亦然），以便跨设备工作时无需借助其他工具传递文本。

#### 验收标准

1. WHEN 主控端用户执行复制操作 THEN 系统 SHALL 在 500ms 内将剪贴板文本通过 TCP 发送给被控端（`KvmFrame::Clipboard { text }`）
2. WHEN 被控端收到 `KvmFrame::Clipboard` THEN 系统 SHALL 将文本写入本机剪贴板
3. WHEN 剪贴板内容超过 1MB 时 THEN 系统 SHALL 截断并记录警告，不发送超大内容
4. WHEN 剪贴板内容为非文本类型时 THEN 系统 SHALL 仅同步纯文本部分，忽略二进制内容
5. WHEN macOS 监听剪贴板变化时 THEN 系统 SHALL 使用轮询方式（每 200ms 检查 `NSPasteboard.changeCount`）
6. WHEN Windows 监听剪贴板变化时 THEN 系统 SHALL 使用 `AddClipboardFormatListener` 注册窗口消息监听
7. WHEN 收到远端剪贴板写入本机时 THEN 系统 SHALL 使用标志位防止循环同步
8. WHEN 用户在设置中关闭剪贴板共享 THEN 系统 SHALL 停止监听和发送剪贴板变化

### 需求 13：TLS 1.3 加密传输

**用户故事：** 作为用户，我希望键鼠数据和剪贴板内容在网络传输时是加密的，以便在局域网中不担心数据（包括密码输入）被窃听。

#### 验收标准

1. WHEN 两端建立连接时 THEN 系统 SHALL 使用 TLS 1.3 加密所有 TCP 流量
2. WHEN 首次连接时 THEN 系统 SHALL 采用 TOFU 模式：展示对端证书指纹，用户确认后保存，后续自动验证
3. WHEN 证书指纹不匹配时 THEN 系统 SHALL 拒绝连接并弹出安全警告
4. WHEN 用户在设置中关闭加密 THEN 系统 SHALL 允许降级为明文连接，但显示安全警告
5. WHEN 首次启动时 THEN 系统 SHALL 使用 `rcgen` 自动生成 RSA-2048 自签名证书，保存到应用数据目录
6. WHEN TLS 握手失败时 THEN 系统 SHALL 返回具体错误原因并通过 Tauri 事件通知前端
7. WHEN 使用 TLS 时 THEN 系统 SHALL 使用 `rustls` crate（纯 Rust 实现，无 OpenSSL 依赖）
