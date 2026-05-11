# 状态管理重构 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构 KVMini 状态管理系统，解决 Win 被控端在 Mac 主控断连后键鼠锁死、双端状态显示错误/不匹配的问题。

**Architecture:** 
- 新增 `state_guard` 模块，封装 Watchdog（3s 超时自动回 Idle）、状态同步快照、日志增强
- Win 被控端增加 Ctrl+Alt+S 本地快捷键强制回 Idle（最安全兜底）
- 前端状态归一化：用 `sharing-state-sync` 事件推送完整状态快照，消除事件顺序不确定性

**Tech Stack:** Rust (kvm-core + src-tauri)、JavaScript (前端)、Tauri Events

---

## 根因回顾

1. **Win 被控端 Mac 断连后键鼠锁死**：Mac 异常断开时心跳超时 15s，期间 hook `should_swallow()` 吞掉所有物理键鼠
2. **状态显示不匹配**：`control-state-changed` 和 `connection-status` 两个事件无原子性保证，到达顺序不确定
3. **缺少快速恢复手段**：除 Ctrl+Alt+Delete (Windows SAS) 外无其他解锁方式

---

### Task 1: 新增 `state_guard` 模块骨架

**Files:**
- Create: `src-tauri/src/state_guard.rs`
- Modify: `src-tauri/src/main.rs` (添加 mod 声明)

**Step 1: 创建 state_guard.rs 骨架**

```rust
//! 状态守卫模块（State Guard）
//!
//! 职责：
//! 1. Watchdog 看门狗：被控端 RemoteControlling 态下，3s 无远端帧自动回 Idle
//! 2. 本地强制恢复快捷键（Win: Ctrl+Alt+S）
//! 3. 统一状态快照推送给前端
//! 4. 结构化日志

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Notify;

/// Watchdog 超时阈值（毫秒）
const WATCHDOG_TIMEOUT_MS: u64 = 3000;

/// 状态快照（原子推送给前端，确保一致性）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateSnapshot {
    /// 连接状态
    pub connected: bool,
    /// 控制状态 (Idle/Armed/LocalControlling/RemoteControlling)
    pub control_state: String,
    /// Watchdog 是否激活
    pub watchdog_active: bool,
    /// 时间戳
    pub timestamp_ms: u64,
}

/// 状态守卫（管理 Watchdog 生命周期 + 快捷键注册）
pub struct StateGuard {
    /// 最后收到远端帧的时间戳（毫秒，0 表示未初始化）
    last_remote_frame_ms: Arc<AtomicU64>,
    /// Watchdog 是否运行中
    watchdog_running: Arc<AtomicBool>,
    /// 停止 Watchdog 的信号
    watchdog_stop: Arc<Notify>,
}

impl StateGuard {
    pub fn new() -> Self {
        Self {
            last_remote_frame_ms: Arc::new(AtomicU64::new(0)),
            watchdog_running: Arc::new(AtomicBool::new(false)),
            watchdog_stop: Arc::new(Notify::new()),
        }
    }

    /// 记录收到远端帧（由 inject_remote_frame 调用）
    pub fn touch_remote_frame(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_remote_frame_ms.store(now, Ordering::Relaxed);
    }

    /// 启动 Watchdog（进入 RemoteControlling 时调用）
    pub fn start_watchdog(&self, app: tauri::AppHandle) {
        if self.watchdog_running.swap(true, Ordering::SeqCst) {
            log::warn!("[StateGuard] Watchdog 已在运行，跳过重复启动");
            return;
        }

        // 初始化时间戳
        self.touch_remote_frame();

        let last_frame = Arc::clone(&self.last_remote_frame_ms);
        let running = Arc::clone(&self.watchdog_running);
        let stop = Arc::clone(&self.watchdog_stop);

        log::info!("[StateGuard] Watchdog 已启动（超时 {}ms）", WATCHDOG_TIMEOUT_MS);

        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.notified() => {
                        log::info!("[StateGuard] Watchdog 收到停止信号");
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }

                        let last = last_frame.load(Ordering::Relaxed);
                        if last == 0 {
                            continue; // 未初始化，跳过
                        }

                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);

                        let elapsed = now.saturating_sub(last);
                        if elapsed >= WATCHDOG_TIMEOUT_MS {
                            log::warn!(
                                "[StateGuard] ⚠️ Watchdog 超时！{}ms 未收到远端帧，强制回 Idle",
                                elapsed
                            );
                            // 触发强制回 Idle
                            trigger_force_idle(&app).await;
                            break;
                        }
                    }
                }
            }
            running.store(false, Ordering::SeqCst);
            log::info!("[StateGuard] Watchdog 已退出");
        });
    }

    /// 停止 Watchdog（离开 RemoteControlling 时调用）
    pub fn stop_watchdog(&self) {
        if self.watchdog_running.swap(false, Ordering::SeqCst) {
            self.watchdog_stop.notify_waiters();
            self.last_remote_frame_ms.store(0, Ordering::Relaxed);
            log::info!("[StateGuard] Watchdog 已停止");
        }
    }

    /// Watchdog 是否运行中
    pub fn is_watchdog_running(&self) -> bool {
        self.watchdog_running.load(Ordering::SeqCst)
    }
}

impl Default for StateGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建当前状态快照
pub fn build_snapshot(connected: bool, control_state: &str, watchdog_active: bool) -> StateSnapshot {
    StateSnapshot {
        connected,
        control_state: control_state.to_string(),
        watchdog_active,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    }
}

/// Watchdog 超时触发强制回 Idle
async fn trigger_force_idle(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(sm_state) = app.try_state::<Arc<crate::sharing::SharingManager>>() {
        let sm = sm_state.inner().clone();
        log::warn!("[StateGuard] Watchdog 触发 on_disconnected（强制回 Idle）");
        sm.on_disconnected(app).await;
    } else {
        log::error!("[StateGuard] 无法获取 SharingManager，Watchdog 恢复失败");
    }
}
```

