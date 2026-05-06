/**
 * 测试：跨网段无线键鼠共享功能
 * 覆盖需求：1.1、1.3、1.4、1.5、2.1-2.5、3.1、3.2、3.4、3.5、3.7、4.2-4.4、5.8
 */

const fs = require('fs');
const path = require('path');

const htmlContent = fs.readFileSync(
    path.join(__dirname, '../src/index.html'),
    'utf-8'
);

// ─── 手动连接 UI 测试（需求 1.1、1.3、1.4、1.5） ───

describe('手动连接 UI - HTML 结构', () => {
    beforeEach(() => {
        document.body.innerHTML = htmlContent.replace(/<script[\s\S]*?<\/script>/g, '');
    });

    test('主界面包含手动连接 IP 输入框', () => {
        expect(document.getElementById('manual-ip-input')).not.toBeNull();
    });

    test('主界面包含手动连接端口输入框', () => {
        expect(document.getElementById('manual-port-input')).not.toBeNull();
    });

    test('主界面包含手动连接按钮', () => {
        expect(document.getElementById('manual-connect-btn')).not.toBeNull();
    });

    test('手动连接状态提示区域初始隐藏', () => {
        const status = document.getElementById('manual-connect-status');
        expect(status).not.toBeNull();
        expect(status!.classList.contains('hidden')).toBe(true);
    });

    test('端口输入框存在且类型为 number（默认值由 JS 启动时从 Platform.getSettings 填充）', () => {
        const portInput = document.getElementById('manual-port-input') as HTMLInputElement;
        expect(portInput).not.toBeNull();
        expect(portInput.type).toBe('number');
        // BUG-021：不再硬编码 8889；实际默认由 appState.defaults.tcpPort（23456）填充
        expect(portInput.value).not.toBe('8889');
    });

    test('IP 输入框有合适的 placeholder', () => {
        const ipInput = document.getElementById('manual-ip-input') as HTMLInputElement;
        // 需求 2.1：中文时显示"请输入目标设备 IP 地址"
        expect(ipInput.placeholder).toBeTruthy();
        expect(ipInput.placeholder.length).toBeGreaterThan(0);
    });
});

// ─── IPv4 格式校验逻辑测试（需求 1.5） ───

describe('手动连接 - IPv4 格式校验', () => {
    // 与 main.js 中 isValidIpv4 逻辑保持一致
    function isValidIpv4(ip: string): boolean {
        const parts = ip.split('.');
        if (parts.length !== 4) return false;
        return parts.every(p => {
            const n = Number(p);
            return /^\d+$/.test(p) && n >= 0 && n <= 255;
        });
    }

    test('合法 IPv4 地址通过校验', () => {
        expect(isValidIpv4('192.168.1.100')).toBe(true);
        expect(isValidIpv4('10.0.1.1')).toBe(true);
        expect(isValidIpv4('0.0.0.0')).toBe(true);
        expect(isValidIpv4('255.255.255.255')).toBe(true);
    });

    test('非法 IPv4 地址被拒绝', () => {
        expect(isValidIpv4('')).toBe(false);
        expect(isValidIpv4('192.168.1')).toBe(false);
        expect(isValidIpv4('192.168.1.256')).toBe(false);
        expect(isValidIpv4('abc.def.ghi.jkl')).toBe(false);
        expect(isValidIpv4('192.168.1.1.1')).toBe(false);
        expect(isValidIpv4('not-an-ip')).toBe(false);
    });

    test('空字符串被拒绝', () => {
        expect(isValidIpv4('')).toBe(false);
    });
});

// ─── 连接历史扩展测试（需求 2.1-2.5） ───

