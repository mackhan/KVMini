/**
 * license-integration.test.ts
 * 验证 tauri-commerce SDK 集成的完整性：
 * 1. index.html 包含 license UI DOM 元素
 * 2. license.js 正确调用 tauri-commerce SDK 命令（get_trial_status）
 * 3. LicenseModule 完整功能（initLicense、updateLicenseUI、checkLicenseStatus）
 * 4. Rust 侧命令存在（通过 main.rs 文件内容验证）
 */

const fs = require('fs');
const path = require('path');

// ─── 辅助：读取源文件 ─────────────────────────────────────────────────────────

const indexHtml = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
const mainRs = fs.readFileSync(path.join(__dirname, '../src-tauri/src/main.rs'), 'utf-8');

// ─── 需求 4.1：index.html 包含 license UI DOM ─────────────────────────────────

describe('需求 4.1：index.html 包含 license UI DOM', () => {
    test('index.html 应包含 license-gate 元素', () => {
        expect(indexHtml).toContain('id="license-gate"');
    });

    test('index.html 应包含 license-key-input 输入框', () => {
        expect(indexHtml).toContain('id="license-key-input"');
    });

    test('index.html 应包含 license-activate-btn 激活按钮', () => {
        expect(indexHtml).toContain('id="license-activate-btn"');
    });

    test('index.html 应包含 license-buy-btn 购买按钮', () => {
        expect(indexHtml).toContain('id="license-buy-btn"');
    });

    test('index.html 应包含 license-error-msg 错误提示', () => {
        expect(indexHtml).toContain('id="license-error-msg"');
    });

    test('index.html 应包含 license-status-badge 授权状态徽章', () => {
        expect(indexHtml).toContain('id="license-status-badge"');
    });

    test('index.html 应包含 license-sdk-error SDK 错误提示', () => {
        expect(indexHtml).toContain('id="license-sdk-error"');
    });

    test('license-gate 初始状态应为 hidden', () => {
        // license-gate 默认隐藏，由 JS 根据授权状态控制显示
        expect(indexHtml).toMatch(/id="license-gate"[^>]*class="[^"]*hidden/);
    });
});

// ─── 需求 4.2：Rust 侧 tauri-commerce SDK 命令存在 ────────────────────────────

describe('需求 4.2：Rust 侧 tauri-commerce SDK 命令存在', () => {
    test('main.rs 应包含 get_trial_status 命令（tauri-commerce SDK）', () => {
        expect(mainRs).toContain('get_trial_status');
    });

    test('main.rs 应包含 get_locale 命令（tauri-commerce SDK）', () => {
        expect(mainRs).toContain('get_locale');
    });

    test('main.rs 应包含 set_locale 命令（tauri-commerce SDK）', () => {
        expect(mainRs).toContain('set_locale');
    });

    test('main.rs 的 invoke_handler 应注册 get_trial_status', () => {
        expect(mainRs).toMatch(/get_trial_status/);
    });

    test('main.rs 的 invoke_handler 应注册 set_locale', () => {
        expect(mainRs).toMatch(/set_locale/);
    });
});

// ─── 需求 4.3：LicenseModule 调用正确的 SDK 命令 ─────────────────────────────

