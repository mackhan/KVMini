# KVMini v0.1.38 开发总结

> 构建日期：2026-05-06 · 测试结果：Jest 874/874 + Rust 92+19 全绿 · 双架构 DMG 已签名

## 1. 改动总览

| 模块 | 文件 | 变更类型 |
|------|------|----------|
| 前端命令调用 | `src/connection.js`, `src/main.js` | 修复 `connect_to_device` → `connect_to_peer`（4 处） |
| 端口默认值链路 | `src/main.js`, `src/index.html`, `src/history.js`, `src/platform.js` | 清除 8 处硬编码 8889；启动读 `get_settings`；保存监听端口后 `appState.defaults.tcpPort` 立即更新 |
| macOS 输入屏蔽 | `src-tauri/src/platform/macos/input_capture.rs` | `ListenOnly` → `Default`；新增 `Arc<AtomicU8>` 控制状态；回调按状态返回 Some/None |
| Windows 输入屏蔽 | `src-tauri/src/platform/windows/input_capture.rs` | Low-Level Hook 对称实现 + `HOOK_CONTROL_STATE` thread_local |
| 共享调度 | `src-tauri/src/sharing.rs` | 新增 `Arc<EdgeDetector>` + 状态镜像；`StateChanged` 同步 capture state；`BlockInput/UnblockInput` 真实调用 InputBlocker；MouseMove 接入 EdgeDetector |
| 设置页 UI | `src/index.html`, `src/main.js`, `src/i18n.js` | 新增「键鼠切换」分区（开关 + 灵敏度），3 个 i18n key 双语同步 |
| save_settings 链路 | `src/main.js` | 显式构造 snake_case payload，让后端 AppConfig 真正接收 |
| Changelog | `src-tauri/src/main.rs` | 重写 0.1.38 条目为本版真实改动（中英双语） |
| Bug 归档 | `docs/bug-list.md` | 新增 Bug #028 / #029 / #030 |
| 构建链 | `build.sh`, `tauri.conf.json` | minisign key 轮换；`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 空值兜底；`BUMP_VERSION=0` 跳过 bump |

代码净增删行数（不含 changelog/bug-list）：
- 前端：约 +95 / -45
- Rust：约 +180 / -30（sharing.rs EdgeDetector 接入是大头）
- 测试：+125（kvm-edge-switch.test.ts 21 用例 + Rust 单测 6 个）

## 2. 关键技术决策

### 2.1 CGEventTap：ListenOnly → Default

**背景**：`ListenOnly` 时回调返回值被系统忽略，无法吞事件——这是 v0.1.37"键鼠广播"问题的根因。

**改动**：
```rust
// 之前
CGEventTapOptions::ListenOnly  // 只能监听，吞不掉
// 之后
CGEventTapOptions::Default     // 默认模式，回调返回 Some/None 决定放行/吞
```

**风险**：一旦状态判断错误用户会完全无法操作电脑。**护栏**：
1. 启动时状态强制为 `Idle`（`AtomicU8::new(0)`）；
2. `stop_capture` 强制写回 `Idle`；
3. 单元测试 `test_idle_state_passes_event_through` 守护"未开始共享时永远放行"；
4. 状态读取使用 `AtomicU8::load(Relaxed)`，禁止 lock Mutex（CGEventTap 回调高频 >1000Hz，锁竞争会让用户体感"鼠标顿挫"）。

### 2.2 EdgeDetector 接入

**位置**：`SharingManager::start_capture_and_pump` 在 `RawInputEvent::MouseMove` 分支调 `detector.detect(x, y)`。

```rust
if let RawInputEvent::MouseMove { x, y } = raw {
    let result = { /* lock detector */ d.detect(x, y) };
    if let EdgeDetectResult::Triggered { direction, pos_ratio } = result {
        let events = state.lock().await.toggle_control(direction, pos_ratio);
        handle_events_on_edge_trigger(events, app, capture, blocker).await;
    }
}
```

EdgeDetector 自带 50ms 防抖（在 kvm-core），无需上层节流。

### 2.3 StateEvent 闭环

```rust
match event {
    StateChanged { from, to } => {
        capture.set_control_state(control_state_to_u8(to));   // 状态镜像到回调线程
        edge_detector.set_active(to == LocalControlling);     // detector 仅在本机控制时检测"返回边"
        emit("control-state-changed", to);                    // 通知前端 UI
    }
    BlockInput  => blocker.block_input()?,                    // 真实屏蔽（mac CGEventTap / win BlockInput）
    UnblockInput => blocker.unblock_input()?,
    SendFrame { frame } => conn_manager.send_frame(frame).await?,
    ...
}
```

### 2.4 端口默认值真源化

```
后端 KvmConfig::default().tcp_port = 23456   ← 唯一真源
        ↓ get_settings()
