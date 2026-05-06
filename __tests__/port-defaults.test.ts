/**
 * v0.1.38 BUG-021：端口默认值混乱修复
 *
 * 根因：kvm-core 定义默认端口 23456、后端 AppConfig.port 默认也是 23456，
 *       但前端 index.html 两处 value="8889"、main.js 多处 "|| 8889" 兜底，
 *       导致 UI 显示与实际配置脱节。用户在设置里改默认端口后，连接对话框
 *       和监听输入框都还在显示 8889。
 *
 * 本测试集验证：
 *   1. src/index.html 的 manual-port-input / listen-port-input 不再硬编码 value="8889"
 *   2. src/main.js 不再出现 "|| 8889" 或字面量 8889 作为端口兜底
 *   3. src/main.js 必须从后端 Platform.getSettings() 填充 appState.defaults.tcpPort
 *   4. src/main.js 至少有一处引用 appState.defaults.tcpPort 作为端口兜底
 *   5. src/platform.js 的 getSettings 默认端口为 23456（保持与 kvm-core 一致）
 *   6. src/connection.js 的手动连接端口兜底不再是 8889
 *   7. src/history.js 的历史重连端口兜底不再是 8889
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');

function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

describe('BUG-021 端口默认值配置链', () => {
  it('src/index.html 中 manual-port-input 和 listen-port-input 都不再硬编码 8889', () => {
    const html = read('src/index.html');
    // 不允许任何 input 的 value="8889"
    const hard = html.match(/value\s*=\s*['"]8889['"]/g);
    expect(hard).toBeNull();
  });

  it('src/main.js 不允许出现 "|| 8889" 硬编码兜底', () => {
    const src = read('src/main.js');
    const hard = src.match(/\|\|\s*8889/g);
    expect(hard).toBeNull();
  });

  it('src/connection.js 不允许出现 "|| 8889" 硬编码兜底', () => {
    const src = read('src/connection.js');
    const hard = src.match(/\|\|\s*8889/g);
    expect(hard).toBeNull();
  });

  it('src/history.js 不允许出现 "|| 8889" 硬编码兜底（如文件存在）', () => {
    const p = path.join(repoRoot, 'src/history.js');
    if (!fs.existsSync(p)) return;
    const src = fs.readFileSync(p, 'utf8');
    expect(src.match(/\|\|\s*8889/g)).toBeNull();
  });

  it('src/main.js 必须声明 appState.defaults 结构并包含 tcpPort 字段', () => {
    const src = read('src/main.js');
    // 初始声明时必须有 defaults: { ... tcpPort: ... }
    const hasDefaultsStructure =
      new RegExp('defaults\\s*:\\s*\\{[^}]*tcpPort\\s*:', 's').test(src) ||
      new RegExp('appState\\.defaults\\s*=\\s*\\{[^}]*tcpPort', 's').test(src);
    expect(hasDefaultsStructure).toBe(true);
  });

  it('src/main.js 必须引用 appState.defaults.tcpPort（至少 1 处）', () => {
    const src = read('src/main.js');
    const matches = src.match(/appState\.defaults[.?]\s*\.?\s*tcpPort|appState\.defaults\?\.tcpPort/g);
    expect(matches).not.toBeNull();
    expect(matches!.length).toBeGreaterThanOrEqual(1);
  });

  it('src/main.js 必须在启动阶段通过 Platform.getSettings 拉取默认端口', () => {
    const src = read('src/main.js');
    // 必须出现一次调用并把 port 字段映射到 appState.defaults.tcpPort
    const hasFetchAndMap =
      /Platform\.getSettings\s*\(\s*\)/.test(src) &&
      /appState\.defaults\s*[=.]/.test(src);
    expect(hasFetchAndMap).toBe(true);
  });

  it('src/platform.js 中 getSettings 默认端口与 kvm-core 一致为 23456', () => {
    const src = read('src/platform.js');
    // 检查 safeInvoke('get_settings', ... { port: 23456 })
    const ok = /port\s*:\s*23456/.test(src);
    expect(ok).toBe(true);
  });

  it('src/platform.js 中 getListenConfig 默认端口与 kvm-core 一致为 23456', () => {
    const src = read('src/platform.js');
    // 允许 settings.port || 23456 这种兜底（因为这里的兜底已经是真源值）
    const usesCorrectDefault = new RegExp('port\\s*:\\s*0\\.0\\.0\\.0.*?23456|settings\\.port\\s*\\|\\|\\s*23456', 's').test(src);
    expect(usesCorrectDefault).toBe(true);
  });

  it('src/connection.js 中手动连接端口兜底为 23456（不再是 8889）', () => {
    const src = read('src/connection.js');
    // parseInt(..., 10) || 23456
    const ok = /parseInt\([^)]+\)\s*\|\|\s*23456/.test(src);
    expect(ok).toBe(true);
  });

  it('整个 src/ 不应再出现字面量 8889 作为端口值', () => {
    const files = ['src/main.js', 'src/connection.js', 'src/history.js', 'src/index.html', 'src/platform.js'];
    for (const f of files) {
      const p = path.join(repoRoot, f);
      if (!fs.existsSync(p)) continue;
      const src = fs.readFileSync(p, 'utf8');
      // 允许注释提到 8889（讲清历史原因），只禁止代码里用 8889 作为值
      // 规则：找所有 8889，如果存在且不在行注释里 → 失败
      const lines = src.split('\n');
      for (const line of lines) {
        if (!line.includes('8889')) continue;
        const trimmed = line.trim();
        if (trimmed.startsWith('//') || trimmed.startsWith('*')) continue;
        // HTML 注释
        if (trimmed.startsWith('<!--')) continue;
        // 找到非注释行里的 8889
        throw new Error(`${f}: 发现非注释行中的 8889：\n  ${trimmed}`);
      }
    }
  });
});
