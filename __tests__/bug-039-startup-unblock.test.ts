/**
 * BUG-039: Win 被控端进程被强杀后重启，鼠标不能控制
 *
 * 根因:
 *   Win32 BlockInput(1) 是系统级 API，不随进程退出自动还原。
 *   WinInputBlocker::Drop 在 TerminateProcess 时不执行。
 *   下次启动 kvmini.exe 时 BlockInput 残留，导致系统级鼠标被锁。
 *
 * 修复:
 *   1. input_block.rs 添加 startup_cleanup()：启动时无条件调用 BlockInput(0)
 *   2. main.rs setup 最开始调用 startup_cleanup()
 *   3. on_window_event(Destroyed) 也调用 startup_cleanup()，正常退出也清理
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

describe('BUG-039 Win 被控端强杀后重启键鼠恢复', () => {
  it('input_block.rs 必须包含 startup_cleanup 函数', () => {
    const src = read('src-tauri/src/platform/windows/input_block.rs');
    expect(/fn\s+startup_cleanup/.test(src)).toBe(true);
  });

  it('startup_cleanup 必须调用 BlockInput(0)', () => {
    const src = read('src-tauri/src/platform/windows/input_block.rs');
    expect(/BlockInput\s*\(\s*0\s*\)/.test(src)).toBe(true);
  });

  it('main.rs setup 必须调用 startup_cleanup', () => {
    const src = read('src-tauri/src/main.rs');
    expect(/startup_cleanup\s*\(\s*\)/.test(src)).toBe(true);
  });

  it('main.rs 必须在 setup 中（BUG-039 注释附近）调用 startup_cleanup', () => {
    const src = read('src-tauri/src/main.rs');
    // 确保是在 setup 区域调用，不是随意位置
    expect(/BUG-039[\s\S]{0,200}startup_cleanup/.test(src)).toBe(true);
  });

  it('main.rs 必须注册 on_window_event 退出 cleanup', () => {
    const src = read('src-tauri/src/main.rs');
    expect(/on_window_event/.test(src)).toBe(true);
    // Destroyed 事件处理（中间可能有 #[cfg(...)] 等行，扩大范围）
    expect(/Destroyed[\s\S]{0,300}startup_cleanup/.test(src)).toBe(true);
  });
});
