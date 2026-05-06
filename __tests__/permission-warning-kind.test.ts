/**
 * BUG-031：横幅需按 kind 区分 Accessibility / Input Monitoring
 *
 * 现象：用户在系统「辅助功能」里已授权 KVMini，但因「输入监控」未授权，
 *       横幅仍显示"需要辅助功能权限"——文案误导，且无可操作的跳转按钮。
 *
 * 期望行为（TDD 先定义）：
 *   1. showPermissionWarning 接受第二参数 kind ∈ {'accessibility','input-monitoring'}
 *   2. kind='input-monitoring' 时文案含"输入监控"且提示需重启
 *   3. 横幅 DOM 中存在 #permission-warning-btn，按 kind 切换文案和 data-kind
 *   4. 不传 kind 时默认 'accessibility'（向后兼容）
 *   5. i18n 字典 zh/en 均含 4 条新 key
 */

import { MESSAGES, t, setLocale } from '../src/i18n.js';

describe('BUG-031 i18n 新增 4 条 key（zh + en）', () => {
    const REQUIRED_KEYS = [
        'permissionWarningText',
        'permissionWarningInputMonitoringText',
        'permissionWarningBtnAccessibility',
        'permissionWarningBtnInputMonitoring',
    ];

    test.each(['zh', 'en'])('%s 字典必须包含所有必需 key 且非空', (locale) => {
        const dict = (MESSAGES as any)[locale];
        expect(dict).toBeDefined();
        for (const key of REQUIRED_KEYS) {
            expect(dict[key]).toBeDefined();
            expect(typeof dict[key]).toBe('string');
            expect(dict[key].length).toBeGreaterThan(0);
        }
    });

    test('中文「输入监控」文案必须提到输入监控 + 重启', () => {
        const zh = (MESSAGES as any).zh;
        expect(zh.permissionWarningInputMonitoringText).toMatch(/输入监控/);
        expect(zh.permissionWarningInputMonitoringText).toMatch(/重启|重新打开|重新启动/);
    });

    test('英文「Input Monitoring」文案必须提到 Input Monitoring + restart/relaunch', () => {
        const en = (MESSAGES as any).en;
        expect(en.permissionWarningInputMonitoringText).toMatch(/Input Monitoring/i);
        expect(en.permissionWarningInputMonitoringText.toLowerCase()).toMatch(/restart|relaunch|reopen/);
    });
});

describe('BUG-031 横幅按 kind 区分权限类型', () => {
    beforeEach(() => {
        // 完整还原横幅 DOM 结构（含新增按钮）
        document.body.innerHTML = `
            <div id="permission-warning" class="permission-warning hidden">
                <span class="permission-warning-icon">⚠️</span>
                <span class="permission-warning-text" data-i18n="permissionWarningText"></span>
                <button id="permission-warning-btn" class="permission-warning-btn hidden" data-kind="accessibility"></button>
            </div>
        `;
        setLocale('zh');
        // 确保 accessibility.js 能用 i18n.t
        (window as any).i18n = { t, setLocale };

        // 清理模块缓存，保证每个 test 都是独立加载
        jest.resetModules();
        // 同步清理旧的 AccessibilityModule 挂载
        delete (window as any).AccessibilityModule;

        // 加载 accessibility.js（会挂载 window.AccessibilityModule）
        require('../src/accessibility.js');
    });

    test('Accessibility 横幅：文案=「辅助功能」，按钮 data-kind=accessibility 且可见', () => {
        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements, 'accessibility');

        const text = document.querySelector('.permission-warning-text')!.textContent!;
        const btn = document.getElementById('permission-warning-btn')!;

        expect(text).toMatch(/辅助功能/);
        expect(text).not.toMatch(/输入监控/);
        expect(btn.getAttribute('data-kind')).toBe('accessibility');
        expect(btn.classList.contains('hidden')).toBe(false);
        expect(btn.textContent).toMatch(/辅助功能/);
    });

    test('Input Monitoring 横幅：文案=「输入监控」+ 重启提示，按钮 data-kind=input-monitoring', () => {
        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');

        const text = document.querySelector('.permission-warning-text')!.textContent!;
        const btn = document.getElementById('permission-warning-btn')!;

        expect(text).toMatch(/输入监控/);
        expect(text).toMatch(/重启|重新打开|重新启动/);
        expect(btn.getAttribute('data-kind')).toBe('input-monitoring');
        expect(btn.classList.contains('hidden')).toBe(false);
        expect(btn.textContent).toMatch(/输入监控/);
    });

    test('英文 locale：Input Monitoring 文案含 Input Monitoring + restart', () => {
        setLocale('en');
        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');

        const text = document.querySelector('.permission-warning-text')!.textContent!;
        expect(text).toMatch(/Input Monitoring/i);
        expect(text.toLowerCase()).toMatch(/restart|relaunch|reopen/);
    });

    test('不传 kind 时默认 accessibility（向后兼容）', () => {
        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements);

        const btn = document.getElementById('permission-warning-btn')!;
        expect(btn.getAttribute('data-kind')).toBe('accessibility');
    });

    test('hidePermissionWarning 隐藏按钮', () => {
        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');
        const btn = document.getElementById('permission-warning-btn')!;
        expect(btn.classList.contains('hidden')).toBe(false);

        (window as any).AccessibilityModule.hidePermissionWarning(elements);
        expect(btn.classList.contains('hidden')).toBe(true);
    });

    test('点击按钮：data-kind=input-monitoring 调 open_input_monitoring_settings', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(null);
        (window as any).__TAURI__ = {
            core: { invoke: mockInvoke },
            event: { listen: jest.fn(), emit: jest.fn() },
        };

        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');

        const btn = document.getElementById('permission-warning-btn')!;
        btn.click();
        // 等 microtask flush
        await Promise.resolve();

        expect(mockInvoke).toHaveBeenCalledWith('open_input_monitoring_settings');
    });

    test('点击按钮：data-kind=accessibility 调 open_accessibility_settings', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(null);
        (window as any).__TAURI__ = {
            core: { invoke: mockInvoke },
            event: { listen: jest.fn(), emit: jest.fn() },
        };

        const elements = {
            permissionWarning: document.getElementById('permission-warning'),
            startSharingBtn: null,
        };
        (window as any).AccessibilityModule.showPermissionWarning(elements, 'accessibility');

        const btn = document.getElementById('permission-warning-btn')!;
        btn.click();
        await Promise.resolve();

        expect(mockInvoke).toHaveBeenCalledWith('open_accessibility_settings');
    });
});
