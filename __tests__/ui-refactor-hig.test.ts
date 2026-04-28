/**
 * ui-refactor-hig.test.ts
 * Apple HIG 风格 UI 重构测试用例
 *
 * 覆盖：
 * 1. 视图切换机制（主界面 ↔ 设置页面）
 * 2. 激活码 UI 已删除
 * 3. IAP 购买/恢复购买功能
 * 4. Apple HIG CSS 变量系统
 * 5. 设置页面左侧 Tab + 右侧内容区布局
 * 6. 版本号显示
 */

const fs = require('fs');
const path = require('path');

// ─── 辅助：读取源文件 ─────────────────────────────────────────────────────────

const indexHtml = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
const stylesCss = fs.readFileSync(path.join(__dirname, '../src/styles.css'), 'utf-8');
const licenseJs = fs.readFileSync(path.join(__dirname, '../src/license.js'), 'utf-8');
const i18nJs = fs.readFileSync(path.join(__dirname, '../src/i18n.js'), 'utf-8');
const cargoToml = fs.readFileSync(path.join(__dirname, '../src-tauri/Cargo.toml'), 'utf-8');

// ═══════════════════════════════════════════════════════════════════════════════
// 1. 视图切换机制（方案 D：整页替换 + 返回按钮）
// ═══════════════════════════════════════════════════════════════════════════════

