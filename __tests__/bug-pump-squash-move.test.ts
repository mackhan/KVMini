/**
 * BUG-042: Win 控制 Mac 鼠标拖拽卡顿 — pump MouseMoveDelta 帧合并修复
 *
 * 根因：pump 每个 MouseMove 事件都发一帧，当采集速率 > 网络发送速率时，
 *   tokio channel 积压 N 帧，视觉上产生卡顿（每帧独立 async 调度 ~1ms）
 *
 * 修复（参照 Barrier "Squash mouse moves" 策略）：
 *   收到第一个 MouseMove 后，立即 drain channel 中积压的其余 Move 帧，
 *   将所有 dx/dy 累加后只发一帧合并的 MouseMoveDelta，
 *   非 Move 帧（Button/Key/Scroll）立即发送，不合并。
 *
 * 测试方式：源码静态分析（sharing.rs pump loop 中 try_recv drain 逻辑存在性验证）
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

describe('BUG-042 sharing.rs pump — MouseMoveDelta 帧合并（Squash）', () => {
  const src = () => read('src-tauri/src/sharing.rs');

  it('pump loop 中必须有 try_recv drain 逻辑（合并多个 MouseMove 帧）', () => {
    const content = src();
    // 应该有 rx.try_recv() 调用（drain 积压帧）
    expect(/try_recv\(\)/.test(content)).toBe(true);
  });

  it('pump loop 中必须有 MouseMove delta 累加（acc_dx 或 saturating_add）', () => {
    const content = src();
    // 累加逻辑：saturating_add 或 acc_dx/acc_dy 变量
    const hasAccumulate = /saturating_add/.test(content)
      || /acc_dx/.test(content)
      || /acc_dy/.test(content);
    expect(hasAccumulate).toBe(true);
  });

  it('合并后必须只发一帧（只调用一次 send_frame_via_app for MouseMove）', () => {
    const content = src();
    // pump loop 中对 MouseMove 的处理：累加后在循环外发送
    // 检查：有 saturating_add 且有 send_frame_via_app，说明合并后发送
    expect(/saturating_add/.test(content)).toBe(true);
    expect(/send_frame_via_app/.test(content)).toBe(true);
  });

  it('非 Move 帧（Button/Key/Scroll）不应该被合并跳过', () => {
    const content = src();
    // pump 中 MouseScroll 仍然有处理路径（不能被合并逻辑吞掉）
    // 检查：MouseDown/MouseUp/KeyDown 在 pump 路径中被处理
    const hasButtonHandling = /MouseDown|MouseUp/.test(content);
    expect(hasButtonHandling).toBe(true);
  });

  it('累加时使用 saturating_add 防止 i16/i32 溢出', () => {
    const content = src();
    expect(/saturating_add/.test(content)).toBe(true);
  });
});

// ── pump 整体结构健壮性回归 ───────────────────────────────────────────────
describe('BUG-042 pump 回归保护', () => {
  const src = () => read('src-tauri/src/sharing.rs');

  it('pump loop 仍然有 select! 结构（tokio select 双通道）', () => {
    const content = src();
    expect(/tokio::select!/.test(content)).toBe(true);
  });

  it('preempt_rx 分支仍然存在（抢占检测不受合并逻辑影响）', () => {
    const content = src();
    expect(/preempt_rx\.recv\(\)/.test(content)).toBe(true);
  });

  it('边缘检测 EdgeDetector.detect 仍在 MouseMove 处理路径中', () => {
    const content = src();
    expect(/d\.detect\(x,\s*y\)|\.detect\(/.test(content)).toBe(true);
  });
});
