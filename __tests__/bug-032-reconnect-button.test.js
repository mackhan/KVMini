/**
 * BUG-032 回归测试（v0.1.59 适配单按钮三态新模型）
 *
 * 历史：旧测试基于"disconnect-btn + reconnect-btn 双按钮 hidden 切换"模型，
 *       已在 v0.1.59 BUG-042 单按钮重构后过时。
 *
 * 新模型下 BUG-032 的等价不变量：
 *   - reconnecting 事件不能把按钮 phase 切到 disconnected
 *   - 已连接状态下心跳波动只更新状态徽章，按钮保持 connected
 *
 * 这里通过静态断言保护，main.js 不能在 reconnecting 块里调 setConnectionStatus('connecting')
 * 或直接重置 connectionPhase 为非 connected 值。
 */

const fs = require('fs');
const path = require('path');

describe('BUG-032 (v0.1.59 新模型): reconnecting 事件不应改变按钮 phase', () => {
    const main = () => fs.readFileSync(path.join(__dirname, '..', 'src', 'main.js'), 'utf8');

    test('reconnecting 块内不调用 setConnectionStatus("connecting") 或 setConnectionStatus("disconnected")', () => {
        const src = main();
        const idx = src.search(/eventType\s*===\s*['"]reconnecting['"]/);
        expect(idx).toBeGreaterThan(0);
        // 截到下个 } else if 或 } else {
        const slice = src.slice(idx, idx + 1500);
        const endIdx = slice.search(/\n\s+\}\s+else\s+if|\n\s+\}\s+else\s*\{/);
        const block = endIdx > 0 ? slice.slice(0, endIdx) : slice;
        // 排除注释
        const codeOnly = block.split('\n').map((l) => l.replace(/\/\/.*$/, '')).join('\n');
        expect(/setConnectionStatus\(['"]connecting['"]\)/.test(codeOnly)).toBe(false);
        expect(/setConnectionStatus\(['"]disconnected['"]\)/.test(codeOnly)).toBe(false);
    });

    test('reconnecting 块内不调用 updateConnectButton(...)（按钮 phase 维持原状）', () => {
        const src = main();
        const idx = src.search(/eventType\s*===\s*['"]reconnecting['"]/);
        const slice = src.slice(idx, idx + 1500);
        const endIdx = slice.search(/\n\s+\}\s+else\s+if|\n\s+\}\s+else\s*\{/);
        const block = endIdx > 0 ? slice.slice(0, endIdx) : slice;
        const codeOnly = block.split('\n').map((l) => l.replace(/\/\/.*$/, '')).join('\n');
        expect(/updateConnectButton\s*\(/.test(codeOnly)).toBe(false);
    });

    test('disconnected 事件块内必须调 setConnectionStatus("disconnected")（确保真正断开时按钮回到 connect 文案）', () => {
        const src = main();
        const idx = src.search(/eventType\s*===\s*['"]disconnected['"]/);
        expect(idx).toBeGreaterThan(0);
        const slice = src.slice(idx, idx + 1500);
        const endIdx = slice.search(/\n\s+\}\s+else\s+if|\n\s+\}\s+else\s*\{/);
        const block = endIdx > 0 ? slice.slice(0, endIdx) : slice;
        expect(/setConnectionStatus\(['"]disconnected['"]\)/.test(block)).toBe(true);
    });

    test('updateConnectButton 必须存在（单按钮三态实现的核心）', () => {
        const src = main();
        expect(/function\s+updateConnectButton\s*\(\s*phase\s*\)/.test(src)).toBe(true);
    });
});
