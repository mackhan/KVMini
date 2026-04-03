# 实施计划：辅助功能权限引导 & 连接真实性验证

- [ ] 1. 平台隔离重构：建立 platform 模块统一接口
   - 创建 `src-tauri/src/platform/` 目录，拆分为 `mod.rs`（trait 定义）、`macos/mod.rs`、`windows/mod.rs`
   - 定义 `AccessibilityChecker` trait，macOS 实现调用真实 API，Windows 实现直接返回 `Granted`
   - 将现有 `injector.rs`、`capturer.rs` 中的平台特定代码迁移到对应子模块，公共模块只保留 trait 定义和 Mock 实现
   - 确保 macOS 编译时不含任何 `SendInput`/`SetWindowsHookEx`，Windows 编译时不含任何 `CGEvent`/`AXIsProcessTrusted`
   - _需求：7.1、7.2、7.3、7.5_

- [ ] 2. macOS 辅助功能权限真实检测
   - 在 `platform/macos/mod.rs` 中实现 `AccessibilityChecker`，调用 `AXIsProcessTrustedWithOptions` 系统 API
   - 移除 `accessibility.rs` 中硬编码返回 `Denied` 的逻辑，改为调用 platform 模块接口
   - Windows 实现返回 `Granted`（空实现），保持接口一致
   - 为 macOS 和 Windows 实现分别编写单元测试（macOS 用 Mock 验证调用路径，Windows 验证始终返回 Granted）
   - _需求：1.1、1.2、1.3、1.4、7.4、7.6_

- [ ] 3. 自签名证书配置脚本（参考 OneNoteCopy）
   - 创建 `scripts/create-dev-cert.sh`，使用 `security create-certificate` 生成自签名证书 `KVMini Dev`
   - 创建 `scripts/sign-dev.sh`，在每次 `cargo build` 后对可执行文件执行 `codesign --sign "KVMini Dev"`
   - 在 `README.md` 或 `docs/dev-setup.md` 中补充开发环境签名配置说明
   - _需求：2.1、2.2、2.3_

- [ ] 4. 辅助功能权限引导 UI
   - 在前端 `index.html` 顶部添加权限引导横幅（Banner），默认隐藏
   - 在 `main.js` 中：应用启动时调用 `check_accessibility` 命令，未授权时显示横幅；点击"前往授权"调用 `open_accessibility_settings` Tauri 命令打开系统设置；每 2 秒轮询权限状态，授权后自动隐藏横幅
   - 在后端新增 `open_accessibility_settings` Tauri 命令（macOS 打开系统设置页，Windows 为空操作）
   - 在 `styles.css` 中添加横幅样式（警告色、关闭按钮）
   - _需求：3.1、3.2、3.3、3.4、3.5_

- [ ] 5. 连接真实性验证（KVMini 握手协议）
   - 修改 `connection_manager.rs` 中的 `connect_by_ip` 命令：TCP 连接成功后，发送 `Handshake` 帧并等待 `HandshakeAck` 响应（超时 3 秒）
   - 握手成功后将 TCP stream 保存到 Tauri 管理的全局状态（`Arc<Mutex<Option<TcpStream>>>`）
   - 握手失败/超时/版本不匹配时返回对应错误信息，前端展示具体原因
   - 为握手流程编写单元测试（模拟正常握手、版本不匹配、超时三种场景）
   - _需求：4.1、4.2、4.3、4.4、4.5、4.6、6.1、6.2_

- [ ] 6. 连接状态管理与断线检测
   - 在全局状态中增加连接状态字段，连接成功后启动后台心跳检测任务（30 秒超时）
   - 对端断开或心跳超时时，通过 Tauri `emit` 事件通知前端，前端更新 UI 显示"连接已断开"
   - 新增 `disconnect` Tauri 命令，关闭 TCP 连接并清除全局连接状态
   - 前端添加"断开连接"按钮，连接成功后显示，点击调用 `disconnect` 命令
   - Windows 防火墙错误时在连接失败提示中增加"请检查防火墙设置"提示
   - _需求：5.1、5.2、5.3、6.3_

- [ ] 7. 集成测试与平台隔离验证
   - 编写集成测试验证：握手全链路（client 发起 → server 响应 → 状态保存）
   - 编写平台隔离验证测试：确认 macOS 模块不含 Windows 符号，Windows 模块不含 macOS 符号（通过 `#[cfg]` 条件编译测试）
   - 运行 `cargo test` 确保所有测试通过，无平台交叉依赖
   - _需求：7.3、7.7_
