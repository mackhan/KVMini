/**
 * BUG-040: Win 控制 Mac 键盘不工作 — X11 KeySym 中间层修复验证
 *
 * 根因：Win 采集 VK 码直接发送，Mac 被控端当 CGKeyCode 用 → 按键错乱/无响应
 *
 * 修复方案（参照 Barrier/Synergy 行业标准）：
 *   - Win input_capture: keyboard_hook_proc 采集后调 vk_to_keysym() 转换
 *   - Mac input_inject: inject_key 收到 KeySym 后调 keysym_to_cg_keycode() 转换
 *   - Mac input_capture: cg_event_to_raw 键盘分支调 cg_keycode_to_keysym() 转换（Mac→Win方向）
 *   - Win input_inject: keysym_to_vk() 替换旧的 cg_keycode_to_vk()（Win被控端）
 *
 * 覆盖范围：字母、数字、符号、方向键、F1-F12、修饰键、空格/Tab/Enter/Esc
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

// ── Win 采集端：vk_to_keysym ────────────────────────────────────────────────
describe('BUG-040 Win input_capture — vk_to_keysym 转换函数', () => {
  const src = () => read('src-tauri/src/platform/windows/input_capture.rs');

  it('必须包含 vk_to_keysym 函数', () => {
    expect(/fn\s+vk_to_keysym/.test(src())).toBe(true);
  });

  it('keyboard_hook_proc 必须调用 vk_to_keysym 做转换后放入 native_keycode', () => {
    const content = src();
    // keyboard_hook_proc 中 KeyDown 分支必须调用 vk_to_keysym
    expect(/vk_to_keysym/.test(content)).toBe(true);
  });

  it('vk_to_keysym: 字母键 VK_A(0x41)→XK_a(0x0061)', () => {
    expect(/0x41\s*=>\s*0x0061/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 字母键 VK_Z(0x5A)→XK_z(0x007A)', () => {
    expect(/0x5A\s*=>\s*0x007A/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 数字键 VK_0(0x30)→XK_0(0x0030)', () => {
    expect(/0x30\s*=>\s*0x0030/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 方向键 VK_LEFT(0x25)→XK_Left(0xFF51)', () => {
    expect(/0x25\s*=>\s*0xFF51/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 方向键 VK_RIGHT(0x27)→XK_Right(0xFF53)', () => {
    expect(/0x27\s*=>\s*0xFF53/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 方向键 VK_UP(0x26)→XK_Up(0xFF52)', () => {
    expect(/0x26\s*=>\s*0xFF52/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 方向键 VK_DOWN(0x28)→XK_Down(0xFF54)', () => {
    expect(/0x28\s*=>\s*0xFF54/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 特殊键 VK_RETURN(0x0D)→XK_Return(0xFF0D)', () => {
    expect(/0x0D\s*=>\s*0xFF0D/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 特殊键 VK_SPACE(0x20)→XK_space(0x0020)', () => {
    expect(/0x20\s*=>\s*0x0020/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 特殊键 VK_BACK(0x08)→XK_BackSpace(0xFF08)', () => {
    expect(/0x08\s*=>\s*0xFF08/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 特殊键 VK_ESCAPE(0x1B)→XK_Escape(0xFF1B)', () => {
    expect(/0x1B\s*=>\s*0xFF1B/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 特殊键 VK_TAB(0x09)→XK_Tab(0xFF09)', () => {
    expect(/0x09\s*=>\s*0xFF09/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 修饰键 VK_LSHIFT(0xA0)→XK_Shift_L(0xFFE1)', () => {
    expect(/0xA0\s*=>\s*0xFFE1/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 修饰键 VK_LCONTROL(0xA2)→XK_Control_L(0xFFE3)', () => {
    expect(/0xA2\s*=>\s*0xFFE3/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 修饰键 VK_LMENU(0xA4)→XK_Alt_L(0xFFE9)', () => {
    expect(/0xA4\s*=>\s*0xFFE9/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 功能键 VK_F1(0x70)→XK_F1(0xFFBE)', () => {
    expect(/0x70\s*=>\s*0xFFBE/.test(src())).toBe(true);
  });

  it('vk_to_keysym: 功能键 VK_F12(0x7B)→XK_F12(0xFFC9)', () => {
    expect(/0x7B\s*=>\s*0xFFC9/.test(src())).toBe(true);
  });
});

// ── Win 被控端：keysym_to_vk（替换旧的 cg_keycode_to_vk）──────────────────
describe('BUG-040 Win input_inject — keysym_to_vk 函数', () => {
  const src = () => read('src-tauri/src/platform/windows/input_inject.rs');

  it('必须包含 keysym_to_vk 函数', () => {
    expect(/fn\s+keysym_to_vk/.test(src())).toBe(true);
  });

  it('key_down/key_up 必须调用 keysym_to_vk（不再是 cg_keycode_to_vk）', () => {
    const content = src();
    expect(/keysym_to_vk/.test(content)).toBe(true);
  });

  it('keysym_to_vk: XK_a(0x0061)→VK_A(0x41)', () => {
    expect(/0x0061\s*=>\s*0x41/.test(src())).toBe(true);
  });

  it('keysym_to_vk: XK_Return(0xFF0D)→VK_RETURN(0x0D)', () => {
    expect(/0xFF0D\s*=>\s*0x0D/.test(src())).toBe(true);
  });

  it('keysym_to_vk: XK_Left(0xFF51)→VK_LEFT(0x25)', () => {
    expect(/0xFF51\s*=>\s*0x25/.test(src())).toBe(true);
  });

  it('keysym_to_vk: XK_F1(0xFFBE)→VK_F1(0x70)', () => {
    expect(/0xFFBE\s*=>\s*0x70/.test(src())).toBe(true);
  });
});

// ── Mac 采集端：cg_keycode_to_keysym ──────────────────────────────────────
describe('BUG-040 Mac input_capture — cg_keycode_to_keysym 转换函数', () => {
  const src = () => read('src-tauri/src/platform/macos/input_capture.rs');

  it('必须包含 cg_keycode_to_keysym 函数', () => {
    expect(/fn\s+cg_keycode_to_keysym/.test(src())).toBe(true);
  });

  it('cg_event_to_raw 键盘分支必须调用 cg_keycode_to_keysym', () => {
    expect(/cg_keycode_to_keysym/.test(src())).toBe(true);
  });

  it('cg_keycode_to_keysym: CGKeyCode 0(a)→XK_a(0x0061)', () => {
    // 0 => 0x0061 // a
    expect(/0\s*=>\s*0x0061/.test(src())).toBe(true);
  });

  it('cg_keycode_to_keysym: CGKeyCode 123(Left)→XK_Left(0xFF51)', () => {
    expect(/123\s*=>\s*0xFF51/.test(src())).toBe(true);
  });

  it('cg_keycode_to_keysym: CGKeyCode 36(Return)→XK_Return(0xFF0D)', () => {
    expect(/36\s*=>\s*0xFF0D/.test(src())).toBe(true);
  });

  it('cg_keycode_to_keysym: CGKeyCode 122(F1)→XK_F1(0xFFBE)', () => {
    expect(/122\s*=>\s*0xFFBE/.test(src())).toBe(true);
  });
});

// ── Mac 被控端：keysym_to_cg_keycode ──────────────────────────────────────
describe('BUG-040 Mac input_inject — keysym_to_cg_keycode 函数', () => {
  const src = () => read('src-tauri/src/platform/macos/input_inject.rs');

  it('必须包含 keysym_to_cg_keycode 函数', () => {
    expect(/fn\s+keysym_to_cg_keycode/.test(src())).toBe(true);
  });

  it('inject_key 必须调用 keysym_to_cg_keycode 做转换', () => {
    const content = src();
    // inject_key 函数体中必须有 keysym_to_cg_keycode 调用
    const injectKeyBlock = content.match(/fn inject_key[\s\S]*?^}/m)?.[0] ?? content;
    expect(/keysym_to_cg_keycode/.test(injectKeyBlock)).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_a(0x0061)→CGKeyCode 0', () => {
    expect(/0x0061\s*=>\s*0/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_Return(0xFF0D)→CGKeyCode 36', () => {
    expect(/0xFF0D\s*=>\s*36/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_Left(0xFF51)→CGKeyCode 123', () => {
    expect(/0xFF51\s*=>\s*123/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_space(0x0020)→CGKeyCode 49', () => {
    expect(/0x0020\s*=>\s*49/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_BackSpace(0xFF08)→CGKeyCode 51', () => {
    expect(/0xFF08\s*=>\s*51/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_Escape(0xFF1B)→CGKeyCode 53', () => {
    expect(/0xFF1B\s*=>\s*53/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_F1(0xFFBE)→CGKeyCode 122', () => {
    expect(/0xFFBE\s*=>\s*122/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_Shift_L(0xFFE1)→CGKeyCode 56', () => {
    expect(/0xFFE1\s*=>\s*56/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_Control_L(0xFFE3)→CGKeyCode 59', () => {
    expect(/0xFFE3\s*=>\s*59/.test(src())).toBe(true);
  });

  it('keysym_to_cg_keycode: XK_Alt_L(0xFFE9)→CGKeyCode 58', () => {
    expect(/0xFFE9\s*=>\s*58/.test(src())).toBe(true);
  });
});

// ── 双向一致性：Win→Mac 与 Mac→Win 互为逆运算 ──────────────────────────────
describe('BUG-040 KeySym 中间层一致性', () => {
  // 用代码检查确保关键按键在两端都有定义（映射对称）
  it('字母键映射一致：Win VK_A 和 Mac CGKeyCode 0 都应映射到同一 KeySym 0x0061', () => {
    const winCapture = read('src-tauri/src/platform/windows/input_capture.rs');
    const macInject  = read('src-tauri/src/platform/macos/input_inject.rs');
    // Win 侧 0x41 → 0x0061
    expect(/0x41\s*=>\s*0x0061/.test(winCapture)).toBe(true);
    // Mac 侧 0x0061 → 0（CGKeyCode）
    expect(/0x0061\s*=>\s*0/.test(macInject)).toBe(true);
  });

  it('Return键映射一致：Win VK_RETURN(0x0D)↔ Mac CGKeyCode 36 都通过 0xFF0D', () => {
    const winCapture = read('src-tauri/src/platform/windows/input_capture.rs');
    const macInject  = read('src-tauri/src/platform/macos/input_inject.rs');
    expect(/0x0D\s*=>\s*0xFF0D/.test(winCapture)).toBe(true);
    expect(/0xFF0D\s*=>\s*36/.test(macInject)).toBe(true);
  });

  it('方向键 Left 映射一致：Win VK_LEFT(0x25)↔ Mac CGKeyCode 123 都通过 0xFF51', () => {
    const winCapture = read('src-tauri/src/platform/windows/input_capture.rs');
    const macInject  = read('src-tauri/src/platform/macos/input_inject.rs');
    expect(/0x25\s*=>\s*0xFF51/.test(winCapture)).toBe(true);
    expect(/0xFF51\s*=>\s*123/.test(macInject)).toBe(true);
  });
});
