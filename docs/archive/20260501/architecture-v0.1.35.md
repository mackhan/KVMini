# KVMini v0.1.35 架构图

**日期**：2026-05-01  
**范围**：整体架构 + 本次 BUG-018/019/020 修复后的新增链路

---

## 一、整体架构（v0.1.35）

```mermaid
graph TB
    subgraph 前端 [前端 Web UI]
        UI[src/index.html<br/>按钮 / 状态栏 / 设备列表]
        MainJS[src/main.js<br/>事件绑定 + invoke 调用]
        I18N[src/i18n.js<br/>zh + en 字典 + applyI18n]
    end

    subgraph Tauri [Tauri 命令层]
        CMD_TOG[toggle_sharing<br/>get_control_state]
        CMD_CONN[connect_to_peer<br/>start_listening<br/>disconnect_peer]
        CMD_DISC[discover_devices<br/>diagnose_network]
        CMD_CONF[save_config<br/>get_app_version]
    end

    subgraph Sharing [SharingManager<br/>Tauri 托管单例]
        SM[StateMachine<br/>Idle / Local / Remote]
        CAP[InputCapture<br/>Mac=CGEventTap 子线程]
        EXE[InputExecutor<br/>Mac=CGEventPost]
        CHAN[tokio mpsc<br/>RawInputEvent 通道]
        PUMP[pump task<br/>raw→KvmFrame→send_frame]
    end

    subgraph Conn [KvmConnectionManager]
        INNER[kvm-core ConnectionManager]
        LOOP[start_event_loop<br/>FrameReceived 分发]
        PING[Ping task 每 3s]
    end

    subgraph Core [kvm-core]
        NET[network.rs<br/>TCP + TLS + Handshake]
        PROTO[protocol.rs<br/>KvmFrame 枚举]
        STATE[state.rs<br/>StateMachine + StateEvent]
        PLAT[platform trait<br/>InputCapture/Executor]
        ERR[error.rs]
    end

    subgraph Discovery [网络发现]
        LIF[list_lan_ipv4_ifaces<br/>VPN 过滤]
        DISCV2[discover_devices_v2<br/>多接口 UDP 广播]
        SCAN[scan_subnet_multi<br/>多网段并发]
    end

    UI --> MainJS
    MainJS --> I18N
    MainJS -->|click toggle_sharing| CMD_TOG
    MainJS -->|click connect| CMD_CONN
    MainJS -->|auto/click scan| CMD_DISC

    CMD_TOG --> SM
    CMD_CONN --> INNER
    CMD_DISC --> LIF
    LIF --> DISCV2
    LIF --> SCAN

    SM -->|StartCapture| CAP
    CAP -->|回调| CHAN
    CHAN --> PUMP
    PUMP -->|KvmFrame| INNER
    INNER -->|TCP 发送| NET

    NET -->|TCP 接收| LOOP
    LOOP -->|FrameReceived InputEvent| EXE
    LOOP -->|FrameReceived SwitchControl| SM
    LOOP -->|Disconnected| SM
    LOOP -->|emit connection-status| MainJS

    SM -->|emit control-state-changed| MainJS
```

---

## 二、BUG-018（TLS 握手）新链路

```mermaid
sequenceDiagram
    participant A as 机器 A (connect_to)
    participant B as 机器 B (handle_stream)
    
    Note over A,B: 双方 local_info.tls_enabled 均为 true
    
    A->>A: local_handshake_info()<br/>tls_enabled=true
    A->>B: TCP connect + Handshake 帧<br/>screen_info=HandshakePayload{desktop,tls_enabled=true}
    
    B->>B: local_handshake_info()<br/>tls_enabled=true
    B->>B: accept_loop 解析 peer 握手<br/>peer.tls_enabled=true
    
    B->>A: HandshakeAck<br/>screen_info=HandshakePayload{desktop,tls_enabled=true}
    
    A->>A: parse_payload(ack.screen_info)<br/>→ (desktop, tls_enabled=true)
    A->>A: 比较 local_tls(true) == remote_tls(true) ✅<br/>不 emit tls_mismatch
    
    Note over A,B: ✅ 修复前：screen_info 只传 desktop，<br/>A 端永远 remote_tls=false → 误报
```

