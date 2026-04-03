# 需求文档：UI修复、跨平台功能一致性与进程清理

## 引言

KVMini（KVM Share）是一款跨平台鼠标键盘共享软件，基于 Tauri + Rust 构建，支持 macOS 和 Windows 两个平台。

当前存在以下问题需要解决：

1. **`invoke` 解构错误**：`main.js` 第一行 `const { invoke } = window.__TAURI__.tauri` 在 Tauri 未初始化时执行，导致 `TypeError`。
2. **跨平台功能不一致**：Windows 版与 macOS 版功能存在差异，界面和功能应保持完全一致，同时做到平台安全隔离互不影响。
3. **kvnshare 遗留进程**：早期版本进程在后台运行，需要清理。
4. **跨平台功能同步检测**：每次拉取代码时，自动检测某个平台是否缺少另一个平台已有的功能，并提醒补全。

---

## 需求

### 需求 1：修复 Tauri invoke 解构错误

**用户故事：** 作为一名用户，我希望应用启动时不出现 JavaScript 错误，以便应用能够正常加载和使用。

#### 验收标准

1. WHEN 应用启动加载前端页面 THEN 系统 SHALL 确保 `window.__TAURI__` 已完全初始化后再调用 `invoke`
2. WHEN `window.__TAURI__` 或 `window.__TAURI__.tauri` 为 `null/undefined` THEN 系统 SHALL 使用安全的方式获取 `invoke`（改用 `@tauri-apps/api/tauri` 模块导入方式）
3. WHEN 页面 DOM 加载完成 THEN 系统 SHALL 在 `DOMContentLoaded` 事件后才执行 invoke 相关初始化
4. IF Tauri 版本为 1.x THEN 系统 SHALL 使用 `import { invoke } from '@tauri-apps/api/tauri'` 的正确模块导入方式
5. WHEN 修复完成后重新构建 THEN 系统 SHALL 不再出现 `TypeError: Cannot destructure property 'invoke'` 错误

---

### 需求 2：Windows 与 macOS 版本功能完全一致

**用户故事：** 作为一名用户，我希望 Windows 版和 macOS 版拥有完全相同的界面和功能，以便在不同平台上获得一致的使用体验。

#### 验收标准

1. WHEN 用户在任意平台打开应用 THEN 系统 SHALL 显示完全相同的主界面布局，包含：设备列表、连接状态、连接/断开按钮、延迟显示、设置入口
2. WHEN 用户点击设置按钮 THEN 系统 SHALL 显示完整的设置面板（不得使用 `alert` 代替），包含快捷键配置、连接偏好、启动选项等
3. WHEN 应用在任意平台运行 THEN 系统 SHALL 支持系统托盘图标，允许用户从托盘控制应用（最小化到托盘、显示/隐藏窗口、快速连接/断开）
4. WHEN 设备连接成功或断开 THEN 系统 SHALL 在两个平台上均显示系统通知提示用户
5. WHEN 用户曾连接过设备 THEN 系统 SHALL 在两个平台上均显示连接历史记录，支持快速重连
6. WHEN 设备发现功能正在运行 THEN 系统 SHALL 在界面上显示扫描状态动画，而非静默等待
7. WHEN 用户配置快捷键 THEN 系统 SHALL 在两个平台上均支持全局快捷键切换控制焦点
8. WHEN 应用在 macOS 上运行 THEN 系统 SHALL 界面样式与 Windows 版保持视觉一致性（布局、颜色、字体、交互方式）

---

### 需求 3：跨平台安全隔离，互不影响

**用户故事：** 作为一名开发者，我希望 Windows 和 macOS 的平台特定代码相互隔离，以便在修改一个平台的功能时不会影响另一个平台的稳定性。

#### 验收标准

1. WHEN 开发者修改 macOS 特定功能 THEN 系统 SHALL 确保 Windows 平台的构建和运行不受影响
2. WHEN 开发者修改 Windows 特定功能 THEN 系统 SHALL 确保 macOS 平台的构建和运行不受影响
3. WHEN 存在平台差异的功能（如系统托盘、快捷键、通知） THEN 系统 SHALL 使用条件编译（`#[cfg(target_os = "...")]`）或平台抽象层进行隔离
4. WHEN 前端调用平台特定 API THEN 系统 SHALL 通过统一的 Tauri 命令接口封装，前端代码不直接包含平台判断逻辑
5. IF 某个平台的特定功能不可用 THEN 系统 SHALL 优雅降级，不影响其他功能的正常使用
6. WHEN 运行测试 THEN 系统 SHALL 提供跨平台的单元测试，确保核心逻辑在两个平台上行为一致

---

### 需求 4：拉取代码时自动检测跨平台功能缺失

**用户故事：** 作为一名开发者，我希望每次拉取代码后能自动检测两个平台之间的功能差异，以便及时发现并补全缺失的功能。

#### 验收标准

1. WHEN 开发者执行 `git pull` 或拉取代码后 THEN 系统 SHALL 自动运行跨平台功能一致性检查脚本
2. WHEN 检查发现某个平台缺少另一个平台已有的 Tauri 命令（后端功能） THEN 系统 SHALL 在终端输出警告，列出缺失的命令名称和所在文件
3. WHEN 检查发现某个平台缺少另一个平台已有的前端功能模块 THEN 系统 SHALL 在终端输出警告，列出缺失的功能模块
4. WHEN 检查发现功能缺失 THEN 系统 SHALL 提示开发者需要补全的具体内容，并给出参考实现的文件路径
5. IF 两个平台功能完全一致 THEN 系统 SHALL 输出"跨平台功能检查通过"的确认信息
6. WHEN 开发者完成功能补全后 THEN 系统 SHALL 要求为新增功能编写对应的测试用例，测试通过后方可提交

---

### 需求 5：清理 kvnshare 遗留进程

**用户故事：** 作为一名用户，我希望系统后台不运行无用的旧版进程，以便节省系统资源并避免混淆。

#### 验收标准

1. WHEN 用户安装或运行新版 KVM Share THEN 系统 SHALL 不启动 `kvnshare` 旧版进程
2. IF `kvnshare` 进程正在后台运行 THEN 系统 SHALL 提供清理脚本，自动检测并终止该进程
3. WHEN 应用启动时 THEN 系统 SHALL 检测是否存在 `kvnshare` 遗留进程，若存在则自动终止并提示用户
4. WHEN 系统开机启动项中存在 `kvnshare` THEN 系统 SHALL 从 LaunchAgents/LaunchDaemons（macOS）或注册表启动项（Windows）中移除该启动项
5. WHEN 清理完成后 THEN 系统 SHALL 确保只有 `KVM Share`（新版）进程在运行，不存在重复或遗留进程
