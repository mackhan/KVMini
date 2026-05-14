# KVMini v0.1.59 Changelog

发布日期：2026-05-14

---

## 中文

### 修复

- **BUG-041 Win→Mac 滚轮失效**：补全 Mac `inject_scroll` `dx==0 && dy==0` 早返回保险，给 Win 采集 / sharing pump / Mac 注入全链路埋入诊断日志，新增 17 个端到端 invariant 守恒测试
- **BUG-042 连接按钮文案错乱**：参考 Synergy 3 / Deskflow，废弃旧的「断开 + 重连」双按钮模型，统一为单按钮三态（Connect / Connecting / Disconnect），心跳波动的 `reconnecting` 事件不再误改按钮文案
- **BUG-043 杀进程键鼠锁死（第 4 次复发，纵深修复）**：新增 OS 层 TCP Keepalive（idle 5s + interval 3s × retries 2），应用层 Watchdog 超时阈值 3s → 10s，三层链路（5s/10s/15s）逐级生效，远端进程 kill 后本机键鼠在 ≤12s 内完全恢复，Ctrl+Alt+S 安全阀立即可用
- **BUG-044 跨端鼠标切换异常**：`dispatch_state_event` 行为与 `handle_one` 对称（Watchdog 生命周期、EdgeDetector active 切换、sharing-state-sync 推送）；`on_remote_switch_back` 在 EdgeReturn / Preempt 任意场景都进入 800ms 静默期 + reset_overshoot 防乒乓

### 新增

- **F5 全链路诊断日志基础设施**：Win/Mac 双端 capture / inject / sharing / connection / state_guard 关键路径埋点，按 trace/debug/info/warn 分级，可通过环境变量 `RUST_LOG=kvmini=debug` 打开

### 测试

- 新增 `__tests__/bug041-scroll-inject.test.ts` 17 个滚轮端到端 invariant 守恒测试
- 新增 `__tests__/bug042-connect-button-tri-state.test.ts` 18 个按钮三态测试
- 新增 `__tests__/bug043-watchdog-recovery.test.ts` 21 个纵深防御静态断言
- 新增 `__tests__/bug044-cross-end-switch.test.ts` 20 个跨端切换 RC 修复断言
- `kvm-core/src/network.rs` 新增 2 个 TCP Keepalive 单元测试
- 重写 `__tests__/bug-032-reconnect-button.test.js` 4 个静态断言（适配单按钮模型）

---

## English

### Fixed

- **BUG-041 Win→Mac scroll wheel not working**: added `dx==0 && dy==0` early-return guard in Mac `inject_scroll`, instrumented Win capture / sharing pump / Mac inject chain with diagnostic logs, added 17 end-to-end invariant tests
- **BUG-042 connect button caption mess**: inspired by Synergy 3 / Deskflow, deprecated dual `Disconnect + Reconnect` buttons, unified to single tri-state button (Connect / Connecting / Disconnect); heartbeat `reconnecting` event no longer corrupts button caption
- **BUG-043 killed-process keyboard/mouse lockup (4th recurrence, defense in depth fix)**: added OS-layer TCP Keepalive (idle 5s + interval 3s × retries 2), raised application-layer Watchdog timeout from 3s to 10s, three-layer chain (5s/10s/15s) cascades; local input restores within 12s after remote process killed, Ctrl+Alt+S safety valve always available
- **BUG-044 cross-end mouse switching anomalies**: `dispatch_state_event` now symmetric with `handle_one` (Watchdog lifecycle, EdgeDetector active toggle, sharing-state-sync push); `on_remote_switch_back` now enters 800ms silence period + reset_overshoot for both EdgeReturn and Preempt to prevent edge-bouncing

### Added

- **F5 full-chain diagnostic log infrastructure**: instrumented Win/Mac capture / inject / sharing / connection / state_guard hot paths with trace/debug/info/warn levels, enable via `RUST_LOG=kvmini=debug` env var

### Tests

- Added `__tests__/bug041-scroll-inject.test.ts` 17 tests
- Added `__tests__/bug042-connect-button-tri-state.test.ts` 18 tests
- Added `__tests__/bug043-watchdog-recovery.test.ts` 21 tests
- Added `__tests__/bug044-cross-end-switch.test.ts` 20 tests
- Added 2 TCP Keepalive unit tests in `kvm-core/src/network.rs`
- Rewrote `__tests__/bug-032-reconnect-button.test.js` 4 static assertions for single-button model
