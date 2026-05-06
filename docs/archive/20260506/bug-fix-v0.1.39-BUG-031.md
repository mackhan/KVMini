# KVMini v0.1.39 发布归档 — BUG-031 权限横幅误报修复

**发布日期**: 2026-05-06
**版本号**: 0.1.39
**构建产物**:
- `target/aarch64-apple-darwin/release/bundle/dmg/KVMini_0.1.39_aarch64.dmg` （6.3 MB，M 芯片）
- `target/x86_64-apple-darwin/release/bundle/dmg/KVMini_0.1.39_x64.dmg` （6.5 MB，Intel）
- 签名：Developer ID Application: han yufei (8F772XVA9L)

---

## 一、问题现场

用户反馈：KVMini 已在「系统设置 → 隐私与安全性 → 辅助功能」中授予权限并看到 ✅ 绿色开关，但主界面仍显示「需要辅助功能权限」横幅，且「开始共享」按钮保持禁用。

**截图表现**：
- 设备列表已成功连接到对端
- 横幅文案：「需要辅助功能权限才能捕获鼠标键盘」
- 按钮：灰色不可点

---

## 二、第一性原理根因分析

macOS 需要两个独立的 TCC 权限才能完成 KVM 共享：

| 权限 | 用途 | 判定 API |
|---|---|---|
| **Accessibility（辅助功能）** | 向本机**注入**鼠标键盘事件（被控端） | `AXIsProcessTrusted()` |
| **Input Monitoring（输入监控）** | **捕获** CGEventTap 全局键鼠事件（主控端） | `CGPreflightListenEventAccess()` |

**关键 Bug**：
`src/accessibility.js::checkPermissionAfterConnect()` 会**同时**判定两个权限，但当任一权限缺失时**都调用同一条文案** `showPermissionWarning()`，而这条文案只说"辅助功能"。

→ 当用户仅授权了 Accessibility 但缺 Input Monitoring 时，看到的横幅却在说辅助功能，陷入「明明授权了为什么还提示」的死循环。

**次级 Bug**：
`CGPreflightListenEventAccess` 在进程内有缓存，授权 Input Monitoring 后必须**重启 App（Cmd+Q 完全退出）**才会刷新判定结果。旧文案从未告知用户这一点。

---

## 三、修复方案（奥卡姆剃刀 + 最小改动）

### 1. 数据流：kind 参数路由

在 `checkPermissionAfterConnect` 中分别判定两种权限，按缺失类型给 `showPermissionWarning(elements, kind)` 传递 `'accessibility'` / `'input-monitoring'`，由该函数按 kind 渲染横幅文案、按钮文案、按钮 `data-kind` 属性，并绑定相应的系统设置打开命令。

### 2. DOM 按钮替代闭包捕获

横幅改为包含一个按钮 `#permission-warning-btn`，按钮 `data-kind` 属性决定点击时 invoke 的命令：
- `data-kind="accessibility"` → `open_accessibility_settings`
- `data-kind="input-monitoring"` → `open_input_monitoring_settings`

每次 `showPermissionWarning` 调用使用 `btnEl.onclick = ...` 覆盖式赋值，避免监听器累积。

### 3. i18n 中英双语同步

`src/i18n.js` zh / en 两个字典各新增 3 个 key：
- `permissionWarningInputMonitoringText` — 必须明示「授权后需重新打开 KVMini (Cmd+Q)」
- `permissionWarningBtnAccessibility` — 「前往辅助功能设置 / Open Accessibility Settings」
- `permissionWarningBtnInputMonitoring` — 「前往输入监控设置 / Open Input Monitoring Settings」

并修订 `permissionWarningText` 精确到「辅助功能权限」以对齐 kind 语义。

### 4. applyI18n 动态 key 刷新

`src/main.js::applyI18n()` 改为从 DOM 元素的 `data-i18n` 属性读取 key 再调 `t()`，避免语言切换时把 input-monitoring 横幅回刷为 accessibility 文案。

---

## 四、测试覆盖（TDD）

新建 `__tests__/permission-warning-kind.test.ts`，11 个 case：

