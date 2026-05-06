/**
 * display-layout-backend.test.ts
 * 显示器布局后端命令 + 多语言覆盖 + 更新日志 + 设置按钮修复 测试用例
 *
 * 覆盖本次修复：
 * 1. get_monitors / get_layout_config / set_layout_config 命令存在（main.rs）
 * 2. 分享功能接入 i18n（不再硬编码中文）
 * 3. 关于Tab文案全部接入 i18n（隐私政策/更新日志加载失败/反馈失败）
 * 4. 更新日志有内容（不再为空）
 * 5. 网卡状态文案接入 i18n
 * 6. 设置按钮有文字标签
 */

const fs = require('fs');
const path = require('path');

const mainRs = fs.readFileSync(path.join(__dirname, '../src-tauri/src/main.rs'), 'utf-8');
const mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');
const indexHtml = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
const i18n = require('../src/i18n.js');
const { t, setLocale, MESSAGES } = i18n;

// ═══════════════════════════════════════════════════════════════════════════════
// 1. 后端命令存在性验证
// ═══════════════════════════════════════════════════════════════════════════════

describe('显示器布局后端命令', () => {
    test('main.rs 包含 get_monitors 命令', () => {
        expect(mainRs).toContain('fn get_monitors()');
    });

    test('main.rs 包含 get_layout_config 命令', () => {
        expect(mainRs).toContain('fn get_layout_config()');
    });

    test('main.rs 包含 set_layout_config 命令', () => {
        expect(mainRs).toContain('fn set_layout_config(');
    });

    test('invoke_handler 中注册了 get_monitors', () => {
        expect(mainRs).toMatch(/generate_handler!\[[\s\S]*get_monitors[\s\S]*\]/);
    });

    test('invoke_handler 中注册了 get_layout_config', () => {
        expect(mainRs).toMatch(/generate_handler!\[[\s\S]*get_layout_config[\s\S]*\]/);
    });

    test('invoke_handler 中注册了 set_layout_config', () => {
        expect(mainRs).toMatch(/generate_handler!\[[\s\S]*set_layout_config[\s\S]*\]/);
    });

    test('LayoutConfig 结构体存在于 config.rs', () => {
        const configRs = fs.readFileSync(path.join(__dirname, '../src-tauri/src/config.rs'), 'utf-8');
        expect(configRs).toContain('struct LayoutConfig');
        expect(configRs).toContain('pub layout: String');
    });

    test('platform/mod.rs 包含 get_monitors_impl 函数', () => {
        const platformMod = fs.readFileSync(path.join(__dirname, '../src-tauri/src/platform/mod.rs'), 'utf-8');
        expect(platformMod).toContain('fn get_monitors_impl');
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 2. 更新日志有内容
// ═══════════════════════════════════════════════════════════════════════════════

describe('更新日志内容验证', () => {
    test('main.rs 中 get_changelog 命令包含实际版本数据（非空列表）', () => {
        expect(mainRs).toContain('fn get_changelog()');
        // 不再是空列表
        expect(mainRs).not.toContain('tauri_commerce::changelog::get_changelog()');
        // 包含版本条目
        expect(mainRs).toContain('version:');
        expect(mainRs).toContain('date:');
        expect(mainRs).toContain('changes:');
    });

    test('get_changelog 包含初始版本 0.1.0 的内容', () => {
        expect(mainRs).toContain('"0.1.0"');
    });

    test('get_changelog 包含 changes_en 英文更新内容', () => {
        expect(mainRs).toContain('changes_en:');
    });

    test('前端 showChangelogModal 支持 changes_en 英文切换', () => {
        expect(mainJs).toContain('changes_en');
        expect(mainJs).toContain("locale === 'en'");
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 3. 多语言覆盖：关于Tab文案
// ═══════════════════════════════════════════════════════════════════════════════

describe('关于Tab多语言覆盖', () => {
    test('i18n 包含 privacyLoadFailed（中文）', () => {
        setLocale('zh');
        expect(t('privacyLoadFailed')).not.toBe('privacyLoadFailed');
        expect(t('privacyLoadFailed').length).toBeGreaterThan(0);
    });

    test('i18n 包含 privacyLoadFailed（英文）', () => {
        setLocale('en');
        expect(t('privacyLoadFailed')).not.toBe('privacyLoadFailed');
        expect(t('privacyLoadFailed').length).toBeGreaterThan(0);
    });

    test('i18n 包含 changelogEmpty（中英文）', () => {
        setLocale('zh');
        expect(t('changelogEmpty')).not.toBe('changelogEmpty');
        setLocale('en');
        expect(t('changelogEmpty')).not.toBe('changelogEmpty');
    });

    test('i18n 包含 changelogLoadFailed（中英文）', () => {
        setLocale('zh');
        expect(t('changelogLoadFailed')).not.toBe('changelogLoadFailed');
        setLocale('en');
        expect(t('changelogLoadFailed')).not.toBe('changelogLoadFailed');
    });

    test('i18n 包含 feedbackFailed（中英文）', () => {
        setLocale('zh');
        expect(t('feedbackFailed')).not.toBe('feedbackFailed');
        setLocale('en');
        expect(t('feedbackFailed')).not.toBe('feedbackFailed');
    });

    test('i18n 包含 restoreFailed（中英文）', () => {
        setLocale('zh');
        expect(t('restoreFailed')).not.toBe('restoreFailed');
        setLocale('en');
        expect(t('restoreFailed')).not.toBe('restoreFailed');
    });

    test('main.js 中 privacyLoadFailed 使用 i18n.t 而非硬编码', () => {
        // 不应再有写死的中文失败提示
        expect(mainJs).not.toContain("'隐私政策内容加载失败，请稍后重试。'");
        expect(mainJs).toContain("t('privacyLoadFailed')");
    });

    test('main.js 中 changelogEmpty 使用 i18n.t 而非硬编码', () => {
        expect(mainJs).not.toContain("'暂无更新日志'");
        expect(mainJs).toContain("t('changelogEmpty')");
    });

    test('main.js 中 feedbackFailed 使用 i18n.t 而非硬编码', () => {
        expect(mainJs).not.toContain("'反馈功能暂时不可用'");
        expect(mainJs).toContain("t('feedbackFailed')");
    });

    test('main.js 中 purchaseFailed 使用 i18n.t 而非硬编码', () => {
        expect(mainJs).not.toContain("'购买功能暂时不可用'");
        expect(mainJs).toContain("t('purchaseFailed')");
    });

    test('关于模态框关闭按钮使用 i18n.t 而非硬编码「关闭」', () => {
        // 不应再有硬编码的「关闭」按钮文字（在 showAboutModal 中）
        expect(mainJs).toContain("data-i18n=\"btnClose\"");
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 4. 分享功能多语言
// ═══════════════════════════════════════════════════════════════════════════════

describe('分享功能多语言', () => {
    // BUG-021/plan v0.1.38：分享文案搬到 i18n.js 的 shareText key 下统一管理，
    // main.js 只写 t('shareText')，不再出现 locale === 'en' 字面量和完整文案
    const i18nJs = fs.readFileSync(path.join(__dirname, '../src/i18n.js'), 'utf-8');

    test("main.js 分享调用统一走 i18n t('shareText')，不再在 JS 里写 locale 分支", () => {
        expect(mainJs).toContain("t('shareText')");
        // 旧的 `locale === 'en'` 分享分支已迁走
        expect(mainJs).not.toMatch(/locale === 'en'[\s\S]{0,120}Cross-platform keyboard/);
    });

    test('i18n.js 分享文案有英文版本', () => {
        expect(i18nJs).toContain('Cross-platform keyboard & mouse sharing tool');
    });

    test('i18n.js 分享文案有中文版本', () => {
        expect(i18nJs).toContain('跨平台键鼠共享工具');
    });

    test('分享成功提示使用 i18n.t 而非硬编码', () => {
        expect(mainJs).toContain("t('shareCopied')");
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 5. 网卡状态多语言
// ═══════════════════════════════════════════════════════════════════════════════

describe('网卡状态多语言', () => {
    test('i18n 包含 nicStatusUp（中英文）', () => {
        setLocale('zh');
        expect(t('nicStatusUp')).toBe('活跃');
        setLocale('en');
        expect(t('nicStatusUp')).toBe('Active');
    });

    test('i18n 包含 nicStatusDown（中英文）', () => {
        setLocale('zh');
        expect(t('nicStatusDown')).toBe('断开');
        setLocale('en');
        expect(t('nicStatusDown')).toBe('Down');
    });

    test('i18n 包含 nicNoActive（中英文）', () => {
        setLocale('zh');
        expect(t('nicNoActive')).not.toBe('nicNoActive');
        setLocale('en');
        expect(t('nicNoActive')).not.toBe('nicNoActive');
    });

    test('i18n 包含 nicDisconnectedWarning（中英文）', () => {
        setLocale('zh');
        expect(t('nicDisconnectedWarning')).toContain('{name}');
        setLocale('en');
        expect(t('nicDisconnectedWarning')).toContain('{name}');
    });

    test('main.js 网卡状态使用 i18n.t 而非硬编码「活跃」「断开」', () => {
        expect(mainJs).not.toContain("'活跃' : '断开'");
        expect(mainJs).toContain("t('nicStatusUp')");
        expect(mainJs).toContain("t('nicStatusDown')");
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 6. 设置按钮文字标签
// ═══════════════════════════════════════════════════════════════════════════════

describe('设置按钮可见性', () => {
    test('index.html 设置按钮包含文字标签 span[data-i18n="btnSettings"]', () => {
        // 设置按钮中应有 data-i18n="btnSettings" 的文字标签
        expect(indexHtml).toContain('id="settings-btn"');
        expect(indexHtml).toContain('data-i18n="btnSettings"');
    });

    test('i18n 中 btnSettings 有中英文翻译', () => {
        setLocale('zh');
        expect(t('btnSettings')).not.toBe('btnSettings');
        setLocale('en');
        expect(t('btnSettings')).not.toBe('btnSettings');
    });
});

// ═══════════════════════════════════════════════════════════════════════════════
// 7. refreshMonitors 不再引用不存在的 display-layout-select
// ═══════════════════════════════════════════════════════════════════════════════

describe('显示器布局 DOM 引用修复', () => {
    test('main.js refreshMonitors 使用 _currentLayout 而非 display-layout-select', () => {
        // 不应再引用不存在的 display-layout-select 元素
        // 应使用全局变量 _currentLayout
        expect(mainJs).not.toContain("getElementById('display-layout-select')");
        expect(mainJs).toContain('_currentLayout');
    });
});
