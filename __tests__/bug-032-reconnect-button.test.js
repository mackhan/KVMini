/**
 * BUG-032 回归测试
 * 
 * 现象：连接成功后按钮正确显示"断开"，但后续 reconnecting 事件导致按钮错误切换为"重连"
 * 根因：reconnecting 事件处理器 (L454-459) 无条件调用 updateConnectionButtons(false)，
 *       即使 appState.connected 实际上仍为 true（心跳波动触发 reconnecting 但连接未真正断开）
 * 修复：updateConnectionButtons 内部读取 appState.connected 作为单一真相源，忽略外部参数
 */

describe('BUG-032: reconnecting 事件不应导致已连接状态下按钮误显示', () => {
    let disconnectBtn, reconnectBtn;

    beforeEach(() => {
        // 模拟 DOM 元素
        disconnectBtn = document.createElement('button');
        disconnectBtn.id = 'disconnect-btn';
        disconnectBtn.classList.add('hidden');
        document.body.appendChild(disconnectBtn);

        reconnectBtn = document.createElement('button');
        reconnectBtn.id = 'reconnect-btn';
        reconnectBtn.classList.add('hidden');
        document.body.appendChild(reconnectBtn);

        // 模拟 elements 对象
        global.elements = {
            disconnectBtn,
            reconnectBtn,
        };

        // 模拟 appState
        global.appState = {
            connected: true,
        };

        // 加载 main.js 并注入 mock
        const main = require('../src/main.js');
        main.__setElements(global.elements);
        main.__setAppState(global.appState);
    });

    afterEach(() => {
        document.body.innerHTML = '';
        delete global.elements;
        delete global.appState;
    });

    test('updateConnectionButtons 应以 appState.connected 为准，忽略参数', () => {
        // 加载被测函数 —— updateConnectionButtons 将从 appState.connected 读取状态
        const { updateConnectionButtons } = require('../src/main.js');

        // appState.connected = true 时，即使传入 false 也应该显示断开按钮
        appState.connected = true;
        updateConnectionButtons(false); // 模拟 reconnecting 事件传入 false

        expect(disconnectBtn.classList.contains('hidden')).toBe(false);
        expect(reconnectBtn.classList.contains('hidden')).toBe(true);
    });

    test('appState.connected = false 时应正确显示重连按钮', () => {
        const { updateConnectionButtons } = require('../src/main.js');

        appState.connected = false;
        updateConnectionButtons(false);

        expect(disconnectBtn.classList.contains('hidden')).toBe(true);
        expect(reconnectBtn.classList.contains('hidden')).toBe(false);
    });

    test('多次 reconnecting 事件后按钮状态不变（connected=true 场景）', () => {
        const { updateConnectionButtons } = require('../src/main.js');

        appState.connected = true;

        // 模拟多次心跳波动引发的 reconnecting 事件
        for (let i = 0; i < 5; i++) {
            updateConnectionButtons(false);
        }

        // 按钮必须仍显示"断开连接"
        expect(disconnectBtn.classList.contains('hidden')).toBe(false);
        expect(disconnectBtn.disabled).toBe(false);
        expect(reconnectBtn.classList.contains('hidden')).toBe(true);
    });

    test('真正断连后按钮应切换为重连', () => {
        const { updateConnectionButtons } = require('../src/main.js');

        // 先连接状态
        appState.connected = true;
        updateConnectionButtons(true);
        expect(disconnectBtn.classList.contains('hidden')).toBe(false);

        // 真正断连
        appState.connected = false;
        updateConnectionButtons(false);
        expect(disconnectBtn.classList.contains('hidden')).toBe(true);
        expect(reconnectBtn.classList.contains('hidden')).toBe(false);
    });
});
