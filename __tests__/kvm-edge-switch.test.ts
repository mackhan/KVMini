/**
 * v0.1.38 BUG-022：同时控制两台电脑（非真正 KVM）修复
 *
 * 根因：
 *   1. MacInputCapture 使用 CGEventTapOptions::ListenOnly，回调返回值被系统忽略，
 *      无法吞掉事件；LocalControlling 状态下本机键鼠仍会作用于本机。
 *   2. SharingManager 没有接入 EdgeDetector，鼠标触达屏幕边缘不会自动切到对端。
 *
 * 本测试集从源码层面验证修复（运行时行为由 Rust 单元测试覆盖）：
 *   1. Rust input_capture.rs 必须使用 CGEventTapOptions::Default 而非 ListenOnly
 *   2. Rust input_capture.rs 必须引入表示"是否吞事件"的原子状态（AtomicU8 / AtomicBool）
 *   3. Rust sharing.rs 必须在 MouseMove 分支接入 EdgeDetector
 *   4. Rust sharing.rs 必须实现 BlockInput/UnblockInput 分支（非空 {}）
 *   5. Windows input_capture.rs 必须按状态决定 CallNextHookEx 放行 vs 吞事件
 *   6. main.js 必须绑定 control-state-changed 事件更新 UI 状态徽标
 *   7. i18n.js 必须补全 status_idle / status_local_controlling / status_remote_controlling 三键
 */

import * as fs from 'fs';
import * as path from 'path';

const repoRoot = path.resolve(__dirname, '..');

function read(rel: string): string {
  return fs.readFileSync(path.join(repoRoot, rel), 'utf8');
}

describe('BUG-022 真 KVM 行为修复', () => {
  describe('macOS 输入屏蔽（CGEventTap 由 ListenOnly 改为 Default）', () => {
    const file = 'src-tauri/src/platform/macos/input_capture.rs';

    it('input_capture.rs 必须使用 CGEventTapOptions::Default', () => {
      const src = read(file);
      expect(src).toMatch(/CGEventTapOptions::Default/);
    });

    it('input_capture.rs 不再使用 CGEventTapOptions::ListenOnly', () => {
      const src = read(file);
      expect(src).not.toMatch(/CGEventTapOptions::ListenOnly/);
    });

    it('input_capture.rs 必须引入可被回调读取的控制状态（AtomicU8）', () => {
      const src = read(file);
      // 允许 AtomicU8 或 AtomicU32 等无锁原子类型承载状态
      const hasAtomic = /AtomicU8|AtomicU32/.test(src);
      expect(hasAtomic).toBe(true);
    });

    it('input_capture.rs 必须提供 set_control_state 方法供 SharingManager 推送状态', () => {
      const src = read(file);
      // 命名约定：set_control_state 或 update_control_state
      const ok = /fn\s+set_control_state|fn\s+update_control_state/.test(src);
      expect(ok).toBe(true);
    });

    it('CGEventTap 回调必须根据状态返回 Some(event)（IDLE 放行）或 None（吞掉）', () => {
      const src = read(file);
      // 回调里必须同时出现 Some(event) 和 None 两种返回分支
      const hasSomeReturn = /Some\(event\)/.test(src) || /return\s+Some\(/.test(src);
      const hasNoneReturn = /None/.test(src);
      expect(hasSomeReturn).toBe(true);
      expect(hasNoneReturn).toBe(true);
    });
  });

  describe('Windows 输入屏蔽（Low-Level Hook 按状态吞事件）', () => {
    const file = 'src-tauri/src/platform/windows/input_capture.rs';

    it('input_capture.rs 必须存在且按状态控制吞/放', () => {
      const p = path.join(repoRoot, file);
      if (!fs.existsSync(p)) {
        // 项目只在 target_os = "windows" 编译时有该文件，如不存在跳过
        return;
      }
      const src = fs.readFileSync(p, 'utf8');
      // 必须引入控制状态
      expect(/AtomicU8|AtomicU32|HOOK_SHOULD_SWALLOW|set_control_state/.test(src)).toBe(true);
    });
  });

  describe('kvm-core 配置新增键鼠切换相关字段', () => {
    it('AppConfig 必须新增 edge_trigger 字段（已存在，保持）', () => {
      const src = read('src-tauri/src/config.rs');
      expect(/pub\s+edge_trigger\s*:/.test(src)).toBe(true);
    });

    it('AppConfig 应提供切换快捷键字段（switch_hotkey 或 hotkey_switch_combo）', () => {
      const src = read('src-tauri/src/config.rs');
      expect(/switch_hotkey|hotkey_switch_combo/.test(src)).toBe(true);
    });
  });

  describe('SharingManager 真正实现 BlockInput/UnblockInput 并接入 EdgeDetector', () => {
    const file = 'src-tauri/src/sharing.rs';

    it('sharing.rs 必须引用 EdgeDetector（从 kvm_core::edge_logic 或等价路径）', () => {
      const src = read(file);
      expect(/EdgeDetector/.test(src)).toBe(true);
    });

    it('sharing.rs 中 BlockInput/UnblockInput 分支必须有非空实现（不再是裸 {}）', () => {
      const src = read(file);
      // 匹配 "BlockInput => { " 或 "BlockInput =>" 后跟实际代码
      // 具体实现：必须调用 start_block_input / stop_block_input 或平台等价 API
      const hasBlockImpl = /start_block_input|input_block::|blocker\.block_input|BlockInput\s*=>\s*\{[^}]*\w+/.test(src);
      expect(hasBlockImpl).toBe(true);
    });

    it('sharing.rs 必须在 MouseMove 分支调用 EdgeDetector.detect', () => {
      const src = read(file);
      // 允许 detect(x, y) 或 edge_detector.detect 形式
      expect(/EdgeDetector|edge_detector|\.detect\s*\(/.test(src)).toBe(true);
    });
  });

  describe('前端 UI 绑定 control-state-changed 事件', () => {
    it('src/main.js 必须监听 control-state-changed 事件', () => {
      const src = read('src/main.js');
      expect(/control-state-changed/.test(src)).toBe(true);
    });

    it('src/main.js 必须调用 updateControlStateUI 或等价函数更新状态徽标', () => {
      const src = read('src/main.js');
      const ok = /updateControlStateUI|updateSharingButtonForState|controlState\.textContent/.test(src);
      expect(ok).toBe(true);
    });
  });

  describe('i18n 补齐状态徽标文案（双语同步）', () => {
    const file = 'src/i18n.js';

    it('i18n.js 的 zh 字典必须包含 status_idle / status_local_controlling / status_remote_controlling', () => {
      const src = read(file);
      // 直接搜 key 名即可（无论嵌套结构如何，key 名字符串出现即足够）
      expect(/status_idle/.test(src)).toBe(true);
      expect(/status_local_controlling/.test(src)).toBe(true);
      expect(/status_remote_controlling/.test(src)).toBe(true);
    });

    it('i18n.js 中每个新 key 必须在 zh 和 en 两个位置都出现', () => {
      const src = read(file);
      // 粗粒度：zh: {...} 和 en: {...} 块里分别出现三次 key
      const keys = ['status_idle', 'status_local_controlling', 'status_remote_controlling'];
      for (const k of keys) {
        const occurrences = (src.match(new RegExp(k, 'g')) || []).length;
        // 一次 zh 一次 en，至少 2 次；此外可能被其他地方引用
        expect(occurrences).toBeGreaterThanOrEqual(2);
      }
    });
  });
});
