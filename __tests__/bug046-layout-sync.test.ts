/**
 * BUG-046 (v0.1.60): 显示器布局两端不同步 —— 鼠标切过去回不来
 *
 * 修复方案：发起连接方为准 + 被动方自动镜像（类似 Logitech Flow）
 * - HandshakePayload 新增 layout_direction 字段
 * - ScreenLayout 新增 mirror() 方法
 * - 被动方收到握手后自动镜像，保存 layout.json，emit layout-synced 前端事件
 * - 前端监听 layout-synced 事件，刷新 UI + Toast 提示
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');
const read = (rel: string) => fs.readFileSync(path.join(repoRoot, rel), 'utf8');

// ── i18n: 必须有 layoutAutoSynced 文案（中英双语） ──────────────────────────

describe('BUG-046 i18n — 布局同步文案', () => {
  const i18n = () => read('src/i18n.js');

  it('zh / en 字典都必须包含 layoutAutoSynced', () => {
    const src = i18n();
    const matches = src.match(/layoutAutoSynced\s*:/g) ?? [];
    // 至少 2 次（zh + en）
    expect(matches.length).toBeGreaterThanOrEqual(2);
  });

  it('zh 字典 layoutAutoSynced 必须包含中文', () => {
    const src = i18n();
    // 匹配第一个 layoutAutoSynced（zh 字典在前）
    const zhMatch = src.match(/layoutAutoSynced\s*:\s*['"]([^'"]+)['"]/);
    expect(zhMatch).toBeTruthy();
    // 必须包含中文字符
    expect(zhMatch![1]).toMatch(/[\u4e00-\u9fff]/);
  });

  it('en 字典 layoutAutoSynced 必须包含英文', () => {
    const src = i18n();
    // 找所有 layoutAutoSynced 匹配
    const allMatches = [...src.matchAll(/layoutAutoSynced\s*:\s*['"]([^'"]+)['"]/g)];
    expect(allMatches.length).toBeGreaterThanOrEqual(2);
    // 第二个是 en 字典
    const enValue = allMatches[1][1];
    expect(enValue).toMatch(/[a-zA-Z]/);
  });
});

// ── 前端: layout-synced 事件监听 ────────────────────────────────────────────

describe('BUG-046 前端 — layout-synced 事件监听', () => {
  const mainJs = () => read('src/main.js');

  it('main.js 必须注册 layout-synced 事件监听', () => {
    const src = mainJs();
    expect(src).toContain("'layout-synced'");
  });

  it('layout-synced 处理中必须检查 was_changed 字段', () => {
    const src = mainJs();
    // 找到 layout-synced 监听代码块
    const block = src.match(/layout-synced[\s\S]*?\.catch/);
    expect(block).toBeTruthy();
    expect(block![0]).toContain('was_changed');
  });

  it('layout-synced 处理中必须更新 window._currentLayout', () => {
    const src = mainJs();
    const block = src.match(/layout-synced[\s\S]*?\.catch/);
    expect(block).toBeTruthy();
    expect(block![0]).toContain('_currentLayout');
  });

  it('layout-synced 处理中 was_changed=true 时必须调用 showSuccess', () => {
    const src = mainJs();
    const block = src.match(/layout-synced[\s\S]*?\.catch/);
    expect(block).toBeTruthy();
    expect(block![0]).toContain('showSuccess');
  });

  it('layout-synced 处理中必须使用 layoutAutoSynced i18n key', () => {
    const src = mainJs();
    const block = src.match(/layout-synced[\s\S]*?\.catch/);
    expect(block).toBeTruthy();
    expect(block![0]).toContain('layoutAutoSynced');
  });
});

// ── Rust kvm-core: ScreenLayout mirror 测试覆盖 ────────────────────────────

describe('BUG-046 Rust 源码 — ScreenLayout::mirror 存在性', () => {
  const configRs = () => read('kvm-core/src/config.rs');

  it('ScreenLayout 必须有 mirror() 方法', () => {
    const src = configRs();
    expect(src).toContain('fn mirror');
    expect(src).toContain('Self::LocalLeft => Self::LocalRight');
    expect(src).toContain('Self::LocalRight => Self::LocalLeft');
    expect(src).toContain('Self::LocalAbove => Self::LocalBelow');
    expect(src).toContain('Self::LocalBelow => Self::LocalAbove');
  });

  it('ScreenLayout 必须有 to_layout_string() 方法', () => {
    const src = configRs();
    expect(src).toContain('fn to_layout_string');
    expect(src).toContain('LocalLeftOfRemote');
    expect(src).toContain('LocalRightOfRemote');
  });

  it('ScreenLayout 必须有 from_layout_string() 方法', () => {
    const src = configRs();
    expect(src).toContain('fn from_layout_string');
  });

  it('config.rs 测试中必须覆盖 mirror 方法', () => {
    const src = configRs();
    expect(src).toContain('test_screen_layout_mirror');
  });

  it('config.rs 测试中必须覆盖 round_trip', () => {
    const src = configRs();
    expect(src).toContain('test_screen_layout_round_trip');
  });
});

// ── Rust kvm-core: HandshakePayload 扩展 ────────────────────────────────────

describe('BUG-046 Rust 源码 — HandshakePayload layout_direction', () => {
  const networkRs = () => read('kvm-core/src/network.rs');

  it('HandshakePayload 必须包含 layout_direction 字段', () => {
    const src = networkRs();
    expect(src).toContain('layout_direction');
  });

  it('layout_direction 字段必须有 #[serde(default)] 标注（向后兼容）', () => {
    const src = networkRs();
    // 在 HandshakePayload 结构体定义附近
    const payloadBlock = src.match(/struct HandshakePayload[\s\S]*?}/);
    expect(payloadBlock).toBeTruthy();
    // layout_direction 前必须有 serde(default)
    const layoutIdx = payloadBlock![0].indexOf('layout_direction');
    expect(layoutIdx).toBeGreaterThan(-1);
    // 向前找 serde(default)
    const before = payloadBlock![0].substring(Math.max(0, layoutIdx - 100), layoutIdx);
    expect(before).toContain('serde(default');
  });

  it('HandshakeInfo 必须包含 layout_direction 字段', () => {
    const src = networkRs();
    const infoBlock = src.match(/pub struct HandshakeInfo[\s\S]*?}/);
    expect(infoBlock).toBeTruthy();
    expect(infoBlock![0]).toContain('layout_direction');
  });

  it('NetworkEvent 必须包含 LayoutSynced 变体', () => {
    const src = networkRs();
    expect(src).toContain('LayoutSynced');
    expect(src).toContain('new_layout');
    expect(src).toContain('was_changed');
  });

  it('网络测试中必须覆盖 layout_direction 携带和兼容', () => {
    const src = networkRs();
    expect(src).toContain('test_handshake_payload_carries_layout_direction');
    expect(src).toContain('test_handshake_payload_layout_direction_none_compat');
  });
});

// ── Rust src-tauri: connection/mod.rs layout-synced 事件转发 ────────────────

describe('BUG-046 Rust src-tauri — layout-synced 事件转发', () => {
  const connectionMod = () => read('src-tauri/src/connection/mod.rs');

  it('connection/mod.rs 必须处理 LayoutSynced 事件', () => {
    const src = connectionMod();
    expect(src).toContain('LayoutSynced');
  });

  it('start_event_loop 中必须 emit layout-synced 前端事件', () => {
    const src = connectionMod();
    expect(src).toContain('"layout-synced"');
  });

  it('start_event_loop 中必须保存 layout.json', () => {
    const src = connectionMod();
    expect(src).toContain('save_layout_config');
  });

  it('start_event_loop 中必须重建 EdgeDetector', () => {
    const src = connectionMod();
    expect(src).toContain('rebuild_edge_detector');
  });
});

// ── Rust src-tauri: sharing.rs rebuild_edge_detector ────────────────────────

describe('BUG-046 Rust src-tauri — SharingManager::rebuild_edge_detector', () => {
  const sharingRs = () => read('src-tauri/src/sharing.rs');

  it('SharingManager 必须有 rebuild_edge_detector 方法', () => {
    const src = sharingRs();
    expect(src).toContain('pub async fn rebuild_edge_detector');
  });
});

// ── Rust src-tauri: platform/mod.rs 握手携带 layout ────────────────────────

describe('BUG-046 Rust src-tauri — local_handshake_info 携带 layout', () => {
  const platformMod = () => read('src-tauri/src/platform/mod.rs');

  it('local_handshake_info 必须读取 layout_config 并携带 layout_direction', () => {
    const src = platformMod();
    expect(src).toContain('load_layout_config');
    expect(src).toContain('layout_direction');
    expect(src).toContain('new_with_tls_and_layout');
  });
});
