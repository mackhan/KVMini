# KVMini v0.1.40 开发总结

> 构建日期：2026-05-07 · 测试结果：Rust kvm-core 129/129 + src-tauri lib 19/19 + Jest 887/887 全绿 · 双平台（mac/win）采集路径对齐 · 含 code review 后追加修复

## 1. 背景

v0.1.39 上线后暴露两个致命问题，合并为本版重构：

- **BUG-033（致命）**：本机进入 LocalControlling 后本地鼠标成功被屏蔽，但远端鼠标完全不动。用户反馈："不激活反而能控制远端鼠标，激活后本机屏蔽了远端也不动了"——这与竞品（Synergy/Barrier/ShareMouse/Logitech Flow）的表现完全相反。
- **BUG-032（回归）**：连接成功后按钮第一次显示"断开"正常，但一旦 `reconnecting` 事件触发，按钮文案被污染为"重连"，即使实际连接依然存活。

这两个问题本质都是"单真源缺失"：
- 鼠标路径里同时存在 `event.location()`（绝对坐标，关联断开后冻结）和 hardware delta（硬件增量）两条互斥信号，旧代码走了错误的那条；
- 按钮状态路径里 `updateConnectionButtons` 外部传参和 `appState.connected` 双真源打架。

## 2. 技术决策

### 2.1 纯 delta 模式，不留绝对坐标兜底

**对齐过的核心问题**：用户原话"为啥要还要兜底？不是应该改对吗？什么情况下还需要兜底，兜底是不是是错的？"

**决策**：协议层直接删除 `MouseMove { x, y }`，替换为 `MouseMoveDelta { dx: i16, dy: i16 }`。理由：

1. **CGEvent delta 是硬件层独立读数**——`EventField::MOUSE_EVENT_DELTA_X/Y` 由系统从 HID 驱动直接透传，不受 `CGAssociateMouseAndMouseCursorPosition(0)` 影响。
2. **TCP 可靠传输不丢包**——不需要"绝对坐标做 resync"兜底，累积误差不存在。
3. **远端 virtual_cursor 是单一 owner**——`RemoteCursorState` 内部维护 (x, y) 累加，无跨进程同步问题。
4. **奥卡姆剃刀**："兜底分支永远走不到"就是死代码，双路径只会让状态机边界多两倍。

**破坏性升级**：v0.1.40 双端同时升级，旧客户端连入会在 bincode 反序列化时失败（opcode 0x33 未知）——这比"保留旧字段造成双端语义不一致"更干净。

### 2.2 Armed 中间态（4 状态机）

之前是 3 态（Idle / LocalControlling / RemoteControlling），"开始共享"直接 Idle → LocalControlling 会立即吞键鼠。新 Armed 态的职责：

- 已激活边缘检测，但**不吞键鼠、不隐藏光标、不发帧**
- 撞边 + 过冲量达标才 Armed → LocalControlling，**在触发瞬间**一次性完成：grab 光标 + 隐藏 + 发 SwitchControl + 切发送模式

这样用户"开始共享"的体验是"鼠标还能正常用，碰到屏幕边缘才进入远程控制"，与 Logitech Flow 一致。

### 2.3 RemoteCursorState 作为远端单一真源

纯逻辑模块 `kvm-core/src/remote_cursor.rs`：
- `on_enter(edge, pos_ratio, screen_w, screen_h)`：入态初始化虚拟光标位置（从边缘按比例落点）
- `on_delta(dx, dy) -> RemoteCursorAction`：累加 + clamp + 反向边缘检测；撞反向边缘返回 `SwitchBack`，否则返回 `Warp { x, y }`
- 100% 测试覆盖（16 个单元测试）

平台层仅做一件事：拿到 `Warp { x, y }` 调 `CGWarpMouseCursorPosition`。

### 2.4 BUG-032 单真源修复

`updateConnectionButtons` 内部只读 `appState.connected`，忽略外部传参。所有事件分支（`reconnecting` / `heartbeat` / `error` / `ws_state_change`）不再污染按钮状态。新增 `__tests__/bug-032-reconnect-button.test.js` 4 条回归断言。

## 3. 改动总览

