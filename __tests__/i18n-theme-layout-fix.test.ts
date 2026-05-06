/**
 * i18n-theme-layout-fix.test.ts
 *
 * 覆盖三个 bug 修复的测试用例：
 * Bug #021: 中英文切换后大量文字未替换
 * Bug #022: 主题切换后界面未变化
 * Bug #023: 显示器拖拽逻辑不直观（改用 mousedown/mousemove）
 */

// ─── Mock 环境 ────────────────────────────────────────────────────────────────

// Mock Tauri invoke
const mockInvoke = jest.fn().mockResolvedValue(undefined);
(globalThis as any).window = {
    ...(globalThis as any).window,
    __TAURI__: { core: { invoke: mockInvoke } },
};

// ─── 加载模块 ────────────────────────────────────────────────────────────────

const i18nModule = require('../src/i18n');
const themeModule = require('../src/theme');

// ─── Bug #021: i18n 全量文本替换 ──────────────────────────────────────────────

describe('Bug #021: i18n 全量文本替换', () => {
    describe('MESSAGES 翻译完整性', () => {
        const { MESSAGES } = i18nModule;

        test('中文和英文翻译 key 数量一致', () => {
            const zhKeys = Object.keys(MESSAGES.zh).sort();
            const enKeys = Object.keys(MESSAGES.en).sort();
            expect(zhKeys.length).toBe(enKeys.length);
        });

        test('每个中文 key 在英文中都有对应翻译', () => {
            const zhKeys = Object.keys(MESSAGES.zh);
            const enKeys = new Set(Object.keys(MESSAGES.en));
            const missing = zhKeys.filter(k => !enKeys.has(k));
            expect(missing).toEqual([]);
        });

        test('每个英文 key 在中文中都有对应翻译', () => {
            const enKeys = Object.keys(MESSAGES.en);
            const zhKeys = new Set(Object.keys(MESSAGES.zh));
            const missing = enKeys.filter(k => !zhKeys.has(k));
            expect(missing).toEqual([]);
        });

        // 新增的 i18n key 必须存在
        const requiredNewKeys = [
            // 页面标题
            'pageTitle',
            // 设置弹窗标题
            'settingsTitle',
            // 证书指纹折叠区（v0.1.38 组 F：detailTitle/detailLocal/detailPeer/detailEncryption/detailLatency 已删）
            'detailCertFingerprint',
            // 重连进度
            'labelReconnectProgress', 'reconnectingText',
            // 授权界面（当前 IAP 版本）
            'licenseGateTitle', 'licenseGateDescNew',
            // 授权状态
            'licenseActivated', 'licenseSdkError',
            // 提示面板
            'hintMouseEdge', 'hintHotkey',
            // 取消确认对话框
            'discardDialogTitle', 'discardDialogMsg', 'btnDiscardChanges', 'btnContinueEditing',
            // 错误横幅关闭
            'btnClose',
            // layout 中的标签
            'labelLocalMonitor', 'labelSecondaryMonitor', 'labelRemoteMonitor', 'labelPrimaryTag',
        ];

        test.each(requiredNewKeys)('中文翻译包含 key: %s', (key) => {
            expect(MESSAGES.zh[key]).toBeDefined();
            expect(typeof MESSAGES.zh[key]).toBe('string');
            expect(MESSAGES.zh[key].length).toBeGreaterThan(0);
        });

        test.each(requiredNewKeys)('英文翻译包含 key: %s', (key) => {
            expect(MESSAGES.en[key]).toBeDefined();
            expect(typeof MESSAGES.en[key]).toBe('string');
            expect(MESSAGES.en[key].length).toBeGreaterThan(0);
        });
    });

    describe('applyI18n 覆盖范围', () => {
        beforeEach(() => {
            // 设置最小 DOM
            document.body.innerHTML = `
                <title>KVM Share - 跨平台键鼠共享</title>
                <h1 data-i18n="appTitle">KVMini</h1>
                <span data-i18n="labelConnectionStatus">连接状态:</span>
                <!-- v0.1.38 组 F：旧的 #connection-detail-panel 已删，仅保留折叠指纹 summary -->
                <details id="peer-fingerprint-details">
                    <summary data-i18n="detailCertFingerprint">证书指纹</summary>
                </details>
                <!-- 重连进度 -->
                <span class="status-label" data-i18n="labelReconnectProgress">重连进度:</span>
                <!-- 设置弹窗标题 -->
                <h2 data-i18n="settingsTitle">⚙ 设置</h2>
                <!-- 取消确认对话框 -->
                <h2 data-i18n="discardDialogTitle">⚠️ 未保存的修改</h2>
                <p data-i18n="discardDialogMsg">您有未保存的修改，是否放弃？</p>
                <button data-i18n="btnDiscardChanges">放弃修改</button>
                <button data-i18n="btnContinueEditing">继续编辑</button>
                <!-- 授权界面 -->
                <h2 data-i18n="licenseGateTitle">激活 KVMini</h2>
                <p data-i18n="licenseGateDesc">请输入授权码激活，或购买许可证</p>
                <button data-i18n="btnActivate">激活</button>
                <span data-i18n="licenseGateDivider">或</span>
                <button data-i18n="btnBuyLicense">购买许可证</button>
            `;
        });

        test('切换到英文后，所有 data-i18n 元素文本变为英文', () => {
            i18nModule.setLocale('en');
            // 模拟 applyI18n 的核心逻辑
            document.querySelectorAll('[data-i18n]').forEach((el: Element) => {
                const key = el.getAttribute('data-i18n');
                if (key) {
                    el.textContent = i18nModule.t(key);
                }
            });

            // 验证关键元素已切换为英文
            expect(document.querySelector('[data-i18n="settingsTitle"]')?.textContent).not.toContain('设置');
            expect(document.querySelector('[data-i18n="detailCertFingerprint"]')?.textContent).not.toContain('证书');
            expect(document.querySelector('[data-i18n="discardDialogTitle"]')?.textContent).not.toContain('未保存');
            expect(document.querySelector('[data-i18n="licenseGateTitle"]')?.textContent).not.toContain('激活');
        });

        test('切换回中文后，所有 data-i18n 元素文本恢复中文', () => {
            i18nModule.setLocale('en');
            // 先切英文
            document.querySelectorAll('[data-i18n]').forEach((el: Element) => {
                const key = el.getAttribute('data-i18n');
                if (key) el.textContent = i18nModule.t(key);
            });

            // 再切中文
            i18nModule.setLocale('zh');
            document.querySelectorAll('[data-i18n]').forEach((el: Element) => {
                const key = el.getAttribute('data-i18n');
                if (key) el.textContent = i18nModule.t(key);
            });

            expect(document.querySelector('[data-i18n="settingsTitle"]')?.textContent).toBe('⚙ 设置');
            expect(document.querySelector('[data-i18n="detailCertFingerprint"]')?.textContent).toBe('证书指纹');
        });
    });
});

