/**
 * accessibility-event-driven.test.ts
 * 测试辅助功能权限引导的事件驱动新架构
 *
 * 新架构核心：
 * - Rust 侧主动检测权限并发送 Tauri 事件
 * - 前端只监听事件，被动响应，不主动 invoke
 * - 遮罩默认隐藏，收到 permission-denied 事件才显示
 * - 收到 permission-granted 事件后隐藏遮罩
 */

// ─── DOM 环境准备 ─────────────────────────────────────────────────────────────

function createAccessibilityDOM() {
    document.body.innerHTML = `
        <div id="accessibility-overlay" class="accessibility-overlay hidden"></div>
        <div id="permission-warning" class="permission-warning hidden"></div>
        <button id="open-accessibility-btn" class="btn btn-accessibility-overlay">前往授权 →</button>
        <button id="start-sharing-btn" class="btn btn-success hidden"></button>
    `;
}

// 模拟 Tauri 事件系统
let tauriEventListeners: Record<string, Array<(event: any) => void>> = {};

function setupTauriEventMock() {
    tauriEventListeners = {};
    (window as any).__TAURI__ = {
        invoke: jest.fn(),
        event: {
            listen: jest.fn((eventName: string, callback: (event: any) => void) => {
                if (!tauriEventListeners[eventName]) {
                    tauriEventListeners[eventName] = [];
                }
                tauriEventListeners[eventName].push(callback);
                // 返回 unlisten 函数
                return Promise.resolve(() => {
                    tauriEventListeners[eventName] = tauriEventListeners[eventName].filter(cb => cb !== callback);
                });
            }),
            emit: jest.fn(),
        },
    };
}

/** 触发 Tauri 事件（模拟 Rust 发送事件） */
function fireTauriEvent(eventName: string, payload?: any) {
    const listeners = tauriEventListeners[eventName] || [];
    listeners.forEach(cb => cb({ event: eventName, payload }));
}

beforeAll(() => {
    (window as any).i18n = { t: (key: string) => key };
});

beforeEach(() => {
    createAccessibilityDOM();
    setupTauriEventMock();
    jest.clearAllTimers();
    jest.clearAllMocks();
    // 重新加载模块（每次测试前重置状态）
    jest.resetModules();
});

afterEach(() => {
    jest.clearAllTimers();
});

// ─── 需求 3：前端被动响应权限事件 ────────────────────────────────────────────

describe('需求 3：前端被动响应权限事件', () => {
    test('3.1 收到 permission-denied 事件时：遮罩应显示（移除 hidden class）', () => {
        require('../src/accessibility.js');

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true); // 初始隐藏

        fireTauriEvent('accessibility://permission-denied');

        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('3.2 收到 permission-granted 事件时：遮罩应隐藏（添加 hidden class）', () => {
        require('../src/accessibility.js');

        // 先显示遮罩
        const overlay = document.getElementById('accessibility-overlay');
        overlay?.classList.remove('hidden');
        expect(overlay?.classList.contains('hidden')).toBe(false);

        fireTauriEvent('accessibility://permission-granted');

        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('3.3 页面加载完成时：应注册两个事件监听器（不依赖 invoke）', () => {
        require('../src/accessibility.js');

        // 验证两个事件都已注册监听器
        expect(tauriEventListeners['accessibility://permission-denied']).toBeDefined();
        expect(tauriEventListeners['accessibility://permission-denied']?.length).toBeGreaterThan(0);
        expect(tauriEventListeners['accessibility://permission-granted']).toBeDefined();
        expect(tauriEventListeners['accessibility://permission-granted']?.length).toBeGreaterThan(0);
    });

    test('3.3 事件监听器注册不依赖 invoke 是否就绪（__TAURI__ 存在即可）', () => {
        // 不设置 invoke，只有 event.listen
        (window as any).__TAURI__ = {
            event: {
                listen: jest.fn((eventName: string, callback: (event: any) => void) => {
                    if (!tauriEventListeners[eventName]) {
                        tauriEventListeners[eventName] = [];
                    }
                    tauriEventListeners[eventName].push(callback);
                    return Promise.resolve(() => { });
                }),
            },
        };

        // 不应抛出异常
        expect(() => {
            require('../src/accessibility.js');
        }).not.toThrow();

        // 事件监听器应已注册
        expect(tauriEventListeners['accessibility://permission-denied']?.length).toBeGreaterThan(0);
    });

    test('3.4 遮罩 DOM 不存在时：不抛出异常，只记录警告', () => {
        // 清空 DOM
        document.body.innerHTML = '';

        const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => { });

        expect(() => {
            require('../src/accessibility.js');
            fireTauriEvent('accessibility://permission-denied');
            fireTauriEvent('accessibility://permission-granted');
        }).not.toThrow();

        warnSpy.mockRestore();
    });

    test('3.5 遮罩显示时：z-index 应足够高（CSS 中已定义，JS 不覆盖）', () => {
        require('../src/accessibility.js');

        fireTauriEvent('accessibility://permission-denied');

        const overlay = document.getElementById('accessibility-overlay');
        // 遮罩应显示（hidden class 已移除）
        expect(overlay?.classList.contains('hidden')).toBe(false);
        // z-index 由 CSS 控制，这里只验证遮罩确实显示
        expect(overlay?.classList.contains('accessibility-overlay')).toBe(true);
    });

    test('多次收到 permission-denied 事件：遮罩保持显示，不重复操作', () => {
        require('../src/accessibility.js');

        const overlay = document.getElementById('accessibility-overlay');

        fireTauriEvent('accessibility://permission-denied');
        fireTauriEvent('accessibility://permission-denied');
        fireTauriEvent('accessibility://permission-denied');

        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('先 denied 后 granted：遮罩最终隐藏', () => {
        require('../src/accessibility.js');

        const overlay = document.getElementById('accessibility-overlay');

        fireTauriEvent('accessibility://permission-denied');
        expect(overlay?.classList.contains('hidden')).toBe(false);

        fireTauriEvent('accessibility://permission-granted');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });
});

// ─── 需求 4：前往授权按钮功能 ─────────────────────────────────────────────────

describe('需求 4：前往授权按钮功能', () => {
    test('4.1 点击前往授权按钮：应调用 invoke("open_accessibility_settings")', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);
        (window as any).__TAURI__.invoke = mockInvoke;

        require('../src/accessibility.js');

        const btn = document.getElementById('open-accessibility-btn');
        expect(btn).not.toBeNull();

        btn?.click();
        await Promise.resolve();

        expect(mockInvoke).toHaveBeenCalledWith('open_accessibility_settings');
    });

    test('4.3 打开系统设置调用失败：不影响遮罩显示，只记录错误', async () => {
        const mockInvoke = jest.fn().mockRejectedValue(new Error('打开失败'));
        (window as any).__TAURI__.invoke = mockInvoke;
        const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => { });

        require('../src/accessibility.js');

        // 先显示遮罩
        const overlay = document.getElementById('accessibility-overlay');
        overlay?.classList.remove('hidden');

        const btn = document.getElementById('open-accessibility-btn');
        btn?.click();
        await Promise.resolve();
        await Promise.resolve();

        // 遮罩应仍然显示（不因按钮点击失败而关闭）
        expect(overlay?.classList.contains('hidden')).toBe(false);
        consoleSpy.mockRestore();
    });

    test('4.4 点击按钮后：不关闭遮罩（等待 Rust 轮询授权后自动关闭）', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);
        (window as any).__TAURI__.invoke = mockInvoke;

        require('../src/accessibility.js');

        // 先显示遮罩
        const overlay = document.getElementById('accessibility-overlay');
        overlay?.classList.remove('hidden');

        const btn = document.getElementById('open-accessibility-btn');
        btn?.click();
        await Promise.resolve();
        await Promise.resolve();

        // 点击按钮后遮罩不应自动关闭
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });
});

