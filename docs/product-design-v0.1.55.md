# KVMini v0.1.55 产品设计 — 被控端 capture 启动 + 鼠标按键释放兜底

> 发布日期：2026-05-11
> 上一版：v0.1.54
> 本版主题：修复被控端从未启动 capture 导致的连锁 bug + 跨屏拖动按键卡死

---

## 一、问题描述（用户视角）

| 症状编号 | 用户描述 | 影响平台 |
|---------|----------|---------|
| 症状 A | Win 被控状态下，Win 本机键鼠操作不能抢回控制权 | Win 被控 |
| 症状 B | Mac 主控跨屏拖动到 Win 被控端 → 松开鼠标后 Win 端窗口仍处于拖动状态 | 全平台 |
| 症状 C | Mac 被控时屏蔽实际失效，主控+被控两端都各自有鼠标在动，互相错乱 | 全平台 |

---

## 二、根因分析（第一性原理）

### 统一根因（症状 A + C）

**被控端从 Idle 直接进入 RemoteControlling 时，capture（Mac CGEventTap / Win Hook）从未启动。**

证据链：

```
StateMachine::switch_to_remote_controlling()  ← v0.1.54 之前
  → StateChanged + SendFrame(SwitchControlAck) + BlockInput + DecoupleCursor
  ✗ 没有 StartCapture
```

连锁后果：

1. **症状 A（Win 抢占失效）**：PreemptDetector 依赖 hook 回调喂 RawInputEvent 样本 → hook 没装 → detector 永远不触发 → Win 物理输入抢占功能事实上从未生效（v0.1.43 设计 vs 实施脱节）。

2. **症状 C-Win（屏蔽失效）**：Win `WinInputBlocker::block_input` 主要靠 hook 软屏蔽（`BlockInput` Win32 API 非管理员权限静默失败），hook 没装 → 软屏蔽实际失效。

3. **症状 C-Mac（屏蔽失效）**：Mac `MacInputBlocker::block_input` 注释明写「实际屏蔽由 input_capture.rs 的 `control_state == REMOTE_CONTROLLING` 分支完成（CGEventTap 回调返回 None）」，Tap 没装 → MacInputBlocker 也实际失效。

### 独立根因（症状 B）

**LocalControlling 退出（边缘回切 / 抢占 / 断连）时，主控端不追踪未释放鼠标按键，也不给被控端补发 MouseUp。**

时序：

```
T0: 主控按左键 → pump 转发 MouseDown 帧 → 被控注入 LeftButton.Down
T1: 拖动越屏 / 撞反向边缘 → 主控 LocalControlling → Armed
T2: 主控端 capture 停 → 物理鼠标松开时 hook 收到 MouseUp 但不发帧
T3: 被控端永远等不到 LeftButton.Up → 拖动状态卡死
```

---

## 三、解决方案（奥卡姆剃刀 + 双端共享）

### 修复 A：被控端入态时启动 capture

**核心改动**：`kvm-core/src/state.rs::switch_to_remote_controlling`

```rust
vec![
    StateEvent::StateChanged { from, to: self.state },
    StateEvent::SendFrame(KvmFrame::SwitchControlAck),
    StateEvent::StartCapture,    // ← v0.1.55 新增
    StateEvent::BlockInput,
    StateEvent::DecoupleCursor,
]
```

**顺序设计**：先 StartCapture → 后 BlockInput。理由：BlockInput 在 Mac/Win 都依赖 hook/Tap 已装才能吞事件，反过来不行。

**对称释放**：`switch_to_idle` 的 RemoteControlling 分支追加 `StopCapture`，避免 hook/Tap 常驻消耗。

**事件链路修复**：`sharing.rs::inject_remote_frame` 首帧到达时把 `handle_state_events_silent(evs)` 改为 `handle_state_events_with_emit(evs, app)`。silent 版本由于 `app=None`，handle_one 里 StartCapture 分支会被 `if let Some(app)` 守卫静默丢弃。删除已废弃的 silent 版本。

### 修复 B：鼠标按键追踪与释放（与 ReleaseModifiers 对称）

**StateMachine 新增字段**：`pressed_mouse_buttons: u8`（5 个按键的位掩码）。

**新增 API**：

```rust
pub fn track_mouse_button_down(&mut self, btn: MouseButton);
pub fn track_mouse_button_up(&mut self, btn: MouseButton);
pub fn drain_pressed_mouse_buttons(&mut self) -> Vec<MouseButton>;
pub fn has_pressed_mouse_buttons(&self) -> bool;
```

**新增事件**：`StateEvent::ReleaseMouseButtons(Vec<MouseButton>)`。

**触发点**：`local_to_armed` / `switch_to_idle`(LocalControlling 分支) drain 仍按下的按键并产生事件。

**pump 任务接入**：`start_capture_and_pump` 在 `is_local` 分支识别 `KvmFrame::MouseButton{pressed}` 调 track_mouse_button_*（含 squash 路径中的非 Move 帧）。

**事件处理**：`handle_one` 与 `handle_events_on_edge_trigger` 都新增 `ReleaseMouseButtons` 分支，遍历按键发 `KvmFrame::MouseButton{pressed: false}` 给被控端。

