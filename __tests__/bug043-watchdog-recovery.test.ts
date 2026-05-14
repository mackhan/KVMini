/**
 * BUG-043 (v0.1.59): 远端进程被 kill 后本机键鼠锁死（已第 3 次复发）
 *
 * 根因：单一应用层路径不可靠
 *   - read_loop 异常退出后才 emit Disconnected，时序窗口大
 *   - 心跳路径 5s × 3 = 15s 才超时
 *   - 任一环节挂死，本机键鼠永久锁死
 *
 * 修复策略（M4 纵深防御 三层兜底）：
 *   层 1 — TCP Keepalive（OS 层）：idle 5s / interval 3s / retries 2 ≈ 11s 感知
 *   层 2 — Watchdog（应用层）    ：10s 无远端帧强制 force_recovery
 *   层 3 — 心跳超时             ：15s 兜底
 *   force_recovery 必须重置：control_state=Idle、capture.stop、blocker.unblock、
 *                            cursor.recouple、remote_cursor=None、watchdog_stop、
 *                            sharing-state-sync(disconnected) 推送
 */
import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
const read = (rel: string): string => fs.readFileSync(path.join(repoRoot, rel), 'utf8');

// ── L1: TCP Keepalive 已在 kvm-core 注入 ────────────────────────────────────
describe('BUG-043 L1 — TCP Keepalive (OS 层兜底)', () => {
  const network = read('kvm-core/src/network.rs');

  it('apply_tcp_keepalive 函数已声明并以 socket2 实现', () => {
    expect(/pub fn apply_tcp_keepalive\s*\(/.test(network)).toBe(true);
    expect(/socket2::\{?\s*SockRef\s*,\s*TcpKeepalive\s*\}?/.test(network)).toBe(true);
    expect(/set_tcp_keepalive\s*\(/.test(network)).toBe(true);
  });

  it('keepalive 参数：idle=5s / interval=3s / retries=2（与 plan 对齐）', () => {
    expect(/TCP_KEEPALIVE_IDLE_SECS\s*:\s*u64\s*=\s*5\b/.test(network)).toBe(true);
    expect(/TCP_KEEPALIVE_INTERVAL_SECS\s*:\s*u64\s*=\s*3\b/.test(network)).toBe(true);
    expect(/TCP_KEEPALIVE_RETRIES\s*:\s*u32\s*=\s*2\b/.test(network)).toBe(true);
  });

  it('connect_to (主动连接) 在 set_nodelay 之后调用 apply_tcp_keepalive', () => {
    // 抓 connect_to 函数体并验证 keepalive 调用存在
    const m = network.match(/pub async fn connect_to[\s\S]+?\n    \}\n/);
    expect(m).toBeTruthy();
    const body = m![0];
    expect(/set_nodelay\(/.test(body)).toBe(true);
    expect(/apply_tcp_keepalive\s*\(\s*&stream\s*\)/.test(body)).toBe(true);
    // 调用顺序：set_nodelay 在 apply_tcp_keepalive 之前
    expect(body.indexOf('set_nodelay')).toBeLessThan(body.indexOf('apply_tcp_keepalive'));
  });

  it('handle_stream (被动 accept) 也调用 apply_tcp_keepalive', () => {
    const m = network.match(/async fn handle_stream\([\s\S]+?\n    let \(mut reader/);
    expect(m).toBeTruthy();
    expect(/apply_tcp_keepalive\s*\(\s*&stream\s*\)/.test(m![0])).toBe(true);
  });

  it('keepalive 失败必须 warn 但不阻塞连接（降级语义）', () => {
    // 不能用 ? 操作符让失败传播
    const m = network.match(/apply_tcp_keepalive\(&stream\)[\s\S]{0,200}/g);
    expect(m).toBeTruthy();
    m!.forEach((occ) => {
      // 不能直接 ?，必须包在 if let Err(e) = ... 里
      expect(/apply_tcp_keepalive\(&stream\)\?/.test(occ)).toBe(false);
    });
    expect(/log::warn!\([^)]*"\[Network\][^)]*Keepalive[^)]*\)/.test(network)).toBe(true);
  });
});

// ── L2: Watchdog（应用层兜底）─────────────────────────────────────────────
describe('BUG-043 L2 — Watchdog (应用层兜底)', () => {
  const guard = read('src-tauri/src/state_guard.rs');

  it('Watchdog 超时阈值 = 10_000ms（v0.1.59 调整）', () => {
    expect(/const WATCHDOG_TIMEOUT_MS\s*:\s*u64\s*=\s*10_?000\b/.test(guard)).toBe(true);
  });

  it('start_watchdog 周期检查间隔 ≤ 1s（保证 10s 内能多次触发）', () => {
    const m = guard.match(/const WATCHDOG_CHECK_INTERVAL_MS\s*:\s*u64\s*=\s*(\d+)/);
    expect(m).toBeTruthy();
    const interval = parseInt(m![1], 10);
    expect(interval).toBeGreaterThan(0);
    expect(interval).toBeLessThanOrEqual(1000);
  });

  it('超时分支调用 trigger_force_idle，最终走 sm.on_disconnected 完整恢复', () => {
    expect(/elapsed >= WATCHDOG_TIMEOUT_MS[\s\S]{0,300}trigger_force_idle/.test(guard)).toBe(true);
    expect(/sm\.on_disconnected\(app\)/.test(guard)).toBe(true);
  });

  it('touch_remote_frame 喂狗机制存在（避免远端正常收帧时误触发）', () => {
    expect(/pub fn touch_remote_frame\(&self\)/.test(guard)).toBe(true);
  });

  it('Watchdog 仅在 RemoteControlling 进入时启动', () => {
    const sharing = read('src-tauri/src/sharing.rs');
    // 状态机事件分发处必须按 to == RemoteControlling 启动 watchdog
    expect(
      /to\s*==\s*ControlState::RemoteControlling[\s\S]{0,200}state_guard\.start_watchdog/.test(
        sharing
      )
    ).toBe(true);
  });

  it('Watchdog 在 from == RemoteControlling 时停止，避免空转', () => {
    const sharing = read('src-tauri/src/sharing.rs');
    expect(
      /from\s*==\s*ControlState::RemoteControlling[\s\S]{0,200}state_guard\.stop_watchdog/.test(
        sharing
      )
    ).toBe(true);
  });
});

// ── L3: on_disconnected 兜底重置全部状态 ────────────────────────────────────
describe('BUG-043 L3 — on_disconnected force_recovery (BUG-036 兜底强化)', () => {
  const sharing = read('src-tauri/src/sharing.rs');
  const m = sharing.match(/pub async fn on_disconnected[\s\S]+?\n    \}\n/);
  expect(m).toBeTruthy();
  const body = m![0];

  it('停止 Watchdog（防止超时回调与 on_disconnected 竞态）', () => {
    expect(/state_guard\.stop_watchdog\(\)/.test(body)).toBe(true);
  });

  it('Hook 层 control_state 强制 → Idle（停止吞事件）', () => {
    expect(
      /capture\.set_control_state\(\s*control_state_to_u8\(\s*ControlState::Idle\s*\)\s*\)/.test(
        body
      )
    ).toBe(true);
  });

  it('blocker.unblock_input 强制解除键鼠屏蔽', () => {
    expect(/blocker\.unblock_input\(\)/.test(body)).toBe(true);
  });

  it('cursor.recouple_physical_mouse 恢复物理鼠标↔光标绑定', () => {
    expect(/cursor\.recouple_physical_mouse\(\)/.test(body)).toBe(true);
  });

  it('remote_cursor = None 清理虚拟光标', () => {
    expect(/\*self\.remote_cursor\.lock\(\)\.await\s*=\s*None/.test(body)).toBe(true);
  });

  it('capture.stop_capture 释放采集 Hook', () => {
    expect(/capture\.stop_capture\(\)/.test(body)).toBe(true);
  });

  it('emit sharing-state-sync 推送最终 disconnected snapshot 给前端', () => {
    expect(/app\.emit\(\s*"sharing-state-sync"/.test(body)).toBe(true);
  });

  it('整个 on_disconnected 用 log::info!/warn! 标注关键步骤（便于 bug 排查）', () => {
    expect(/log::info!\([^)]*"\[StateGuard\][^)]*on_disconnected/.test(body)).toBe(true);
  });
});

// ── L4: socket2 依赖正确声明 ───────────────────────────────────────────────
describe('BUG-043 L4 — socket2 workspace 依赖配置', () => {
  it('workspace 根 Cargo.toml 注册 socket2 0.5+', () => {
    const root = read('Cargo.toml');
    expect(/socket2\s*=\s*\{[^}]*version\s*=\s*"0\.5/.test(root)).toBe(true);
  });

  it('kvm-core/Cargo.toml 引用 workspace socket2', () => {
    const sub = read('kvm-core/Cargo.toml');
    expect(/socket2\s*=\s*\{\s*workspace\s*=\s*true\s*\}/.test(sub)).toBe(true);
  });
});
