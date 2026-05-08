# KVMini v0.1.40 架构图

> 对应版本：v0.1.40（2026-05-07） · 鼠标控制竞品级重构 + Armed 4 状态机 + BUG-032/033 修复

## 1. 顶层分层架构

```mermaid
graph TB
    subgraph UI["前端 UI 层 (src/)"]
        HTML[index.html]
        MAIN[main.js<br/>单真源按钮渲染]
        I18N[i18n.js<br/>zh/en 双语]
        CONN[connection.js]
    end

    subgraph TAURI["Tauri 命令层 (src-tauri/)"]
        CMD[main.rs<br/>invoke_handler + changelog]
        SHARE[sharing.rs<br/>SharingManager + RemoteCursorMgr]
        CONNMOD[connection/mod.rs<br/>帧路由]
    end

    subgraph PLATFORM["平台抽象层 (src-tauri/platform/)"]
        MAC[macos/<br/>CGEventTap + hardware delta]
        WIN[windows/<br/>WH_MOUSE_LL + 帧差]
    end

    subgraph CORE["纯逻辑核心 (kvm-core/)"]
        STATE[state.rs<br/>4 状态机]
        EDGE[edge_logic.rs<br/>EdgeDetector + overshoot]
        PROTO[protocol.rs<br/>KvmFrame + MouseMoveDelta]
        REMOTE[remote_cursor.rs<br/>RemoteCursorState]
        CURSOR[cursor_mapping.rs<br/>边缘→落点]
        CFG[config.rs<br/>AppConfig]
    end

    UI -->|invoke| TAURI
    TAURI -->|调用| PLATFORM
    TAURI -->|使用| CORE
    PLATFORM -->|RawInputEvent| SHARE
    SHARE -->|KvmFrame| CONNMOD
    CONNMOD -.TCP.-> CONNMOD
    CONNMOD -->|远端帧| SHARE
    SHARE -->|逻辑判断| CORE
```

## 2. 4 状态机迁移图

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Armed: arm()<br/>开始共享
    Armed --> Idle: disarm()/disconnect
    Armed --> LocalControlling: edge_trigger<br/>overshoot达标
    LocalControlling --> Armed: Ctrl+Alt+S<br/>local_to_armed
    LocalControlling --> Armed: recv SwitchBackControl
    Idle --> RemoteControlling: recv SwitchControl
    Armed --> RemoteControlling: recv SwitchControl
    RemoteControlling --> Idle: recv SwitchBackControl
    RemoteControlling --> Idle: disconnect
    LocalControlling --> Idle: disconnect

    note right of Idle
        EventTap 透传
        光标正常
        不发帧
    end note
    note right of Armed
        EventTap 透传
        光标正常
        不发帧
        边缘检测开
    end note
    note right of LocalControlling
        EventTap 吞
        光标 grab+隐藏
        发 MouseMoveDelta
    end note
    note right of RemoteControlling
        EventTap 吞
        光标正常
        接收方 warp
    end note
```

## 3. 鼠标数据流（重构后）

```mermaid
flowchart LR
    A[CGEventTap callback] -->|event.location<br/>+ EventField::DELTA_X/Y| B[RawInputEvent::MouseMove<br/>x,y,dx,dy]
    B --> C[mpsc tx]
    C --> D[sharing.rs pump]

    D -->|state=Armed| E[EdgeDetector.detect x,y<br/>overshoot_accumulator]
    D -->|state=LocalControlling| F[KvmFrame::MouseMoveDelta]

    E -->|未触发| EE[丢弃]
    E -->|达标触发| L[armed_to_local_controlling<br/>edge, pos_ratio]
    L --> M[grab 光标 + 隐藏<br/>+ 发 SwitchControl]

    F --> G[connection.send]
    G -.TCP.-> H[远端接收]
    H --> I[RemoteCursorState.on_delta]
    I -->|hit return edge| J[发 SwitchBackControl]
    I -->|else| K[CGWarpMouseCursorPosition]
