# KVMini v0.1.43 产品设计文档

**版本**：0.1.43
**日期**：2026-05-09
**主题**：远端虚拟光标显示修复（Bug #034） + 远端物理输入抢占回切（Feature #035，Synergy / Logitech Flow 风格）

---

## 1. 背景与目标

### 1.1 问题清单

v0.1.42 已实现鼠标/键盘 delta 共享、4 态状态机、Mac/Win 双端对称，但仍有两个体验缺口：

1. **Bug #034 远端虚拟光标不显示**：本机撞边进入 `LocalControlling` 后，远端（被控端）屏幕看不到虚拟光标，只能「盲点」。delta 事件已到达 sharing 层、`cursor.set_position()` 也被调用，但视觉层面光标静止或卡在屏幕边缘半隐半现。
2. **Feature #035 被控端要能一键夺回**：当前被控端只能等本机用户手动按 `Ctrl+Alt+S` 或撞到反向边缘，才能结束共享。主流 KM 共享产品（Synergy、Barrier、Logitech Flow）都有「被控端任意键鼠即恢复本机控制」的抢占机制，KVMini 缺失。

### 1.2 设计目标

- **修复 Bug #034**：远端用户在整个 `RemoteControlling` 生命周期都能看到虚拟光标随本机鼠标实时移动（不止「落点正确」，而是「视觉流畅」）。
- **实现 Feature #035**：被控端检测本机物理键鼠活动 → 阈值命中立即终止共享 → 控制端秒退 `Armed` 态并弹 toast 通知用户。
- **Mac + Win 行为与 UI 完全一致，代码隔离**。
- **TDD 全程护航**，新增/扩展测试覆盖协议兼容、阈值边界、状态机流转、i18n key 完整性。

---

## 2. Bug #034 根因与修复

### 2.1 根因链路

1. 被控端进入 `RemoteControlling` 时，状态机只发 `BlockInput` 事件，**没有解耦物理光标**。
2. macOS `CGAssociateMouseAndMouseCursorPosition` 在默认关联态下，`CGWarpMouseCursorPosition` 的调用会被物理鼠标 delta 瞬间覆盖（物理鼠标不动，但被 block input 吞掉），呈现出「光标被钉在某个点」的视觉假象。
3. 进一步的历史隐患：`cursor.rs::warp_cursor` 在 `!grabbed` 分支每次 warp 后都调用 `CGAssociate(1)`，把被控端刚解耦的光标又悄悄关联回去。

### 2.2 修复方案（第一性原理 + 奥卡姆剃刀）

- 新增事件 `StateEvent::DecoupleCursor` / `RecoupleCursor`。
- `state.rs::switch_to_remote_controlling` 在 `BlockInput` 后追加 `DecoupleCursor`。
- `state.rs::switch_to_idle`（来自 `RemoteControlling` 分支）追加 `RecoupleCursor`。
- `CursorController` trait 增加默认 no-op 的 `decouple_physical_mouse()` / `recouple_physical_mouse()`。
- **Mac 实现**：
  - `decouple_physical_mouse` = `CGAssociateMouseAndMouseCursorPosition(0)` + `NSCursor unhide`（保证光标可见）。
  - `recouple_physical_mouse` = `CGAssociateMouseAndMouseCursorPosition(1)`。
  - **删除** `warp_cursor` 中 `!grabbed` 分支的 `CGAssociate(1)`，避免 warp 后反向关联。
- **Win 实现**：Win 平台物理鼠标与光标天然不耦合，`SetCursorPos` 即可移动光标，保持 trait 默认 no-op 即可。

---

## 3. Feature #035 设计

### 3.1 触发条件（对齐 Synergy / Logitech Flow）

- **鼠标位移**：被控端 `RemoteControlling` 态下，累计欧氏位移 ≥ **40 像素**（平方和 `dx² + dy² ≥ 1600` 避免 sqrt）。
- **键盘**：**任意一次** `KeyDown` 立即触发。
- 阈值硬编码常量 `PREEMPT_MOVE_THRESHOLD_PX = 40`，不做设置项（第一性原理：用户要的是「能夺回」而不是「调参数」）。

### 3.2 核心算法（kvm-core/src/preempt.rs）

```rust
pub const PREEMPT_MOVE_THRESHOLD_PX: i32 = 40;
pub const PREEMPT_MOVE_THRESHOLD_SQ: i64 = 1600; // 40 * 40

pub enum PhysicalInputSample { Move { dx: i32, dy: i32 }, KeyDown }
pub enum PreemptCause { MouseMove, KeyDown }
pub enum PreemptDecision { Idle, ShouldPreempt { cause: PreemptCause } }

pub struct PreemptDetector {
    accumulated_sq: AtomicI64,
    triggered: AtomicBool,
}

impl PreemptDetector {
    pub fn feed(&self, sample: PhysicalInputSample) -> PreemptDecision { /* 无锁累加 */ }
    pub fn reset(&self);
}
```

- **无锁**：原子操作 O(1)、无互斥锁，不阻塞 CGEventTap / LowLevelHook 实时路径。
- **防抖动**：触发一次后 `triggered.swap(true)` 置位，后续 feed 直接返回 `Idle`，直到 `reset()`。
- **重置时机**：每次进入 `RemoteControlling` 时由 capture 层 `reset()`。

### 3.3 合成事件过滤（关键安全阀）

被控端自身会注入本机鼠标/键盘事件（例如远端控制的 scroll/click），必须排除自己的注入事件否则会**自我抢占**。