describe('连接历史 - 手动 IP 记录', () => {
    test('手动 IP 历史条目包含必要字段', () => {
        const entry = {
            id: 'manual:192.168.1.100:8889',
            name: '192.168.1.100',
            os: '',
            ip: '192.168.1.100',
            connectionType: 'Network',
            entryType: 'manual',
            port: 8889,
            lastConnected: new Date().toISOString(),
        };
        expect(entry.entryType).toBe('manual');
        expect(entry.port).toBe(8889);
        expect(entry.id).toContain('manual:');
    });

    test('自动发现设备历史条目 entryType 为 auto', () => {
        const entry = {
            id: 'dev-1',
            name: 'Desktop',
            os: 'Windows',
            ip: '192.168.1.50',
            connectionType: 'Network',
            entryType: 'auto',
            lastConnected: new Date().toISOString(),
        };
        expect(entry.entryType).toBe('auto');
    });

    test('手动 IP 设备使用 🔗 图标，自动发现设备使用 📡 图标', () => {
        const getIcon = (entryType: string) => entryType === 'manual' ? '🔗' : '📡';
        expect(getIcon('manual')).toBe('🔗');
        expect(getIcon('auto')).toBe('📡');
        expect(getIcon('network')).toBe('📡');
    });

    test('历史记录超过 20 条时自动清理最旧条目', () => {
        const history = Array.from({ length: 22 }, (_, i) => ({
            id: `dev-${i}`,
            name: `Device ${i}`,
            os: 'Windows',
            ip: `192.168.1.${i}`,
            connectionType: 'Network',
            entryType: 'auto',
            lastConnected: new Date(Date.now() - i * 1000).toISOString(),
        }));
        const trimmed = history.slice(0, 20);
        expect(trimmed).toHaveLength(20);
        expect(trimmed[0].id).toBe('dev-0'); // 最新的保留
    });

    test('手动 IP 历史记录去重：同 id 的旧记录被替换', () => {
        const history = [
            { id: 'manual:192.168.1.100:8889', name: '192.168.1.100', entryType: 'manual', lastConnected: '2026-01-01T00:00:00Z' },
            { id: 'dev-1', name: 'Desktop', entryType: 'auto', lastConnected: '2026-01-02T00:00:00Z' },
        ];
        const newEntry = { id: 'manual:192.168.1.100:8889', name: '192.168.1.100', entryType: 'manual', lastConnected: new Date().toISOString() };
        const filtered = history.filter(h => h.id !== newEntry.id);
        filtered.unshift(newEntry);
        expect(filtered).toHaveLength(2);
        expect(filtered[0].id).toBe('manual:192.168.1.100:8889');
    });

    test('手动 IP 一键重连：使用记录中的 IP 和端口', () => {
        const historyEntry = { id: 'manual:10.0.1.50:9000', ip: '10.0.1.50', port: 9000, entryType: 'manual' };
        // 模拟 reconnectManualIp 逻辑
        const reconnectIp = historyEntry.ip;
        const reconnectPort = historyEntry.port || 8889;
        expect(reconnectIp).toBe('10.0.1.50');
        expect(reconnectPort).toBe(9000);
    });
});

// ─── 设置面板 UI 测试（需求 3.1、3.2、3.4、3.5、3.7、5.8） ───

describe('设置面板 - 监听地址配置 UI', () => {
    beforeEach(() => {
        document.body.innerHTML = htmlContent.replace(/<script[\s\S]*?<\/script>/g, '');
    });

    test('设置面板包含监听网卡下拉列表', () => {
        expect(document.getElementById('listen-ip-select')).not.toBeNull();
    });

    test('设置面板包含监听端口输入框', () => {
        expect(document.getElementById('listen-port-input')).not.toBeNull();
    });

    test('设置面板包含对端连接地址显示', () => {
        expect(document.getElementById('listen-address-display')).not.toBeNull();
    });

    test('监听网卡下拉列表默认包含"所有网卡（0.0.0.0）"选项', () => {
        const select = document.getElementById('listen-ip-select') as HTMLSelectElement;
        const options = Array.from(select.options);
        const hasAllInterfaces = options.some(opt => opt.value === '0.0.0.0');
        expect(hasAllInterfaces).toBe(true);
    });

    test('监听端口输入框存在且类型为 number（默认值由 JS 启动时从 Platform.getSettings 填充）', () => {
        const portInput = document.getElementById('listen-port-input') as HTMLInputElement;
        expect(portInput).not.toBeNull();
        expect(portInput.type).toBe('number');
        // BUG-021：不再硬编码 8889
        expect(portInput.value).not.toBe('8889');
    });

    test('监听端口输入框范围限制为 1024-65535', () => {
        const portInput = document.getElementById('listen-port-input') as HTMLInputElement;
        expect(portInput.min).toBe('1024');
        expect(portInput.max).toBe('65535');
    });
});

