/**
 * BUG-044 (v0.1.59) F4 跨端鼠标切换链路修复回归测试
 *
 * 现象（v0.1.58）：
 *   - Win 主控 → Mac 被控：鼠标移到 Win 屏幕边缘后无法切换控制 Mac（或切过去后回不来）
 *   - Mac 被控 → Win 主控：Mac 端虚拟光标撞反向边缘 / 物理输入抢占都无法切回 Win
 *
 * 根因（subagent code-explorer 诊断）：
 *   RC#1: pump 内的边缘/抢占路径走 dispatch_state_event 的 StateChanged 分支，
 *         缺失 EdgeDetector::set_active 切换 / Watchdog 生命周期 / sharing-state-sync 推送，
 *         与 handle_one::StateChanged 不对称，导致状态切换"半成品"
 *   RC#2: 进入 LocalControlling 时 EdgeDetector 没 reset_overshoot；
 *         LocalControlling 期间 squash 路径仍喂 detect，回切到 Armed 立即触发乒乓
 *   RC#3: on_remote_switch_back EdgeReturn 路径无静默期；
 *         主控端鼠标可能仍贴着边缘，回 Armed 瞬间立即重新触发 LocalControlling
 *
 * 修复策略（src-tauri/src/sharing.rs）：
 *   1. dispatch_state_event 增加 edge_detector + state_guard 参数，StateChanged 分支补齐：
 *      - Watchdog start/stop（被控端 RemoteControlling 入态/离态）
 *      - EdgeDetector::set_active(to == Armed) + reset_overshoot
 *      - 回到 Armed 时进入 800ms 静默期
 *      - sharing-state-sync 完整 snapshot 推送
 *   2. handle_events_on_edge_trigger 透传 edge_detector / state_guard
 *   3. on_remote_switch_back EdgeReturn 路径也调 reset_overshoot + enter_silence(800)
 *
 * 测试策略：纯静态断言，确保关键修复点在源码中存在且不被回归
 */

import * as fs from 'fs';
import * as path from 'path';

const SHARING_RS = path.join(__dirname, '..', 'src-tauri', 'src', 'sharing.rs');
const STATE_GUARD_RS = path.join(__dirname, '..', 'src-tauri', 'src', 'state_guard.rs');
const EDGE_LOGIC_RS = path.join(__dirname, '..', 'kvm-core', 'src', 'edge_logic.rs');

let sharingSrc = '';
let stateGuardSrc = '';
let edgeLogicSrc = '';

beforeAll(() => {
  sharingSrc = fs.readFileSync(SHARING_RS, 'utf-8');
  stateGuardSrc = fs.readFileSync(STATE_GUARD_RS, 'utf-8');
  edgeLogicSrc = fs.readFileSync(EDGE_LOGIC_RS, 'utf-8');
});