- **Mac**：inject 时 `event.set_integer_value_field(kCGEventSourceUserData=42, 0x4B56_4D49 = "KVMI")`；capture 回调用同一字段值识别并跳过。
- **Win**：`LowLevelMouseProc` 使用 `LLMHF_INJECTED` flag；`LowLevelKeyboardProc` 使用 `LLKHF_INJECTED` flag；`SendInput` 会自动打标，无需手动 tag。

### 3.4 协议扩展

`KvmFrame::SwitchBackControl` 新增 `reason: SwitchBackReason` 字段：

```rust
pub enum SwitchBackReason { EdgeReturn = 0, Preempt = 1 }
```

- payload 由 0 字节变为 **1 字节**。
- **向后兼容**：decode 时 `payload.is_empty()` → `EdgeReturn`，保证 v0.1.42 及以下客户端发来的空 payload 帧仍能解析。
- 新增 4 个测试（encode/decode/legacy/reason round-trip），全部 29 protocol 测试通过。

### 3.5 通路（被控端 → 控制端）

```
物理事件 → capture 回调（过滤合成 + 过滤非 RemoteControlling）
         → PreemptDetector.feed()
         → ShouldPreempt
         → preempt_callback → preempt_tx.send(cause)
         → pump task (tokio::select!) 接收
         → send SwitchBackControl{Preempt} 上行
         → on_disconnected + state.switch_to_idle(UnblockInput + RecoupleCursor)
         → 控制端收到 SwitchBackControl{Preempt}
         → state.local_to_armed + emit("kvm://preempted")
         → 前端 showSuccess(t('preempt_resume_toast'))  // 3s toast
```

### 3.6 UI 反馈

- **控制端**：3 秒 toast「远端用户恢复了本机控制 / Remote user resumed control」，复用既有 `showSuccess()`。
- **被控端**：**静默**，用户刚刚用物理键鼠抢占，无需再弹窗打扰。
- **i18n 双语 key**：`preempt_resume_toast` 同时写入 `src/i18n.js` zh + en 字典。

---

## 4. 平台实现

### 4.1 Mac 端

- `platform/macos/input_capture.rs`
  - 新增 `preempt_detector: Arc<PreemptDetector>` + `preempt_callback: Arc<Mutex<Option<Box<dyn Fn(PreemptCause) + Send + Sync>>>>`。
  - `set_control_state` 切到 RemoteControlling 时 `reset()`。
  - CGEventTap 回调：仅 RemoteControlling、过滤 `kCGEventSourceUserData == KVMINI_INJECT_USER_DATA`、对 MouseMoved 读 `kCGMouseEventDeltaX/Y` 喂 `Move`、对 KeyDown 喂 `KeyDown`；触发时 `callback(cause)`。
- `platform/macos/input_inject.rs`
  - 新增 `tag_synthetic(&event)` helper，在所有 4 个 inject 函数（mouse_move/mouse_button/key/scroll）的 `event.post()` 前调用。
- `platform/macos/cursor.rs`
  - 实现 `decouple_physical_mouse` / `recouple_physical_mouse`。
  - 修复 `warp_cursor` 不再误调 `CGAssociate(1)`。

### 4.2 Win 端

- `platform/windows/input_capture.rs`
  - 镜像 Mac 字段 / 行为。
  - `mouse_hook_proc` / `keyboard_hook_proc` 用 `LLMHF_INJECTED_MASK = 0x3` / `LLKHF_INJECTED_MASK = 0x12` 过滤。
  - 新增 4 个单元测试覆盖 enter REMOTE 重置 / set_preempt_callback / 非 REMOTE 不重置。
- `platform/windows/cursor.rs`：保持默认 no-op（平台特性天然适配）。

### 4.3 跨平台核心

- `kvm-core/src/preempt.rs`：**NEW**，18 单元测试。
- `kvm-core/src/protocol.rs`：扩展 `SwitchBackReason` + 4 新测试。
- `kvm-core/src/state.rs`：新增 `DecoupleCursor` / `RecoupleCursor` 事件。
- `kvm-core/src/platform.rs`：`CursorController` trait 增加默认 no-op 方法。

---

## 5. 测试矩阵

| 层 | 数量 | 状态 |
|---|---|---|
| kvm-core/preempt 单元 | 18 | ✅ |
| kvm-core/protocol（含新增 4） | 29 | ✅ |
| kvm-core/state（扩展） | 32 | ✅ |
| src-tauri 其余 Rust 测试 | 169 | ✅ |
| **Rust 合计** | **248** | ✅ |
| jest i18n（含新增 `preempt_resume_toast` 守卫） | 33 | ✅ |

---

## 6. 版本兼容与回归

- **协议兼容**：v0.1.42 控制端连 v0.1.43 被控端：被控端抢占发 `{reason=Preempt}`（1 字节 payload），老控制端 decode 忽略多余字节按空 payload 解析为 `EdgeReturn`，行为退化为普通回切，**无崩溃无错误**。
- **v0.1.43 控制端连 v0.1.42 被控端**：被控端无抢占能力，但不影响其他功能。
- 光标 decouple 仅在 `RemoteControlling` 时生效，切回 Idle 自动 recouple，**不影响本机日常使用**。

---

## 7. 开发规则遵守清单

- [x] 第一性原理：阈值硬编码 40px / 任意键（不做设置项）
- [x] 奥卡姆剃刀：用 `PreemptDetector` 单模块替代散落在两平台的自实现
- [x] TDD：Red → Green → Refactor，每一项先写测试
- [x] i18n 铁律：zh + en 同时加 `preempt_resume_toast`，并加守卫测试
- [x] Mac + Win 代码隔离、行为一致
- [x] 版本号唯一真相 Cargo.toml workspace，package.json + index.html + changelog 同步
- [x] bug-list.md 归档 Bug #034 / Feature #035
- [x] build.sh 构建产物带版本号（由脚本统一处理）
