# 需求文档：心跳逻辑独立模块化

## 引言

当前项目中，心跳（Heartbeat）相关逻辑分散在多个文件中：
- `kvm-core/src/network.rs`：心跳发送线程、超时检测、last_heartbeat 时间戳管理
- `kvm-agent/src/server.rs`：服务端心跳超时判断（硬编码 10 秒）
- `kvm-agent/src/connection/transport.rs`：客户端心跳回复逻辑
- `kvm-core/src/config.rs`：心跳间隔和超时次数配置

存在的问题：
1. 超时阈值在 `server.rs` 中硬编码为 10 秒，与 `config.rs` 中的 `heartbeat_interval_secs(5) × heartbeat_timeout_count(3) = 15秒` 不一致
2. 心跳逻辑无法复用，Windows/Mac 两端各自重复实现
3. 心跳状态（是否超时、上次心跳时间）没有统一的管理入口

本需求将心跳逻辑抽取为 `kvm-core/src/heartbeat.rs` 独立模块，统一管理心跳发送、接收确认、超时检测，并修复超时阈值不一致的 bug。

---

## 需求

### 需求 1：抽取心跳模块到 kvm-core

**用户故事：** 作为开发者，我希望心跳逻辑集中在一个独立模块中，以便维护和复用，避免各端重复实现。

#### 验收标准

1. WHEN 项目编译时 THEN `kvm-core` SHALL 包含 `heartbeat.rs` 模块，对外暴露 `HeartbeatManager` 结构体
2. WHEN `HeartbeatManager` 初始化时 THEN 系统 SHALL 从 `NetworkConfig` 读取 `heartbeat_interval_secs` 和 `heartbeat_timeout_count`，不允许硬编码
3. WHEN `kvm-agent/src/server.rs` 和 `kvm-agent/src/connection/transport.rs` 使用心跳功能时 THEN 系统 SHALL 调用 `HeartbeatManager`，移除各自的心跳相关代码
4. WHEN `kvm-core/src/network.rs` 使用心跳功能时 THEN 系统 SHALL 调用 `HeartbeatManager`，移除内联的心跳线程代码

---

### 需求 2：修复超时阈值不一致 Bug

**用户故事：** 作为开发者，我希望超时阈值由配置统一计算，以便避免误判断线。

#### 验收标准

1. WHEN 心跳超时检测运行时 THEN 系统 SHALL 使用 `heartbeat_interval_secs × heartbeat_timeout_count` 作为超时阈值（例如 5×3=15 秒），而不是硬编码的 10 秒
2. IF 配置中 `heartbeat_interval_secs = 5`，`heartbeat_timeout_count = 3` THEN 系统 SHALL 在 **15 秒**未收到心跳后才判定超时断线
3. WHEN 超时阈值计算时 THEN 系统 SHALL 在 `HeartbeatManager` 内部完成，调用方无需关心具体数值

---

### 需求 3：HeartbeatManager 提供统一的发送与接收接口

**用户故事：** 作为开发者，我希望通过统一接口操作心跳，以便客户端和服务端都能简单调用。

#### 验收标准

1. WHEN 客户端连接建立后 THEN `HeartbeatManager` SHALL 提供 `start_sender` 方法，在后台线程定时发送 `KvmFrame::Heartbeat`
2. WHEN 收到对端的 `KvmFrame::Heartbeat` 时 THEN `HeartbeatManager` SHALL 提供 `on_heartbeat_received` 方法，更新内部时间戳
3. WHEN 收到对端的 `KvmFrame::HeartbeatAck` 时 THEN `HeartbeatManager` SHALL 提供 `on_ack_received` 方法，更新内部时间戳
4. WHEN 需要检测超时时 THEN `HeartbeatManager` SHALL 提供 `is_timed_out` 方法，返回 `bool`，由调用方决定如何处理断线

---

### 需求 4：超时后触发回调或事件通知

**用户故事：** 作为开发者，我希望心跳超时时能自动触发断线处理，以便不需要在业务代码中轮询检查。

#### 验收标准

1. WHEN `HeartbeatManager` 检测到超时时 THEN 系统 SHALL 通过 `tokio::sync::mpsc` channel 发送 `HeartbeatEvent::Timeout` 事件
2. WHEN 超时事件发出后 THEN 调用方（`network.rs` 或 `server.rs`）SHALL 收到事件并执行断线清理逻辑
3. IF 连接在超时前恢复心跳 THEN 系统 SHALL 取消超时计时，不发送超时事件
