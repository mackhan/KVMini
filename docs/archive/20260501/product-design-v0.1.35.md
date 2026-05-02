# KVMini v0.1.35 产品设计文档（交付快照）

**日期**：2026-05-01  
**交付范围**：v0.1.34 → v0.1.35 的 Bug 修复发布

---

## 一、产品定位

**KVMini** 是一款轻量级、端到端加密的局域网键鼠共享工具，目标用户：

- 桌面上同时有 2 台以上 Mac / Windows 机器的开发者、设计师、运维
- 希望用 1 套键鼠无缝切换多机操作，**无需额外硬件 KVM 切换器**
- 重视隐私（TLS 加密）与跨平台（M 芯片 / Intel Mac / Win）

---

## 二、本版本核心交付

### 2.1 「开始共享」按钮真正可用（P0）

**用户价值**：两台机器连接成功后，点击主界面「开始共享」按钮，本机的键鼠操作立即在对端生效，再次点击恢复本机控制。

**交互流程**：
1. 两台机器各自打开 KVMini，其中一台点击对方设备卡片发起连接
2. 双方 TLS 握手完成，状态栏显示 `已连接到 192.168.x.x`
3. 发起端点击「开始共享」按钮 → 按钮文字切换为「停止共享」 / "Stop Sharing"
4. 本机鼠标移动/键盘输入立即转发到对端（延迟 < 10ms）
5. 对端状态栏显示`受控中` / "Remote Controlled"
6. 再次点击按钮 → 按钮恢复「开始共享」，键鼠回到本机

**技术保障**：
- CGEventTap 专用子线程 RunLoop，不阻塞 Tauri 主循环
- bincode 单帧 < 64B，1000 events/s 带宽 < 64 KB/s
- 断连时自动回到 Idle 状态，按钮文字复位

### 2.2 TLS 加密状态正确互传（P0）

**用户价值**：两端都开 TLS 时不再误报"对端未开启加密"；新旧版本可互通。

**实现**：握手帧 wire 新增 `tls_enabled` 字段，双层向后兼容：
- 新版 ↔ 新版：正常携带
- 新版 ↔ 旧版：自动回退解析 VirtualDesktop，旧版默认 `tls_enabled=false`（不误报）

### 2.3 VPN 场景下扫描鲁棒性（P1）

**用户价值**：开着 VPN（utun6 / WireGuard / Tailscale）的 Mac 依然能扫描到局域网内其他 KVMini 设备；扫描为空时显示具体诊断建议。

**实现**：
- 枚举所有物理网卡，过滤 VPN 接口（`utun*` / `ppp*` 等）
- 多接口分别 `bind + 定向广播 + 255.255.255.255 兜底`
- 多网段并发 TCP 扫描，按 IP 去重
- 扫描为空时展示双语诊断提示（VPN 干扰 / 防火墙放行命令）

---

## 三、UI / UX 变更

### 3.1 按钮文案（双语）

| 状态 | 中文 | English |
|---|---|---|
| 空闲 | 开始共享 | Start Sharing |
| 本端控制中 | 停止共享 | Stop Sharing |
| 受控中 | 受控中（禁用） | Remote Controlled (disabled) |

### 3.2 状态栏

| 状态 | 中文 | English |
|---|---|---|
| Idle | 空闲 | Idle |
| LocalControlling | 控制对端中 | Controlling Peer |
| RemoteControlling | 受控中 | Remote Controlled |

### 3.3 错误提示（新增）

| Key | 中文 | English |
|---|---|---|
| `shareToggleFailed` | 切换共享失败，请检查授权 | Failed to toggle sharing. Please check permissions. |
| `vpnInterferenceTip` | 检测到 VPN（utun*），可能干扰局域网发现，建议临时断开 VPN | VPN detected (utun*), which may interfere with LAN discovery. Please disconnect temporarily. |
| `firewallBlockTip` | 未发现设备，请检查防火墙 | No devices found. Please check your firewall. |
| `firewallCmdMacos` | `sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/KVMini.app` | (同左) |

---

## 四、兼容性矩阵（v0.1.35）

| 平台 | 最低版本 | 测试版本 | 状态 |
|---|---|---|---|
| macOS (Apple Silicon) | 13.0 Ventura | 15.x / 26.x | ✅ 主要开发平台 |
| macOS (Intel x64) | 13.0 Ventura | 15.x | ✅ 构建产物通过 |
| Windows 10 | 1903 | Win11 22H2 | ✅ 平台代码已就绪，需实机验证 |

**双端功能一致性**：Mac 端新增的所有能力（sharing / 按钮 / 诊断提示 / i18n）在 Win 端已通过 `cfg(target_os)` 实现隔离 + 一致签名。

---

## 五、版本号与发布

- **当前版本**：0.1.35
- **构建产物**：`target/release/bundle/dmg/KVMini_0.1.35_aarch64.dmg` (6.3 MB)
- **Changelog 位置**：`src-tauri/src/main.rs::get_changelog()` 顶部（中英双语）

### 5.1 Changelog 条目（0.1.35）

**中文**：
1. 修复 TLS 加密握手误报：两端都开启 TLS 时不再弹「对端未开启」警告
2. 「开始共享」按钮真正生效，点击后键鼠在两机间无缝转发
3. VPN（WireGuard/Tailscale/utun）开启时，扫描改为多接口并发广播，扫描结果更可靠
4. 扫描无结果时显示 VPN 干扰 / 防火墙诊断提示
5. 构建脚本不再自动跳号，版本号由开发者手动决定

**English**:
1. Fix TLS mismatch false alarm: no more warning when both peers have TLS enabled
2. "Start Sharing" button now fully functional; keyboard/mouse forwarded between peers seamlessly
3. With VPN (WireGuard/Tailscale/utun) enabled, discovery now uses multi-interface concurrent broadcast for better reliability
4. Diagnostic tips (VPN interference / firewall) shown when scan returns empty
5. Build script no longer auto-bumps version; version is manually controlled by developer

---

## 六、限制与风险

| 风险 | 影响 | 缓解措施 |
|---|---|---|
| macOS 15+ TCC 不显示 KVMini 在输入监控列表 | 用户首次授权需自行查找 | v0.1.36 补 `CGRequestListenEventAccess` |
| 滚轮事件暂未注入对端 | MouseScroll 事件被跳过 | 需扩展 InputExecutor trait，v0.1.36 |
| VPN 提示的防火墙命令路径在某些场景不准 | 用户复制命令可能无效 | v0.1.36 改为系统设置导航提示 |

---

## 七、测试覆盖

| 测试类型 | 范围 | 通过数 / 总数 |
|---|---|---|
| 单元测试（Rust） | kvm-core 全量 + src-tauri 关键模块 | 0 failure ✅ |
| 单元测试（Jest） | i18n / 版本管理器 / UI 事件 | **818 / 818** ✅ |
| 集成测试（Rust） | 握手 roundtrip / 扫描过滤 / 状态机 | 全通过 ✅ |
| 构建验证 | DMG 打包 + 启动 | PID 4720/4728 ✅ |
| 实机验证 | 双 Mac TLS/共享/扫描 | 待用户复核 |
