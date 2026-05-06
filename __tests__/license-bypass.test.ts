/**
 * license-bypass.test.ts
 *
 * 验证 v0.1.37 起的默认屏蔽行为：
 *   - LicenseModule 默认不调用 get_trial_status / purchase / restore_purchase
 *   - checkLicenseStatus 直接返回 { licensed: true, status: 'licensed' }
 *   - updateLicenseUI 自动隐藏 license-gate，主界面可见
 *   - 后端命令未被调用 → 说明 Keychain 记录完全不被触碰
 *
 * 将来要恢复商业化 UI：把 src/license.js 顶部 LICENSE_GATE_BYPASS 改为 false
 * （或测试中设置 window.__LICENSE_GATE_BYPASS__ = false）即可。
 */

describe('v0.1.37 许可检查屏蔽（LICENSE_GATE_BYPASS）', () => {
    let mockInvoke: jest.Mock;

    beforeEach(() => {
        mockInvoke = jest.fn();
        (window as any).__TAURI__ = { core: { invoke: mockInvoke } };
        document.body.innerHTML = `
            <div id="license-gate"></div>
            <div id="view-main" class="hidden"></div>
            <div id="license-status-badge" class="hidden"></div>
            <button id="license-purchase-btn"></button>
            <button id="license-restore-btn"></button>
        `;
        jest.resetModules();
        // 明确不设置 __LICENSE_GATE_BYPASS__，走 license.js 默认值（true）
        delete (window as any).__LICENSE_GATE_BYPASS__;
    });

    afterEach(() => {
        delete (window as any).__TAURI__;
        delete (window as any).LicenseModule;
    });

    test('checkLicenseStatus 默认屏蔽：不调用 invoke，直接返回 licensed=true', async () => {
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();

        expect(result.licensed).toBe(true);
        expect(result.status).toBe('licensed');
        expect(mockInvoke).not.toHaveBeenCalled();
    });

    test('initLicense 默认屏蔽：不调用任何 SDK 命令，license-gate 被隐藏，view-main 可见', async () => {
        require('../src/license.js');

        await (window as any).LicenseModule.initLicense();

        const gate = document.getElementById('license-gate');
        const main = document.getElementById('view-main');
        expect(gate?.classList.contains('hidden')).toBe(true);
        expect(main?.classList.contains('hidden')).toBe(false);
        // 关键断言：屏蔽期间绝不调用后端 get_trial_status
        expect(mockInvoke).not.toHaveBeenCalledWith('get_trial_status');
    });

    test('window.__LICENSE_GATE_BYPASS__ = false 时走真实 SDK 路径（向后兼容）', async () => {
        (window as any).__LICENSE_GATE_BYPASS__ = false;
        mockInvoke.mockResolvedValue('Purchased');
        require('../src/license.js');

        await (window as any).LicenseModule.checkLicenseStatus();

        // 关掉 bypass 后应调用 SDK
        expect(mockInvoke).toHaveBeenCalledWith('get_trial_status');

        delete (window as any).__LICENSE_GATE_BYPASS__;
    });

    test('屏蔽模式下 getCurrentStatus 应返回 licensed', async () => {
        require('../src/license.js');

        await (window as any).LicenseModule.checkLicenseStatus();

        expect((window as any).LicenseModule.getCurrentStatus()).toBe('licensed');
    });
});

describe('license.js 源码包含屏蔽开关', () => {
    const fs = require('fs');
    const path = require('path');
    const src = fs.readFileSync(path.join(__dirname, '../src/license.js'), 'utf-8');

    test('license.js 应定义 LICENSE_GATE_BYPASS 常量', () => {
        expect(src).toMatch(/LICENSE_GATE_BYPASS\s*=\s*true/);
    });

    test('license.js 应支持 window.__LICENSE_GATE_BYPASS__ 测试覆盖', () => {
        expect(src).toContain('__LICENSE_GATE_BYPASS__');
    });
});
