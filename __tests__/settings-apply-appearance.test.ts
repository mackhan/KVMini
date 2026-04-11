/**
 * settings-apply-appearance.test.ts
 * TDD 测试：保存设置后语言/主题立即生效
 * 需求：1.1 ~ 1.7（语言切换）、1.3 ~ 1.5（主题切换）
 */

// ─── 模拟 localStorage ────────────────────────────────────────────────────────

const localStorageMock = (() => {
    let store: Record<string, string> = {};
    return {
        getItem: (key: string) => store[key] ?? null,
        setItem: (key: string, value: string) => { store[key] = value; },
        removeItem: (key: string) => { delete store[key]; },
        clear: () => { store = {}; },
    };
})();
Object.defineProperty(global, 'localStorage', { value: localStorageMock });

// ─── 模拟 matchMedia ──────────────────────────────────────────────────────────

const mockMatchMedia = (prefersDark: boolean) => ({
    matches: prefersDark,
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    addListener: jest.fn(),
    removeListener: jest.fn(),
});
Object.defineProperty(global, 'matchMedia', {
    writable: true,
    value: jest.fn(() => mockMatchMedia(false)),
});

// ─── 加载模块 ─────────────────────────────────────────────────────────────────

const i18n = require('../src/i18n.js');
const themeModule = require('../src/theme.js');

describe('保存设置后语言/主题立即生效', () => {

    beforeEach(() => {
        // 重置语言为中文
        i18n.setLocale('zh');
        // 重置主题为 auto
        themeModule.setTheme('auto');
        // 重置 html data-theme
        document.documentElement.removeAttribute('data-theme');
        localStorage.clear();
    });

    // ─── 语言切换 ─────────────────────────────────────────────────────────────

    describe('语言切换（需求 1.1 ~ 1.2）', () => {

        it('1.1 保存设置时语言为 en，i18n.setLocale 应被切换为 en', () => {
            // 模拟保存设置时调用语言切换
            const newLang = 'en';
            i18n.setLocale(newLang);
            expect(i18n.getLocale()).toBe('en');
        });

        it('1.2 保存设置时语言为 zh，i18n.setLocale 应被切换为 zh', () => {
            i18n.setLocale('en');
            const newLang = 'zh';
            i18n.setLocale(newLang);
            expect(i18n.getLocale()).toBe('zh');
        });

        it('1.3 切换到英文后 t("btnSave") 返回英文', () => {
            i18n.setLocale('en');
            expect(i18n.t('btnSave')).toBe('Save Settings');
        });

        it('1.4 切换到中文后 t("btnSave") 返回中文', () => {
            i18n.setLocale('zh');
            expect(i18n.t('btnSave')).toBe('保存设置');
        });

        it('1.5 切换到英文后 t("btnCancel") 返回英文', () => {
            i18n.setLocale('en');
            expect(i18n.t('btnCancel')).toBe('Cancel');
        });

        it('1.6 应用启动时从 localStorage 加载语言', () => {
            localStorage.setItem('kvm_settings', JSON.stringify({ language: 'en' }));
            i18n.loadLocaleFromStorage();
            expect(i18n.getLocale()).toBe('en');
        });

        it('1.7 语言切换失败（无效值）时保持当前语言不变', () => {
            i18n.setLocale('zh');
            i18n.setLocale('invalid' as any);
            expect(i18n.getLocale()).toBe('zh');
        });
    });

    // ─── 主题切换 ─────────────────────────────────────────────────────────────

    describe('主题切换（需求 1.3 ~ 1.5）', () => {

        it('2.1 保存设置时主题为 dark，data-theme 应立即变为 dark', () => {
            themeModule.setTheme('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
            expect(themeModule.getTheme()).toBe('dark');
        });

        it('2.2 保存设置时主题为 light，data-theme 应立即变为 light', () => {
            themeModule.setTheme('light');
            expect(document.documentElement.getAttribute('data-theme')).toBe('light');
            expect(themeModule.getTheme()).toBe('light');
        });

        it('2.3 保存设置时主题为 auto，data-theme 应根据系统主题设置', () => {
            (global as any).matchMedia = jest.fn(() => mockMatchMedia(false));
            themeModule.setTheme('auto');
            expect(document.documentElement.getAttribute('data-theme')).toBe('light');
            expect(themeModule.getTheme()).toBe('auto');
        });

        it('2.4 保存设置时主题为 auto 且系统为深色，data-theme 应为 dark', () => {
            (global as any).matchMedia = jest.fn(() => mockMatchMedia(true));
            themeModule.setTheme('auto');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
        });

        it('2.5 应用启动时从 localStorage 加载主题', () => {
            localStorage.setItem('kvm_settings', JSON.stringify({ theme: 'dark' }));
            themeModule.loadThemeFromStorage();
            expect(themeModule.getTheme()).toBe('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
        });

        it('2.6 主题切换失败（无效值）时保持当前主题不变', () => {
            themeModule.setTheme('dark');
            themeModule.setTheme('invalid' as any);
            expect(themeModule.getTheme()).toBe('dark');
        });
    });

    // ─── 保存设置后立即应用（集成验证）────────────────────────────────────────

    describe('保存设置后立即应用（集成验证）', () => {

        it('3.1 模拟 saveSettings 流程：保存语言 en 后 i18n 立即生效', () => {
            // 模拟 saveSettings 中的语言应用逻辑
            const newSettings = { language: 'en', theme: 'auto' };
            localStorage.setItem('kvm_settings', JSON.stringify(newSettings));

            // 应用语言（这是 saveSettings 应该调用的）
            i18n.setLocale(newSettings.language);

            expect(i18n.getLocale()).toBe('en');
            expect(i18n.t('btnSave')).toBe('Save Settings');
        });

        it('3.2 模拟 saveSettings 流程：保存主题 dark 后 themeManager 立即生效', () => {
            const newSettings = { language: 'zh', theme: 'dark' };
            localStorage.setItem('kvm_settings', JSON.stringify(newSettings));

            // 应用主题（这是 saveSettings 应该调用的）
            themeModule.setTheme(newSettings.theme);

            expect(themeModule.getTheme()).toBe('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
        });

        it('3.3 模拟 initAppearance 流程：启动时从 localStorage 加载并应用语言和主题', () => {
            localStorage.setItem('kvm_settings', JSON.stringify({ language: 'en', theme: 'dark' }));

            // 模拟 initAppearance 逻辑
            const saved = JSON.parse(localStorage.getItem('kvm_settings')!);
            i18n.setLocale(saved.language);
            themeModule.setTheme(saved.theme);

            expect(i18n.getLocale()).toBe('en');
            expect(themeModule.getTheme()).toBe('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
        });
    });
});
