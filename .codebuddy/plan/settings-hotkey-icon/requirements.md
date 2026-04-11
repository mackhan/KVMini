# 需求文档：设置功能完善 + 快捷键修复 + 应用图标设计

## 引言

本次迭代来源于用户在实际使用中发现的 4 类问题，涵盖 **2 个 Bug 修复**、**1 个功能完善** 和 **1 个设计任务**：

1. **[Bug] 语言/主题等设置未生效** - 设置页中语言切换、主题切换等功能保存后没有实际效果，疑似 SDK 未正确接入或前端未响应设置变更
2. **[Bug] 设置页取消按钮无效** - 修改设置后点击"取消"没有弹出"是否放弃修改"确认提示，直接关闭或无响应，导致用户无法安全放弃修改
3. **[Bug] 快捷键设置失败弹窗** - 设置快捷键时会弹出错误窗口，快捷键功能无法正常使用
4. **[功能] 设置页全量测试用例** - 设置页所有功能（语言、主题、快捷键、网络、TLS、保存/取消等）缺乏完整测试覆盖
5. **[设计] 应用图标设计** - 当前图标为空白或大蓝底，需要设计一个符合 KVMini 产品定位的专业图标

---

## 需求

### 需求 1：[Bug] 通过 tauri-commerce SDK 实现语言/主题设置生效

**用户故事：** 作为用户，我希望在设置页修改语言、主题等选项并保存后，界面能立即反映这些变更，而不是保存后毫无变化。

#### 背景（已确认根因）

**根本原因**：项目已引入 `tauri-commerce` SDK，该 SDK 提供了语言国际化（i18n）和主题切换能力，但**前端代码未正确调用 SDK 的 API**，导致设置保存后界面没有任何变化。

**错误调用链**：
```
用户保存语言/主题设置
  → invoke('save_settings', { config: { language, theme } })
  → Rust 后端保存配置文件 ✅
  → ❌ 前端未调用 tauri-commerce SDK 的 applyLanguage() / applyTheme()
  → 界面无变化
```

**修复方向**：
1. 在前端保存设置成功后，调用 `tauri-commerce` SDK 提供的语言切换 API（如 `i18n.setLocale(lang)`）
2. 调用 SDK 提供的主题切换 API（如 `theme.apply(themeName)`）
3. 应用启动时读取已保存的语言/主题配置，通过 SDK 初始化

#### 验收标准

1. WHEN 用户在设置页将语言切换为"English"并保存 THEN 系统 SHALL 通过 tauri-commerce SDK 立即将界面所有文字切换为英文
2. WHEN 用户在设置页将语言切换为"中文"并保存 THEN 系统 SHALL 通过 tauri-commerce SDK 立即将界面所有文字切换为中文
3. WHEN 用户在设置页将主题切换为"深色"并保存 THEN 系统 SHALL 通过 tauri-commerce SDK 立即应用深色主题
4. WHEN 用户在设置页将主题切换为"浅色"并保存 THEN 系统 SHALL 通过 tauri-commerce SDK 立即应用浅色主题
5. WHEN 用户在设置页将主题切换为"跟随系统"并保存 THEN 系统 SHALL 根据系统当前主题自动切换
6. WHEN 应用启动时 THEN 系统 SHALL 读取已保存的语言和主题配置，通过 tauri-commerce SDK 初始化
7. IF tauri-commerce SDK 调用失败 THEN 系统 SHALL 记录错误日志并保持当前状态不变

---

### 需求 2：[Bug] 设置页取消按钮需弹出确认提示

**用户故事：** 作为用户，我希望在修改了设置但未保存时点击"取消"，系统能弹出确认提示，让我选择是否放弃修改，而不是直接关闭或无响应。

#### 背景

当前现象：用户修改了设置项（如修改了端口号、切换了语言），点击"取消"按钮后：
- 没有弹出"是否放弃修改"的确认对话框
- 直接关闭设置页或无响应

期望行为：
- 如果设置有未保存的修改，点击"取消"应弹出确认对话框："您有未保存的修改，是否放弃？"
- 用户选择"放弃修改"→ 恢复原始设置并关闭设置页
- 用户选择"继续编辑"→ 保持设置页打开，不做任何修改

#### 验收标准

