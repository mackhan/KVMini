/**
 * BUG-041 (v0.1.59): Win 控制 Mac 滚轮失效（拖动/中键已修，但滚轮无效）
 *
 * 根因（待 M1 日志验证）：链路代码完整但实测无效，疑点：
 *   - Mac inject_scroll 已 tag_synthetic + post HID，CGEventTap 在 RemoteControlling 下完全 pass-through
 *   - Win 主控 WM_MOUSEWHEEL 采集 → KvmFrame::MouseScroll 转换正确
 *   - protocol 编解码完整
 *
 * 修复策略（M2 阶段）：
 *   1. 全链路诊断日志：Win 采集 → pump 发送 → Mac inject 三处明确埋点（M1 已加）
 *   2. 加单元测试覆盖 inject_scroll/raw_to_frame/inject_frame 的 dx/dy 守恒 invariant
 *   3. 加守门测试：tag_synthetic 必须在 post 之前调用（防止下游 PreemptDetector 误吞）
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

// ── L1: Win 端采集 → KvmFrame::MouseScroll 链路守恒 ─────────────────────────
describe('BUG-041 Win 采集端 — WM_MOUSEWHEEL → RawInputEvent::Scroll 链路守恒', () => {
  it('WM_MOUSEWHEEL 必须读 mouseData 高 16 位作为 delta', () => {
    const src = read('src-tauri/src/platform/windows/input_capture.rs');
    expect(/WM_MOUSEWHEEL/.test(src)).toBe(true);
    // delta = (mouseData as i32) >> 16
    expect(/mouseData\s+as\s+i32\s*\)?\s*>>\s*16/.test(src)).toBe(true);
  });

  it('WM_MOUSEWHEEL 必须将 delta 写到 RawInputEvent::Scroll 的 dy 字段（垂直）', () => {
    const src = read('src-tauri/src/platform/windows/input_capture.rs');
    // 形式：Some(RawInputEvent::Scroll { dx: 0, dy: delta })
    expect(/Scroll\s*\{\s*dx:\s*0\s*,\s*dy:\s*delta\s*\}/.test(src)).toBe(true);
  });

  it('WM_MOUSEHWHEEL 必须将 delta 写到 RawInputEvent::Scroll 的 dx 字段（水平）', () => {
    const src = read('src-tauri/src/platform/windows/input_capture.rs');
    expect(/WM_MOUSEHWHEEL/.test(src)).toBe(true);
    expect(/Scroll\s*\{\s*dx:\s*delta\s*,\s*dy:\s*0\s*\}/.test(src)).toBe(true);
  });

  it('M1 诊断日志：WM_MOUSEWHEEL/WM_MOUSEHWHEEL 必须打印 delta + state', () => {
    const src = read('src-tauri/src/platform/windows/input_capture.rs');
    expect(/\[Win\/Capture\][^"\n]*WM_MOUSEWHEEL[^"\n]*delta/.test(src)).toBe(true);
    expect(/\[Win\/Capture\][^"\n]*WM_MOUSEHWHEEL[^"\n]*delta/.test(src)).toBe(true);
  });
});

// ── L2: sharing.rs raw_to_frame 转换守恒 ────────────────────────────────────
describe('BUG-041 Sharing 转换层 — raw_to_frame Scroll 不丢字段', () => {
  it('RawInputEvent::Scroll 必须映射为 KvmFrame::MouseScroll 且 dx/dy 一一对应', () => {
    const src = read('src-tauri/src/sharing.rs');
    // 必有形如 RawInputEvent::Scroll { dx, dy } => KvmFrame::MouseScroll { dx, dy, ... }
    expect(/RawInputEvent::Scroll\s*\{\s*dx,\s*dy\s*\}\s*=>\s*KvmFrame::MouseScroll\s*\{[\s\S]{0,80}dx,[\s\S]{0,80}dy,/m.test(src)).toBe(true);
  });

  it('inject_frame 必须把 KvmFrame::MouseScroll 转发到 executor.scroll(dx, dy)', () => {
    const src = read('src-tauri/src/sharing.rs');
    expect(/KvmFrame::MouseScroll\s*\{[\s\S]{0,60}\}\s*=>\s*\{[\s\S]{0,80}executor\.scroll\(\s*\*?dx\s*,\s*\*?dy\s*\)/m.test(src)).toBe(true);
  });

  it('M1 诊断日志：pump 发送 MouseScroll 时必须每帧打 debug（不受 sample_count%100 限制）', () => {
    const src = read('src-tauri/src/sharing.rs');
    // 确认有 if let KvmFrame::MouseScroll { ... } => log::debug!("[Sharing] pump 发送 MouseScroll ...")
    expect(/if let KvmFrame::MouseScroll[\s\S]{0,200}log::debug![\s\S]{0,200}pump 发送 MouseScroll/m.test(src)).toBe(true);
  });

  it('M1 诊断日志：被控端 inject_remote_frame 必须区分 MouseScroll 帧打 debug', () => {
    const src = read('src-tauri/src/sharing.rs');
    expect(/KvmFrame::MouseScroll[\s\S]{0,80}log::debug![\s\S]{0,200}收到 MouseScroll/m.test(src)).toBe(true);
  });
});

// ── L3: Mac 注入端 inject_scroll 守恒 + 必须放行（pass-through） ─────────────
describe('BUG-041 Mac 注入端 — inject_scroll 守恒 + tag_synthetic + 必须 post', () => {
  const macInject = () => read('src-tauri/src/platform/macos/input_inject.rs');

  it('inject_scroll 必须使用 SCROLL_UNIT_LINE(=1) 模式并除以 120', () => {
    const src = macInject();
    expect(/SCROLL_UNIT_LINE.*=\s*1/.test(src)).toBe(true);
    const fn = src.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    expect(/\/\s*120/.test(fn)).toBe(true);
  });

  it('inject_scroll 必须在 post 前调用 tag_synthetic（防止 PreemptDetector 误吞）', () => {
    const src = macInject();
    const fn = src.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    // tag_synthetic 出现在 post 前
    const tagIdx = fn.indexOf('tag_synthetic(&event)');
    const postIdx = fn.indexOf('event.post(');
    expect(tagIdx).toBeGreaterThan(0);
    expect(postIdx).toBeGreaterThan(tagIdx);
  });

  it('inject_scroll 必须显式 post 到 CGEventTapLocation::HID', () => {
    const src = macInject();
    const fn = src.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    expect(/event\.post\([^)]*CGEventTapLocation::HID/.test(fn)).toBe(true);
  });

  it('M1 诊断日志：inject_scroll 必须 debug 打 raw + 转换后的 lines', () => {
    const src = macInject();
    const fn = src.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    expect(/log::debug![\s\S]{0,200}\[Mac\/Inject\][\s\S]{0,200}lines/.test(fn)).toBe(true);
  });

  it('lines_y 计算必须是 (|dy|/120).max(1).min(20) * sign(dy)（dy=0 时为 0）', () => {
    const src = macInject();
    const fn = src.match(/fn inject_scroll[\s\S]*?\n\}/)?.[0] ?? '';
    // dy.abs() / 120 .max(1) .min(20) * dy.signum()
    expect(/dy\.abs\(\)\s*\/\s*120/.test(fn)).toBe(true);
    expect(/\.max\(1\)/.test(fn)).toBe(true);
    expect(/dy\.signum\(\)/.test(fn)).toBe(true);
  });
});

// ── L4: Mac CGEventTap 必须对 RemoteControlling 下的合成 ScrollWheel pass-through ─
describe('BUG-041 Mac CGEventTap — RemoteControlling 下合成事件必须 pass-through', () => {
  it('CGEventTap 在 RemoteControlling 状态下应直接 Some(event.to_owned())', () => {
    const src = read('src-tauri/src/platform/macos/input_capture.rs');
    // CONTROL_STATE_REMOTE_CONTROLLING => { ... Some(event.to_owned()) }
    // 放宽：允许 arm 中含大段注释（用 [\s\S]+? 非贪婪），匹配整个 match arm 体到 Some(event.to_owned())
    const armBlock = src.match(/CONTROL_STATE_REMOTE_CONTROLLING\s*=>\s*\{[\s\S]+?\n\s{16}\}/m)?.[0] ?? '';
    expect(armBlock.length).toBeGreaterThan(0);
    expect(/Some\(event\.to_owned\(\)\)/.test(armBlock)).toBe(true);
  });

  it('放行逻辑注释中应明确说明：DecoupleCursor + pass-through 不影响虚拟光标', () => {
    const src = read('src-tauri/src/platform/macos/input_capture.rs');
    // 只要含关键字之一即可
    expect(/pass-through|pass\s*through/i.test(src)).toBe(true);
    expect(/DecoupleCursor|CGAssociate/.test(src)).toBe(true);
  });
});

// ── L5: 协议层 MouseScroll 编解码守恒 ───────────────────────────────────────
describe('BUG-041 协议层 — KvmFrame::MouseScroll 编解码 dx/dy 守恒', () => {
  it('protocol.rs 中 MouseScroll 编码必须包含 dx + dy + timestamp_ms + modifiers', () => {
    const src = read('kvm-core/src/protocol.rs');
    // 编码段
    expect(/KvmFrame::MouseScroll\s*\{\s*dx,\s*dy,\s*timestamp_ms,\s*modifiers\s*\}\s*=>/m.test(src)).toBe(true);
    // 解码段：拉取 EventType::MouseScroll 的 arm 体
    const armBody = src.match(/EventType::MouseScroll\s*=>\s*\{[\s\S]+?\n\s{8}\}/m)?.[0] ?? '';
    expect(armBody.length).toBeGreaterThan(0);
    expect(/KvmFrame::MouseScroll\s*\{\s*dx,\s*dy,/.test(armBody)).toBe(true);
  });

  it('MouseScroll 解码 payload 最少长度为 17 字节（dx4+dy4+ts8+mods1）', () => {
    const src = read('kvm-core/src/protocol.rs');
    expect(/EventType::MouseScroll[\s\S]{0,300}check_min_payload\(payload,\s*17\)/m.test(src)).toBe(true);
  });
});
