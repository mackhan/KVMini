# 需求文档：设置功能完善 + 快捷键修复 + 应用图标设计 + tauri-commerce SDK 接入

## 引言

本次迭代来源于用户在实际使用中发现的 4 类问题，涵盖 **2 个 Bug 修复**、**1 个功能完善** 和 **1 个设计任务**，以及 **1 个 SDK 接入任务**：

1. **[SDK 接入] tauri-commerce SDK 首次完整接入** - 项目从未接入 tauri-commerce SDK，需要完整接入，支持 App Store（IAP）和直接下载（trial）双模式构建
2. **[Bug] 语言/主题等设置未生效** - 设置页中语言切换、主题切换等功能保存后没有实际效果，需通过 tauri-commerce SDK 的 i18n 模块实现
3. **[Bug] 设置页取消按钮无效** - 修改设置后点击"取消"没有弹出"是否放弃修改"确认提示
4. **[Bug] 快捷键设置失败弹窗** - 设置快捷键时会弹出错误窗口，快捷键功能无法正常使用
5. **[功能] 设置页全量测试用例** - 设置页所有功能缺乏完整测试覆盖
6. **[设计] 应用图标设计** - 当前图标为空白或大蓝底，需要设计专业图标

---

## 需求

### 需求 1：[SDK 接入] tauri-commerce SDK 完整接入（双模式构建）

**用户故事：** 作为开发者，我希望 KVMini 完整接入 tauri-commerce SDK，支持 App Store（IAP 内购）和直接下载（试用期管理）两种发布模式，并通过构建时 feature flags 切换，以便在开发阶段直接下载测试，上架时切换为 App Store 模式。

#### 背景（已确认现状）

**当前状态**：
- KVMini 的 `src-tauri/Cargo.toml` 中**完全没有** tauri-commerce 依赖
- `src/license.js` 中只有注释占位符，写着"待 SDK 可用后替换"
- 项目有自己的 `i18n.js` 和 `theme.js`，但这些应该被 SDK 的 i18n 模块替代

**SDK 信息**：
- 私有 Git 仓库：`https://github.com/mackhan/tauri-commerce`
- 最新版本：`v0.1.2`（2026-04-12）
- 接入方式：使用 **Git 引用**（与 OnenoteCopy 保持一致，SDK 只在 tauri-commerce 仓库中修改，KVMini 只是消费方）
- 参考项目：`~/Documents/Git/OnenoteCopy`（已有完整接入示例，引用方式相同）

**重要约束**：
- KVMini **不能修改** tauri-commerce SDK 源码，SDK 的修改只在 tauri-commerce 仓库中进行
- KVMini 通过 Git tag 引用固定版本，升级时只需更新 tag 版本号

**双模式构建方案**：

| 模式 | 适用场景 | Feature Flags | 说明 |
|------|---------|--------------|------|
| App Store 版 | MAS 上架 | `trial`, `iap`, `i18n`, `permissions`, `feedback`, `review`, `autostart` | 启用 IAP，禁用 updater |
| 直接下载版 | 开发阶段 / 官网下载 | `trial`, `i18n`, `permissions`, `feedback`, `review`, `autostart`, `changelog` | 无 IAP，可用 updater |

**Cargo.toml 配置方案**（Git 引用，与 OnenoteCopy 一致）：
```toml
# 直接下载版（默认/开发阶段）
[dependencies]
tauri-commerce = { git = "https://github.com/mackhan/tauri-commerce", tag = "v0.1.2", default-features = false, features = ["trial", "i18n", "permissions", "feedback", "review", "autostart", "changelog"] }

# App Store 版（构建时切换 features）：
# tauri-commerce = { git = "https://github.com/mackhan/tauri-commerce", tag = "v0.1.2", default-features = false, features = ["trial", "iap", "i18n", "permissions", "feedback", "review", "autostart"] }
```

**SDK 提供的 Tauri 命令**（需注册到 invoke_handler）：
- `get_trial_status` - 获取试用/购买状态
- `purchase` / `restore_purchase` - IAP 内购（仅 App Store 版）
- `get_locale` / `set_locale` - 语言切换
- `check_accessibility_permission` / `request_accessibility_permission` - 辅助功能权限
- `send_feedback` - 用户反馈
- `request_review` - 评分引导
- `get_changelog` - 更新日志
- `get_app_version` - 应用版本

**main.rs 初始化示例**（参考 OnenoteCopy）：
```rust
let commerce_config = tauri_commerce::CommerceConfig::builder()
    .app_name("KVMini")
    .app_version(env!("CARGO_PKG_VERSION"))
    .bundle_id("com.kvmini.app")
    .product_id("com.kvmini.app.pro")  // App Store 版内购产品 ID
    .feedback_email("support@kvmini.app")
    .trial_days(7)
    .review_trigger_count(10)
    .build()
    .expect("Commerce SDK 配置构建失败");
tauri_commerce::init(commerce_config).expect("SDK 初始化失败");
tauri_commerce::iap::init_iap();  // 仅 App Store 版需要
```

#### 验收标准

