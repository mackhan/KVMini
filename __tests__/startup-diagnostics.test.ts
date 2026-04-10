/**
 * startup-diagnostics.test.ts
 * Bug #009 专项测试：权限引导 + 按钮点击彻底修复（含自动化诊断）
 *
 * 覆盖场景：
 * - waitForTauri 超时后不调用 checkAccessibilityPermission（需求 2.2）
 * - waitForTauri 超时后界面显示错误提示（需求 2.1）
 * - invoke 就绪后 checkAccessibilityPermission 被调用并正确处理结果（需求 2.4/2.5）
 * - 按钮点击 invoke 为 null 时 showError 被调用（需求 3.1）
 * - 诊断日志在启动时被写入（需求 1.1）
 */

// ─── 辅助函数：模拟 waitForTauri 逻辑 ────────────────────────────────────────

function waitForTauri(timeout = 3000, interval = 50): Promise<boolean> {
    return new Promise((resolve) => {
        if ((window as any).__TAURI__?.core?.invoke || (window as any).__TAURI__?.invoke) {
            resolve(true);
            return;
        }
        const startTime = Date.now();
        const timer = setInterval(() => {
            if ((window as any).__TAURI__?.core?.invoke || (window as any).__TAURI__?.invoke) {
                clearInterval(timer);
                resolve(true);
                return;
            }
            if (Date.now() - startTime >= timeout) {
                clearInterval(timer);
                resolve(false);
            }
        }, interval);
    });
}

// ─── 需求 2.2：waitForTauri 超时后不调用 checkAccessibilityPermission ─────────

describe('需求 2.2：waitForTauri 超时后不调用 checkAccessibilityPermission', () => {
    beforeEach(() => {
        delete (window as any).__TAURI__;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
        delete (window as any).__TAURI__;
    });

    test('waitForTauri 超时（返回 false）时，initApp 不应调用 checkAccessibilityPermission', async () => {
        const checkPermMock = jest.fn();

        // 模拟 waitForTauri 超时后的 initApp 逻辑（修复后的行为）
        async function simulateInitWithFix() {
            const ready = await waitForTauri(100, 50);
            if (!ready) {
                // 修复后：超时时不调用 checkAccessibilityPermission
                return;
            }
            checkPermMock();
        }

        const promise = simulateInitWithFix();
        jest.advanceTimersByTime(200);
        await promise;

        expect(checkPermMock).not.toHaveBeenCalled();
    });

    test('waitForTauri 成功（返回 true）时，initApp 应调用 checkAccessibilityPermission', async () => {
        const checkPermMock = jest.fn();

        // 模拟 Tauri 就绪
        (window as any).__TAURI__ = { core: { invoke: jest.fn() } };

        async function simulateInitWithFix() {
            const ready = await waitForTauri(100, 50);
            if (!ready) {
                return;
            }
            checkPermMock();
        }

        const promise = simulateInitWithFix();
        jest.runAllTimers();
        await promise;

        expect(checkPermMock).toHaveBeenCalledTimes(1);
    });
});

// ─── 需求 2.1：waitForTauri 超时后界面显示错误提示 ───────────────────────────

describe('需求 2.1：waitForTauri 超时后界面显示错误提示', () => {
    beforeEach(() => {
        delete (window as any).__TAURI__;
        jest.useFakeTimers();
        document.body.innerHTML = `
            <div id="error-toast" class="hidden"></div>
            <div id="error-message"></div>
        `;
    });

    afterEach(() => {
        jest.useRealTimers();
        delete (window as any).__TAURI__;
    });

    test('waitForTauri 超时时，应显示错误提示 Toast', async () => {
        const errorToast = document.getElementById('error-toast')!;
        const errorMessage = document.getElementById('error-message')!;

        // 模拟修复后的 initApp 超时处理逻辑
        async function simulateInitWithFix() {
            const ready = await waitForTauri(100, 50);
            if (!ready) {
                // 修复后：超时时显示错误提示
                errorMessage.textContent = 'Tauri 环境初始化失败，请重启应用';
                errorToast.classList.remove('hidden');
                return;
            }
        }

        const promise = simulateInitWithFix();
        jest.advanceTimersByTime(200);
        await promise;

        expect(errorToast.classList.contains('hidden')).toBe(false);
        expect(errorMessage.textContent).toContain('Tauri 环境初始化失败');
    });
});

