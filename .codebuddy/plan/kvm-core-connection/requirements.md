# 需求文档：KVM 核心连接功能

## 引言

本项目是一个跨平台 KVM（Keyboard/Video/Mouse）软件，目标是让 Windows 电脑通过局域网控制 Mac 电脑的键盘和鼠标（类似 Synergy/Barrier）。

**平台目标：**
- Windows 端：兼容 Windows 10/11（当前主流版本）
- Mac 端：兼容 macOS 13 Ventura 及以上（最新操作系统）

**当前已实现（但不完整）：**
- Windows 端 Tauri 应用 UI（设备列表、状态面板、本机IP显示）
- UDP 广播设备发现（Windows 端发送，但 Mac 端没有响应程序）
- USB 设备扫描（纯模拟数据）
- 鼠标/键盘事件类型定义（未实现转发逻辑）

**核心缺失：Mac 端没有任何程序**，因此目前根本无法真正连接。要实现 Windows → Mac 的 KVM 控制，需要：
1. Mac 端的被控端程序（接收并执行输入事件）
2. Windows 端与 Mac 端之间的通信协议
3. 鼠标/键盘事件的捕获与转发

---

## 需求

### 需求 1：Mac 端被控端程序

**用户故事：** 作为用户，我希望在 Mac 上运行一个后台程序，以便 Windows 端能够发现并连接到它。

#### 验收标准

1. WHEN Mac 端程序启动 THEN 系统 SHALL 在局域网内监听 UDP 广播端口 8888，并回复 `KVM_RESPONSE` 响应包（包含设备名称、OS 类型）
2. WHEN Windows 端发起 TCP 连接请求 THEN Mac 端 SHALL 在端口 9999 接受 TCP 连接并建立持久会话
3. WHEN Mac 端程序运行 THEN 系统 SHALL 在系统托盘显示图标，支持启动/停止控制
4. IF Mac 端程序已连接 THEN 系统 SHALL 显示当前连接的 Windows 端 IP 地址

### 需求 2：统一通信协议

**用户故事：** 作为开发者，我希望 Windows 端和 Mac 端使用统一的消息协议通信，以便可靠地传输输入事件。

#### 验收标准

1. WHEN 双端建立 TCP 连接 THEN 系统 SHALL 使用 JSON 格式传输消息，每条消息以换行符分隔
2. WHEN Windows 端发送输入事件 THEN 消息 SHALL 包含事件类型（mouse_move/mouse_down/mouse_up/key_down/key_up/scroll）和对应参数
3. WHEN 连接建立后 THEN 系统 SHALL 每 2 秒发送一次心跳包，超过 6 秒未收到心跳则判定连接断开
4. IF 连接意外断开 THEN Windows 端 SHALL 自动尝试重连，最多重试 3 次

### 需求 3：Windows 端鼠标/键盘事件捕获与转发

**用户故事：** 作为用户，我希望当鼠标移动到屏幕边缘时自动切换到 Mac 控制模式，以便无缝操控 Mac。

#### 验收标准

1. WHEN 用户在 Windows 端点击"连接"按钮后连接成功 THEN 系统 SHALL 进入"监听模式"，捕获全局鼠标和键盘事件
2. WHEN 鼠标移动到屏幕右边缘（或用户配置的触发边缘）THEN 系统 SHALL 将鼠标锁定，并将后续所有输入事件转发给 Mac 端
3. WHEN 用户按下预设的切换快捷键（默认 Ctrl+Alt+Tab）THEN 系统 SHALL 切换回 Windows 本地控制模式
4. WHEN 处于转发模式 THEN 系统 SHALL 将鼠标移动的相对坐标、按键 keycode、滚轮增量实时发送给 Mac 端
5. IF 连接断开 THEN 系统 SHALL 立即释放鼠标锁定，恢复 Windows 本地控制

### 需求 4：Mac 端执行输入事件

**用户故事：** 作为用户，我希望 Mac 端能够真实执行从 Windows 端收到的鼠标和键盘操作，以便实现远程控制。

#### 验收标准

1. WHEN Mac 端收到 `mouse_move` 事件 THEN 系统 SHALL 使用 macOS CGEvent API 移动鼠标到对应位置
2. WHEN Mac 端收到 `mouse_down`/`mouse_up` 事件 THEN 系统 SHALL 模拟对应鼠标按键的按下和释放
3. WHEN Mac 端收到 `key_down`/`key_up` 事件 THEN 系统 SHALL 将 keycode 转换为 macOS 虚拟键码并模拟按键
4. WHEN Mac 端收到 `scroll` 事件 THEN 系统 SHALL 模拟滚轮滚动
5. IF Mac 端没有辅助功能权限 THEN 系统 SHALL 弹出系统提示引导用户在"系统设置 → 隐私与安全性 → 辅助功能"中授权（适配 macOS 13+ 新版系统设置路径）

### 需求 5：Windows 端连接流程优化

**用户故事：** 作为用户，我希望 Windows 端能够清晰地显示连接状态和错误信息，以便我知道当前是否已成功连接到 Mac。

#### 验收标准

1. WHEN 设备发现到 Mac 端程序 THEN 设备列表 SHALL 显示 Mac 的主机名、IP 地址和 OS 类型（而非 "undefined"）
2. WHEN 用户点击"连接"按钮 THEN 系统 SHALL 显示连接进度（发现 → 握手 → 已连接）
3. WHEN 连接成功 THEN 状态面板 SHALL 显示已连接设备名称、连接延迟（ms）
4. WHEN 连接失败 THEN 系统 SHALL 显示具体错误原因（如"目标设备未运行 KVM 程序"、"网络不可达"）

### 需求 6：平台兼容性与代码隔离

**用户故事：** 作为开发者，我希望 Windows 端和 Mac 端的平台特定代码严格隔离，以便两端互不影响、各自适配最新系统。

#### 验收标准

1. WHEN 编译 Windows 端 THEN 所有 Windows 特定代码（如 `SetWindowsHookEx`、Win32 API）SHALL 使用 `#[cfg(target_os = "windows")]` 条件编译隔离，不得出现在 Mac 端编译产物中
2. WHEN 编译 Mac 端 THEN 所有 macOS 特定代码（如 `CGEvent`、`CoreGraphics`）SHALL 使用 `#[cfg(target_os = "macos")]` 条件编译隔离，不得出现在 Windows 端编译产物中
3. WHEN 定义共享通信协议结构体 THEN 协议相关代码 SHALL 放置在独立的共享模块（`shared/`）中，Windows 端和 Mac 端均可引用，不包含任何平台特定依赖
4. IF 某个平台的 API 在目标系统版本上不可用 THEN 系统 SHALL 在编译期或运行期给出明确的错误提示，而非静默失败
5. WHEN Mac 端程序运行在 macOS 13 Ventura 及以上版本 THEN 系统 SHALL 使用新版"系统设置"（System Settings）路径而非旧版"系统偏好设置"（System Preferences）的 API 和引导文案
6. WHEN Windows 端程序运行在 Windows 10/11 THEN 系统 SHALL 兼容两个版本的 Win32 API 差异，不使用仅 Windows 11 独有的 API
7. WHEN 两端同时运行 THEN 各自的崩溃或异常 SHALL 不影响对方进程，连接断开后双端均能独立恢复到正常待机状态
