# 实施计划：设置功能完善 + 快捷键修复 + 应用图标设计 + tauri-commerce SDK 接入

- [x] 1. 参考 OnenoteCopy 接入 tauri-commerce SDK（Rust 侧）
   - 读取 `~/Documents/Git/OnenoteCopy/src-tauri/Cargo.toml`，确认 Git 引用写法和 feature flags
   - 在 `src-tauri/Cargo.toml` 中添加 tauri-commerce Git 依赖（tag = "v0.1.2"），配置直接下载版 features（默认）和 App Store 版 features（注释备用）
   - 在 `src-tauri/src/main.rs` 中参考 OnenoteCopy 初始化 SDK（`CommerceConfig::builder()`），注册所有 SDK Tauri 命令到 `invoke_handler`
   - _需求：1.1、1.2、1.3、1.6、1.7_

- [x] 2. 参考 OnenoteCopy 接入 tauri-commerce SDK（前端侧）
   - 读取 OnenoteCopy 前端代码，确认 SDK 前端调用方式
   - 将 `src/license.js` 中的占位符替换为真实的 SDK 调用（`get_trial_status`、`purchase`、`restore_purchase`）
   - 更新 `src/main.js` 启动初始化逻辑，通过 SDK `get_locale` 读取已保存语言
   - _需求：1.10、2.6_

- [x] 3. 修复语言/主题设置保存后未生效 Bug
   - 修改 `src/main.js` 的 `saveSettings()` 函数：保存成功后调用 `invoke('set_locale', { locale })` 切换语言
   - 保存成功后调用 `window.themeManager.setTheme(theme)` 切换主题
   - 删除原有 `src/i18n.js` 中与 SDK 重复的语言切换逻辑（奥卡姆剃刀）
   - _需求：2.1、2.2、2.3、2.4、2.5_

- [x] 4. 实现设置页取消按钮确认对话框
   - 在 `src/index.html` 中添加取消确认对话框 DOM（"放弃修改" / "继续编辑"两个按钮）
   - 修改 `src/main.js` 的 `closeSettings()` 函数：有 dirty 修改时显示自定义对话框，无修改时直接关闭
   - 补充 ESC 键和遮罩点击的 dirty 检测逻辑
   - _需求：3.1、3.2、3.3、3.4、3.5、3.6_

- [x] 5. 实现 Rust 后端 register_global_hotkey Tauri 命令
   - 在 `src-tauri/src/main.rs` 中添加 `register_global_hotkey` 和 `unregister_global_hotkey` Tauri 命令函数
   - 命令内部通过 `#[cfg(target_os)]` 分别调用 `MacHotkeyManager` 和 `WinHotkeyManager`
   - 将两个命令注册到 `invoke_handler`
   - _需求：4.1、4.2、4.7、4.8_

- [x] 6. 完善快捷键注销与启动自动注册逻辑
   - 在 `src-tauri/src/main.rs` 的 `setup()` 中添加启动时自动读取已保存快捷键并注册的逻辑
   - 前端 `saveSettings()` 保存快捷键时先调用 `unregister_global_hotkey` 再调用 `register_global_hotkey`
   - 快捷键为空时只调用 `unregister_global_hotkey`
   - _需求：4.3、4.4、4.5、4.6_

- [x] 7. 编写设置页全量测试用例
   - 创建 `__tests__/settings-page.test.ts`，mock `@tauri-apps/api/core` 的 `invoke`
   - 覆盖：语言切换（验证 `set_locale` 调用）、主题切换（验证 CSS 类名）、快捷键捕获/保存/注销、取消确认对话框（有修改/无修改）、保存设置（成功/失败）
   - 覆盖：TLS 设置、网络设置、设置项默认值验证
   - _需求：5.1、5.2、5.3、5.4、5.5、5.6、5.7_

- [x] 8. 设计并生成应用图标 SVG 源文件
   - 创建 `src-tauri/icons/icon.svg`：深灰 `#2C2C2E` 圆角矩形背景 + 白色双设备连接箭头 + 白色鼠标图标
   - 视觉规格：左右两个简化显示器轮廓，中间双向箭头，底部鼠标图标，线条统一粗细
   - _需求：6.1、6.3、6.4、6.5_

- [x] 9. 导出所有尺寸图标并替换项目图标文件
   - 编写图标导出脚本（使用 `sharp` 或 `rsvg-convert`），从 SVG 生成 16/32/64/128/256/512/1024px PNG
   - 生成 `icon.icns`（macOS）和 `icon.ico`（Windows），替换 `src-tauri/icons/` 下所有旧图标
   - _需求：6.1、6.2、6.6_

- [x] 10. 回归测试与构建验证
   - 运行 `npm test` 确保所有测试通过（包含新增的 settings-page 测试）
   - 运行 `npm run build` 构建直接下载版，验证图标、语言切换、主题切换、快捷键功能正常
   - 版本号 +1，删除旧构建产物，构建产物名称加上版本号
   - _需求：1.1、1.6、2.1、3.1、4.1、5.1、6.2_
