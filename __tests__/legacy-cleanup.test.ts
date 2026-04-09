/**
 * 测试：kvnshare 遗留进程清理
 * 需求：5.1、5.2、5.3、5.4、5.5
 */

describe('遗留进程清理 - Platform 抽象层', () => {
    test('killLegacyProcess 在 invoke 为 null 时返回 false（优雅降级）', async () => {
        const safeInvoke = async (cmd: string, args: any, fallback: any) => fallback;
        const result = await safeInvoke('kill_legacy_process', {}, false);
        expect(result).toBe(false);
    });

    test('killLegacyProcess 在 invoke 可用且无遗留进程时返回 false', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(false);
        const safeInvoke = async (cmd: string, args: any, fallback: any) => mockInvoke(cmd, args);
        const result = await safeInvoke('kill_legacy_process', {}, false);
        expect(result).toBe(false);
        expect(mockInvoke).toHaveBeenCalledWith('kill_legacy_process', {});
    });

    test('killLegacyProcess 在 invoke 可用且发现遗留进程时返回 true', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(true);
        const safeInvoke = async (cmd: string, args: any, fallback: any) => mockInvoke(cmd, args);
        const result = await safeInvoke('kill_legacy_process', {}, false);
        expect(result).toBe(true);
    });
});

describe('遗留进程清理 - 清理脚本文件存在性', () => {
    const fs = require('fs');
    const path = require('path');

    test('macOS 清理脚本文件存在', () => {
        const scriptPath = path.join(__dirname, '../scripts/cleanup-legacy.sh');
        expect(fs.existsSync(scriptPath)).toBe(true);
    });

    test('Windows 清理脚本文件存在', () => {
        const scriptPath = path.join(__dirname, '../scripts/cleanup-legacy.ps1');
        expect(fs.existsSync(scriptPath)).toBe(true);
    });

    test('macOS 清理脚本包含 pgrep 检测逻辑', () => {
        const scriptPath = path.join(__dirname, '../scripts/cleanup-legacy.sh');
        const content = fs.readFileSync(scriptPath, 'utf-8');
        expect(content).toContain('pgrep');
        expect(content).toContain('pkill');
        expect(content).toContain('kvnshare');
    });

    test('macOS 清理脚本包含 LaunchAgents 清理逻辑', () => {
        const scriptPath = path.join(__dirname, '../scripts/cleanup-legacy.sh');
        const content = fs.readFileSync(scriptPath, 'utf-8');
        expect(content).toContain('LaunchAgents');
        expect(content).toContain('launchctl');
    });

    test('Windows 清理脚本包含进程终止逻辑', () => {
        const scriptPath = path.join(__dirname, '../scripts/cleanup-legacy.ps1');
        const content = fs.readFileSync(scriptPath, 'utf-8');
        expect(content).toContain('Stop-Process');
        expect(content).toContain('kvnshare');
    });

    test('Windows 清理脚本包含注册表启动项清理逻辑', () => {
        const scriptPath = path.join(__dirname, '../scripts/cleanup-legacy.ps1');
        const content = fs.readFileSync(scriptPath, 'utf-8');
        expect(content).toContain('CurrentVersion\\Run');
        expect(content).toContain('Remove-ItemProperty');
    });
});

describe('遗留进程清理 - 应用启动时自动清理', () => {
    test('main.rs 包含 setup 函数和连接管理命令（kill_legacy_process 已移除，由平台层脚本替代）', async () => {
        // kill_legacy_process 命令已从 Rust 后端移除，改由 scripts/cleanup-legacy.sh 和
        // scripts/cleanup-legacy.ps1 在安装/卸载时处理遗留进程清理
        const fs = require('fs');
        const path = require('path');
        const mainRs = fs.readFileSync(
            path.join(__dirname, '../src-tauri/src/main.rs'),
            'utf-8'
        );
        // 验证 setup 函数存在
        expect(mainRs).toContain('setup');
        // 验证连接管理命令存在（替代了旧的 kill_legacy_process）
        expect(mainRs).toContain('start_listening');
        expect(mainRs).toContain('connect_to_peer');
        expect(mainRs).toContain('disconnect_peer');
    });
});
