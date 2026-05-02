/**
 * integration-self-test.test.ts
 *
 * 集成自测：模拟真实 DOM 环境，验证三个 bug 是否真正修复
 * Bug #021: 中英文切换后所有文字变化
 * Bug #022: 主题切换后界面变化
 * Bug #023: 显示器拖拽使用 mousedown/mousemove/mouseup
 *
 * 这个测试模拟用户在设置页面操作的完整流程
 */

const fs = require('fs');
const path = require('path');

// ─── 加载模块 ────────────────────────────────────────────────────────────────

const i18n = require('../src/i18n');
const themeModule = require('../src/theme');
const layoutModule = require('../src/layout');

// ─── 工具函数 ────────────────────────────────────────────────────────────────

/**
 * 模拟 applyI18n 的核心逻辑（批量替换 data-i18n 元素）
 */
function applyI18nToDOM() {
    document.querySelectorAll('[data-i18n]').forEach((el: Element) => {
        const key = el.getAttribute('data-i18n');
        if (key) el.textContent = i18n.t(key);
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach((el: HTMLInputElement) => {
        const key = el.getAttribute('data-i18n-placeholder');
        if (key) el.placeholder = i18n.t(key);
    });
    document.title = i18n.t('pageTitle');
}

/**
 * 从 index.html 加载完整 DOM
 */
function loadIndexHtml() {
    const html = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
    // 提取 body 内容
    const bodyMatch = html.match(/<body[^>]*>([\s\S]*)<\/body>/i);
    if (bodyMatch) {
        document.body.innerHTML = bodyMatch[1];
    }
}

// ─── Bug #021: 中英文切换完整性测试 ──────────────────────────────────────────

describe('Bug #021: 中英文切换完整性（集成测试）', () => {
    beforeEach(() => {
        loadIndexHtml();
        // 初始状态：中文
        i18n.setLocale('zh');
        applyI18nToDOM();
    });

    test('初始状态：所有 data-i18n 元素显示中文', () => {
        const settingsTitle = document.querySelector('[data-i18n="settingsTitle"]');
        expect(settingsTitle?.textContent).toBe('⚙ 设置');

        // v0.1.38 组 F：detailLocal 已随旧面板移除，改验证证书指纹 summary
        const detailCert = document.querySelector('[data-i18n="detailCertFingerprint"]');
        expect(detailCert?.textContent).toBe('证书指纹');

        const discardTitle = document.querySelector('[data-i18n="discardDialogTitle"]');
        expect(discardTitle?.textContent).toBe('⚠️ 未保存的修改');

        const licenseTitle = document.querySelector('[data-i18n="licenseGateTitle"]');
        expect(licenseTitle?.textContent).toBe('试用已到期');
    });

    test('切换到英文后：所有 data-i18n 元素显示英文', () => {
        // 模拟用户在设置页选择英文并保存
        i18n.setLocale('en');
        applyI18nToDOM();

        // 验证关键元素已切换
        const settingsTitle = document.querySelector('[data-i18n="settingsTitle"]');
        expect(settingsTitle?.textContent).toBe('⚙ Settings');

        // v0.1.38 组 F：detailLocal/detailPeer/detailEncryption/detailLatency 已随
        // #connection-detail-panel 移除；只保留 detailCertFingerprint（折叠区 summary）
        const detailCert = document.querySelector('[data-i18n="detailCertFingerprint"]');
        expect(detailCert?.textContent).toBe('Certificate Fingerprint');

        const reconnectLabel = document.querySelector('[data-i18n="labelReconnectProgress"]');
        expect(reconnectLabel?.textContent).toBe('Reconnect:');

        const discardTitle = document.querySelector('[data-i18n="discardDialogTitle"]');
        expect(discardTitle?.textContent).toBe('⚠️ Unsaved Changes');

        const discardMsg = document.querySelector('[data-i18n="discardDialogMsg"]');
        expect(discardMsg?.textContent).toBe('You have unsaved changes. Discard them?');

        const discardBtn = document.querySelector('[data-i18n="btnDiscardChanges"]');
        expect(discardBtn?.textContent).toBe('Discard Changes');

        const continueBtn = document.querySelector('[data-i18n="btnContinueEditing"]');
        expect(continueBtn?.textContent).toBe('Continue Editing');

        const licenseTitle = document.querySelector('[data-i18n="licenseGateTitle"]');
        expect(licenseTitle?.textContent).toBe('Trial Expired');

        // IAP 系统：验证 licenseGateDescNew（而非旧的 licenseGateDesc）
        const licenseDescNew = document.querySelector('[data-i18n="licenseGateDescNew"]');
        if (licenseDescNew) {
            expect(licenseDescNew?.textContent).toBe('Thank you for trying KVMini! Purchase the full version to continue using all features.');
        }

        const tabGeneral = document.querySelector('[data-tab="general"][data-i18n="tabGeneral"]');
        expect(tabGeneral?.textContent).toBe('General');

        const tabDisplay = document.querySelector('[data-tab="display"][data-i18n="tabDisplay"]');
        expect(tabDisplay?.textContent).toBe('Display Layout');

        const tabAbout = document.querySelector('[data-tab="about"][data-i18n="tabAbout"]');
        expect(tabAbout?.textContent).toBe('About');

        const hintMouseEdge = document.querySelector('[data-i18n="hintMouseEdge"]');
        expect(hintMouseEdge?.textContent).not.toContain('连接成功后');

        const hintHotkey = document.querySelector('[data-i18n="hintHotkey"]');
        expect(hintHotkey?.textContent).not.toContain('按 Ctrl+Alt+S');
    });

    test('切换回中文后：所有元素恢复中文', () => {
        // 先切英文
        i18n.setLocale('en');
        applyI18nToDOM();

        // 再切中文
        i18n.setLocale('zh');
        applyI18nToDOM();

        const settingsTitle = document.querySelector('[data-i18n="settingsTitle"]');
        expect(settingsTitle?.textContent).toBe('⚙ 设置');

        // v0.1.38 组 F：detailLocal 已随旧面板移除，改验证证书指纹 summary 回中文
        const detailCert = document.querySelector('[data-i18n="detailCertFingerprint"]');
        expect(detailCert?.textContent).toBe('证书指纹');
    });

    test('页面标题切换到英文后变化', () => {
        i18n.setLocale('en');
        applyI18nToDOM();
        expect(document.title).toBe('KVMini - Cross-platform KVM Sharing');
    });

    test('页面标题切换回中文后恢复', () => {
        i18n.setLocale('en');
        applyI18nToDOM();
        i18n.setLocale('zh');
        applyI18nToDOM();
        expect(document.title).toBe('KVMini - 跨平台键鼠共享');
    });

    test('统计 index.html 中所有 data-i18n 元素，确认没有遗漏', () => {
        const html = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
        const dataI18nMatches = html.match(/data-i18n="([^"]+)"/g) || [];
        const keys = dataI18nMatches.map((m: string) => m.replace(/data-i18n="([^"]+)"/, '$1'));

        // 每个 key 都应该在 i18n 翻译中存在
        const missingInZh: string[] = [];
        const missingInEn: string[] = [];
        keys.forEach((key: string) => {
            if (!i18n.MESSAGES.zh[key]) missingInZh.push(key);
            if (!i18n.MESSAGES.en[key]) missingInEn.push(key);
        });

        expect(missingInZh).toEqual([]);
        expect(missingInEn).toEqual([]);
    });
});

// ─── Bug #022: 主题切换完整性测试 ────────────────────────────────────────────

describe('Bug #022: 主题切换完整性（集成测试）', () => {
    beforeEach(() => {
        document.documentElement.removeAttribute('data-theme');
    });

    test('初始状态：没有 data-theme 属性', () => {
        expect(document.documentElement.getAttribute('data-theme')).toBeNull();
    });

    test('setTheme("dark") 后 data-theme="dark"', () => {
        themeModule.setTheme('dark');
        expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    test('setTheme("light") 后 data-theme="light"', () => {
        themeModule.setTheme('light');
        expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });

    test('setTheme("auto") 后 data-theme 为 dark 或 light', () => {
        themeModule.setTheme('auto');
        const theme = document.documentElement.getAttribute('data-theme');
        expect(['dark', 'light']).toContain(theme);
    });

    test('从 dark 切换到 light 后 data-theme 变化', () => {
        themeModule.setTheme('dark');
        expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
        themeModule.setTheme('light');
        expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });

    test('从 light 切换到 dark 后 data-theme 变化', () => {
        themeModule.setTheme('light');
        expect(document.documentElement.getAttribute('data-theme')).toBe('light');
        themeModule.setTheme('dark');
        expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    test('CSS 文件中 [data-theme="dark"] 和 [data-theme="light"] 都有完整的变量定义', () => {
        const css = fs.readFileSync(path.join(__dirname, '../src/styles.css'), 'utf-8');

        // 检查深色主题变量
        expect(css).toContain('[data-theme="dark"]');
        expect(css).toMatch(/\[data-theme="dark"\]\s*\{[^}]*--panel-bg:/);
        expect(css).toMatch(/\[data-theme="dark"\]\s*\{[^}]*--text-primary:/);

        // 检查浅色主题变量
        expect(css).toContain('[data-theme="light"]');
        expect(css).toMatch(/\[data-theme="light"\]\s*\{[^}]*--panel-bg:/);
        expect(css).toMatch(/\[data-theme="light"\]\s*\{[^}]*--text-primary:/);
    });

    test('CSS 文件中没有孤立的属性声明（语法错误）', () => {
        const css = fs.readFileSync(path.join(__dirname, '../src/styles.css'), 'utf-8');
        // 检查深色主题选择器存在
        expect(css).toContain('[data-theme="dark"]');
        // 不应该有孤立的 font-size 在选择器外
        const lines = css.split('\n');
        let braceDepth = 0;
        const orphanedProps: string[] = [];
        lines.forEach((line, idx) => {
            const trimmed = line.trim();
            braceDepth += (line.match(/\{/g) || []).length;
            braceDepth -= (line.match(/\}/g) || []).length;
            // 如果 braceDepth === 0 且有属性声明（包含冒号但不是选择器）
            if (braceDepth === 0 && trimmed.includes(':') && !trimmed.startsWith('//') && !trimmed.startsWith('/*') && !trimmed.startsWith('*') && trimmed.endsWith(';')) {
                orphanedProps.push(`Line ${idx + 1}: ${trimmed}`);
            }
        });
        expect(orphanedProps).toEqual([]);
    });
});

// ─── Bug #023: 显示器拖拽完整性测试 ──────────────────────────────────────────

describe('Bug #023: 显示器拖拽完整性（集成测试）', () => {
    test('layout.js 使用 mousedown/mousemove/mouseup 而非 dragstart', () => {
        const code = fs.readFileSync(path.join(__dirname, '../src/layout.js'), 'utf-8');
        expect(code).toContain("'mousedown'");
        expect(code).toContain("'mousemove'");
        expect(code).toContain("'mouseup'");
        expect(code).not.toContain("'dragstart'");
        expect(code).not.toContain('"dragstart"');
    });

    test('index.html 中 local-monitor-group 没有 draggable 属性', () => {
        const html = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
        expect(html).not.toMatch(/id="local-monitor-group"[^>]*draggable/);
        expect(html).not.toMatch(/draggable[^>]*id="local-monitor-group"/);
    });

    test('index.html 中没有 display-layout-select 下拉菜单', () => {
        const html = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
        expect(html).not.toContain('id="display-layout-select"');
    });

    test('拖拽到左侧 → LocalLeftOfRemote', () => {
        // 本机在左(100,200)，远端在右(300,200)
        expect(layoutModule.calcLayoutFromDrop(100, 200, 300, 200)).toBe('LocalLeftOfRemote');
    });

    test('拖拽到右侧 → LocalRightOfRemote', () => {
        expect(layoutModule.calcLayoutFromDrop(300, 200, 100, 200)).toBe('LocalRightOfRemote');
    });

    test('拖拽到上方 → LocalAboveRemote', () => {
        expect(layoutModule.calcLayoutFromDrop(200, 100, 200, 300)).toBe('LocalAboveRemote');
    });

    test('拖拽到下方 → LocalBelowRemote', () => {
        expect(layoutModule.calcLayoutFromDrop(200, 300, 200, 100)).toBe('LocalBelowRemote');
    });

    test('initLayoutDrag 函数存在且可调用', () => {
        expect(typeof layoutModule.initLayoutDrag).toBe('function');
    });

    test('拖拽交互：mousedown 触发后 isDragging 状态正确', () => {
        // 设置 DOM
        document.body.innerHTML = `
            <div id="display-layout-preview" style="width:400px;height:200px;position:relative;">
                <div id="local-monitor-group" style="width:100px;height:80px;"></div>
                <div id="layout-divider">→</div>
                <div id="remote-monitor-group" style="width:100px;height:80px;"></div>
            </div>
        `;

        const localGroup = document.getElementById('local-monitor-group') as HTMLElement;
        const preview = document.getElementById('display-layout-preview') as HTMLElement;

        let layoutChanged = false;
        let newLayoutValue = '';

        layoutModule.initLayoutDrag((newLayout: string) => {
            layoutChanged = true;
            newLayoutValue = newLayout;
        });

        // 模拟 mousedown
        const mousedownEvent = new MouseEvent('mousedown', {
            clientX: 50,
            clientY: 40,
            bubbles: true,
        });
        localGroup.dispatchEvent(mousedownEvent);

        // 验证拖拽开始后的状态
        expect(localGroup.classList.contains('dragging')).toBe(true);
        expect(preview.classList.contains('drag-active')).toBe(true);
        expect(localGroup.style.position).toBe('absolute');
        expect(localGroup.style.opacity).toBe('0.85');

        // 模拟 mouseup（在远端右侧）
        const mouseupEvent = new MouseEvent('mouseup', {
            clientX: 350,
            clientY: 100,
            bubbles: true,
        });
        document.dispatchEvent(mouseupEvent);

        // 验证拖拽结束后的状态
        expect(localGroup.classList.contains('dragging')).toBe(false);
        expect(preview.classList.contains('drag-active')).toBe(false);
    });
});

// ─── 综合验证：模拟完整的用户操作流程 ─────────────────────────────────────────

describe('综合验证：模拟用户操作流程', () => {
    beforeEach(() => {
        loadIndexHtml();
        i18n.setLocale('zh');
        applyI18nToDOM();
        document.documentElement.removeAttribute('data-theme');
    });

    test('完整流程：中文→英文→主题切换→中文', () => {
        // Step 1: 初始中文状态
        expect(document.querySelector('[data-i18n="settingsTitle"]')?.textContent).toBe('⚙ 设置');
        expect(document.documentElement.getAttribute('data-theme')).toBeNull();

        // Step 2: 切换到英文
        i18n.setLocale('en');
        applyI18nToDOM();
        expect(document.querySelector('[data-i18n="settingsTitle"]')?.textContent).toBe('⚙ Settings');
        expect(document.querySelector('[data-i18n="tabGeneral"]')?.textContent).toBe('General');

        // Step 3: 切换到深色主题
        themeModule.setTheme('dark');
        expect(document.documentElement.getAttribute('data-theme')).toBe('dark');

        // Step 4: 切换到浅色主题
        themeModule.setTheme('light');
        expect(document.documentElement.getAttribute('data-theme')).toBe('light');

        // Step 5: 切换回中文
        i18n.setLocale('zh');
        applyI18nToDOM();
        expect(document.querySelector('[data-i18n="settingsTitle"]')?.textContent).toBe('⚙ 设置');
        expect(document.querySelector('[data-i18n="tabGeneral"]')?.textContent).toBe('通用');

        // Step 6: 主题仍然是浅色（语言切换不影响主题）
        expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });
});