---

## 三、BUG-019（开始共享）完整链路

```mermaid
sequenceDiagram
    participant UI as 前端按钮
    participant CMD as toggle_sharing 命令
    participant SM as SharingManager
    participant CAP as CGEventTap 子线程
    participant PUMP as pump task
    participant NET as ConnectionManager
    participant PEER as 对端
    participant EXE as 对端 InputExecutor
    
    UI->>CMD: click → invoke('toggle_sharing')
    CMD->>SM: toggle(&app)
    SM->>SM: state_machine.toggle_control<br/>Idle→LocalControlling
    SM->>UI: emit control-state-changed {from:Idle, to:Local}
    UI->>UI: 按钮 data-i18n 切 stop_sharing<br/>applyI18n()
    
    SM->>CAP: start_capture(callback)<br/>专用子线程 CFRunLoop::run_current()
    SM->>PUMP: spawn pump task 消费 mpsc
    
    Note over CAP,PUMP: 用户移动鼠标
    CAP->>CAP: CGEventTap 回调<br/>cg_event_to_raw
    CAP->>PUMP: mpsc.send(RawInputEvent)
    PUMP->>PUMP: raw_to_frame(KvmFrame::MouseMove)
    PUMP->>NET: send_frame(frame)
    NET->>PEER: TCP 发送 bincode(KvmFrame)
    
    PEER->>PEER: start_event_loop 收到<br/>NetworkEvent::FrameReceived(MouseMove)
    PEER->>EXE: sm.inject_remote_frame(frame)
    EXE->>EXE: move_mouse(x, y)<br/>→ CGEventPost 真实操作系统
    
    Note over UI,EXE: 用户再次点击按钮
    UI->>CMD: click → invoke('toggle_sharing')
    CMD->>SM: toggle
    SM->>SM: Local→Idle
    SM->>CAP: stop_capture<br/>CFRunLoop::stop(run_loop)
    SM->>UI: emit control-state-changed {to:Idle}
    UI->>UI: 按钮恢复 start_sharing
```

---

## 四、BUG-020（VPN 扫描）链路

```mermaid
flowchart TB
    START[用户点击扫描]
    IF[list_lan_ipv4_ifaces]
    FILTER{过滤 VPN/loopback}
    
    START --> IF
    IF --> FILTER
    FILTER -->|macOS 黑名单: utun*, ppp*, tun*, tap*, vboxnet| RES[物理网卡列表]
    FILTER -->|Windows 黑名单: Loopback, PPP, TAP, WireGuard, OpenVPN, Tailscale| RES
    
    RES --> BC[多接口并发 UDP 广播]
    RES --> SCAN[多网段并发 TCP 扫描]
    
    BC -->|每个 iface: bind<ip>:0 + send 255.255.255.255 + send iface_broadcast| RESP[收集响应 1.5s]
    SCAN -->|每个 iface: 扫描 /24 子网| TCP[TCP 握手收集]
    
    RESP --> DEDUP[HashSet 按 IP 去重]
    TCP --> DEDUP
    
    DEDUP --> EMPTY{为空?}
    EMPTY -->|否| LIST[返回设备列表]
    EMPTY -->|是| DIAG[diagnose_network]
    
    DIAG --> VPN_TIP{检测到 utun*?}
    VPN_TIP -->|是| TIP1[追加 vpn_interference_tip<br/>建议临时断开 VPN]
    VPN_TIP -->|否| TIP2[跳过 VPN 提示]
    
    TIP1 --> FW[追加 firewall_block_tip<br/>sudo socketfilterfw --add]
    TIP2 --> FW
    
    FW --> UI_SHOW[前端 toast 展示双语提示]
```

---

## 五、CGEventTap 线程模型（关键细节）