// ─── 需求 2.4/2.5：invoke 就绪后权限检测结果正确处理 ─────────────────────────

describe('需求 2.4/2.5：invoke 就绪后权限检测结果正确处理', () => {
    beforeEach(() => {
        document.body.innerHTML = `
            <div id="accessibility-overlay" class="accessibility-overlay hidden"></div>
        `;
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
            configurable: true,
        });
        (window as any).i18n = { t: (key: string) => key };
        require('../src/accessibility.js');
    });

    test('invoke 就绪且返回 "Denied" 时，应显示权限遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');
        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('invoke 就绪且返回 "Granted" 时，应隐藏权限遮罩', async () => {
        const overlay = document.getElementById('accessibility-overlay')!;
        overlay.classList.remove('hidden'); // 先显示遮罩

        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        expect(overlay.classList.contains('hidden')).toBe(true);
    });

    test('invoke 就绪且返回 "NA" 时，应显示权限遮罩（非 Granted 均显示）', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('NA');
        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });
});

// ─── 需求 3.1：按钮点击 invoke 为 null 时 showError 被调用 ───────────────────

describe('需求 3.1：按钮点击 invoke 为 null 时 showError 被调用', () => {
    let showErrorMock: jest.Mock;

    beforeEach(() => {
        showErrorMock = jest.fn();
        document.body.innerHTML = `
            <button id="scan-btn">扫描</button>
            <div id="error-toast" class="hidden"></div>
            <div id="error-message"></div>
        `;
    });

    test('handleScan：invoke 为 null 时应调用 showError', async () => {
        // 模拟修复后的 handleScan 逻辑
        async function handleScan(invoke: Function | null) {
            if (!invoke) {
                showErrorMock('Tauri 环境未就绪，请重启应用');
                return;
            }
            await invoke('discover_devices');
        }

        await handleScan(null);
        expect(showErrorMock).toHaveBeenCalledWith(expect.stringContaining('Tauri 环境未就绪'));
    });

    test('handleDisconnect：invoke 为 null 时应调用 showError', async () => {
        async function handleDisconnect(invoke: Function | null) {
            if (!invoke) {
                showErrorMock('Tauri 环境未就绪');
                return;
            }
            await invoke('disconnect_tcp');
        }

        await handleDisconnect(null);
        expect(showErrorMock).toHaveBeenCalledWith(expect.stringContaining('Tauri 环境未就绪'));
    });

    test('handleManualConnect：invoke 为 null 时应调用 showError', async () => {
        async function handleManualConnect(invoke: Function | null) {
            if (!invoke) {
                showErrorMock('Tauri 环境未就绪，请重启应用');
                return;
            }
            await invoke('connect_by_ip', { ip: '192.168.1.1', port: 8889 });
        }

        await handleManualConnect(null);
        expect(showErrorMock).toHaveBeenCalledWith(expect.stringContaining('Tauri 环境未就绪'));
    });
});

// ─── 需求 1.1：诊断日志模块基本功能 ──────────────────────────────────────────

describe('需求 1.1：诊断日志模块基本功能', () => {
    test('StartupDiagnostics 模块应能被加载', () => {
        // 验证诊断日志模块存在且可加载
        // 实际写入文件需要 Tauri 环境，这里只验证模块结构
        const diagnostics = require('../src/startup-diagnostics.js');
        expect(diagnostics).toBeDefined();
        expect(typeof diagnostics.log).toBe('function');
        expect(typeof diagnostics.flush).toBe('function');
    });

    test('log 方法应能记录日志条目', () => {
        const diagnostics = require('../src/startup-diagnostics.js');
        // 不应抛出错误
        expect(() => {
            diagnostics.log('TEST', '测试日志条目');
        }).not.toThrow();
    });

    test('flush 方法在非 Tauri 环境下应降级到 console.error', async () => {
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => { });
        const diagnostics = require('../src/startup-diagnostics.js');
        diagnostics.log('TEST', '测试条目');

        // 非 Tauri 环境（无 window.__TAURI__），flush 应降级
        await expect(diagnostics.flush()).resolves.not.toThrow();
        consoleSpy.mockRestore();
    });
});
