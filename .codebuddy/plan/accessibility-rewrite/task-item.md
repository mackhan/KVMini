# 实施计划：辅助功能权限引导彻底重写

- [ ] 1. 编写测试用例（TDD 先行）
   - 为前端事件监听逻辑编写测试：收到 `permission-denied` 事件时遮罩显示、收到 `permission-granted` 事件时遮罩隐藏
   - 为遮罩 DOM 不存在的边界情况编写测试（不抛出异常）
   - 为"前往授权"按钮点击调用 `open_accessibility_settings` 编写测试
   - _需求：3.1、3.2、3.4、4.1、6.1_

- [ ] 2. 重写 Rust 侧权限检测与事件推送
   - 在 `setup` 钩子中（macOS 平台）调用 `check_accessibility_permission`
   - 权限为 `Denied` 时发送 `accessibility://permission-denied` 事件，`Granted` 时发送 `accessibility://permission-granted` 事件
   - Windows 平台不发送任何权限事件
   - 检测失败时记录错误日志，不影响应用启动
   - _需求：1.1、1.2、1.3、1.4、1.5_

- [ ] 3. 实现 Rust 侧后台权限轮询
   - 权限为 `Denied` 时启动后台线程，每 2 秒轮询一次
   - 检测到权限变为 `Granted` 时发送事件并停止轮询
   - 应用窗口关闭时停止轮询，释放资源
   - 轮询期间检测失败时忽略错误，继续下一轮
   - _需求：2.1、2.2、2.3、2.4、2.5_

- [ ] 4. 重写前端 `accessibility.js`（事件驱动版）
   - 删除 `checkAccessibilityPermission`、`startAccessibilityPolling`、`stopAccessibilityPolling` 旧函数
   - 在 `DOMContentLoaded` 时注册 `accessibility://permission-denied` 和 `accessibility://permission-granted` 两个事件监听器
   - 收到事件后操作遮罩的 `hidden` class，遮罩 DOM 不存在时只记录警告不抛异常
   - 确保遮罩 `z-index` 足够高，覆盖整个界面
   - _需求：3.1、3.2、3.3、3.4、3.5、5.1_

- [ ] 5. 清理 `main.js` 中的旧调用
   - 删除 `initApp()` 中的 `checkAccessibilityPermission()` 调用
   - 在 `initApp()` 中绑定"前往授权"按钮的点击事件（调用 `invoke('open_accessibility_settings')`）
   - 调用失败时记录错误，不关闭遮罩
   - _需求：4.1、4.2、4.3、4.4、5.2、5.3_

- [ ] 6. 运行全量测试并验证
   - 运行 `npm test`，确保所有测试通过，无回归
   - 确认新增测试用例全部通过
   - _需求：6.1、6.2_

- [ ] 7. 构建并手动验证
   - 运行 `npm run version:update` 更新版本号，删除旧构建产物
   - 运行 `npm run build` 构建新版本
   - 启动应用，验证：首次启动时遮罩正确显示、在系统设置授权后遮罩自动消失、"前往授权"按钮能打开系统设置
   - _需求：1.1～1.5、2.1～2.5、3.1～3.5、4.1～4.4_
