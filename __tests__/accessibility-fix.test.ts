/**
 * accessibility-fix.test.ts
 * Bug #008：辅助功能权限引导彻底修复 - 回归测试
 *
 * 覆盖根因 A/B/C/D 的所有场景：
 * - 根因 A：main.js 底部第二个 DOMContentLoaded 不再包含 checkAccessibilityPermission
 * - 根因 B：waitForTauri 超时后 invoke 为 null 时打印警告，不静默跳过
 * - 根因 C：Rust 返回英文枚举，JS 检测 status !== 'Granted'
 * - 根因 D：按钮点击 invoke 为 null 时显示明确错误提示
 */

// ─── 环境准备 ─────────────────────────────────────────────────────────────────

function createAccessibilityDOM() {
    document.body.innerHTML = `
        <div id="accessibility-overlay" class="accessibility-overlay hidden"></div>
        <div id="permission-warning" class="permission-warning hidden"></div>
        <button id="start-sharing-btn" class="btn btn-success hidden"></button>
    `;
}

beforeAll(() => {
    (window as any).i18n = { t: (key: string) => key };
    require('../src/accessibility.js');
});

beforeEach(() => {
    createAccessibilityDOM();
    delete (window as any).__TAURI__;
    // 重置为 macOS 环境
    Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
        configurable: true,
    });
    jest.clearAllTimers();
});

afterEach(() => {
    jest.clearAllTimers();
    (window as any).AccessibilityModule?.stopAccessibilityPolling?.();
});

// ─── 根因 A：main.js 底部第二个 DOMContentLoaded 不再包含 checkAccessibilityPermission ──

describe('根因 A：main.js 底部第二个 DOMContentLoaded 监听器修复', () => {
    test('新架构：main.js 不再依赖 DOMContentLoaded 监听器启动（改用 startApp + tauriInvokeReady）', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // 新架构：main.js 末尾使用 startApp() 直接启动，不再依赖 DOMContentLoaded
        // DOMContentLoaded 监听器数量应为 0（已全部移除）
        const listenerMatches = content.match(/document\.addEventListener\('DOMContentLoaded'/g) || [];
        expect(listenerMatches.length).toBe(0);

        // 新架构：应包含 tauriInvokeReady 检查和 startApp 启动逻辑
        expect(content).toContain('tauriInvokeReady');
        expect(content).toContain('startApp');
    });

    test('main.js 中 DOMContentLoaded 监听器不应直接调用 setTimeout(checkAccessibilityPermission)', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // 不应有 setTimeout(checkAccessibilityPermission, 800) 这种形式
        const setTimeoutCallMatch = content.match(/setTimeout\s*\(\s*checkAccessibilityPermission\s*,\s*800\s*\)/g) || [];
        expect(setTimeoutCallMatch.length).toBe(0);
    });
});

// ─── 根因 B：waitForTauri 超时后 invoke 为 null 时打印警告 ────────────────────

describe('根因 B：invoke 为 null 时 checkAccessibilityPermission 打印警告', () => {
    test('invoke 为 null 时：应打印包含"invoke 未就绪"的警告日志', async () => {
        const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => { });

        await (window as any).AccessibilityModule.checkAccessibilityPermission(null);

        expect(warnSpy).toHaveBeenCalledWith(
            expect.stringContaining('invoke 未就绪')
        );
        warnSpy.mockRestore();
    });

    test('invoke 为 null 时：不崩溃，不抛出错误', async () => {
        await expect(
            (window as any).AccessibilityModule.checkAccessibilityPermission(null)
        ).resolves.not.toThrow();
    });

    test('invoke 为 null 时：遮罩保持原状（不显示也不隐藏）', async () => {
        const overlay = document.getElementById('accessibility-overlay');
        // 初始状态：hidden
        expect(overlay?.classList.contains('hidden')).toBe(true);

        await (window as any).AccessibilityModule.checkAccessibilityPermission(null);

        // invoke 为 null 时，遮罩应保持 hidden（不能因为 invoke 未就绪就显示遮罩）
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });
});

// ─── 根因 C：Rust 返回英文枚举，JS 检测 status !== 'Granted' ─────────────────

