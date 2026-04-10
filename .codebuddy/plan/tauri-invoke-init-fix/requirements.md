# 需求文档：Tauri invoke 初始化时序修复

## 引言

KVMini 在 build 版本（安装包）中存在两个严重问题，根因相同：**`window.__TAURI__` 在 `DOMContentLoaded` 触发时尚未挂载，导致 `invoke = null`**。

### 已确认的根因

在 Tauri 2.x 的 build 版本中，`withGlobalTauri: true` 的 `window.__TAURI__` 是通过注入脚本异步挂载的。`DOMContentLoaded` 触发时，`window.__TAURI__` 可能还未就绪，导致 `getTauriInvoke()` 返回 `null`。

这一个根因导致了两个表现：

1. **版本号显示 v0.2.0**：`getAppVersion()` 的 `safeInvoke` 因 `_invoke = null` 走降级路径，返回硬编码的 `'0.2.0'`；`initVersion()` 的降级值也是 `'v0.2.0'`
2. **权限引导从不出现**：`checkAccessibilityPermission(invoke)` 第一行 `if (!overlay || !invoke) return`，`invoke` 为 null 直接返回

### 次要问题

- `getAppVersion()` 和 `initVersion()` 的降级值硬编码为 `'0.2.0'`，应该从 `package.json` 动态读取或使用更合理的降级策略
- 权限检测失败时静默 `return`，没有任何日志，难以调试

---

## 需求

### 需求 1：等待 Tauri 就绪后再初始化

**用户故事：** 作为 KVMini 用户，我希望软件在 build 版本中能正确初始化 Tauri invoke，以便所有功能（权限引导、版本号、扫描等）都能正常工作。

#### 验收标准

1. WHEN 应用在 build 版本中启动 THEN 系统 SHALL 等待 `window.__TAURI__` 就绪后再调用 `initApp()`，而不是在 `DOMContentLoaded` 时立即调用
2. WHEN `window.__TAURI__` 在 500ms 内就绪 THEN 系统 SHALL 正常初始化所有功能
3. WHEN `window.__TAURI__` 超过 3000ms 仍未就绪 THEN 系统 SHALL 以降级模式启动（非 Tauri 环境），并在控制台打印警告
4. IF 应用在 dev 模式下运行 THEN 系统 SHALL 行为与 build 版本一致，不引入新的兼容性问题

### 需求 2：版本号正确显示

**用户故事：** 作为 KVMini 用户，我希望界面上显示的版本号与实际构建版本一致，以便我能确认自己使用的是最新版本。

#### 验收标准

1. WHEN Tauri invoke 就绪 THEN 系统 SHALL 通过 `get_app_version` 命令获取真实版本号并显示
2. WHEN `get_app_version` 调用失败 THEN 系统 SHALL 显示 `package.json` 中的版本号作为降级值，而不是硬编码的 `'0.2.0'`
3. WHEN 版本号获取成功 THEN 系统 SHALL 在主界面和设置界面都正确显示 `v{version}` 格式

### 需求 3：权限引导在 build 版本中正常触发

**用户故事：** 作为 KVMini 用户，我希望在 macOS 上首次启动时能看到辅助功能权限引导，以便我能完成授权并正常使用键鼠共享功能。

#### 验收标准

1. WHEN macOS 用户首次启动 build 版本 THEN 系统 SHALL 在 invoke 就绪后检测辅助功能权限
2. WHEN 辅助功能权限未授权 THEN 系统 SHALL 显示全屏引导遮罩，引导用户前往系统设置授权
3. WHEN 权限检测失败（invoke 为 null 或后端报错）THEN 系统 SHALL 打印详细错误日志，而不是静默 return
4. WHEN 用户完成授权 THEN 系统 SHALL 通过轮询检测到权限变化并自动隐藏遮罩

### 需求 4：完善诊断日志

**用户故事：** 作为开发者，我希望在 build 版本中能通过控制台日志快速定位初始化问题，以便减少调试时间。

#### 验收标准

1. WHEN `initApp` 开始执行 THEN 系统 SHALL 打印 `window.__TAURI__` 就绪状态和 `invoke` 获取结果
2. WHEN `checkAccessibilityPermission` 因 invoke 为 null 跳过 THEN 系统 SHALL 打印 `[Accessibility] invoke 未就绪，跳过权限检测` 警告
3. WHEN `getAppVersion` 走降级路径 THEN 系统 SHALL 打印降级原因（invoke 为 null 还是命令失败）
4. WHEN Tauri 等待超时 THEN 系统 SHALL 打印超时警告和当前 `window.__TAURI__` 状态

### 需求 5：回归测试覆盖

**用户故事：** 作为开发者，我希望有完整的测试用例覆盖 invoke 初始化时序问题，以便防止此类问题再次出现。

#### 验收标准

1. WHEN 测试 `waitForTauri` 函数 THEN 测试 SHALL 覆盖：立即就绪、延迟就绪、超时降级三种场景
2. WHEN 测试版本号显示 THEN 测试 SHALL 验证：invoke 就绪时显示真实版本、invoke 为 null 时显示 package.json 版本（不是硬编码 0.2.0）
3. WHEN 测试权限检测 THEN 测试 SHALL 验证：invoke 为 null 时打印警告日志而不是静默 return
4. IF 所有测试通过 THEN 系统 SHALL 保证 318 个现有测试无回归
