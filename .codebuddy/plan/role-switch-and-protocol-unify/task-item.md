# 实施计划：角色自由切换 & 协议统一

## 任务清单

- [ ] 1. 将 kvm-agent 协议层替换为二进制帧协议
   - 将 `kvm-agent/src/protocol.rs` 中的 JSON 文本协议替换为与 `src-tauri/src/protocol.rs` 完全一致的二进制帧编解码实现（帧头 + 类型 + 长度 + 数据）
   - 实现 `Frame::read_from` 和 `Frame::write_to` 异步读写方法
   - _需求：1.1、1.2_

- [ ] 2. 统一握手流程（kvm-agent 端）
   - 修改 `kvm-agent/src/server.rs` 中的握手逻辑：被控端等待接收 `Handshake` 帧后回复 `HandshakeAck` 帧，移除原有 Mac 端主动发握手的逻辑
   - 添加协议版本校验，版本不匹配时拒绝连接并记录错误
   - 将监听端口从 9999 改为 8889
   - _需求：1.3、1.4、1.5_

- [ ] 3. 实现 kvm-agent 二进制帧输入事件执行器
   - 修改 `kvm-agent/src/input_executor.rs`，将原来解析 JSON 事件的逻辑替换为解析二进制 `Frame` 枚举
   - 支持处理 `MouseMove`、`MouseButton`、`MouseScroll`、`KeyEvent`、`Heartbeat` 五种帧类型
   - _需求：2.1、2.2、2.3、2.4、2.5_

- [ ] 4. 在 kvm-agent 中实现角色管理器
   - 新建或修改 `kvm-agent/src/role_manager.rs`，参考 `src-tauri/src/role_manager.rs` 实现 `AppRole` 枚举（`Controller` / `Controlled`）和角色切换逻辑
   - Mac 端启动时默认角色设为 `Controlled`（被控端）
   - 提供 `switch_to_controller()` 和 `switch_to_controlled()` 方法，切换前优雅断开当前连接
   - _需求：3.2、3.6、3.7_

- [ ] 5. 修改 src-tauri Windows 端默认角色为主控端
   - 修改 `src-tauri/src/role_manager.rs` 中的初始角色，确保 Windows 启动时默认为 `Controller`（主控端）
   - 修改 `src-tauri/src/main.rs` 启动流程，启动时自动初始化主控端键鼠采集服务
   - _需求：3.1_

- [ ] 6. 实现 kvm-agent 主控端连接流程
   - 在 `kvm-agent/src/server.rs` 中新增主控端模式：当角色切换为 `Controller` 时，停止 TCP 监听，改为主动连接对端 IP:8889，发送 `Handshake` 帧并等待 `HandshakeAck`
   - 连接成功后启动键鼠事件采集并发送
   - _需求：3.3_

- [ ] 7. 完善 src-tauri Windows 端被控端切换流程
   - 修改 `src-tauri/src/server.rs` 和 `src-tauri/src/role_manager.rs`，当角色切换为 `Controlled` 时，断开主控端连接，启动 TCP 监听服务（端口 8889），等待对端连接后执行输入注入
   - _需求：3.4、3.6、3.7_

- [ ] 8. 角色切换后通知前端更新 UI（双端）
   - 在 `src-tauri/src/main.rs` 中，角色切换完成后通过 Tauri 事件（`emit`）向前端发送 `role-changed` 事件，携带当前角色信息
   - 在 `kvm-agent/src/tray.rs` 中，角色切换后更新托盘菜单文字（"切换为主控端" / "切换为被控端"）
   - _需求：3.5_

- [ ] 9. 实现连接状态实时反馈（双端）
   - 在 `src-tauri/src/connection_manager.rs` 中新增连接状态事件推送：连接成功时发送 `connection-status` 事件（含对端 IP），断开时发送断开事件
   - 在 `kvm-agent/src/server.rs` 中同步更新托盘图标/菜单反映连接状态
   - 主控端断开时自动重连（最多 5 次，间隔 3 秒）；被控端断开时继续监听
   - _需求：4.1、4.2、4.3、4.4_

- [ ] 10. 更新前端 UI（src-tauri）支持角色切换与状态显示
   - 在前端页面（`src/` 目录）新增"切换为主控端"/"切换为被控端"按钮，调用对应 Tauri 命令
   - 监听 `role-changed` 和 `connection-status` 事件，实时更新 UI 显示当前角色和连接状态（已连接/已断开 + 对端 IP）
   - _需求：3.3、3.4、3.5、4.1、4.2_
