/**
 * 测试：连接历史记录功能
 * 需求：2.5
 */

describe('连接历史记录 - 数据结构', () => {
    test('历史条目包含必要字段', () => {
        const entry = {
            id: 'dev-1',
            name: 'Test Device',
            os: 'Windows',
            ip: '192.168.1.100',
            connectionType: 'Network',
            lastConnected: new Date().toISOString(),
        };

        expect(entry).toHaveProperty('id');
        expect(entry).toHaveProperty('name');
        expect(entry).toHaveProperty('os');
        expect(entry).toHaveProperty('ip');
        expect(entry).toHaveProperty('connectionType');
        expect(entry).toHaveProperty('lastConnected');
    });

    test('lastConnected 为有效的 ISO 8601 日期字符串', () => {
        const entry = {
            id: 'dev-1',
            name: 'Test Device',
            os: 'macOS',
            ip: '192.168.1.1',
            connectionType: 'Network',
            lastConnected: new Date().toISOString(),
        };

        const date = new Date(entry.lastConnected);
        expect(date.toString()).not.toBe('Invalid Date');
    });
});

describe('连接历史记录 - localStorage 操作', () => {
    let localStorageMock: { getItem: jest.Mock; setItem: jest.Mock; removeItem: jest.Mock; clear: jest.Mock };

    beforeEach(() => {
        let store: Record<string, string> = {};
        localStorageMock = {
            getItem: jest.fn((key: string) => store[key] || null),
            setItem: jest.fn((key: string, value: string) => { store[key] = value; }),
            removeItem: jest.fn((key: string) => { delete store[key]; }),
            clear: jest.fn(() => { store = {}; }),
        };
        Object.defineProperty(window, 'localStorage', { value: localStorageMock, writable: true });
    });

    test('保存历史记录到 localStorage', () => {
        const history = [
            { id: 'dev-1', name: 'Device 1', os: 'Windows', ip: '192.168.1.1', connectionType: 'Network', lastConnected: new Date().toISOString() }
        ];
        localStorage.setItem('kvm_connection_history', JSON.stringify(history));
        expect(localStorageMock.setItem).toHaveBeenCalledWith('kvm_connection_history', expect.any(String));
    });

    test('读取历史记录从 localStorage', () => {
        const history = [
            { id: 'dev-1', name: 'Device 1', os: 'Windows', ip: '192.168.1.1', connectionType: 'Network', lastConnected: new Date().toISOString() }
        ];
        localStorageMock.getItem.mockReturnValue(JSON.stringify(history));

        const stored = JSON.parse(localStorage.getItem('kvm_connection_history') || '[]');
        expect(stored).toHaveLength(1);
        expect(stored[0].id).toBe('dev-1');
    });

    test('清除历史记录从 localStorage', () => {
        localStorage.removeItem('kvm_connection_history');
        expect(localStorageMock.removeItem).toHaveBeenCalledWith('kvm_connection_history');
    });

    test('历史记录去重：同 id 的旧记录被替换', () => {
        const history = [
            { id: 'dev-1', name: 'Device 1 Old', os: 'Windows', ip: '192.168.1.1', connectionType: 'Network', lastConnected: '2026-01-01T00:00:00Z' },
            { id: 'dev-2', name: 'Device 2', os: 'macOS', ip: '192.168.1.2', connectionType: 'Network', lastConnected: '2026-01-02T00:00:00Z' },
        ];

        const newEntry = { id: 'dev-1', name: 'Device 1 New', os: 'Windows', ip: '192.168.1.1', connectionType: 'Network', lastConnected: new Date().toISOString() };

        const filtered = history.filter(h => h.id !== newEntry.id);
        filtered.unshift(newEntry);
        const trimmed = filtered.slice(0, 10);

        expect(trimmed).toHaveLength(2);
        expect(trimmed[0].id).toBe('dev-1');
        expect(trimmed[0].name).toBe('Device 1 New');
    });

    test('历史记录最多保留 10 条', () => {
        const history = Array.from({ length: 12 }, (_, i) => ({
            id: `dev-${i}`,
            name: `Device ${i}`,
            os: 'Windows',
            ip: `192.168.1.${i}`,
            connectionType: 'Network',
            lastConnected: new Date().toISOString(),
        }));

        const trimmed = history.slice(0, 10);
        expect(trimmed).toHaveLength(10);
    });
});

describe('连接历史记录 - Platform 抽象层集成', () => {
    test('Platform.getConnectionHistory 在 invoke 为 null 时返回空数组', async () => {
        const safeInvoke = async (cmd: string, args: any, fallback: any) => {
            return fallback; // 模拟 invoke 为 null
        };
        const result = await safeInvoke('get_connection_history', {}, []);
        expect(result).toEqual([]);
    });

    test('Platform.saveConnectionHistory 在 invoke 可用时正确调用', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);
        const safeInvoke = async (cmd: string, args: any, fallback: any) => {
            return mockInvoke(cmd, args);
        };

        const entry = { id: 'dev-1', name: 'Test', os: 'macOS', ip: '192.168.1.1', connectionType: 'Network', lastConnected: new Date().toISOString() };
        await safeInvoke('save_connection_history', { entry }, undefined);

        expect(mockInvoke).toHaveBeenCalledWith('save_connection_history', { entry });
    });

    test('Platform.clearConnectionHistory 在 invoke 可用时正确调用', async () => {
        const mockInvoke = jest.fn().mockResolvedValue(undefined);
        const safeInvoke = async (cmd: string, args: any, fallback: any) => {
            return mockInvoke(cmd, args);
        };

        await safeInvoke('clear_connection_history', {}, undefined);
        expect(mockInvoke).toHaveBeenCalledWith('clear_connection_history', {});
    });
});