1. WHEN 用户修改了设置项后点击"取消" THEN 系统 SHALL 弹出确认对话框，提示"您有未保存的修改，是否放弃？"
2. WHEN 用户在确认对话框中选择"放弃修改" THEN 系统 SHALL 恢复所有设置项为上次保存的值，并关闭设置页
3. WHEN 用户在确认对话框中选择"继续编辑" THEN 系统 SHALL 关闭对话框，保持设置页打开，保留当前修改
4. WHEN 用户未修改任何设置项直接点击"取消" THEN 系统 SHALL 直接关闭设置页，不弹出确认对话框
5. IF 用户通过键盘 ESC 键关闭设置页且有未保存修改 THEN 系统 SHALL 同样弹出确认对话框
6. WHEN 用户点击设置页外部区域（遮罩）关闭设置页且有未保存修改 THEN 系统 SHALL 同样弹出确认对话框

---

### 需求 3：[Bug] 实现缺失的全局快捷键注册命令

**用户故事：** 作为用户，我希望能正常设置全局快捷键，而不是每次保存设置都弹出"快捷键注册失败"的错误窗口。

#### 背景（已确认根因）

**根本原因**：前端调用 `invoke('register_global_hotkey', { hotkey })` 时，Rust 后端**根本没有这个 Tauri 命令**，导致 Tauri 返回 "command not found" 错误，前端 catch 后弹出错误窗口。

**错误调用链**：
```
前端保存设置
  → invoke('register_global_hotkey', { hotkey })
  → ❌ Rust 后端无此命令（docs/platform-feature-matrix.md 已记录为"两端均缺失"）
  → Tauri 返回错误
  → showError(`快捷键注册失败: ${e}`) → 弹出错误窗口
```

**已有基础**：Rust 后端已实现 `MacHotkeyManager`（`src-tauri/src/platform/macos/hotkey.rs`）和 `WinHotkeyManager`（`src-tauri/src/platform/windows/hotkey.rs`），但未暴露为 Tauri 命令。

**修复方向**：
1. 在 Rust 后端添加 `register_global_hotkey` Tauri 命令，调用已有的 `HotkeyManager`
2. 将该命令注册到 Tauri 的 `invoke_handler`
3. 应用启动时自动注册已保存的快捷键

#### 验收标准

1. WHEN 用户保存包含快捷键的设置 THEN 系统 SHALL 成功调用 `register_global_hotkey` 命令，不弹出错误窗口
2. WHEN `register_global_hotkey` 命令被调用 THEN 系统 SHALL 通过 `HotkeyManager` 注册全局快捷键
3. WHEN 快捷键成功注册后 THEN 系统 SHALL 在全局范围内响应该快捷键，触发控制权切换操作
4. WHEN 用户清空快捷键并保存 THEN 系统 SHALL 注销之前注册的全局快捷键
5. WHEN 应用启动时 THEN 系统 SHALL 自动读取已保存的快捷键并重新注册
6. IF 快捷键注册失败（如辅助功能权限不足）THEN 系统 SHALL 返回具体错误信息，前端显示友好提示而非通用错误弹窗
7. WHEN 快捷键注册命令在 macOS 上运行 THEN 系统 SHALL 使用 `MacHotkeyManager` 实现
8. WHEN 快捷键注册命令在 Windows 上运行 THEN 系统 SHALL 使用 `WinHotkeyManager` 实现

---

### 需求 4：[功能] 设置页全量测试用例

**用户故事：** 作为开发者，我希望设置页的所有功能都有完整的测试用例覆盖，以便在修改代码时能快速发现回归问题。

#### 需要覆盖的测试范围

- 语言切换（中文/英文）
- 主题切换（浅色/深色/跟随系统）
- 快捷键设置（捕获、保存、注销、冲突检测）
- 网络设置（端口、监听 IP、网卡选择）
- TLS 设置（开关、证书生成、指纹显示）
- 保存设置（成功/失败场景）
- 取消设置（有修改/无修改场景）
- 设置页打开/关闭
- 设置项默认值验证

#### 验收标准