| # | 用例 | 校验目标 |
|---|---|---|
| 1 | i18n zh 字典含 4 个权限 key | 完整性 |
| 2 | i18n en 字典含 4 个权限 key | 完整性 |
| 3 | input-monitoring zh 文案必须包含「重新打开」 | 关键提示 |
| 4 | input-monitoring en 文案必须包含 "relaunch" 或 "restart" | 关键提示 |
| 5 | showPermissionWarning(kind=accessibility) 设置横幅 + 按钮 | 路由分支 |
| 6 | showPermissionWarning(kind=input-monitoring) 切换文案 + data-kind | 路由分支 |
| 7 | showPermissionWarning 默认 kind 为 accessibility | 兼容既有调用 |
| 8 | hidePermissionWarning 同步隐藏按钮 | 按钮不残留 |
| 9 | 按钮 onclick 按 data-kind 调 open_accessibility_settings | 幂等绑定 |
| 10 | 按钮 onclick 按 data-kind 调 open_input_monitoring_settings | 幂等绑定 |
| 11 | showPermissionWarning 反复调用不累加 click 监听器 | 幂等性 |

**回归测试全量结果**：
- Jest：53 suites / **879 tests passed**
- Rust：`cargo test --workspace` **111 tests passed**
- cargo check --workspace：编译通过（仅 pre-existing dead_code warning）

---

## 五、变更文件清单

### BUG-031 核心修复
- `src/accessibility.js` — kind 参数路由 + 按钮绑定幂等化 + hidePermissionWarning 同步隐藏按钮
- `src/i18n.js` — zh/en 字典各 +3 key，修订 permissionWarningText
- `src/index.html` — 横幅内新增 `#permission-warning-btn`；app-version meta → 0.1.39
- `src/main.js` — applyI18n 按 data-i18n 属性动态刷新横幅文案和按钮文案
- `src/styles.css` — `.permission-warning-btn` 基础样式 + hover/active 状态
- `__tests__/permission-warning-kind.test.ts` — 新建，11 个 case

### v0.1.38 遗漏代码并入（方案 B）
- `src-tauri/src/sharing.rs` — TLS 证书与指纹核对、连接建立路径增强
- `src-tauri/src/main.rs` — get_changelog 新增 v0.1.39 条目（中英双语）
- `docs/bug-list.md` — 追加 BUG-031 完整记录

### 版本号同步
- `Cargo.toml` — 0.1.38 → 0.1.39
- `Cargo.lock` — 自动同步
- `package.json` — 0.1.38 → 0.1.39
- `src/index.html` — meta app-version → 0.1.39

### 文档
- `docs/plans/2026-05-06-BUG-031-permission-warning-kind.md` — 8 Task 实施计划
- `docs/archive/20260506/bug-fix-v0.1.39-BUG-031.md` — 本文件

---

## 六、兼容性确认

- **Mac M 芯片（aarch64）**：构建签名通过，6.3MB dmg
- **Mac Intel（x86_64）**：构建签名通过，6.5MB dmg
- **Windows**：本次改动均在 src/ 前端 + i18n + 通用 main.rs，无平台特定代码分叉；Tauri 命令 `open_accessibility_settings` / `open_input_monitoring_settings` 在非 macOS 平台会被 no-op 或返回错误（现有行为保留）。权限横幅本身只在 macOS 下由 `platform.js::isMacOS()` 触发显示，Win 侧不受影响。

---

## 七、后续注意

1. 用户在「输入监控」中授权后，必须 Cmd+Q 完全退出并重新打开 KVMini 才会生效，文案已明示。
2. 横幅按钮点击打开的是 macOS 系统设置 App，不同系统版本（13+ Ventura / 14+ Sonoma / 26 Tahoe beta）面板路径可能略有差异，但 `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` / `...?Privacy_ListenEvent` URL scheme 在 13~26 beta 均已验证可用。
3. 下次如果出现类似「权限看似已授但横幅仍提示」反馈，优先排查是不是混淆了 Accessibility / Input Monitoring 两类权限，而不是怀疑系统缓存。
