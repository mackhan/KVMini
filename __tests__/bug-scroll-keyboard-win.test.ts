/**
 * BUG-037/038: Win 被控端滚轮/键盘不工作修复验证
 *
 * 根因:
 *   1. sharing.rs inject_frame 的 MouseScroll 分支被硬编码跳过（"MVP 未实现"注释）
 *   2. Mac CGKeyCode 与 Windows VK 两套体系不同，需要在 Win 注入层转换
 *
 * 本测试从源码层面验证修复（运行时行为由 Rust 单元测试覆盖）:
 *   1. sharing.rs 的 MouseScroll 分支必须调用 executor.scroll()，不能 skip
 *   2. input_inject.rs 必须包含 cg_keycode_to_vk 转换函数
 *   3. 转换表必须覆盖字母/数字/方向键/功能键/特殊键
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

describe('BUG-037 Win 被控端滚轮修复', () => {
  const sharingFile = 'src-tauri/src/sharing.rs';

  it('inject_frame 的 MouseScroll 分支不能出现 "跳过" 或 "未实现" 注释', () => {
    const src = read(sharingFile);
    // 确保旧的跳过代码不存在
    expect(src).not.toMatch(/MouseScroll 注入未实现，跳过/);
    expect(src).not.toMatch(/log::debug!.*MouseScroll.*跳过/);
  });

  it('inject_frame 的 MouseScroll 分支必须调用 executor.scroll()', () => {
    const src = read(sharingFile);
    // 匹配 MouseScroll 分支中调用 scroll
    const hasScrollCall = /MouseScroll\s*\{[^}]*\}\s*=>\s*\{?\s*executor\.scroll/.test(src)
      || /MouseScroll.*executor\.scroll/.test(src);
    expect(hasScrollCall).toBe(true);
  });
});

describe('BUG-038 Win 被控端键盘转换（v0.1.50: KeySym→VK）', () => {
  const injectFile = 'src-tauri/src/platform/windows/input_inject.rs';

  it('input_inject.rs 必须包含 keysym_to_vk 函数（v0.1.50 KeySym 架构）', () => {
    const src = read(injectFile);
    expect(/fn\s+keysym_to_vk/.test(src)).toBe(true);
  });

  it('key_down/key_up 必须调用 keysym_to_vk 做转换', () => {
    const src = read(injectFile);
    const keyDownSection = src.match(/fn key_down[\s\S]*?fn key_up/)?.[0] ?? '';
    expect(/keysym_to_vk/.test(keyDownSection)).toBe(true);
  });

  it('转换表必须覆盖字母键 XK_a(0x0061)→VK_A(0x41)', () => {
    const src = read(injectFile);
    expect(/0x0061\s*=>\s*0x41/.test(src)).toBe(true); // XK_a → VK_A
    expect(/0x0073\s*=>\s*0x53/.test(src)).toBe(true); // XK_s → VK_S
    expect(/0x0071\s*=>\s*0x51/.test(src)).toBe(true); // XK_q → VK_Q
  });

  it('转换表必须覆盖方向键 XK_Left(0xFF51)→VK_LEFT(0x25) 等', () => {
    const src = read(injectFile);
    expect(/0xFF51\s*=>\s*0x25/.test(src)).toBe(true); // XK_Left  → VK_LEFT
    expect(/0xFF53\s*=>\s*0x27/.test(src)).toBe(true); // XK_Right → VK_RIGHT
    expect(/0xFF54\s*=>\s*0x28/.test(src)).toBe(true); // XK_Down  → VK_DOWN
    expect(/0xFF52\s*=>\s*0x26/.test(src)).toBe(true); // XK_Up    → VK_UP
  });

  it('转换表必须覆盖特殊键 XK_Return(0xFF0D) XK_space(0x0020) 等', () => {
    const src = read(injectFile);
    expect(/0xFF0D\s*=>\s*0x0D/.test(src)).toBe(true); // XK_Return    → VK_RETURN
    expect(/0x0020\s*=>\s*0x20/.test(src)).toBe(true); // XK_space     → VK_SPACE
    expect(/0xFF08\s*=>\s*0x08/.test(src)).toBe(true); // XK_BackSpace → VK_BACK
    expect(/0xFF1B\s*=>\s*0x1B/.test(src)).toBe(true); // XK_Escape    → VK_ESCAPE
  });

  it('转换表必须覆盖 F1-F12 功能键（KeySym 编码）', () => {
    const src = read(injectFile);
    expect(/0xFFBE\s*=>\s*0x70/.test(src)).toBe(true); // XK_F1  → VK_F1
    expect(/0xFFBF\s*=>\s*0x71/.test(src)).toBe(true); // XK_F2  → VK_F2
    expect(/0xFFC9\s*=>\s*0x7B/.test(src)).toBe(true); // XK_F12 → VK_F12
  });

  it('转换表必须覆盖修饰键 XK_Shift_L(0xFFE1) XK_Super_L(0xFFEB) XK_Alt_L(0xFFE9)', () => {
    const src = read(injectFile);
    expect(/0xFFE1\s*=>\s*0xA0/.test(src)).toBe(true); // XK_Shift_L → VK_LSHIFT
    expect(/0xFFEB\s*=>\s*0x5B/.test(src)).toBe(true); // XK_Super_L → VK_LWIN
    expect(/0xFFE9\s*=>\s*0xA4/.test(src)).toBe(true); // XK_Alt_L   → VK_LMENU
  });
});