1. WHEN 运行测试套件 THEN 系统 SHALL 通过 `settings-page.test.ts` 中所有测试用例
2. WHEN 语言切换测试运行 THEN 系统 SHALL 验证切换后 DOM 文字内容正确变更
3. WHEN 主题切换测试运行 THEN 系统 SHALL 验证切换后 `body` 的 CSS 类名正确变更
4. WHEN 快捷键测试运行 THEN 系统 SHALL 验证快捷键捕获、格式化、保存、注销流程
5. WHEN 取消确认测试运行 THEN 系统 SHALL 验证有修改时弹出确认对话框，无修改时直接关闭
6. WHEN 保存设置测试运行 THEN 系统 SHALL 验证 `invoke('save_settings', { config: ... })` 被正确调用
7. WHEN 所有测试通过 THEN 测试覆盖率 SHALL 达到设置相关代码的 80% 以上

---

### 需求 5：[功能] 升级 tauri-commerce SDK 到最新版本

**用户故事：** 作为开发者，我希望项目使用最新版本的 tauri-commerce SDK，以便获得最新的功能修复和 API 改进，确保语言/主题等功能能正确接入。

#### 背景

在修复需求1（语言/主题设置生效）之前，需要先确认并升级 tauri-commerce SDK 到最新版本，避免基于旧版 API 开发后又需要重构。

#### 验收标准

1. WHEN 检查 tauri-commerce SDK 版本 THEN 系统 SHALL 确认当前使用的版本号
2. WHEN 发现有新版本可用 THEN 系统 SHALL 将 SDK 升级到最新稳定版本
3. WHEN SDK 升级完成 THEN 系统 SHALL 确认所有现有测试仍然通过
4. WHEN SDK 升级完成 THEN 系统 SHALL 更新 `package.json` 中的版本号记录
5. IF 新版本有破坏性变更（Breaking Changes）THEN 系统 SHALL 记录变更内容并相应更新调用代码

---

### 需求 6：[设计] 应用图标设计

**用户故事：** 作为产品所有者，我希望 KVMini 有一个专业、美观的应用图标，能体现"键鼠共享"的产品定位，而不是空白图片或大蓝底。

#### 产品定位

- **KVMini** = Keyboard + Video/Mouse Mini，跨设备键鼠共享工具
- 核心概念：**两台设备之间无缝共享键盘和鼠标**
- 目标风格：简洁现代，适合 macOS/Windows 应用图标规范

---

#### 🎨 图标设计方案（已确定：方案A内容 + 方案B风格）

**✅ 选定方案：双设备连接箭头 × macOS 极简深灰风**

**内容元素（来自方案A）：**
- 左右两个简化显示器/设备图标，中间有双向箭头连接
- 底部有鼠标图标，体现"键鼠共享"核心功能
- 整体构图直观表达"两台设备之间共享控制"

**视觉风格（来自方案B）：**
- 背景色：深灰 `#2C2C2E`（macOS 系统原生深灰，与 macOS Dock 融合感强）
- 图形色：纯白 `#FFFFFF`，线条简洁，无渐变
- 圆角矩形背景，符合 macOS/iOS 图标规范
- 极简线条风格，参考 Karabiner-Elements 的克制美学
- 线条粗细统一，视觉重量平衡

**最终效果描述：**
- 深灰圆角矩形背景
- 左侧：白色简化显示器轮廓（矩形框 + 底座）
- 右侧：白色简化显示器轮廓（矩形框 + 底座）
- 中间：白色双向箭头（←→），表示双向控制共享
- 底部中央：白色简化鼠标图标（圆角矩形 + 中线 + 滚轮点）
- 整体留白充足，小尺寸下清晰可辨

---

#### 技术规格

- **尺寸**：16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024
- **格式**：PNG（透明背景）+ ICO（Windows）+ ICNS（macOS）
- **输出目录**：`src-tauri/icons/`

#### 验收标准

1. WHEN 图标设计完成 THEN 系统 SHALL 在 `src-tauri/icons/` 目录下包含所有必要尺寸的图标文件
2. WHEN 应用构建完成 THEN 系统 SHALL 在 macOS Dock 和 Windows 任务栏中显示新图标
3. WHEN 图标在 16x16 尺寸下显示 THEN 图标 SHALL 仍然清晰可辨，不模糊
4. WHEN 图标在深色背景下显示 THEN 图标 SHALL 有足够的对比度，清晰可见
5. WHEN 图标在浅色背景下显示 THEN 图标 SHALL 有足够的对比度，清晰可见
6. IF 图标使用透明背景 THEN 系统 SHALL 在不同背景色下都能正常显示
