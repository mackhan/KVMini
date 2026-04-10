# 需求文档：Tauri 启动架构彻底修复

## 引言

### 背景与问题演化

KVMini 在 build 版本（安装包）中长期存在启动失败问题，经过多轮调试，最终确认了真正的根因：

**macOS WKWebView（Tauri 使用的 WebView 引擎）会在 `<script src="main.js">` 执行完成后，清除该脚本中新增到 `window` 的属性，但保留之前脚本中定义的全局函数声明。**

这导致 `main.js` 中定义的所有启动函数（`kvminiBootstrap`、`initApp`、`waitForTauri` 等）在后续的 `bootstrapApp` 内联脚本中全部不可见（均为 `undefined`），应用永远无法启动。

### 问题演化链

```
Tauri 2.x 升级
  → window.__TAURI__ 变为异步注入
  → Bug #007: invoke=null, 版本号显示错误
  → 修复: 添加 waitForTauri 轮询
  → Bug #008: 第二个 DOMContentLoaded 监听器冲突
  → 修复: 删除第二个监听器
  → Bug #009: waitForTauri 超时后仍调用 initApp
  → 修复: 添加 startup-diagnostics.js + tauriInvokeReady
  → 当前问题: main.js 全局变量在 bootstrapApp 中不可见
  → 根因: Tauri WKWebView 外部脚本隔离
```

### 修复方案（方案 B：调整脚本顺序）

**核心思路：** 将 `bootstrapApp` 内联脚本移到 `<script src="main.js">` 之前执行，只负责捕获 `invoke` 并保存到 `window.tauriInvokeReady`。`main.js` 末尾直接读取 `window.tauriInvokeReady` 并调用 `initApp()`，完全不依赖跨脚本边界的全局变量。

**优点：**
- 彻底解决 WKWebView 脚本隔离问题
- 删除复杂的 `bootstrapApp` 启动逻辑
- 保持 `main.js` 独立文件，代码结构清晰
- 改动最小，符合奥卡姆剃刀原则

---

## 需求

### 需求 1：重构 index.html 脚本加载顺序

**用户故事：** 作为开发者，我希望 `index.html` 的脚本加载顺序能正确处理 WKWebView 的脚本隔离特性，以便 `main.js` 中的启动函数能被正确调用。

#### 验收标准

1. WHEN `index.html` 加载 THEN 系统 SHALL 在 `<script src="main.js">` 之前执行一个内联脚本，捕获当前可用的 `invoke` 并保存到 `window.tauriInvokeReady`
2. WHEN 内联脚本执行时 `window.__TAURI__` 已就绪 THEN 系统 SHALL 将 `invoke` 函数保存到 `window.tauriInvokeReady`
3. WHEN 内联脚本执行时 `window.__TAURI__` 尚未就绪 THEN 系统 SHALL 将 `window.tauriInvokeReady` 设为 `null`，由 `main.js` 的 `waitForTauri` 降级处理
4. WHEN 脚本加载完成后 THEN 系统 SHALL 不再需要 `bootstrapApp` 函数来触发启动，删除该函数
5. IF 存在 `startup-diagnostics.js` 等调试脚本 THEN 系统 SHALL 在修复完成后将其删除，保持代码整洁

### 需求 2：重构 main.js 启动入口

**用户故事：** 作为开发者，我希望 `main.js` 末尾的启动逻辑能直接读取 `window.tauriInvokeReady` 并调用 `initApp()`，以便不依赖任何跨脚本边界的全局变量。

#### 验收标准

1. WHEN `main.js` 执行到末尾 THEN 系统 SHALL 检查 `window.tauriInvokeReady` 是否有值
2. IF `window.tauriInvokeReady` 有值 THEN 系统 SHALL 直接调用 `initApp()`（invoke 已就绪）
3. IF `window.tauriInvokeReady` 为 null THEN 系统 SHALL 通过 `waitForTauri` 等待 invoke 就绪后再调用 `initApp()`
4. WHEN `waitForTauri` 超时（超过 5000ms）THEN 系统 SHALL 打印错误日志并显示用户友好的错误提示，而不是静默失败
5. WHEN `main.js` 末尾启动逻辑执行 THEN 系统 SHALL 删除所有 `kvminiBootstrap`、`__kvminiBootstrap`、`kvminiStart` 等历史遗留的启动函数暴露，保持代码整洁

### 需求 3：清理所有调试代码

**用户故事：** 作为开发者，我希望修复完成后能清理所有临时调试代码，以便代码库保持整洁，不影响生产环境性能。

#### 验收标准

1. WHEN 修复完成 THEN 系统 SHALL 删除 `main.js` 中所有 `[DEBUG]`、`[BOOTSTRAP]`、`[STARTUP]` 等调试日志
2. WHEN 修复完成 THEN 系统 SHALL 删除 `index.html` 中 `bootstrapApp` 函数内的所有早期日志写入代码
3. WHEN 修复完成 THEN 系统 SHALL 删除 `src/startup-diagnostics.js` 文件（如存在）
4. WHEN 修复完成 THEN 系统 SHALL 保留必要的生产级别日志（`[KVMini]` 前缀的关键节点日志）
5. IF 存在 `window.kvminiBootstrapDone`、`window.__kvminiBootstrapDone` 等历史标志位 THEN 系统 SHALL 将其删除

### 需求 4：应用在 build 版本中正常启动

**用户故事：** 作为 KVMini 用户，我希望安装包版本能正常启动并显示正确的版本号和权限引导，以便我能正常使用软件。

#### 验收标准

1. WHEN 用户在 macOS 上启动 build 版本 THEN 系统 SHALL 在 3 秒内完成初始化并显示主界面
2. WHEN 应用启动成功 THEN 系统 SHALL 在界面上显示正确的版本号（与 `package.json` 一致，不是硬编码的 `v0.2.0`）
3. WHEN macOS 用户首次启动且未授权辅助功能 THEN 系统 SHALL 显示权限引导遮罩
4. WHEN 应用启动 THEN 系统 SHALL 在日志文件（`~/Library/Logs/KVMini/startup.log`）中记录启动成功的关键节点

### 需求 5：回归测试覆盖

**用户故事：** 作为开发者，我希望有完整的测试用例覆盖新的启动架构，以便防止此类问题再次出现。

#### 验收标准

1. WHEN 测试 `waitForTauri` 函数 THEN 测试 SHALL 覆盖：立即就绪（`tauriInvokeReady` 有值）、延迟就绪（轮询）、超时降级三种场景
2. WHEN 测试启动流程 THEN 测试 SHALL 验证：`tauriInvokeReady` 有值时直接调用 `initApp()`，无值时走 `waitForTauri` 路径
3. WHEN 测试版本号显示 THEN 测试 SHALL 验证：invoke 为 null 时显示 `package.json` 版本而非硬编码 `0.2.0`
4. IF 所有测试通过 THEN 系统 SHALL 保证现有测试无回归（全量测试通过）