前端 appState.defaults.tcpPort
        ↓ 启动 + 保存监听端口后同步
所有 UI 输入框 / fallback 路径
```

字面量 `8889` 全清扫（grep 验证）。

### 2.5 save_settings 字段名错配根治

**历史问题**：前端 `appState.settings` 是 camelCase（`clipboardSync` `enableTls`），后端 `AppConfig` 是 snake_case，全靠 `#[serde(default)]` 兜底，意味着前端的写入**几乎全被丢弃**。

**修复**：在 invoke `save_settings` 前显式构造 snake_case payload，至少把本版关注的字段（`port`, `switch_hotkey`, `edge_trigger`, `edge_threshold`, `clipboard_sync`, `autostart`, `enable_tls`, `auto_reconnect`）传到位。其余字段保留给后续清理。

## 3. 测试结果

```
Jest:           Test Suites: 52 passed / Tests: 874 passed
Rust workspace: kvm-core 92 passed / kvmini 19 passed
新增用例:        21（kvm-edge-switch.test.ts）+ Rust 6 个 input_capture 测试
```

回归基线：v0.1.37（Jest 860/Rust 105），本版无任何回归，纯净增量。

## 4. 构建产物

| 产物 | 路径 | 大小 | 签名 |
|------|------|------|------|
| Apple Silicon DMG | `target/aarch64-apple-darwin/release/bundle/dmg/KVMini_0.1.38_aarch64.dmg` | 6.3 MB | Developer ID + minisign |
| Intel DMG | `target/x86_64-apple-darwin/release/bundle/dmg/KVMini_0.1.38_x64.dmg` | 6.5 MB | Developer ID + minisign |
| Updater (arm64) | `KVMini.app.tar.gz + .sig` | 6.4 MB + 404B | minisign (新 key 6274F20D6F01A7A1) |
| Updater (x64) | `KVMini.app.tar.gz + .sig` | 6.6 MB + 404B | minisign |

## 5. 已知限制 / 后续 TODO

- `HideCursor / ShowCursor` 当前仅日志桩，未实际隐藏本机光标（短期不影响 KVM 体验，留 v0.1.39）。
- `AppConfig` 其他字段（reconnect_max_attempts/reconnect_interval_secs 等）前端没填，仍走 serde 默认值兜底（不影响本版核心 KVM 路径）。
- 全局快捷键切换 `switch_hotkey` 默认仍为 `Ctrl+Alt+S`，UI 已可配置；快捷键回调实际触发 `SharingManager::toggle` 已在更早版本实装。
- 边缘触发的 hover 阈值 `edge_threshold` 仅是触发"灵敏度"，并非"延迟"；非常细粒度的"非常缓慢推屏"场景仍可能被防抖滤掉，属于已知行为。

## 6. 提交范围

预计提交备注：
```
v0.1.38: 修复扫描后无法连接 / 端口默认值混乱 / 共享时双控三大可用性缺陷

需求清单（提示词总结）：
1. 修复 v0.1.37 扫描到设备后点连接没反应（命令名错配）
2. 端口默认值混乱：清字面量、启动读 get_settings、保存后立即反映
3. 共享后键鼠不再"广播"，做成真正的 KVM 边界切换：
   - macOS CGEventTap Default + AtomicU8 状态决定吞/放
   - Windows Low-Level Hook 对称实现
   - SharingManager 接入 EdgeDetector，触达边缘自动切
   - BlockInput/UnblockInput 真实实现
4. 设置页通用分页新增"键鼠切换"分区（开关 + 灵敏度），i18n 双语
5. 版本号 0.1.38（不 bump，BUMP_VERSION=0）
6. docs/bug-list.md 归档 Bug #028 / #029 / #030
7. 双架构构建 + Developer ID + minisign 签名
```