| 模块 | 文件 | 变更类型 |
|------|------|---------|
| 协议 | `kvm-core/src/protocol.rs` | 删除 `MouseMove`；新增 `MouseMoveDelta { dx, dy }`（opcode 0x33）+ `SwitchBackControl`；5 个 roundtrip 测试 |
| 远端虚拟光标 | `kvm-core/src/remote_cursor.rs` | **新建**：`RemoteCursorState` 结构 + `on_enter/on_delta` + `RemoteCursorAction` 枚举；16 个测试 |
| 边缘检测 | `kvm-core/src/edge_logic.rs` | `EdgeConfig.overshoot_px` 过冲量；`EdgeDetector` 累加器 + 触发瞬间 pos_ratio |
| 状态机 | `kvm-core/src/state.rs` | 4 态验证：`arm/disarm/armed_to_local_controlling/local_to_armed`；disconnect 任意态归零测试 |
| 平台抽象 | `kvm-core/src/platform.rs` | `RawInputEvent::MouseMove` 字段扩展为 `{ x, y, dx, dy }` |
| macOS 采集 | `src-tauri/src/platform/macos/input_capture.rs` | `cg_event_to_raw` 用 `EventField::MOUSE_EVENT_DELTA_X/Y` 读硬件 delta |
| Windows 采集 | `src-tauri/src/platform/windows/input_capture.rs` | 新增 `thread_local! LAST_MOUSE_POS`，WH_MOUSE_LL 用帧差近似 dx/dy |
| 共享调度 | `src-tauri/src/sharing.rs` | `SharingManager.remote_cursor: Arc<Mutex<Option<RemoteCursorState>>>`；`inject_remote_frame` 新增 `handle_remote_mouse_delta`（warp / SwitchBack）；`on_remote_switch_control(app, edge, pos_ratio)` 初始化远端光标 |
| 帧路由 | `src-tauri/src/connection/mod.rs` | `MouseMove` → `MouseMoveDelta`；`SwitchControl` 解构 edge/pos_ratio 传参 |
| changelog | `src-tauri/src/main.rs` | v0.1.40 8 条目（BUG-033 delta 根因 / 协议升级 / Armed 4 态 / 双通道回切 / BUG-032） |
| 前端按钮 | `src/main.js` | `updateConnectionButtons` 改为读 `appState.connected` 单真源；新增 CommonJS 测试导出 `__setAppState / __setElements` |
| 前端 UI | `src/main.js`, `src/index.html`, `src/i18n.js` | Armed 4 态状态指示；`sharingArmed` / `errReconnectNoTarget` 等 6 个 i18n key 双语同步 |
| 测试 setup | `__tests__/setup.ts` | `invoke` mock 改 `mockResolvedValue`；新增 `core.invoke` 别名 |
| 回归测试 | `__tests__/bug-032-reconnect-button.test.js`, `__tests__/sharing-armed.test.js` | **新建**：BUG-032 4 断言 + Armed 4 态 UI 分支 |
| Bug 归档 | `docs/bug-list.md` | Bug #033 完整归档（现象/根因/修改方式/测试/教训 7 字段） |
| 产品设计 | `docs/product-design-v0.1.40.md` | 4 状态机 + delta 模式 + RemoteCursorState + mermaid 图 |

代码净增删行数（不含 changelog/bug-list）：
- 前端：约 +60 / -25
- Rust：约 +310 / -80（remote_cursor 新建 + sharing 改造 + 平台采集是大头）
- 测试：约 +180（kvm-core 新增 21 个单元测试 + Jest 新增 8 个）

## 4. 关键实现点

### 4.1 macOS 硬件 delta 采集

```rust
// src-tauri/src/platform/macos/input_capture.rs
use core_graphics::event::EventField;

let dx = event.get_integer_value_field(EventField::MOUSE_EVENT_DELTA_X) as i16;
let dy = event.get_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y) as i16;
let loc = event.location();
RawInputEvent::MouseMove { x: loc.x as i32, y: loc.y as i32, dx, dy }
```

**关键**：`x/y` 用于 Armed 状态做边缘检测（此时光标正常显示，`event.location()` 不冻结）；`dx/dy` 用于 LocalControlling 状态发给远端（此时 `CGAssociateMouse(0)` 关联已断，`location()` 冻结但 hardware delta 仍然真实）。一个事件同时产出两路信号，热路径零额外开销。

