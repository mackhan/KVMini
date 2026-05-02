# KVMini v0.1.35 开发总结

**日期**：2026-05-01  
**版本号**：0.1.35  
**本次交付 Bug 修复**：3 个 P0/P1 + 1 个构建流程修复

---

## 一、本次交付内容概览

| ID | 严重度 | 标题 | 修复范围 |
|---|---|---|---|
| BUG-018 | P0 | 手动连接成功后误报「TLS 不一致」 | `kvm-core/src/network.rs` 握手 wire |
| BUG-019 | P0 | 连接成功后点击「开始共享」按钮没有任何反应 | 新增 `src-tauri/src/sharing.rs`，接入 `connection::start_event_loop`，改造 MacInputCapture 线程模型，前端新增 `toggle_sharing` 调用 |
| BUG-020 | P1 | macOS 防火墙 + VPN(utun*) 两台机互扫不到 | 新增 `list_lan_ipv4_ifaces` / `discover_devices_v2` / `scan_subnet_multi`，新增诊断提示双语文案 |
| FIX-VB | P2 | build.sh 自动 bump 版本号导致跳号 | `scripts/version-update.js` 抽出 `decideBumpAction` 纯函数 + 单元测试 |

---

## 二、开发流程复盘

### 2.1 Bug 1（TLS 误报）— 向后兼容优先

**根因**：握手帧 `HandshakeAck.screen_info` 只携带 `VirtualDesktop` (desktop 结构)，不传本机 `tls_enabled`，connect_to 端永远把 `peer.tls_enabled = false`。

**修复思路**：不破坏二进制兼容 → 新增 `HandshakePayload { desktop, tls_enabled }` 内部结构，`screen_info` 字段仍复用，通过"子串探测 + 双层回退"保证新旧客户端互通。

**关键代码**：
- `HandshakePayload::screen_info_bytes()`：序列化新 payload
- `HandshakeInfo::parse_payload(bytes)`：新版优先 → 旧版 VirtualDesktop 回退 → 默认值
- `connect_to` / `handle_stream` / `accept_loop` 三处闭环透传 `local_info`

**测试**：`kvm-core/src/network.rs` tests + `__tests__/handshake_payload.test.ts` 覆盖新旧格式 roundtrip 与兼容。

### 2.2 Bug 2（开始共享按钮）— 补足接入层

**根因诊断**：plan 阶段 code-explorer 子代理扫描确认 4 个关键缺口：
1. 后端无 `toggle_sharing` 命令
2. `StateMachine` / `InputCapture` / `InputExecutor` 均存在但未接入主流程
3. `platform/mod.rs` 顶部有 `#![allow(dead_code)]`，是死码的证据
4. `connection::start_event_loop` 的 `_ => None` 把所有 `KvmFrame::InputEvent` 吞掉

**修复思路**（选择 plan 的 A 方案，一次性完整接入）：
- 新建 `src-tauri/src/sharing.rs` 的 `SharingManager`，作为 Tauri 托管单例
- `toggle_sharing` / `get_control_state` 两条 Tauri 命令
- `connection::start_event_loop` 新增 `FrameReceived(InputEvent/SwitchControl/Ack)` 与 `Disconnected` 的分支，直通 `SharingManager`
- 改造 `MacInputCapture::start_capture` → 专用子线程跑 `CFRunLoop::run_current()`，遵循 `macos-cgevent-tap-hotkey` skill
- 前端 `src/main.js`：按钮 click → `invoke('toggle_sharing')`；`listen('control-state-changed')` 根据 payload.to 切 data-i18n 并 `applyI18n()`
- 删除幽灵调用 `toggle_control` / `stop_input_capture`

**关键设计要点**：
1. **RunLoop 隔离**：Tauri 主线程跑 NSApplication RunLoop，CGEventTap 必须在专用子线程 → 子线程获取 `CFRunLoop::get_current()` 并 add_source → `CFRunLoop::run_current()` 阻塞
2. **CFMachPort 防释放**：`std::mem::forget(tap)` 保证 CGEventTap 自身不被析构
3. **Channel 转发**：CGEventTap 回调 → `tokio::sync::mpsc::unbounded_channel` → pump task 异步消费 → `ConnectionManager.send_frame`
4. **状态机驱动**：所有状态变化产生 `StateEvent`，由 `handle_one` 统一分发（StartCapture/StopCapture/SendFrame/StateChanged）

### 2.3 Bug 3（VPN 扫描）— 多接口并发

**根因**：VPN 接管默认路由后，单 socket 广播走 utun* 被丢；多网卡场景没有分接口定向广播。