---

## 四、改动文件清单

```
kvm-core/src/state.rs           StateMachine 加按键追踪 + 调整事件序列；新增 11 个测试
src-tauri/src/sharing.rs        pump 追踪 + handle_one 新增分支 + inject_remote_frame 改 with_emit + 删除 silent
src-tauri/src/main.rs           get_changelog 新增 v0.1.55 中英文条目
Cargo.toml                      version 0.1.54 → 0.1.55
package.json                    version 0.1.54 → 0.1.55
src/index.html                  meta[name=app-version] 0.1.54 → 0.1.55
docs/bug-list.md                新增 Bug #037 / Bug #038 完整记录
docs/product-design-v0.1.55.md  本文（按日期归档铁律）
```

---

## 五、测试覆盖

### 单元测试（kvm-core/src/state.rs，新增 11 个）

| 测试 | 覆盖点 |
|------|-------|
| `test_switch_to_remote_controlling_emits_start_capture` | RemoteControlling 入态包含 StartCapture |
| `test_switch_to_remote_controlling_start_capture_before_block_input` | StartCapture 必须在 BlockInput 之前 |
| `test_switch_to_idle_from_remote_emits_stop_capture` | 退出 RemoteControlling 必须 StopCapture |
| `test_track_mouse_button_down_then_up_cleared` | down/up 配对正确清零 |
| `test_drain_pressed_mouse_buttons_returns_all` | drain 返回所有按键并清空 |
| `test_local_to_armed_with_pressed_buttons_emits_release` | local→armed 有按键时发 release |
| `test_local_to_armed_without_pressed_buttons_no_release` | 无按键时不发多余事件 |
| `test_switch_to_idle_from_local_releases_pressed_buttons` | switch_to_idle 路径释放 |
| `test_on_disconnected_from_local_releases_pressed_buttons` | 断连兜底释放 |
| `test_force_idle_clears_pressed_buttons` | force_idle 清空状态 |

### 回归测试

- `cargo test --workspace`：kvm-core 164 通过 + kvmini 32 通过
- `cargo check -p kvmini`：编译通过，无新增 warning

### 手动回归（构建后）

| 场景 | 预期 |
|------|------|
| Win 被控 + Mac 主控，Win 本机键盘任意键抬起 | Win 端 PreemptDetector 触发 → SwitchBackControl → Mac 主控回 Armed + Win 解锁 |
| Mac 主控按住左键拖窗口越屏到 Win 被控，松开 | Win 端窗口正常释放，不再卡拖动 |
| Mac 主控控制 Win 被控时，Win 本机鼠标乱动 | Win 鼠标无任何效果（hook 软屏蔽生效） |
| Mac 被控 + Win 主控时，Mac 本机鼠标乱动 | Mac 鼠标无任何效果（CGEventTap 吞事件生效） |

---

## 六、架构图（关键链路）

```
[主控端 LocalControlling]                        [被控端 RemoteControlling]
   │                                                  │
   ├─ pump.spawn → RawInputEvent::MouseDown           │
   │     └→ state.track_mouse_button_down(Left)       │
   │     └→ raw_to_frame → KvmFrame::MouseButton ─────▶ inject_remote_frame
   │                                                  │     └→ 首帧时 switch_to_remote_controlling
   │                                                  │           └→ StartCapture (v0.1.55 新增)
   │                                                  │           └→ BlockInput  ← hook 已装可吞
   │                                                  │           └→ DecoupleCursor
   │                                                  │           └→ capture pump 启动
   │                                                  │                └→ control_state==REMOTE → 吞本机键鼠
   │                                                  │                └→ PreemptDetector 喂样本
   │                                                  │                     └→ 物理输入抢占触发
   │                                                  │
   ├─ EdgeDetector triggered (反向边缘)               │
   │   └→ state.local_to_armed()                      │
   │         ├ ReleaseMouseButtons([Left]) (v0.1.55)──▶ inject_frame(MouseButton{pressed:false})
   │         ├ StopCapture                            │     └→ Win 端窗口正常释放
   │         └ ShowCursor                             │
```

---

## 七、向前兼容性

- KvmFrame 协议**未新增字段**——所有改动局限在 StateMachine 内部 + sharing.rs 事件处理；
- v0.1.55 主控端 ↔ v0.1.54 被控端：被控端仍按旧路径不启动 capture（兼容降级），但抢占/屏蔽不工作（与 v0.1.54 行为一致）；
- v0.1.54 主控端 ↔ v0.1.55 被控端：被控端按 v0.1.55 启动 capture，抢占/屏蔽生效；主控端仍不发释放帧，跨屏拖动卡死问题在主控端尚未升级时仍存在；
- **结论**：单端升级即可获得部分修复，双端升级才能完整修复 3 个症状。

---

## 八、版本号

- 旧版：v0.1.54
- 新版：v0.1.55
- 升级位置：Cargo.toml / package.json / src/index.html meta / main.rs::get_changelog
- UI 显示：通过 initVersion → get_app_version 后端命令读取 CARGO_PKG_VERSION，无需 HTML 硬编码同步