describe('BUG-044 v0.1.59 跨端切换链路 - RC#1: dispatch_state_event 与 handle_one 对称', () => {
  test('dispatch_state_event 函数签名包含 edge_detector 参数', () => {
    expect(sharingSrc).toMatch(
      /async fn dispatch_state_event\([\s\S]+?edge_detector:\s*&Arc<Mutex<Option<EdgeDetector>>>/
    );
  });

  test('dispatch_state_event 函数签名包含 state_guard 参数', () => {
    expect(sharingSrc).toMatch(
      /async fn dispatch_state_event\([\s\S]+?state_guard:\s*&Arc<crate::state_guard::StateGuard>/
    );
  });

  test('dispatch_state_event::StateChanged 分支启动 watchdog（RemoteControlling 入态）', () => {
    // 抓 dispatch_state_event 函数体
    const fnMatch = sharingSrc.match(
      /async fn dispatch_state_event\([\s\S]+?\n\}\n/
    );
    expect(fnMatch).not.toBeNull();
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(
      /to\s*==\s*ControlState::RemoteControlling[\s\S]{0,200}state_guard\.start_watchdog/
    );
  });

  test('dispatch_state_event::StateChanged 分支停止 watchdog（RemoteControlling 离态）', () => {
    const fnMatch = sharingSrc.match(
      /async fn dispatch_state_event\([\s\S]+?\n\}\n/
    );
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(
      /from\s*==\s*ControlState::RemoteControlling[\s\S]{0,200}state_guard\.stop_watchdog/
    );
  });

  test('dispatch_state_event::StateChanged 分支推送 sharing-state-sync 完整快照', () => {
    const fnMatch = sharingSrc.match(
      /async fn dispatch_state_event\([\s\S]+?\n\}\n/
    );
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(/sharing-state-sync/);
    expect(fnBody).toMatch(/build_snapshot/);
  });
});

describe('BUG-044 v0.1.59 跨端切换链路 - RC#2: EdgeDetector active/overshoot 防乒乓', () => {
  test('dispatch_state_event::StateChanged 分支按 to==Armed 切换 EdgeDetector active', () => {
    const fnMatch = sharingSrc.match(
      /async fn dispatch_state_event\([\s\S]+?\n\}\n/
    );
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(/d\.set_active\(to\s*==\s*ControlState::Armed\)/);
  });

  test('dispatch_state_event::StateChanged 入非 Armed 态时 reset_overshoot', () => {
    const fnMatch = sharingSrc.match(
      /async fn dispatch_state_event\([\s\S]+?\n\}\n/
    );
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(/to\s*!=\s*ControlState::Armed[\s\S]{0,200}reset_overshoot/);
  });

  test('dispatch_state_event::StateChanged 入 Armed 态时也要 reset_overshoot（清零防乒乓）', () => {
    const fnMatch = sharingSrc.match(
      /async fn dispatch_state_event\([\s\S]+?\n\}\n/
    );
    const fnBody = fnMatch![0];
    // else 分支 (to == Armed) 必有 reset_overshoot
    expect(fnBody).toMatch(/}\s*else\s*\{[\s\S]{0,300}reset_overshoot/);
  });

  test('handle_one::StateChanged 分支也调用 set_active(to == Armed)（保持原有对称）', () => {
    expect(sharingSrc).toMatch(/d\.set_active\(to\s*==\s*ControlState::Armed\)/);
  });
});

describe('BUG-044 v0.1.59 跨端切换链路 - RC#3: on_remote_switch_back 静默期', () => {
  test('on_remote_switch_back 在 EdgeReturn / Preempt 任意场景都进入 800ms 静默期', () => {
    const fnMatch = sharingSrc.match(
      /pub async fn on_remote_switch_back\([\s\S]+?\n    \}\n/
    );
    expect(fnMatch).not.toBeNull();
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(/enter_silence\(800\)/);
    expect(fnBody).toMatch(/reset_overshoot/);
  });

  test('on_remote_switch_back 静默期不再用 if-let Preempt 限制（v0.1.59 改为通用）', () => {
    // 旧实现：只有 Preempt 才进静默期，会导致 EdgeReturn 路径乒乓
    // 新实现：先无条件进入静默期，再判断是否需要 emit kvm://preempted
    const fnMatch = sharingSrc.match(
      /pub async fn on_remote_switch_back\([\s\S]+?\n    \}\n/
    );
    const fnBody = fnMatch![0];
    // 静默期代码块出现在 emit kvm://preempted 的分支之外
    const silenceIdx = fnBody.indexOf('enter_silence(800)');
    const preemptEmitIdx = fnBody.indexOf('kvm://preempted');
    expect(silenceIdx).toBeGreaterThan(0);
    expect(preemptEmitIdx).toBeGreaterThan(silenceIdx);
  });
});

describe('BUG-044 v0.1.59 跨端切换链路 - 调用点透传完整性', () => {
  test('handle_events_on_edge_trigger 函数签名包含 edge_detector 参数', () => {
    expect(sharingSrc).toMatch(
      /async fn handle_events_on_edge_trigger\([\s\S]+?edge_detector:\s*&Arc<Mutex<Option<EdgeDetector>>>/
    );
  });

  test('handle_events_on_edge_trigger 函数签名包含 state_guard 参数', () => {
    expect(sharingSrc).toMatch(
      /async fn handle_events_on_edge_trigger\([\s\S]+?state_guard:\s*&Arc<crate::state_guard::StateGuard>/
    );
  });

  test('所有 handle_events_on_edge_trigger 调用点都传入 edge_detector_for_dispatch + state_guard_arc', () => {
    // 调用点：第一个参数是 events 或 evs（不是函数定义的 events: Vec<...>）
    const callMatches = sharingSrc.match(
      /handle_events_on_edge_trigger\(\s*\n\s+(?:events|evs),[\s\S]+?\.await/g
    );
    expect(callMatches).not.toBeNull();
    // 至少 3 处调用点（边缘触发主路径 L811 + drain 重检 L888 + Preempt L996）
    expect(callMatches!.length).toBeGreaterThanOrEqual(3);
    // 每个调用都必须传 edge_detector_for_dispatch 和 state_guard_arc
    for (const call of callMatches!) {
      expect(call).toMatch(/edge_detector_for_dispatch/);
      expect(call).toMatch(/state_guard_arc/);
    }
  });

  test('pump 入口 clone 了 state_guard_arc 和 edge_detector_for_dispatch', () => {
    expect(sharingSrc).toMatch(
      /let state_guard_arc\s*=\s*Arc::clone\(&self\.state_guard\)/
    );
    expect(sharingSrc).toMatch(
      /let edge_detector_for_dispatch\s*=\s*Arc::clone\(&self\.edge_detector\)/
    );
  });
});

describe('BUG-044 v0.1.59 跨端切换链路 - 依赖完整性', () => {
  test('EdgeDetector::set_active 公开方法存在', () => {
    expect(edgeLogicSrc).toMatch(/pub fn set_active\(/);
  });

  test('EdgeDetector::reset_overshoot 公开方法存在', () => {
    expect(edgeLogicSrc).toMatch(/pub fn reset_overshoot\(/);
  });

  test('EdgeDetector::enter_silence(duration_ms) 公开方法存在且接收 u64 ms 参数', () => {
    expect(edgeLogicSrc).toMatch(/pub fn enter_silence\(\s*&mut self\s*,\s*\w+:\s*u64/);
  });

  test('StateGuard::start_watchdog 接收 AppHandle', () => {
    expect(stateGuardSrc).toMatch(/pub fn start_watchdog\([\s\S]{0,200}AppHandle/);
  });

  test('StateGuard::stop_watchdog 公开方法存在', () => {
    expect(stateGuardSrc).toMatch(/pub fn stop_watchdog\(/);
  });
});
