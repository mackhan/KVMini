/**
 * settings-module.test.ts
 * 测试 src/settings.js 中的设置管理函数
 */

beforeAll(() => {
    (window as any).i18n = { t: (key: string) => key };
    require('../src/settings.js');
});

beforeEach(() => {
    localStorage.clear();
});

describe('settings.js - loadSettingsFromStorage', () => {
    test('空存储时应返回默认值', () => {
        const settings = (window as any).SettingsModule.loadSettingsFromStorage();
        expect(settings).toBeDefined();
        expect(settings.autoReconnect).toBe(true);
        expect(settings.language).toBe('zh');
        expect(settings.theme).toBe('auto');
    });

    test('存储损坏时应返回默认值', () => {
        localStorage.setItem('kvm_settings', 'bad-json');
        const settings = (window as any).SettingsModule.loadSettingsFromStorage();
        expect(settings.autoReconnect).toBe(true);
    });

    test('有数据时应合并默认值', () => {
        localStorage.setItem('kvm_settings', JSON.stringify({ hotkey: 'Ctrl+Alt+K' }));
        const settings = (window as any).SettingsModule.loadSettingsFromStorage();
        expect(settings.hotkey).toBe('Ctrl+Alt+K');
        expect(settings.autoReconnect).toBe(true); // 默认值保留
    });
});

describe('settings.js - saveSettingsToStorage', () => {
    test('应将设置持久化到 localStorage', () => {
        const settings = { hotkey: 'Ctrl+Alt+K', autoReconnect: false, language: 'en', theme: 'dark' };
        (window as any).SettingsModule.saveSettingsToStorage(settings);
        const loaded = (window as any).SettingsModule.loadSettingsFromStorage();
        expect(loaded.hotkey).toBe('Ctrl+Alt+K');
        expect(loaded.autoReconnect).toBe(false);
        expect(loaded.language).toBe('en');
    });
});

describe('settings.js - DEFAULT_SETTINGS', () => {
    test('默认快捷键应为 Ctrl+Alt+S', () => {
        const defaults = (window as any).SettingsModule.DEFAULT_SETTINGS;
        expect(defaults.hotkey).toBe('Ctrl+Alt+S');
    });

    test('默认自动重连应为 true', () => {
        const defaults = (window as any).SettingsModule.DEFAULT_SETTINGS;
        expect(defaults.autoReconnect).toBe(true);
    });

    test('默认语言应为 zh', () => {
        const defaults = (window as any).SettingsModule.DEFAULT_SETTINGS;
        expect(defaults.language).toBe('zh');
    });
});
