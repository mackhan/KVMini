/**
 * accessibility-module.test.ts
 * 测试 src/accessibility.js 中的辅助功能权限相关函数
 */

// ─── 全局 DOM 环境准备 ────────────────────────────────────────────────────────

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
    // 清除 Tauri mock
    delete (window as any).__TAURI__;
    // 重置 userAgent 为 macOS
    Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
        configurable: true,
    });
    // 清除所有定时器
    jest.clearAllTimers();
});

afterEach(() => {
    jest.clearAllTimers();
});

// ─── isWindowsPlatform ────────────────────────────────────────────────────────

describe('isWindowsPlatform - 平台检测', () => {
    const originalUserAgent = navigator.userAgent;

    afterEach(() => {
        Object.defineProperty(navigator, 'userAgent', {
            value: originalUserAgent,
            configurable: true,
        });
        delete (window as any).__TAURI__;
    });

    test('Windows userAgent 应返回 true', () => {
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
            configurable: true,
        });
        expect((window as any).AccessibilityModule.isWindowsPlatform()).toBe(true);
    });

    test('macOS userAgent 应返回 false', () => {
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
            configurable: true,
        });
        expect((window as any).AccessibilityModule.isWindowsPlatform()).toBe(false);
    });

    test('Tauri win32 平台应返回 true', () => {
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
            configurable: true,
        });
        (window as any).__TAURI__ = { os: { platform: 'win32' } };
        expect((window as any).AccessibilityModule.isWindowsPlatform()).toBe(true);
    });

    test('Linux userAgent 应返回 false', () => {
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36',
            configurable: true,
        });
        expect((window as any).AccessibilityModule.isWindowsPlatform()).toBe(false);
    });
});

// ─── 启动时权限引导 ───────────────────────────────────────────────────────────

describe('启动时权限引导 - checkAccessibilityPermission', () => {
    test('macOS 未授权时：显示全屏遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');
        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
        expect(mockInvoke).toHaveBeenCalledWith('check_accessibility_permission');
    });

    test('macOS 已授权时：隐藏全屏遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        // 先显示遮罩
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('Windows 平台：跳过检测，直接隐藏遮罩', async () => {
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
            configurable: true,
        });
        const mockInvoke = jest.fn();
        // 先显示遮罩
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
        // Windows 不应调用后端命令
        expect(mockInvoke).not.toHaveBeenCalled();
    });

    test('invoke 为 null 时：不崩溃，静默退出', async () => {
        await expect(
            (window as any).AccessibilityModule.checkAccessibilityPermission(null)
        ).resolves.not.toThrow();
    });

    test('后端命令异常时：隐藏遮罩（不阻断主流程）', async () => {
        const mockInvoke = jest.fn().mockRejectedValue(new Error('命令不存在'));
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('返回 Denied（英文）时也应显示遮罩', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');
        await (window as any).AccessibilityModule.checkAccessibilityPermission(mockInvoke);

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });
});

// ─── 操作时无授权拦截 ─────────────────────────────────────────────────────────

describe('操作时无授权拦截 - checkPermissionAfterConnect', () => {
    function makeElements() {
        return {
            startSharingBtn: document.getElementById('start-sharing-btn') as HTMLButtonElement,
            permissionWarning: document.getElementById('permission-warning') as HTMLElement,
        };
    }

    test('macOS 未授权时：显示权限警告，禁用开始共享按钮', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');
        const elements = makeElements();

        await (window as any).AccessibilityModule.checkPermissionAfterConnect(elements, mockInvoke);

        expect(elements.permissionWarning.classList.contains('hidden')).toBe(false);
        expect(elements.startSharingBtn.disabled).toBe(true);
    });

    test('macOS 已授权时：隐藏权限警告，启用开始共享按钮', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        const elements = makeElements();
        // 先设置为禁用状态
        elements.startSharingBtn.disabled = true;
        elements.permissionWarning.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkPermissionAfterConnect(elements, mockInvoke);

        expect(elements.permissionWarning.classList.contains('hidden')).toBe(true);
        expect(elements.startSharingBtn.disabled).toBe(false);
    });

    test('Windows 平台：不检查权限，直接启用按钮', async () => {
        Object.defineProperty(navigator, 'userAgent', {
            value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
            configurable: true,
        });
        const mockInvoke = jest.fn();
        const elements = makeElements();
        elements.startSharingBtn.disabled = true;

        await (window as any).AccessibilityModule.checkPermissionAfterConnect(elements, mockInvoke);

        expect(elements.startSharingBtn.disabled).toBe(false);
        expect(mockInvoke).not.toHaveBeenCalled();
    });

    test('连接后开始共享按钮应从隐藏变为可见', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Granted');
        const elements = makeElements();
        // 初始状态：按钮隐藏
        elements.startSharingBtn.classList.add('hidden');

        await (window as any).AccessibilityModule.checkPermissionAfterConnect(elements, mockInvoke);

        expect(elements.startSharingBtn.classList.contains('hidden')).toBe(false);
    });

    test('后端命令异常时：不阻断，隐藏警告允许继续', async () => {
        const mockInvoke = jest.fn().mockRejectedValue(new Error('权限检测失败'));
        const elements = makeElements();
        elements.permissionWarning.classList.remove('hidden');

        await (window as any).AccessibilityModule.checkPermissionAfterConnect(elements, mockInvoke);

        expect(elements.permissionWarning.classList.contains('hidden')).toBe(true);
    });
});