**Step 2: 在 main.rs 添加 mod 声明**

在 `src-tauri/src/main.rs` 的 mod 声明区域添加：
```rust
mod state_guard;
```

**Step 3: 验证编译**

Run: `cd /Users/hanyufei/Documents/Git/KVMini && cargo check -p kvmini`
Expected: 编译通过（state_guard 模块被正确引入）

**Step 4: Commit**

```bash
git add src-tauri/src/state_guard.rs src-tauri/src/main.rs
git commit -m "feat(state-guard): 新增 state_guard 模块骨架（Watchdog + 状态快照）"
```

---

### Task 2: 集成 Watchdog 到 SharingManager

**Files:**
- Modify: `src-tauri/src/sharing.rs`

**Step 1: 在 SharingManager 中集成 StateGuard**

在 `SharingManager` 结构体新增字段：
```rust
/// 状态守卫（Watchdog + 快捷键）
state_guard: Arc<crate::state_guard::StateGuard>,
```

在 `SharingManager::new()` 中初始化：
```rust
state_guard: Arc::new(crate::state_guard::StateGuard::new()),
```

**Step 2: inject_remote_frame 中调用 touch_remote_frame**

在 `inject_remote_frame` 方法的状态检查之后、帧处理之前，添加：
```rust
// StateGuard: 记录最后收到远端帧的时间
self.state_guard.touch_remote_frame();
```

**Step 3: 进入 RemoteControlling 时启动 Watchdog**

在 `handle_one` 的 `StateEvent::StateChanged` 分支中，当 `to == RemoteControlling` 时启动 Watchdog：
```rust
// StateGuard: 进入 RemoteControlling 启动 Watchdog
if to == ControlState::RemoteControlling {
    if let Some(app) = app {
        self.state_guard.start_watchdog(app.clone());
    }
}
```

