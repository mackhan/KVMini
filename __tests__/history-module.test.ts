/**
 * history-module.test.ts
 * 测试 src/history.js 中的连接历史管理函数
 */

beforeAll(() => {
    (window as any).i18n = {
        t: (key: string) => key,
    };
    require('../src/history.js');
});

beforeEach(() => {
    localStorage.clear();
});

describe('history.js - getHistoryFromStorage', () => {
    test('空存储时应返回空数组', () => {
        expect((window as any).HistoryModule.getHistoryFromStorage()).toEqual([]);
    });

    test('存储损坏时应返回空数组', () => {
        localStorage.setItem('kvmini_connection_history', 'invalid-json');
        expect((window as any).HistoryModule.getHistoryFromStorage()).toEqual([]);
    });

    test('有数据时应返回解析后的数组', () => {
        const data = [{ id: 'test', ip: '1.2.3.4' }];
        localStorage.setItem('kvmini_connection_history', JSON.stringify(data));
        expect((window as any).HistoryModule.getHistoryFromStorage()).toEqual(data);
    });
});

describe('history.js - saveToStorage', () => {
    test('应保存条目到 localStorage', () => {
        (window as any).HistoryModule.saveToStorage({ id: 'manual:1.2.3.4:23456', ip: '1.2.3.4', port: 23456 });
        const history = (window as any).HistoryModule.getHistoryFromStorage();
        expect(history).toHaveLength(1);
        expect(history[0].ip).toBe('1.2.3.4');
    });

    test('相同 id 应去重（保留最新）', () => {
        (window as any).HistoryModule.saveToStorage({ id: 'manual:1.2.3.4:23456', ip: '1.2.3.4', name: 'first' });
        (window as any).HistoryModule.saveToStorage({ id: 'manual:1.2.3.4:23456', ip: '1.2.3.4', name: 'second' });
        const history = (window as any).HistoryModule.getHistoryFromStorage();
        expect(history.filter((h: any) => h.id === 'manual:1.2.3.4:23456')).toHaveLength(1);
        expect(history[0].name).toBe('second');
    });

    test('最新条目应排在最前', () => {
        (window as any).HistoryModule.saveToStorage({ id: 'a', ip: '1.1.1.1' });
        (window as any).HistoryModule.saveToStorage({ id: 'b', ip: '2.2.2.2' });
        const history = (window as any).HistoryModule.getHistoryFromStorage();
        expect(history[0].id).toBe('b');
    });

    test('超过 20 条时应截断', () => {
        for (let i = 0; i < 25; i++) {
            (window as any).HistoryModule.saveToStorage({ id: `item-${i}`, ip: `1.1.1.${i}` });
        }
        const history = (window as any).HistoryModule.getHistoryFromStorage();
        expect(history).toHaveLength(20);
    });
});

describe('history.js - clearStorage', () => {
    test('应清空所有历史记录', () => {
        (window as any).HistoryModule.saveToStorage({ id: 'test', ip: '1.1.1.1' });
        (window as any).HistoryModule.clearStorage();
        expect((window as any).HistoryModule.getHistoryFromStorage()).toEqual([]);
    });
});
