# BUG-031 横幅权限提示区分 Accessibility / Input Monitoring 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复 macOS 上用户已授予辅助功能权限后仍然看到"需要辅助功能权限"横幅且无法点击「开始共享」的体验问题——让横幅准确指明缺失的是 Accessibility 还是 Input Monitoring，并提供一键跳转对应系统设置的按钮。

**Architecture:**
- 前端 `checkPermissionAfterConnect` 在调用 `showPermissionWarning` 时传入 `kind ∈ {'accessibility', 'input-monitoring'}`，横幅文案与按钮动作均据此路由。
- 后端不改，复用现有 `open_accessibility_settings` / `open_input_monitoring_settings`。
- Input Monitoring 授权后由于 `CGPreflightListenEventAccess` 进程内缓存无法刷新，横幅追加"请重启 KVMini 以生效"的副文案，把物理事实透明化给用户。
- Windows 代码路径（`isWindowsPlatform`）完全不受影响。

**Tech Stack:** 纯前端 JS + i18n（zh/en 双语）+ Jest（jsdom）；Rust 侧零改动。

**版本：** 0.1.38 → 0.1.39（按项目规则 patch +1，唯一真相 `Cargo.toml:9`）。

---

## Task 1: Jest 回归测试——横幅 kind 路由

**Files:**
- Create: `__tests__/permission-warning-kind.test.ts`

**Step 1: Write the failing test**

