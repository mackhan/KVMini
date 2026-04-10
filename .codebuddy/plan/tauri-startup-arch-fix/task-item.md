# 实施计划：Tauri 启动架构彻底修复

## 背景

根因：macOS WKWebView 会在 `<script src="main.js">` 执行完成后清除该脚本中新增到 `window` 的属性，导致 `main.js` 中定义的所有启动函数在后续 `bootstrapApp` 内联脚本中不可见（均为 `undefined`），应用永远无法启动。

**修复方案：** 将 `bootstrapApp` 内联脚本移到 `<script src="main.js">` 之前，只负责捕获 `invoke` 到 `window.tauriInvokeReady`；`main.js` 末尾直接读取该值并调用 `initApp()`。

---

- [ ] 1. 先写测试：为新启动架构编写 TDD 测试用例
   - 在 `__tests__/tauri-startup-arch.test.js` 中新增测试
   - 覆盖：`tauriInvokeReady` 有值时直接调用 `initApp()`
   - 覆盖：`tauriInvokeReady` 为 null 时走 `waitForTauri` 轮询路径
   - 覆盖：`waitForTauri` 超时（5000ms）时打印错误日志
   - 覆盖：invoke 为 null 时版本号降级显示 `package.json` 版本而非硬编码 `0.2.0`
   - _需求：5.1、5.2、5.3_

- [ ] 2. 重构 `index.html`：将前置内联脚本移到 `main.js` 之前
   - 删除现有 `bootstrapApp` 函数及其所有调试日志代码
   - 在 `<script src="main.js">` 之前添加前置内联脚本，只做一件事：`window.tauriInvokeReady = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke || null`
   - 删除 `<script src="startup-diagnostics.js">` 引用（如存在）
   - _需求：1.1、1.2、1.3、1.4、1.5_

- [ ] 3. 重构 `main.js` 末尾启动入口
   - 删除所有 `window.kvminiBootstrap`、`window.__kvminiBootstrap`、`window.kvminiStart`、`window.kvminiBootstrapDone`、`window.__kvminiBootstrapDone` 等历史遗留暴露
   - 在文件末尾添加启动逻辑：检查 `window.tauriInvokeReady`，有值则直接调用 `initApp()`，无值则走 `waitForTauri(5000).then(initApp)` 路径
   - 超时时调用 `showError('Tauri 环境初始化失败，请重启应用')` 并写入日志
   - _需求：2.1、2.2、2.3、2.4、2.5_

- [ ] 4. 清理所有调试代码
   - 删除 `main.js` 中所有 `[DEBUG]`、`[BOOTSTRAP]`、`[STARTUP]` 等临时调试日志
   - 删除 `src/startup-diagnostics.js` 文件（如存在）
   - 保留必要的生产级别日志（`[KVMini]` 前缀的关键节点日志）
   - _需求：3.1、3.2、3.3、3.4、3.5_

- [ ] 5. 更新 bug-list.md，记录本次根因和修复方式
   - 在 `docs/bug-list.md` 中更新 Bug #007/008/009 的记录，补充最终根因（WKWebView 脚本隔离）
   - 新增 Bug #010 条目，记录本次发现的 WKWebView 外部脚本隔离问题
   - _需求：4.4_

- [ ] 6. 运行全量测试，验证无回归
   - 执行 `npm test`，确保所有现有测试通过
   - 确认新增测试（`tauri-startup-arch.test.js`）全部通过
   - _需求：5.4_

- [ ] 7. 构建并验证 build 版本正常启动
   - 执行 `npm run version:update` 更新版本号
   - 执行 `npm run build` 构建新版本
   - 启动 build 产物，验证：界面显示正确版本号、macOS 上出现权限引导遮罩、`startup.log` 记录启动成功
   - _需求：4.1、4.2、4.3、4.4_