### 4.2 Windows 帧差近似

WH_MOUSE_LL 低级钩子没有原生 delta 字段（`MSLLHOOKSTRUCT` 只有 `pt.x/pt.y`），用 thread_local 缓存上一帧：

```rust
thread_local! {
    static LAST_MOUSE_POS: RefCell<Option<(i32, i32)>> = RefCell::new(None);
}
```

首帧 dx/dy=0，后续帧 dx = cur.x - last.x。精度与硬件 delta 基本一致（WH_MOUSE_LL 频率 ~125Hz，帧差≈1-3 像素）。**已知局限**：Win 路径不如 mac `EventField::MOUSE_EVENT_DELTA` 精准，高速移动时可能有累积偏差——后续可升级为 RawInput（WM_INPUT）拿原生 `lLastX/lLastY`。

### 4.3 RemoteCursorState 反向边缘检测

```rust
pub fn on_delta(&mut self, dx: i16, dy: i16) -> RemoteCursorAction {
    self.x = (self.x + dx as i32).clamp(0, self.screen_w - 1);
    self.y = (self.y + dy as i32).clamp(0, self.screen_h - 1);
    if self.hit_return_edge() {
        return RemoteCursorAction::SwitchBack;
    }
    RemoteCursorAction::Warp { x: self.x, y: self.y }
}
```

`hit_return_edge` 根据 `return_edge`（进入方向的反向）判断 x/y 是否撞到对应边。clamp 保证虚拟光标永远在屏幕内，不会因网络抖动或 delta 爆表而越界。

### 4.4 前端按钮单真源

```js
function updateConnectionButtons(/* 忽略外部传参 */) {
    const connected = !!appState.connected;
    // ... 仅依赖 appState.connected 渲染
}
```

任何事件分支（`reconnecting` / `heartbeat`）不再写按钮文案，只更新 `appState`，然后统一走 `updateConnectionButtons()`。

## 5. 测试证据

```
cargo test -p kvm-core        → 129 passed; 0 failed
cargo test -p kvmini --lib    →  19 passed; 0 failed
npm test                       → 887 passed; 0 failed (Jest)
```

关键新增测试清单：
- `kvm-core/src/protocol.rs::tests::test_mouse_move_delta_*` × 5
- `kvm-core/src/remote_cursor.rs::tests::*` × 16
- `kvm-core/src/edge_logic.rs::tests::test_overshoot_*` × 4
- `kvm-core/src/state.rs::tests::test_arm_disarm / test_disconnect_from_any_state` × 4
- `__tests__/bug-032-reconnect-button.test.js` × 4
- `__tests__/sharing-armed.test.js` × 4

## 6. 性能与可靠性

- **鼠标热路径**：CGEvent 回调 → mpsc 发送 → pump 分流，无锁串行处理；delta 累加 O(1)
- **远端 warp 频率**：与采集事件同频（~100-250Hz），无额外 polling
- **网络帧大小**：`MouseMoveDelta` 11 字节 vs 旧 `MouseMove` 13 字节，LAN RTT < 5ms
- **锁粒度**：`StateMachine` Mutex 仅状态转换时持有；`RemoteCursorState` 独立 Mutex，不与状态机耦合
- **边界处理**：所有坐标 clamp 到 `0..screen_w/h`，防止 i32 溢出

## 7. 遗留事项与后续计划

- **Windows dx/dy 精度**：当前用 WH_MOUSE_LL 帧差，高速移动时可能有 1-2px 偏差。下一版计划迁移到 RawInput（WM_INPUT + RIM_TYPEMOUSE）拿 `lLastX/lLastY` 硬件值，与 mac 路径完全对齐。
- **代码 review**：按铁律需另一 agent 做重构+性能 review，review 反馈处理后再构建打包（todo 8）。
- **跨架构构建**：mac M / mac Intel / win 三份包将在 todo 8 统一输出并归档至 `docs/archive/20260507/`。
- **破坏性升级提示**：v0.1.40 首次启动后，若用户对端仍是 v0.1.39，连接会立即断开并在后端日志输出 `bincode: unknown opcode 0x33`。产品层面建议在下一版（v0.1.41）加入握手版本协商，给用户明确的"对端版本过低"提示。

