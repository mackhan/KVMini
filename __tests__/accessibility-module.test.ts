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

// ─── 启动时权限引导（新架构：事件驱动） ────────────────────────────────────

describe('启动时权限引导 - 新架构：事件驱动', () => {
    let tauriListeners: Record<string, Function[]> = {};

    beforeEach(() => {
        tauriListeners = {};
        createAccessibilityDOM();
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

    test('收到 permission-denied 事件时：显示全屏遮罩', () => {
        tauriListeners['accessibility://permission-denied']?.forEach(cb => cb({}));
        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('收到 permission-granted 事件时：隐藏全屏遮罩', () => {
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');
        tauriListeners['accessibility://permission-granted']?.forEach(cb => cb({}));
        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('Windows 平台：不发送任何权限事件（Rust 侧控制）', () => {
        // 新架构：Windows 平台由 Rust 侧不发送事件，前端无需处理
        // 验证：即使没有收到事件，遮罩也保持隐藏
        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('__TAURI__ 不存在时：不崩溃', () => {
        delete (window as any).__TAURI__;
        jest.resetModules();
        expect(() => require('../src/accessibility.js')).not.toThrow();
    });

    test('遮罩 DOM 不存在时：不崩溃', () => {
        document.body.innerHTML = '';
        expect(() => {
            tauriListeners['accessibility://permission-denied']?.forEach(cb => cb({}));
        }).not.toThrow();
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

// ─── 权限轮询（新架构：由 Rust 后台线程负责，前端无轮询逻辑） ────────────────

describe('权限轮询 - 新架构：由 Rust 后台线程负责', () => {
    test('AccessibilityModule 不再导出 startAccessibilityPolling', () => {
        expect((window as any).AccessibilityModule.startAccessibilityPolling).toBeUndefined();
    });

    test('AccessibilityModule 不再导出 stopAccessibilityPolling', () => {
        expect((window as any).AccessibilityModule.stopAccessibilityPolling).toBeUndefined();
    });

    test('AccessibilityModule 不再导出 checkAccessibilityPermission', () => {
        expect((window as any).AccessibilityModule.checkAccessibilityPermission).toBeUndefined();
    });

    test('AccessibilityModule 导出 initAccessibilityEventListeners', () => {
        expect(typeof (window as any).AccessibilityModule.initAccessibilityEventListeners).toBe('function');
    });
});
