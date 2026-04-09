# 实施计划 — KVMini 全面重构

- [ ] 1. 清理冗余项目和旧代码
   - 删除 `kvm-win/` 和 `kvm-mac/` 目录
   - 删除 `kvm-agent/` 目录（旧的独立 agent）
   - 更新根目录 `Cargo.toml`，`[workspace] members` 只保留 `["kvm-core", "src-tauri"]`
   - 删除 `src-tauri/src/connection/transport.rs`、`error.rs` 等自造轮子文件
   - _需求：1.1、1.2、1.3、1.4、1.5_

- [ ] 2. 统一版本号
   - 确认根目录 `Cargo.toml` 中 `[workspace.package] version = "0.2.0"` 为唯一版本来源
   - 修改 `src-tauri/Cargo.toml`，将 `version` 改为 `version.workspace = true`
   - 同步更新 `src-tauri/tauri.conf.json` 中的 `package.version` 为 `0.2.0`
   - _需求：4.1、4.2、4.3_

- [ ] 3. `src-tauri` 接入 `kvm-core` 依赖
   - 在 `src-tauri/Cargo.toml` 中添加 `kvm-core = { path = "../kvm-core" }` 依赖
   - 按平台分组添加平台依赖：`[target.'cfg(target_os = "windows")'.dependencies]` 和 `[target.'cfg(target_os = "macos")'.dependencies]`
   - 确认 `kvm-core` 中 `ConnectionManager`、`NetworkEvent`、`ConnectionState`、`PROTOCOL_VERSION` 等公开导出
   - _需求：2.1、8.6_

- [ ] 4. 建立 `platform/` 模块，定义 `PlatformAdapter` trait
   - 新建 `src-tauri/src/platform/mod.rs`，定义 `PlatformAdapter` trait（含输入采集、输入注入、快捷键、屏幕信息、剪贴板、开机自启动等接口）
   - 新建 `src-tauri/src/platform/windows.rs`，从 `kvm-win/src/platform/` 迁移 Windows 实现（输入采集/注入/快捷键/屏幕/剪贴板/自启动）
   - 新建 `src-tauri/src/platform/macos.rs`，从 `kvm-mac/src/platform/` 迁移 macOS 实现（含辅助功能权限检查 `permission.rs` 逻辑）
   - 确保所有平台特定代码只在 `platform/` 目录下，通过 `#[cfg(target_os)]` 隔离
   - _需求：3.6、3.7、8.2、8.3、8.4、8.5_

- [ ] 5. 重写 `src-tauri` 连接模块，接入 `kvm-core`
   - 删除旧的 `src-tauri/src/connection/` 目录下自实现的网络代码
   - 新建 `src-tauri/src/connection/mod.rs`，使用 `kvm_core::network::ConnectionManager` 管理连接
   - 实现连接状态监听（`kvm_core::network::NetworkEvent`）并通过 Tauri `emit` 推送到前端
   - 删除 `DEFAULT_PEER_IP` 硬编码常量
   - _需求：2.2、2.3、2.4、2.5、5.1_

- [ ] 6. 实现配置持久化（替换硬编码 IP）
   - 新建 `src-tauri/src/config.rs`，实现读写 `~/.config/kvmini/config.json`（含 `peer_ip`、`port`、快捷键、自动重连等字段）
   - 首次启动时 `peer_ip` 为空字符串，不预填任何 IP
   - 实现 `get_settings` 和 `save_settings` Tauri 命令，从配置文件读写，不使用硬编码值
   - _需求：5.2、5.3、5.4、5.5、5.6_

- [ ] 7. 对齐并补全所有 Tauri 命令
   - 实现 `get_app_version` 命令，返回 `env!("CARGO_PKG_VERSION")`
   - 实现 `get_local_ip` 命令，返回本机所有局域网 IP 列表
   - 实现 `connect_to_peer` 命令，调用 `kvm_core::network::ConnectionManager` 建立连接
   - 在 `invoke_handler` 中注册所有命令，确保与前端 `platform.js` 中的 `invoke` 调用完全对应
   - _需求：6.1、6.2、6.3、6.4、6.5、6.6、4.4_

- [ ] 8. macOS 编译支持与权限引导
   - 修复 `src-tauri/tauri.conf.json`，添加 macOS 编译所需的 bundle 配置（`identifier`、`entitlements` 等）
   - 在 macOS 启动时调用 `platform::PlatformAdapter::check_accessibility_permission()`，未授权时通过 Tauri `dialog` 弹出引导对话框
   - 验证 `cargo tauri build --target aarch64-apple-darwin` 编译成功，无平台相关错误
   - _需求：3.1、3.2、3.3、3.4、3.5_

- [ ] 9. 前端版本号与界面对齐
   - 修改前端版本号显示逻辑，改为通过 `invoke('get_app_version')` 从后端读取，删除所有硬编码版本字符串
   - 确认 Windows 和 macOS 前端界面完全一致（连接面板、设置面板、系统托盘菜单项）
   - 不支持的功能显示灰色禁用状态并附说明文字，不直接隐藏
   - _需求：4.4、4.5、7.9_

- [ ] 10. 验证两端功能完整性
   - 验证 Windows 端：连接/断开、UDP 发现、手动 IP 直连、快捷键切换、边缘触发、剪贴板共享、开机自启动均正常
   - 验证 macOS 端：上述所有功能均正常，辅助功能权限引导正常弹出
   - 验证两端互联：Windows ↔ macOS 握手成功，协议版本一致（`PROTOCOL_VERSION` 共用），心跳正常
   - _需求：7.1～7.8、8.1_
