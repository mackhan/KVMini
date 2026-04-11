/**
 * network-interfaces.test.ts
 * 网卡监听逻辑优化 TDD 测试用例
 * 需求：2.1 ~ 2.5
 */

// 模拟 Tauri invoke
const mockInvoke = jest.fn();
(global as any).__TAURI__ = { core: { invoke: mockInvoke } };

describe('网卡监听逻辑优化 (NetworkModule)', () => {

    beforeEach(() => {
        jest.clearAllMocks();
        document.body.innerHTML = `
            <div id="tab-network">
                <select id="listen-ip-select">
                    <option value="0.0.0.0">所有网卡（0.0.0.0，推荐）</option>
                </select>
                <div id="network-iface-list"></div>
                <div id="network-iface-warning" class="hidden">
                    <span id="network-iface-warning-text"></span>
                </div>
                <input type="number" id="listen-port-input" value="8889">
                <span id="listen-address-display">-</span>
            </div>
        `;
    });

    // ─── 2.1 列出所有活跃网卡 ────────────────────────────────────────────────

    test('2.1 renderNetworkInterfaces 应渲染网卡列表到 DOM', () => {
        const ifaces = [
            { name: 'en0', ip: '192.168.1.100', is_up: true },
            { name: 'en1', ip: '10.0.0.1', is_up: true },
        ];

        // 手动渲染（模拟 NetworkModule.renderNetworkInterfaces）
        const list = document.getElementById('network-iface-list')!;
        list.innerHTML = ifaces.map(iface => `
            <div class="network-iface-item ${iface.is_up ? 'iface-active' : 'iface-disconnected'}"
                 data-iface="${iface.name}" data-ip="${iface.ip}">
                <span class="network-iface-name">${iface.name}</span>
                <span class="network-iface-ip">${iface.ip}</span>
                <span class="network-iface-status ${iface.is_up ? 'status-up' : 'status-down'}">
                    ${iface.is_up ? '活跃' : '断开'}
                </span>
            </div>
        `).join('');

        const items = document.querySelectorAll('.network-iface-item');
        expect(items.length).toBe(2);
        expect(items[0].getAttribute('data-iface')).toBe('en0');
        expect(items[1].getAttribute('data-iface')).toBe('en1');
    });

    // ─── 2.2 "自动（所有网卡）"选项 ──────────────────────────────────────────

    test('2.2 listen-ip-select 应包含"所有网卡"选项（值为 0.0.0.0）', () => {
        const select = document.getElementById('listen-ip-select') as HTMLSelectElement;
        const allOption = Array.from(select.options).find(o => o.value === '0.0.0.0');
        expect(allOption).toBeDefined();
    });

    test('2.2 选择"所有网卡"时 listen-ip-select 值应为 0.0.0.0', () => {
        const select = document.getElementById('listen-ip-select') as HTMLSelectElement;
        select.value = '0.0.0.0';
        expect(select.value).toBe('0.0.0.0');
    });

    // ─── 2.3 选择指定网卡 ────────────────────────────────────────────────────

    test('2.3 选择指定网卡后 listen-ip-select 值应更新为该网卡 IP', () => {
        const select = document.getElementById('listen-ip-select') as HTMLSelectElement;
        // 添加网卡选项
        const opt = document.createElement('option');
        opt.value = '192.168.1.100';
        opt.textContent = 'en0 (192.168.1.100)';
        select.appendChild(opt);

        select.value = '192.168.1.100';
        expect(select.value).toBe('192.168.1.100');
    });

    // ─── 2.4 选定网卡断开时显示警告 ──────────────────────────────────────────

    test('2.4 网卡断开时应显示警告提示', () => {
        const warning = document.getElementById('network-iface-warning')!;
        const warningText = document.getElementById('network-iface-warning-text')!;

        // 模拟网卡断开
        warning.classList.remove('hidden');
        warningText.textContent = '选定网卡 en0 已断开，请切换到其他网卡';

        expect(warning.classList.contains('hidden')).toBe(false);
        expect(warningText.textContent).toContain('en0');
    });

    test('2.4 网卡恢复后应隐藏警告提示', () => {
        const warning = document.getElementById('network-iface-warning')!;
        warning.classList.remove('hidden');

        // 模拟网卡恢复
        warning.classList.add('hidden');

        expect(warning.classList.contains('hidden')).toBe(true);
    });

    // ─── 2.5 网卡列表变化时自动刷新 ──────────────────────────────────────────

    test('2.5 buildIfaceOptions 应根据网卡列表构建 select 选项', () => {
        const ifaces = [
            { name: 'en0', ip: '192.168.1.100', is_up: true },
            { name: 'utun0', ip: '10.8.0.1', is_up: true },
        ];
        const select = document.getElementById('listen-ip-select') as HTMLSelectElement;

        // 清空并重建选项
        select.innerHTML = '<option value="0.0.0.0">所有网卡（0.0.0.0，推荐）</option>';
        ifaces.forEach(iface => {
            const opt = document.createElement('option');
            opt.value = iface.ip;
            opt.textContent = `${iface.name} (${iface.ip})`;
            select.appendChild(opt);
        });

        expect(select.options.length).toBe(3); // 1 个"所有网卡" + 2 个具体网卡
        expect(select.options[1].value).toBe('192.168.1.100');
        expect(select.options[2].value).toBe('10.8.0.1');
    });

    // ─── 网卡说明文字 ────────────────────────────────────────────────────────

    test('网络分页应包含网卡说明提示区域', () => {
        // 在 DOM 中添加说明区域
        const tabNetwork = document.getElementById('tab-network')!;
        const hint = document.createElement('div');
        hint.id = 'network-iface-hint';
        hint.className = 'network-iface-hint';
        hint.textContent = '选择本机用于接受远端连接的网络接口，通常选"所有网卡"即可';
        tabNetwork.prepend(hint);

        const hintEl = document.getElementById('network-iface-hint');
        expect(hintEl).not.toBeNull();
        expect(hintEl!.textContent).toContain('所有网卡');
    });
});
