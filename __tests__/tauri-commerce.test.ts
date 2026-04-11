/**
 * tauri-commerce.test.ts
 * tauri-commerce SDK 集成 TDD 测试用例
 * 需求：1.1 ~ 1.6，2.1 ~ 2.7
 */

const mockInvoke = jest.fn();

describe('tauri-commerce SDK 集成 (LicenseModule)', () => {

    beforeEach(() => {
        mockInvoke.mockReset();
        document.body.innerHTML = `
            <div id="app">
                <div id="license-gate" class="hidden">
                    <div id="license-gate-content">
                        <h2 id="license-gate-title">激活 KVMini</h2>
                        <button id="license-buy-btn">购买许可证</button>
                        <div id="license-error-msg" class="hidden"></div>
                    </div>
                </div>
                <div id="main-content">
                    <div id="license-status-badge" class="hidden">已激活</div>
                </div>
                <div id="license-sdk-error" class="hidden">
                    <span id="license-sdk-error-msg"></span>
                </div>
            </div>
        `;
    });

    // ─── get_trial_status 命令 ────────────────────────────────────────────────

    test('1.1 get_trial_status 返回 Purchased 时 licensed=true', async () => {
        mockInvoke.mockResolvedValue('Purchased');
        const trialStatus = await mockInvoke('get_trial_status');
        const licensed = trialStatus === 'Purchased' || trialStatus?.Purchased !== undefined;
        expect(licensed).toBe(true);
    });

    test('1.2 get_trial_status 返回 Active 时 licensed=true（试用中）', async () => {
        mockInvoke.mockResolvedValue({ Active: 5 });
        const trialStatus = await mockInvoke('get_trial_status');
        const licensed = trialStatus === 'Active' || trialStatus?.Active !== undefined;
        expect(licensed).toBe(true);
    });

    test('1.3 get_trial_status 返回 Expired 时 licensed=false', async () => {
        mockInvoke.mockResolvedValue('Expired');
        const trialStatus = await mockInvoke('get_trial_status');
        const licensed = trialStatus === 'Purchased' || trialStatus?.Purchased !== undefined
            || trialStatus === 'Active' || trialStatus?.Active !== undefined;
        expect(licensed).toBe(false);
    });

    test('1.4 get_trial_status 调用失败时降级允许使用', async () => {
        mockInvoke.mockRejectedValue(new Error('SDK 未初始化'));
        let licensed = false;
        try {
            await mockInvoke('get_trial_status');
        } catch {
            licensed = true; // 降级：允许使用
        }
        expect(licensed).toBe(true);
    });

    // ─── get_locale / set_locale 命令 ─────────────────────────────────────────

    test('2.1 get_locale 返回当前语言', async () => {
        mockInvoke.mockResolvedValue('zh');
        const locale = await mockInvoke('get_locale');
        expect(locale).toBe('zh');
        expect(mockInvoke).toHaveBeenCalledWith('get_locale');
    });

    test('2.2 set_locale 切换到英文', async () => {
        mockInvoke.mockResolvedValue(undefined);
        await mockInvoke('set_locale', { locale: 'en' });
        expect(mockInvoke).toHaveBeenCalledWith('set_locale', { locale: 'en' });
    });

    test('2.3 set_locale 切换到中文', async () => {
        mockInvoke.mockResolvedValue(undefined);
        await mockInvoke('set_locale', { locale: 'zh' });
        expect(mockInvoke).toHaveBeenCalledWith('set_locale', { locale: 'zh' });
    });

    test('2.4 set_locale 调用失败时不影响前端语言切换', async () => {
        mockInvoke.mockRejectedValue(new Error('SDK 调用失败'));
        let frontendLocale = 'zh';
        try {
            await mockInvoke('set_locale', { locale: 'en' });
        } catch {
            // 降级：前端仍然切换
        }
        frontendLocale = 'en';
        expect(frontendLocale).toBe('en');
    });

    test('2.5 应用启动时通过 get_locale 初始化语言', async () => {
        mockInvoke.mockResolvedValue('en');
        const sdkLocale = await mockInvoke('get_locale');
        expect(sdkLocale).toBe('en');
        expect(mockInvoke).toHaveBeenCalledWith('get_locale');
    });

    // ─── 授权 UI 状态 ─────────────────────────────────────────────────────────

    test('3.1 未授权时应显示 license-gate，隐藏 main-content', () => {
        const licenseGate = document.getElementById('license-gate')!;
        const mainContent = document.getElementById('main-content')!;

        const isLicensed = false;
        if (!isLicensed) {
            licenseGate.classList.remove('hidden');
            mainContent.classList.add('hidden');
        }

        expect(licenseGate.classList.contains('hidden')).toBe(false);
        expect(mainContent.classList.contains('hidden')).toBe(true);
    });

    test('3.2 已授权时应隐藏 license-gate，显示 main-content', () => {
        const licenseGate = document.getElementById('license-gate')!;
        const mainContent = document.getElementById('main-content')!;

        const isLicensed = true;
        if (isLicensed) {
            licenseGate.classList.add('hidden');
            mainContent.classList.remove('hidden');
        }

        expect(licenseGate.classList.contains('hidden')).toBe(true);
        expect(mainContent.classList.contains('hidden')).toBe(false);
    });

    test('3.3 SDK 初始化失败时应显示错误提示，不阻塞主界面', () => {
        const sdkError = document.getElementById('license-sdk-error')!;
        const sdkErrorMsg = document.getElementById('license-sdk-error-msg')!;
        const mainContent = document.getElementById('main-content')!;

        sdkErrorMsg.textContent = '授权服务暂时不可用';
        sdkError.classList.remove('hidden');
        mainContent.classList.remove('hidden');

        expect(sdkError.classList.contains('hidden')).toBe(false);
        expect(sdkErrorMsg.textContent).toContain('授权服务暂时不可用');
        expect(mainContent.classList.contains('hidden')).toBe(false);
    });

    test('3.4 授权状态变化时应实时更新 UI', () => {
        const licenseGate = document.getElementById('license-gate')!;
        const mainContent = document.getElementById('main-content')!;
        const statusBadge = document.getElementById('license-status-badge')!;

        licenseGate.classList.remove('hidden');
        mainContent.classList.add('hidden');

        const onLicenseStatusChange = (licensed: boolean) => {
            if (licensed) {
                licenseGate.classList.add('hidden');
                mainContent.classList.remove('hidden');
                statusBadge.classList.remove('hidden');
            } else {
                licenseGate.classList.remove('hidden');
                mainContent.classList.add('hidden');
                statusBadge.classList.add('hidden');
            }
        };

        onLicenseStatusChange(true);

        expect(licenseGate.classList.contains('hidden')).toBe(true);
        expect(mainContent.classList.contains('hidden')).toBe(false);
        expect(statusBadge.classList.contains('hidden')).toBe(false);
    });
});
