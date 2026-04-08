# 需求文档：角色自由切换 & 协议统一

## 引言

当前 KVMini 项目中，Windows 端（`src-tauri`）和 Mac 端（`kvm-agent`）使用了**两套完全不兼容的通信协议**，导致双端无法互联：

- **Mac 端（kvm-agent）**：JSON 文本协议，换行分隔，监听端口 9999，连接建立后 Mac 端主动发握手
- **Windows 端（src-tauri）**：二进制帧协议，监听端口 8889，Windows 端主动发握手

此外，角色（主控端/被控端）目前是硬编码的，无法在运行时自由切换。

本需求文档覆盖两个目标：
1. **统一双端通信协议**，使 Windows 和 Mac 能够正常互联
2. **支持运行时角色切换**，任意一端都可以随时切换为主控端或被控端

> **默认角色约定**：应用启动后，**Windows 端默认为主控端**（采集并发送键鼠事件），**Mac 端默认为被控端**（接收并执行键鼠事件）。用户可在运行时随时切换。

> **代码范围**：本需求涉及 `src-tauri`（Windows 端）和 `kvm-agent`（Mac 端）两个目录的代码，两端均需修改。

---

## 需求

### 需求 1：统一通信协议

**用户故事：** 作为开发者，我希望 Windows 端和 Mac 端使用同一套通信协议，以便两端能够正常建立连接并通信。

#### 验收标准

1. WHEN Windows 端连接 Mac 端 THEN 系统 SHALL 使用统一的二进制帧协议（与现有 `src-tauri/src/protocol.rs` 定义一致）进行通信
2. WHEN Mac 端（kvm-agent）收到连接 THEN 系统 SHALL 能正确解析二进制帧协议，而非 JSON 文本协议
3. WHEN 握手发生 THEN 系统 SHALL 遵循"主控端先发 Handshake 帧，被控端回复 HandshakeAck 帧"的统一握手流程
4. IF 协议版本不匹配 THEN 系统 SHALL 拒绝连接并输出版本不兼容错误
5. WHEN 连接建立后 THEN 系统 SHALL 统一使用端口 **8889** 进行 TCP 通信（Mac 端 kvm-agent 改为监听 8889）

---

### 需求 2：Mac 端支持二进制帧输入事件

**用户故事：** 作为被控端用户，我希望 Mac 端能够接收并执行来自主控端的鼠标和键盘事件，以便实现跨设备控制。

#### 验收标准

1. WHEN Mac 端收到 `MouseMove` 帧 THEN 系统 SHALL 将鼠标移动到对应坐标
2. WHEN Mac 端收到 `MouseButton` 帧 THEN 系统 SHALL 执行对应的鼠标按下/释放操作
3. WHEN Mac 端收到 `MouseScroll` 帧 THEN 系统 SHALL 执行滚轮操作
4. WHEN Mac 端收到 `KeyEvent` 帧 THEN 系统 SHALL 执行对应的键盘按下/释放操作
5. WHEN Mac 端收到 `Heartbeat` 帧 THEN 系统 SHALL 更新心跳时间戳，不执行输入操作

---

### 需求 3：运行时角色切换（含默认角色）

**用户故事：** 作为用户，我希望在不重启应用的情况下随时切换主控端和被控端角色，以便灵活控制哪台电脑作为输入源。

#### 验收标准

1. WHEN 应用启动 THEN Windows 端 SHALL 默认进入**主控端**模式，自动启动键鼠采集并等待建立连接
2. WHEN 应用启动 THEN Mac 端 SHALL 默认进入**被控端**模式，自动监听端口 8889 等待主控端连接
3. WHEN 用户在 UI 中点击"切换为主控端" THEN 系统 SHALL 停止当前被控端监听服务，并启动主控端连接流程
4. WHEN 用户在 UI 中点击"切换为被控端" THEN 系统 SHALL 断开当前主控端连接，并重新启动 TCP 监听服务
5. WHEN 角色切换完成 THEN 系统 SHALL 通过事件通知前端更新 UI 显示当前角色
6. IF 切换角色时存在活跃连接 THEN 系统 SHALL 先优雅断开连接，再完成角色切换
7. WHEN 任意一端处于被控端模式 THEN 系统 SHALL 监听端口 8889，等待对端主控端连接并注入输入事件

---

### 需求 4：连接状态实时反馈

**用户故事：** 作为用户，我希望在 UI 上实时看到连接状态（已连接/断开/重连中），以便了解当前控制链路是否正常。

#### 验收标准

1. WHEN 连接成功建立 THEN 系统 SHALL 在 UI 显示"已连接"及对端 IP 地址
2. WHEN 连接断开 THEN 系统 SHALL 在 UI 显示"已断开"并提示可重连
3. WHEN 角色为主控端且连接断开 THEN 系统 SHALL 自动尝试重连（最多 5 次，间隔 3 秒）
4. WHEN 角色为被控端 THEN 系统 SHALL 不主动重连，而是继续监听等待新连接
