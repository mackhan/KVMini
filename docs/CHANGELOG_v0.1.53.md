# v0.1.53 修复归档

日期：2026-05-09

## 修复问题

### Bug 1：拖拽无中间过程（根本修复）

**症状**：Win 控制 Mac 时，在被控 Mac 上拖拽窗口无法移动（只有 Down/Up，窗口没有跟随）。

**根因**：`input_inject.rs` 的 `inject_mouse_move` 始终注入 `CGEventType::MouseMoved`，但 macOS WindowServer 在鼠标按键按下期间需要收到 `LeftMouseDragged` 事件才会移动被拖窗口。`MouseMoved` 在按键按下时被系统忽略。

**修复**：通过 `CGEventSourceButtonState(HIDSystemState, LeftButton)` FFI 查询左键当前按下状态，按下时注入 `LeftMouseDragged`，否则注入 `MouseMoved`。

**文件**：`src-tauri/src/platform/macos/input_inject.rs`

---

### Bug 2：滚轮计算错误（负数 max 逻辑）

**症状**：Mac 被控端向上滚动（dy 为负）时，`lines_y` 计算错误。

**根因**：`(dy / 120).max(1)` 对负数 `dy` 结果是 `max(1, -1)` = `1`，方向翻转变成向下滚。

**修复**：改为 `(dy.abs() / 120).max(1) * dy.signum()`，先对绝对值计算行数再乘符号。

**文件**：`src-tauri/src/platform/macos/input_inject.rs`

---

### Bug 3：远端（被控端）按钮显示错误

**症状**：远端被控后，UI 按钮仍显示"开始共享"，没有变为"被远端控制中"。

**根因**：`updateSharingButtonForState` 函数没有 `RemoteControlling` 分支，`else` 路径显示"开始共享"。

**修复**：加入 `RemoteControlling` 分支，显示"🖱 被远端控制中"并禁用按钮（防止被控时误操作）。`control-state-changed` 监听器也补充了 `RemoteControlling` 的 toast 提示。

**文件**：`src/main.js`、`src/i18n.js`（新增 `btnRemoteControlled` key）

---

### Bug 4：断开连接后仍可控制（根本修复）

**症状**：点击"断开连接"按钮后，仍然可以继续控制远端。

**根因**：`handleDisconnect` 调用了 `disconnect_from_mac`、`disconnect_tcp`、`disconnect_devices` 三个后端不存在的命令，调用均 `.catch(() => {})` 静默吞掉了错误，实际上**从未断开**。真正的断开命令是 `disconnect_peer`。

**修复**：`handleDisconnect` 改为调用唯一正确的 `disconnect_peer` 命令。`disconnect_peer` 调用链：`ConnectionManager.disconnect()` → `NetworkEvent::ConnectionStateChanged(Disconnected)` → `on_disconnected()` → 状态机归零、解除键鼠屏蔽、停止 capture。

**文件**：`src/main.js`

---

## 文件变更列表

| 文件 | 改动类型 | 说明 |
|------|----------|------|
| `src-tauri/src/platform/macos/input_inject.rs` | 修复 | inject_mouse_move 感知左键状态发 LeftMouseDragged；滚轮 lines 计算修复 |
| `src/main.js` | 修复 | handleDisconnect 调用 disconnect_peer；updateSharingButtonForState 加 RemoteControlling 分支 |
| `src/i18n.js` | 新增 | btnRemoteControlled zh/en 翻译 |

## 测试

60 套件 / 966 用例全部通过（0 失败）
