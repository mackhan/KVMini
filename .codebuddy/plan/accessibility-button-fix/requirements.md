# 需求文档：权限引导 + 按钮点击彻底修复（含自动化诊断）

## 引言

该问题已反复出现 4+ 次，每次修复只解决表面症状，根本原因是**缺乏在 build 版本中可自动运行的诊断手段**，导致每次修复都是盲目的。

本次需求的核心目标是：
1. **设计并实现一套启动时自动诊断机制**，在 build 版本中将关键状态写入持久化日志文件，开发者无需用户操作即可复现和验证问题
2. **彻底修复权限引导不出现**的根因（基于诊断日志确认的真实根因）
3. **彻底修复按钮点击无响应**的根因（基于诊断日志确认的真实根因）

### 历史修复回顾（为什么一直没修好）

| 修复版本 | 修复内容 | 为什么没解决 |
|---------|---------|------------|
| Bug #006 | initApp 中添加 checkAccessibilityPermission 调用 | invoke 为 null 时静默跳过 |
| Bug #007 | 添加 waitForTauri 等待机制 | 超时后仍调用 initApp，invoke 仍为 null |
| Bug #008 | 删除第二个 DOMContentLoaded，统一英文枚举 | 未验证 build 版本中 invoke 是否真的就绪 |

**根本问题：每次修复都没有在 build 版本中验证，只靠单元测试（mock 环境），无法发现 build 版本的真实问题。**

---

## 需求

### 需求 1：启动时自动诊断日志（核心需求）

**用户故事：** 作为开发者，我希望应用在 build 版本启动时自动将关键状态写入本地日志文件，以便在不需要用户操作的情况下，通过查看日志文件来验证修复是否生效。

#### 验收标准

1. WHEN 应用启动 THEN 系统 SHALL 在 `~/Library/Logs/KVMini/startup.log` 写入以下信息：
   - 启动时间戳
   - `window.__TAURI__` 是否就绪（true/false）
   - `invoke` 是否就绪（true/false）
   - `waitForTauri` 等待耗时（ms）
   - `check_accessibility_permission` 调用结果（Granted/Denied/NA/ERROR）
   - 权限遮罩最终状态（shown/hidden）
2. WHEN `invoke` 为 null THEN 系统 SHALL 在日志中写入 `[FATAL] invoke is null after waitForTauri`
3. WHEN `check_accessibility_permission` 调用失败 THEN 系统 SHALL 在日志中写入完整错误信息
4. IF 日志目录不存在 THEN 系统 SHALL 自动创建目录
5. WHEN 日志写入失败 THEN 系统 SHALL 降级到 `console.error` 输出，不影响应用正常运行

### 需求 2：invoke 就绪状态的强保证

**用户故事：** 作为用户，我希望应用在 invoke 未就绪时不会静默失败，而是有明确的视觉反馈，以便我知道应用状态是否正常。

#### 验收标准

1. WHEN `waitForTauri` 超时（3秒内 invoke 未就绪）THEN 系统 SHALL 在界面显示"Tauri 环境初始化失败，请重启应用"的错误提示
2. WHEN `waitForTauri` 超时 THEN 系统 SHALL 不调用 `checkAccessibilityPermission`（避免静默跳过）
3. WHEN `waitForTauri` 成功 THEN 系统 SHALL 在日志中记录等待耗时
4. IF `invoke` 就绪 AND `check_accessibility_permission` 返回非 `"Granted"` THEN 系统 SHALL 显示权限引导遮罩
5. IF `invoke` 就绪 AND `check_accessibility_permission` 返回 `"Granted"` THEN 系统 SHALL 确保权限遮罩隐藏

### 需求 3：按钮点击无响应的根因诊断与修复

**用户故事：** 作为用户，我希望点击扫描、连接等按钮时能得到明确的反馈（成功或失败），而不是毫无响应。

#### 验收标准

1. WHEN 用户点击任意功能按钮 AND `invoke` 为 null THEN 系统 SHALL 显示错误提示 Toast（"Tauri 环境未就绪，请重启应用"）
2. WHEN 用户点击任意功能按钮 AND `invoke` 就绪 AND Rust 命令调用失败 THEN 系统 SHALL 显示具体错误信息
3. WHEN 权限遮罩显示时 THEN 系统 SHALL 确保遮罩的 `z-index` 足够高，覆盖所有按钮（防止用户在未授权时操作）
4. WHEN 权限遮罩隐藏时 THEN 系统 SHALL 确保遮罩完全不拦截任何点击事件（`display: none`）
5. WHEN 应用启动完成 THEN 系统 SHALL 在日志中记录所有按钮的事件监听器绑定状态

### 需求 4：可在 build 版本中运行的专项测试脚本

**用户故事：** 作为开发者，我希望有一个可以在 build 版本中运行的测试脚本，能够自动验证权限引导和按钮点击功能是否正常，而不需要每次都让用户手动测试。

#### 验收标准

1. WHEN 开发者运行 `npm run test:e2e:accessibility` THEN 系统 SHALL 启动 build 版本并自动验证：
   - `invoke` 是否在 3 秒内就绪
   - `check_accessibility_permission` 是否被调用
   - 权限遮罩是否根据权限状态正确显示/隐藏
   - 扫描按钮点击是否有响应
2. WHEN 测试通过 THEN 系统 SHALL 输出 `[PASS] 权限引导和按钮点击功能正常`
3. WHEN 测试失败 THEN 系统 SHALL 输出具体失败原因和诊断日志路径

### 需求 5：单元测试覆盖真实场景

**用户故事：** 作为开发者，我希望单元测试能覆盖 build 版本中的真实场景，而不只是 mock 环境。

#### 验收标准

1. WHEN 运行 `npm test` THEN 系统 SHALL 包含以下测试场景：
   - `waitForTauri` 超时后 `initApp` 不调用 `checkAccessibilityPermission`
   - `invoke` 就绪后 `checkAccessibilityPermission` 被调用且结果正确处理
   - 按钮点击 `invoke` 为 null 时 `showError` 被调用
   - 诊断日志在启动时被写入
2. IF 测试中 `invoke` 为 null THEN 测试 SHALL 验证 `showError` 被调用（而非静默跳过）
