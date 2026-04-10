# 需求文档：辅助功能权限引导彻底重写

## 引言

辅助功能权限引导功能已反复出现 5+ 次故障，每次修复都是在修补脆弱的 JS 调用链，而非解决根本问题。

**根本问题**：当前实现将权限引导做成 HTML overlay，依赖 WebView 内的 JS → Tauri invoke 调用链。这条链路在 macOS WKWebView 环境下极不稳定（脚本隔离、invoke 注入时序等问题），任何一环断裂都会导致遮罩不出现或按钮无响应。

**本次目标**：彻底重写辅助功能权限引导模块，采用**最简单、最直接、最不依赖时序**的实现方式：

- **Rust 侧主动检测**：应用启动时由 Rust 检测权限，权限不足时通过 Tauri 事件通知前端
- **前端被动响应**：前端只监听事件，收到通知后显示遮罩，不主动发起 invoke 调用
- **遮罩默认隐藏**：遮罩初始状态为隐藏，只有收到 Rust 的"权限不足"事件才显示
- **轮询由 Rust 负责**：权限轮询在 Rust 侧进行，授权后 Rust 发送"权限已授权"事件，前端隐藏遮罩

这样彻底消除了前端 invoke 调用链的时序依赖。

---

## 需求

### 需求 1：Rust 侧启动时主动检测权限并通知前端

**用户故事：** 作为用户，我希望应用启动后立即检测辅助功能权限状态，权限不足时自动弹出引导遮罩，无需任何手动操作。

#### 验收标准

1. WHEN 应用启动 AND 平台为 macOS THEN Rust SHALL 在 `setup` 钩子中调用 `check_accessibility_permission`
2. WHEN 权限状态为 `Denied` THEN Rust SHALL 向前端发送 `accessibility://permission-denied` 事件
3. WHEN 权限状态为 `Granted` THEN Rust SHALL 向前端发送 `accessibility://permission-granted` 事件
4. WHEN 平台为 Windows THEN Rust SHALL 不发送任何权限事件（Windows 不需要此权限）
5. WHEN 权限检测调用失败（异常）THEN Rust SHALL 记录错误日志，不发送事件，不影响应用启动

### 需求 2：Rust 侧权限轮询（授权后自动通知）

**用户故事：** 作为用户，我希望在系统设置中授权后，应用能自动检测到并关闭引导遮罩，无需手动重启应用。

#### 验收标准

1. WHEN Rust 检测到权限为 `Denied` THEN Rust SHALL 启动后台轮询（每 2 秒检测一次）
2. WHEN 轮询检测到权限变为 `Granted` THEN Rust SHALL 向前端发送 `accessibility://permission-granted` 事件
3. WHEN 轮询检测到权限变为 `Granted` THEN Rust SHALL 停止轮询
4. WHEN 应用窗口关闭 THEN Rust SHALL 停止轮询，释放资源
5. IF 轮询期间检测调用失败 THEN Rust SHALL 忽略该次错误，继续下一轮轮询

### 需求 3：前端被动响应权限事件

**用户故事：** 作为用户，我希望权限引导遮罩能准确反映当前权限状态，不受 JS 启动时序影响。

#### 验收标准

1. WHEN 前端收到 `accessibility://permission-denied` 事件 THEN 前端 SHALL 移除遮罩的 `hidden` class，显示引导遮罩
2. WHEN 前端收到 `accessibility://permission-granted` 事件 THEN 前端 SHALL 添加遮罩的 `hidden` class，隐藏遮罩
3. WHEN 页面加载完成 THEN 前端 SHALL 注册上述两个事件监听器（不依赖 invoke 是否就绪）
4. IF 遮罩 DOM 元素不存在 THEN 前端 SHALL 记录警告日志，不抛出异常
5. WHEN 遮罩显示时 THEN 遮罩 SHALL 覆盖整个界面（`z-index` 足够高），阻止用户操作底层 UI

### 需求 4：前往授权按钮功能

**用户故事：** 作为用户，我希望点击"前往授权"按钮后能直接打开系统设置的辅助功能页面，操作简单直接。

#### 验收标准

1. WHEN 用户点击"前往授权"按钮 THEN 系统 SHALL 调用 `invoke('open_accessibility_settings')` 打开系统设置
2. WHEN `invoke` 未就绪 THEN 系统 SHALL 等待 `invoke` 就绪后再绑定按钮事件（或在 `initApp` 中绑定）
3. WHEN 打开系统设置调用失败 THEN 系统 SHALL 在控制台记录错误，不影响遮罩显示
4. WHEN 用户点击按钮后 THEN 系统 SHALL 不关闭遮罩（等待 Rust 轮询检测到授权后自动关闭）

### 需求 5：清理旧实现，删除冗余代码

**用户故事：** 作为开发者，我希望代码库中没有遗留的旧实现代码，避免新旧逻辑冲突。

#### 验收标准

1. WHEN 重写完成 THEN `accessibility.js` 中的 `checkAccessibilityPermission`、`startAccessibilityPolling`、`stopAccessibilityPolling` 函数 SHALL 被删除或替换为事件监听版本
2. WHEN 重写完成 THEN `main.js` 中的 `checkAccessibilityPermission()` 调用 SHALL 被删除（改为 Rust 侧主动推送）
3. WHEN 重写完成 THEN `initApp()` 中不再有任何 `invoke('check_accessibility_permission')` 的直接调用
4. WHEN 重写完成 THEN 所有测试用例 SHALL 更新为测试事件驱动的新架构

### 需求 6：测试覆盖

**用户故事：** 作为开发者，我希望新实现有完整的测试覆盖，确保不会再次出现回归。

#### 验收标准

1. WHEN 运行 `npm test` THEN 系统 SHALL 包含以下测试场景：
   - 收到 `permission-denied` 事件时遮罩显示
   - 收到 `permission-granted` 事件时遮罩隐藏
   - 遮罩 DOM 不存在时不抛出异常
   - 前往授权按钮点击调用 `open_accessibility_settings`
2. WHEN 所有测试通过 THEN 覆盖率 SHALL 不低于现有水平