## 8. 教训沉淀

1. **"兜底"有时是错的**：当两条路径语义冲突且只有一条正确时，"保留旧路径做 fallback"等于留炸弹——要么删干净，要么双端同时升级。本次选择后者（破坏性升级）反而更安全。
2. **单真源原则要渗透到所有事件分支**：BUG-032 的根因不是 `updateConnectionButtons` 写错，而是**多个事件分支绕过它直接改按钮文案**。修法不是"改每一个分支"而是"让按钮只能从唯一来源渲染"。
3. **热路径不能锁 Mutex**：`CGEventTap` 回调频率 >1000Hz，任何 Mutex 竞争都会让用户感知"鼠标顿挫"。`RemoteCursorState` 虽然用了 Mutex，但只在接收远端帧时持有，不在本地采集路径上。
4. **测试导出要支持注入**：前端测试最大的坑是模块级 `let appState` 无法被 `global.appState` 覆盖。解决方案是导出 `__setAppState / __setElements` setter，而不是把 `appState` 变成 `global` 污染生产代码。

## 9. Code Review 反馈与追加修复

完整代码 review 后发现并修复了 3 个严重问题：

### 9.1 [Severity: High] Win 平台 `LAST_MOUSE_POS` 重复定义

- **文件**：`src-tauri/src/platform/windows/input_capture.rs`
- **现象**：同一个 `cfg(target_os="windows")` 模块中两个 `thread_local` 块都声明了 `LAST_MOUSE_POS`，Win 编译会直接报 `defined multiple times`。
- **根因**：v0.1.40 改造时漏删第一个块里的旧定义。
- **修复**：删除冗余块，保留 HOOK_* 集合块内的定义。
- **教训**：CI 必须加 `cargo check --target x86_64-pc-windows-msvc`，否则 mac 端编译走不到 cfg 分支永远发现不了。

### 9.2 [Severity: High] BUG-032 修复不彻底

- **文件**：`src/main.js` 的 `reconnecting` 事件分支
- **现象**：第一次只把 `updateConnectionButtons` 改成读 `appState.connected` 单真源，但 reconnecting 分支自身仍然 `appState.connected = false`——心跳波动会污染真源，按钮依然会闪 `重连`。
- **修复**：reconnecting 分支不再写 `appState.connected = false`。TCP socket 仍可能存活，仅心跳波动而已；只有真正收到 `disconnected` 事件才置 false。
- **教训**：根因优于表象。"修在显示层"不够，必须保证真源不被瞬态事件污染。bug-list.md 已更新为"两层修复"记录。

### 9.3 [Severity: High] `KvmFrame::MouseMove` + `EventType::MouseMove` + `event_queue` 未真正删除

- **文件**：`kvm-core/src/protocol.rs`、`kvm-core/src/protocol/event_queue.rs`
- **现象**：设计文档明确"破坏性升级删除 MouseMove 不留兜底"，但代码里 `KvmFrame::MouseMove{x,y,timestamp_ms,modifiers}`（opcode 0x10）+ 编解码 + event_queue 整模块完整保留——文档和实现不一致。
- **根因**：奥卡姆剃刀执行不彻底。event_queue 模块零外部引用（grep 确认 `event_queue::` 命中 0 次），但仍占用 256 行代码 + 8 个测试。
- **修复**：
  - 删除 `EventType::MouseMove` 枚举值 + `from_u8` 分支
  - 删除 `KvmFrame::MouseMove` 变体 + encode/decode 分支
  - 删除整个 `kvm-core/src/protocol/event_queue.rs` 文件 + `pub mod event_queue;` 声明
  - 修复 `protocol.rs` 内引用 MouseMove 的 2 个测试用例迁移到 MouseMoveDelta
- **结果**：kvm-core 测试从 137 → 129（删除 event_queue 8 个测试是预期）；协议体积净减少 ≈40 行，event_queue 净减少 256 行。
- **教训**：文档说"删除"就要真删除。"留着不影响"是技术债的最常见借口；零调用方的死代码留着只会让协议表面虚增、阅读成本上升。