describe('视图切换机制（方案 D）', () => {
    test('index.html 应包含 #view-main 主界面视图容器', () => {
        expect(indexHtml).toContain('id="view-main"');
    });

    test('index.html 应包含 #view-settings 设置视图容器', () => {
        expect(indexHtml).toContain('id="view-settings"');
    });

    test('设置视图应包含返回按钮', () => {
        expect(indexHtml).toMatch(/id="settings-back-btn"/);
    });

    test('设置视图应包含左侧垂直 Tab 导航栏', () => {
        expect(indexHtml).toMatch(/class="[^"]*settings-sidebar/);
    });

    test('设置视图应包含右侧内容区', () => {
        expect(indexHtml).toMatch(/class="[^"]*settings-content-area/);
    });

    test('不应存在模态框式设置面板 (#settings-modal)', () => {
        expect(indexHtml).not.toContain('id="settings-modal"');
    });

    test('不应存在 modal-overlay 遮罩（设置页）', () => {
        // 设置页面不再使用模态框遮罩
        expect(indexHtml).not.toMatch(/class="modal-overlay"[^>]*onclick="closeSettings\(\)"/);
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 2. 激活码 UI 已删除
// ═══════════════════════════════════════════════════════════════════════════════

describe('激活码 UI 已删除', () => {
    test('index.html 不应包含 license-key-input 输入框', () => {
        expect(indexHtml).not.toContain('id="license-key-input"');
    });

    test('index.html 不应包含 license-activate-btn 激活按钮', () => {
        expect(indexHtml).not.toContain('id="license-activate-btn"');
    });

    test('index.html 不应包含激活码分隔符 "或"', () => {
        expect(indexHtml).not.toMatch(/class="license-gate-divider"/);
    });

    test('index.html 不应包含 license-error-msg 错误提示', () => {
        expect(indexHtml).not.toContain('id="license-error-msg"');
    });

    test('license.js 不应包含 activate 相关函数', () => {
        expect(licenseJs).not.toMatch(/function\s+activateLicenseKey/);
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 3. IAP 购买/恢复购买功能
// ═══════════════════════════════════════════════════════════════════════════════

describe('IAP 内购功能', () => {
    test('license.js 应包含 purchaseProduct 函数', () => {
        expect(licenseJs).toMatch(/purchaseProduct|purchase/);
    });

    test('license.js 应包含 restorePurchase 函数', () => {
        expect(licenseJs).toMatch(/restorePurchase|restore/);
    });

    test('license.js 应调用 invoke("purchase")', () => {
        expect(licenseJs).toContain("'purchase'");
    });

    test('license.js 应调用 invoke("restore_purchase")', () => {
        expect(licenseJs).toContain("'restore_purchase'");
    });

    test('LicenseModule 应导出 purchaseProduct', () => {
        expect(licenseJs).toMatch(/purchaseProduct/);
    });

    test('LicenseModule 应导出 restorePurchase', () => {
        expect(licenseJs).toMatch(/restorePurchase/);
    });

    test('Cargo.toml 应启用 iap feature', () => {
        // iap feature 不应被注释掉
        expect(cargoToml).toMatch(/features\s*=\s*\[.*"iap"/);
    });

    test('过期引导界面应包含购买按钮', () => {
        expect(indexHtml).toMatch(/id="license-purchase-btn"|id="license-buy-btn"/);
    });

    test('过期引导界面应包含恢复购买按钮', () => {
        expect(indexHtml).toMatch(/id="license-restore-btn"/);
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 4. Apple HIG CSS 变量系统
// ═══════════════════════════════════════════════════════════════════════════════

describe('Apple HIG CSS 变量系统', () => {
    test('CSS 应定义 Apple HIG 浅色背景色 #f5f5f7', () => {
        expect(stylesCss).toContain('#f5f5f7');
    });

    test('CSS 应定义 Apple HIG 主色 #007aff', () => {
        expect(stylesCss.toLowerCase()).toContain('#007aff');
    });

    test('CSS 应定义深色模式背景色 #1c1c1e', () => {
        expect(stylesCss.toLowerCase()).toContain('#1c1c1e');
    });

    test('CSS 应定义深色模式主色 #0a84ff', () => {
        expect(stylesCss.toLowerCase()).toContain('#0a84ff');
    });

    test('CSS 不应使用紫色渐变背景 #667eea', () => {
        expect(stylesCss).not.toContain('#667eea');
    });

    test('CSS 不应使用紫色渐变背景 #764ba2', () => {
        expect(stylesCss).not.toContain('#764ba2');
    });

    test('CSS 应使用 12px 圆角卡片', () => {
        expect(stylesCss).toMatch(/border-radius:\s*12px/);
    });

    test('CSS 深色主题应使用 [data-theme="dark"] 选择器', () => {
        expect(stylesCss).toContain('[data-theme="dark"]');
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 5. 设置页面布局
// ═══════════════════════════════════════════════════════════════════════════════

describe('设置页面布局（左侧 Tab + 右侧内容区）', () => {
    test('设置视图应包含 5 个 Tab（通用/显示器布局/网络/安全/关于）', () => {
        // 检查 tab 数据属性
        expect(indexHtml).toContain('data-tab="general"');
        expect(indexHtml).toContain('data-tab="display"');
        expect(indexHtml).toContain('data-tab="network"');
        expect(indexHtml).toContain('data-tab="security"');
        expect(indexHtml).toContain('data-tab="about"');
    });

    test('CSS 应定义 settings-sidebar 样式（固定宽度 180px）', () => {
        expect(stylesCss).toMatch(/\.settings-sidebar/);
    });

    test('CSS 应定义 settings-content-area 样式', () => {
        expect(stylesCss).toMatch(/\.settings-content-area/);
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 6. 版本号显示
// ═══════════════════════════════════════════════════════════════════════════════

describe('版本号显示', () => {
    test('主界面应有版本号显示元素', () => {
        expect(indexHtml).toContain('id="app-version"');
    });

    test('设置页面应有版本号显示元素', () => {
        expect(indexHtml).toContain('id="settings-version"');
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 7. i18n 新增购买相关翻译
// ═══════════════════════════════════════════════════════════════════════════════

describe('i18n 购买相关翻译', () => {
    test('i18n 应包含购买按钮翻译 key (purchaseBtn)', () => {
        expect(i18nJs).toContain('purchaseBtn');
    });

    test('i18n 应包含恢复购买翻译 key (restoreBtn)', () => {
        expect(i18nJs).toContain('restoreBtn');
    });

    test('i18n 应包含购买成功翻译 key (purchaseSuccess)', () => {
        expect(i18nJs).toContain('purchaseSuccess');
    });

    test('i18n 应包含购买失败翻译 key (purchaseFailed)', () => {
        expect(i18nJs).toContain('purchaseFailed');
    });

    test('i18n 不应包含旧的激活码翻译 key (btnActivate)', () => {
        expect(i18nJs).not.toContain('btnActivate');
    });

    test('i18n 不应包含旧的激活码描述 (licenseGateDesc)', () => {
        // 使用正则精确匹配 licenseGateDesc 但排除 licenseGateDescNew
        expect(i18nJs).not.toMatch(/licenseGateDesc[^N]/);
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 8. 视图切换 JS 逻辑
// ═══════════════════════════════════════════════════════════════════════════════

describe('视图切换 JS 逻辑', () => {
    let mainJs: string;

    beforeAll(() => {
        mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');
    });

    test('openSettings 应切换 #view-main 和 #view-settings 的显隐', () => {
        expect(mainJs).toMatch(/view-main/);
        expect(mainJs).toMatch(/view-settings/);
    });

    test('不应包含模态框打开逻辑 (settingsModal.classList.remove)', () => {
        expect(mainJs).not.toMatch(/settingsModal\.classList\.remove\('hidden'\)/);
    });
});
