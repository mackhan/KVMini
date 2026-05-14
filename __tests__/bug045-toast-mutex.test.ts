/**
 * BUG-045 (v0.1.60): Toast 互斥缺失 —— "未连接"和"已连接"同时显示
 *
 * 根因：showSuccess / showReconnectToast / showError 各自独立操作对应的 Toast 元素，
 *       互不干扰。网络抖动时（先 disconnected 后立即 connected），两个 Toast 同时可见。
 *
 * 修复：每个 show*Toast 函数入口处先隐藏其他 Toast + 清除对方定时器。
 *       同一时间只显示一个状态 Toast。
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
const read = (rel: string) => fs.readFileSync(path.join(repoRoot, rel), 'utf8');

// ── 源码静态分析 ────────────────────────────────────────────────────────────

describe('BUG-045 源码: showSuccess 互斥', () => {
  const mainJs = () => read('src/main.js');

  it('showSuccess 函数体中必须隐藏 reconnectToast', () => {
    const src = mainJs();
    // 匹配 showSuccess 函数定义到下一个 function/let/const/var 声明之间
    const fnMatch = src.match(/function showSuccess\([\s\S]*?(?=\nfunction |\nlet |\nconst |\nvar |\n\/\/ ─)/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('reconnectToast');
    expect(fnBody).toMatch(/reconnectToast.*classList.*add.*hidden|reconnectToast.*hidden/);
  });

  it('showSuccess 函数体中必须清除 reconnectToastTimer', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showSuccess\([\s\S]*?(?=\nfunction |\nlet |\nconst |\nvar |\n\/\/ ─)/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(/clearTimeout\s*\(\s*reconnectToastTimer\s*\)/);
  });

  it('showSuccess 函数体中必须隐藏 errorToast', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showSuccess\([\s\S]*?(?=\nfunction |\nlet |\nconst |\nvar |\n\/\/ ─)/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('errorToast');
  });

  it('showSuccess 函数体中必须调用 hideReconnectStatus()', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showSuccess\([\s\S]*?(?=\nfunction |\nlet |\nconst |\nvar |\n\/\/ ─)/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('hideReconnectStatus()');
  });
});

describe('BUG-045 源码: showReconnectToast 互斥', () => {
  const mainJs = () => read('src/main.js');

  it('showReconnectToast 函数体中必须隐藏 successToast', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showReconnectToast\([\s\S]*?(?=\n\/\/ |\nfunction |\nlet |\nconst )/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('successToast');
    expect(fnBody).toMatch(/successToast.*classList.*add.*hidden|successToast.*hidden/);
  });

  it('showReconnectToast 函数体中必须清除 successTimer', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showReconnectToast\([\s\S]*?(?=\n\/\/ |\nfunction |\nlet |\nconst )/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toMatch(/clearTimeout\s*\(\s*successTimer\s*\)/);
  });

  it('showReconnectToast 函数体中必须隐藏 errorToast', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showReconnectToast\([\s\S]*?(?=\n\/\/ |\nfunction |\nlet |\nconst )/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('errorToast');
  });
});

describe('BUG-045 源码: showError 互斥', () => {
  const mainJs = () => read('src/main.js');

  it('showError 函数体中必须隐藏 successToast', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showError\([\s\S]*?(?=\n\/\/ |\nfunction |\nlet |\nconst )/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('successToast');
  });

  it('showError 函数体中必须隐藏 reconnectToast', () => {
    const src = mainJs();
    const fnMatch = src.match(/function showError\([\s\S]*?(?=\n\/\/ |\nfunction |\nlet |\nconst )/);
    expect(fnMatch).toBeTruthy();
    const fnBody = fnMatch![0];
    expect(fnBody).toContain('reconnectToast');
  });
});
