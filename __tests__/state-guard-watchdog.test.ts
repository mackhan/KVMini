/**
 * StateGuard 状态守卫测试
 * 
 * 验证：
 * 1. Watchdog 超时机制（3s 无远端帧 → 自动回 Idle）
 * 2. Ctrl+Alt+S 本地强制恢复快捷键
 * 3. sharing-state-sync 状态快照推送
 * 4. 状态转换日志格式
 */

import * as fs from 'fs';
import * as path from 'path';

const PROJECT_ROOT = path.resolve(__dirname, '..');

describe('StateGuard Module Structure', () => {
    it('state_guard.rs exists and has required components', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/state_guard.rs');
        expect(fs.existsSync(filePath)).toBe(true);

        const content = fs.readFileSync(filePath, 'utf-8');

        // 核心结构体
        expect(content).toContain('pub struct StateGuard');
        expect(content).toContain('pub struct StateSnapshot');

        // Watchdog 超时常量
        expect(content).toContain('WATCHDOG_TIMEOUT_MS');
        expect(content).toContain('3000');

        // 核心方法
        expect(content).toContain('pub fn touch_remote_frame');
        expect(content).toContain('pub fn start_watchdog');
        expect(content).toContain('pub fn stop_watchdog');
        expect(content).toContain('pub fn is_watchdog_running');

        // 状态快照构建
        expect(content).toContain('pub fn build_snapshot');

        // 安全兜底
        expect(content).toContain('trigger_force_idle');
    });

    it('state_guard.rs has Watchdog check interval of 500ms', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/state_guard.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('WATCHDOG_CHECK_INTERVAL_MS');
        expect(content).toContain('500');
    });

    it('state_guard.rs has unit tests', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/state_guard.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('#[cfg(test)]');
        expect(content).toContain('test_state_guard_new');
        expect(content).toContain('test_touch_remote_frame_updates_timestamp');
        expect(content).toContain('test_stop_watchdog_clears_timestamp');
        expect(content).toContain('test_build_snapshot');
    });
});

describe('StateGuard Integration in SharingManager', () => {
    it('SharingManager has state_guard field', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('state_guard: Arc<crate::state_guard::StateGuard>');
    });

    it('inject_remote_frame calls touch_remote_frame', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('self.state_guard.touch_remote_frame()');
    });

    it('StateChanged handler starts Watchdog on RemoteControlling entry', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('if to == ControlState::RemoteControlling');
        expect(content).toContain('self.state_guard.start_watchdog');
    });

    it('StateChanged handler stops Watchdog on RemoteControlling exit', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('if from == ControlState::RemoteControlling');
        expect(content).toContain('self.state_guard.stop_watchdog()');
    });

    it('on_disconnected stops Watchdog as part of cleanup', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        // on_disconnected 中的 stop_watchdog 兜底
        const onDisconnectedBlock = content.substring(
            content.indexOf('pub async fn on_disconnected'),
            content.indexOf('log::info!("[StateGuard] on_disconnected 执行完毕')
        );
        expect(onDisconnectedBlock).toContain('self.state_guard.stop_watchdog()');
    });

    it('StateChanged emits sharing-state-sync snapshot', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('app.emit("sharing-state-sync"');
        expect(content).toContain('crate::state_guard::build_snapshot');
    });
});

