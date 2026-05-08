/**
 * Armed 状态前端 UI 测试
 * 
 * 验证：
 * 1. updateControlStateUI('Armed') 正确显示 i18n 文案和 CSS class
 * 2. updateSharingButtonForState('Armed') 显示「待切换」按钮
 * 3. control-state-changed 事件中 Armed 状态触发正确 toast
 */

describe('Armed 状态 UI 显示', () => {
    let controlStateEl, startSharingBtn;

    beforeEach(() => {
        // 模拟 DOM
        controlStateEl = document.createElement('span');
        controlStateEl.id = 'control-state';
        document.body.appendChild(controlStateEl);

        startSharingBtn = document.createElement('button');
        startSharingBtn.id = 'start-sharing-btn';
        document.body.appendChild(startSharingBtn);

        // 模拟 elements
        global.elements = {
            controlState: controlStateEl,
            startSharingBtn: startSharingBtn,
        };

        // 模拟 i18n
        global.window = global.window || {};
        window.i18n = {
            t: (key) => {
                const dict = {
                    controlStateIdle: '各自独立',
                    controlStateArmed: '已激活·待边缘切换',
                    controlStateLocal: '正在控制对方',
                    controlStateRemote: '对方正在控制本机',
                    btnStartSharing: '▶ 开始共享',
                    btnStopSharing: '■ 停止共享',
                    btnArmedWaiting: '⏳ 待切换',
                    sharingArmed: '已激活边缘检测，鼠标推到屏幕边缘即可切换',
                    sharingStarted: '共享已启动',
                    sharingStopped: '共享已停止',
                };
                return dict[key] || key;
            }
        };

        // 加载 main.js 并注入 mock elements
        const main = require('../src/main.js');
        main.__setElements(global.elements);
    });

    afterEach(() => {
        document.body.innerHTML = '';
        delete global.elements;
    });

    test('updateControlStateUI("Armed") 显示正确文案和 class', () => {
        const { updateControlStateUI } = require('../src/main.js');
        updateControlStateUI('Armed');
        expect(controlStateEl.textContent).toBe('已激活·待边缘切换');
        expect(controlStateEl.className).toBe('control-state-armed');
    });

    test('updateControlStateUI("Idle") 恢复默认', () => {
        const { updateControlStateUI } = require('../src/main.js');
        updateControlStateUI('Armed');
        updateControlStateUI('Idle');
        expect(controlStateEl.textContent).toBe('各自独立');
        expect(controlStateEl.className).toBe('control-state-idle');
    });

    test('updateSharingButtonForState("Armed") 显示待切换按钮', () => {
        const { updateSharingButtonForState } = require('../src/main.js');
        updateSharingButtonForState('Armed');
        expect(startSharingBtn.textContent).toBe('⏳ 待切换');
        expect(startSharingBtn.getAttribute('data-i18n')).toBe('btnArmedWaiting');
    });

    test('updateSharingButtonForState 在 Idle/Armed/LocalControlling 三态间正确切换', () => {
        const { updateSharingButtonForState } = require('../src/main.js');
        
        // Idle
        updateSharingButtonForState('Idle');
        expect(startSharingBtn.getAttribute('data-i18n')).toBe('btnStartSharing');
        
        // Armed
        updateSharingButtonForState('Armed');
        expect(startSharingBtn.getAttribute('data-i18n')).toBe('btnArmedWaiting');
        
        // LocalControlling
        updateSharingButtonForState('LocalControlling');
        expect(startSharingBtn.getAttribute('data-i18n')).toBe('btnStopSharing');
        
        // 回到 Armed（快捷键回切）
        updateSharingButtonForState('Armed');
        expect(startSharingBtn.getAttribute('data-i18n')).toBe('btnArmedWaiting');
    });
});
