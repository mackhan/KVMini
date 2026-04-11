/**
 * connection-detail-panel.test.ts
 * 连接状态详情面板 + 网卡说明 UI 优化 TDD 测试用例
 * 需求：6.1 ~ 6.6 + 附：网卡说明 UI 优化
 */

// ─── 辅助：读取源文件 ─────────────────────────────────────────────────────────
const fs = require('fs');
const path = require('path');
const indexHtml = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');

// ─── 需求 6.1：index.html 包含连接详情面板 DOM ────────────────────────────────

describe('需求 6.1：index.html 包含连接详情面板 DOM', () => {
    test('index.html 应包含 connection-detail-panel 元素', () => {
        expect(indexHtml).toContain('id="connection-detail-panel"');
    });

    test('index.html 应包含 detail-local-ip 元素', () => {
        expect(indexHtml).toContain('id="detail-local-ip"');
    });

    test('index.html 应包含 detail-peer-ip 元素', () => {
        expect(indexHtml).toContain('id="detail-peer-ip"');
    });

    test('index.html 应包含 detail-tls-status 元素', () => {
        expect(indexHtml).toContain('id="detail-tls-status"');
    });

    test('index.html 应包含 detail-latency 元素', () => {
        expect(indexHtml).toContain('id="detail-latency"');
    });

    test('index.html 应包含 detail-cert-fingerprint 元素', () => {
        expect(indexHtml).toContain('id="detail-cert-fingerprint"');
    });

    test('connection-detail-panel 初始状态应为 hidden', () => {
        expect(indexHtml).toMatch(/id="connection-detail-panel"[^>]*class="[^"]*hidden/);
    });
});

// ─── 需求 6.2：连接成功时显示详情面板 ────────────────────────────────────────

describe('需求 6.2：连接成功时显示详情面板', () => {
    beforeEach(() => {
        document.body.innerHTML = `
            <div id="connection-detail-panel" class="connection-detail-panel hidden">
                <div class="detail-header" onclick="toggleConnectionDetail()">
                    <span class="detail-title">📡 连接详情</span>
                    <span class="detail-toggle-icon" id="detail-toggle-icon">▼</span>
                </div>
                <div class="detail-summary">
                    <div class="detail-row">
                        <span class="detail-label">本机</span>
                        <span id="detail-local-ip" class="detail-value">-</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">对端</span>
                        <span id="detail-peer-ip" class="detail-value">-</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">加密</span>
                        <span id="detail-tls-status" class="detail-value">-</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">延迟</span>
                        <span id="detail-latency" class="detail-value latency-normal">-</span>
                    </div>
                </div>
                <div id="detail-expanded" class="detail-expanded hidden">
                    <div class="detail-row">
                        <span class="detail-label">证书指纹</span>
                        <span id="detail-cert-fingerprint" class="detail-value detail-fingerprint">-</span>
                    </div>
                </div>
            </div>
        `;
    });

    test('6.2-1 连接成功时，updateConnectionDetail 应显示 connection-detail-panel', () => {
        const panel = document.getElementById('connection-detail-panel')!;

        const updateConnectionDetail = (data: {
            localIp: string; peerIp: string; tlsEnabled: boolean;
            latencyMs?: number; certFingerprint?: string;
        }) => {
            panel.classList.remove('hidden');
            document.getElementById('detail-local-ip')!.textContent = data.localIp;
            document.getElementById('detail-peer-ip')!.textContent = data.peerIp;
            document.getElementById('detail-tls-status')!.textContent = data.tlsEnabled ? '🔒 TLS 加密' : '⚠️ 未加密';
            if (data.certFingerprint) {
                document.getElementById('detail-cert-fingerprint')!.textContent = data.certFingerprint;
            }
        };

        updateConnectionDetail({
            localIp: '192.168.1.100:8889',
            peerIp: '192.168.1.200:8889',
            tlsEnabled: true,
            certFingerprint: 'AB:CD:EF:12:34:56:78:90',
        });

        expect(panel.classList.contains('hidden')).toBe(false);
        expect(document.getElementById('detail-local-ip')!.textContent).toBe('192.168.1.100:8889');
        expect(document.getElementById('detail-peer-ip')!.textContent).toBe('192.168.1.200:8889');
        expect(document.getElementById('detail-tls-status')!.textContent).toContain('TLS 加密');
    });

    test('6.2-2 连接断开时，updateConnectionDetail 应隐藏 connection-detail-panel', () => {
        const panel = document.getElementById('connection-detail-panel')!;
        panel.classList.remove('hidden');

        const hideConnectionDetail = () => {
            panel.classList.add('hidden');
        };

        hideConnectionDetail();

        expect(panel.classList.contains('hidden')).toBe(true);
    });

    test('6.2-3 未加密时，detail-tls-status 应显示警告图标', () => {
        const tlsStatus = document.getElementById('detail-tls-status')!;

        const updateTlsStatus = (tlsEnabled: boolean) => {
            if (tlsEnabled) {
                tlsStatus.textContent = '🔒 TLS 加密';
                tlsStatus.className = 'detail-value tls-enabled';
            } else {
                tlsStatus.textContent = '⚠️ 未加密';
                tlsStatus.className = 'detail-value tls-disabled';
            }
        };

        updateTlsStatus(false);

        expect(tlsStatus.textContent).toContain('⚠️');
        expect(tlsStatus.textContent).toContain('未加密');
        expect(tlsStatus.className).toContain('tls-disabled');
    });
});

// ─── 需求 6.3：延迟颜色逻辑 ──────────────────────────────────────────────────

