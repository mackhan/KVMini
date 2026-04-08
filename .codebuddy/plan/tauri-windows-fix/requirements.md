# 需求文档：Tauri Windows 兼容性修复

## 引言

KVM Share 是一款跨平台鼠标键盘共享软件，基于 Tauri 框架构建。当前 Windows 版本在启动时报错：

> 无法定位程序输入点 `TaskDialogIndirect` 于动态链接库

根本原因是 Tauri 在 Windows 上依赖 **Microsoft Edge WebView2 运行时**，而 `TaskDialogIndirect` 是 WebView2 安装引导程序内部调用的 Windows API。当 WebView2 未正确安装或版本过低时，Tauri 应用无法启动。

本次修复目标：
1. 配置 Tauri 的 WebView2 安装策略，确保 Windows 用户能正常启动应用
2. 保持 macOS 版本不受任何影响
3. 不更换 GUI 框架，保留现有 Tauri 技术栈

---

## 需求

### 需求 1：WebView2 运行时自动引导安装

**用户故事：** 作为 Windows 用户，我希望在缺少 WebView2 运行时时应用能自动引导我安装，以便不需要手动处理依赖问题就能正常使用软件。

#### 验收标准

1. WHEN Windows 用户首次启动应用且系统未安装 WebView2 THEN 系统 SHALL 自动下载并运行 WebView2 Bootstrapper 安装程序
2. WHEN WebView2 安装完成 THEN 系统 SHALL 正常启动应用窗口，不再报 `TaskDialogIndirect` 错误
3. IF WebView2 已安装 THEN 系统 SHALL 跳过安装步骤直接启动

---

### 需求 2：tauri.conf.json 配置 WebView2 安装策略

**用户故事：** 作为开发者，我希望在 `tauri.conf.json` 中正确配置 WebView2 安装模式，以便打包后的 Windows 应用能自动处理运行时依赖。

#### 验收标准

1. WHEN 构建 Windows 安装包 THEN `tauri.conf.json` SHALL 包含 `webviewInstallMode` 配置项，类型为 `downloadBootstrapper`
2. WHEN 配置修改完成 THEN 系统 SHALL 能够正常执行 `cargo tauri build` 而不报配置错误
3. IF 目标平台为 macOS THEN 系统 SHALL 忽略 `webviewInstallMode` 配置，不影响 macOS 构建流程

---

### 需求 3：Cargo.toml 依赖版本稳定

**用户故事：** 作为开发者，我希望 Tauri 及相关依赖版本保持稳定且兼容，以便 Windows 和 macOS 都能正常编译运行。

#### 验收标准

1. WHEN 执行 `cargo build` THEN 系统 SHALL 使用 `tauri = "1.0.0"` 和 `tauri-build = "1.0"` 版本成功编译，无版本冲突错误
2. WHEN 在 Windows 上运行编译产物 THEN 系统 SHALL 正常启动，不出现 `STATUS_ENTRYPOINT_NOT_FOUND` 错误
3. IF 需要添加 Windows 特定依赖 THEN 系统 SHALL 使用 `[target.'cfg(windows)'.dependencies]` 条件编译块隔离，不影响 macOS 编译

---

### 需求 4：跨平台构建隔离

**用户故事：** 作为开发者，我希望 Windows 修复代码与 macOS 代码完全隔离，以便上传代码后不会破坏 macOS 版本的构建。

#### 验收标准

1. WHEN 在 macOS 上执行 `cargo build` THEN 系统 SHALL 成功编译，不受 Windows 特定配置影响
2. WHEN Windows 特定 API 调用存在于代码中 THEN 系统 SHALL 使用 `#[cfg(target_os = "windows")]` 或 `[target.'cfg(windows)'.dependencies]` 进行隔离
3. IF `tauri.conf.json` 中存在 Windows 专属配置 THEN 该配置 SHALL 仅在 Windows 构建时生效，macOS 构建时被忽略

---

### 需求 5：平台隔离完整性保障

**用户故事：** 作为开发者，我希望所有平台差异代码都通过明确的隔离机制管理，以便任何一端的修改都不会意外影响另一端的功能。

#### 验收标准

1. WHEN 向代码库提交任何 Windows 相关修复 THEN 所有 Windows 专属依赖 SHALL 放置在 `[target.'cfg(windows)'.dependencies]` 块中，禁止放在通用 `[dependencies]` 块
2. WHEN 源码中存在平台差异逻辑 THEN 系统 SHALL 使用 `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "macos")]` 属性明确标注，不得使用运行时判断替代编译期隔离
3. WHEN `tauri.conf.json` 需要添加 Windows 专属字段（如 `webviewInstallMode`、`certificateThumbprint` 等）THEN 这些字段 SHALL 仅出现在 `bundle.windows` 子节点下，不影响 `bundle.macOS` 节点
4. IF 新增任何第三方 crate THEN 开发者 SHALL 确认该 crate 支持目标平台，或通过条件编译限制其仅在对应平台编译
5. WHEN 在 Windows 上完成修复并提交代码后 THEN macOS 的 CI 构建 SHALL 仍然通过，不引入任何新的编译错误或警告
