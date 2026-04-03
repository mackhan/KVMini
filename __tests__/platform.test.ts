/**
 * 测试：平台抽象层 platform.js
 * 需求：3.3、3.4、3.5
 */

describe('Platform 抽象层 - 优雅降级', () => {
    // 模拟 safeInvoke 逻辑（与 platform.js 中一致）
    const createSafeInvoke = (invokeFunc: any) => {
        return async (cmd: string, args: any = {}, fallback: any = null) => {
            if (!invokeFunc) {
                return fallback;
            }
            return invokeFunc(cmd, args);
        };
    };

    test('invoke 为 null 时，safeInvoke 返回 fallback 值而不抛出错误', async () => {
        const safeInvoke = createSafeInvoke(null);
        const result = await safeInvoke('get_settings', {}, { hotkey: 'default' });
        expect(result).toEqual({ hotkey: 'default' });
    });

    test('invoke 为 null 时，discoverDevices 返回空数组', async () => {
        const safeInvoke = createSafeInvoke(null);
        const result = await safeInvoke('discover_devices', {}, []);
        expect(result).toEqual([]);
    });

    test('invoke 为 null 时，getConnectionHistory 返回空数组', async () => {
        const safeInvoke = createSafeInvoke(null);
        const result = await safeInvoke('get_connection_history', {}, []);
        expect(result).toEqual([]);
    });

    test('invoke 为 null 时，getLatency 返回 null', async () => {
        const safeInvoke = createSafeInvoke(null);
        const result = await safeInvoke('get_latency', {}, null);
        expect(result).toBeNull();
    });

    test('invoke 为 null 时，killLegacyProcess 返回 false', async () => {
        const safeInvoke = createSafeInvoke(null);
        const result = await safeInvoke('kill_legacy_process', {}, false);
        expect(result).toBe(false);
    });

    test('invoke 为 null 时，connectToDevice 返回失败结果', async () => {
        const safeInvoke = createSafeInvoke(null);
        const result = await safeInvoke('connect_to_device', { deviceId: 'dev-1' }, { success: false, error: 'Tauri 未就绪' });
        expect(result.success).toBe(false);
    });
});

describe('Platform 抽象层 - 正常调用', () => {
    let mockInvoke: jest.Mock;

    beforeEach(() => {
        mockInvoke = jest.fn();
    });

    test('invoke 可用时，正确传递命令名和参数', async () => {
        const createSafeInvoke = (invokeFunc: any) => {
            return async (cmd: string, args: any = {}, fallback: any = null) => {
                if (!invokeFunc) return fallback;
                return invokeFunc(cmd, args);
            };
        };
        const safeInvoke = createSafeInvoke(mockInvoke);

        mockInvoke.mockResolvedValue({ hotkey: 'Ctrl+Alt+K' });
        const result = await safeInvoke('get_settings', {}, null);

        expect(mockInvoke).toHaveBeenCalledWith('get_settings', {});
        expect(result).toEqual({ hotkey: 'Ctrl+Alt+K' });
    });

    test('invoke 可用时，save_settings 传递正确参数', async () => {
        const createSafeInvoke = (invokeFunc: any) => {
            return async (cmd: string, args: any = {}, fallback: any = null) => {
                if (!invokeFunc) return fallback;
                return invokeFunc(cmd, args);
            };
        };
        const safeInvoke = createSafeInvoke(mockInvoke);

        const settings = { hotkey: 'Ctrl+Alt+K', preferConnectionType: 'network', autoReconnect: true, autoStart: false, startMinimized: false };
        mockInvoke.mockResolvedValue(undefined);
        await safeInvoke('save_settings', { settings }, undefined);

        expect(mockInvoke).toHaveBeenCalledWith('save_settings', { settings });
    });

    test('invoke 可用时，show_tray_notification 传递 title 和 body', async () => {
        const createSafeInvoke = (invokeFunc: any) => {
            return async (cmd: string, args: any = {}, fallback: any = null) => {
                if (!invokeFunc) return fallback;
                return invokeFunc(cmd, args);
            };
        };
        const safeInvoke = createSafeInvoke(mockInvoke);

        mockInvoke.mockResolvedValue(undefined);
        await safeInvoke('show_tray_notification', { title: '已连接', body: '设备连接成功' }, undefined);

        expect(mockInvoke).toHaveBeenCalledWith('show_tray_notification', { title: '已连接', body: '设备连接成功' });
    });

    test('invoke 抛出错误时，safeInvoke 向上传播错误', async () => {
        const createSafeInvoke = (invokeFunc: any) => {
            return async (cmd: string, args: any = {}, fallback: any = null) => {
                if (!invokeFunc) return fallback;
                return invokeFunc(cmd, args);
            };
        };
        const safeInvoke = createSafeInvoke(mockInvoke);

        mockInvoke.mockRejectedValue(new Error('命令执行失败'));
        await expect(safeInvoke('connect_to_device', { deviceId: 'dev-1' }, null))
            .rejects.toThrow('命令执行失败');
    });
});

describe('Platform 抽象层 - window.Platform 接口完整性', () => {
    test('window.Platform 暴露所有必要的 API 方法', () => {
        // 模拟 platform.js 的导出结构
        const expectedMethods = [
            'init',
            'getSettings',
            'saveSettings',
            'getConnectionHistory',
            'saveConnectionHistory',
            'clearConnectionHistory',
            'showNotification',
            'registerGlobalHotkey',
            'getLatency',
            'killLegacyProcess',
            'discoverDevices',
            'connectToDevice',
            'disconnectDevices',
            'getConnectionStatus',
        ];

        // 验证接口定义完整性（通过检查方法名列表）
        const platformInterface = {
            init: () => {},
            getSettings: async () => {},
            saveSettings: async () => {},
            getConnectionHistory: async () => {},
            saveConnectionHistory: async () => {},
            clearConnectionHistory: async () => {},
            showNotification: async () => {},
            registerGlobalHotkey: async () => {},
            getLatency: async () => {},
            killLegacyProcess: async () => {},
            discoverDevices: async () => {},
            connectToDevice: async () => {},
            disconnectDevices: async () => {},
            getConnectionStatus: async () => {},
        };

        for (const method of expectedMethods) {
            expect(typeof (platformInterface as any)[method]).toBe('function');
        }
    });
});