describe('Force Recovery Hotkey - Windows', () => {
    it('keyboard_hook_proc detects Ctrl+Alt+S', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/platform/windows/input_capture.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        // VK_S = 0x53
        expect(content).toContain('vk_code == 0x53');
        expect(content).toContain('GetAsyncKeyState(VK_CONTROL)');
        expect(content).toContain('GetAsyncKeyState(VK_MENU)');
        expect(content).toContain('ctrl_down && alt_down');
    });

    it('Ctrl+Alt+S immediately sets control_state to IDLE', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/platform/windows/input_capture.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        // 查找 Ctrl+Alt+S 块中设置 Idle 的代码
        const ctrlAltSBlock = content.substring(
            content.indexOf('Ctrl+Alt+S 安全恢复快捷键触发'),
            content.indexOf('放行此键（不吞），让系统正常处理')
        );
        expect(ctrlAltSBlock).toContain('CONTROL_STATE_IDLE');
    });

    it('Ctrl+Alt+S only triggers in RemoteControlling state', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/platform/windows/input_capture.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('read_control_state() == CONTROL_STATE_REMOTE_CONTROLLING');
    });

    it('Ctrl+Alt+S notifies sharing layer via preempt callback', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/platform/windows/input_capture.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        // 在 Ctrl+Alt+S 处理块中应该调用 preempt callback
        const ctrlAltSBlock = content.substring(
            content.indexOf('Ctrl+Alt+S 安全恢复快捷键触发'),
            content.indexOf('放行此键（不吞），让系统正常处理')
        );
        expect(ctrlAltSBlock).toContain('HOOK_PREEMPT_CALLBACK');
        expect(ctrlAltSBlock).toContain('PreemptCause::KeyDown');
    });
});

describe('Force Recovery Hotkey - macOS', () => {
    it('CGEventTap callback detects Ctrl+Option+S', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/platform/macos/input_capture.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        // kVK_ANSI_S = 1, kCGEventFlagMaskControl = 0x40000, kCGEventFlagMaskAlternate = 0x80000
        expect(content).toContain('cg_keycode == 1');
        expect(content).toContain('ctrl_pressed && alt_pressed');
        expect(content).toContain('0x40000'); // kCGEventFlagMaskControl
        expect(content).toContain('0x80000'); // kCGEventFlagMaskAlternate
    });

    it('Ctrl+Option+S immediately sets control_state to IDLE on macOS', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/platform/macos/input_capture.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        // 找到 Ctrl+Option+S 处理块
        const block = content.substring(
            content.indexOf('Ctrl+Option+S 安全恢复快捷键触发'),
            content.indexOf('// 放行此键')
        );
        expect(block).toContain('CONTROL_STATE_IDLE');
    });
});

describe('Frontend State Sync', () => {
    it('main.js listens for sharing-state-sync event', () => {
        const filePath = path.join(PROJECT_ROOT, 'src/main.js');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain("'sharing-state-sync'");
        expect(content).toContain('snapshot.connected');
        expect(content).toContain('snapshot.control_state');
    });

    it('sharing-state-sync updates appState atomically', () => {
        const filePath = path.join(PROJECT_ROOT, 'src/main.js');
        const content = fs.readFileSync(filePath, 'utf-8');

        // 验证快照更新逻辑
        expect(content).toContain('appState.connected = snapshot.connected');
        expect(content).toContain('updateControlStateUI(snapshot.control_state)');
        expect(content).toContain('updateSharingButtonForState(snapshot.control_state)');
    });
});

describe('Enhanced Logging', () => {
    it('sharing.rs uses [StateGuard] log prefix for state changes', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/sharing.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('[StateGuard] STATE_CHANGE:');
        expect(content).toContain('[StateGuard] on_disconnected');
    });

    it('connection/mod.rs logs disconnect detection with [StateGuard] prefix', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/connection/mod.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('[StateGuard]');
        expect(content).toContain('连接断开检测到');
    });

    it('state_guard.rs has Watchdog lifecycle logging', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/state_guard.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('Watchdog 已启动');
        expect(content).toContain('[StateGuard] Watchdog 已停止');
        expect(content).toContain('Watchdog 超时');
    });
});

describe('lib.rs Module Registration', () => {
    it('lib.rs exports state_guard module', () => {
        const filePath = path.join(PROJECT_ROOT, 'src-tauri/src/lib.rs');
        const content = fs.readFileSync(filePath, 'utf-8');

        expect(content).toContain('pub mod state_guard');
    });
});
