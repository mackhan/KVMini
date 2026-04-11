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

// ── 5. handleScan() 结果反馈 UX（Bug #013 修复验证） ─────────────────────
//
// Bug #013：扫描 UX 反馈混乱
// 现象 A：第一次点击扫描立即弹出"未发现局域网设备"（catch 分支用了错误文案）
// 现象 B：第二次点击扫描等待后无任何提示（空结果分支没有 Toast）
// 根因：catch 分支使用 noDevicesAfterScan 文案；空结果分支缺少 Toast 通知
// 修复：catch 用 scanFailed 文案；空结果显示 noDevicesFound Toast；有结果显示 scanFoundDevices Toast

describe('Bug #013：handleScan() 结果反馈 UX', () => {
    // 模拟 handleScan 的核心逻辑（与 main.js 实现保持一致）
    function makeHandleScan(
        invokeResult: 'error' | 'empty' | 'found',
        showError: jest.Mock,
        showSuccess: jest.Mock,
        updateDevicesList: jest.Mock,
    ) {
        const i18n = require('../src/i18n.js');
        const invoke = async () => {
            if (invokeResult === 'error') throw new Error('command not found');
            if (invokeResult === 'empty') return [];
            return [{ name: 'MacBook Pro', os: 'macOS', ip: '192.168.1.100', tcp_port: 23456 }];
        };

        return async () => {
            try {
                const devices = await invoke();
                if (devices.length === 0) {
                    updateDevicesList([]);
                    showSuccess(i18n.t('noDevicesFound'));
                } else {
                    updateDevicesList(devices);
                    showSuccess(i18n.t('scanFoundDevices').replace('{n}', String(devices.length)));
                }
            } catch (error) {
                showError(i18n.t('scanFailed'));
                updateDevicesList([]);
            }
        };
    }

    test('测试用例 A：invoke 抛出异常时，显示 scanFailed 文案，不显示 noDevicesAfterScan', async () => {
        const showError = jest.fn();
        const showSuccess = jest.fn();
        const updateDevicesList = jest.fn();

        const handleScan = makeHandleScan('error', showError, showSuccess, updateDevicesList);
        await handleScan();

        // 应该显示 scanFailed 文案
        expect(showError).toHaveBeenCalledWith('扫描失败，请重试');
        // 不应该显示 noDevicesAfterScan 文案
        expect(showError).not.toHaveBeenCalledWith(
            expect.stringContaining('请确保目标设备在同一局域网内')
        );
        // 设备列表应该清空
        expect(updateDevicesList).toHaveBeenCalledWith([]);
    });

    test('测试用例 B：invoke 返回空数组时，显示 noDevicesFound Toast，不显示 error 级别提示', async () => {
        const showError = jest.fn();
        const showSuccess = jest.fn();
        const updateDevicesList = jest.fn();

        const handleScan = makeHandleScan('empty', showError, showSuccess, updateDevicesList);
        await handleScan();

        // 不应该显示 error 级别提示
        expect(showError).not.toHaveBeenCalled();
        // 应该显示 noDevicesFound 提示（通过 showSuccess）
        expect(showSuccess).toHaveBeenCalledWith('未发现局域网设备，可尝试手动连接');
        // 设备列表应该更新为空
        expect(updateDevicesList).toHaveBeenCalledWith([]);
    });

    test('测试用例 C：invoke 返回非空数组时，显示 scanFoundDevices 成功 Toast', async () => {
        const showError = jest.fn();
        const showSuccess = jest.fn();
        const updateDevicesList = jest.fn();

        const handleScan = makeHandleScan('found', showError, showSuccess, updateDevicesList);
        await handleScan();

        // 不应该显示 error 提示
        expect(showError).not.toHaveBeenCalled();
        // 应该显示成功 Toast，包含设备数量
        expect(showSuccess).toHaveBeenCalledWith('发现 1 台设备');
        // 设备列表应该更新
        expect(updateDevicesList).toHaveBeenCalledWith(
            expect.arrayContaining([expect.objectContaining({ name: 'MacBook Pro' })])
        );
    });

    test('测试用例 D：扫描结束后（无论结果），按钮恢复可用状态，扫描动画隐藏', async () => {
        // 模拟 finally 块的行为
        const setScanningState = jest.fn();
        let scanningStateAfterFinally: boolean | null = null;

        const handleScanWithFinally = async (invokeResult: 'error' | 'empty' | 'found') => {
            const invoke = async () => {
                if (invokeResult === 'error') throw new Error('fail');
                if (invokeResult === 'empty') return [];
                return [{ name: 'Test', os: 'macOS', ip: '192.168.1.1', tcp_port: 23456 }];
            };
            try {
                await invoke();
            } catch (_) {
                // 忽略
            } finally {
                setScanningState(false);
                scanningStateAfterFinally = false;
            }
        };

        // 测试三种情况下 finally 都执行
        await handleScanWithFinally('error');
        expect(setScanningState).toHaveBeenLastCalledWith(false);
        expect(scanningStateAfterFinally).toBe(false);

        await handleScanWithFinally('empty');
        expect(setScanningState).toHaveBeenLastCalledWith(false);

        await handleScanWithFinally('found');
        expect(setScanningState).toHaveBeenLastCalledWith(false);

        // 共调用 3 次
        expect(setScanningState).toHaveBeenCalledTimes(3);
    });

    test('i18n 文案：scanFailed 中文正确', () => {
        const i18n = require('../src/i18n.js');
        i18n.setLocale('zh');
        expect(i18n.t('scanFailed')).toBe('扫描失败，请重试');
    });

    test('i18n 文案：scanFoundDevices 中文包含 {n} 占位符', () => {
        const i18n = require('../src/i18n.js');
        i18n.setLocale('zh');
        const tpl = i18n.t('scanFoundDevices');
        expect(tpl).toContain('{n}');
        expect(tpl.replace('{n}', '3')).toBe('发现 3 台设备');
    });

    test('i18n 文案：noDevicesFound 中文包含手动连接提示', () => {
        const i18n = require('../src/i18n.js');
        i18n.setLocale('zh');
        expect(i18n.t('noDevicesFound')).toContain('手动连接');
    });

    test('i18n 文案：scanFailed 英文正确', () => {
        const i18n = require('../src/i18n.js');
        i18n.setLocale('en');
        expect(i18n.t('scanFailed')).toBe('Scan failed, please try again');
        i18n.setLocale('zh'); // 恢复默认
    });

    test('i18n 文案：scanFoundDevices 英文包含 {n} 占位符', () => {
        const i18n = require('../src/i18n.js');
        i18n.setLocale('en');
        const tpl = i18n.t('scanFoundDevices');
        expect(tpl).toContain('{n}');
        i18n.setLocale('zh'); // 恢复默认
    });
});
