# 需求文档：键鼠共享核心功能

## 引言

KVMini 目前已实现设备发现、手动 IP 直连、设置保存等基础功能，但**核心的键盘鼠标共享功能尚未实现**。本需求文档定义键鼠共享的完整核心功能，包括：

- **主控端（Client）**：捕获本机键盘/鼠标输入，通过 TCP 发送给被控端
- **被控端（Server）**：监听 TCP 端口，接收事件并注入本机操作系统
- **屏幕边缘切换**：鼠标移到屏幕边缘自动切换到另一台设备
- **多显示器布局配置**：可视化拖拽配置两台设备的显示器相对位置

参考同类软件（Synergy、Barrier、Input Leap、ShareMouse）的设计，采用**双端均可互相控制**的对等架构，角色由用户在 UI 中切换。

---

## 需求

### 需求 1：主控端/被控端角色切换

**用户故事：** 作为用户，我希望能在同一台设备上切换"主控端"和"被控端"角色，以便灵活决定哪台设备控制哪台设备。

#### 验收标准

1. WHEN 用户打开应用 THEN 系统 SHALL 显示当前角色（主控端 / 被控端），默认为被控端
2. WHEN 用户点击切换角色按钮 THEN 系统 SHALL 切换角色并重启对应的服务（停止旧服务，启动新服务）
3. IF 当前角色为被控端 THEN 系统 SHALL 启动 TCP 监听服务，等待主控端连接
4. IF 当前角色为主控端 THEN 系统 SHALL 启动键鼠捕获，并将事件发送给已连接的被控端
5. WHEN 角色切换时 THEN 系统 SHALL 断开当前所有连接并通知用户

---

### 需求 2：被控端 TCP 服务与事件注入

**用户故事：** 作为被控端用户，我希望应用能监听 TCP 连接并将收到的键鼠事件注入本机，以便主控端可以远程控制我的设备。

#### 验收标准

1. WHEN 被控端启动 THEN 系统 SHALL 在配置的 IP 和端口（默认 0.0.0.0:8889）上启动 TCP 监听
2. WHEN 主控端建立连接 THEN 系统 SHALL 接受连接并开始接收事件流
3. WHEN 收到鼠标移动事件 THEN 系统 SHALL 调用平台 API 将鼠标移动到指定坐标（macOS: CGEvent；Windows: SendInput）
4. WHEN 收到鼠标点击事件 THEN 系统 SHALL 模拟对应的鼠标按键按下/释放
5. WHEN 收到键盘事件 THEN 系统 SHALL 模拟对应的键盘按键按下/释放
6. WHEN 连接断开 THEN 系统 SHALL 停止事件注入并更新连接状态
7. IF macOS 被控端首次运行 THEN 系统 SHALL 检测辅助功能（Accessibility）权限，若未授权则引导用户开启
8. IF Windows 被控端运行 THEN 系统 SHALL 使用 SendInput API 注入事件，无需额外权限申请
9. WHEN 被控端和主控端均需编写单元测试 THEN 系统 SHALL 覆盖 macOS 和 Windows 两个平台的事件注入逻辑

---

### 需求 3：主控端键鼠捕获与事件发送

**用户故事：** 作为主控端用户，我希望应用能捕获我的键盘和鼠标输入，并以极低延迟发送给被控端，以便实现流畅的远程控制体验。

#### 验收标准

1. WHEN 主控端连接到被控端后 THEN 系统 SHALL 开始捕获本机鼠标移动、点击、滚轮和键盘事件
2. WHEN 捕获到事件 THEN 系统 SHALL 通过已建立的 TCP 连接以二进制协议发送事件（目标端到端延迟 < 5ms 局域网）
3. WHEN 主控端处于"控制模式"时 THEN 系统 SHALL 拦截本机键鼠输入（不让本机响应），仅转发给被控端
4. WHEN 用户按下切换快捷键（默认 Ctrl+Alt+S）THEN 系统 SHALL 退出控制模式，键鼠恢复本机响应
5. IF macOS 主控端 THEN 系统 SHALL 使用 CGEvent Tap 捕获全局键鼠事件，需要辅助功能权限
6. IF Windows 主控端 THEN 系统 SHALL 使用 SetWindowsHookEx 低级钩子捕获全局键鼠事件
7. WHEN 网络发送失败 THEN 系统 SHALL 触发断线重连逻辑（已有实现），不丢失连接状态
8. WHEN 主控端和被控端均需编写单元测试 THEN 系统 SHALL 覆盖事件序列化/反序列化、发送逻辑

---

### 需求 4：屏幕边缘自动切换（鼠标跟随屏幕）

**用户故事：** 作为用户，我希望当鼠标移动到屏幕边缘时能自动切换到另一台设备，以便像使用多显示器一样无缝操作两台电脑。

#### 验收标准

