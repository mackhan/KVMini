# v0.1.52 修复日志（2026-05-09）

## Bug 修复

### Bug 1：拖拽有响应但无中间过程（Mac 控制 Win）
- **根因**：`macos/input_capture.rs` 的 `cg_event_to_raw` 没有 `LeftMouseDragged/RightMouseDragged/OtherMouseDragged` 分支，拖拽时返回 `None`，channel 无帧，远端只收到 Down/Up，无中间移动轨迹
- **修复**：给三种拖拽 CGEventType 加与 `MouseMoved` 完全相同的 delta 读取逻辑（合并为 or-pattern 分支）

### Bug 2：滚轮不工作（Win 控制 Mac）
- **根因**：`inject_scroll` 之前改为 `SCROLL_UNIT_PIXEL(=2)` 模式但与 macOS 滚动机制不符
- **修复**：改回 `SCROLL_UNIT_LINE(=1)` + `lines = delta / 120`，Barrier 行业标准方案。±120 协议单位除以 120 = ±1 行，感受与本机鼠标一致

### Bug 3：MacInputBlocker 无效代码清理
- **根因**：`create_block_tap` 在 `CFRunLoop::get_current()` 注册 source 但不运行 RunLoop，屏蔽 Tap 永远不生效
- **修复**：删除无效 `create_block_tap`，`MacInputBlocker` 仅维护标志位，屏蔽由 `input_capture.rs` 的 `control_state == REMOTE_CONTROLLING` 分支（`return None`）双重保障

### Bug 4：Mac 本机触摸板被完全屏蔽（抢占交互流程不正确）
- **根因**：被控端（Mac）抢占夺回控制后发 `SwitchBackControl(Preempt)`，主控端（Win）回 Armed，但 EdgeDetector 未重置，主控端鼠标微动立即重新触发 LocalControlling，Mac 再次进入 RemoteControlling 被屏蔽
- **修复**：
  1. `EdgeDetector` 新增 `enter_silence(duration_ms)` 方法，设置不触发边缘的静默期
  2. 主控端收到 `Preempt` 回 Armed 后，对 EdgeDetector 设置 800ms 静默期（参照 Barrier "focus cooldown" 设计）
  3. 被控端抢占处理日志更新，明确注释交互模式

## 交互模式说明（正确的 KVM 流程）
```
主控端（Win）          被控端（Mac）
Armed → 鼠标推边缘 → LocalControlling  →  SwitchControl → RemoteControlling（屏蔽本机输入）
         ↑                                                        ↓
Armed（等待边缘）← SwitchBackControl(Preempt) ← 用户物理输入 → Idle（本机正常）
  + 800ms 静默期                                    
  （防止立即重新触发）
```

## 变更文件
- `src-tauri/src/platform/macos/input_capture.rs` — 拖拽分支修复
- `src-tauri/src/platform/macos/input_inject.rs` — 滚轮 LINE 模式修复
- `src-tauri/src/platform/macos/input_block.rs` — 删除无效 Tap 代码
- `kvm-core/src/edge_logic.rs` — 新增 `enter_silence` / `silence_until` 机制
- `src-tauri/src/sharing.rs` — 抢占后 EdgeDetector 静默期调用
- `__tests__/bug-scroll-win-ctrl-mac.test.ts` — 测试更新（LINE+/120 方案）

## 测试结果
60 套件 / 966 用例全部通过
