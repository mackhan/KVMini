/**
 * smart-status-panel.test.ts
 * plan v0.1.38 组 F：主面板合并智能状态面板（去重 detail-panel）
 *
 * 策略（务实版）：
 * - 保留现有 .status-panel 为单一数据源
 * - 加入 #encryption-badge 显示 🔒/⚠️（替代重复的「加密」行）
 * - 加入 <details> 折叠证书指纹（替代整块 #connection-detail-panel）
 * - 删除 #connection-detail-panel 整块及其专属 i18n key
 */

import * as fs from 'fs';
import * as path from 'path';

const html = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
const mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');
const stylesCss = fs.readFileSync(path.join(__dirname, '../src/styles.css'), 'utf-8');

describe('组 F：主面板去重 — 删除 #connection-detail-panel', () => {
    test('index.html 不再含 #connection-detail-panel 整块', () => {
        expect(html).not.toMatch(/id="connection-detail-panel"/);
    });

    test('main.js 不再导出 updateConnectionDetailPanel / updateDetailLatency / toggleConnectionDetail', () => {
        expect(mainJs).not.toMatch(/function\s+updateConnectionDetailPanel\s*\(/);
        expect(mainJs).not.toMatch(/function\s+updateDetailLatency\s*\(/);
        expect(mainJs).not.toMatch(/function\s+toggleConnectionDetail\s*\(/);
    });

    test('styles.css 不再定义 .connection-detail-panel 样式', () => {
        // 允许 .detail- 开头的其他通用规则，但 .connection-detail-panel 专属规则必须移除
        expect(stylesCss).not.toMatch(/\.connection-detail-panel\s*\{/);
    });
});

describe('组 F：智能徽章 #encryption-badge', () => {
    test('index.html 在连接状态同一行插入 #encryption-badge', () => {
        expect(html).toMatch(/id="encryption-badge"/);
    });

    test('main.js 有 updateSmartStatus 统一入口', () => {
        expect(mainJs).toMatch(/function\s+updateSmartStatus\s*\(/);
    });

    test('main.js 有加密徽章更新逻辑（🔒 / ⚠️）', () => {
        // 至少出现一次 🔒 和 ⚠️ 字符（作为 badge 的 textContent 设置值）
        expect(mainJs).toMatch(/🔒/);
        expect(mainJs).toMatch(/⚠️/);
    });
});

describe('组 F：证书指纹折叠区（代替旧的 expandable panel）', () => {
    test('index.html 状态面板内含 <details> + #detail-cert-fingerprint', () => {
        // 保留 id 名（main.js 原有引用），但位置挪入主 status-panel
        expect(html).toMatch(/id="detail-cert-fingerprint"/);
        // 必须是 <details> 原生元素，不再是点击 toggle 的自定义 div
        expect(html).toMatch(/<details[\s\S]{0,400}id="detail-cert-fingerprint"/);
    });
});
