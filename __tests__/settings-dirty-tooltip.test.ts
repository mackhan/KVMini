/**
 * settings-dirty-tooltip.test.ts
 * plan v0.1.38 组 B / Task B1 + B2
 *
 * B1：tab-dirty 红点必须有双语 tooltip，key = tabDirtyTooltip
 * B2：任一 tab 变脏时，顶部「保存」按钮加 .btn-save-dirty class（脉冲动画）
 *     所有 tab 清空 dirty 时，按钮移除该 class
 */

// 使用 jsdom 构建最小 DOM，然后 require settings-tabs.js
import * as fs from 'fs';
import * as path from 'path';

const i18nJs = fs.readFileSync(path.join(__dirname, '../src/i18n.js'), 'utf-8');
const settingsTabsJs = fs.readFileSync(path.join(__dirname, '../src/settings-tabs.js'), 'utf-8');
const stylesCss = fs.readFileSync(path.join(__dirname, '../src/styles.css'), 'utf-8');

describe('B1: 未保存红点 tooltip', () => {
    test('i18n.js 定义了 tabDirtyTooltip（zh/en）', () => {
        expect(i18nJs).toMatch(/tabDirtyTooltip:\s*['"]此分类下有未保存的修改['"]/);
        expect(i18nJs).toMatch(/tabDirtyTooltip:\s*['"]Unsaved changes in this section['"]/);
    });

    test('markDirty 会给 button 加 title 属性（文案来自 i18n t）', () => {
        // 构建 jsdom 环境
        document.body.innerHTML = `
            <button class="settings-tab-btn" data-tab="general">通用</button>
            <button id="settings-save-btn">保存</button>
        `;
        // 加载 i18n + settings-tabs
        (global as any).window = globalThis;
        eval(i18nJs);
        (global as any).window.i18n = (globalThis as any).window.i18n || require('../src/i18n.js');
        eval(settingsTabsJs);

        const mod = (globalThis as any).window.SettingsTabsModule;
        mod.markDirty('general');

        const btn = document.querySelector('[data-tab="general"]') as HTMLButtonElement;
        expect(btn.title).toBe('此分类下有未保存的修改');
    });
});

describe('B2: 脏态时保存按钮强调', () => {
    test('styles.css 含 save-pulse 动画 + .btn-save-dirty 类', () => {
        expect(stylesCss).toMatch(/@keyframes\s+save-pulse/);
        // 不使用 /s flag（受限于 ts 目标），用 [\s\S] 等价表达"跨行匹配"
        expect(stylesCss).toMatch(/\.btn-save-dirty\s*\{[\s\S]*?animation:/);
    });

    test('markDirty 会给 #settings-save-btn 加 btn-save-dirty class；clearAllDirty 会清除', () => {
        document.body.innerHTML = `
            <button class="settings-tab-btn" data-tab="general">通用</button>
            <button id="settings-save-btn">保存</button>
        `;
        (global as any).window = globalThis;
        eval(i18nJs);
        eval(settingsTabsJs);

        const mod = (globalThis as any).window.SettingsTabsModule;
        const saveBtn = document.getElementById('settings-save-btn') as HTMLButtonElement;

        expect(saveBtn.classList.contains('btn-save-dirty')).toBe(false);

        mod.markDirty('general');
        expect(saveBtn.classList.contains('btn-save-dirty')).toBe(true);

        mod.clearAllDirty();
        expect(saveBtn.classList.contains('btn-save-dirty')).toBe(false);
    });
});