```ts
/**
 * BUG-031：横幅需按 kind 区分 Accessibility / Input Monitoring
 * 覆盖：
 *   - kind='input-monitoring' 时横幅文案含"输入监控"
 *   - kind='input-monitoring' 时按钮 data-kind='input-monitoring'
 *   - 点击按钮按 kind 调 open_input_monitoring_settings
 *   - 重启提示仅在 input-monitoring 分支出现
 */
import { t, setLocale } from '../src/i18n.js';

describe('BUG-031 横幅按 kind 区分权限类型', () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <div id="permission-warning" class="permission-warning hidden">
        <span class="permission-warning-icon">⚠️</span>
        <span class="permission-warning-text" data-i18n="permissionWarningText"></span>
        <button id="permission-warning-btn" class="permission-warning-btn hidden"></button>
      </div>
    `;
    setLocale('zh');
    // @ts-ignore
    window.i18n = { t, setLocale };
  });

  test('Accessibility 横幅：文案=辅助功能，按钮 data-kind=accessibility', () => {
    require('../src/accessibility.js');
    const elements = {
      permissionWarning: document.getElementById('permission-warning'),
      startSharingBtn: null,
    };
    // @ts-ignore
    window.AccessibilityModule.showPermissionWarning(elements, 'accessibility');
    const text = document.querySelector('.permission-warning-text')!.textContent!;
    const btn = document.getElementById('permission-warning-btn')!;
    expect(text).toMatch(/辅助功能/);
    expect(text).not.toMatch(/输入监控/);
    expect(btn.getAttribute('data-kind')).toBe('accessibility');
    expect(btn.classList.contains('hidden')).toBe(false);
  });

  test('Input Monitoring 横幅：文案=输入监控+重启提示，按钮 data-kind=input-monitoring', () => {
    jest.resetModules();
    require('../src/accessibility.js');
    const elements = {
      permissionWarning: document.getElementById('permission-warning'),
      startSharingBtn: null,
    };
    // @ts-ignore
    window.AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');
    const text = document.querySelector('.permission-warning-text')!.textContent!;
    const btn = document.getElementById('permission-warning-btn')!;
    expect(text).toMatch(/输入监控/);
    expect(text).toMatch(/重启/); // BUG-031 关键：提示用户必须重启
    expect(btn.getAttribute('data-kind')).toBe('input-monitoring');
  });

  test('英文 locale：Input Monitoring 文案含 Input Monitoring + restart', () => {
    setLocale('en');
    jest.resetModules();
    require('../src/accessibility.js');
    const elements = {
      permissionWarning: document.getElementById('permission-warning'),
      startSharingBtn: null,
    };
    // @ts-ignore
    window.AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');
    const text = document.querySelector('.permission-warning-text')!.textContent!;
    expect(text).toMatch(/Input Monitoring/);
    expect(text.toLowerCase()).toMatch(/restart/);
  });

  test('不传 kind 时默认 accessibility（向后兼容）', () => {
    jest.resetModules();
    require('../src/accessibility.js');
    const elements = {
      permissionWarning: document.getElementById('permission-warning'),
      startSharingBtn: null,
    };
    // @ts-ignore
    window.AccessibilityModule.showPermissionWarning(elements);
    const btn = document.getElementById('permission-warning-btn')!;
    expect(btn.getAttribute('data-kind')).toBe('accessibility');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx jest __tests__/permission-warning-kind.test.ts -v`
Expected: 4 个 case 全 FAIL（`showPermissionWarning` 不接受 kind 参数 / 按钮元素不存在 / 文案无"输入监控"）

**Step 3: Commit the failing tests**

```bash
git add __tests__/permission-warning-kind.test.ts
git commit -m "test(BUG-031): add failing regression tests for permission warning kind routing"
```

---

## Task 2: i18n 新增 4 条 key（中英双语）

**Files:**
- Modify: `src/i18n.js`（zh 块 ~83-85、en 块 ~464-465）

**Step 1: 在 zh 块的 `permissionWarningText` 下补 3 条 + 修原条**

把：
```js
    // 权限警告
    permissionWarningText: '需要辅助功能权限才能开始键鼠共享，请先前往授权',
```
替换为：
```js
    // 权限警告（BUG-031 按权限类型分开）
    permissionWarningText: '需要「辅助功能」权限才能开始键鼠共享，请点击右侧按钮前往系统设置授权',
    permissionWarningInputMonitoringText: '需要「输入监控」权限才能开始键鼠共享。授权后请完全退出 KVMini（Cmd+Q）再重新打开，权限才会生效',
    permissionWarningBtnAccessibility: '前往辅助功能设置',
    permissionWarningBtnInputMonitoring: '前往输入监控设置',
```

**Step 2: 在 en 块同位置镜像改动**

把：
```js
    // Permission warning
    permissionWarningText: 'Accessibility permission is required to start sharing. Please authorize first.',
```
替换为：
```js
    // Permission warning (BUG-031: distinguish permission kinds)
    permissionWarningText: 'Accessibility permission is required to start sharing. Click the button on the right to open System Settings.',
    permissionWarningInputMonitoringText: 'Input Monitoring permission is required to start sharing. After authorizing, please fully quit KVMini (Cmd+Q) and relaunch for the permission to take effect.',
    permissionWarningBtnAccessibility: 'Open Accessibility Settings',
    permissionWarningBtnInputMonitoring: 'Open Input Monitoring Settings',
```

**Step 3: Commit**

```bash
git add src/i18n.js
git commit -m "feat(BUG-031): add input-monitoring specific i18n keys with restart hint"
```

---

## Task 3: index.html 横幅加按钮

**Files:**
- Modify: `src/index.html:165-168`

**Step 1: 替换横幅 block**

把：
```html
        <div id="permission-warning" class="permission-warning hidden">
            <span class="permission-warning-icon">⚠️</span>
            <span class="permission-warning-text" data-i18n="permissionWarningText">需要辅助功能权限才能开始键鼠共享，请先前往授权</span>
        </div>
```
替换为：
```html
        <div id="permission-warning" class="permission-warning hidden">
            <span class="permission-warning-icon">⚠️</span>
            <span class="permission-warning-text" data-i18n="permissionWarningText">需要「辅助功能」权限才能开始键鼠共享，请点击右侧按钮前往系统设置授权</span>
            <button id="permission-warning-btn" class="permission-warning-btn hidden" data-kind="accessibility" data-i18n="permissionWarningBtnAccessibility">前往辅助功能设置</button>
        </div>
```

**Step 2: Commit**

```bash
git add src/index.html
git commit -m "feat(BUG-031): add action button to permission warning banner"
```

---

## Task 4: accessibility.js 按 kind 路由横幅

**Files:**
- Modify: `src/accessibility.js:199-246`

**Step 1: `checkPermissionAfterConnect` 传 kind**

定位 `checkPermissionAfterConnect` 内的两处 `showPermissionWarning(elements);`，分别改为：
- 辅助功能分支（`if (ax !== 'Granted')` 之后）：`showPermissionWarning(elements, 'accessibility');`
- 输入监控分支（`if (im !== 'Granted' && im !== 'N/A')` 之后）：`showPermissionWarning(elements, 'input-monitoring');`

**Step 2: `showPermissionWarning` 加 kind 参数 + 文案 + 按钮**

替换 `showPermissionWarning` 函数：
```js
function showPermissionWarning(elements, kind = 'accessibility') {
    const t = (window.i18n && window.i18n.t) ? window.i18n.t : (k) => k;
    if (elements?.permissionWarning) {
        elements.permissionWarning.classList.remove('hidden');
        const textEl = elements.permissionWarning.querySelector('.permission-warning-text');
        const btnEl = elements.permissionWarning.querySelector('.permission-warning-btn') ||
                      document.getElementById('permission-warning-btn');
        if (textEl) {
            const textKey = kind === 'input-monitoring'
                ? 'permissionWarningInputMonitoringText'
                : 'permissionWarningText';
            textEl.textContent = t(textKey);
            textEl.setAttribute('data-i18n', textKey);
        }
        if (btnEl) {
            const btnKey = kind === 'input-monitoring'
                ? 'permissionWarningBtnInputMonitoring'
                : 'permissionWarningBtnAccessibility';
            btnEl.textContent = t(btnKey);
            btnEl.setAttribute('data-i18n', btnKey);
            btnEl.setAttribute('data-kind', kind);
            btnEl.classList.remove('hidden');
            // 幂等绑定：用 onclick 而不是 addEventListener，避免多次连接/断开累积
            const tauri = window.__TAURI__;
            const invoke = tauri?.core?.invoke || tauri?.invoke || null;
            btnEl.onclick = async () => {
                if (!invoke) return;
                const cmd = btnEl.getAttribute('data-kind') === 'input-monitoring'
                    ? 'open_input_monitoring_settings'
                    : 'open_accessibility_settings';
                try { await invoke(cmd); } catch (e) {
                    console.error('[Permission] 打开系统设置失败:', e);
                }
            };
        }
    }
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.disabled = true;
    }
}
```

**Step 3: `hidePermissionWarning` 顺带隐藏按钮**

在 `hidePermissionWarning` 中 `elements.permissionWarning.classList.add('hidden');` 之后追加：
```js
        const btnEl = elements.permissionWarning.querySelector('.permission-warning-btn');
        if (btnEl) btnEl.classList.add('hidden');
```

**Step 4: Run tests to verify they pass**

Run: `npx jest __tests__/permission-warning-kind.test.ts -v`
Expected: 4/4 PASS

**Step 5: Commit**

```bash
git add src/accessibility.js
git commit -m "fix(BUG-031): route permission warning banner by kind with action button"
```

---

## Task 5: 全量回归 + 既有测试不挂

**Step 1: 跑全量 Jest**

Run: `npx jest 2>&1 | tail -6`
Expected: Test Suites: 53 passed, 53 total（+1 个新 suite）；Tests: 878 passed（+4）

**Step 2: 跑 Rust 单测（确认后端零影响）**

Run: `cargo test --workspace --lib 2>&1 | tail -6`
Expected: test result: ok. 111 passed

**Step 3: 如失败，按失败日志定位。不允许跳过。**

---

## Task 6: 版本号 bump 0.1.38 → 0.1.39

**Files:**
- Modify: `Cargo.toml:9` → `version = "0.1.39"`
- Modify: `package.json` version 字段 → `"0.1.39"`
- Modify: `src/index.html` meta[name="app-version"] content → `"0.1.39"`
- Modify: `src-tauri/src/main.rs` 的 `get_changelog()` 顶部插入 v0.1.39 条目

**Step 1: 改 Cargo.toml**

把 `version = "0.1.38"` 改成 `version = "0.1.39"`。

**Step 2: 改 package.json**

同步改成 `"version": "0.1.39"`。

**Step 3: 改 index.html meta**

搜 `app-version`，把 content 改成 `0.1.39`。

**Step 4: changelog 顶部插 v0.1.39 条目**

新增一条（格式参考现有条目）：
- version: "0.1.39"
- date: "2026-05-06"
- changes zh: ["修复 BUG-031：macOS 已授予辅助功能权限后仍显示权限横幅的误导文案", "横幅按缺失权限类型（辅助功能 / 输入监控）显示差异化文案", "横幅新增「前往设置」按钮，一键跳转对应系统偏好设置页", "输入监控权限授权后提示必须重启 App 才能生效"]
- changes en: 对应英文

**Step 5: Commit**

```bash
git add Cargo.toml package.json src/index.html src-tauri/src/main.rs
git commit -m "chore: bump version to 0.1.39 for BUG-031 fix"
```

---

## Task 7: docs/bug-list.md 追加 BUG #031

**Files:**
- Modify: `docs/bug-list.md`（在末尾追加）

**Step 1: 追加记录**

```markdown
## Bug #031：横幅提示"需要辅助功能权限"但实际缺的是输入监控权限

| 字段 | 内容 |
|------|------|
| **现象** | macOS 上用户在系统设置里已开启 KVMini 的「辅助功能」开关，但横幅仍显示"需要辅助功能权限才能开始键鼠共享"，且「开始共享」按钮始终禁用 |
| **根因** | `checkPermissionAfterConnect` 同时校验 Accessibility + Input Monitoring 两种权限，任一 Denied 都亮横幅，但 `permissionWarningText` 只写了"辅助功能"，用户反复检查辅助功能开关却没发现真正未授予的是「输入监控」；叠加 `CGPreflightListenEventAccess` 进程内缓存，用户即使发现了去授权也必须重启 App 才能生效 |
| **修改方式** | 1) `showPermissionWarning` 加 kind 参数区分两种权限；2) i18n 新增 `permissionWarningInputMonitoringText`（含"请重启 KVMini"提示）+ 两条按钮文案；3) 横幅 HTML 加「前往设置」按钮，按 data-kind 路由到 `open_accessibility_settings` 或 `open_input_monitoring_settings`；4) `checkPermissionAfterConnect` 两条分支分别传 'accessibility' / 'input-monitoring' |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/permission-warning-kind.test.ts`（4 个 case 覆盖中英 locale + 两种 kind + 默认值向后兼容） |
```

