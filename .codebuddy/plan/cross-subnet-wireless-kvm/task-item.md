# 实施计划：跨网段无线键鼠共享

- [ ] 1. 新增 Rust 后端命令：手动 IP 直连
   - 在 `src-tauri/src/platform.rs` 中新增 `connect_by_ip(ip: String, port: u16)` 命令
   - 实现 IPv4 格式校验，非法格式返回错误
   - 使用带超时（5秒）的 TCP 连接，区分"连接拒绝"、"超时"、"不可达"三种错误类型并返回对应错误码
   - 在 `main.rs` 中注册该命令
   - 编写单元测试：合法/非法 IP 格式、超时、拒绝三种场景，确保 macOS 和 Windows 均可通过
   - _需求：1.2、1.4、1.5、5.7_

- [ ] 2. 新增 Rust 后端命令：获取本机所有网卡 IP
   - 在 `platform.rs` 中新增 `get_network_interfaces()` 命令，枚举所有非回环 IPv4 地址及网卡名称
   - 返回结构体列表 `[{name, ip}]`，供前端设置面板展示
   - Windows 分支需正确处理 `ipconfig` 输出中的 `(Preferred)` 后缀，确保返回干净的 IP 字符串
   - 在 `main.rs` 中注册该命令
   - _需求：3.1、5.5_

- [ ] 3. 新增 Rust 后端命令：监听地址配置（含 macOS + Windows 测试）
   - 在 `platform.rs` 中新增 `set_listen_address(ip: String, port: u16)` 和 `get_listen_config()` 命令
   - 验证端口范围（1024-65535），保存配置到本地文件，并重启 TCP 监听服务
   - 在 `main.rs` 中注册上述命令
   - 编写单元测试（macOS 和 Windows 的 `cargo test` 均需通过）：
     - 合法端口（1024-65535）保存成功
     - 非法端口（< 1024 或 > 65535）返回错误
     - 配置持久化：保存后重新读取 `get_listen_config` 返回相同值
     - 选择"所有网卡（0.0.0.0）"时正确保存
   - 测试不得使用仅在单一平台可用的 API 或路径格式
   - _需求：3.2、3.3、3.4、3.6、5.7_

- [ ] 4. 前端：手动 IP 连接 UI 与交互逻辑
   - 在 `src/index.html` 设备发现面板下方新增"手动连接"区域：IP 输入框 + 连接按钮
   - 在 `src/main.js` 中实现 `connectByIp()` 函数：调用 `platform.js` 封装的 `connectByIp`，处理三种错误类型并显示对应 Toast 提示
   - 在 `src/styles.css` 中添加手动连接区域样式
   - 编写 UI 测试：输入框渲染、合法/非法 IP 提示、成功/失败 Toast
   - _需求：1.1、1.3、1.4、1.5_

- [ ] 5. 前端：连接历史扩展（支持手动 IP 记录）
   - 扩展 `src/main.js` 中的历史记录逻辑：手动 IP 连接成功后写入历史，记录 `{ip, name, type: 'manual', lastConnected}`
   - 历史列表中用不同图标区分"自动发现"和"手动 IP"设备
   - 点击手动 IP 历史条目时直接调用 `connectByIp()` 重连
   - 历史记录超过 20 条时自动清理最旧条目
   - 编写测试：历史写入、图标区分、一键重连、超限清理
   - _需求：2.1、2.2、2.3、2.5_

- [ ] 6. 前端：设置面板扩展（监听地址配置，含 macOS + Windows 测试）
   - 在 `src/index.html` 设置模态框中新增"监听地址"选择区：下拉列表展示所有网卡 IP + "所有网卡（0.0.0.0）"选项，以及端口输入框
   - 在 `src/main.js` 中调用 `get_network_interfaces` 填充下拉列表，保存时调用 `set_listen_address`
   - 主界面状态面板中实时显示当前监听 IP 和端口
   - 编写 `npm test` 测试（需覆盖 macOS 和 Windows 两端行为差异）：
     - 网卡下拉列表正确渲染（含"所有网卡"选项）
     - 端口输入框非法值时显示错误提示
     - 保存成功后主界面状态面板更新显示新的监听地址
     - 前端逻辑对 macOS 和 Windows 两端返回的网卡名称格式均能正确处理
   - _需求：3.1、3.2、3.4、3.5、3.7、5.8_

- [ ] 7. 前端：连接延迟监测与断线重连
   - 在 `src/main.js` 中实现 `startLatencyMonitor()`：连接建立后每 2 秒发送心跳包，计算 RTT 并更新界面显示
   - 延迟 > 50ms 时以黄色警告色显示，连接中断时自动重连（最多 3 次，间隔 2 秒），重连失败后弹出通知
   - 在 `platform.js` 中封装对应 Tauri 命令调用
   - 编写测试：正常延迟显示、超阈值警告色、断线重连逻辑
   - _需求：4.2、4.3、4.4_

- [ ] 8. 跨平台一致性验证与全量测试
   - 更新 `scripts/check-platform-features.js`，将手动连接、历史记录、网络诊断、监听配置四项加入检查项
   - 确认 macOS 和 Windows 的 `platform.rs` 条件编译分支均已覆盖上述命令
   - 运行全量测试（`npm test` + `cargo test`），确保所有新增测试（包括任务 3 的 Rust 测试和任务 6 的 UI 测试）在两个平台上均通过
   - _需求：5.1、5.3、5.4、5.7、5.8_
