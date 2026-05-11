/**
 * BUG-040: Win 主控 → Mac 被控 滚轮无响应 + 拖拽无中间过程 + 无法选择文字
 *
 * 统一根因：
 *   1. CGEventTap 在 RemoteControlling 下无差别吞掉所有事件（包括自注入的合成事件）
 *      → 滚轮/按键注入后被 tap 吞掉，系统从未收到
 *   2. handle_remote_mouse_delta 用 CGWarpMouseCursorPosition 移动光标
 *      → Apple 文档："moves cursor WITHOUT generating or posting an event"
 *      → 拖拽需要 LeftMouseDragged 事件，文字选择也需要，但从未生成
 *
 * 修复（v0.1.56）：
 *   Fix 1: CGEventTap 在 RemoteControlling 下对合成事件（KVMINI_INJECT_USER_DATA tag）放行
 *   Fix 2: handle_remote_mouse_delta 改用 executor.move_mouse（CGEvent post），不再用 CGWarpMouseCursorPosition
 *   Fix 3: Win hook 在 RemoteControlling 下对 is_injected 事件放行
 *
 * 竞品参考：Input Leap/Barrier 的 OSXScreen::fakeMouseMove 也用 CGEventPost 而非 CGWarp
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fix 1: Mac CGEventTap 对合成事件放行
// ═══════════════════════════════════════════════════════════════════════════════
describe('BUG-040 Fix1: Mac CGEventTap — RemoteControlling 合成事件放行', () => {
  const src = () => read('src-tauri/src/platform/macos/input_capture.rs');

  it('RemoteControlling 分支中必须检查 KVMINI_INJECT_USER_DATA 决定放行/吞', () => {
    const content = src();
    // 在事件吞掉/放行的决策区块中，RemoteControlling 必须有 KVMINI_INJECT_USER_DATA 检查
    // 而不是和 LocalControlling 合并为一个 match arm 无差别吞掉
    expect(content).toMatch(/CONTROL_STATE_REMOTE_CONTROLLING\s*=>/);
    // RemoteControlling 分支内必须有 is_synthetic / KVMINI_INJECT_USER_DATA 判断
    const remoteBlock = content.match(
      /CONTROL_STATE_REMOTE_CONTROLLING\s*=>\s*\{[\s\S]*?\n\s{16}\}/
    )?.[0] ?? '';
    expect(remoteBlock).toMatch(/is_synthetic|KVMINI_INJECT_USER_DATA/);
  });

  it('RemoteControlling 合成事件放行（返回 Some）', () => {
    const content = src();
    // 在 RemoteControlling 分支中，合成事件路径必须返回 Some(event.to_owned())
    const remoteBlock = content.match(
      /CONTROL_STATE_REMOTE_CONTROLLING\s*=>\s*\{[\s\S]*?\n\s{16}\}/
    )?.[0] ?? '';
    expect(remoteBlock).toMatch(/Some\(event\.to_owned\(\)\)/);
  });

  it('LocalControlling 仍然无差别吞掉（不放行合成事件）', () => {
    const content = src();
    // LocalControlling 是主控端，应该吞掉所有本机事件（转发给被控端）
    // 不需要放行合成事件（主控端没有自注入场景）
    const localBlock = content.match(
      /CONTROL_STATE_LOCAL_CONTROLLING\s*=>\s*\{[\s\S]*?\n\s{16}\}/
    )?.[0] ?? '';
    expect(localBlock).toMatch(/None/);
    // LocalControlling 不应该有 is_synthetic 判断
    expect(localBlock).not.toMatch(/is_synthetic|KVMINI_INJECT_USER_DATA/);
  });

  it('Idle/Armed 仍然完全放行', () => {
    const content = src();
    expect(content).toMatch(/Idle\/Armed.*完全放行|完全放行.*防误吞/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Fix 2: handle_remote_mouse_delta 改用 executor.move_mouse
// ═══════════════════════════════════════════════════════════════════════════════
describe('BUG-040 Fix2: handle_remote_mouse_delta — 使用 move_mouse 而非 set_position', () => {
  const src = () => read('src-tauri/src/sharing.rs');

  it('handle_remote_mouse_delta 的 Warp 分支必须调用 executor.move_mouse', () => {
    const content = src();
    // 在 RemoteCursorAction::Warp 分支中必须有 executor.move_mouse 或 self.executor.move_mouse
    const warpBlock = content.match(
      /RemoteCursorAction::Warp\s*\{[\s\S]*?(?=RemoteCursorAction::SwitchBack)/
    )?.[0] ?? '';
    expect(warpBlock).toMatch(/executor\.move_mouse/);
  });

  it('handle_remote_mouse_delta 不再使用 cursor.set_position 作为主要移动手段', () => {
    const content = src();
    // Warp 分支内不应该有 cursor.set_position（或者如果有也必须在 move_mouse 之后/作为 fallback 注释说明）
    const warpBlock = content.match(
      /RemoteCursorAction::Warp\s*\{[\s\S]*?(?=RemoteCursorAction::SwitchBack)/
    )?.[0] ?? '';
    // cursor.set_position 不应作为唯一或主要移动手段
    // 如果存在，必须也有 move_mouse（move_mouse 是主路径）
    if (warpBlock.includes('cursor.set_position') || warpBlock.includes('self.cursor.set_position')) {
      expect(warpBlock).toMatch(/executor\.move_mouse/);
    }
  });

  it('Mac inject_mouse_move 中必须区分 LeftMouseDragged 和 MouseMoved', () => {
    const injectSrc = read('src-tauri/src/platform/macos/input_inject.rs');
    expect(injectSrc).toMatch(/LeftMouseDragged/);
    expect(injectSrc).toMatch(/MouseMoved/);
    // 必须通过 CGEventSourceButtonState 检测左键状态来选择
    expect(injectSrc).toMatch(/CGEventSourceButtonState/);
  });

  it('Mac inject_mouse_move 必须给事件打 synthetic tag', () => {
    const injectSrc = read('src-tauri/src/platform/macos/input_inject.rs');
    // inject_mouse_move 函数中必须有 tag_synthetic 调用
    const fnBody = injectSrc.match(/fn inject_mouse_move[\s\S]*?\n\}/)?.[0] ?? '';
    expect(fnBody).toMatch(/tag_synthetic/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Fix 3: Win hook 对合成事件放行
// ═══════════════════════════════════════════════════════════════════════════════
describe('BUG-040 Fix3: Win hook — RemoteControlling 合成事件放行', () => {
  const src = () => read('src-tauri/src/platform/windows/input_capture.rs');

  it('鼠标 hook 在 should_swallow 时检查 is_injected + RemoteControlling 放行', () => {
    const content = src();
    // 鼠标 hook 中 should_swallow() 后必须有 is_injected + REMOTE_CONTROLLING 放行分支
    expect(content).toMatch(/should_swallow\(\)[\s\S]*?is_injected[\s\S]*?CONTROL_STATE_REMOTE_CONTROLLING/);
  });

  it('键盘 hook 在 should_swallow 时检查 is_injected + RemoteControlling 放行', () => {
    const content = src();
    // keyboard_hook_proc 中也需要同样的放行逻辑
    // 找到 keyboard_hook_proc 之后的 should_swallow 调用
    const kbSection = content.match(/keyboard_hook_proc[\s\S]*$/)?.[0] ?? '';
    expect(kbSection).toMatch(/should_swallow\(\)[\s\S]*?is_injected[\s\S]*?CONTROL_STATE_REMOTE_CONTROLLING/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 滚轮注入链路完整性
// ═══════════════════════════════════════════════════════════════════════════════
describe('BUG-040 滚轮注入链路完整性', () => {
  it('Win 采集滚轮 → 协议帧 → Mac 注入：完整链路存在', () => {
    const winCapture = read('src-tauri/src/platform/windows/input_capture.rs');
    const sharing = read('src-tauri/src/sharing.rs');
    const macInject = read('src-tauri/src/platform/macos/input_inject.rs');

    // Win 端：WM_MOUSEWHEEL → RawInputEvent::Scroll
    expect(winCapture).toMatch(/WM_MOUSEWHEEL[\s\S]*?Scroll/);

    // sharing.rs：RawInputEvent::Scroll → KvmFrame::MouseScroll
    expect(sharing).toMatch(/Scroll\s*\{.*dx.*dy.*\}\s*=>\s*KvmFrame::MouseScroll/);

    // sharing.rs：inject_frame 中 MouseScroll → executor.scroll
    expect(sharing).toMatch(/MouseScroll\s*\{[\s\S]*?executor\.scroll/);

    // Mac 端：inject_scroll 存在并调用 new_scroll_event
    expect(macInject).toMatch(/fn inject_scroll/);
    expect(macInject).toMatch(/new_scroll_event/);
  });

  it('Mac inject_scroll 必须 tag_synthetic（确保 tap 放行）', () => {
    const macInject = read('src-tauri/src/platform/macos/input_inject.rs');
    const fnBody = macInject.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    expect(fnBody).toMatch(/tag_synthetic/);
  });

  it('Mac inject_scroll 必须 post 到 HID', () => {
    const macInject = read('src-tauri/src/platform/macos/input_inject.rs');
    const fnBody = macInject.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    expect(fnBody).toMatch(/post\(.*HID/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 拖拽中间过程保证
// ═══════════════════════════════════════════════════════════════════════════════
describe('BUG-040 拖拽中间过程 — MouseDown + 连续 move_mouse(LeftMouseDragged)', () => {
  it('MouseButton 帧通过 inject_frame → executor.mouse_down/up 注入', () => {
    const sharing = read('src-tauri/src/sharing.rs');
    expect(sharing).toMatch(/MouseButton\s*\{[\s\S]*?mouse_down|mouse_up/);
  });

  it('MouseMoveDelta 帧走 move_mouse 路径（非 set_position only）', () => {
    const sharing = read('src-tauri/src/sharing.rs');
    // handle_remote_mouse_delta 完整函数体中必须有 move_mouse 调用
    // 函数从 "async fn handle_remote_mouse_delta" 开始，到下一个 "pub async fn" 或 "/// " 结束
    const fnStart = sharing.indexOf('async fn handle_remote_mouse_delta');
    const fnEnd = sharing.indexOf('pub async fn on_remote_switch_control');
    const deltaHandler = sharing.substring(fnStart, fnEnd > fnStart ? fnEnd : fnStart + 2000);
    expect(deltaHandler).toMatch(/move_mouse/);
  });

  it('Mac inject_mouse_move 在左键按下时发 LeftMouseDragged', () => {
    const macInject = read('src-tauri/src/platform/macos/input_inject.rs');
    const fnBody = macInject.match(/fn inject_mouse_move[\s\S]*?\n\}/)?.[0] ?? '';
    // 必须有：left_btn_down → LeftMouseDragged
    expect(fnBody).toMatch(/left_btn_down[\s\S]*?LeftMouseDragged/);
  });

  it('inject_mouse_button 必须给事件打 synthetic tag（确保 tap 放行）', () => {
    const macInject = read('src-tauri/src/platform/macos/input_inject.rs');
    const fnBody = macInject.match(/fn inject_mouse_button[\s\S]*?\n\}/)?.[0] ?? '';
    expect(fnBody).toMatch(/tag_synthetic/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 安全兜底：不误吞正常输入
// ═══════════════════════════════════════════════════════════════════════════════
describe('BUG-040 安全兜底 — Idle/Armed 下不误吞', () => {
  it('Mac tap: Idle/Armed 必须放行所有事件（无条件 Some）', () => {
    const src = read('src-tauri/src/platform/macos/input_capture.rs');
    // 必须有默认分支放行
    expect(src).toMatch(/_\s*=>\s*\{[\s\S]*?Some\(event\.to_owned\(\)\)/);
  });

  it('Win hook: should_swallow 在 Idle/Armed 下返回 false', () => {
    const src = read('src-tauri/src/platform/windows/input_capture.rs');
    // should_swallow 仅在 LOCAL_CONTROLLING | REMOTE_CONTROLLING 返回 true
    expect(src).toMatch(
      /fn should_swallow[\s\S]*?CONTROL_STATE_LOCAL_CONTROLLING\s*\|\s*CONTROL_STATE_REMOTE_CONTROLLING/
    );
  });
});
