# 需求文档：KVMini 从 Tauri 1.x 升级到 Tauri 2.x

## 引言

KVMini 当前使用 Tauri 1.8 + tao 0.16.x 构建，由于 tao 0.16.x 与 macOS 26 (Tahoe) 存在 ObjC API 兼容性问题，导致应用在 macOS 26 上启动即崩溃。

Tauri 2.x 使用了更新的 tao 版本（0.35.x），已修复该兼容性问题。本次迁移目标是将 KVMini 从 Tauri 1.x 升级到 Tauri 2.x，解决 macOS 26 崩溃问题，同时保持所有现有功能完整可用。

### Windows 兼容性说明

**Tauri 2.x 的 API 变更是跨平台统一的**，Windows 和 macOS 的改法完全一致，不存在只改 macOS 而破坏 Windows 的风险。具体而言：

- `winapi`、`winreg` 等 Windows 平台专属依赖**无需改动**
- `#[cfg(target_os = "windows")]` 条件编译的业务逻辑**无需改动**
- Windows 自启动逻辑**无需改动**
- Windows bundle 配置（WebView2 安装方式、WiX 中文语言包）**完整保留**
- 迁移后 Windows 版本需与 macOS 版本同步验证功能完整性

### 当前技术栈
- Tauri: 1.8
- @tauri-apps/cli: ^1.6.3
- tao: 0.16.11（崩溃根因）
- 前端：原生 HTML/JS（无框架）
- 后端：Rust，使用 tokio 异步运行时

### 迁移范围
- `src-tauri/Cargo.toml`：Tauri 依赖版本升级
- `src-tauri/tauri.conf.json`：配置文件结构变更
- `src-tauri/src/main.rs`：API 迁移（系统托盘、窗口管理、事件等）
- `src/main.js`：前端 JS invoke API 迁移
- `package.json`：CLI 工具版本升级
- `Cargo.toml`（workspace 根）：清理 patch 配置

---

## 需求

### 需求 1：Rust 后端依赖升级

**用户故事：** 作为开发者，我希望将 Tauri Rust 依赖升级到 2.x，以便应用能在 macOS 26 上正常启动。

#### 验收标准

1. WHEN 执行 `cargo build` THEN 系统 SHALL 使用 `tauri = "2"` 和 `tauri-build = "2"` 成功编译，无编译错误
2. WHEN 检查 Cargo.lock THEN 系统 SHALL 显示 tao 版本为 0.35.x 或更高
3. WHEN 检查根目录 Cargo.toml THEN 系统 SHALL 不包含任何 `[patch.crates-io]` 的 tao 补丁配置
4. IF 目标平台为 macOS THEN `src-tauri/Cargo.toml` SHALL 保留 `core-foundation`、`core-graphics`、`objc` 等 macOS 平台依赖
5. IF 目标平台为 Windows THEN `src-tauri/Cargo.toml` SHALL 保留 `winapi`、`winreg` 等 Windows 平台依赖，且这些依赖版本不变

---

### 需求 2：tauri.conf.json 配置文件迁移

**用户故事：** 作为开发者，我希望将 `tauri.conf.json` 迁移到 Tauri 2.x 格式，以便构建配置正确生效。

#### 验收标准

1. WHEN 读取 `tauri.conf.json` THEN 系统 SHALL 使用 Tauri 2.x 的配置结构（移除 `tauri.allowlist`，改用 `plugins` 和 `capabilities`）
2. WHEN 配置系统托盘 THEN 系统 SHALL 使用 `trayIcon` 替代 `systemTray` 字段
3. WHEN 配置窗口 THEN 系统 SHALL 使用 `app.windows` 替代 `tauri.windows` 字段
4. WHEN 配置构建 THEN 系统 SHALL 使用 `frontendDist` 替代 `distDir`，`devUrl` 替代 `devPath`
5. WHEN 配置权限 THEN 系统 SHALL 创建 `capabilities/default.json` 文件，声明 `shell:open`、`notification`、`global-shortcut` 等所需权限
6. WHEN 配置 Windows 构建 THEN 系统 SHALL 保留 `bundle.windows.webviewInstallMode`（downloadBootstrapper）和 `bundle.windows.wix.language`（zh-CN）配置不变

---

### 需求 3：Rust 后端 API 迁移（main.rs）