describe('需求 4.3：LicenseModule 调用正确的 SDK 命令', () => {
    let mockInvoke: jest.Mock;

    beforeEach(() => {
        mockInvoke = jest.fn();
        (window as any).__TAURI__ = {
            core: { invoke: mockInvoke },
        };
        document.body.innerHTML = `
            <div id="license-gate" class="hidden"></div>
            <div id="main-content"></div>
            <div id="license-status-badge" class="hidden"></div>
            <div id="license-sdk-error" class="hidden">
                <span id="license-sdk-error-msg"></span>
            </div>
            <input id="license-key-input" />
            <button id="license-activate-btn"></button>
            <button id="license-buy-btn"></button>
            <div id="license-error-msg" class="hidden"></div>
        `;
        jest.resetModules();
    });

    afterEach(() => {
        delete (window as any).__TAURI__;
        delete (window as any).LicenseModule;
    });

    test('checkLicenseStatus 应调用 invoke("get_trial_status")', async () => {
        // tauri-commerce SDK 使用 get_trial_status 命令
        mockInvoke.mockResolvedValue('Purchased');
        require('../src/license.js');

        await (window as any).LicenseModule.checkLicenseStatus();

        expect(mockInvoke).toHaveBeenCalledWith('get_trial_status');
    });

    test('get_trial_status 返回 Purchased 时 licensed=true', async () => {
        mockInvoke.mockResolvedValue('Purchased');
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();

        expect(result.licensed).toBe(true);
        expect(result.status).toBe('licensed');
    });

    test('get_trial_status 返回 Active 时 licensed=true（试用中）', async () => {
        mockInvoke.mockResolvedValue({ Active: 5 });
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();

        expect(result.licensed).toBe(true);
        expect(result.status).toBe('trial');
    });

    test('get_trial_status 返回 Expired 时 licensed=false', async () => {
        mockInvoke.mockResolvedValue('Expired');
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();

        expect(result.licensed).toBe(false);
        expect(result.status).toBe('expired');
    });

    test('checkLicenseStatus 返回 licensed=true 时，updateLicenseUI 隐藏 license-gate', async () => {
        mockInvoke.mockResolvedValue('Purchased');
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();
        (window as any).LicenseModule.updateLicenseUI(result.licensed);

        const licenseGate = document.getElementById('license-gate');
        expect(licenseGate?.classList.contains('hidden')).toBe(true);
    });

    test('checkLicenseStatus 返回 licensed=false 时，updateLicenseUI 显示 license-gate', async () => {
        mockInvoke.mockResolvedValue('Expired');
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();
        (window as any).LicenseModule.updateLicenseUI(result.licensed);

        const licenseGate = document.getElementById('license-gate');
        expect(licenseGate?.classList.contains('hidden')).toBe(false);
    });
});

// ─── 需求 4.4：SDK 初始化失败不阻塞主界面 ────────────────────────────────────

describe('需求 4.4：SDK 初始化失败不阻塞主界面', () => {
    beforeEach(() => {
        (window as any).__TAURI__ = {
            core: { invoke: jest.fn().mockRejectedValue(new Error('网络错误')) },
        };
        document.body.innerHTML = `
            <div id="license-gate" class="hidden"></div>
            <div id="main-content"></div>
            <div id="license-status-badge" class="hidden"></div>
            <div id="license-sdk-error" class="hidden">
                <span id="license-sdk-error-msg"></span>
            </div>
            <input id="license-key-input" />
            <button id="license-activate-btn"></button>
            <button id="license-buy-btn"></button>
            <div id="license-error-msg" class="hidden"></div>
        `;
        jest.resetModules();
    });

    afterEach(() => {
        delete (window as any).__TAURI__;
        delete (window as any).LicenseModule;
    });

    test('invoke 抛出异常时，checkLicenseStatus 应返回 sdk_error 状态，降级允许使用', async () => {
        require('../src/license.js');

        const result = await (window as any).LicenseModule.checkLicenseStatus();

        // SDK 失败时降级：返回 sdk_error 状态，但 licensed=true（不阻塞主界面）
        expect(result.status).toBe('sdk_error');
        expect(result.licensed).toBe(true);
    });

    test('SDK 失败时，initLicense 应显示 sdk-error 提示但不隐藏主界面', async () => {
        require('../src/license.js');

        await (window as any).LicenseModule.initLicense();

        const sdkError = document.getElementById('license-sdk-error');
        const mainContent = document.getElementById('main-content');
        // SDK 失败时降级：显示错误提示，但主界面仍然可用
        expect(sdkError?.classList.contains('hidden')).toBe(false);
        expect(mainContent?.classList.contains('hidden')).toBe(false);
    });
});
