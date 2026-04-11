# 实施计划：扫描设备 UX 修复

- [ ] 1. 补充 i18n 文案
   - 在中英文 i18n 配置文件中新增 `scanFailed`、`scanFoundDevices`、`noDevicesFound` 三个 key
   - 中文：`scanFailed` = "扫描失败，请重试"，`scanFoundDevices` = "发现 {n} 台设备"，`noDevicesFound` = "未发现局域网设备，可尝试手动连接"
   - 英文对应翻译同步添加
   - _需求：4.1、4.2、4.3、4.4_

- [ ] 2. 为 `handleScan()` 的三种结果分支编写测试用例（TDD）
   - 测试用例 A：`invoke` 抛出异常时，Toast 显示 `scanFailed` 文案，不显示 `noDevicesAfterScan`
   - 测试用例 B：`invoke` 返回空数组时，显示 `noDevicesFound` Toast，不显示 error 级别提示
   - 测试用例 C：`invoke` 返回非空数组时，显示 `scanFoundDevices` 成功 Toast
   - 测试用例 D：扫描结束后（无论结果），按钮恢复可用状态，扫描动画隐藏
   - _需求：1.1、1.2、2.1、2.2、2.4_

- [ ] 3. 修改 `handleScan()` 实现，通过上述测试
   - `catch` 分支：改用 `scanFailed` 文案，并 `console.error` 输出具体错误
   - 空结果分支：调用 `updateDevicesList([])` 后，额外显示 `noDevicesFound` Toast（info 级别）
   - 有结果分支：显示 `scanFoundDevices` 成功 Toast，文案中插入设备数量 `n`
   - `finally` 块：确保按钮恢复、动画隐藏（无论成功/失败/空结果）
   - _需求：1.1、1.2、1.3、2.1、2.2、2.3、2.4、3.1、3.2_

- [ ] 4. 验证冷启动问题（第一次点击不立即报错）
   - 确认 `discover_devices` 命令已在 `main.rs` 的 `invoke_handler` 中正确注册
   - 在开发模式下手动测试：首次点击扫描应进入等待状态（5秒），而非立即弹出错误
   - _需求：3.1、3.3_

- [ ] 5. 回归测试并更新 Bug 记录
   - 运行全量测试 `npm test`，确保所有测试通过
   - 在 `docs/bug-list.md` 中记录本次两个 bug 的现象、根因、修改方式
   - 版本号 +1（`npm run version:update`）
   - _需求：1.1、1.2、2.1、2.2、3.1_