// ─── 权限警告显示/隐藏 ────────────────────────────────────────────────────────

describe('权限警告 UI - showPermissionWarning / hidePermissionWarning', () => {
    function makeElements() {
        return {
            startSharingBtn: document.getElementById('start-sharing-btn') as HTMLButtonElement,
            permissionWarning: document.getElementById('permission-warning') as HTMLElement,
        };
    }

    test('showPermissionWarning：显示警告，禁用按钮', () => {
        const elements = makeElements();
        (window as any).AccessibilityModule.showPermissionWarning(elements);

        expect(elements.permissionWarning.classList.contains('hidden')).toBe(false);
        expect(elements.startSharingBtn.disabled).toBe(true);
    });

    test('hidePermissionWarning：隐藏警告，启用按钮', () => {
        const elements = makeElements();
        elements.permissionWarning.classList.remove('hidden');
        elements.startSharingBtn.disabled = true;

        (window as any).AccessibilityModule.hidePermissionWarning(elements);

        expect(elements.permissionWarning.classList.contains('hidden')).toBe(true);
        expect(elements.startSharingBtn.disabled).toBe(false);
    });

    test('elements 为 null 时：不崩溃', () => {
        expect(() => {
            (window as any).AccessibilityModule.showPermissionWarning(null);
            (window as any).AccessibilityModule.hidePermissionWarning(null);
        }).not.toThrow();
    });

    test('elements 字段缺失时：不崩溃', () => {
        expect(() => {
            (window as any).AccessibilityModule.showPermissionWarning({});
            (window as any).AccessibilityModule.hidePermissionWarning({});
        }).not.toThrow();
    });
});

// ─── 打开授权设置页 ───────────────────────────────────────────────────────────

describe('打开授权设置页 - openAccessibilitySettings', () => {
    test('调用正确的后端命令 open_accessibility_settings', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);
        await (window as any).AccessibilityModule.openAccessibilitySettings(mockInvoke);

        expect(mockInvoke).toHaveBeenCalledWith('open_accessibility_settings');
    });

    test('invoke 为 null 时：不崩溃', async () => {
        await expect(
            (window as any).AccessibilityModule.openAccessibilitySettings(null)
        ).resolves.not.toThrow();
    });

    test('后端命令失败时：不崩溃，打印错误日志', async () => {
        const mockInvoke = jest.fn().mockRejectedValue(new Error('打开失败'));
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => { });

        await expect(
            (window as any).AccessibilityModule.openAccessibilitySettings(mockInvoke)
        ).resolves.not.toThrow();

        consoleSpy.mockRestore();
    });
});

// ─── 权限轮询 ─────────────────────────────────────────────────────────────────

describe('权限轮询 - startAccessibilityPolling / stopAccessibilityPolling', () => {
    beforeEach(() => {
        // 确保每次测试前轮询已停止（清除模块级变量）
        (window as any).AccessibilityModule.stopAccessibilityPolling();
        jest.useFakeTimers();
    });

    afterEach(() => {
        (window as any).AccessibilityModule.stopAccessibilityPolling();
        jest.useRealTimers();
    });

    test('重复调用 startAccessibilityPolling 不会创建多个定时器', () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');
        const setIntervalSpy = jest.spyOn(global, 'setInterval');

        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);
        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);
        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);

        expect(setIntervalSpy).toHaveBeenCalledTimes(1);
        setIntervalSpy.mockRestore();
    });

    test('stopAccessibilityPolling 停止轮询后不再调用 invoke', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('Denied');

        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);
        (window as any).AccessibilityModule.stopAccessibilityPolling();

        jest.advanceTimersByTime(10000);
        await Promise.resolve();

        expect(mockInvoke).not.toHaveBeenCalled();
    });

    test('授权后自动隐藏遮罩并停止轮询', async () => {
        // 直接返回已授权（轮询触发后立即授权）
        const mockInvoke = jest.fn().mockResolvedValue('Granted');

        // 先显示遮罩
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');

        (window as any).AccessibilityModule.startAccessibilityPolling(mockInvoke);

        // 推进 2 秒触发轮询，多次 flush 确保 async 回调完成
        jest.advanceTimersByTime(2000);
        await Promise.resolve();
        await Promise.resolve();
        await Promise.resolve();

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });
});