**Step 4: 离开 RemoteControlling 时停止 Watchdog**

当 `from == RemoteControlling` 时停止 Watchdog：
```rust
// StateGuard: 离开 RemoteControlling 停止 Watchdog
if from == ControlState::RemoteControlling {
    self.state_guard.stop_watchdog();
}
```

**Step 5: on_disconnected 中也确保停止 Watchdog**

在 `on_disconnected` 方法的兜底部分添加：
```rust
// 5. 停止 Watchdog（防止已超时但 on_disconnected 先到的竞态）
self.state_guard.stop_watchdog();
```

**Step 6: 验证编译**

Run: `cd /Users/hanyufei/Documents/Git/KVMini && cargo check -p kvmini`
Expected: 编译通过

**Step 7: Commit**

```bash
git add src-tauri/src/sharing.rs
git commit -m "feat(state-guard): 集成 Watchdog 到 SharingManager（3s 无帧自动回 Idle）"
```

---

### Task 3: Win 被控端 Ctrl+Alt+S 本地强制回 Idle

**Files:**
- Modify: `src-tauri/src/platform/windows/input_capture.rs`
- Modify: `src-tauri/src/state_guard.rs`

**Step 1: 在 keyboard_hook_proc 中检测 Ctrl+Alt+S**

在 `keyboard_hook_proc` 的 `WM_KEYDOWN | WM_SYSKEYDOWN` 分支中，`should_swallow()` 检查之前，添加安全恢复快捷键检测：

```rust
// StateGuard: Ctrl+Alt+S 本地强制回 Idle（安全阀）
// 即使处于 RemoteControlling 吞事件状态，此组合键也必须放行并触发恢复
if !is_injected && vk_code == 0x53 { // VK_S = 0x53
    let ctrl_down = (winapi::um::winuser::GetAsyncKeyState(winapi::um::winuser::VK_CONTROL) as u16 & 0x8000) != 0;
    let alt_down = (winapi::um::winuser::GetAsyncKeyState(winapi::um::winuser::VK_MENU) as u16 & 0x8000) != 0;
    if ctrl_down && alt_down {
        log::warn!("[Win][StateGuard] Ctrl+Alt+S 安全恢复快捷键触发！强制回 Idle");
        // 通过 preempt 通道通知 sharing 层（复用已有的 preempt 机制）
        feed_preempt(PhysicalInputSample::KeyDown);
        // 直接把 control_state 设回 Idle（立即停止吞事件）
        HOOK_CONTROL_STATE.with(|cell| {
            if let Some(state) = cell.borrow().as_ref() {
                state.store(CONTROL_STATE_IDLE, Ordering::Relaxed);
            }
        });
        // 放行此键（不吞），让系统正常处理
        return winapi::um::winuser::CallNextHookEx(
            std::ptr::null_mut(), n_code, w_param, l_param
        );
    }
}
```

**Step 2: macOS 端对称实现（在 input_capture.rs 中）**

在 `src-tauri/src/platform/macos/input_capture.rs` 的 CGEventTap 回调中增加类似检测：
检测 Ctrl+Option+S（macOS 对应 Ctrl+Alt+S）时强制回 Idle。

**Step 3: 验证编译**

Run: `cd /Users/hanyufei/Documents/Git/KVMini && cargo check -p kvmini`
Expected: 编译通过

**Step 4: Commit**

```bash
git add src-tauri/src/platform/windows/input_capture.rs src-tauri/src/platform/macos/input_capture.rs
git commit -m "feat(state-guard): Ctrl+Alt+S 本地强制回 Idle 安全阀（Win+Mac）"
```

---

### Task 4: 前端状态归一化（sharing-state-sync 事件）

**Files:**
- Modify: `src-tauri/src/sharing.rs` (发送 sync 事件)
- Modify: `src/main.js` (监听 sync 事件)