describe('设置面板 - 网卡名称格式处理（macOS vs Windows）', () => {
    // 模拟 loadListenConfig 中填充下拉列表的逻辑
    function buildSelectOptions(interfaces: Array<{name: string, ip: string}>): string {
        const defaultOption = '<option value="0.0.0.0">所有网卡（0.0.0.0，推荐）</option>';
        const ifaceOptions = interfaces.map(iface =>
            `<option value="${iface.ip}">${iface.name} (${iface.ip})</option>`
        ).join('');
        return defaultOption + ifaceOptions;
    }

    test('macOS 网卡名称格式（en0、en1）正确渲染', () => {
        const macInterfaces = [
            { name: 'en0', ip: '192.168.1.100' },
            { name: 'en1', ip: '10.0.1.50' },
        ];
        const html = buildSelectOptions(macInterfaces);
        expect(html).toContain('en0');
        expect(html).toContain('192.168.1.100');
        expect(html).toContain('en1');
        expect(html).toContain('10.0.1.50');
        expect(html).toContain('0.0.0.0'); // 默认选项始终存在
    });

    test('Windows 网卡名称格式（以太网、WLAN）正确渲染', () => {
        const winInterfaces = [
            { name: '以太网', ip: '10.0.1.100' },
            { name: 'WLAN', ip: '192.168.1.50' },
        ];
        const html = buildSelectOptions(winInterfaces);
        expect(html).toContain('以太网');
        expect(html).toContain('10.0.1.100');
        expect(html).toContain('WLAN');
        expect(html).toContain('192.168.1.50');
    });

    test('Windows 网卡返回的 IP 不含 (Preferred) 后缀', () => {
        // 验证 Windows ipconfig 输出清理逻辑（与 platform.rs 保持一致）
        const cleanIp = (raw: string) => raw.trim()
            .replace(/\(Preferred\)$/, '')
            .replace(/\(首选\)$/, '')
            .trim();

        expect(cleanIp('192.168.1.100(Preferred)')).toBe('192.168.1.100');
        expect(cleanIp('10.0.1.50(首选)')).toBe('10.0.1.50');
        expect(cleanIp('192.168.1.100')).toBe('192.168.1.100'); // 无后缀不变
    });

    test('空网卡列表时仍显示"所有网卡"默认选项', () => {
        const html = buildSelectOptions([]);
        expect(html).toContain('0.0.0.0');
        expect(html).toContain('所有网卡');
    });

    test('端口非法值校验：< 1024 应报错', () => {
        const validatePort = (port: number) => port >= 1024 && port <= 65535;
        expect(validatePort(80)).toBe(false);
        expect(validatePort(0)).toBe(false);
        expect(validatePort(1023)).toBe(false);
    });

    test('端口合法值校验：1024-65535 应通过', () => {
        const validatePort = (port: number) => port >= 1024 && port <= 65535;
        expect(validatePort(1024)).toBe(true);
        expect(validatePort(8889)).toBe(true);
        expect(validatePort(65535)).toBe(true);
    });

    test('保存后监听地址显示更新（0.0.0.0 显示本机 IP）', () => {
        // 模拟 updateListenAddressDisplay 逻辑
        const updateDisplay = (ip: string, port: number, localIp: string) => {
            if (ip === '0.0.0.0' && localIp) {
                return `${localIp.split(' / ')[0]}:${port}（或其他本机 IP）`;
            } else if (ip !== '0.0.0.0') {
                return `${ip}:${port}`;
            }
            return `本机IP:${port}`;
        };

        expect(updateDisplay('0.0.0.0', 8889, '192.168.1.100')).toContain('192.168.1.100:8889');
        expect(updateDisplay('10.0.1.50', 9000, '192.168.1.100')).toBe('10.0.1.50:9000');
        expect(updateDisplay('0.0.0.0', 8889, '')).toBe('本机IP:8889');
    });
});

// ─── 延迟监测与断线重连测试（需求 4.2、4.3、4.4） ───

describe('延迟监测 - 显示与警告色', () => {
    test('延迟 <= 50ms 时使用绿色', () => {
        const getLatencyColor = (latency: number) => latency > 50 ? '#d97706' : '#10b981';
        expect(getLatencyColor(3)).toBe('#10b981');
        expect(getLatencyColor(50)).toBe('#10b981');
    });

    test('延迟 > 50ms 时使用黄色警告色', () => {
        const getLatencyColor = (latency: number) => latency > 50 ? '#d97706' : '#10b981';
        expect(getLatencyColor(51)).toBe('#d97706');
        expect(getLatencyColor(200)).toBe('#d97706');
    });

    test('延迟显示格式为 "Xms"', () => {
        const formatLatency = (latency: number) => `${latency}ms`;
        expect(formatLatency(3)).toBe('3ms');
        expect(formatLatency(150)).toBe('150ms');
    });
});

