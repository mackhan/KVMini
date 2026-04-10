/**
 * ui-module.test.ts
 * 测试 src/ui.js 中的 UI 工具函数
 */

beforeAll(() => {
    (window as any).i18n = {
        t: (key: string) => {
            const map: Record<string, string> = {
                statusConnected: '已连接',
                statusConnecting: '连接中',
                statusDisconnected: '未连接',
                controlStateLocal: '本机控制',
                controlStateRemote: '远端控制',
                controlStateIdle: '空闲',
            };
            return map[key] || key;
        },
    };
    require('../src/ui.js');
});

describe('ui.js - escapeHtml', () => {
    test('应转义 < 和 >', () => {
        expect((window as any).UIModule.escapeHtml('<script>')).toBe('&lt;script&gt;');
    });

    test('应转义双引号', () => {
        expect((window as any).UIModule.escapeHtml('"hello"')).toBe('&quot;hello&quot;');
    });

    test('应转义单引号', () => {
        expect((window as any).UIModule.escapeHtml("it's")).toBe("it&#39;s");
    });

    test('应转义 & 符号', () => {
        expect((window as any).UIModule.escapeHtml('a&b')).toBe('a&amp;b');
    });

    test('null 应返回空字符串', () => {
        expect((window as any).UIModule.escapeHtml(null)).toBe('');
    });

    test('undefined 应返回空字符串', () => {
        expect((window as any).UIModule.escapeHtml(undefined)).toBe('');
    });

    test('普通字符串不应被修改', () => {
        expect((window as any).UIModule.escapeHtml('hello world')).toBe('hello world');
    });

    test('数字应转为字符串', () => {
        expect((window as any).UIModule.escapeHtml(42 as any)).toBe('42');
    });
});

describe('ui.js - setConnectionStatus', () => {
    let statusEl: HTMLElement;

    beforeEach(() => {
        document.body.innerHTML = '<span id="connection-status"></span>';
        statusEl = document.getElementById('connection-status')!;
    });

    test('connected 状态应设置正确的 class 和文本', () => {
        (window as any).UIModule.setConnectionStatus('connected', statusEl);
        expect(statusEl.className).toBe('status-connected');
        expect(statusEl.textContent).toBe('已连接');
    });

    test('connecting 状态应设置正确的 class 和文本', () => {
        (window as any).UIModule.setConnectionStatus('connecting', statusEl);
        expect(statusEl.className).toBe('status-connecting');
        expect(statusEl.textContent).toBe('连接中');
    });

    test('disconnected 状态应设置正确的 class 和文本', () => {
        (window as any).UIModule.setConnectionStatus('disconnected', statusEl);
        expect(statusEl.className).toBe('status-disconnected');
        expect(statusEl.textContent).toBe('未连接');
    });

    test('未知状态应降级为 disconnected', () => {
        (window as any).UIModule.setConnectionStatus('unknown', statusEl);
        expect(statusEl.className).toBe('status-disconnected');
    });
});

describe('ui.js - updateControlStateUI', () => {
    let el: HTMLElement;

    beforeEach(() => {
        document.body.innerHTML = '<span id="control-state"></span>';
        el = document.getElementById('control-state')!;
    });

    test('LocalControlling 应设置本机控制状态', () => {
        (window as any).UIModule.updateControlStateUI('LocalControlling', el);
        expect(el.className).toBe('control-state-local');
        expect(el.textContent).toBe('本机控制');
    });

    test('RemoteControlling 应设置远端控制状态', () => {
        (window as any).UIModule.updateControlStateUI('RemoteControlling', el);
        expect(el.className).toBe('control-state-remote');
        expect(el.textContent).toBe('远端控制');
    });

    test('Idle 应设置空闲状态', () => {
        (window as any).UIModule.updateControlStateUI('Idle', el);
        expect(el.className).toBe('control-state-idle');
        expect(el.textContent).toBe('空闲');
    });
});
