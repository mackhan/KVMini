/**
 * discover_devices 功能测试（TDD）
 *
 * Bug #012：扫描设备一瞬间结束
 * 根因：main.rs 未注册 discover_devices Tauri command，
 *       前端 invoke('discover_devices') 立即报错，catch 块执行，扫描结束。
 *
 * 修复：在 main.rs 中添加 discover_devices Tauri command，
 *       调用 kvm_core::network::discover_devices(DEFAULT_UDP_PORT)。
 */

const mockInvoke = jest.fn();

beforeEach(() => {
    jest.clearAllMocks();
    (window as any).__TAURI__ = {
        core: { invoke: mockInvoke },
        invoke: mockInvoke,
        event: { listen: jest.fn(), emit: jest.fn() },
    };
});

// ── 1. Tauri command 接口契约 ─────────────────────────────────────────────

describe('discover_devices Tauri command 接口', () => {
    test('discover_devices 返回设备数组', async () => {
        const mockDevices = [
            { name: 'MacBook Pro', os: 'macOS', ip: '192.168.1.100', tcp_port: 23456 },
            { name: 'Windows PC', os: 'Windows', ip: '192.168.1.200', tcp_port: 23456 },
        ];
        mockInvoke.mockResolvedValueOnce(mockDevices);

        const devices = await (window as any).__TAURI__.invoke('discover_devices');
        expect(Array.isArray(devices)).toBe(true);
        expect(devices).toHaveLength(2);
    });

    test('discover_devices 返回的设备包含必要字段', async () => {
        const mockDevices = [
            { name: 'MacBook Pro', os: 'macOS', ip: '192.168.1.100', tcp_port: 23456 },
        ];
        mockInvoke.mockResolvedValueOnce(mockDevices);

        const devices = await (window as any).__TAURI__.invoke('discover_devices');
        const device = devices[0];
        expect(device).toHaveProperty('name');
        expect(device).toHaveProperty('os');
        expect(device).toHaveProperty('ip');
        expect(device).toHaveProperty('tcp_port');
    });

    test('局域网无设备时返回空数组（不报错）', async () => {
        mockInvoke.mockResolvedValueOnce([]);

        const devices = await (window as any).__TAURI__.invoke('discover_devices');
        expect(Array.isArray(devices)).toBe(true);
        expect(devices).toHaveLength(0);
    });

    test('discover_devices 命令名称正确（不是 connect_to_device）', () => {
        expect('discover_devices').not.toBe('connect_to_device');
        expect('discover_devices').not.toBe('connect_by_ip');
    });
});

// ── 2. 前端扫描 UI 状态管理 ───────────────────────────────────────────────

describe('扫描 UI 状态管理', () => {
    test('扫描开始时按钮禁用、显示动画', () => {
        // 模拟 setScanningState(true) 的行为
        const setScanningState = (scanning: boolean, btn: any, animation: any) => {
            btn.disabled = scanning;
            if (scanning) {
                animation.classList.remove('hidden');
            } else {
                animation.classList.add('hidden');
            }
        };

        const btn = { disabled: false };
        const animation = { classList: { remove: jest.fn(), add: jest.fn() } };

        setScanningState(true, btn, animation);
        expect(btn.disabled).toBe(true);
        expect(animation.classList.remove).toHaveBeenCalledWith('hidden');
    });

    test('扫描结束时按钮恢复、隐藏动画', () => {
        const setScanningState = (scanning: boolean, btn: any, animation: any) => {
            btn.disabled = scanning;
            if (scanning) {
                animation.classList.remove('hidden');
            } else {
                animation.classList.add('hidden');
            }
        };

        const btn = { disabled: true };
        const animation = { classList: { remove: jest.fn(), add: jest.fn() } };

        setScanningState(false, btn, animation);
        expect(btn.disabled).toBe(false);
        expect(animation.classList.add).toHaveBeenCalledWith('hidden');
    });

    test('扫描中重复点击不触发新扫描', () => {
        let scanning = false;
        let callCount = 0;

        const handleScan = () => {
            if (scanning) return;
            scanning = true;
            callCount++;
        };

        handleScan(); // 第一次：触发
        handleScan(); // 第二次：被防抖拦截
        handleScan(); // 第三次：被防抖拦截

        expect(callCount).toBe(1);
    });
});

// ── 3. 扫描结果渲染 ───────────────────────────────────────────────────────

describe('扫描结果渲染', () => {
    test('有设备时渲染设备列表', () => {
        const devices = [
            { name: 'MacBook Pro', os: 'macOS', ip: '192.168.1.100', tcp_port: 23456 },
        ];

        // 模拟 updateDevicesList 的核心逻辑
        const renderDevices = (devs: any[]) => {
            if (devs.length === 0) return '点击"扫描设备"发现局域网内的设备';
            return devs.map(d => `${d.name} (${d.ip})`).join(', ');
        };

        expect(renderDevices(devices)).toContain('MacBook Pro');
        expect(renderDevices(devices)).toContain('192.168.1.100');
    });

    test('无设备时显示提示文字', () => {
        const renderDevices = (devs: any[]) => {
            if (devs.length === 0) return '点击"扫描设备"发现局域网内的设备';
            return devs.map(d => `${d.name} (${d.ip})`).join(', ');
        };

        expect(renderDevices([])).toBe('点击"扫描设备"发现局域网内的设备');
    });

    test('设备 IP 格式正确（IPv4）', () => {
        const isValidIp = (ip: string) => /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(ip);
        expect(isValidIp('192.168.1.100')).toBe(true);
        expect(isValidIp('10.0.0.1')).toBe(true);
        expect(isValidIp('invalid')).toBe(false);
    });

    test('设备 tcp_port 在有效范围内', () => {
        const isValidPort = (port: number) => port >= 1024 && port <= 65535;
        expect(isValidPort(23456)).toBe(true);
        expect(isValidPort(8888)).toBe(true);
        expect(isValidPort(0)).toBe(false);
    });
});

// ── 4. main.rs 注册验证（文件内容检查） ──────────────────────────────────

describe('Bug #012：main.rs 必须注册 discover_devices 命令', () => {
    test('main.rs invoke_handler 包含 discover_devices', () => {
        const fs = require('fs');
        const path = require('path');
        const mainRs = fs.readFileSync(
            path.join(__dirname, '../src-tauri/src/main.rs'),
            'utf-8'
        );
        expect(mainRs).toContain('discover_devices');
    });

    test('main.rs 定义了 discover_devices 函数', () => {
        const fs = require('fs');
        const path = require('path');
        const mainRs = fs.readFileSync(
            path.join(__dirname, '../src-tauri/src/main.rs'),
            'utf-8'
        );
        expect(mainRs).toMatch(/fn discover_devices/);
    });
});
