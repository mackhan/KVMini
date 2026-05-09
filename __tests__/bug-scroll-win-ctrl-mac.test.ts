/**
 * BUG-041: Win 控制 Mac 滚轮不工作 + 鼠标左键首次不响应
 *
 * 根因1（滚轮）：
 *   Win WM_MOUSEWHEEL 采集 ±120（WHEEL_DELTA），通过协议发到 Mac，
 *   Mac inject_scroll 用 SCROLL_UNIT_LINE 模式把 ±120 当行数注入 → 一次滚 120 行
 *
 * 修复：Mac inject_scroll 改用 SCROLL_UNIT_PIXEL(=2) 模式，
 *   ±120px 在 Mac 上滚动幅度合理（约 3-4 行等效，与 Barrier 协议 ±120 单位一致）
 *
 * 根因2（鼠标点击）：
 *   DecoupleCursor 状态下（CGAssociate 断开物理鼠标绑定），
 *   inject_mouse_button 用 HIDSystemState 读光标位置 → 位置是物理鼠标而非屏幕光标
 *   导致点击坐标偏移，第一次点击命中空白处无响应
 *
 * 修复：改用 CombinedSessionState 读取合成/warp 后的准确光标位置
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

// ── Bug 1：滚轮 PIXEL 模式 ─────────────────────────────────────────────────
describe('BUG-041 Mac inject_scroll — PIXEL 模式修复', () => {
  const src = () => read('src-tauri/src/platform/macos/input_inject.rs');

  it('inject_scroll 必须使用 SCROLL_UNIT_PIXEL(=2)，不能使用 SCROLL_UNIT_LINE(=1)', () => {
    const content = src();
    // 必须有 SCROLL_UNIT_PIXEL 常量定义
    expect(/SCROLL_UNIT_PIXEL.*=\s*2/.test(content)).toBe(true);
  });

  it('new_scroll_event 调用必须传入 SCROLL_UNIT_PIXEL 而非 SCROLL_UNIT_LINE', () => {
    const content = src();
    // new_scroll_event 的第二个参数必须是 SCROLL_UNIT_PIXEL
    // 匹配: new_scroll_event(source, SCROLL_UNIT_PIXEL, ...)
    expect(/new_scroll_event\s*\([^)]*SCROLL_UNIT_PIXEL/.test(content)).toBe(true);
  });

  it('不能再使用旧的 SCROLL_UNIT_LINE = 1 作为 new_scroll_event 参数', () => {
    const content = src();
    // 旧代码: new_scroll_event(source, SCROLL_UNIT_LINE, 2, dy, dx, 0)
    // SCROLL_UNIT_LINE 常量本身可以存在，但不能作为 new_scroll_event 的第二参数
    const callMatch = content.match(/new_scroll_event\s*\([^)]+\)/);
    if (callMatch) {
      // 调用中不应该出现值为 1 的字面量作为第二参数（SCROLL_UNIT_LINE）
      expect(/new_scroll_event\s*\(\s*\w+\s*,\s*1\s*,/.test(content)).toBe(false);
    }
  });

  it('dy 值应直接传入（协议 ±120 在 PIXEL 模式下是合理像素量）', () => {
    const content = src();
    // PIXEL 模式下不需要做 /120 转换，dy 直接传
    // 确保 inject_scroll 函数中没有 "/ 120" 或 "/120"
    const injectScrollFn = content.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    expect(/\/\s*120/.test(injectScrollFn)).toBe(false);
  });
});

// ── Bug 2：鼠标点击 CombinedSessionState ───────────────────────────────────
describe('BUG-041 Mac inject_mouse_button — CombinedSessionState 修复', () => {
  const src = () => read('src-tauri/src/platform/macos/input_inject.rs');

  it('inject_mouse_button 中获取 current_pos 必须使用 CombinedSessionState', () => {
    const content = src();
    expect(/CombinedSessionState/.test(content)).toBe(true);
  });

  it('不应该使用 HIDSystemState 来获取光标位置（用于 current_pos）', () => {
    const content = src();
    // inject_mouse_button 函数内不能用 HIDSystemState 读 location()
    // 提取 inject_mouse_button 函数体
    const fnBody = content.match(/fn inject_mouse_button[\s\S]*?(?=\n#\[cfg|\nfn |\z)/)?.[0] ?? content;
    // 如果函数里有 HIDSystemState 同时有 location()，说明还在用旧方式读坐标
    // 正确做法：HIDSystemState 用于创建事件 source，CombinedSessionState 用于读坐标
    // 检查：不能同时在同一行出现 HIDSystemState 和 location()
    const lines = fnBody.split('\n');
    const badLine = lines.find(l => /HIDSystemState/.test(l) && /location\(\)/.test(l));
    expect(badLine).toBeUndefined();
  });

  it('inject_mouse_button 中必须通过 CombinedSessionState 读取 current_pos', () => {
    const content = src();
    // 必须有一行同时含 CombinedSessionState 和 location（或 cursor_position）
    const hasCombinedPos = /CombinedSessionState[\s\S]{0,200}location\(\)/.test(content)
      || /CombinedSessionState[\s\S]{0,200}current_pos/.test(content)
      || /current_pos[\s\S]{0,200}CombinedSessionState/.test(content);
    expect(hasCombinedPos).toBe(true);
  });
});

// ── 回归保护：Mac→Win 方向（Mac 控制 Win）的滚轮路径不受影响 ────────────────
describe('BUG-041 回归保护 — Mac 采集外接鼠标滚轮 ×120 逻辑不变', () => {
  const src = () => read('src-tauri/src/platform/macos/input_capture.rs');

  it('cg_event_to_raw ScrollWheel 分支中仍应有 ×120 逻辑（外接鼠标格滚动放大）', () => {
    const content = src();
    // 外接鼠标（非连续）分支：dy_fixed >> 16 后 * 120
    expect(/\*\s*120/.test(content)).toBe(true);
  });

  it('is_continuous 区分逻辑仍存在（触摸板 vs 外接鼠标）', () => {
    const content = src();
    expect(/is_continuous/.test(content)).toBe(true);
  });
});

// ── Win 采集端滚轮回归：WHEEL_DELTA ±120 不变 ─────────────────────────────
describe('BUG-041 回归保护 — Win WM_MOUSEWHEEL 采集不变', () => {
  const src = () => read('src-tauri/src/platform/windows/input_capture.rs');

  it('Win WM_MOUSEWHEEL 仍然采集原始 WHEEL_DELTA（不做额外处理）', () => {
    const content = src();
    // 仍然有 (mouseData as i32) >> 16 的读取
    expect(/mouseData\s+as\s+i32.*>>\s*16|mouseData.*>>\s*16.*i32/.test(content)).toBe(true);
  });
});