1. WHEN 项目构建时 THEN 系统 SHALL 成功编译 tauri-commerce 依赖，无编译错误
2. WHEN 应用启动时 THEN 系统 SHALL 成功初始化 tauri-commerce SDK，日志显示"SDK 初始化成功"
3. WHEN 前端调用 `get_trial_status` THEN 系统 SHALL 返回正确的试用状态（Active/Expired/Purchased）
4. WHEN 前端调用 `get_locale` / `set_locale` THEN 系统 SHALL 正确读取和切换语言
5. WHEN 前端调用 `check_accessibility_permission` THEN 系统 SHALL 返回当前辅助功能权限状态
6. WHEN 使用直接下载版 feature flags 构建 THEN 系统 SHALL 不包含 IAP 相关代码，构建成功
7. WHEN 使用 App Store 版 feature flags 构建 THEN 系统 SHALL 包含 IAP 模块，构建成功
8. IF SDK 初始化失败 THEN 系统 SHALL 记录错误日志，应用仍能正常启动（降级处理）
9. WHEN 在 Windows 上构建 THEN 系统 SHALL 自动禁用 iap feature，只使用 trial 模式
10. WHEN SDK 接入完成 THEN 系统 SHALL 将 `src/license.js` 中的占位符替换为真实的 SDK 调用

---

### 需求 2：[Bug] 通过 tauri-commerce SDK 实现语言/主题设置生效

**用户故事：** 作为用户，我希望在设置页修改语言、主题等选项并保存后，界面能立即反映这些变更，而不是保存后毫无变化。

#### 背景（已确认根因）

**根本原因**：项目有自己的 `i18n.js` 和 `theme.js`，但 tauri-commerce SDK 提供了更完整的 i18n 支持（Rust 后端 + 前端联动）。接入 SDK 后，语言切换应通过 SDK 的 `set_locale` 命令实现，主题切换继续使用前端 `theme.js`（SDK 不管理主题）。

**修复方向**：
1. 保存设置后，调用 `invoke('set_locale', { locale })` 切换语言
2. 调用 `window.themeManager.setTheme(theme)` 切换主题
3. 应用启动时通过 SDK 的 `get_locale` 初始化语言

#### 验收标准

1. WHEN 用户在设置页将语言切换为"English"并保存 THEN 系统 SHALL 通过 SDK `set_locale` 立即将界面切换为英文
2. WHEN 用户在设置页将语言切换为"中文"并保存 THEN 系统 SHALL 通过 SDK `set_locale` 立即将界面切换为中文
3. WHEN 用户在设置页将主题切换为"深色"并保存 THEN 系统 SHALL 立即应用深色主题
4. WHEN 用户在设置页将主题切换为"浅色"并保存 THEN 系统 SHALL 立即应用浅色主题
5. WHEN 用户在设置页将主题切换为"跟随系统"并保存 THEN 系统 SHALL 根据系统当前主题自动切换
6. WHEN 应用启动时 THEN 系统 SHALL 通过 SDK `get_locale` 读取已保存语言并初始化
7. IF SDK 调用失败 THEN 系统 SHALL 记录错误日志并保持当前状态不变

---

### 需求 3：[Bug] 设置页取消按钮需弹出确认提示

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

### 需求 4：[Bug] 实现缺失的全局快捷键注册命令

**用户故事：** 作为用户，我希望能正常设置全局快捷键，而不是每次保存设置都弹出"快捷键注册失败"的错误窗口。

#### 背景（已确认根因）

**根本原因**：前端调用 `invoke('register_global_hotkey', { hotkey })` 时，Rust 后端**根本没有这个 Tauri 命令**，导致 Tauri 返回 "command not found" 错误。

**已有基础**：Rust 后端已实现 `MacHotkeyManager` 和 `WinHotkeyManager`，但未暴露为 Tauri 命令。

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
6. IF 快捷键注册失败（如辅助功能权限不足）THEN 系统 SHALL 返回具体错误信息，前端显示友好提示
7. WHEN 快捷键注册命令在 macOS 上运行 THEN 系统 SHALL 使用 `MacHotkeyManager` 实现
8. WHEN 快捷键注册命令在 Windows 上运行 THEN 系统 SHALL 使用 `WinHotkeyManager` 实现

---

### 需求 5：[功能] 设置页全量测试用例

**用户故事：** 作为开发者，我希望设置页的所有功能都有完整的测试用例覆盖，以便在修改代码时能快速发现回归问题。

#### 需要覆盖的测试范围

- 语言切换（中文/英文，通过 SDK set_locale）
- 主题切换（浅色/深色/跟随系统）
- 快捷键设置（捕获、保存、注销、冲突检测）
- 网络设置（端口、监听 IP、网卡选择）
- TLS 设置（开关、证书生成、指纹显示）
- 保存设置（成功/失败场景）
- 取消设置（有修改/无修改场景）
- 设置页打开/关闭
- 设置项默认值验证
- tauri-commerce SDK 命令 mock 测试

#### 验收标准

1. WHEN 运行测试套件 THEN 系统 SHALL 通过 `settings-page.test.ts` 中所有测试用例
2. WHEN 语言切换测试运行 THEN 系统 SHALL 验证 `invoke('set_locale')` 被正确调用
3. WHEN 主题切换测试运行 THEN 系统 SHALL 验证切换后 `body` 的 CSS 类名正确变更
4. WHEN 快捷键测试运行 THEN 系统 SHALL 验证快捷键捕获、格式化、保存、注销流程
5. WHEN 取消确认测试运行 THEN 系统 SHALL 验证有修改时弹出确认对话框，无修改时直接关闭
6. WHEN 保存设置测试运行 THEN 系统 SHALL 验证 `invoke('save_settings', { config: ... })` 被正确调用
7. WHEN 所有测试通过 THEN 测试覆盖率 SHALL 达到设置相关代码的 80% 以上

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