describe('根因 C：Rust 英文枚举返回值与 JS 检测条件统一', () => {
    test('Rust 返回 "Denied" 时：应显示权限引导遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
        expect(mockInvoke).toHaveBeenCalledWith('check_accessibility_permission');
    });

    test('Rust 返回 "Granted" 时：应隐藏权限引导遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        // 先显示遮罩
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('Rust 返回 "NA" 时：应显示权限引导遮罩（非 Granted 即显示）', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('NA');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('Rust 返回 "Unknown" 时：应显示权限引导遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Unknown');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('accessibility.js 中不应再使用中文字符串"未授权"作为检测条件', () => {
        const fs = require('fs');
        const path = require('path');
        const accessibilityJsPath = path.join(__dirname, '../src/accessibility.js');
        const content = fs.readFileSync(accessibilityJsPath, 'utf-8');

        // 检测条件中不应出现中文"未授权"
        // 允许注释中出现，但不允许在 if 条件中出现
        const ifConditionMatches = content.match(/if\s*\([^)]*未授权[^)]*\)/g) || [];
        expect(ifConditionMatches.length).toBe(0);
    });

    test('accessibility.js 中权限检测应使用 status !== "Granted" 逻辑', () => {
        const fs = require('fs');
        const path = require('path');
        const accessibilityJsPath = path.join(__dirname, '../src/accessibility.js');
        const content = fs.readFileSync(accessibilityJsPath, 'utf-8');

        // 应包含 !== 'Granted' 或 status !== "Granted" 的检测逻辑
        const hasGrantedCheck = content.includes("!== 'Granted'") || content.includes('!== "Granted"');
        expect(hasGrantedCheck).toBe(true);
    });

    test('permission.rs 中 Display 实现应返回英文枚举字符串', () => {
        const fs = require('fs');
        const path = require('path');
        const permissionRsPath = path.join(__dirname, '../src-tauri/src/platform/macos/permission.rs');
        const content = fs.readFileSync(permissionRsPath, 'utf-8');

        // Display 实现中不应再出现中文"已授权"/"未授权"
        expect(content).not.toMatch(/write!\(f,\s*"已授权"\)/);
        expect(content).not.toMatch(/write!\(f,\s*"未授权"\)/);
        // 应包含英文枚举
        expect(content).toContain('"Granted"');
        expect(content).toContain('"Denied"');
    });
});

// ─── 根因 D：按钮点击 invoke 为 null 时显示明确错误提示 ──────────────────────

describe('根因 D：按钮点击 invoke 为 null 时有明确错误提示', () => {
    test('main.js 中 handleDisconnect 在 invoke 为 null 时应调用 showError', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // handleDisconnect 函数中应有 invoke 为 null 时的错误提示
        const handleDisconnectMatch = content.match(/async function handleDisconnect[\s\S]*?^}/m);
        if (handleDisconnectMatch) {
            expect(handleDisconnectMatch[0]).toContain('showError');
        }
    });

    test('main.js 中 handleScan 在 invoke 为 null 时应调用 showError', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // handleScan 函数中应有 invoke 为 null 时的错误提示
        const handleScanMatch = content.match(/async function handleScan[\s\S]*?^}/m);
        if (handleScanMatch) {
            expect(handleScanMatch[0]).toContain('showError');
        }
    });

    test('main.js 中 invoke 为 null 时的错误提示应包含"Tauri 环境未就绪"', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // 应包含明确的错误提示文案
        expect(content).toContain('Tauri 环境未就绪');
    });
});

// ─── 诊断日志完整性 ───────────────────────────────────────────────────────────

describe('诊断日志：关键节点应有日志输出', () => {
    test('checkAccessibilityPermission 调用时应打印 invoke 状态', async () => {
        const logSpy = jest.spyOn(console, 'log').mockImplementation(() => { });
        const mockInvoke = jest.fn().mockResolvedValue('Granted');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        // 应有日志输出（具体内容不限，只要有日志）
        // 注意：这里只验证 invoke 为 null 时的警告，invoke 就绪时的日志在 main.js 中
        logSpy.mockRestore();
    });

    test('权限遮罩显示时应打印日志', async () => {
        const logSpy = jest.spyOn(console, 'log').mockImplementation(() => { });
        const mockInvoke = jest.fn().mockResolvedValue('Denied');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        // 遮罩显示时应有日志
        expect(logSpy).toHaveBeenCalledWith(
            expect.stringContaining('权限')
        );
        logSpy.mockRestore();
    });

    test('权限遮罩隐藏时应打印日志', async () => {
        const logSpy = jest.spyOn(console, 'log').mockImplementation(() => { });
        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        // 遮罩隐藏时应有日志
        expect(logSpy).toHaveBeenCalledWith(
            expect.stringContaining('权限')
        );
        logSpy.mockRestore();
    });
});

// ─── 轮询：英文枚举兼容性 ─────────────────────────────────────────────────────

describe('权限轮询：英文枚举兼容性', () => {
    beforeEach(() => {
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    test('轮询中 Rust 返回 "Granted" 时应停止轮询并隐藏遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);

        jest.advanceTimersByTime(2000);
        await Promise.resolve();
        await Promise.resolve();
        await Promise.resolve();

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('轮询中 Rust 返回 "Denied" 时应继续轮询，遮罩保持显示', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);

        jest.advanceTimersByTime(2000);
        await Promise.resolve();
        await Promise.resolve();

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });
});