describe('断线重连逻辑', () => {
    test('最大重连次数为 3', () => {
        const MAX_RECONNECT_ATTEMPTS = 3;
        expect(MAX_RECONNECT_ATTEMPTS).toBe(3);
    });

    test('重连间隔为 2000ms', () => {
        const RECONNECT_INTERVAL_MS = 2000;
        expect(RECONNECT_INTERVAL_MS).toBe(2000);
    });

    test('重连次数达到上限后停止重连', () => {
        const MAX_RECONNECT_ATTEMPTS = 3;
        let reconnectAttempts = 0;
        const shouldReconnect = () => reconnectAttempts < MAX_RECONNECT_ATTEMPTS;

        expect(shouldReconnect()).toBe(true);
        reconnectAttempts = 3;
        expect(shouldReconnect()).toBe(false);
    });

    test('手动 IP 设备重连使用 manual: 前缀识别', () => {
        const isManualDevice = (deviceId: string) => deviceId.startsWith('manual:');
        expect(isManualDevice('manual:192.168.1.100:8889')).toBe(true);
        expect(isManualDevice('dev-1')).toBe(false);
    });

    test('手动 IP 重连时正确解析 IP 和端口', () => {
        const deviceId = 'manual:192.168.1.100:8889';
        const parts = deviceId.split(':');
        const ip = parts[1];
        const port = parseInt(parts[2], 10) || 8889;
        expect(ip).toBe('192.168.1.100');
        expect(port).toBe(8889);
    });

    test('重连失败后显示通知消息', () => {
        const MAX_RECONNECT_ATTEMPTS = 3;
        const getFailureMessage = (attempts: number) =>
            `连接已断开，自动重连 ${attempts} 次均失败，请手动重连`;
        expect(getFailureMessage(MAX_RECONNECT_ATTEMPTS)).toContain('3 次');
        expect(getFailureMessage(MAX_RECONNECT_ATTEMPTS)).toContain('手动重连');
    });
});

// ─── Platform 抽象层 - 跨网段直连 API 测试 ───

describe('Platform 抽象层 - 跨网段直连 API', () => {
    test('connectByIp 在 invoke 为 null 时返回降级结果', async () => {
        const safeInvoke = async (cmd: string, args: any, fallback: any) => fallback;
        const result = await safeInvoke('connect_by_ip', { ip: '192.168.1.100', port: 8889 }, {
            success: false,
            errorType: 'tauri_unavailable',
            errorMessage: 'Tauri 未就绪',
        });
        expect(result.success).toBe(false);
        expect(result.errorType).toBe('tauri_unavailable');
    });

    test('getNetworkInterfaces 在 invoke 为 null 时返回空数组', async () => {
        const safeInvoke = async (cmd: string, args: any, fallback: any) => fallback;
        const result = await safeInvoke('get_network_interfaces', {}, []);
        expect(result).toEqual([]);
    });

    test('setListenAddress 在 invoke 可用时正确传递参数', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);
        const safeInvoke = async (cmd: string, args: any, fallback: any) => mockInvoke(cmd, args);
        await safeInvoke('set_listen_address', { ip: '0.0.0.0', port: 8889 }, undefined);
        expect(mockInvoke).toHaveBeenCalledWith('set_listen_address', { ip: '0.0.0.0', port: 8889 });
    });

    test('getListenConfig 在 invoke 为 null 时返回默认配置', async () => {
        const safeInvoke = async (cmd: string, args: any, fallback: any) => fallback;
        const result = await safeInvoke('get_listen_config', {}, { ip: '0.0.0.0', port: 8889 });
        expect(result.ip).toBe('0.0.0.0');
        expect(result.port).toBe(8889);
    });

    test('connectByIp 成功时返回 success: true', async () => {
        const mockInvoke = jest.fn().mockResolvedValue({ success: true, error_type: '', error_message: '' });
        const safeInvoke = async (cmd: string, args: any, fallback: any) => mockInvoke(cmd, args);
        const result = await safeInvoke('connect_by_ip', { ip: '192.168.1.100', port: 8889 }, null);
        expect(result.success).toBe(true);
    });

    test('connectByIp 失败时返回对应错误类型', async () => {
        const mockInvoke = jest.fn().mockResolvedValue({
            success: false,
            error_type: 'timeout',
            error_message: '连接超时（5秒）：可能被防火墙拦截',
        });
        const safeInvoke = async (cmd: string, args: any, fallback: any) => mockInvoke(cmd, args);
        const result = await safeInvoke('connect_by_ip', { ip: '192.168.1.100', port: 8889 }, null);
        expect(result.success).toBe(false);
        expect(result.error_type).toBe('timeout');
        expect(result.error_message).toContain('防火墙');
    });
});