```

## 4. 协议帧（v0.1.40）

```mermaid
classDiagram
    class KvmFrame {
        <<enum>>
    }

    class SwitchControl {
        edge: ScreenEdge
        pos_ratio: u16
    }
    class SwitchControlAck
    class SwitchBackControl {
        NEW v0.1.40
    }
    class MouseMoveDelta {
        dx: i16
        dy: i16
        NEW v0.1.40 opcode 0x33
    }
    class MouseButton {
        button: u8
        pressed: bool
    }
    class MouseWheel {
        dx: i16
        dy: i16
    }
    class KeyEvent {
        keycode: u32
        pressed: bool
    }
    class ClipboardSync {
        text: String
    }
    class Heartbeat

    KvmFrame <|-- SwitchControl
    KvmFrame <|-- SwitchControlAck
    KvmFrame <|-- SwitchBackControl
    KvmFrame <|-- MouseMoveDelta
    KvmFrame <|-- MouseButton
    KvmFrame <|-- MouseWheel
    KvmFrame <|-- KeyEvent
    KvmFrame <|-- ClipboardSync
    KvmFrame <|-- Heartbeat

    note for KvmFrame "v0.1.40 删除 MouseMove{x,y}<br/>破坏性升级，双端同时 v0.1.40"
```

## 5. RemoteCursorState 状态转移

```mermaid
stateDiagram-v2
    [*] --> NotInitialized
    NotInitialized --> Active: on_enter(edge,pos_ratio,w,h)
    Active --> Active: on_delta → Warp(x,y)
    Active --> [*]: on_delta → SwitchBack<br/>(hit return_edge)
    Active --> NotInitialized: disconnect

    note right of Active
        x += dx
        y += dy
        clamp 0..w/h
        检测 return_edge
    end note
```

## 6. 状态-行为对照表

| 状态 | CGEventTap 返回 | 本机光标 | 网络发送 | 接收处理 | 边缘检测 |
|------|----------------|---------|---------|---------|---------|
| Idle | Some(透传) | 正常 | 不发 | SwitchControl→Remote | 关 |
| Armed | Some(透传) | 正常 | 不发 | SwitchControl→Remote | 本机(x,y)+overshoot |
| LocalControlling | None(吞) | grab+隐藏 | MouseMoveDelta | SwitchBack→Armed | 不检测 |
| RemoteControlling | None(吞) | 正常 | 不发 | MouseMoveDelta→virtual cursor warp | 远端虚拟光标反向边缘 |

## 7. 跨平台采集差异

| 能力 | macOS | Windows |
|------|-------|---------|
| Hook 类型 | CGEventTap (Default mode) | WH_MOUSE_LL Low-Level Hook |
| dx/dy 来源 | `EventField::MOUSE_EVENT_DELTA_X/Y`（硬件原生） | 帧差近似（`thread_local LAST_MOUSE_POS`） |
| 精度 | 硬件透传，无累积误差 | 1-2px 偏差（后续可升级 RawInput） |
| 吞事件方式 | 回调返回 None | `return 1;` from hook proc |
| 光标隐藏 | `CGDisplayHideCursor` | `ShowCursor(FALSE)` |
| 光标关联 | `CGAssociateMouseAndMouseCursorPosition(0)` | `ClipCursor` |
| 远端 warp | `CGWarpMouseCursorPosition` | `SetCursorPos` |

## 8. BUG-032 / BUG-033 根因路径

```mermaid
flowchart TB
    subgraph BUG033["BUG-033: 远端鼠标不动"]
        B33A[LocalControlling: CGAssociateMouse 0 关联断开]
        B33B[event.location 被系统冻结<br/>永远返回同一个点]
        B33C[旧代码发 MouseMove x,y<br/>远端 CGWarp 到同一点]
        B33D[视觉: 远端鼠标不动]
        B33A --> B33B --> B33C --> B33D
        B33E[修复: 用 EventField DELTA_X/Y<br/>换 MouseMoveDelta 协议]
    end

    subgraph BUG032["BUG-032: 按钮文案污染"]
        B32A[connect 成功 → appState.connected=true]
        B32B[reconnecting 事件触发]
        B32C[旧代码: updateConnectionButtons false]
        B32D[按钮显示 '重连' 即使连接正常]
        B32A --> B32B --> B32C --> B32D
        B32E[修复: 内部只读 appState.connected<br/>忽略外部传参]
    end
```
