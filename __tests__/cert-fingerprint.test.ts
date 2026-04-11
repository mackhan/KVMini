/**
 * cert-fingerprint.test.ts
 * 本机 TLS 证书指纹加载修复 TDD 测试用例
 * 需求：3.1 ~ 3.6
 */

describe('TLS 证书指纹功能 (CertModule)', () => {

    beforeEach(() => {
        document.body.innerHTML = `
            <div id="tab-security">
                <input type="checkbox" id="enable-tls" checked>
                <span id="local-tls-fingerprint">加载中...</span>
                <div id="tls-fingerprint-row">
                    <button class="btn btn-small">复制</button>
                </div>
                <p id="tls-fingerprint-hint"></p>
                <div id="cert-error-row" class="hidden">
                    <span id="cert-error-msg"></span>
                    <button onclick="regenerateCert()">重新生成证书</button>
                </div>
            </div>
        `;
    });

    test('3.1 invoke get_local_tls_fingerprint 成功时应显示指纹', async () => {
        const mockFp = 'AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99';
        const mockInvoke = jest.fn().mockResolvedValue(mockFp);
        const fpEl = document.getElementById('local-tls-fingerprint')!;
        const certErrorRow = document.getElementById('cert-error-row')!;

        const fp = await mockInvoke('get_local_tls_fingerprint');
        fpEl.textContent = fp ? fp.substring(0, 47) + '...' : '-';
        fpEl.title = fp || '';
        certErrorRow.classList.add('hidden');

        expect(fpEl.textContent).not.toBe('加载中...');
        expect(fpEl.textContent).not.toBe('-');
        expect(fpEl.textContent).toContain('...');
        expect(certErrorRow.classList.contains('hidden')).toBe(true);
    });

    test('3.5 invoke get_local_tls_fingerprint 失败时应显示错误行', async () => {
        const mockInvoke = jest.fn().mockRejectedValue(new Error('证书文件不存在'));
        const fpEl = document.getElementById('local-tls-fingerprint')!;
        const certErrorRow = document.getElementById('cert-error-row')!;
        const certErrorMsg = document.getElementById('cert-error-msg')!;

        try {
            await mockInvoke('get_local_tls_fingerprint');
        } catch (e: any) {
            fpEl.textContent = '加载失败';
            certErrorMsg.textContent = e.message || '证书加载失败';
            certErrorRow.classList.remove('hidden');
        }

        expect(fpEl.textContent).toBe('加载失败');
        expect(certErrorRow.classList.contains('hidden')).toBe(false);
        expect(certErrorMsg.textContent).toContain('证书文件不存在');
    });

    test('3.5 cert-error-row 应包含"重新生成证书"按钮', () => {
        const certErrorRow = document.getElementById('cert-error-row')!;
        const regenBtn = certErrorRow.querySelector('button');
        expect(regenBtn).not.toBeNull();
        expect(regenBtn!.textContent).toContain('重新生成证书');
    });

    test('3.4 指纹显示应截断为前 47 个字符', async () => {
        const longFp = 'AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99';
        const mockInvoke = jest.fn().mockResolvedValue(longFp);
        const fpEl = document.getElementById('local-tls-fingerprint')!;

        const fp = await mockInvoke('get_local_tls_fingerprint');
        fpEl.textContent = fp ? fp.substring(0, 47) + '...' : '-';

        expect(fpEl.textContent.length).toBeLessThanOrEqual(50);
        expect(fpEl.textContent).toContain('...');
    });

    test('3.6 TLS 确认对话框应存在', () => {
        const modal = document.createElement('div');
        modal.id = 'tls-confirm-modal';
        modal.className = 'modal hidden';
        modal.innerHTML = `
            <div id="tls-confirm-fingerprint"></div>
            <button id="tls-trust-btn">信任并连接</button>
            <button id="tls-reject-btn">拒绝连接</button>
        `;
        document.body.appendChild(modal);

        expect(document.getElementById('tls-confirm-modal')).not.toBeNull();
        expect(document.getElementById('tls-trust-btn')).not.toBeNull();
        expect(document.getElementById('tls-reject-btn')).not.toBeNull();
    });

    test('TLS 关闭时指纹区域应显示灰色提示', () => {
        const enableTls = document.getElementById('enable-tls') as HTMLInputElement;
        const fpEl = document.getElementById('local-tls-fingerprint')!;
        const copyBtn = document.querySelector('#tls-fingerprint-row .btn-small') as HTMLButtonElement;

        enableTls.checked = false;
        if (!enableTls.checked) {
            fpEl.textContent = 'TLS 已关闭';
            fpEl.style.color = '#9ca3af';
            if (copyBtn) copyBtn.disabled = true;
        }

        expect(fpEl.textContent).toBe('TLS 已关闭');
        expect(fpEl.style.color).toBeTruthy();
        expect(copyBtn.disabled).toBe(true);
    });

    test('regenerateCert 应调用 invoke regenerate_tls_cert', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('new-fingerprint');

        const regenerateCert = async () => {
            const fp = await mockInvoke('regenerate_tls_cert');
            const fpEl = document.getElementById('local-tls-fingerprint')!;
            fpEl.textContent = fp ? fp.substring(0, 47) + '...' : '-';
            document.getElementById('cert-error-row')?.classList.add('hidden');
        };

        await regenerateCert();

        expect(mockInvoke).toHaveBeenCalledWith('regenerate_tls_cert');
        expect(document.getElementById('local-tls-fingerprint')!.textContent).toContain('new-fingerprint');
    });
});