# KVMini v0.1.59 产品设计

发布日期：2026-05-14

---

## 1. 产品定位

KVMini 是跨平台 KVM 键鼠共享软件，让用户在多台电脑（Win + Mac）之间用**同一套键盘鼠标**无缝控制多个屏幕，类似 Synergy / Deskflow / Logitech Flow / Barrier。

支持系统：
- macOS 11+（M 芯片 + Intel 芯片）
- Windows 10 RS3+

---

## 2. v0.1.59 用户可感知改进

### 2.1 连接按钮新模型

- 旧（v0.1.58）：双按钮「断开」+「重连」，hidden 切换，状态混乱
- 新（v0.1.59）：单按钮三态切换
  - **Disconnected**：显示 "连接 / Connect"，可点
  - **Connecting**：显示 "连接中… / Connecting…"，禁用 + spinner
  - **Connected**：显示 "断开 / Disconnect"，可点

参考竞品：Synergy 3 / Deskflow 都是单按钮三态。

### 2.2 远程进程异常退出后键鼠快速恢复

- v0.1.58 之前：远端 KVMini 被 kill 后，本机被控端键鼠可能锁死 15s 甚至更久，Ctrl+Alt+Tab 也无效
- v0.1.59：纵深防御三层链路（TCP Keepalive 11s + Watchdog 10s + Heartbeat 15s + Ctrl+Alt+S 即时安全阀），**任意方式下线 ≤12s 完全恢复**

### 2.3 滚轮跨端可用

- v0.1.58：Win 主控端→Mac 被控端鼠标拖动可用、中键可用、但滚轮无响应
- v0.1.59：滚轮垂直/水平方向均正常生效

### 2.4 跨端鼠标切换稳定

- v0.1.58：Win→Mac 切过去后切不回来，Mac→Win 切回失败，整体乒乓异常
- v0.1.59：双向切换稳定，回切后 800ms 静默期防止贴边乒乓

---

## 3. 核心交互流程

### 3.1 主控端（控制端）使用流程

```
1. 启动 KVMini → 选择本机为「主控」
2. 点击「连接 / Connect」按钮
3. 选择历史 Peer 或扫描局域网
4. 等待对端确认（按钮显示「连接中… / Connecting…」）
5. 连接成功（按钮变为「断开 / Disconnect」）
6. 鼠标移到屏幕指定边缘 → 自动切换控制对端屏幕
7. 物理键盘按下任意键 → 抢占回本机控制权（Preempt）
   或：鼠标在对端屏幕撞反向边缘 → 自然回切（EdgeReturn）
8. 点击「断开 / Disconnect」 → 立即结束控制会话
```

### 3.2 被控端使用流程

```
1. 启动 KVMini → 选择本机为「被控」
2. 等待主控端连接（按钮显示「连接 / Connect」）
3. 收到主控端连接请求 → 自动转入 Connected 态
4. 主控端鼠标越过边缘 → 本机进入 RemoteControlling 模式
5. 主控端任意时刻可断开，本机键鼠 ≤12s 内完全恢复
6. 紧急情况：按 Ctrl+Alt+S 立即解除被控状态
```

---

## 4. 状态机模型

### 4.1 后端 ControlState

```
Idle → Armed → LocalControlling → Armed → Idle
  ↑                                        ↓
  └────────── on_disconnected ────────────┘

Idle → Armed → RemoteControlling → Idle (被控端)
```

### 4.2 前端 connectionPhase

```
disconnected → connecting → connected → disconnected
```

后端 `sharing-state-sync` snapshot 是前端按钮状态唯一驱动。

---

## 5. 安全防护机制

| 机制 | 触发条件 | 动作 |
|---|---|---|
| TCP Keepalive | OS 内核层，5s idle + 3s interval × 2 retries | RST 断流，应用层立即感知 |
| Watchdog | 应用层 10s 无远端帧 | force_recovery：停 Hook、解 blocker、recouple cursor、清虚拟光标 |
| Heartbeat | 应用层 5s × 3 无 Ack | emit Disconnected 事件链路 |
| Ctrl+Alt+S 安全阀 | 用户按下 | 立即 force_idle，无视任何状态 |
| EdgeDetector 静默期 | 状态切换瞬间 | 800ms 内禁止再次触发，防乒乓 |
| EdgeDetector overshoot | 鼠标必须连续撞边 N 次才触发 | 防误触 |

---

## 6. UI 国际化

所有用户可见文本通过 `src/i18n.js` 的 `t()` 函数国际化，禁止硬编码中文：

- `btnConnect` / `btnConnecting` / `btnDisconnect` / `btnReconnect`（历史列表用）
- `statusReconnecting` 状态徽章
- 完整 zh / en 双语字典同步维护

---

## 7. 版本号显示

启动后通过 `initVersion()` 调用后端 `get_app_version`（返回 `CARGO_PKG_VERSION="0.1.59"`），渲染到：
- 主界面右上角 `#app-version` (v0.1.59)
- 设置页 `#settings-version` (v0.1.59)
- 关于页 changelog 顶部条目

降级方案：HTML `<meta name="app-version" content="0.1.59">` 在后端不可用时使用。

---

## 8. 兼容性矩阵

| 平台 | 最低版本 | TCP Keepalive | Watchdog | 备注 |
|---|---|---|---|---|
| macOS | 11.0 (Big Sur) | ✅ socket2 完全支持 | ✅ | M 芯片 + Intel 芯片 |
| Windows | Windows 10 RS3 | ✅ socket2 完全支持 | ✅ | Win10 RS3 之前回退系统默认 keepalive，不报错 |

---

## 9. 后续路线图

- F1 滚轮已修，需用户实测确认真实场景生效率
- F4 跨端切换 RC#1/RC#2/RC#3 已 cover，长期观察是否有遗漏边界场景
- 探索：iOS / Android 客户端作为远程被控端