**Step 2: Commit**

```bash
git add docs/bug-list.md
git commit -m "docs(BUG-031): append bug record to bug-list"
```

---

## Task 8: 双架构构建 + 归档 + 打开 Finder

**Step 1: 跑构建脚本（禁止再次 bump）**

Run: `NO_BUMP=1 bash build.sh`（如 build.sh 不识别 NO_BUMP 环境变量，回退手动：`cargo tauri build --target aarch64-apple-darwin && cargo tauri build --target x86_64-apple-darwin`）
Expected: 两份 DMG 产出，文件名含 `0.1.39`

**Step 2: 验证签名**

Run:
```bash
codesign -dv --verbose=2 target/aarch64-apple-darwin/release/bundle/macos/KVMini.app 2>&1 | grep Authority
codesign -dv --verbose=2 target/x86_64-apple-darwin/release/bundle/macos/KVMini.app 2>&1 | grep Authority
```
Expected: 两个 .app 均有 `Authority=Developer ID Application: han yufei (8F772XVA9L)`

**Step 3: 归档文档**

在 `docs/archive/20260506/` 下复用现有三件套的风格新增：
- `bug-fix-v0.1.39-BUG-031.md`（简短总结：现象 / 根因 / 修复范围 / 测试 / 构建产物路径）