**Step 1: 后端每次状态变化后推送完整快照**

在 `sharing.rs` 的 `handle_one` 中 `StateEvent::StateChanged` 分支的 `emit("control-state-changed")` 之后，追加推送完整状态快照：

```rust
// StateGuard: 推送完整状态快照（消除前端事件顺序依赖）
let snapshot = crate::state_guard::build_snapshot(
    // 连接状态需要从 ConnectionManager 读取
    is_connected_via_app(app).await,
    state_name(to),
    self.state_guard.is_watchdog_running(),
);
let _ = app.emit("sharing-state-sync", snapshot);
```

在 `connection/mod.rs` 的事件循环中，连接状态变化时也推送快照。

**Step 2: 前端监听 sharing-state-sync**

```javascript
// 监听完整状态快照（StateGuard 推送，原子更新，消除事件顺序问题）
window.__TAURI__.event.listen('sharing-state-sync', (event) => {
    const snapshot = event.payload || {};
    console.log('[KVMini] 状态快照:', JSON.stringify(snapshot));
    
    // 原子更新 appState
    appState.connected = snapshot.connected;
    
    // 更新控制状态 UI
    updateControlStateUI(snapshot.control_state);
    updateSharingButtonForState(snapshot.control_state);
    
    // 更新连接状态 UI
    setConnectionStatus(snapshot.connected ? 'connected' : 'disconnected');
    updateConnectionButtons();
    
    // Watchdog 指示器（可选：显示被控端保护状态）
    if (snapshot.watchdog_active) {
        console.log('[KVMini] Watchdog 保护中');
    }
}).catch(e => console.warn('sharing-state-sync 事件监听注册失败:', e));
```

**Step 3: 验证编译 + 前端语法检查**

Run: `cd /Users/hanyufei/Documents/Git/KVMini && cargo check -p kvmini`
Expected: 编译通过

**Step 4: Commit**

```bash
git add src-tauri/src/sharing.rs src-tauri/src/state_guard.rs src/main.js
git commit -m "feat(state-guard): 前端状态归一化（sharing-state-sync 完整快照推送）"
```

---

### Task 5: 增强日志系统

**Files:**
- Modify: `src-tauri/src/sharing.rs`
- Modify: `src-tauri/src/connection/mod.rs`
- Modify: `src-tauri/src/state_guard.rs`
- Modify: `src-tauri/src/platform/windows/input_capture.rs`

**Step 1: 状态转换日志格式化**

所有状态转换日志统一格式：
```
[StateGuard] ⟨时间⟩ STATE_CHANGE: ⟨from⟩ → ⟨to⟩ (trigger=⟨原因⟩)
```

原因枚举：
- `edge_trigger`: 边缘检测触发
- `remote_switch`: 远端 SwitchControl
- `hotkey`: 快捷键切换
- `disconnect`: 连接断开
- `watchdog_timeout`: Watchdog 超时
- `force_recovery`: Ctrl+Alt+S 强制恢复
- `preempt`: 物理输入抢占

**Step 2: 连接断开链路日志**

在 `connection/mod.rs` 事件循环中 `ConnectionStateChanged(Disconnected)` 分支添加：
```rust
log::warn!(
    "[StateGuard] 🔌 连接断开检测到！正在触发 on_disconnected... (peer_ip={})",
    ip
);
```

**Step 3: Watchdog 日志**

Watchdog 每秒打一次 trace 日志（可通过 RUST_LOG 控制）：
```rust
log::trace!(
    "[StateGuard] Watchdog tick: elapsed={}ms, threshold={}ms",
    elapsed, WATCHDOG_TIMEOUT_MS
);
```

**Step 4: Hook 层日志**

