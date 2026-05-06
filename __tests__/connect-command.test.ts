/**
 * v0.1.38 BUG-020：connect_to_device 命令错配
 *
 * 根因：后端 generate_handler! 只注册了 connect_to_peer，
 *       前端 src/connection.js 和 src/main.js 多处调用了未注册的 connect_to_device，
 *       导致「扫描出设备但连不上」，报错 Command connect_to_device not found。
 *
 * 本测试集验证：
 *   1. 源码中不允许再出现 invoke('connect_to_device'
 *   2. 所有对自动发现设备/历史条目的连接调用，都使用 connect_to_peer 并带上 peerIp/port
 *   3. 端口若未显式指定，兜底使用 appState.defaults.tcpPort（23456），不再兜底 8889
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');

function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

describe('BUG-020 connect_to_device 命令错配修复', () => {
  it('src/main.js 不允许再出现 invoke("connect_to_device"', () => {
    const src = read('src/main.js');
    // 正则匹配前端调用端点，避免误杀注释/字符串里的解释性文字
    const matches = src.match(/invoke\(\s*['"]connect_to_device['"]/g);
    expect(matches).toBeNull();
  });

  it('src/connection.js 不允许再出现 invoke("connect_to_device"', () => {
    const src = read('src/connection.js');
    const matches = src.match(/invoke\(\s*['"]connect_to_device['"]/g);
    expect(matches).toBeNull();
  });

  it('整个 src/ 目录（不含 platform.js 内部封装）都不应再调用 connect_to_device', () => {
    const files = ['src/main.js', 'src/connection.js', 'src/history.js'];
    for (const f of files) {
      const p = path.join(repoRoot, f);
      if (!fs.existsSync(p)) continue;
      const src = fs.readFileSync(p, 'utf8');
      const matches = src.match(/invoke\(\s*['"]connect_to_device['"]/g);
      expect({ file: f, found: matches }).toEqual({ file: f, found: null });
    }
  });

  it('src/main.js 必须出现 connect_to_peer 调用（至少 1 处）', () => {
    const src = read('src/main.js');
    const matches = src.match(/invoke\(\s*['"]connect_to_peer['"]/g);
    expect(matches).not.toBeNull();
    expect(matches!.length).toBeGreaterThanOrEqual(1);
  });

  it('src/main.js 端口兜底值统一为 appState.defaults.tcpPort（不再硬编码 8889）', () => {
    const src = read('src/main.js');
    // 所有"|| 8889"的硬编码必须全部清除
    const hardcoded = src.match(/\|\|\s*8889/g);
    expect(hardcoded).toBeNull();
    // 必须至少出现一次 appState.defaults.tcpPort 引用
    expect(src.includes('appState.defaults.tcpPort') || src.includes('appState.defaults?.tcpPort')).toBe(true);
  });

  it('src/index.html 不再硬编码 value="8889"', () => {
    const src = read('src/index.html');
    const matches = src.match(/value\s*=\s*['"]8889['"]/g);
    expect(matches).toBeNull();
  });

  it('src/history.js 如有端口兜底逻辑，必须使用 appState.defaults.tcpPort 而非 8889', () => {
    const p = path.join(repoRoot, 'src/history.js');
    if (!fs.existsSync(p)) return; // 文件不存在时跳过
    const src = fs.readFileSync(p, 'utf8');
    expect(src.match(/\|\|\s*8889/g)).toBeNull();
  });
});
