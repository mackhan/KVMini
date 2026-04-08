# 实施计划：角色自由切换修复

- [ ] 1. 修复 IP 配置（双端）
   - 修改 `kvm-agent/src/connection/mod.rs`，将 `DEFAULT_PEER_IP` 改为 `30.19.24.43`（Windows IP）
   - 修改 `src-tauri/src/connection/mod.rs`，将 `DEFAULT_PEER_IP` 改为 `30.19.140.39`（Mac IP）
   - 启动时打印本机 IP 和对端 IP 信息
   - _需求：3.1、3.2、3.3、3.4_

- [ ] 2. 重构连接模型：双端始终监听
   - 修改 `kvm-agent/src/connection/manager.rs`，在启动时同时开启监听循环（独立 tokio task），无论当前角色
   - 修改 `src-tauri/src/connection/manager.rs`，同样在启动时开启监听循环
   - 实现"新连接到来时断开旧连接"逻辑
   - _需求：1.1、1.2、1.4_

- [ ] 3. 修复握手协议：连接发起方为主控端
   - 修改握手逻辑：主动发起连接的一端在握手帧中声明自己为主控端（Controller）
   - 接受连接的一端在握手 ACK 中确认自己为被控端（Controlled）
   - 握手完成后双端分别启动对应的发送/接收循环
   - _需求：1.3、2.5_

- [ ] 4. 修复 Mac 端角色切换逻辑（kvm-agent）
   - 修改 `switch_to_controller()`：主动连接 `30.19.24.43:8889`，发起握手声明主控端
   - 修改 `switch_to_controlled()`：断开当前主控连接，回到纯监听状态（不再主动连接）
   - 确保切换时不影响监听循环的运行
   - _需求：2.1、2.2_

- [ ] 5. 修复 Windows 端角色切换逻辑（src-tauri）
   - 修改 `switch_to_controller()`：主动连接 `30.19.140.39:8889`，发起握手声明主控端
   - 修改 `switch_to_controlled()`：断开当前主控连接，回到纯监听状态
   - 确保切换时不影响监听循环的运行
   - _需求：2.3、2.4_

- [ ] 6. 修复屏幕边缘触发逻辑（双端）
   - 修改 `kvm-agent/src/control/edge.rs`：被控端右边缘触发 `switch_to_controller`，主控端左边缘触发 `switch_to_controlled`
   - 修改 `src-tauri/src/control/edge.rs`：主控端右边缘触发 `switch_to_controlled`，被控端左边缘触发 `switch_to_controller`
   - 添加 50ms 防抖处理
   - _需求：4.1、4.2、4.3、4.4、4.5_

- [ ] 7. 修复快捷键触发逻辑（双端）
   - 修改 `kvm-agent/src/control/hotkey.rs`：Ctrl+Alt+K 触发角色切换，切换成功/失败后显示系统通知
   - 修改 `src-tauri/src/control/hotkey.rs`：同样的快捷键和通知逻辑
   - _需求：5.1、5.2、5.3_
