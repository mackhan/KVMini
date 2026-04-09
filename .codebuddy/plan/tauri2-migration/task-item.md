# 实施计划：KVMini Tauri 1.x → 2.x 迁移

- [ ] 1. 升级 Rust 后端依赖
   - 修改 `src-tauri/Cargo.toml`：将 `tauri = "1"` 改为 `tauri = "2"`，`tauri-build = "1"` 改为 `tauri-build = "2"`
   - 添加 Tauri 2.x 所需插件依赖：`tauri-plugin-shell`、`tauri-plugin-notification`、`tauri-plugin-global-shortcut`
   - 清理根目录 `Cargo.toml` 中所有 `[patch.crates-io]` 补丁配置（tao、objc2 等）
   - 执行 `cargo build` 验证编译通过，确认 Cargo.lock 中 tao 版本为 0.35.x+
   - _需求：1.1、1.2、1.3、1.4、1.5_

- [ ] 2. 迁移 tauri.conf.json 配置文件
   - 将 `tauri.windows` 改为 `app.windows`，`systemTray` 改为 `trayIcon`
   - 将 `distDir` 改为 `frontendDist`，`devPath` 改为 `devUrl`
   - 移除 `tauri.allowlist`，改用 `plugins` 配置块
   - 保留 Windows bundle 配置（`webviewInstallMode: downloadBootstrapper`、`wix.language: zh-CN`）
   - _需求：2.1、2.2、2.3、2.4、2.6_

- [ ] 3. 创建 capabilities 权限配置
   - 新建 `src-tauri/capabilities/default.json`，声明 `shell:open`、`notification:default`、`global-shortcut:allow-register` 等权限
   - _需求：2.5_

- [ ] 4. 迁移 main.rs 后端 API
   - 将 `SystemTray` / `SystemTrayMenu` / `CustomMenuItem` 替换为 `tauri::tray::TrayIconBuilder`
   - 将 `on_system_tray_event` 替换为 `on_tray_icon_event`，事件类型改为 `TrayIconEvent`
   - 将 `app.get_window("main")` 替换为 `app.get_webview_window("main")`
   - 将 `app.emit_all(...)` 替换为 `app.emit(...)`
   - 注册 Tauri 2.x 插件（`tauri_plugin_shell`、`tauri_plugin_notification`、`tauri_plugin_global_shortcut`）
   - 保持所有 `#[tauri::command]` 函数签名不变，保持 `#[cfg(target_os = "windows")]` 块不变
   - _需求：3.1、3.2、3.3、3.4、3.5、3.6、3.8_

- [ ] 5. 迁移前端 JS API（main.js）
   - 检查并更新 `invoke` 调用路径（`window.__TAURI__.core.invoke` vs `window.__TAURI__.tauri.invoke`）
   - 验证 `window.__TAURI__.event.listen` 事件监听在 2.x 中正常工作
   - 确认 `withGlobalTauri: true` 配置保留，`window.__TAURI__` 全局对象可用
   - _需求：4.1、4.2、4.4_

- [ ] 6. 升级 npm CLI 工具
   - 修改 `package.json`：将 `@tauri-apps/cli` 从 `^1.6.3` 升级到 `^2`
   - 执行 `npm install` 更新依赖
   - _需求：5.1_

- [ ] 7. 编写迁移相关测试用例（TDD）
   - 针对 Tauri 2.x 托盘 API、窗口获取、事件发送编写单元测试
   - 针对 macOS 26 启动不崩溃场景编写集成测试（验证 Bug #001 修复）
   - 在 `docs/bug-list.md` 中记录 Bug #001 的根因和修复方式
   - _需求：7.1、7.2、7.3_

- [ ] 8. 全量功能验证与回归测试
   - 执行 `npm run dev` 验证开发模式正常启动（macOS 26 不崩溃）
   - 验证托盘图标、菜单、窗口显示/隐藏、连接/断开、设置保存等功能完整
   - 执行 `npm test` 确保所有测试通过
   - 执行 `npm run build` 确保构建成功，版本号正确显示
   - _需求：6.1、6.2、6.3、6.4、6.5、6.6、6.7、5.2、5.3_
