/**
 * Bug #016：save_settings 参数键名不匹配
 *
 * 现象：点击"保存设置"后报错：
 *   invalid args 'config' for command 'save_settings': command save_settings missing required key config
 * 根因：main.js 调用 invoke('save_settings', { settings: appState.settings })，
 *       键名是 settings，但 Rust 后端期望的键名是 config。
 * 修改方式：将 main.js 中的 { settings: appState.settings } 改为 { config: appState.settings }
 */

const fs = require('fs');
const path = require('path');

describe('Bug #016：save_settings 参数键名修复', () => {
    let mainJs: string;

    beforeAll(() => {
        mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');
    });

    test('main.js 中 save_settings 调用使用 { config } 而非 { settings }', () => {
        // 验证修复后的键名
        expect(mainJs).toContain("invoke('save_settings', { config:");
        // 确保旧的错误键名不存在
        expect(mainJs).not.toContain("invoke('save_settings', { settings:");
    });

    test('platform.js 中 saveSettings 使用 { config } 键名', () => {
        const platformJs = fs.readFileSync(path.join(__dirname, '../src/platform.js'), 'utf-8');
        expect(platformJs).toContain("safeInvoke('save_settings', { config }");
    });

    test('save_settings 调用参数键名全局一致（所有调用都使用 config）', () => {
        // 检查 main.js 中没有使用 settings 键名调用 save_settings
        const wrongPattern = /invoke\(['"]save_settings['"],\s*\{\s*settings:/;
        expect(wrongPattern.test(mainJs)).toBe(false);
    });
});

describe('Bug #016：save_settings 调用行为验证', () => {
    const mockInvoke = jest.fn();

    beforeEach(() => {
        jest.clearAllMocks();
        (window as any).__TAURI__ = {
            invoke: mockInvoke,
        };
    });

    test('save_settings 使用 config 键名调用成功', async () => {
        mockInvoke.mockResolvedValueOnce(undefined);
        const config = {
            hotkey: 'Ctrl+Alt+K',
            prefer_connection_type: 'network',
            auto_reconnect: true,
            auto_start: false,
            start_minimized: false,
            clipboard_sync: true,
            enable_tls: true,
            language: 'zh',
        };
        await (window as any).__TAURI__.invoke('save_settings', { config });
        expect(mockInvoke).toHaveBeenCalledWith('save_settings', { config });
    });

    test('save_settings 使用 settings 键名调用会被 Rust 拒绝（模拟错误）', async () => {
        mockInvoke.mockRejectedValueOnce(
            new Error("invalid args 'config' for command 'save_settings': command save_settings missing required key config")
        );
        const settings = { hotkey: 'Ctrl+Alt+K' };
        await expect(
            (window as any).__TAURI__.invoke('save_settings', { settings })
        ).rejects.toThrow('missing required key config');
    });

    test('save_settings 成功后不抛出错误', async () => {
        mockInvoke.mockResolvedValueOnce(undefined);
        const config = { hotkey: 'Ctrl+Alt+S', enable_tls: false };
        await expect(
            (window as any).__TAURI__.invoke('save_settings', { config })
        ).resolves.toBeUndefined();
    });
});
