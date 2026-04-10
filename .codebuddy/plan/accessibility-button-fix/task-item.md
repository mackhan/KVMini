# 实施计划：权限引导 + 按钮点击彻底修复（含自动化诊断）

- [ ] 1. 更新 bug-list.md，记录 Bug #009
   - 记录现象：权限引导不出现、按钮点击无响应（悬浮变色但点击无效）
   - 记录根因：invoke 为 null 时静默跳过、缺乏 build 版本诊断手段
   - 记录历史修复次数（第 4 次修复）
   - _需求：引言历史修复回顾_

- [ ] 2. TDD 先行：编写诊断日志和 invoke 强保证的测试用例
   - 创建 `src/tests/startup-diagnostics.test.ts`
   - 测试场景：`waitForTauri` 超时后不调用 `checkAccessibilityPermission`（需求 2.2）
   - 测试场景：`waitForTauri` 超时后界面显示错误提示（需求 2.1）
   - 测试场景：`invoke` 就绪后 `checkAccessibilityPermission` 被调用并正确处理结果（需求 2.4/2.5）
   - 测试场景：按钮点击 `invoke` 为 null 时 `showError` 被调用（需求 3.1）
   - 测试场景：诊断日志在启动时被写入（需求 1.1）
   - _需求：2.1、2.2、2.4、3.1、5.1_

- [ ] 3. 实现启动时自动诊断日志模块
   - 新建 `src/startup-diagnostics.js`，封装日志写入逻辑
   - 使用 Tauri `fs` 插件写入 `~/Library/Logs/KVMini/startup.log`
   - 记录：时间戳、`window.__TAURI__` 就绪状态、`invoke` 就绪状态、`waitForTauri` 耗时、权限检测结果、遮罩最终状态
   - 日志目录不存在时自动创建（需求 1.4）
   - 写入失败时降级到 `console.error`，不影响主流程（需求 1.5）
   - _需求：1.1、1.2、1.3、1.4、1.5_

- [ ] 4. 修复 waitForTauri 超时后的处理逻辑（根因修复）
   - 修改 `src/main.js` 中 `waitForTauri` 的 `.then()` 回调，接收返回值
   - WHEN `waitForTauri` 返回 `false`（超时）THEN 显示错误提示并终止初始化流程（需求 2.1、2.2）
   - WHEN `waitForTauri` 返回 `true` THEN 继续调用 `initApp()`（需求 2.3）
   - 在诊断日志中记录等待耗时
   - _需求：2.1、2.2、2.3_

- [ ] 5. 修复权限检测调用链路（根因修复）
   - 在 `src/accessibility.js` 的 `checkAccessibilityPermission` 中添加前置断言：`invoke` 为 null 时写入 `[FATAL]` 日志并返回（需求 1.2）
   - 确认 Rust 返回值枚举（`"Granted"/"Denied"/"NA"`）与 JS 判断条件完全一致（需求 2.4、2.5）
   - 在诊断日志中记录权限检测结果和遮罩最终状态（需求 1.1）
   - _需求：1.2、2.4、2.5_

- [ ] 6. 修复按钮点击无响应（根因修复）
   - 检查 `src/main.js` 中所有按钮事件监听器的绑定时机，确保在 `initApp()` 完成后绑定
   - 确认每个按钮的 handler 在 `invoke` 为 null 时都调用 `showError`（需求 3.1）
   - 确认 `showError` 所依赖的 `elements.errorToast` 在 `initApp()` 中已正确初始化（需求 3.2）
   - 在诊断日志中记录按钮事件监听器绑定状态（需求 3.5）
   - _需求：3.1、3.2、3.5_

- [ ] 7. 实现可在 build 版本中运行的专项测试脚本
   - 新建 `scripts/test-accessibility-e2e.js`，使用 Node.js 脚本模拟 build 版本启动
   - 读取 `~/Library/Logs/KVMini/startup.log` 验证关键状态
   - 验证：`invoke` 是否就绪、权限检测是否被调用、遮罩状态是否正确
   - 在 `package.json` 中添加 `"test:e2e:accessibility"` 命令
   - _需求：4.1、4.2、4.3_

- [ ] 8. 运行全量测试，验证所有测试通过
   - 运行 `npm test`，确认新增测试全部通过，无回归
   - 确认 `waitForTauri` 超时场景、权限检测场景、按钮点击场景均有测试覆盖
   - _需求：5.1、5.2_

- [ ] 9. 版本更新、构建并通过诊断日志验证
   - 运行 `npm run version:update` 更新版本号
   - 运行 `npm run build` 构建新版本
   - 启动 build 版本，查看 `~/Library/Logs/KVMini/startup.log` 验证诊断日志内容
   - 运行 `npm run test:e2e:accessibility` 验证专项测试通过
   - _需求：1.1、4.1_
