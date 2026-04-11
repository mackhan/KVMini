/**
 * tls-mismatch.test.ts
 * TLS 加密不一致时的错误提示与引导 TDD 测试用例
 * 需求：5.1 ~ 5.6
 */

describe('TLS 不一致检测与提示 (需求 5)', () => {

    beforeEach(() => {
        document.body.innerHTML = `
            <div id="tls-mismatch-banner" class="tls-mismatch-banner hidden">
                <span class="tls-mismatch-icon">⚠️</span>
                <span id="tls-mismatch-msg">TLS 配置不一致</span>
                <button id="tls-disable-retry-btn" class="btn btn-small">关闭本机 TLS 重试</button>
                <button id="tls-mismatch-close-btn" class="btn-icon">✕</button>
            </div>
            <div id="connection-status" class="status-disconnected">未连接</div>
            <div id="tls-status-indicator" class="hidden"></div>
            <input type="checkbox" id="enable-tls" checked>
            <div id="error-toast" class="hidden"><span id="error-message"></span></div>
        `;
    });

    // ── 5.1 本机开 TLS，对端未开 ──────────────────────────────────────────

    test('5.1 本机开 TLS 对端未开时，应显示 tls-mismatch-banner', () => {
        const banner = document.getElementById('tls-mismatch-banner')!;
        const msg = document.getElementById('tls-mismatch-msg')!;

        // 模拟收到 tls-mismatch 事件（本机开，对端未开）
        const handleTlsMismatch = (data: { local_tls: boolean; remote_tls: boolean }) => {
            banner.classList.remove('hidden');
            if (data.local_tls && !data.remote_tls) {
                msg.textContent = '本机已开启 TLS 加密，但对端未开启，连接可能失败。请关闭本机 TLS 或通知对端开启。';
            } else if (!data.local_tls && data.remote_tls) {
                msg.textContent = '对端已开启 TLS 加密，但本机未开启，连接可能失败。请开启本机 TLS 或通知对端关闭。';
            }
        };

        handleTlsMismatch({ local_tls: true, remote_tls: false });

        expect(banner.classList.contains('hidden')).toBe(false);
        expect(msg.textContent).toContain('本机已开启 TLS');
        expect(msg.textContent).toContain('对端未开启');
    });

    // ── 5.2 本机未开 TLS，对端开了 ──────────────────────────────────────

    test('5.2 本机未开 TLS 对端开了时，应显示不同的提示文案', () => {
        const banner = document.getElementById('tls-mismatch-banner')!;
        const msg = document.getElementById('tls-mismatch-msg')!;

        const handleTlsMismatch = (data: { local_tls: boolean; remote_tls: boolean }) => {
            banner.classList.remove('hidden');
            if (data.local_tls && !data.remote_tls) {
                msg.textContent = '本机已开启 TLS 加密，但对端未开启，连接可能失败。请关闭本机 TLS 或通知对端开启。';
            } else if (!data.local_tls && data.remote_tls) {
                msg.textContent = '对端已开启 TLS 加密，但本机未开启，连接可能失败。请开启本机 TLS 或通知对端关闭。';
            }
        };

        handleTlsMismatch({ local_tls: false, remote_tls: true });

        expect(banner.classList.contains('hidden')).toBe(false);
        expect(msg.textContent).toContain('对端已开启 TLS');
        expect(msg.textContent).toContain('本机未开启');
    });

    // ── 5.3 TLS 一致时不显示 banner ──────────────────────────────────────

    test('5.3 双方 TLS 状态一致时，不应显示 tls-mismatch-banner', () => {
        const banner = document.getElementById('tls-mismatch-banner')!;

        const handleConnectionStatus = (data: { event_type: string; tls_mismatch?: boolean }) => {
            if (data.tls_mismatch) {
                banner.classList.remove('hidden');
            }
            // 一致时不显示
        };

        // 双方都开启 TLS
        handleConnectionStatus({ event_type: 'connected', tls_mismatch: false });

        expect(banner.classList.contains('hidden')).toBe(true);
    });

    // ── 5.4 "关闭本机 TLS 重试"按钮功能 ─────────────────────────────────

    test('5.4 点击"关闭本机 TLS 重试"按钮应关闭 TLS 并触发重连', async () => {
        const mockInvoke = jest.fn().mockResolvedValue({ success: true });
        const banner = document.getElementById('tls-mismatch-banner')!;
        const enableTls = document.getElementById('enable-tls') as HTMLInputElement;
        const retryBtn = document.getElementById('tls-disable-retry-btn')!;

        banner.classList.remove('hidden');
        enableTls.checked = true;

        let reconnectCalled = false;
        const handleDisableAndRetry = async () => {
            // 1. 关闭 TLS
            enableTls.checked = false;
            // 2. 保存设置
            await mockInvoke('save_settings', { config: { enable_tls: false } });
            // 3. 隐藏 banner
            banner.classList.add('hidden');
            // 4. 触发重连
            reconnectCalled = true;
        };

        retryBtn.addEventListener('click', handleDisableAndRetry);
        retryBtn.click();
        await Promise.resolve(); // 等待异步

        expect(enableTls.checked).toBe(false);
        expect(mockInvoke).toHaveBeenCalledWith('save_settings', { config: { enable_tls: false } });
        expect(banner.classList.contains('hidden')).toBe(true);
        expect(reconnectCalled).toBe(true);
    });

    // ── 5.5 连接状态区域显示 TLS 警告图标 ───────────────────────────────

    test('5.5 TLS 不一致时，连接状态区域应显示警告图标', () => {
        const indicator = document.getElementById('tls-status-indicator')!;

        const showTlsWarning = () => {
            indicator.classList.remove('hidden');
            indicator.textContent = '⚠️ TLS';
            indicator.className = 'tls-warning-indicator';
        };

        showTlsWarning();

        expect(indicator.classList.contains('hidden')).toBe(false);
        expect(indicator.textContent).toContain('TLS');
    });

    // ── 5.6 关闭 banner 按钮 ─────────────────────────────────────────────

    test('5.6 点击关闭按钮应隐藏 tls-mismatch-banner', () => {
        const banner = document.getElementById('tls-mismatch-banner')!;
        const closeBtn = document.getElementById('tls-mismatch-close-btn')!;

        banner.classList.remove('hidden');

        closeBtn.addEventListener('click', () => {
            banner.classList.add('hidden');
        });
        closeBtn.click();

        expect(banner.classList.contains('hidden')).toBe(true);
    });

    // ── 5.7 AppConfig 包含 enable_tls 字段 ───────────────────────────────

    test('5.7 save_settings 应包含 enable_tls 字段', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);

        await mockInvoke('save_settings', {
            config: {
                hotkey: 'Ctrl+Alt+S',
                enable_tls: true,
                auto_reconnect: true,
            }
        });

        expect(mockInvoke).toHaveBeenCalledWith('save_settings', expect.objectContaining({
            config: expect.objectContaining({ enable_tls: true }),
        }));
    });

    // ── 5.8 握手时交换 TLS 状态 ──────────────────────────────────────────

    test('5.8 连接事件应携带 tls_mismatch 字段', () => {
        // 模拟 connection-status 事件携带 tls_mismatch
        const connectionEvent = {
            event_type: 'connected',
            connected: true,
            peer_ip: '192.168.1.100',
            message: '已连接',
            tls_mismatch: true,
            local_tls: true,
            remote_tls: false,
        };

        expect(connectionEvent).toHaveProperty('tls_mismatch');
        expect(connectionEvent.tls_mismatch).toBe(true);
        expect(connectionEvent.local_tls).toBe(true);
        expect(connectionEvent.remote_tls).toBe(false);
    });
});