**用户故事：** 作为开发者，我希望将 `main.rs` 中的 Tauri 1.x API 迁移到 2.x，以便后端逻辑正常运行。

#### 验收标准

1. WHEN 使用系统托盘 THEN 系统 SHALL 使用 `tauri::tray::TrayIconBuilder` 替代 `SystemTray` / `SystemTrayMenu` / `CustomMenuItem`
2. WHEN 处理托盘事件 THEN 系统 SHALL 使用 `on_tray_icon_event` 替代 `on_system_tray_event`，事件类型使用 `TrayIconEvent`
3. WHEN 获取窗口 THEN 系统 SHALL 使用 `app.get_webview_window("main")` 替代 `app.get_window("main")`
4. WHEN 发送全局事件 THEN 系统 SHALL 使用 `app.emit("event", payload)` 替代 `app.emit_all("event", payload)`
5. WHEN 使用 `tauri::Manager` THEN 系统 SHALL 确保正确导入 `use tauri::Manager`
6. WHEN 构建应用 THEN 系统 SHALL 保留所有现有 `#[tauri::command]` 函数，无需修改命令签名
7. WHEN 应用启动 THEN 系统 SHALL 在 macOS 26 上不崩溃，正常显示主窗口
8. WHEN 编译 Windows 目标 THEN 系统 SHALL `#[cfg(target_os = "windows")]` 条件编译块内的代码保持不变，编译通过

---

### 需求 4：前端 JS API 迁移（main.js）

**用户故事：** 作为开发者，我希望将前端 JS 中的 Tauri invoke 调用迁移到 2.x，以便前后端通信正常工作。

#### 验收标准

1. WHEN 前端调用后端命令 THEN 系统 SHALL 使用 `window.__TAURI__.core.invoke()` 替代 `window.__TAURI__.tauri.invoke()`（如有变更）
2. WHEN 前端监听事件 THEN 系统 SHALL 使用 `window.__TAURI__.event.listen()` 确保事件监听正常工作
3. WHEN 应用运行 THEN 系统 SHALL 所有现有功能（连接、断开、设置、自启动、权限检查）均正常工作
4. IF `withGlobalTauri` 为 true THEN 系统 SHALL 确保 `window.__TAURI__` 全局对象在 2.x 中仍然可用

---

### 需求 5：npm/CLI 工具升级

**用户故事：** 作为开发者，我希望将 `@tauri-apps/cli` 升级到 2.x，以便 `npm run dev` 和 `npm run build` 命令正常工作。

#### 验收标准

1. WHEN 检查 `package.json` THEN 系统 SHALL 使用 `@tauri-apps/cli@^2` 替代 `^1.6.3`
2. WHEN 执行 `npm run dev` THEN 系统 SHALL 成功启动开发模式，应用正常运行
3. WHEN 执行 `npm run build` THEN 系统 SHALL 成功构建发布包，无构建错误

---

### 需求 6：功能完整性验证

**用户故事：** 作为用户，我希望升级后所有原有功能保持完整，以便日常使用不受影响。

#### 验收标准

1. WHEN 应用在 macOS 26 上启动 THEN 系统 SHALL 不崩溃，正常显示主界面
2. WHEN 用户点击系统托盘图标 THEN 系统 SHALL 正确显示/隐藏主窗口
3. WHEN 用户点击托盘菜单项 THEN 系统 SHALL 正确执行连接、断开、退出等操作
4. WHEN 用户进行设备连接 THEN 系统 SHALL 正常建立连接并同步鼠标键盘事件
5. WHEN 用户修改设置 THEN 系统 SHALL 正确保存和读取配置
6. WHEN 应用在 Windows 上构建 THEN 系统 SHALL 编译成功，所有功能正常，不受 macOS 迁移影响
7. WHEN 检查界面 THEN 系统 SHALL 版本号正确显示

---

### 需求 7：测试与回归验证

**用户故事：** 作为开发者，我希望迁移后所有测试通过，以便确保代码质量。

#### 验收标准

1. WHEN 执行 `npm test` THEN 系统 SHALL 所有现有测试用例通过
2. WHEN 迁移完成 THEN 系统 SHALL 新增针对 Tauri 2.x API 调用的测试用例
3. WHEN 发现迁移引入的 bug THEN 系统 SHALL 在 `docs/bug-list.md` 中记录并新增对应测试用例
