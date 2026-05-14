/**
 * BUG-042 (v0.1.59): 连接按钮状态混乱（连接成功后还显示"重连"）
 *
 * 根因：双按钮（disconnect-btn + reconnect-btn）+ 心跳波动 reconnecting 事件
 *       会让按钮在"断开 / 重连"之间频繁来回切换，远端与本地状态不同步。
 *
 * 修复策略：参考 Synergy 3 / Deskflow 单按钮三态：
 *   - data-state="disconnected" → 文案 "连接 / Connect"，btn-primary
 *   - data-state="connecting"   → 文案 "连接中… / Connecting…"，btn-secondary，disabled
 *   - data-state="connected"    → 文案 "断开 / Disconnect"，btn-secondary
 *
 * 唯一真相源：appState.connectionPhase + sharing-state-sync snapshot。
 * reconnecting 事件不切按钮文案（按钮保持 connected），只更新状态徽章和 reconnectStatusRow。
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
const read = (rel: string) => fs.readFileSync(path.join(repoRoot, rel), 'utf8');

// ── DOM 层：单按钮 + data-state 属性 ────────────────────────────────────────
describe('BUG-042 DOM — index.html 单按钮三态结构', () => {
  const html = () => read('src/index.html');

  it('必须存在 #connect-btn 单一按钮（替代旧的 disconnect-btn / reconnect-btn 双按钮）', () => {
    const src = html();
    expect(/id=["']connect-btn["']/.test(src)).toBe(true);
  });

  it('必须移除旧的 #disconnect-btn 和 #reconnect-btn 元素声明', () => {
    const src = html();
    // 旧的 id 不能再出现在 controls 区域
    expect(/id=["']disconnect-btn["']/.test(src)).toBe(false);
    expect(/id=["']reconnect-btn["']/.test(src)).toBe(false);
  });

  it('#connect-btn 必须带 data-state 属性，初始值为 disconnected', () => {
    const src = html();
    const btnTag = src.match(/<button[^>]*id=["']connect-btn["'][^>]*>/)?.[0] ?? '';
    expect(btnTag).toMatch(/data-state=["']disconnected["']/);
  });

  it('#connect-btn 必须有 data-i18n 默认指向 btnConnect', () => {
    const src = html();
    const btnTag = src.match(/<button[^>]*id=["']connect-btn["'][^>]*>/)?.[0] ?? '';
    expect(btnTag).toMatch(/data-i18n=["']btnConnect["']/);
  });
});

// ── i18n：必须有 btnConnect / btnConnecting / btnDisconnect，禁止删 btnReconnect 因历史列表用 ──
describe('BUG-042 i18n — 三态 key 完整 + 中英双语', () => {
  const i18n = () => read('src/i18n.js');

  it('zh / en 字典都必须包含 btnConnect、btnConnecting、btnDisconnect', () => {
    const src = i18n();
    for (const key of ['btnConnect', 'btnConnecting', 'btnDisconnect']) {
      // 至少出现 2 次（zh + en）
      const matches = src.match(new RegExp(`${key}\\s*:`, 'g')) ?? [];
      expect(matches.length).toBeGreaterThanOrEqual(2);
    }
  });

  it('btnConnecting 中文必须包含"连接中"或"连接 中"', () => {
    const src = i18n();
    const zhLine = src.match(/btnConnecting:\s*['"]([^'"]+)['"]/)?.[1] ?? '';
    expect(/连接中/.test(zhLine)).toBe(true);
  });

  it('btnConnecting 英文必须包含 Connecting', () => {
    const src = i18n();
    // 找第二次匹配（en 字典）
    const re = /btnConnecting:\s*['"]([^'"]+)['"]/g;
    const matches: string[] = [];
    let m;
    while ((m = re.exec(src)) !== null) matches.push(m[1]);
    expect(matches.length).toBeGreaterThanOrEqual(2);
    expect(/Connecting/i.test(matches[1])).toBe(true);
  });

  it('btnReconnect 仍存在（历史列表中的"重连"按钮使用）— 不能误删', () => {
    const src = i18n();
    expect(/btnReconnect:/.test(src)).toBe(true);
  });
});

// ── main.js：appState.connectionPhase + updateConnectButton + handleConnectButtonClick ──
describe('BUG-042 main.js — 单按钮状态机实现', () => {
  const main = () => read('src/main.js');

  it('appState 必须新增 connectionPhase 字段，默认 disconnected', () => {
    const src = main();
    expect(/connectionPhase:\s*['"]disconnected['"]/.test(src)).toBe(true);
  });

  it('必须存在 updateConnectButton(phase) 函数', () => {
    const src = main();
    expect(/function\s+updateConnectButton\s*\(\s*phase\s*\)/.test(src)).toBe(true);
  });

  it('updateConnectButton 必须根据 phase 设置 data-state、textContent、disabled', () => {
    const src = main();
    const fn = src.match(/function\s+updateConnectButton[\s\S]*?\n\}\n/)?.[0] ?? '';
    expect(/setAttribute\(['"]data-state['"]/.test(fn)).toBe(true);
    expect(/textContent\s*=\s*label/.test(fn) || /\.textContent\s*=\s*\w+/.test(fn)).toBe(true);
    expect(/disabled/.test(fn)).toBe(true);
  });

  it('updateConnectButton 三态文案必须分别引用 btnConnect / btnConnecting / btnDisconnect', () => {
    const src = main();
    const fn = src.match(/function\s+updateConnectButton[\s\S]*?\n\}\n/)?.[0] ?? '';
    expect(/btnConnect/.test(fn)).toBe(true);
    expect(/btnConnecting/.test(fn)).toBe(true);
    expect(/btnDisconnect/.test(fn)).toBe(true);
  });

  it('必须存在 handleConnectButtonClick 单按钮 click 分发器', () => {
    const src = main();
    expect(/(async\s+)?function\s+handleConnectButtonClick/.test(src)).toBe(true);
  });

  it('handleConnectButtonClick 必须按 state 分发：connected→handleDisconnect，否则→handleReconnect', () => {
    const src = main();
    const fn = src.match(/(async\s+)?function\s+handleConnectButtonClick[\s\S]*?\n\}/)?.[0] ?? '';
    // connected 路径 → handleDisconnect
    expect(/state\s*===\s*['"]connected['"]/.test(fn) && /handleDisconnect\s*\(/.test(fn)).toBe(true);
    // disconnected 路径 → handleReconnect
    expect(/handleReconnect\s*\(/.test(fn)).toBe(true);
    // connecting 路径 → 早返回
    expect(/state\s*===\s*['"]connecting['"]/.test(fn)).toBe(true);
  });

  it('setupEventListeners 必须只绑定 connect-btn 一个按钮（不再有独立 reconnectBtn 绑定）', () => {
    const src = main();
    expect(/elements\.connectBtn\.addEventListener\(['"]click['"]\s*,\s*handleConnectButtonClick\)/.test(src)).toBe(true);
    // 旧的 reconnectBtn 监听必须删除
    expect(/elements\.reconnectBtn\.addEventListener\(['"]click['"]\s*,\s*handleReconnect/.test(src)).toBe(false);
  });

  it('reconnecting 事件分支必须不调用 setConnectionStatus("connecting")（避免按钮闪到"连接中…"）', () => {
    const src = main();
    // 精确截取 `eventType === 'reconnecting'` 后到下一个 `} else if` 或 `}` 块结束之前的代码片段
    const idx = src.search(/eventType\s*===\s*['"]reconnecting['"]/);
    expect(idx).toBeGreaterThan(0);
    // 从此位置取 1500 字符上限切片，足够覆盖整个 reconnecting 处理块
    const slice = src.slice(idx, idx + 1500);
    // 截取到下一个 } else if / } else 出现位置
    const endIdx = slice.search(/\n\s+\}\s+else\s+if|\n\s+\}\s+else\s*\{/);
    const block = endIdx > 0 ? slice.slice(0, endIdx) : slice;
    expect(block.length).toBeGreaterThan(50);
    // reconnecting 块内不应有 setConnectionStatus('connecting') 调用（按钮不切文案）
    // 排除注释行（以 // 开头）后再检查
    const codeOnly = block.split('\n')
      .map((line) => line.replace(/\/\/.*$/, '')) // 去行注释
      .join('\n');
    expect(/setConnectionStatus\(['"]connecting['"]\)/.test(codeOnly)).toBe(false);
  });

  it('setConnectionStatus 必须同步更新 connectionPhase + 调 updateConnectButton', () => {
    const src = main();
    const fn = src.match(/function\s+setConnectionStatus[\s\S]*?\n\}/)?.[0] ?? '';
    expect(/connectionPhase\s*=\s*status/.test(fn)).toBe(true);
    expect(/updateConnectButton\(/.test(fn)).toBe(true);
  });
});

// ── 兼容性：保留 elements.disconnectBtn 别名以免大量旧引用爆炸 ───────────────
describe('BUG-042 兼容层 — disconnectBtn 别名保留指向同一按钮', () => {
  const main = () => read('src/main.js');

  it('elements.disconnectBtn 仍然存在但指向 connect-btn DOM（兼容旧引用）', () => {
    const src = main();
    // disconnectBtn: document.getElementById('connect-btn') 或注释中说明合并
    expect(/disconnectBtn:\s*document\.getElementById\(['"]connect-btn['"]\)/.test(src)).toBe(true);
  });
});
