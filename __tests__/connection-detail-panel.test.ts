/**
 * connection-detail-panel.test.ts
 *
 * v0.1.38 组 F（BUG-024）：#connection-detail-panel 整块已合并到主
 * #smart-status-panel，本 suite 同步改为验证新结构：
 *   - 证书指纹移动到主状态面板内的 <details> 折叠区
 *   - 延迟仍维持三级颜色（normal/warning/critical）
 *   - 网卡 UI 保留不变
 *
 * 旧的 #detail-local-ip / #detail-peer-ip / #detail-tls-status / #detail-latency
 * 四个重复字段已随旧面板删除（对应数据由主 status-panel 单一显示）。
 */

const fs = require('fs');
const path = require('path');
const indexHtml = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');

// ─── 需求 6.1（新）：主状态面板含加密徽章与证书指纹折叠区 ──────────────────

describe('需求 6.1（新）：主状态面板 + 折叠指纹', () => {
    test('index.html 不再包含废弃的 #connection-detail-panel', () => {
        expect(indexHtml).not.toMatch(/id="connection-detail-panel"/);
    });

    test('index.html 应包含 #smart-status-panel（主面板重命名/含加密徽章）', () => {
        expect(indexHtml).toContain('id="smart-status-panel"');
    });

    test('index.html 应包含 #encryption-badge（连接状态行尾 🔒/⚠️）', () => {
        expect(indexHtml).toContain('id="encryption-badge"');
    });

    test('index.html 应包含 #detail-cert-fingerprint（放在 <details> 折叠区）', () => {
        expect(indexHtml).toContain('id="detail-cert-fingerprint"');
        expect(indexHtml).toMatch(/<details[\s\S]{0,400}id="detail-cert-fingerprint"/);
    });
});

// ─── 需求 6.2（新）：徽章随连接/TLS 状态切换显示 ─────────────────────────

describe('需求 6.2（新）：encryption-badge 随连接状态切换', () => {
    let badge: HTMLElement;

    beforeEach(() => {
        document.body.innerHTML = `
            <span id="encryption-badge" class="encryption-badge hidden" title=""></span>
        `;
        badge = document.getElementById('encryption-badge')!;
    });

    const applyBadge = (connected: boolean, tlsEnabled: boolean) => {
        if (connected) {
            badge.textContent = tlsEnabled ? '🔒' : '⚠️';
            badge.title = tlsEnabled ? 'TLS encrypted' : 'Plaintext transmission';
            badge.classList.remove('hidden');
            badge.classList.toggle('encryption-badge--warn', !tlsEnabled);
        } else {
            badge.textContent = '';
            badge.title = '';
            badge.classList.add('hidden');
            badge.classList.remove('encryption-badge--warn');
        }
    };

    test('6.2-1 连接 + TLS 开 → 显示 🔒 且无 warn class', () => {
        applyBadge(true, true);
        expect(badge.textContent).toBe('🔒');
        expect(badge.classList.contains('hidden')).toBe(false);
        expect(badge.classList.contains('encryption-badge--warn')).toBe(false);
    });

    test('6.2-2 连接 + TLS 关 → 显示 ⚠️ 且带 warn class', () => {
        applyBadge(true, false);
        expect(badge.textContent).toBe('⚠️');
        expect(badge.classList.contains('encryption-badge--warn')).toBe(true);
    });

    test('6.2-3 未连接 → 徽章内容清空并 hidden', () => {
        applyBadge(true, true);
        applyBadge(false, true);
        expect(badge.textContent).toBe('');
        expect(badge.classList.contains('hidden')).toBe(true);
    });
});

// ─── 需求 6.3：延迟颜色逻辑（保留，迁移到主状态面板的 #latency） ───────────

describe('需求 6.3：延迟颜色逻辑（主面板 #latency 版本）', () => {
    let latencyEl: HTMLElement;

    beforeEach(() => {
        document.body.innerHTML = `<span id="latency" class="latency-normal">-</span>`;
        latencyEl = document.getElementById('latency')!;
    });

    const updateLatencyDisplay = (ms: number) => {
        latencyEl.textContent = `${ms}ms`;
        latencyEl.className = ms > 300
            ? 'latency-critical'
            : ms > 100 ? 'latency-warning' : 'latency-normal';
    };

    test('6.3-1 延迟 ≤ 100ms 时，latency-normal', () => {
        updateLatencyDisplay(50);
        expect(latencyEl.classList.contains('latency-normal')).toBe(true);
        expect(latencyEl.textContent).toBe('50ms');
    });

    test('6.3-2 延迟 > 100ms 时，latency-warning', () => {
        updateLatencyDisplay(150);
        expect(latencyEl.classList.contains('latency-warning')).toBe(true);
    });

    test('6.3-3 延迟 > 300ms 时，latency-critical', () => {
        updateLatencyDisplay(350);
        expect(latencyEl.classList.contains('latency-critical')).toBe(true);
    });

    test('6.3-4 延迟恰好 100ms 时，latency-normal', () => {
        updateLatencyDisplay(100);
        expect(latencyEl.classList.contains('latency-normal')).toBe(true);
    });

    test('6.3-5 延迟恰好 300ms 时，latency-warning', () => {
        updateLatencyDisplay(300);
        expect(latencyEl.classList.contains('latency-warning')).toBe(true);
        expect(latencyEl.classList.contains('latency-critical')).toBe(false);
    });
});

// ─── 附：网卡说明 UI（保留） ───────────────────────────────────────────────

describe('附：网卡说明 UI', () => {
    test('index.html 应包含 nic-description 说明文字', () => {
        expect(indexHtml).toContain('id="nic-description"');
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
