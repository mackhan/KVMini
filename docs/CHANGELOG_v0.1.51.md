# v0.1.51 修复日志 — Win 控制 Mac 键盘/滚轮/鼠标全面修复

**日期**: 2026-05-09

---

## Bug 修复

### BUG-040: 键盘完全不工作（Win 控制 Mac）

**根因**: Win 采集 VK 码直接发送，Mac 被控端当 CGKeyCode 使用，两套编码体系完全不兼容

**方案**: 参照 Barrier/Synergy 行业标准，引入 **X11 KeySym** 作为跨平台通用键码中间层

| 端 | 改动 | 文件 |
|----|------|------|
| Win 采集 | `keyboard_hook_proc` 采集 vkCode 后调 `vk_to_keysym()` 转换 | `windows/input_capture.rs` |
| Win 注入 | `keysym_to_vk()` 替换旧的 `cg_keycode_to_vk()` | `windows/input_inject.rs` |
| Mac 采集 | `cg_event_to_raw` 键盘分支调 `cg_keycode_to_keysym()` 转换 | `macos/input_capture.rs` |
| Mac 注入 | `inject_key` 调 `keysym_to_cg_keycode()` 转换后注入 | `macos/input_inject.rs` |

覆盖范围：字母、数字、符号、方向键、F1-F15、修饰键(Shift/Ctrl/Alt/Cmd)、空格/Tab/Enter/Esc/Backspace、数字键盘

### BUG-041a: 滚轮完全不工作（Win 控制 Mac）

**根因**: Win WM_MOUSEWHEEL 采集 ±120（WHEEL_DELTA），Mac `inject_scroll` 用 `SCROLL_UNIT_LINE=1` 注入，±120 被解释为 120 行 → 一次滚 120 行

**方案**: Mac `inject_scroll` 改用 `SCROLL_UNIT_PIXEL=2` 模式，±120px 在 macOS 上等效约 3-4 行，与 Barrier 协议 ±120 单位制一致

- 文件: `macos/input_inject.rs`
- `const SCROLL_UNIT_LINE: u32 = 1` → `const SCROLL_UNIT_PIXEL: u32 = 2`

### BUG-041b: 鼠标左键首次点击不响应

**根因**: `DecoupleCursor` 状态下（CGAssociate 断开物理鼠标绑定），`inject_mouse_button` 通过 `HIDSystemState` 读取的光标位置是物理鼠标坐标，而非屏幕光标坐标，导致点击命中错误位置

**方案**: 改用 `CGEventSourceStateID::CombinedSessionState` 读取合成光标位置（`CGWarpMouseCursorPosition` 后的准确坐标）

- 文件: `macos/input_inject.rs`

### BUG-042: 鼠标拖拽卡顿

**根因**: pump 每个 MouseMove 事件都独立 `send_frame_via_app`，网络 + tokio async 调度积压，视觉卡顿

**方案**: 参照 Barrier "Squash mouse moves" 策略，在 pump loop 中 drain channel 积压的 MouseMove 帧，`saturating_add` 累加 dx/dy，只发一帧合并的 `MouseMoveDelta`

- 文件: `sharing.rs`
- 新变量: `acc_dx`, `acc_dy`, `already_sent`
- 保障: 非 Move 帧（Button/Key/Scroll）不合并，立即发送；EdgeDetector 在 drain 循环内逐帧喂入保证精度

---

## 技术背景

### X11 KeySym 中间层（行业标准）

Barrier/Synergy 均采用 X11 KeySym 作为协议键码，避免平台差异：

```
Win 采集端                协议层         Mac 被控端
VK_A(0x41) ──vk_to_keysym()──> XK_a(0x0061) ──keysym_to_cg_keycode()──> CGKeyCode 0

Mac 采集端                协议层         Win 被控端
CGKeyCode 0 ──cg_keycode_to_keysym()──> XK_a(0x0061) ──keysym_to_vk()──> VK_A(0x41)
```

### 滚轮单位制

Barrier 协议也使用 ±120 单位制（与 WM_MOUSEWHEEL 一致），被控端各自处理：
- Win 被控端: `MOUSEEVENTF_WHEEL` 直接使用 ±120 ✓（已正常）  
- Mac 被控端: `SCROLL_UNIT_PIXEL` 模式传入 ±120px ✓（本次修复）

---

## 测试

新增测试用例：
- `__tests__/bug-keyboard-keysym-roundtrip.test.ts` — 转换表双向一致性（64 个用例）
- `__tests__/bug-scroll-win-ctrl-mac.test.ts` — PIXEL 模式 + CombinedSessionState 验证
- `__tests__/bug-pump-squash-move.test.ts` — Squash 合并逻辑静态分析

全量回归：**60 套件 / 966 用例全部通过**

---

## 版本

- 上个版本: v0.1.49
- 本版本: v0.1.51
- 构建产物: `KVMini_0.1.51_x64-setup.exe`（Win），Mac 端从 master 拉取构建
