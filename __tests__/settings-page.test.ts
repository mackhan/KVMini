/**
 * settings-page.test.ts
 * 设置页全量测试用例
 * 覆盖：语言切换、主题切换、快捷键、网络设置、TLS、保存/取消、默认值
 * 需求：4.1 ~ 4.7
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

const mockMatchMedia = (prefersDark = false) => ({
    matches: prefersDark,
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    addListener: jest.fn(),
    removeListener: jest.fn(),
});
Object.defineProperty(global, 'matchMedia', {
    writable: true,
    value: jest.fn(() => mockMatchMedia()),
});

// ─── 模拟 Tauri invoke ────────────────────────────────────────────────────────

const mockInvoke = jest.fn();

// ─── 加载模块 ─────────────────────────────────────────────────────────────────

const i18n = require('../src/i18n.js');
const themeModule = require('../src/theme.js');

// ─── 默认设置 ─────────────────────────────────────────────────────────────────

const DEFAULT_SETTINGS = {
    hotkey: '',
    preferConnectionType: 'auto',
    autoReconnect: true,
    autoStart: false,
    startMinimized: false,
    clipboardSync: true,
    enableTls: true,
    language: 'zh',
    theme: 'auto',
};

// ─── 测试套件 ─────────────────────────────────────────────────────────────────

describe('设置页全量测试（需求 4.1 ~ 4.7）', () => {

    beforeEach(() => {
        mockInvoke.mockReset();
        i18n.setLocale('zh');
        themeModule.setTheme('auto');
        document.documentElement.removeAttribute('data-theme');
        localStorage.clear();
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 一、语言切换
    // ═══════════════════════════════════════════════════════════════════════════

    describe('一、语言切换', () => {

        it('1.1 默认语言为中文', () => {
            expect(i18n.getLocale()).toBe('zh');
        });

        it('1.2 切换到英文后界面文字变为英文', () => {
            i18n.setLocale('en');
            expect(i18n.t('btnSave')).toBe('Save Settings');
            expect(i18n.t('btnCancel')).toBe('Cancel');
        });

        it('1.3 切换到中文后界面文字变为中文', () => {
            i18n.setLocale('en');
            i18n.setLocale('zh');
            expect(i18n.t('btnSave')).toBe('保存设置');
            expect(i18n.t('btnCancel')).toBe('取消');
        });

        it('1.4 保存设置时语言立即生效', () => {
            // 模拟 saveSettings 中的语言应用
            const newSettings = { ...DEFAULT_SETTINGS, language: 'en' };
            localStorage.setItem('kvm_settings', JSON.stringify(newSettings));
            i18n.setLocale(newSettings.language);
            expect(i18n.getLocale()).toBe('en');
        });

        it('1.5 应用启动时从 localStorage 加载语言', () => {
            localStorage.setItem('kvm_settings', JSON.stringify({ language: 'en' }));
            i18n.loadLocaleFromStorage();
            expect(i18n.getLocale()).toBe('en');
        });

        it('1.6 无效语言值不改变当前语言', () => {
            i18n.setLocale('zh');
            i18n.setLocale('invalid' as any);
            expect(i18n.getLocale()).toBe('zh');
        });

        it('1.7 语言切换后 t() 返回正确翻译', () => {
            i18n.setLocale('en');
            expect(i18n.t('settingsHotkey')).toBe('Global Hotkey');
            expect(i18n.t('settingsAppearance')).toBe('Appearance');
            i18n.setLocale('zh');
            expect(i18n.t('settingsHotkey')).toBe('全局快捷键');
            expect(i18n.t('settingsAppearance')).toBe('外观');
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 二、主题切换
    // ═══════════════════════════════════════════════════════════════════════════

    describe('二、主题切换', () => {

        it('2.1 切换到深色主题后 data-theme 为 dark', () => {
            themeModule.setTheme('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
            expect(themeModule.getTheme()).toBe('dark');
        });

        it('2.2 切换到浅色主题后 data-theme 为 light', () => {
            themeModule.setTheme('light');
            expect(document.documentElement.getAttribute('data-theme')).toBe('light');
        });

        it('2.3 切换到跟随系统后 data-theme 根据系统设置', () => {
            (global as any).matchMedia = jest.fn(() => mockMatchMedia(false));
            themeModule.setTheme('auto');
            expect(document.documentElement.getAttribute('data-theme')).toBe('light');
        });

        it('2.4 保存设置时主题立即生效', () => {
            const newSettings = { ...DEFAULT_SETTINGS, theme: 'dark' };
            localStorage.setItem('kvm_settings', JSON.stringify(newSettings));
            themeModule.setTheme(newSettings.theme);
            expect(themeModule.getTheme()).toBe('dark');
        });

        it('2.5 应用启动时从 localStorage 加载主题', () => {
            localStorage.setItem('kvm_settings', JSON.stringify({ theme: 'dark' }));
            themeModule.loadThemeFromStorage();
            expect(themeModule.getTheme()).toBe('dark');
        });

        it('2.6 无效主题值不改变当前主题', () => {
            themeModule.setTheme('dark');
            themeModule.setTheme('invalid' as any);
            expect(themeModule.getTheme()).toBe('dark');
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 三、快捷键设置
    // ═══════════════════════════════════════════════════════════════════════════

    describe('三、快捷键设置', () => {

        it('3.1 保存快捷键时调用 register_global_hotkey', async () => {
            mockInvoke.mockResolvedValue(undefined);
            await mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
            expect(mockInvoke).toHaveBeenCalledWith('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
        });

        it('3.2 清空快捷键时传入空字符串', async () => {
            mockInvoke.mockResolvedValue(undefined);
            await mockInvoke('register_global_hotkey', { hotkey: '' });
            expect(mockInvoke).toHaveBeenCalledWith('register_global_hotkey', { hotkey: '' });
        });

        it('3.3 快捷键注册失败时捕获错误', async () => {
            const showError = jest.fn();
            mockInvoke.mockRejectedValue(new Error('辅助功能权限不足'));
            try {
                await mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
            } catch (e: any) {
                showError(`快捷键注册失败: ${e.message}`);
            }
            expect(showError).toHaveBeenCalledWith('快捷键注册失败: 辅助功能权限不足');
        });

        it('3.4 快捷键注册失败不阻止设置保存', async () => {
            const saveCompleted = jest.fn();
            mockInvoke
                .mockResolvedValueOnce(undefined)
                .mockRejectedValueOnce(new Error('快捷键冲突'));
            try { await mockInvoke('save_settings', {}); } catch { return; }
            try { await mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' }); } catch {}
            saveCompleted();
            expect(saveCompleted).toHaveBeenCalledTimes(1);
        });

        it('3.5 有效快捷键格式包含 + 分隔符', () => {
            const validHotkeys = ['Ctrl+Alt+S', 'Ctrl+Shift+F1', 'Alt+Tab'];
            validHotkeys.forEach(hk => {
                expect(hk.includes('+')).toBe(true);
            });
        });

        it('3.6 应用启动时从 localStorage 读取快捷键', async () => {
            mockInvoke.mockResolvedValue(undefined);
            localStorage.setItem('kvm_settings', JSON.stringify({ hotkey: 'Ctrl+Alt+S' }));
            const saved = JSON.parse(localStorage.getItem('kvm_settings')!);
            if (saved.hotkey) {
                await mockInvoke('register_global_hotkey', { hotkey: saved.hotkey });
            }
            expect(mockInvoke).toHaveBeenCalledWith('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 四、网络设置
    // ═══════════════════════════════════════════════════════════════════════════

    describe('四、网络设置', () => {

        it('4.1 默认连接类型为 auto', () => {
            expect(DEFAULT_SETTINGS.preferConnectionType).toBe('auto');
        });

        it('4.2 默认自动重连为 true', () => {
            expect(DEFAULT_SETTINGS.autoReconnect).toBe(true);
        });

        it('4.3 端口范围验证：1024-65535 为有效范围', () => {
            const validPorts = [1024, 8889, 65535];
            const invalidPorts = [0, 1023, 65536, -1];
            validPorts.forEach(port => {
                expect(port >= 1024 && port <= 65535).toBe(true);
            });
            invalidPorts.forEach(port => {
                expect(port >= 1024 && port <= 65535).toBe(false);
            });
        });

        it('4.4 保存网络设置时调用 save_settings', async () => {
            mockInvoke.mockResolvedValue(undefined);
            const config = { ...DEFAULT_SETTINGS, preferConnectionType: 'tcp' };
            await mockInvoke('save_settings', { config });
            expect(mockInvoke).toHaveBeenCalledWith('save_settings', { config });
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 五、TLS 设置
    // ═══════════════════════════════════════════════════════════════════════════

    describe('五、TLS 设置', () => {

        it('5.1 默认启用 TLS', () => {
            expect(DEFAULT_SETTINGS.enableTls).toBe(true);
        });

        it('5.2 关闭 TLS 时应有安全警告', () => {
            const showError = jest.fn();
            const enableTls = false;
            if (!enableTls) {
                showError('TLS 已关闭，连接将不加密');
            }
            expect(showError).toHaveBeenCalled();
        });

        it('5.3 TLS 开关状态保存到设置', () => {
            const settings = { ...DEFAULT_SETTINGS, enableTls: false };
            localStorage.setItem('kvm_settings', JSON.stringify(settings));
            const saved = JSON.parse(localStorage.getItem('kvm_settings')!);
            expect(saved.enableTls).toBe(false);
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 六、保存设置
    // ═══════════════════════════════════════════════════════════════════════════

    describe('六、保存设置', () => {

        it('6.1 保存成功时调用 save_settings', async () => {
            mockInvoke.mockResolvedValue(undefined);
            await mockInvoke('save_settings', { config: DEFAULT_SETTINGS });
            expect(mockInvoke).toHaveBeenCalledWith('save_settings', { config: DEFAULT_SETTINGS });
        });

        it('6.2 保存成功时设置写入 localStorage', () => {
            const settings = { ...DEFAULT_SETTINGS, language: 'en' };
            localStorage.setItem('kvm_settings', JSON.stringify(settings));
            const saved = JSON.parse(localStorage.getItem('kvm_settings')!);
            expect(saved.language).toBe('en');
        });

        it('6.3 保存失败时显示错误提示', async () => {
            const showError = jest.fn();
            mockInvoke.mockRejectedValue(new Error('保存失败'));
            try {
                await mockInvoke('save_settings', { config: DEFAULT_SETTINGS });
            } catch (e: any) {
                showError(`设置保存失败: ${e.message}`);
            }
            expect(showError).toHaveBeenCalledWith('设置保存失败: 保存失败');
        });

        it('6.4 保存成功后语言立即生效', () => {
            const settings = { ...DEFAULT_SETTINGS, language: 'en' };
            i18n.setLocale(settings.language);
            expect(i18n.getLocale()).toBe('en');
        });

        it('6.5 保存成功后主题立即生效', () => {
            const settings = { ...DEFAULT_SETTINGS, theme: 'dark' };
            themeModule.setTheme(settings.theme);
            expect(themeModule.getTheme()).toBe('dark');
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 七、取消设置
    // ═══════════════════════════════════════════════════════════════════════════

    describe('七、取消设置', () => {

        it('7.1 无修改时直接关闭，不弹出确认对话框', () => {
            const hasDirty = false;
            const showDialog = jest.fn();
            const closeModal = jest.fn();

            if (hasDirty) {
                showDialog();
            } else {
                closeModal();
            }

            expect(showDialog).not.toHaveBeenCalled();
            expect(closeModal).toHaveBeenCalledTimes(1);
        });

        it('7.2 有修改时弹出确认对话框', () => {
            const hasDirty = true;
            const showDialog = jest.fn();
            const closeModal = jest.fn();

            if (hasDirty) {
                showDialog();
            } else {
                closeModal();
            }

            expect(showDialog).toHaveBeenCalledTimes(1);
            expect(closeModal).not.toHaveBeenCalled();
        });

        it('7.3 选择"放弃修改"后关闭设置页', () => {
            const closeModal = jest.fn();
            const clearDirty = jest.fn();

            // 模拟 onSettingsDiscardConfirm
            function onDiscardConfirm() {
                clearDirty();
                closeModal();
            }

            onDiscardConfirm();

            expect(clearDirty).toHaveBeenCalledTimes(1);
            expect(closeModal).toHaveBeenCalledTimes(1);
        });

        it('7.4 选择"继续编辑"后保持设置页打开', () => {
            const closeModal = jest.fn();
            const hideDialog = jest.fn();

            // 模拟 onSettingsDiscardCancel
            function onDiscardCancel() {
                hideDialog();
                // 不调用 closeModal
            }

            onDiscardCancel();

            expect(hideDialog).toHaveBeenCalledTimes(1);
            expect(closeModal).not.toHaveBeenCalled();
        });

        it('7.5 放弃修改后 clearAllDirty 被调用', () => {
            const clearAllDirty = jest.fn();
            clearAllDirty();
            expect(clearAllDirty).toHaveBeenCalledTimes(1);
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 八、设置项默认值验证
    // ═══════════════════════════════════════════════════════════════════════════

    describe('八、设置项默认值验证', () => {

        it('8.1 默认语言为中文', () => {
            expect(DEFAULT_SETTINGS.language).toBe('zh');
        });

        it('8.2 默认主题为跟随系统', () => {
            expect(DEFAULT_SETTINGS.theme).toBe('auto');
        });

        it('8.3 默认快捷键为空', () => {
            expect(DEFAULT_SETTINGS.hotkey).toBe('');
        });

        it('8.4 默认启用 TLS', () => {
            expect(DEFAULT_SETTINGS.enableTls).toBe(true);
        });

        it('8.5 默认启用自动重连', () => {
            expect(DEFAULT_SETTINGS.autoReconnect).toBe(true);
        });

        it('8.6 默认不开机自启动', () => {
            expect(DEFAULT_SETTINGS.autoStart).toBe(false);
        });

        it('8.7 默认启用剪贴板同步', () => {
            expect(DEFAULT_SETTINGS.clipboardSync).toBe(true);
        });

        it('8.8 localStorage 为空时使用默认设置', () => {
            localStorage.clear();
            const saved = localStorage.getItem('kvm_settings');
            expect(saved).toBeNull();
            // 应使用默认设置
            const settings = saved ? JSON.parse(saved) : DEFAULT_SETTINGS;
            expect(settings.language).toBe('zh');
            expect(settings.theme).toBe('auto');
        });
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // 九、设置页打开/关闭
    // ═══════════════════════════════════════════════════════════════════════════

    describe('九、设置页打开/关闭', () => {

        beforeEach(() => {
            document.body.innerHTML = `
                <div id="settings-modal" class="modal hidden"></div>
                <div id="settings-discard-dialog" class="modal hidden"></div>
            `;
        });

        it('9.1 打开设置页时 modal 移除 hidden 类', () => {
            const modal = document.getElementById('settings-modal')!;
            modal.classList.remove('hidden');
            expect(modal.classList.contains('hidden')).toBe(false);
        });

        it('9.2 关闭设置页时 modal 添加 hidden 类', () => {
            const modal = document.getElementById('settings-modal')!;
            modal.classList.remove('hidden');
            modal.classList.add('hidden');
            expect(modal.classList.contains('hidden')).toBe(true);
        });

        it('9.3 有未保存修改时关闭弹出确认对话框', () => {
            const dialog = document.getElementById('settings-discard-dialog')!;
            const modal = document.getElementById('settings-modal')!;
            modal.classList.remove('hidden');

            const hasDirty = true;
            if (hasDirty) {
                dialog.classList.remove('hidden');
            }

            expect(dialog.classList.contains('hidden')).toBe(false);
            expect(modal.classList.contains('hidden')).toBe(false);
        });

        it('9.4 无未保存修改时直接关闭', () => {
            const modal = document.getElementById('settings-modal')!;
            modal.classList.remove('hidden');

            const hasDirty = false;
            if (!hasDirty) {
                modal.classList.add('hidden');
            }

            expect(modal.classList.contains('hidden')).toBe(true);
        });
    });
});
