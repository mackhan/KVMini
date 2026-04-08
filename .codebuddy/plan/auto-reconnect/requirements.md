# 需求文档：仿付费版 ShareMouse 策略的连接保活与自动重连

## 引言

### 付费版 ShareMouse 的策略（本项目对标目标）

付费版 ShareMouse 的连接保活策略是：

> **应用层心跳（Ping-Pong）+ TCP Keep-Alive（系统层兜底）+ 断线自动无限重连**

- **应用层心跳**：每隔固定时间（约 5 秒）发送心跳包，对端回复 ACK，双向保活，防止防火墙因空闲踢断连接
- **TCP Keep-Alive**：操作系统内核层的保活探测，作为兜底机制。内核每隔一段时间自动发送 1 字节探测包，对端内核回复 ACK，让防火墙认为连接仍然活跃。**注意：这是 OS 自动完成的，应用层只需在建立连接时设置 `SO_KEEPALIVE` 选项即可，默认探测间隔 2 小时太长，需手动调短为 30 秒**
- **自动无限重连**：一旦连接真正断开，自动无限次重连，直到重新连上为止
- **用户可手动断开**，断开后可手动重新发起连接

### 当前代码存在的问题

**问题 1：代码中仍有"主控端/被控端"概念（需要重构）**
- `connection/manager.rs` 中有 `ConnectionRole::Controller` / `ConnectionRole::Controlled` 枚举
- `transport.rs` 中有"主控端连接"和"被控端监听"两套逻辑
- `hotkey.rs` 中有"切换主控/被控角色"的快捷键
- 实际上两台机器是**对等关系**：任意一台都可以发起连接，任意一台都可以接受连接
- 需要重构为只维护"已连接/未连接"两种状态，不区分角色

**问题 2：心跳逻辑混乱**
- `transport.rs` 的发起方（`run_client`）主循环只发送键鼠事件，**没有任何心跳**
- `transport.rs` 的接受方心跳线程每 3 秒**无条件发送 `HeartbeatAck`**，逻辑混乱
- 应统一改为：双方都发送 `Heartbeat`，收到后回复 `HeartbeatAck`

**问题 3：重连次数有上限（MAX_RETRIES = 5）**
- `connection/manager.rs` 中最多重连 5 次后放弃，导致断线后永久无法恢复

**问题 4：没有 TCP Keep-Alive**
- 当前代码没有启用系统层 `SO_KEEPALIVE`，防火墙空闲超时时直接踢断连接

**问题 5：断开/重连没有 UI 提示**
- 连接断开时没有明显的界面提示，重连过程中没有状态显示，用户无法手动触发重连

---

## 需求

### 需求 1：去掉主控端/被控端概念，重构为对等连接模型

**用户故事：** 作为用户，我希望软件不区分主控端和被控端，任意一台机器都可以发起连接，以便使用更灵活。

#### 验收标准

1. WHEN 代码重构后 THEN `ConnectionRole::Controller` 和 `ConnectionRole::Controlled` 枚举 SHALL 被移除
2. WHEN 用户点击"连接"时 THEN 本机作为发起方主动连接对端 IP，对端作为接受方监听端口
3. WHEN 连接建立后 THEN 系统 SHALL 只维护"已连接/未连接"两种状态，不区分角色
4. WHEN 代码重构后 THEN `connection/manager.rs` 中的 `ConnectionRole` 枚举 SHALL 被移除，改为统一的 `ConnectionManager`
5. WHEN 代码重构后 THEN `main.rs` 中的角色切换逻辑 SHALL 简化为"连接/断开"操作
6. WHEN 代码重构后 THEN `hotkey.rs` 中的"切换主控/被控角色"快捷键 SHALL 被移除
7. WHEN 代码重构后 THEN 前端中的"主控端"/"被控端"文案 SHALL 被移除或替换为中性描述

---

### 需求 2：应用层双向心跳（仿付费版 ShareMouse）

**用户故事：** 作为用户，我希望软件在空闲时也能保持连接活跃，以便防火墙不会因为 TCP 连接无数据而断开。

#### 验收标准

