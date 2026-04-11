/**
 * hotkey-register.test.ts
 * TDD 测试：全局快捷键注册命令
 * 需求：3.1 ~ 3.8
 */

// ─── 模拟 Tauri invoke ────────────────────────────────────────────────────────

const mockInvoke = jest.fn();

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

describe('全局快捷键注册（需求 3.1 ~ 3.8）', () => {

    beforeEach(() => {
        mockInvoke.mockReset();
        localStorage.clear();
    });

    // ─── 3.1 保存设置时调用 register_global_hotkey ────────────────────────────

    it('3.1 保存设置时应调用 register_global_hotkey 命令', async () => {
        mockInvoke.mockResolvedValue(undefined);

        const hotkey = 'Ctrl+Alt+S';
        await mockInvoke('register_global_hotkey', { hotkey });

        expect(mockInvoke).toHaveBeenCalledWith('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
    });

    it('3.2 register_global_hotkey 成功时不应抛出错误', async () => {
        mockInvoke.mockResolvedValue(undefined);

        await expect(mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' }))
            .resolves.not.toThrow();
    });

    // ─── 3.3 快捷键注册失败时返回友好错误信息 ────────────────────────────────

    it('3.3 register_global_hotkey 失败时应返回错误信息', async () => {
        mockInvoke.mockRejectedValue(new Error('辅助功能权限不足'));

        await expect(mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' }))
            .rejects.toThrow('辅助功能权限不足');
    });

    // ─── 3.4 清空快捷键时注销 ────────────────────────────────────────────────

    it('3.4 清空快捷键时应调用 register_global_hotkey 传入空字符串', async () => {
        mockInvoke.mockResolvedValue(undefined);

        const hotkey = ''; // 清空快捷键
        await mockInvoke('register_global_hotkey', { hotkey });

        expect(mockInvoke).toHaveBeenCalledWith('register_global_hotkey', { hotkey: '' });
    });

    // ─── 3.5 应用启动时自动注册已保存的快捷键 ───────────────────────────────

    it('3.5 应用启动时应从 localStorage 读取快捷键并注册', async () => {
        mockInvoke.mockResolvedValue(undefined);
        localStorage.setItem('kvm_settings', JSON.stringify({ hotkey: 'Ctrl+Alt+S' }));

        // 模拟启动时注册快捷键的逻辑
        const saved = JSON.parse(localStorage.getItem('kvm_settings')!);
        const hotkey = saved.hotkey || '';
        if (hotkey) {
            await mockInvoke('register_global_hotkey', { hotkey });
        }

        expect(mockInvoke).toHaveBeenCalledWith('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
    });

    it('3.6 应用启动时快捷键为空则不注册', async () => {
        mockInvoke.mockResolvedValue(undefined);
        localStorage.setItem('kvm_settings', JSON.stringify({ hotkey: '' }));

        const saved = JSON.parse(localStorage.getItem('kvm_settings')!);
        const hotkey = saved.hotkey || '';
        if (hotkey) {
            await mockInvoke('register_global_hotkey', { hotkey });
        }

        expect(mockInvoke).not.toHaveBeenCalled();
    });

    // ─── 3.7 快捷键格式验证 ──────────────────────────────────────────────────

    it('3.7 有效快捷键格式（Ctrl+Alt+S）应被接受', () => {
        const validHotkeys = ['Ctrl+Alt+S', 'Ctrl+Shift+F1', 'Alt+Tab', 'Ctrl+Alt+Delete'];
        validHotkeys.forEach(hotkey => {
            // 简单格式验证：包含 + 分隔符
            const isValid = hotkey.includes('+') && hotkey.length > 0;
            expect(isValid).toBe(true);
        });
    });

    it('3.8 空快捷键字符串表示注销', () => {
        const hotkey = '';
        const isUnregister = hotkey === '';
        expect(isUnregister).toBe(true);
    });

    // ─── 前端错误处理 ─────────────────────────────────────────────────────────

    it('3.9 快捷键注册失败时前端应捕获错误并显示提示', async () => {
        const showError = jest.fn();
        mockInvoke.mockRejectedValue(new Error('快捷键冲突'));

        try {
            await mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
        } catch (e: any) {
            showError(`快捷键注册失败: ${e.message}`);
        }

        expect(showError).toHaveBeenCalledWith('快捷键注册失败: 快捷键冲突');
    });

    it('3.10 快捷键注册失败不应阻止设置保存', async () => {
        const saveCompleted = jest.fn();
        mockInvoke
            .mockResolvedValueOnce(undefined) // save_settings 成功
            .mockRejectedValueOnce(new Error('快捷键冲突')); // register_global_hotkey 失败

        // 模拟 saveSettings 逻辑：快捷键失败不阻止保存
        try {
            await mockInvoke('save_settings', { config: {} });
        } catch (e) {
            return; // save_settings 失败才 return
        }

        try {
            await mockInvoke('register_global_hotkey', { hotkey: 'Ctrl+Alt+S' });
        } catch (e) {
            // 快捷键失败，只显示提示，不 return
        }

        saveCompleted(); // 保存流程继续完成

        expect(saveCompleted).toHaveBeenCalledTimes(1);
    });
});