**Step 4: 打开 Finder**

Run: `open -R target/aarch64-apple-darwin/release/bundle/dmg/KVMini_0.1.39_aarch64.dmg`

**Step 5: 最终 Commit**

```bash
git add docs/archive/20260506/bug-fix-v0.1.39-BUG-031.md
git commit -m "[0.1.39] BUG-031 横幅权限提示按类型区分 + 前往设置按钮"
```

---

## Done Definition

- [x] Jest 全量 + Rust cargo test 全绿
- [x] 双架构 DMG 带 Developer ID 签名
- [x] `docs/bug-list.md` 追加 BUG #031
- [x] `docs/archive/20260506/bug-fix-v0.1.39-BUG-031.md` 归档
- [x] index.html 横幅内肉眼可见「前往设置」按钮
- [x] 中英文 locale 切换文案都正确
- [x] 版本号统一到 0.1.39（Cargo.toml / package.json / index.html meta / changelog）

## 用户后续手工验收路径

1. 安装新 DMG 覆盖旧版，启动 KVMini
2. 连接远端设备
3. **反向场景**（已授辅助功能、未授输入监控）：横幅应显示"需要「输入监控」权限……请完全退出 KVMini 再重新打开"，按钮文案=「前往输入监控设置」
4. 点按钮 → 直接跳转到系统设置→隐私→输入监控
5. 打开开关 → Cmd+Q → 重启 → 横幅消失，「开始共享」可点