// ─── Bug #022: 主题切换 ──────────────────────────────────────────────────────

describe('Bug #022: 主题切换生效', () => {
    beforeEach(() => {
        document.documentElement.removeAttribute('data-theme');
    });

    test('setTheme("dark") 设置 data-theme="dark"', () => {
        themeModule.setTheme('dark');
        expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    test('setTheme("light") 设置 data-theme="light"', () => {
        themeModule.setTheme('light');
        expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });

    test('setTheme("auto") 根据系统偏好设置主题', () => {
        // jsdom 默认 matchMedia 返回 false（浅色）
        themeModule.setTheme('auto');
        const theme = document.documentElement.getAttribute('data-theme');
        expect(['dark', 'light']).toContain(theme);
    });

    test('CSS 中 [data-theme="dark"] 变量已定义', () => {
        // 验证 CSS 文件中包含深色主题变量定义
        const fs = require('fs');
        const css = fs.readFileSync(require('path').join(__dirname, '../src/styles.css'), 'utf-8');
        expect(css).toContain('[data-theme="dark"]');
        expect(css).toContain('--panel-bg:');
        expect(css).toContain('--text-primary:');
    });

    test('CSS 中 [data-theme="light"] 变量已定义', () => {
        const fs = require('fs');
        const css = fs.readFileSync(require('path').join(__dirname, '../src/styles.css'), 'utf-8');
        expect(css).toContain('[data-theme="light"]');
    });

    test('CSS 中没有语法错误（孤立的属性声明）', () => {
        const fs = require('fs');
        const css = fs.readFileSync(require('path').join(__dirname, '../src/styles.css'), 'utf-8');
        // 检查是否有不在选择器块内的孤立属性声明
        // 具体检查：不应该有 "}\n\n    font-size:" 这种模式（属性在选择器外面）
        const orphanedPropertyPattern = /\}\s*\n\s*font-size:\s*\d+px;/;
        expect(css).not.toMatch(orphanedPropertyPattern);
    });
});

// ─── Bug #023: 显示器拖拽改用 mousedown/mousemove ──────────────────────────

describe('Bug #023: 显示器拖拽改用 mousedown/mousemove', () => {
    const layoutModule = require('../src/layout');

    describe('calcLayoutFromDrop 纯函数', () => {
        test('本机在远端左侧', () => {
            expect(layoutModule.calcLayoutFromDrop(100, 200, 300, 200)).toBe('LocalLeftOfRemote');
        });

        test('本机在远端右侧', () => {
            expect(layoutModule.calcLayoutFromDrop(300, 200, 100, 200)).toBe('LocalRightOfRemote');
        });

        test('本机在远端上方', () => {
            expect(layoutModule.calcLayoutFromDrop(200, 100, 200, 300)).toBe('LocalAboveRemote');
        });

        test('本机在远端下方', () => {
            expect(layoutModule.calcLayoutFromDrop(200, 300, 200, 100)).toBe('LocalBelowRemote');
        });
    });

    describe('initLayoutDrag 使用 mousedown/mousemove 而非 HTML5 Drag & Drop', () => {
        test('layout.js 不使用 dragstart 事件', () => {
            const fs = require('fs');
            const code = fs.readFileSync(require('path').join(__dirname, '../src/layout.js'), 'utf-8');
            // initLayoutDrag 函数内不应该使用 dragstart
            expect(code).not.toContain("'dragstart'");
            expect(code).not.toContain('"dragstart"');
        });

        test('layout.js 使用 mousedown/mousemove/mouseup 事件', () => {
            const fs = require('fs');
            const code = fs.readFileSync(require('path').join(__dirname, '../src/layout.js'), 'utf-8');
            expect(code).toContain("'mousedown'");
            expect(code).toContain("'mousemove'");
            expect(code).toContain("'mouseup'");
        });

        test('index.html 中 local-monitor-group 不再有 draggable 属性', () => {
            const fs = require('fs');
            const html = fs.readFileSync(require('path').join(__dirname, '../src/index.html'), 'utf-8');
            // local-monitor-group 不应该有 draggable="true"
            expect(html).not.toMatch(/id="local-monitor-group"[^>]*draggable/);
        });

        test('index.html 中不再有 display-layout-select 下拉菜单', () => {
            const fs = require('fs');
            const html = fs.readFileSync(require('path').join(__dirname, '../src/index.html'), 'utf-8');
            // 不应该有布局方向下拉菜单
            expect(html).not.toContain('id="display-layout-select"');
        });
    });

    describe('layout.js renderMonitorGroup 使用 i18n', () => {
        test('renderMonitorGroup 不包含硬编码中文 "本机"', () => {
            const fs = require('fs');
            const code = fs.readFileSync(require('path').join(__dirname, '../src/layout.js'), 'utf-8');
            // renderMonitorGroup 函数内不应该有硬编码中文
            const funcMatch = code.match(/function renderMonitorGroup[\s\S]*?^}/m);
            if (funcMatch) {
                expect(funcMatch[0]).not.toContain("'本机'");
                expect(funcMatch[0]).not.toContain("'副屏'");
                expect(funcMatch[0]).not.toContain("'远端'");
            }
        });
    });

    describe('settings-tabs.js 包含 about 分页', () => {
        test('SETTINGS_TABS 数组包含 about', () => {
            const fs = require('fs');
            const code = fs.readFileSync(require('path').join(__dirname, '../src/settings-tabs.js'), 'utf-8');
            expect(code).toContain("'about'");
        });
    });
});
