# 实施计划：辅助功能权限引导彻底修复

- [ ] 1. 更新 bug-list.md，记录 Bug #008
   - 记录现象：权限引导不出现、按钮点击无响应，已反复出现 4+ 次
   - 记录根因 A/B/C/D（见需求文档引言）
   - 记录修改次数和出现次数
   - _需求：全部（前置条件）_

- [ ] 2. TDD 先行：编写权限引导时序测试用例
   - 在 `src/tests/` 中新增 `accessibility-fix.test.ts`
   - 测试用例覆盖：`waitForTauri` 完成后 `checkAccessibilityPermission` 被调用（需求 1.1、1.2）
   - 测试用例覆盖：`invoke` 为 null 时打印警告日志，不静默跳过（需求 1.3）
   - 测试用例覆盖：底部第二个 `DOMContentLoaded` 不再包含 `checkAccessibilityPermission`（需求 1.4、1.5）
   - 测试用例覆盖：Rust 返回 `"Denied"` 时遮罩显示，返回 `"Granted"` 时遮罩隐藏（需求 5.2、5.3）
   - _需求：1.1、1.2、1.3、1.4、5.1-5.5_

- [ ] 3. 修复 `main.js` 底部重复的 `DOMContentLoaded` 监听器（根因 A）
   - 删除底部第二个 `DOMContentLoaded` 监听器中的 `setTimeout(checkAccessibilityPermission, 800)` 调用
   - 将 `setTimeout(initDisplayLayout, 500)` 移入 `waitForTauri` 完成后的 `initApp()` 流程中
   - 确保权限检测统一在 `initApp()` 末尾调用，只有一个入口
   - _需求：1.4、1.5_

- [ ] 4. 修复 `waitForTauri` 超时后 `invoke` 为 null 时的处理（根因 B）
   - 在 `waitForTauri` 超时后，`initApp()` 中增加 `invoke` 为 null 的明确警告日志
   - 确保即使超时，权限检测仍被调用（传入 null invoke），并打印警告而非静默跳过
   - _需求：1.2、1.3、4.4_

- [ ] 5. 统一 Rust 权限返回值为英文枚举（根因 C）
   - 修改 `src-tauri/src/platform/macos/permission.rs`：返回值从中文改为 `"Granted"` / `"Denied"` / `"NA"`
   - 同步修改 `src/accessibility.js`：检测条件改为 `status !== 'Granted'`（非 Granted 即显示遮罩）
   - 删除 JS 中的 `'Denied'` 英文和 `'未授权'` 中文混用逻辑
   - _需求：2.1、2.2、2.3、2.4_

- [ ] 6. 完善诊断日志
   - `waitForTauri()` 完成时打印：`__TAURI__` 就绪状态、`invoke` 获取结果（需求 4.1）
   - `checkAccessibilityPermission` 调用时打印：调用时机、`invoke` 状态、Rust 返回值（需求 4.2）
   - 权限遮罩显示/隐藏时打印对应日志（需求 4.3）
   - `invoke` 为 null 时任何函数调用打印包含函数名的警告（需求 4.4）
   - _需求：4.1、4.2、4.3、4.4_

- [ ] 7. 修复按钮点击无响应问题（根因 D）
   - 在所有按钮事件处理函数中，`invoke` 为 null 时显示明确错误提示（需求 3.1）
   - `invoke` 调用后端命令失败时，在界面上显示具体错误信息，不静默失败（需求 3.2）
   - _需求：3.1、3.2、3.3_

- [ ] 8. 运行全量测试，验证所有测试通过
   - 运行 `npm test`，确保新增测试全部通过
   - 确保无回归（原有测试全部通过）
   - _需求：5.1-5.5_

- [ ] 9. 版本更新、构建并验证
   - 运行 `npm run version:update` 更新版本号
   - 运行 `npm run build` 构建新版本 DMG
   - 启动程序验证：权限引导遮罩正常显示、按钮点击有响应
   - _需求：全部_