// ─── 需求 5：旧实现已清理 ─────────────────────────────────────────────────────

describe('需求 5：旧实现已清理', () => {
    test('5.1 accessibility.js 不应再包含 checkAccessibilityPermission 函数', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/accessibility.js'), 'utf-8');

        // 旧函数应已删除
        expect(content).not.toContain('function checkAccessibilityPermission');
        expect(content).not.toContain('function startAccessibilityPolling');
        expect(content).not.toContain('function stopAccessibilityPolling');
    });

    test('5.2 main.js 不应再调用 checkAccessibilityPermission()', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');

        // main.js 中不应再有 checkAccessibilityPermission 的调用
        // 注意：函数定义也应删除
        expect(content).not.toContain('checkAccessibilityPermission()');
        expect(content).not.toContain("invoke('check_accessibility_permission')");
    });

    test('5.3 initApp() 中不应有 invoke("check_accessibility_permission") 调用', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');

        expect(content).not.toContain("invoke('check_accessibility_permission')");
        expect(content).not.toContain('invoke("check_accessibility_permission")');
    });
});

// ─── 需求 6：checkPermissionAfterConnect 保留（连接后权限检查） ───────────────

describe('需求 6：checkPermissionAfterConnect 保留（连接后权限检查）', () => {
    test('AccessibilityModule 应仍然导出 checkPermissionAfterConnect', () => {
        require('../src/accessibility.js');
        expect(typeof (window as any).AccessibilityModule?.checkPermissionAfterConnect).toBe('function');
    });

    test('AccessibilityModule 应仍然导出 showPermissionWarning / hidePermissionWarning', () => {
        require('../src/accessibility.js');
        expect(typeof (window as any).AccessibilityModule?.showPermissionWarning).toBe('function');
        expect(typeof (window as any).AccessibilityModule?.hidePermissionWarning).toBe('function');
    });

    test('AccessibilityModule 应仍然导出 openAccessibilitySettings', () => {
        require('../src/accessibility.js');
        expect(typeof (window as any).AccessibilityModule?.openAccessibilitySettings).toBe('function');
    });
});

// ─── 边界情况：__TAURI__ 不存在时 ────────────────────────────────────────────

describe('边界情况：__TAURI__ 不存在时', () => {
    test('__TAURI__ 为 undefined 时：不抛出异常', () => {
        delete (window as any).__TAURI__;

        expect(() => {
            require('../src/accessibility.js');
        }).not.toThrow();
    });

    test('__TAURI__.event 为 undefined 时：不抛出异常', () => {
        (window as any).__TAURI__ = { invoke: jest.fn() };

        expect(() => {
            require('../src/accessibility.js');
        }).not.toThrow();
    });
});
