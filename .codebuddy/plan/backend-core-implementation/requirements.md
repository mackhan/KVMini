# 需求文档：KVMini 后端核心功能完整实现

## 引言

KVMini 是一款基于 Tauri（Rust + WebView）的跨平台键鼠共享软件，目标是让 Mac 和 Windows 两台设备共享同一套键盘鼠标。

### 后端运行架构说明

KVMini 的后端是 Tauri 内嵌的 Rust 进程，**不是独立的服务**，而是随 Tauri 应用一起启动的。具体架构如下：

```
┌─────────────────────────────────────────────────────┐
│  KVM Share.app（Tauri 进程）                          │
│  ┌──────────────┐    ┌──────────────────────────┐   │
│  │  WebView UI  │◄──►│  Rust 后端（同进程）       │   │
│  │  (前端 JS)   │    │  - TCP Server/Client      │   │
│  └──────────────┘    │  - CGEvent Tap（macOS）   │   │
│                      │  - SendInput（Windows）   │   │
│                      │  - 边缘检测 + 切换逻辑    │   │
│                      └──────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

前端通过 `invoke()` 调用后端 Tauri 命令，后端通过 `emit_all()` 向前端推送事件。

### 当前已实现的功能

- ✅ TCP 连接（握手、心跳、断开）
- ✅ 协议编解码（KvmFrame 二进制协议）
- ✅ 边缘检测逻辑（EdgeDetector、SwitchManager）
- ✅ 设置持久化（settings.json、history.json、listen_config.json）
- ✅ 网卡枚举、监听地址配置
- ✅ 辅助功能权限检测（macOS）
- ✅ 托盘菜单事件转发
- ✅ MockCapturer / MockInjector（测试用）
- ✅ HotkeyDetector（Ctrl+Alt+S 键码检测逻辑）

### 当前未实现的功能（本需求文档的目标）

- ❌ macOS CGEvent Tap 真实捕获（`capturer.rs` 中为 TODO）
- ❌ macOS CGEvent 真实注入（`injector.rs` 中为 TODO）
- ❌ Windows SetWindowsHookEx 真实捕获（`capturer.rs` 中为 TODO）
- ❌ Windows SendInput 真实注入（`injector.rs` 中为 TODO）
- ❌ 跨平台键码映射（CGKeyCode ↔ VirtualKey 转换表）**【新增】**
- ❌ 快捷键冲突检测（新增需求）
- ❌ 默认快捷键 `Ctrl+Alt+S` 未在前端显示
- ❌ 托盘菜单文字硬编码中文，未响应语言设置
- ❌ 捕获器与注入器未与 TCP 传输层联动（事件循环未串联）
- ❌ 剪贴板共享（Mac ↔ Windows 双向同步）**【新增】**
- ❌ 加密传输（TLS 1.3 保护键鼠数据和剪贴板内容）**【新增】**
- ❌ 多显示器支持（多屏布局配置与跨屏边缘切换）**【新增】**

---

## 需求

### 需求 1：macOS CGEvent Tap 键鼠捕获实现

**用户故事：** 作为 Mac 主控端用户，我希望软件能捕获我的键盘和鼠标输入，以便将其转发给 Windows 被控端。

#### 验收标准

1. WHEN 用户将本机设为"主控端"角色 THEN 系统 SHALL 在专用线程中启动 CGEvent Tap，捕获全局键鼠事件
2. WHEN CGEvent Tap 启动时辅助功能权限未授权 THEN 系统 SHALL 返回 `CapturerError::PermissionDenied`，并通过 Tauri 事件 `capturer-error` 通知前端弹出权限引导
3. WHEN 捕获到鼠标移动事件 THEN 系统 SHALL 以最高 1000Hz 节流后通过 channel 发送 `KvmFrame::MouseMove`（归一化坐标 * 10000）
4. WHEN 捕获到键盘按下/释放事件 THEN 系统 SHALL 发送 `KvmFrame::KeyEvent`（CGKeyCode）
5. WHEN 捕获到鼠标按键事件 THEN 系统 SHALL 发送 `KvmFrame::MouseButton`
6. WHEN 捕获到鼠标滚轮事件 THEN 系统 SHALL 发送 `KvmFrame::MouseScroll`
7. WHEN `set_intercept(true)` 被调用 THEN 系统 SHALL 拦截事件（不传递给本机），`set_intercept(false)` 时透传
8. WHEN `stop()` 被调用 THEN 系统 SHALL 停止 CGEvent Tap RunLoop 并释放资源
9. WHEN CGEvent Tap 被系统自动禁用（超时未处理事件）THEN 系统 SHALL 检测到并重新启用，最多重试 3 次
10. WHEN 权限检测时 THEN 系统 SHALL 优先使用 `CGPreflightListenEventAccess()` 而非 `AXIsProcessTrustedWithOptions`，避免 TCC 缓存延迟导致授权后仍检测为未授权

### 需求 2：macOS CGEvent 键鼠注入实现

**用户故事：** 作为 Mac 被控端用户，我希望软件能将收到的键鼠事件注入到本机，以便实现远程控制。

#### 验收标准

1. WHEN 收到 `KvmFrame::MouseMove { x, y }` THEN 系统 SHALL 调用 `CGWarpMouseCursorPosition` 将光标移动到对应屏幕像素坐标
2. WHEN 收到 `KvmFrame::MouseButton` THEN 系统 SHALL 调用 `CGEvent::new_mouse_event` 注入鼠标按键事件
3. WHEN 收到 `KvmFrame::MouseScroll` THEN 系统 SHALL 调用 `CGEvent::new_scroll_event` 注入滚轮事件
4. WHEN 收到 `KvmFrame::KeyEvent` THEN 系统 SHALL 调用 `CGEvent::new_keyboard_event` 注入键盘事件
5. WHEN 收到 `KvmFrame::SwitchControl` THEN 系统 SHALL 调用 `CGWarpMouseCursorPosition` 将光标定位到对应边缘位置
6. IF 辅助功能权限未授权 THEN 系统 SHALL 返回 `InjectorError::PermissionDenied`
7. WHEN 注入鼠标移动时 THEN 系统 SHALL 将归一化坐标（* 10000）正确转换为屏幕像素坐标（考虑 Retina 屏幕逻辑像素）

### 需求 3：Windows SendInput 键鼠注入实现

**用户故事：** 作为 Windows 被控端用户，我希望软件能将收到的键鼠事件注入到本机。

#### 验收标准

1. WHEN 收到 `KvmFrame::MouseMove` THEN 系统 SHALL 调用 `SendInput` 以 `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` 标志移动光标
2. WHEN 收到 `KvmFrame::MouseButton` THEN 系统 SHALL 调用 `SendInput` 以对应的 `MOUSEEVENTF_LEFTDOWN/UP/RIGHTDOWN/UP` 标志
3. WHEN 收到 `KvmFrame::MouseScroll` THEN 系统 SHALL 调用 `SendInput` 以 `MOUSEEVENTF_WHEEL` 标志
4. WHEN 收到 `KvmFrame::KeyEvent` THEN 系统 SHALL 调用 `SendInput` 以 `KEYBDINPUT` 结构体注入键盘事件（使用映射后的 VirtualKey）
5. WHEN Windows DPI 缩放不为 100% THEN 系统 SHALL 正确处理 DPI 缩放：使用 `GetSystemMetrics(SM_CXVIRTUALSCREEN)` 获取虚拟屏幕总宽度，坐标乘以 65535 后除以虚拟屏幕宽度，确保 `MOUSEEVENTF_ABSOLUTE` 坐标在 0-65535 范围内正确映射
6. WHEN 收到 `KvmFrame::SwitchControl` THEN 系统 SHALL 将光标定位到对应边缘位置（使用 `SetCursorPos`）
7. WHEN `Cargo.toml` 中 winapi 依赖被注释 THEN 系统 SHALL 启用 `winapi = { version = "0.3", features = ["winuser", "winbase"] }` 依赖

### 需求 4：Windows SetWindowsHookEx 键鼠捕获实现

**用户故事：** 作为 Windows 主控端用户，我希望软件能捕获键鼠输入并转发给 Mac 被控端。

#### 验收标准

1. WHEN 启动捕获 THEN 系统 SHALL 在专用线程中调用 `SetWindowsHookEx(WH_MOUSE_LL, ...)` 和 `SetWindowsHookEx(WH_KEYBOARD_LL, ...)` 安装低级钩子
2. WHEN 捕获到事件 THEN 系统 SHALL 通过 channel 发送对应的 `KvmFrame`
3. WHEN `stop()` 被调用 THEN 系统 SHALL 调用 `UnhookWindowsHookEx` 卸载钩子并退出消息循环
4. WHEN `set_intercept(true)` 被调用 THEN 系统 SHALL 在钩子回调中返回非零值以拦截事件（不传递给其他程序）
5. WHEN 钩子线程启动失败 THEN 系统 SHALL 返回 `CapturerError::SystemError`
6. WHEN 钩子回调处理时间超过 200ms THEN 系统 SHALL 立即将事件放入 channel 后返回，不在回调中执行任何耗时操作（避免系统自动卸载钩子，Windows 超时阈值约 300ms）

### 需求 5：事件循环串联（捕获 → 传输 → 注入）

**用户故事：** 作为用户，我希望在连接建立后，主控端的键鼠操作能实时传输并在被控端执行，以便真正实现跨设备控制。

#### 验收标准

1. WHEN TCP 连接建立成功 THEN 系统 SHALL 根据本机角色自动启动捕获器（主控端）或注入器（被控端）
2. WHEN 主控端捕获到事件 THEN 系统 SHALL 将 `KvmFrame` 通过 TCP 连接发送给被控端
3. WHEN 被控端收到 `KvmFrame` THEN 系统 SHALL 调用注入器将事件注入本机
4. WHEN 边缘检测触发切换 THEN 系统 SHALL 发送 `KvmFrame::SwitchControl`，主控端停止拦截本机输入，被控端开始接管
5. WHEN TCP 连接断开 THEN 系统 SHALL 停止捕获器，恢复本机输入控制权

### 需求 6：切换控制焦点默认快捷键

**用户故事：** 作为用户，我希望有一个默认的快捷键来切换键鼠控制权，以便不需要移动鼠标到屏幕边缘也能切换。

#### 验收标准

1. WHEN 应用首次启动且用户未设置快捷键 THEN 系统 SHALL 使用默认快捷键 `Ctrl+Alt+S`
2. WHEN `get_settings()` 返回空 hotkey 时 THEN 前端 SHALL 在快捷键输入框中显示默认值 `Ctrl+Alt+S`
3. WHEN 用户在设置中修改快捷键 THEN 系统 SHALL 保存新快捷键并重新注册
4. WHEN 快捷键被触发 THEN 系统 SHALL 切换控制权（本机 ↔ 远端），并通过 Tauri 事件 `hotkey-switch` 通知前端更新状态
5. WHEN 快捷键设置为空 THEN 系统 SHALL 禁用快捷键功能

### 需求 7：托盘菜单国际化与状态联动

**用户故事：** 作为用户，我希望任务栏菜单的文字跟随语言设置，且菜单项状态与连接状态同步。

#### 验收标准

1. WHEN 应用语言为英文时 THEN 系统 SHALL 托盘菜单显示英文（Show Window / Connect / Disconnect / Quit）
2. WHEN 应用语言为中文时 THEN 系统 SHALL 托盘菜单显示中文（显示窗口 / 连接设备 / 断开连接 / 退出）
3. WHEN 用户点击托盘菜单"连接设备" THEN 系统 SHALL 通过 Tauri 事件 `tray-connect` 通知前端打开连接面板
4. WHEN 用户点击托盘菜单"断开连接" THEN 系统 SHALL 通过 Tauri 事件 `tray-disconnect` 通知前端断开连接
5. WHEN 连接状态变化时 THEN 系统 SHALL 更新托盘菜单项的启用/禁用状态（已连接时"连接设备"置灰）

### 需求 8：快捷键系统冲突检测

**用户故事：** 作为用户，我希望在设置快捷键时能检测到与系统快捷键的冲突，以便避免快捷键失效或干扰其他应用。

#### 验收标准

1. WHEN 用户在设置界面输入快捷键组合 THEN 系统 SHALL 调用后端 `check_hotkey_conflict` 命令检测冲突
2. WHEN 快捷键与 macOS 系统保留快捷键冲突（如 `Cmd+Space`、`Cmd+Tab`、`Ctrl+Space` 等）THEN 系统 SHALL 返回冲突类型 `"system"` 和冲突说明
3. WHEN 快捷键与 Windows 系统保留快捷键冲突（如 `Win+L`、`Ctrl+Alt+Del`、`Alt+F4` 等）THEN 系统 SHALL 返回冲突类型 `"system"` 和冲突说明
4. WHEN 快捷键与当前已注册的全局快捷键冲突 THEN 系统 SHALL 返回冲突类型 `"registered"`
5. WHEN 快捷键无冲突 THEN 系统 SHALL 返回 `{ conflict: false }`
6. WHEN 前端收到冲突结果 THEN 系统 SHALL 在快捷键输入框下方显示警告文字（不阻止保存，仅提示）
7. WHEN 用户保存有冲突的快捷键 THEN 系统 SHALL 弹出确认对话框，用户确认后仍可保存

### 需求 9：完整的端到端集成测试

**用户故事：** 作为开发者，我希望有完整的测试覆盖核心功能，以便在修改代码时快速发现回归问题。

#### 验收标准

1. WHEN 运行 `cargo test` THEN 系统 SHALL 所有测试通过，无 panic 或 timeout
2. WHEN 测试 macOS 捕获器 THEN 系统 SHALL 使用 MockCapturer 验证事件捕获和节流逻辑（不依赖真实系统 API）
3. WHEN 测试 macOS 注入器 THEN 系统 SHALL 使用 MockInjector 验证坐标转换和事件分发（不依赖真实系统 API）
4. WHEN 测试 Windows 注入器 THEN 系统 SHALL 使用 MockInjector 验证 DPI 缩放和 SendInput 坐标转换
5. WHEN 测试切换焦点快捷键 THEN 系统 SHALL 验证默认快捷键 `Ctrl+Alt+S` 在 macOS 和 Windows 键码下均能触发
6. WHEN 测试快捷键冲突检测 THEN 系统 SHALL 验证 macOS 和 Windows 各自的系统保留快捷键均被正确识别
7. WHEN 测试事件循环串联 THEN 系统 SHALL 使用 MockCapturer + MockInjector 验证端到端事件流（捕获 → channel → TCP 发送 → TCP 接收 → 注入）
8. WHEN 测试跨平台握手 THEN 系统 SHALL 验证 macOS ↔ Windows 握手成功，版本不兼容时返回正确错误
9. WHEN 测试键码映射 THEN 系统 SHALL 验证 CGKeyCode → VirtualKey 和 VirtualKey → CGKeyCode 的双向转换正确性

### 需求 10：跨平台键码映射（新增）

**用户故事：** 作为用户，我希望在 Mac 上按下的键能在 Windows 上正确重现，以便跨平台键盘操作无误。

#### 验收标准

1. WHEN macOS 主控端发送 `KvmFrame::KeyEvent { keycode }` 时 THEN 协议层 SHALL 携带平台标识（`platform: "macos"` 或 `platform: "windows"`）
2. WHEN Windows 被控端收到来自 macOS 的 `KeyEvent` 时 THEN 系统 SHALL 通过键码映射表将 `CGKeyCode` 转换为对应的 Windows `VirtualKey`
3. WHEN macOS 被控端收到来自 Windows 的 `KeyEvent` 时 THEN 系统 SHALL 通过键码映射表将 `VirtualKey` 转换为对应的 `CGKeyCode`
4. WHEN 映射表中不存在对应键码时 THEN 系统 SHALL 丢弃该事件并记录警告日志，不崩溃
5. WHEN 映射常用键时 THEN 系统 SHALL 正确映射以下键：字母 A-Z、数字 0-9、功能键 F1-F12、方向键、Enter、Backspace、Tab、Escape、Space、修饰键（Ctrl/Alt/Shift/Win/Cmd）
6. WHEN macOS 的 `Command` 键被按下时 THEN 系统 SHALL 映射为 Windows 的 `Win` 键（可在设置中配置为 `Ctrl`）

---

### 需求 11：剪贴板共享（Mac ↔ Windows 双向同步）

**用户故事：** 作为用户，我希望在 Mac 上复制的内容能直接粘贴到 Windows 上（反之亦然），以便跨设备工作时无需借助其他工具传递文本。

#### 验收标准

1. WHEN 主控端用户执行复制操作（Cmd+C / Ctrl+C）THEN 系统 SHALL 在 500ms 内将剪贴板文本内容通过 TCP 连接发送给被控端（`KvmFrame::Clipboard { text }`）
2. WHEN 被控端收到 `KvmFrame::Clipboard` THEN 系统 SHALL 将文本写入本机剪贴板，用户可直接粘贴
3. WHEN 剪贴板内容超过 1MB 时 THEN 系统 SHALL 截断并记录警告，不发送超大内容（防止网络阻塞）
4. WHEN 剪贴板内容为非文本类型（图片、文件路径等）THEN 系统 SHALL 仅同步纯文本部分，忽略二进制内容
5. WHEN 用户在设置中关闭剪贴板共享 THEN 系统 SHALL 停止监听和发送剪贴板变化
6. WHEN macOS 监听剪贴板变化时 THEN 系统 SHALL 使用轮询方式（每 200ms 检查 `NSPasteboard.changeCount`），不使用系统通知（兼容性更好）
7. WHEN Windows 监听剪贴板变化时 THEN 系统 SHALL 使用 `AddClipboardFormatListener` 注册窗口消息监听，避免轮询
8. WHEN 剪贴板同步触发时 THEN 系统 SHALL 防止循环同步（收到远端剪贴板后设置本机时，不再触发向远端的同步）

### 需求 12：加密传输（TLS 1.3）

**用户故事：** 作为用户，我希望键鼠数据和剪贴板内容在网络传输时是加密的，以便在公司内网或不受信任的网络环境中使用时不担心数据被窃听。

> **加密是用来干什么的？**
> 键鼠共享软件传输的数据包含你的每一次按键（包括密码输入）和鼠标轨迹，如果在局域网中明文传输，同一网络下的其他设备可以通过抓包工具（如 Wireshark）完整还原你的操作，包括你输入的密码。加密传输（TLS）可以防止这种窃听。

#### 验收标准

1. WHEN 两端建立连接时 THEN 系统 SHALL 使用 TLS 1.3 加密所有 TCP 流量（包括键鼠事件和剪贴板内容）
2. WHEN 首次连接时 THEN 系统 SHALL 采用自签名证书 + 证书指纹验证（TOFU，Trust On First Use）模式：第一次连接时展示对端证书指纹，用户确认后保存，后续自动验证
3. WHEN 证书指纹不匹配时（可能遭受中间人攻击）THEN 系统 SHALL 拒绝连接并弹出安全警告
4. WHEN 用户在设置中关闭加密 THEN 系统 SHALL 允许降级为明文连接（兼容旧版本或特殊场景），但显示安全警告
5. WHEN 生成自签名证书时 THEN 系统 SHALL 使用 `rcgen` crate 在首次启动时自动生成 RSA-2048 证书，保存到应用数据目录
6. WHEN TLS 握手失败时 THEN 系统 SHALL 返回具体错误原因（证书过期、版本不兼容等）并通过 Tauri 事件通知前端
7. WHEN 使用 TLS 时 THEN 系统 SHALL 使用 `rustls` crate（纯 Rust 实现，无 OpenSSL 依赖，跨平台编译友好）

### 需求 13：多显示器支持

**用户故事：** 作为使用多台显示器的用户，我希望能配置每台显示器的相对位置，以便鼠标能在多个屏幕之间自然地跨设备移动。

#### 验收标准

1. WHEN 用户打开设置 THEN 系统 SHALL 显示多显示器布局配置界面，展示本机所有已连接显示器的相对位置
2. WHEN 用户配置显示器布局时 THEN 系统 SHALL 支持设置远端设备相对于本机的方位（左/右/上/下）以及对齐偏移量
3. WHEN 检测到本机有多台显示器时 THEN 系统 SHALL 枚举所有显示器信息（分辨率、DPI、相对位置）并通过 `get_monitors()` 命令返回给前端
4. WHEN 鼠标到达某台显示器的边缘时 THEN 系统 SHALL 根据布局配置判断是否触发跨设备切换（而非仅检测主显示器边缘）
5. WHEN macOS 枚举显示器时 THEN 系统 SHALL 使用 `CGGetActiveDisplayList` 获取所有活跃显示器
6. WHEN Windows 枚举显示器时 THEN 系统 SHALL 使用 `EnumDisplayMonitors` 获取所有显示器
7. WHEN 显示器配置发生变化（插拔显示器）时 THEN 系统 SHALL 重新枚举并通过 Tauri 事件 `monitors-changed` 通知前端更新布局配置界面
8. WHEN 鼠标坐标归一化时 THEN 系统 SHALL 基于虚拟桌面总尺寸（所有显示器的包围盒）进行归一化，而非单个显示器尺寸

---

### macOS CGEvent Tap 4 个失效根因及解决方案

这是实现中最容易出错的部分，必须严格遵守：

| 根因 | 症状 | 解决方案 |
|------|------|----------|
| **RunLoop 线程错误**（最常见） | Tap 创建成功但回调从不触发 | `CFRunLoopAddSource(CFRunLoopGetCurrent(), ...)` + 同线程调用 `CFRunLoopRun()` 阻塞 |
| **CFMachPort 被释放** | 快捷键时有时无 | 用静态变量持有引用，或 `CFRunLoopRun()` 保持线程存活 |
| **TCC 权限缓存延迟** | 授权后仍检测为未授权 | 优先用 `CGPreflightListenEventAccess()`，无缓存问题 |
| **只检测一次就放弃** | 重装后失效，重启也无效 | 权限未授予时持续轮询，授权后自动启动 |

**正确实现模式：**
```rust
// 必须在子线程中执行，不能在主线程
std::thread::spawn(|| {
    // 1. 等待权限（轮询）
    while !CGPreflightListenEventAccess() {
        std::thread::sleep(Duration::from_secs(1));
    }
    // 2. 创建 Tap
    let tap = CGEventTapCreate(...);
    // 3. 加到当前线程的 RunLoop（不是主线程）
    let source = CFMachPortCreateRunLoopSource(tap);
    CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
    CGEventTapEnable(tap, true);
    // 4. 阻塞当前线程驱动事件循环
    CFRunLoopRun(); // 必须在同一子线程调用
});
```

### Windows 低级钩子超时解决方案

Windows `WH_KEYBOARD_LL` / `WH_MOUSE_LL` 钩子回调必须在 **300ms** 内返回，否则系统自动卸载钩子。

**解决方案：**
- 钩子回调中**只做一件事**：将事件数据 `clone()` 后通过 `channel.try_send()` 发送，立即返回
- 所有实际处理（序列化、TCP 发送）在独立的工作线程中完成
- 使用有界 channel（容量 1024），防止内存无限增长
- 消息循环使用 `GetMessage` + `TranslateMessage` + `DispatchMessage`，不做任何额外工作

```rust
// 钩子回调（必须极快返回）
unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb_data = *(lparam as *const KBDLLHOOKSTRUCT);
        let _ = EVENT_SENDER.try_send(RawKeyEvent { vk: kb_data.vkCode, flags: kb_data.flags });
        // 立即返回，不做任何其他操作
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}
```

### Windows DPI 缩放处理

`MOUSEEVENTF_ABSOLUTE` 坐标范围是 **0-65535**，与屏幕分辨率无关，但需要正确映射：

```rust
// 正确的 DPI 感知坐标转换
fn normalized_to_absolute(norm_x: f32, norm_y: f32) -> (i32, i32) {
    // 使用虚拟屏幕尺寸（多显示器时为总宽高）
    let screen_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let screen_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    let abs_x = (norm_x * 65535.0 / screen_w as f32) as i32;
    let abs_y = (norm_y * 65535.0 / screen_h as f32) as i32;
    (abs_x, abs_y)
}
```

注意：需要调用 `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` 确保进程 DPI 感知，否则 `GetSystemMetrics` 返回的是缩放后的逻辑像素。

### 与竞品对比分析

#### 主要竞品

| 特性 | KVMini（本项目） | Barrier（开源免费） | Input Leap（开源免费） | Synergy（付费 $29-$49） | lan-mouse（开源免费） |
|------|-----------------|-------------------|----------------------|------------------------|----------------------|
| 技术栈 | Rust + Tauri | C++ + Qt | C++ + Qt | C++ + Qt | Rust |
| UI | 现代 WebView UI | 传统 Qt UI | 传统 Qt UI | 现代 Qt UI | 无 GUI（CLI） |
| 协议 | 自定义二进制 | Synergy 协议 | Synergy 协议 | Synergy 协议 | 自定义 |
| 延迟 | 目标 < 5ms | ~1-3ms | ~1-3ms | ~1-3ms | ~1-3ms |
| 跨平台 | macOS + Windows | macOS+Win+Linux | macOS+Win+Linux | macOS+Win+Linux | Linux 为主 |
| 快捷键冲突检测 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 剪贴板共享 | ✅（规划中） | ✅ | ✅ | ✅ | ❌ |
| 加密传输 | ✅（规划中，TLS 1.3） | ✅（TLS） | ✅（TLS） | ✅（TLS） | ❌ |
| 多显示器支持 | ✅（规划中） | ✅ | ✅ | ✅ | 部分 |
| 安装包大小 | ~10MB（Tauri） | ~50MB | ~50MB | ~80MB | ~5MB |

#### KVMini 的核心竞争力

1. **现代 UI**：Tauri + WebView 的 UI 比竞品的 Qt UI 更现代、更美观
2. **安装包小**：Tauri 应用约 10MB，远小于 Qt 应用的 50MB+
3. **快捷键冲突检测**：竞品均无此功能，是差异化亮点
4. **Rust 内存安全**：相比 C++ 竞品，无内存安全问题
5. **简单易用**：面向普通用户，无需复杂配置

#### KVMini 当前的不足（与竞品差距）

1. **仅支持 macOS + Windows**：竞品支持 Linux（暂不规划）

#### C++ 会更快吗？

理论上差别极小（< 0.1ms），实际体验感知不到。Rust 的 `core-graphics` crate 调用的是同样的系统 API，性能相当。真正影响延迟的是网络 RTT，而非语言本身。

### App Store 上架与收费分析

#### 能上架 Mac App Store 吗？

**结论：技术上可以，但有重要限制。**

| 问题 | 说明 |
|------|------|
| **沙盒限制** | Mac App Store 要求应用启用沙盒（App Sandbox），但 CGEvent Tap 需要辅助功能权限，沙盒环境下申请辅助功能权限流程更复杂 |
| **辅助功能权限** | 需要在 `entitlements` 中声明 `com.apple.security.temporary-exception.accessibility`，Apple 审核时会重点审查 |
| **网络权限** | 需要声明 `com.apple.security.network.client` 和 `com.apple.security.network.server` |
| **竞品先例** | Synergy 未上架 App Store，Barrier 也未上架，主要通过官网直销 |
| **建议** | 优先通过官网直销（DMG 安装包），避免 App Store 沙盒限制，后续可考虑上架 |

#### 收费方式建议

| 方案 | 说明 | 适合阶段 |
|------|------|----------|
| **免费开源** | 吸引用户，建立口碑 | 早期 |
| **买断制**（$9.99-$19.99） | 一次性付费，参考 Synergy 定价 | 功能完善后 |
| **订阅制**（$2.99/月） | 持续收入，但用户接受度低 | 不推荐 |
| **免费基础版 + 付费高级版** | 基础功能免费，剪贴板共享/加密等高级功能付费 | 推荐 |

#### 竞品定价参考

- **Synergy**：$29（个人）/ $49（商业），买断制，支持 3 台设备
- **Barrier**：完全免费开源
- **Input Leap**：完全免费开源
- **Mouse Without Borders**（微软）：完全免费，仅 Windows

**建议定价**：基础版免费，高级版（剪贴板共享 + 加密 + 多显示器）$9.99 买断，低于 Synergy 形成价格竞争力。

### 平台隔离原则

- 所有平台特定代码使用 `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "windows")]` 隔离
- 公共逻辑（协议、握手、边缘检测、键码映射表）不包含任何平台特定代码
- 测试使用 Mock 实现，不依赖真实系统 API，确保 CI 在任意平台可运行

### 网络延迟预期

- 局域网（同一路由器）：< 1ms RTT，键鼠延迟感知不到
- 跨网段（公司内网）：< 5ms RTT，可接受
- 跨公网：不推荐（延迟 > 20ms 会有明显卡顿感）
- 实现要点：TCP_NODELAY（已实现）+ 持久连接 + 事件合并（鼠标移动节流）

### 可行性最终确认

**做完 14 个任务（含键码映射、剪贴板共享、加密传输、多显示器）后，Mac ↔ Windows 键鼠共享在局域网内是真正可用的，且功能上已与 Barrier/Synergy 持平。**

前提条件（用户需知）：
1. macOS 需要在「系统设置 → 隐私与安全性 → 辅助功能」中手动授权 KVMini（所有同类软件都有此要求，包括 Barrier）
2. 两台设备需在同一局域网内
3. Windows 防火墙需允许 KVMini 的端口（首次运行时会弹出提示）
4. 首次连接时需要在双端确认对方的证书指纹（TLS TOFU 验证，只需确认一次）