**修复思路**：
- 新增 `LanIface { name, ipv4, broadcast }`
- `list_lan_ipv4_ifaces`：用 `if-addrs` 枚举，分平台黑名单过滤 VPN/loopback
- `discover_devices_v2`：对每个物理网卡 `bind(<iface_ip>:0)` + `set_broadcast(true)` + `send_to(<iface_broadcast>:port)` + `send_to(255.255.255.255:port)` 兜底
- `scan_subnet_multi`：多网卡并发 scan_subnet，结果去重
- `diagnose_network`：检测 utun* 时追加 `vpn_interference_tip`，扫描空时追加 `firewall_block_tip`（双语）

### 2.4 FIX-VB（版本号跳号）

**根因**：`build.sh` 默认每次运行都 `bump_version`（patch +1），与"版本号由用户手动决定"的铁律冲突（参考 OnenoteCopy 踩过的坑）。

**修复思路**：A 方案：
- 抽出 `decideBumpAction({ currentVersion, currentTagExists, nextTagExists, bumpDisabled })` 纯函数
- 决策矩阵：current 不存在 → skip(0)；current 存在 + next 不存在 → bump(0)；两者都存在 → bump(2)
- `BUMP_VERSION=0` 环境变量硬开关
- 单元测试 6 个，关键回归："v0.1.35 未发布时不应跳到 v0.1.36"

---

## 三、遵循的开发规则

### 3.1 TDD（严格执行）
- 每个 bug 修复前先写失败测试 → 再实现 → 再回归
- `__tests__/handshake_payload.test.ts`、`__tests__/sharing_button.test.ts`、`__tests__/lan_ifaces_filter.test.ts` 全部先于实现代码落盘

### 3.2 日志
- Rust：`log::info!` 关键节点（握手、状态切换、capture 启停、扫描接口列表）；`log::debug!` 高频事件（采样每 100 帧）；`log::warn!/error!` 降级路径
- 前端：`console.log('[KVMini] toggle sharing -> {state}')` 关键操作

### 3.3 i18n 双语同步
- 新增 10+ key 在 `src/i18n.js` 的 zh + en 两个对象同步添加
- HTML 动态按钮文本切换通过 `data-i18n` + `applyI18n()`，不硬编码

### 3.4 平台隔离
- Mac / Win 的 `InputCapture` / `InputExecutor` 通过 `cfg(target_os)` 选择，`sharing.rs` 只持有 trait 对象
- VPN 过滤黑名单按 `cfg(target_os)` 分平台定义

### 3.5 版本规则
- 本次构建 **不触发 bump**（v0.1.35 即最终版）
- Cargo.toml workspace.package.version / package.json / src/index.html meta[app-version] 三处同步
- `main.rs::get_changelog()` 顶部新增 0.1.35 中英条目

### 3.6 Bug 清单归档
- `docs/bug-list.md` 追加 BUG-018 / BUG-019 / BUG-020 三条记录（现象/根因/修改方式/修改次数=1/出现次数=1）

---

## 四、回归测试结果

| 测试集 | 通过率 | 耗时 |
|---|---|---|
| Jest (前端 + 版本管理器) | **818 / 818** ✅ | ~35s |
| cargo test workspace --no-fail-fast | **0 failure** (exitCode=0) ✅ | ~2min |
| 构建产物 DMG | `KVMini_0.1.35_aarch64.dmg` 6.3MB ✅ | ~3min |
| 实机启动 | PID 4720/4728 运行中 ✅ | - |

---

## 五、已知限制与后续计划

见 `code-review-v0.1.35.md` "八、遗留 Follow-up"：
- BUG-021：macOS 主动调用 `CGRequestListenEventAccess`（OnenoteCopy v1.0.12 经验）
- BUG-022：channel 改 bounded + drop oldest
- BUG-023：防火墙提示改为系统设置导航
- TECH-DEBT：i18n key 风格统一
- OPTIM：MouseScroll 注入（需扩展 InputExecutor trait）

---

## 六、本次提交的需求清单（用于 commit message）

1. ✅ TLS 握手双向透传 `tls_enabled`，不再误报（BUG-018）
2. ✅ 「开始共享」按钮真正生效，双端键鼠转发（BUG-019）
3. ✅ VPN 场景下多接口定向广播 + 诊断提示（BUG-020）
4. ✅ version-update 不再自动跳号，抽出纯函数 + 单元测试（FIX-VB）
5. ✅ 全平台双语 i18n 同步（zh + en）
6. ✅ 日志完善（采样 / 分级 / 平台隔离）
7. ✅ 版本号 0.1.34 → 0.1.35，changelog 中英条目同步