Win hook 线程中 `should_swallow()` 返回 true 时每 100 次打一次 trace：
```rust
// 避免日志爆炸：每 100 次吞事件记一次
static SWALLOW_COUNT: AtomicU64 = AtomicU64::new(0);
let count = SWALLOW_COUNT.fetch_add(1, Ordering::Relaxed);
if count % 100 == 0 {
    log::trace!("[Win][Hook] 已吞 {} 个事件 (state=RemoteControlling)", count);
}
```

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(state-guard): 增强日志系统（结构化状态转换 + 断连链路 + Watchdog + Hook）"
```

---

### Task 6: 测试用例

**Files:**
- Create: `__tests__/state-guard-watchdog.test.ts`
- Modify: `kvm-core/src/state.rs` (补充 Watchdog 触发回 Idle 的测试)

**Step 1: Watchdog 超时测试**

```typescript
// __tests__/state-guard-watchdog.test.ts
describe('StateGuard Watchdog', () => {
    it('should auto-recover to Idle after 3s without remote frames', () => {
        // 模拟：进入 RemoteControlling 后 3s 无帧 → 自动回 Idle
        // 测试状态机层面的 on_disconnected 正确性
    });

    it('should NOT trigger if remote frames keep arriving', () => {
        // 模拟：持续 touch_remote_frame → Watchdog 不触发
    });

    it('should stop when leaving RemoteControlling normally', () => {
        // 模拟：正常 SwitchBack → Watchdog 停止，不误触发
    });
});
```

**Step 2: Ctrl+Alt+S 恢复测试**

```typescript
describe('Force Recovery Hotkey', () => {
    it('should detect Ctrl+Alt+S and force Idle', () => {
        // 模拟：RemoteControlling 态下检测到 Ctrl+Alt+S → control_state 回 Idle
    });
});
```

**Step 3: 状态快照一致性测试**

```typescript
describe('State Snapshot Sync', () => {
    it('should emit sharing-state-sync with correct fields', () => {
        // 验证快照包含 connected, control_state, watchdog_active, timestamp_ms
    });
});
```

**Step 4: Commit**

```bash
git add __tests__/state-guard-watchdog.test.ts kvm-core/src/state.rs
git commit -m "test(state-guard): Watchdog 超时恢复 + Ctrl+Alt+S + 状态快照测试"
```

---

### Task 7: 构建验证 + 集成测试

**Step 1: cargo test**

Run: `cd /Users/hanyufei/Documents/Git/KVMini && cargo test --workspace`
Expected: 所有测试通过

**Step 2: cargo check 双平台**

Mac:
```bash
cargo check -p kvmini
```

Win（交叉编译检查语法）：
```bash
cargo check -p kvmini --target x86_64-pc-windows-msvc
```

**Step 3: npm test（前端）**

```bash
npm test -- --testPathPattern='state-guard'
```

**Step 4: 构建**

```bash
./build.sh
```

---

## 预期效果

| 场景 | 修复前 | 修复后 |
|------|--------|--------|
| Mac 正常关闭 | TCP FIN → 立即 Disconnected → 恢复 ✅ | 同上 + 增强日志 ✅ |
| Mac WiFi 断/crash | 等 15s 心跳超时，期间键鼠锁死 ❌ | Watchdog 3s 自动恢复 ✅ |
| Mac 断后用户操作 | 只能 Ctrl+Alt+Delete | Ctrl+Alt+S 立即恢复 ✅ |
| 状态显示错误 | 事件竞态导致不一致 | sharing-state-sync 原子快照 ✅ |
| 调试困难 | 日志零散 | 结构化状态链路日志 ✅ |

---

## 风险评估

1. **Watchdog 误触发**：正常控制中网络延迟 > 3s 会导致误触发 → 缓解措施：3s 阈值比正常帧间隔（~16ms）宽裕 187 倍，即使 1fps 也不会触发
2. **Ctrl+Alt+S 冲突**：某些应用可能占用此快捷键 → 可接受，紧急恢复优先
3. **性能开销**：Watchdog 每 500ms 一次原子读，开销可忽略
