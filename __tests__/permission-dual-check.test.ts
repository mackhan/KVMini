/**
 * BUG-027 / Bug #027 — macOS 启动时权限提醒不全
 *
 * 现象：启动后只提醒了辅助功能权限，没提醒输入监控权限。
 *       用户点了"打开设置"授权了辅助功能后，以为一切就绪，但「开始共享」依然
 *       失败（CGEventTap 创建失败），且没有任何二次引导。
 *
 * 期望行为（TDD 先定义）：
 *   1. i18n 字典必须新增两个输入监控相关文案（中英双语完整）
 *      - overlayInputMonitoringTitle
 *      - overlayInputMonitoringDesc
 *   2. 原辅助功能文案 key 保持可用，用于遮罩动态切换
 *   3. 辅助功能遮罩的标题/描述/步骤/按钮全部走 i18n（禁止硬编码中文）
 *   4. Rust 事件命名空间扩展：
 *      - accessibility://permission-denied      （保留）
 *      - accessibility://permission-granted     （保留）
 *      - input-monitoring://permission-denied   （新增）
 *      - input-monitoring://permission-granted  （新增）
 *   5. 前端收到任一 permission-denied 都要把遮罩切换为对应的提示文字
 */

import { MESSAGES, t, setLocale } from '../src/i18n.js';

describe('BUG-027：双权限 i18n 字典完整性', () => {
    const REQUIRED_KEYS = [
        // 原有辅助功能
        'overlayTitle',
        'overlayDesc',
        // 新增输入监控
        'overlayInputMonitoringTitle',
        'overlayInputMonitoringDesc',
        // 共用步骤/按钮
        'overlayStep1',
        'overlayStep2',
        'overlayStep3',
        'btnGoAuthorize',
        'overlayPollingHint',
    ];

    test.each(['zh', 'en'])('%s 字典必须包含所有必需 key 且非空', (locale) => {
        const dict = (MESSAGES as any)[locale];
        expect(dict).toBeDefined();
        for (const key of REQUIRED_KEYS) {
            expect(dict[key]).toBeDefined();
            expect(typeof dict[key]).toBe('string');
            expect(dict[key].length).toBeGreaterThan(0);
        }
    });

    test('中文描述必须提到「输入监控」且不与辅助功能文案重复', () => {
        const zh = (MESSAGES as any).zh;
        expect(zh.overlayInputMonitoringTitle).toMatch(/输入监控|Input/);
        // 不应和 overlayTitle 完全一样
        expect(zh.overlayInputMonitoringTitle).not.toBe(zh.overlayTitle);
        expect(zh.overlayInputMonitoringDesc).not.toBe(zh.overlayDesc);
    });

    test('英文描述必须提到 Input Monitoring', () => {
        const en = (MESSAGES as any).en;
        expect(en.overlayInputMonitoringTitle).toMatch(/Input Monitoring/i);
    });

    test('t() 函数在两种语言下都能正确返回输入监控标题', () => {
        setLocale('zh');
        expect(t('overlayInputMonitoringTitle')).toMatch(/输入监控/);
        setLocale('en');
        expect(t('overlayInputMonitoringTitle')).toMatch(/Input Monitoring/i);
    });
});

describe('BUG-027：index.html 权限遮罩不能硬编码中文', () => {
    // 读取 index.html 源文件，校验关键遮罩元素都带 data-i18n 属性
    // 不依赖 DOMParser 的完整解析，直接用正则做字符串校验
    const fs = require('fs');
    const path = require('path');
    const htmlPath = path.resolve(__dirname, '../src/index.html');
    const html = fs.readFileSync(htmlPath, 'utf8');

    test('遮罩标题 <h2 class="accessibility-overlay-title"> 必须带 data-i18n', () => {
        // 匹配形如 <h2 class="accessibility-overlay-title" data-i18n="xxx">
        const re = /<h2[^>]*class="[^"]*accessibility-overlay-title[^"]*"[^>]*data-i18n="([a-zA-Z]+)"/;
        const m = html.match(re);
        expect(m).not.toBeNull();
        // 默认展示辅助功能标题，但 JS 可动态改成输入监控
        expect(m![1]).toBe('overlayTitle');
    });

    test('遮罩描述 <p class="accessibility-overlay-desc"> 必须带 data-i18n', () => {
        const re = /<p[^>]*class="[^"]*accessibility-overlay-desc[^"]*"[^>]*data-i18n="([a-zA-Z]+)"/;
        const m = html.match(re);
        expect(m).not.toBeNull();
        expect(m![1]).toBe('overlayDesc');
    });

    test('"前往授权"按钮必须带 data-i18n=btnGoAuthorize', () => {
        const re = /id="open-accessibility-btn"[^>]*data-i18n="([a-zA-Z]+)"/;
        const m = html.match(re);
        expect(m).not.toBeNull();
        expect(m![1]).toBe('btnGoAuthorize');
    });

    test('"等待授权中"提示必须带 data-i18n=overlayPollingHint', () => {
        const re = /id="accessibility-polling-hint"[^>]*data-i18n="([a-zA-Z]+)"/;
        const m = html.match(re);
        expect(m).not.toBeNull();
        expect(m![1]).toBe('overlayPollingHint');
    });
});

describe('BUG-027：accessibility.js 必须同时监听输入监控事件', () => {
    const fs = require('fs');
    const path = require('path');
    const jsPath = path.resolve(__dirname, '../src/accessibility.js');
    const src = fs.readFileSync(jsPath, 'utf8');

    test('必须监听 input-monitoring://permission-denied 事件', () => {
        expect(src).toContain('input-monitoring://permission-denied');
    });

    test('必须监听 input-monitoring://permission-granted 事件', () => {
        expect(src).toContain('input-monitoring://permission-granted');
    });

    test('收到输入监控 denied 事件时，遮罩文案要切换到输入监控 i18n key', () => {
        // 关键字符串：代码中必须引用 overlayInputMonitoringTitle / Desc
        expect(src).toContain('overlayInputMonitoringTitle');
        expect(src).toContain('overlayInputMonitoringDesc');
    });
});
