/**
 * accessibility-module.test.ts
 * 测试 src/accessibility.js 中的辅助功能权限相关函数
 */

beforeAll(() => {
    (window as any).i18n = { t: (key: string) => key };
    require('../src/accessibility.js');
});

describe('accessibility.js - isWindowsPlatform', () => {
    const originalUserAgent = navigator.userAgent;

    afterEach(() => {
        // 恢复 userAgent
        Object.defineProperty(navigator, 'userAgent', {
            value: originalUserAgent,
            configurable: true,
        });
        // 清除 Tauri mock
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
