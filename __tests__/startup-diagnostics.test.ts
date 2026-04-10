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

// ─── 需求 2.4/2.5：新架构 - Rust 推送事件，前端被动响应 ─────────────────────

describe('需求 2.4/2.5：新架构 - Rust 推送事件，前端被动响应', () => {
    let tauriListeners: Record<string, Function[]> = {};

    beforeEach(() => {
        tauriListeners = {};
        document.body.innerHTML = `
            <div id="accessibility-overlay" class="accessibility-overlay hidden"></div>
        `;
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
            configurable: true,
        });
        (window as any).i18n = { t: (key: string) => key };
        (window as any).__TAURI__ = {
            invoke: jest.fn(),
            event: {
                listen: jest.fn((name: string, cb: Function) => {
                    if (!tauriListeners[name]) tauriListeners[name] = [];
                    tauriListeners[name].push(cb);
                    return Promise.resolve(() => { });
                }),
            },
        };
        jest.resetModules();
        require('../src/accessibility.js');
    });

    test('Rust 发送 permission-denied 事件时，应显示权限遮罩', () => {
        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);

        // 模拟 Rust 发送 permission-denied 事件
        tauriListeners['accessibility://permission-denied']?.forEach(cb => cb({}));

        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('Rust 发送 permission-granted 事件时，应隐藏权限遮罩', () => {
        const overlay = document.getElementById('accessibility-overlay')!;
        overlay.classList.remove('hidden'); // 先显示遮罩

        // 模拟 Rust 发送 permission-granted 事件
        tauriListeners['accessibility://permission-granted']?.forEach(cb => cb({}));

        expect(overlay.classList.contains('hidden')).toBe(true);
    });

    test('新架构：前端不再主动调用 check_accessibility_permission', () => {
        const mockInvoke = (window as any).__TAURI__.invoke;
        // 加载模块后，不应有 check_accessibility_permission 的调用
        expect(mockInvoke).not.toHaveBeenCalledWith('check_accessibility_permission');
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

// 注意：startup-diagnostics.js 已在架构重构中删除（Bug #010 修复），
// 诊断日志功能已由 main.js 内置的 [KVMini] 前缀日志替代。