1. WHEN TCP 连接握手完成后 THEN **双方**都 SHALL 启动独立的心跳发送线程，每 5 秒发送一次 `KvmFrame::Heartbeat`
2. WHEN 任意一方收到 `KvmFrame::Heartbeat` 时 THEN 该方 SHALL 立即回复 `KvmFrame::HeartbeatAck`
3. WHEN 任意一方收到 `KvmFrame::HeartbeatAck` 时 THEN 该方 SHALL 更新 `last_heartbeat_ack` 时间戳
4. WHEN 用户有键鼠操作时 THEN 心跳计时器 SHALL 重置（有数据发送时不需要额外心跳）
5. IF 连续 3 次（15 秒）未收到对端的 `HeartbeatAck` THEN 系统 SHALL 判定连接断开，触发自动重连（需求 3）
6. WHEN 心跳发送失败（写入错误）THEN 心跳线程 SHALL 退出，并通知主循环触发重连
7. WHEN 设置 TCP 读超时时 THEN 读超时 SHALL 设置为 20 秒（大于心跳超时阈值 15 秒），避免读超时误触发

---

### 需求 3：无限自动重连（仿 ShareMouse）

**用户故事：** 作为用户，我希望软件断线后能自动无限重连，以便不需要手动操作就能恢复控制。

#### 验收标准

1. WHEN 连接失败或心跳超时断开时 THEN 系统 SHALL 自动触发重连，**不设置最大重试次数上限**
2. WHEN 重连时 THEN 系统 SHALL 等待 3 秒后重试，避免频繁连接
3. WHEN 重连成功时 THEN 系统 SHALL 立即重启心跳定时器（需求 2）
4. WHEN 用户主动点击"断开连接"时 THEN 系统 SHALL 停止重连循环，不再自动重连
5. WHEN 用户主动点击"重新连接"时 THEN 系统 SHALL 立即发起一次连接，成功后恢复自动重连
6. IF `connection/manager.rs` 中存在 `MAX_RETRIES = 5` THEN 系统 SHALL 移除该限制

---

### 需求 4：TCP Keep-Alive 系统层保活（兜底机制）

**用户故事：** 作为用户，我希望软件启用系统级 TCP 保活，以便在应用层心跳之外提供双重保障，防止防火墙踢断空闲连接。

#### 验收标准

1. WHEN TCP 连接建立后 THEN 系统 SHALL 在 socket 上启用 `SO_KEEPALIVE` 选项
2. WHEN 启用 Keep-Alive 时 THEN 探测间隔 SHALL 设置为 30 秒，探测次数 3 次，探测超时 10 秒（覆盖系统默认的 2 小时间隔）
3. WHEN 系统层 Keep-Alive 探测失败时 THEN 操作系统 SHALL 关闭连接，应用层 SHALL 检测到断开并触发自动重连（需求 3）
4. IF 应用层心跳（5 秒）已足够保活防火墙 THEN TCP Keep-Alive（30 秒）作为兜底机制，两者共存

---

### 需求 5：断开/重连 UI 提示

**用户故事：** 作为用户，我希望在连接断开或重连时能看到明确的界面提示，以便了解当前连接状态。

#### 验收标准

1. WHEN 连接成功建立时 THEN UI SHALL 显示"已连接"状态，并显示对端 IP 地址
2. WHEN 连接断开时 THEN UI SHALL 立即显示"连接已断开"提示（Toast 通知），并切换为"断开"状态
3. WHEN 自动重连中时 THEN UI SHALL 显示"正在重连...（第 N 次）"状态提示
4. WHEN 重连成功时 THEN UI SHALL 显示"重连成功"Toast 提示，并恢复"已连接"状态
5. WHEN 重连失败超过 10 次时 THEN UI SHALL 显示"重连失败，仍在后台尝试"提示，但不停止重连
6. WHEN 用户点击"断开连接"按钮时 THEN UI SHALL 显示"已手动断开"提示，并停止自动重连
7. WHEN 用户点击"重新连接"按钮时 THEN UI SHALL 立即触发重连，并显示"连接中..."状态

---

### 需求 6：手动断开与重连操作

**用户故事：** 作为用户，我希望能手动控制连接的断开和重连，以便在需要时灵活操作。

#### 验收标准

1. WHEN 用户处于已连接状态时 THEN UI SHALL 显示"断开连接"按钮
2. WHEN 用户处于断开/重连中状态时 THEN UI SHALL 显示"重新连接"按钮
3. WHEN 用户点击"断开连接"时 THEN 系统 SHALL 立即关闭 TCP 连接并停止自动重连
4. WHEN 用户点击"重新连接"时 THEN 系统 SHALL 立即发起连接，连接失败时继续自动重连
5. WHEN 用户点击"重新连接"时 THEN 系统 SHALL 使用上次连接的 IP 地址，无需重新输入
