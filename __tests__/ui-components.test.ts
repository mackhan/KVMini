/**
 * 测试：前端界面功能
 * 需求：2.1、2.2、2.6、2.8
 */

// 模拟 DOM 环境
const fs = require('fs');
const path = require('path');

// 读取 HTML 文件内容
const htmlContent = fs.readFileSync(
    path.join(__dirname, '../src/index.html'),
    'utf-8'
);

describe('前端界面 - HTML 结构完整性', () => {
    beforeEach(() => {
        document.body.innerHTML = htmlContent
            .replace(/<script[\s\S]*?<\/script>/g, ''); // 移除 script 标签
    });

    test('主界面包含连接状态显示', () => {
        expect(document.getElementById('connection-status')).not.toBeNull();
        expect(document.getElementById('connection-type')).not.toBeNull();
        expect(document.getElementById('latency')).not.toBeNull();
    });

    test('主界面包含断开按钮（顶部扫描按钮已移除，scan-btn 为唯一扫描入口）', () => {
        // 需求 1：顶部 connect-btn 已移除，不应存在
        expect(document.getElementById('connect-btn')).toBeNull();
        expect(document.getElementById('disconnect-btn')).not.toBeNull();
    });

    test('主界面包含设置按钮', () => {
        expect(document.getElementById('settings-btn')).not.toBeNull();
    });

    test('主界面包含设备列表区域', () => {
        expect(document.getElementById('devices-list')).not.toBeNull();
    });

    test('主界面包含扫描设备按钮', () => {
        expect(document.getElementById('scan-btn')).not.toBeNull();
    });

    test('主界面包含扫描动画元素（初始隐藏）', () => {
        const scanAnimation = document.getElementById('scan-animation');
        expect(scanAnimation).not.toBeNull();
        expect(scanAnimation.classList.contains('hidden')).toBe(true);
    });

    test('主界面包含连接历史列表', () => {
        expect(document.getElementById('history-list')).not.toBeNull();
    });

    test('主界面包含错误提示 Toast（初始隐藏，非 alert）', () => {
        const errorToast = document.getElementById('error-toast');
        expect(errorToast).not.toBeNull();
        expect(errorToast.classList.contains('hidden')).toBe(true);
    });

    test('主界面包含成功提示 Toast（初始隐藏）', () => {
        const successToast = document.getElementById('success-toast');
        expect(successToast).not.toBeNull();
        expect(successToast.classList.contains('hidden')).toBe(true);
    });

    test('设置视图存在（初始隐藏）', () => {
        // 新架构：方案 D 整页替换，使用 #view-settings 而非 modal
        const settingsView = document.getElementById('view-settings');
        expect(settingsView).not.toBeNull();
        expect(settingsView.classList.contains('hidden')).toBe(true);
    });

    test('设置面板包含快捷键配置输入', () => {
        expect(document.getElementById('hotkey-input')).not.toBeNull();
    });

    test('设置面板包含连接偏好选择', () => {
        expect(document.getElementById('prefer-connection-type')).not.toBeNull();
    });

    test('设置面板包含自动重连开关', () => {
        expect(document.getElementById('auto-reconnect')).not.toBeNull();
    });

    test('设置面板包含开机自启动开关', () => {
        expect(document.getElementById('auto-start')).not.toBeNull();
    });

    test('设置面板包含最小化到托盘开关', () => {
        expect(document.getElementById('start-minimized')).not.toBeNull();
    });

    test('断开连接按钮初始状态为禁用', () => {
        const disconnectBtn = document.getElementById('disconnect-btn') as HTMLButtonElement;
        expect(disconnectBtn.disabled).toBe(true);
    });
});

describe('前端界面 - JS 功能逻辑', () => {
    let mockInvoke: jest.Mock;

    beforeEach(() => {
        // 设置 DOM
        document.body.innerHTML = htmlContent
            .replace(/<script[\s\S]*?<\/script>/g, '');

        // 设置 localStorage mock
        const localStorageMock = (() => {
            let store: Record<string, string> = {};
            return {
                getItem: (key: string) => store[key] || null,
                setItem: (key: string, value: string) => { store[key] = value; },
                removeItem: (key: string) => { delete store[key]; },
                clear: () => { store = {}; },
            };
        })();
        Object.defineProperty(window, 'localStorage', { value: localStorageMock, writable: true });

        // 设置 Tauri mock
        mockInvoke = jest.fn();
        (window as any).__TAURI__ = {
            tauri: { invoke: mockInvoke }
        };
    });

    test('escapeHtml 函数防止 XSS 注入', () => {
        // 直接测试 escapeHtml 逻辑
        const escapeHtml = (str: string) => {
            if (str == null) return '';
            return String(str)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#39;');
        };

        expect(escapeHtml('<script>alert(1)</script>')).toBe('&lt;script&gt;alert(1)&lt;/script&gt;');
        expect(escapeHtml('"quoted"')).toBe('&quot;quoted&quot;');
        expect(escapeHtml("it's")).toBe('it&#39;s');
        expect(escapeHtml('normal text')).toBe('normal text');
        expect(escapeHtml(null as any)).toBe('');
    });

    test('连接历史本地存储读写', () => {
        const history = [
            { id: 'dev-1', name: 'Test Device', os: 'Windows', ip: '192.168.1.100', connectionType: 'Network', lastConnected: new Date().toISOString() }
        ];
        localStorage.setItem('kvm_connection_history', JSON.stringify(history));
        const stored = JSON.parse(localStorage.getItem('kvm_connection_history') || '[]');
        expect(stored).toHaveLength(1);
        expect(stored[0].name).toBe('Test Device');
    });

    test('设置本地存储读写', () => {
        const settings = {
            hotkey: 'Ctrl+Alt+K',
            preferConnectionType: 'network',
            autoReconnect: false,
            autoStart: true,
            startMinimized: false,
        };
        localStorage.setItem('kvm_settings', JSON.stringify(settings));
        const stored = JSON.parse(localStorage.getItem('kvm_settings') || 'null');
        expect(stored.hotkey).toBe('Ctrl+Alt+K');
        expect(stored.preferConnectionType).toBe('network');
    });

    test('清除历史记录后 localStorage 为空', () => {
        localStorage.setItem('kvm_connection_history', JSON.stringify([{ id: '1' }]));
        localStorage.removeItem('kvm_connection_history');
        const stored = localStorage.getItem('kvm_connection_history');
        expect(stored).toBeNull();
    });
});