1. WHEN 主控端鼠标到达配置的屏幕边缘（上/下/左/右）THEN 系统 SHALL 自动将控制权切换到被控端
2. WHEN 控制权切换到被控端 THEN 系统 SHALL 将鼠标光标在被控端屏幕的对应边缘位置显示
3. WHEN 被控端鼠标到达对应的反向边缘 THEN 系统 SHALL 将控制权切回主控端
4. WHEN 切换发生 THEN 系统 SHALL 在主控端隐藏本机鼠标光标，在被控端显示鼠标光标
5. IF 切换过程中发生网络延迟 > 50ms THEN 系统 SHALL 在 UI 上显示延迟警告，但不中断切换
6. WHEN 编写测试 THEN 系统 SHALL 覆盖边缘检测逻辑（macOS 和 Windows 坐标系差异）

---

### 需求 5：多显示器布局配置（可视化拖拽）

**用户故事：** 作为用户，我希望能通过可视化界面拖拽两台设备的显示器相对位置，以便精确配置鼠标从哪个边缘切换到哪台设备。

#### 验收标准

1. WHEN 用户打开显示器布局配置页面 THEN 系统 SHALL 显示两个可拖拽的显示器方块（代表本机和远端设备）
2. WHEN 用户拖拽显示器方块 THEN 系统 SHALL 允许将其放置在另一个显示器的左/右/上/下四个方向
3. WHEN 布局保存后 THEN 系统 SHALL 根据布局决定鼠标超出哪个边缘时切换到哪台设备
4. IF 设备 A 在设备 B 的右侧 THEN 系统 SHALL 当设备 B 鼠标超出右边缘时切换到设备 A，且鼠标出现在设备 A 的左边缘
5. WHEN 布局配置变更 THEN 系统 SHALL 持久化保存到本地配置文件（macOS 和 Windows 均支持）
6. WHEN 编写测试 THEN 系统 SHALL 覆盖布局配置的保存/读取和边缘映射逻辑

---

### 需求 6：事件传输协议（极致低延迟优化）

**用户故事：** 作为用户，我希望键鼠操作的延迟极低（局域网 < 5ms），以便获得接近本地操作的流畅体验。

#### 验收标准

1. WHEN 设计事件传输协议 THEN 系统 SHALL 使用紧凑的二进制格式（固定长度帧，无 JSON 序列化开销）
2. WHEN 建立 TCP 连接 THEN 系统 SHALL 设置 TCP_NODELAY（禁用 Nagle 算法）以减少延迟
3. WHEN 发送鼠标移动事件 THEN 系统 SHALL 合并高频事件（节流，最高 1000Hz），避免网络拥塞
4. WHEN 连接建立 THEN 系统 SHALL 进行握手协议验证（版本号、平台信息交换）
5. WHEN 编写测试 THEN 系统 SHALL 覆盖协议编解码的正确性（macOS 和 Windows 字节序一致性）

---

### 需求 7：macOS 辅助功能权限引导

**用户故事：** 作为 macOS 用户，我希望应用能自动检测并引导我开启辅助功能权限，以便键鼠捕获和注入功能正常工作。

#### 验收标准

1. WHEN macOS 应用启动时 THEN 系统 SHALL 检测辅助功能（Accessibility）权限状态
2. IF 权限未授权 THEN 系统 SHALL 在 UI 上显示权限引导提示，并提供"去开启"按钮直接跳转到系统偏好设置
3. WHEN 用户授权后返回应用 THEN 系统 SHALL 自动重新检测权限状态并更新 UI
4. IF Windows 平台 THEN 系统 SHALL 跳过此检测（Windows 不需要此权限）
5. WHEN 编写测试 THEN 系统 SHALL 覆盖权限检测逻辑的 macOS 和 Windows 分支

---

### 需求 8：Windows 平台完整适配

**用户故事：** 作为 Windows 用户，我希望所有键鼠共享功能在 Windows 上与 macOS 完全对等，以便跨平台使用。

#### 验收标准

1. WHEN 在 Windows 上运行被控端 THEN 系统 SHALL 使用 SendInput API 注入鼠标和键盘事件
2. WHEN 在 Windows 上运行主控端 THEN 系统 SHALL 使用 SetWindowsHookEx 低级钩子（WH_MOUSE_LL、WH_KEYBOARD_LL）捕获全局事件
3. WHEN Windows 和 macOS 平台代码不可复用时 THEN 系统 SHALL 使用 `#[cfg(target_os)]` 条件编译隔离，保持统一的接口
4. WHEN 编写测试 THEN 系统 SHALL 为 Windows 和 macOS 各自的平台特定代码编写独立的单元测试
5. WHEN 屏幕坐标系存在差异时（如 DPI 缩放）THEN 系统 SHALL 在事件传输前进行坐标归一化处理
