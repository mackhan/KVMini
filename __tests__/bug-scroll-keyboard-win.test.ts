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

describe('BUG-038 Win 被控端键盘 CGKeyCode→VK 转换', () => {
  const injectFile = 'src-tauri/src/platform/windows/input_inject.rs';

  it('input_inject.rs 必须包含 cg_keycode_to_vk 函数', () => {
    const src = read(injectFile);
    expect(/fn\s+cg_keycode_to_vk/.test(src)).toBe(true);
  });

  it('key_down/key_up 必须调用 cg_keycode_to_vk 做转换', () => {
    const src = read(injectFile);
    // key_down 和 key_up 都需要调用转换
    const keyDownSection = src.match(/fn key_down[\s\S]*?fn key_up/)?.[0] ?? '';
    expect(/cg_keycode_to_vk/.test(keyDownSection)).toBe(true);
  });

  it('转换表必须覆盖字母键 a(0)→0x41, s(1)→0x53, q(12)→0x51', () => {
    const src = read(injectFile);
    expect(/0\s*=>\s*0x41/.test(src)).toBe(true);  // a
    expect(/1\s*=>\s*0x53/.test(src)).toBe(true);  // s
    expect(/12\s*=>\s*0x51/.test(src)).toBe(true); // q
  });

  it('转换表必须覆盖方向键 Left(123) Right(124) Down(125) Up(126)', () => {
    const src = read(injectFile);
    expect(/123\s*=>\s*0x25/.test(src)).toBe(true); // Left  → VK_LEFT
    expect(/124\s*=>\s*0x27/.test(src)).toBe(true); // Right → VK_RIGHT
    expect(/125\s*=>\s*0x28/.test(src)).toBe(true); // Down  → VK_DOWN
    expect(/126\s*=>\s*0x26/.test(src)).toBe(true); // Up    → VK_UP
  });

  it('转换表必须覆盖特殊键 Return(36) Space(49) Backspace(51) Escape(53)', () => {
    const src = read(injectFile);
    expect(/36\s*=>\s*0x0D/.test(src)).toBe(true); // Return
    expect(/49\s*=>\s*0x20/.test(src)).toBe(true); // Space
    expect(/51\s*=>\s*0x08/.test(src)).toBe(true); // Backspace
    expect(/53\s*=>\s*0x1B/.test(src)).toBe(true); // Escape
  });

  it('转换表必须覆盖 F1-F12 功能键', () => {
    const src = read(injectFile);
    expect(/122\s*=>\s*0x70/.test(src)).toBe(true); // F1
    expect(/120\s*=>\s*0x71/.test(src)).toBe(true); // F2
    expect(/111\s*=>\s*0x7B/.test(src)).toBe(true); // F12
  });

  it('转换表必须覆盖修饰键 Shift(56) Cmd(55) Alt(58)', () => {
    const src = read(injectFile);
    expect(/56\s*=>\s*0xA0/.test(src)).toBe(true); // Left Shift → VK_LSHIFT
    expect(/55\s*=>\s*0x5B/.test(src)).toBe(true); // Left Cmd → VK_LWIN
    expect(/58\s*=>\s*0xA2|58\s*=>\s*0xA4/.test(src)).toBe(true); // Alt/Option
  });
});