```mermaid
sequenceDiagram
    participant MAIN as Tauri 主线程<br/>(NSApplication RunLoop)
    participant SM as SharingManager
    participant SUB as 子线程 "kvm-cgeventtap"<br/>(专用 RunLoop)
    participant TAP as CGEventTap
    
    SM->>SUB: std::thread::Builder::spawn
    SUB->>TAP: CGEventTap::new(mask, callback)
    TAP-->>SUB: tap 对象
    SUB->>SUB: tap.mach_port.create_runloop_source
    SUB->>SUB: CFRunLoop::get_current()<br/>⚠️ CRITICAL: 不是 get_main
    SUB->>SUB: run_loop.add_source(loop_source, kCFRunLoopDefaultMode)
    SUB->>SUB: tap.enable()
    SUB->>SUB: std::mem::forget(tap)<br/>⚠️ 防止 CFMachPort 释放
    SUB->>SM: started_tx.send(Ok) 通知就绪
    SM-->>MAIN: start_capture 返回 Ok
    
    SUB->>SUB: *run_loop_slot = Some(RunLoopHandle(rl))
    SUB->>SUB: CFRunLoop::run_current()<br/>⏳ 阻塞等事件
    
    Note over SUB: 用户键鼠事件触发
    TAP->>SUB: CGEventTap 回调
    SUB->>SM: callback.lock().send(RawInputEvent)
    
    Note over MAIN,SUB: stop 流程
    MAIN->>SM: stop_capture()
    SM->>SUB: CFRunLoop::stop(&handle.0)<br/>唤醒 run_current()
    SUB->>SUB: 退出 CFRunLoopRun
    SUB->>SUB: 线程自然结束
```

---

## 六、关键文件对应

| 模块 | 文件 | 本次改动 |
|---|---|---|
| 握手协议 | `kvm-core/src/network.rs` | 新增 HandshakePayload + parse_payload 双层回退 |
| 状态机 | `kvm-core/src/state.rs` | 无改动（验证） |
| 平台 trait | `kvm-core/src/platform/mod.rs` | 无改动（验证） |
| Mac 采集 | `src-tauri/src/platform/macos/input_capture.rs` | **重写 start_capture：专用子线程 + CFRunLoop** |
| Mac 注入 | `src-tauri/src/platform/macos/input_inject.rs` | 无改动（验证） |
| Win 采集/注入 | `src-tauri/src/platform/windows/*.rs` | 无改动（验证） |
| 共享管理 | `src-tauri/src/sharing.rs` | **全新模块** |
| 连接事件 | `src-tauri/src/connection/mod.rs` | **新增 FrameReceived/Disconnected 分发** |
| 网络扫描 | `src-tauri/src/network_scan.rs` | **新增 list_lan_ipv4_ifaces / v2 扫描** |
| 命令注册 | `src-tauri/src/main.rs` | **注册 SharingManager + toggle_sharing** |
| 前端按钮 | `src/main.js` | **新增 handleStartSharing / listen control-state-changed** |
| 前端 i18n | `src/i18n.js` | **新增 10+ key（zh+en）** |
| HTML 按钮 | `src/index.html` | data-i18n 保持 start-sharing-btn |
| 版本管理 | `scripts/version-update.js` | **抽出 decideBumpAction + BUMP_VERSION=0** |

---

## 七、三端隔离状态

| 能力 | macOS (M/Intel) | Windows |
|---|---|---|
| InputCapture 实现 | `MacInputCapture`（CGEventTap 子线程） | `WinInputCapture`（低级钩子） |
| InputExecutor 实现 | `MacInputExecutor`（CGEventPost） | `WinInputExecutor`（SendInput） |
| VPN 过滤黑名单 | `utun*` `ppp*` `tun*` `tap*` `vboxnet` | `Loopback` `PPP` `TAP` `WireGuard` `OpenVPN` `Tailscale` |
| 权限提示 | 辅助功能 + 输入监控（TODO BUG-021） | UAC + 防火墙 |
| UI 文案 | zh + en（data-i18n） | zh + en（data-i18n） |

**铁律**：Mac 和 Win 功能必须保持 1:1 一致；隔离只在底层实现，上层 trait / 前端 UI / 命令签名完全相同。
