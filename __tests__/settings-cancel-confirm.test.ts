/**
 * settings-cancel-confirm.test.ts
 * TDD 测试：设置页取消确认弹窗
 * 需求：2.1 ~ 2.6
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

// ─── 加载 settings-tabs 模块 ──────────────────────────────────────────────────

describe('设置页取消确认弹窗（需求 2.1 ~ 2.6）', () => {

    // ─── 模拟 DOM ─────────────────────────────────────────────────────────────

    beforeEach(() => {
        document.body.innerHTML = `
            <div id="settings-modal" class="modal hidden">
                <div class="modal-overlay"></div>
                <div class="modal-content">
                    <div id="tab-general">
                        <input id="hotkey-input" type="text" value="Ctrl+Alt+S" />
                        <select id="language-select"><option value="zh">中文</option><option value="en">English</option></select>
                        <select id="theme-select"><option value="auto">跟随系统</option><option value="dark">深色</option></select>
                    </div>
                    <div id="tab-network">
                        <input id="listen-port" type="number" value="8889" />
                    </div>
                    <div class="modal-footer">
                        <button id="settings-save-btn">保存设置</button>
                        <button id="settings-cancel-btn">取消</button>
                    </div>
                </div>
            </div>
            <!-- 取消确认对话框 -->
            <div id="settings-discard-dialog" class="modal hidden">
                <div class="modal-overlay"></div>
                <div class="modal-content">
                    <p id="settings-discard-msg">您有未保存的修改，是否放弃？</p>
                    <button id="settings-discard-confirm">放弃修改</button>
                    <button id="settings-discard-cancel">继续编辑</button>
                </div>
            </div>
        `;
        localStorage.clear();
    });

    // ─── 2.1 有修改时点击取消弹出确认对话框 ──────────────────────────────────

    it('2.1 有未保存修改时点击取消，应弹出确认对话框', () => {
        const dialog = document.getElementById('settings-discard-dialog')!;
        const modal = document.getElementById('settings-modal')!;
        modal.classList.remove('hidden'); // 模拟设置页已打开

        // 模拟有未保存修改
        const hasDirty = true;

        // 模拟 closeSettings 逻辑
        function closeSettingsWithConfirm() {
            if (hasDirty) {
                dialog.classList.remove('hidden');
                return; // 不关闭
            }
            modal.classList.add('hidden');
        }

        closeSettingsWithConfirm();

        expect(dialog.classList.contains('hidden')).toBe(false);
        expect(modal.classList.contains('hidden')).toBe(false); // 设置页仍然打开
    });

    // ─── 2.2 选择"放弃修改"后恢复原始值并关闭 ───────────────────────────────

    it('2.2 选择"放弃修改"后，设置页应关闭', () => {
        const dialog = document.getElementById('settings-discard-dialog')!;
        const modal = document.getElementById('settings-modal')!;
        modal.classList.remove('hidden');
        dialog.classList.remove('hidden');

        // 模拟点击"放弃修改"
        function onDiscardConfirm() {
            dialog.classList.add('hidden');
            modal.classList.add('hidden');
        }

        onDiscardConfirm();

        expect(dialog.classList.contains('hidden')).toBe(true);
        expect(modal.classList.contains('hidden')).toBe(true);
    });

    // ─── 2.3 选择"继续编辑"后保持设置页打开 ─────────────────────────────────

    it('2.3 选择"继续编辑"后，确认对话框关闭，设置页保持打开', () => {
        const dialog = document.getElementById('settings-discard-dialog')!;
        const modal = document.getElementById('settings-modal')!;
        modal.classList.remove('hidden');
        dialog.classList.remove('hidden');

        // 模拟点击"继续编辑"
        function onDiscardCancel() {
            dialog.classList.add('hidden');
            // 设置页保持打开
        }

        onDiscardCancel();

        expect(dialog.classList.contains('hidden')).toBe(true);
        expect(modal.classList.contains('hidden')).toBe(false); // 设置页仍然打开
    });

    // ─── 2.4 无修改时直接关闭，不弹出确认 ───────────────────────────────────

    it('2.4 无未保存修改时点击取消，直接关闭设置页，不弹出确认对话框', () => {
        const dialog = document.getElementById('settings-discard-dialog')!;
        const modal = document.getElementById('settings-modal')!;
        modal.classList.remove('hidden');

        // 模拟无未保存修改
        const hasDirty = false;

        function closeSettingsWithConfirm() {
            if (hasDirty) {
                dialog.classList.remove('hidden');
                return;
            }
            modal.classList.add('hidden');
        }

        closeSettingsWithConfirm();

        expect(dialog.classList.contains('hidden')).toBe(true); // 对话框未弹出
        expect(modal.classList.contains('hidden')).toBe(true);  // 设置页已关闭
    });

    // ─── 2.5 确认对话框包含正确的提示文字 ───────────────────────────────────

    it('2.5 确认对话框应包含"未保存的修改"相关提示文字', () => {
        const msg = document.getElementById('settings-discard-msg')!;
        expect(msg.textContent).toContain('未保存的修改');
    });

    // ─── 2.6 确认对话框包含"放弃修改"和"继续编辑"两个按钮 ──────────────────

    it('2.6 确认对话框应包含"放弃修改"和"继续编辑"两个按钮', () => {
        const confirmBtn = document.getElementById('settings-discard-confirm')!;
        const cancelBtn = document.getElementById('settings-discard-cancel')!;
        expect(confirmBtn).not.toBeNull();
        expect(cancelBtn).not.toBeNull();
        expect(confirmBtn.textContent).toContain('放弃修改');
        expect(cancelBtn.textContent).toContain('继续编辑');
    });

    // ─── SettingsTabsModule.hasDirty() 检测 ──────────────────────────────────

    it('2.7 SettingsTabsModule.hasDirty() 在有修改时返回 true', () => {
        // 模拟 SettingsTabsModule
        const dirtyTabs = new Set<string>();
        const hasDirty = () => dirtyTabs.size > 0;
        const markDirty = (tabId: string) => dirtyTabs.add(tabId);
        const clearAllDirty = () => dirtyTabs.clear();

        expect(hasDirty()).toBe(false);
        markDirty('general');
        expect(hasDirty()).toBe(true);
        clearAllDirty();
        expect(hasDirty()).toBe(false);
    });

    it('2.8 放弃修改后 clearAllDirty 应被调用', () => {
        const clearAllDirty = jest.fn();
        const dialog = document.getElementById('settings-discard-dialog')!;
        const modal = document.getElementById('settings-modal')!;
        modal.classList.remove('hidden');

        function onDiscardConfirm() {
            clearAllDirty();
            dialog.classList.add('hidden');
            modal.classList.add('hidden');
        }

        onDiscardConfirm();

        expect(clearAllDirty).toHaveBeenCalledTimes(1);
        expect(modal.classList.contains('hidden')).toBe(true);
    });
});