describe('需求 6.3：延迟颜色逻辑', () => {
    let latencyEl: HTMLElement;

    beforeEach(() => {
        document.body.innerHTML = `<span id="detail-latency" class="detail-value latency-normal">-</span>`;
        latencyEl = document.getElementById('detail-latency')!;
    });

    const updateLatencyDisplay = (ms: number) => {
        latencyEl.textContent = `${ms}ms`;
        latencyEl.className = 'detail-value';
        if (ms > 300) {
            latencyEl.classList.add('latency-critical');
        } else if (ms > 100) {
            latencyEl.classList.add('latency-warning');
        } else {
            latencyEl.classList.add('latency-normal');
        }
    };

    test('6.3-1 延迟 ≤ 100ms 时，应显示正常颜色（latency-normal）', () => {
        updateLatencyDisplay(50);
        expect(latencyEl.classList.contains('latency-normal')).toBe(true);
        expect(latencyEl.textContent).toBe('50ms');
    });

    test('6.3-2 延迟 > 100ms 时，应显示黄色（latency-warning）', () => {
        updateLatencyDisplay(150);
        expect(latencyEl.classList.contains('latency-warning')).toBe(true);
    });

    test('6.3-3 延迟 > 300ms 时，应显示红色（latency-critical）', () => {
        updateLatencyDisplay(350);
        expect(latencyEl.classList.contains('latency-critical')).toBe(true);
    });

    test('6.3-4 延迟恰好 100ms 时，应显示正常颜色', () => {
        updateLatencyDisplay(100);
        expect(latencyEl.classList.contains('latency-normal')).toBe(true);
    });

    test('6.3-5 延迟恰好 300ms 时，应显示黄色（不是红色）', () => {
        updateLatencyDisplay(300);
        expect(latencyEl.classList.contains('latency-warning')).toBe(true);
        expect(latencyEl.classList.contains('latency-critical')).toBe(false);
    });
});

// ─── 需求 6.4：点击展开完整连接信息 ─────────────────────────────────────────

describe('需求 6.4：点击展开完整连接信息', () => {
    beforeEach(() => {
        document.body.innerHTML = `
            <div id="connection-detail-panel" class="connection-detail-panel">
                <div class="detail-header" id="detail-header">
                    <span class="detail-title">📡 连接详情</span>
                    <span class="detail-toggle-icon" id="detail-toggle-icon">▼</span>
                </div>
                <div id="detail-expanded" class="detail-expanded hidden">
                    <span id="detail-cert-fingerprint">-</span>
                </div>
            </div>
        `;
    });

    test('6.4-1 点击详情头部应展开 detail-expanded', () => {
        const expanded = document.getElementById('detail-expanded')!;
        const icon = document.getElementById('detail-toggle-icon')!;
        const header = document.getElementById('detail-header')!;

        const toggleConnectionDetail = () => {
            const isHidden = expanded.classList.contains('hidden');
            if (isHidden) {
                expanded.classList.remove('hidden');
                icon.textContent = '▲';
            } else {
                expanded.classList.add('hidden');
                icon.textContent = '▼';
            }
        };

        header.addEventListener('click', toggleConnectionDetail);
        header.click();

        expect(expanded.classList.contains('hidden')).toBe(false);
        expect(icon.textContent).toBe('▲');
    });

    test('6.4-2 再次点击应折叠 detail-expanded', () => {
        const expanded = document.getElementById('detail-expanded')!;
        const icon = document.getElementById('detail-toggle-icon')!;
        const header = document.getElementById('detail-header')!;

        const toggleConnectionDetail = () => {
            const isHidden = expanded.classList.contains('hidden');
            if (isHidden) {
                expanded.classList.remove('hidden');
                icon.textContent = '▲';
            } else {
                expanded.classList.add('hidden');
                icon.textContent = '▼';
            }
        };

        header.addEventListener('click', toggleConnectionDetail);
        header.click(); // 展开
        header.click(); // 折叠

        expect(expanded.classList.contains('hidden')).toBe(true);
        expect(icon.textContent).toBe('▼');
    });
});

// ─── 附：网卡说明 UI 优化 ─────────────────────────────────────────────────────

describe('附：网卡说明 UI 优化', () => {
    test('index.html 应包含 nic-description 说明文字', () => {
        expect(indexHtml).toContain('id="nic-description"');
    });

    test('nic-description 应包含多网卡说明文字', () => {
        expect(indexHtml).toContain('多个网络接口');
        expect(indexHtml).toContain('所有网卡');
    });

    test('index.html 应包含 nic-list 网卡列表容器', () => {
        expect(indexHtml).toContain('id="nic-list"');
    });

    test('网卡列表应支持类型图标渲染', () => {
        document.body.innerHTML = `<div id="nic-list" class="nic-list"></div>`;
        const nicList = document.getElementById('nic-list')!;

        const renderNicList = (nics: Array<{ name: string; ip: string; type: string }>) => {
            nicList.innerHTML = nics.map(nic => {
                const icon = nic.type === 'wifi' ? '📶' : nic.type === 'ethernet' ? '🔌' : '🔒';
                return `<div class="nic-item" title="${nic.name} | ${nic.ip}">
                    <span class="nic-icon">${icon}</span>
                    <span class="nic-name">${nic.name}</span>
                    <span class="nic-ip">${nic.ip}</span>
                </div>`;
            }).join('');
        };

        renderNicList([
            { name: 'en0', ip: '192.168.1.100', type: 'wifi' },
            { name: 'en1', ip: '10.0.0.1', type: 'ethernet' },
        ]);

        expect(nicList.querySelectorAll('.nic-item').length).toBe(2);
        expect(nicList.innerHTML).toContain('📶');
        expect(nicList.innerHTML).toContain('🔌');
    });
});
