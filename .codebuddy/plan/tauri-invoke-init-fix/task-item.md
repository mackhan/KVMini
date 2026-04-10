# 实施计划：Tauri invoke 初始化时序修复

## 背景

根因：Tauri 2.x build 版本中 `window.__TAURI__` 是异步注入的，`DOMContentLoaded` 触发时尚未就绪，导致 `invoke = null`，引发版本号显示错误（v0.2.0）和权限引导从不出现两个问题。

---

- [ ] 1. 更新 bug-list.md，记录 Bug #007（invoke 初始化时序问题）
   - 在 `docs/bug-list.md` 新增 Bug #007 条目，记录现象、根因、修改方式
   - 标注与 Bug #006 的关联（同一问题的深层根因）
   - _需求：1.1、3.1_

- [ ] 2. 先写测试：为 `waitForTauri` 和初始化时序编写测试用例（TDD）
   - 在 `__tests__/` 中新增 `tauri-init.test.js`
   - 测试用例覆盖：`waitForTauri` 立即就绪、延迟就绪（100ms）、超时降级（3000ms）三种场景
   - 测试用例覆盖：invoke 为 null 时 `checkAccessibilityPermission` 打印警告日志而非静默 return
   - 测试用例覆盖：invoke 为 null 时版本号降级显示 package.json 版本而非硬编码 `0.2.0`
   - _需求：5.1、5.2、5.3_

- [ ] 3. 实现 `waitForTauri` 函数，替换 `DOMContentLoaded` 直接初始化
   - 在 `src/main.js` 中新增 `waitForTauri(timeout = 3000)` 函数：轮询 `window.__TAURI__` 是否就绪，就绪后 resolve，超时后以降级模式 resolve
   - 将 `DOMContentLoaded` 回调中的 `initApp()` 改为：`waitForTauri().then(() => initApp())`
   - `waitForTauri` 超时时打印警告日志，包含当前 `window.__TAURI__` 状态
   - _需求：1.1、1.2、1.3、1.4、4.4_

- [ ] 4. 完善初始化诊断日志
   - 在 `initApp()` 开始处打印 `window.__TAURI__` 就绪状态和 `invoke` 获取结果
   - 在 `checkAccessibilityPermission` 中将静默 `return` 改为打印 `[Accessibility] invoke 未就绪，跳过权限检测` 警告
   - 在 `getAppVersion` 降级路径打印降级原因（invoke 为 null 还是命令调用失败）
   - _需求：4.1、4.2、4.3_

- [ ] 5. 修复版本号硬编码降级值
   - 修改 `src/platform.js` 的 `getAppVersion()` 降级值：从硬编码 `'0.2.0'` 改为从 HTML meta 标签或 `package.json` 读取
   - 修改 `src/main.js` 的 `initVersion()` 降级值：从硬编码 `'v0.2.0'` 改为动态读取
   - 在 `src/index.html` 的 `<head>` 中注入 `<meta name="app-version" content="{{VERSION}}">` 或通过构建脚本写入版本号
   - _需求：2.2、2.3_

- [ ] 6. 运行全量测试，验证无回归
   - 执行 `npm test`，确保所有现有 318 个测试通过
   - 确认新增测试（`tauri-init.test.js`）全部通过
   - 确认 `accessibility.js` 覆盖率不低于之前水平
   - _需求：5.4_

- [ ] 7. 更新版本号并构建验证
   - 执行 `npm run version:update` 更新版本号
   - 执行 `npm run build` 构建新版本
   - 安装 build 产物，验证：启动后显示正确版本号、macOS 上出现权限引导遮罩
   - _需求：1.1、2.1、3.1、3.2_
