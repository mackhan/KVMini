# 实施计划：Tauri Windows 兼容性修复

- [ ] 1. 恢复 Cargo.toml 依赖版本到稳定状态
   - 将 `tauri` 版本恢复为 `"1.5"` 或 `"1.8"`（选择与原始项目一致的稳定版本）
   - 将 `tauri-build` 版本恢复为对应的 `"1.5"` 版本
   - 确保 Windows 特定依赖使用 `[target.'cfg(windows)'.dependencies]` 块隔离，禁止放在通用 `[dependencies]` 块
   - _需求：3.1、3.3、5.1_

- [ ] 2. 修改 tauri.conf.json 添加 WebView2 安装策略
   - 在 `tauri.bundle.windows` 节点下添加 `webviewInstallMode` 配置
   - 配置类型设为 `{ "type": "downloadBootstrapper" }`，使应用在缺少 WebView2 时自动下载安装引导程序
   - 确认 Windows 专属字段仅出现在 `bundle.windows` 子节点下，不影响 `bundle.macOS` 节点
   - _需求：2.1、2.2、5.3_

- [ ] 3. 审查源码中的平台差异逻辑
   - 检查所有平台差异代码，确认使用 `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "macos")]` 编译期属性标注
   - 排查是否存在运行时平台判断替代编译期隔离的情况，若有则改为条件编译
   - 确认新增或现有第三方 crate 均支持目标平台，或已通过条件编译限制
   - _需求：4.2、5.2、5.4_

- [ ] 4. 验证 Windows 构建编译通过
   - 执行 `cargo build` 确认编译无错误、无版本冲突警告
   - 运行编译产物，确认不再出现 `TaskDialogIndirect` / `STATUS_ENTRYPOINT_NOT_FOUND` 错误
   - 若系统未安装 WebView2，确认应用能自动引导安装；若已安装则直接正常启动
   - _需求：1.1、1.2、1.3、3.1、3.2_

- [ ] 5. 验证跨平台隔离（macOS 构建不受影响）
   - 在 macOS 上执行 `cargo build`，确认编译成功，无新增编译错误或警告
   - 确认 `webviewInstallMode` 等 Windows 专属配置不影响 macOS 构建流程
   - 确认 macOS 应用功能与修复前保持一致
   - _需求：2.3、4.1、4.3、5.5_
