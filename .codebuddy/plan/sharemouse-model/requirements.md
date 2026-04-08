# 需求文档：KVMini —— 仿 ShareMouse 双键鼠共享模型

## 引言

KVMini 是一款运行在局域网内的键鼠共享软件，目标是让 Mac 和 Windows 两台电脑各自保留独立的键盘鼠标，通过软件实现"控制权自动流转"。

**核心设计理念（参考 ShareMouse）：**
- **两端完全对等**，不存在固定的"主控端"或"被控端"角色
- **没有角色切换按钮**，控制权完全由鼠标边缘行为或快捷键自动触发
- **连接建立后始终保持**，不因控制权变化而断开重连
- **任意时刻只有一种状态**：要么各自独立操作，要么某一方的键鼠正在控制对方

---

## 重构说明

本次为**基于现有代码的重构**，不是从零开始。以下是现有代码资产与重构策略：

### 现有代码可复用部分

| 现有模块 | 位置 | 重构策略 |
|---------|------|---------|
| `protocol.rs` | 两端各有一份（几乎相同） | 合并到 kvm-core，去除重复 |
| `connection/error.rs` | 两端各有一份 | 合并到 kvm-core，统一错误类型 |
| `connection/manager.rs` | 两端各有一份 | 提取公共逻辑到 kvm-core network |
| `control/edge.rs` | 两端各有一份 | 合并到 kvm-core edge_logic |
| `control/hotkey.rs` | 两端各有一份 | 接口定义到 kvm-core，实现留在各端 |
| `monitor.rs` | src-tauri 中 | 接口定义到 kvm-core ScreenProvider trait，实现留在 kvm-win |
| `capturer.rs` | src-tauri 中 | 接口定义到 kvm-core InputCapture trait，实现留在 kvm-win |
| `injector.rs` | src-tauri 中 | 接口定义到 kvm-core InputExecutor trait，实现留在 kvm-win |
| `input_executor.rs` | kvm-agent 中 | 实现留在 kvm-mac，实现 InputExecutor trait |
| `clipboard.rs` | src-tauri 中 | 协议逻辑到 kvm-core，平台实现留在各端 |
| `tls.rs` | src-tauri 中 | 移到 kvm-core auth 模块 |
| `keycode_map.rs` | src-tauri 中 | 移到 kvm-core protocol 模块 |
| `role_manager.rs` | 两端各有一份 | **废弃**，用 kvm-core state 状态机替代 |
| `server.rs` | 两端各有一份 | 合并到 kvm-core network |
| `discovery.rs` / `network.rs` | 两端各有一份 | UDP 发现逻辑合并到 kvm-core network |
| `tray.rs` | 两端各有一份 | 保留在各端，各自独立 |

### 重构原则
1. **先抽后改**：先将公共代码提取到 kvm-core，再修改各端引用
2. **保持可编译**：每一步重构后两端都应能编译通过
3. **废弃 role_manager**：不再有"主控端/被控端"固定角色，用状态机替代

---

## 模块架构划分（三层架构 + Cargo Workspace）

为最大程度减少重复代码、降低耦合，采用 **三层架构**，通过 Rust Cargo Workspace 组织为多个独立 crate：

### Cargo Workspace 结构

```
KVMini/
├── Cargo.toml                  # workspace 根配置
├── kvm-core/                   # 公共核心库 crate（纯逻辑，零平台依赖）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── network.rs          # 网络连接、握手、心跳、重连、UDP 发现
│       ├── state.rs            # 控制权状态机
│       ├── protocol.rs         # 事件帧定义、序列化、键码映射
│       ├── edge_logic.rs       # 边缘触发判定（纯坐标计算）
│       ├── cursor_mapping.rs   # 光标位置映射（跨屏坐标转换）
│       ├── clipboard.rs        # 剪贴板共享协议逻辑
│       ├── auth.rs             # 密码验证、TLS 配置
│       ├── config.rs           # 配置数据结构定义与序列化
│       ├── error.rs            # 统一错误类型定义
│       ├── log.rs              # 日志接口定义
│       └── platform.rs         # 平台适配 trait 定义
│
├── kvm-mac/                    # Mac 端应用 crate（含 Mac 平台适配层）
│   ├── Cargo.toml              # 依赖 kvm-core
│   └── src/
│       ├── main.rs             # Mac 端启动入口
│       ├── platform/           # Mac 平台适配层（实现 kvm-core 的 trait）
│       │   ├── mod.rs
│       │   ├── screen.rs       # CoreGraphics 屏幕枚举
│       │   ├── input_capture.rs# CGEventTap 键鼠采集
│       │   ├── input_inject.rs # CGEvent 键鼠注入
│       │   ├── input_block.rs  # CGEventTap 键鼠屏蔽
│       │   ├── hotkey.rs       # NSEvent 全局快捷键
│       │   ├── clipboard.rs    # NSPasteboard 剪贴板读写
│       │   ├── cursor.rs       # NSCursor 光标显示/隐藏
│       │   └── permission.rs   # macOS 辅助功能权限检测与引导
│       ├── tray.rs             # Mac 系统托盘 UI
│       ├── autostart.rs        # Mac 开机自启动（LaunchAgent）
│       └── config_store.rs     # Mac 端配置持久化（~/Library/...）
│
├── kvm-win/                    # Windows 端应用 crate（含 Win 平台适配层）
│   ├── Cargo.toml              # 依赖 kvm-core、tauri
│   └── src/
│       ├── main.rs             # Windows 端启动入口（Tauri）
│       ├── platform/           # Win 平台适配层（实现 kvm-core 的 trait）
│       │   ├── mod.rs
│       │   ├── screen.rs       # Win32 EnumDisplayMonitors 屏幕枚举
│       │   ├── input_capture.rs# SetWindowsHookEx 键鼠采集
│       │   ├── input_inject.rs # SendInput 键鼠注入
│       │   ├── input_block.rs  # Hook + BlockInput 键鼠屏蔽
│       │   ├── hotkey.rs       # RegisterHotKey 全局快捷键
│       │   ├── clipboard.rs    # Win32 Clipboard API 剪贴板读写
│       │   └── cursor.rs       # ShowCursor 光标显示/隐藏
│       ├── tray.rs             # Win 系统托盘
│       ├── autostart.rs        # Win 开机自启动（注册表）
│       ├── ui/                 # Tauri 前端 UI
│       └── config_store.rs     # Win 端配置持久化（AppData）
│
└── src/                        # Tauri 前端资源（HTML/CSS/JS）
    └── ...
```

### 架构分层图

```
┌──────────────────────────────────────────────────────────┐
│                    端应用层（App Layer）                    │
│  ┌────────────────────┐      ┌─────────────────────────┐ │
│  │  kvm-mac            │      │  kvm-win                │ │
│  │  · main.rs 启动入口 │      │  · main.rs 启动入口     │ │
│  │  · tray.rs 托盘 UI  │      │  · tray.rs 托盘         │ │
│  │  · autostart.rs     │      │  · ui/ Tauri 桌面 UI    │ │
│  │  · config_store.rs  │      │  · autostart.rs         │ │
│  │                     │      │  · config_store.rs      │ │
│  │  ┌────────────────┐ │      │  ┌───────────────────┐  │ │
│  │  │ platform/      │ │      │  │ platform/         │  │ │
│  │  │ (Mac适配层)    │ │      │  │ (Win适配层)       │  │ │
│  │  │ 实现 trait     │ │      │  │ 实现 trait        │  │ │
│  │  └───────┬────────┘ │      │  └────────┬──────────┘  │ │
│  └──────────┼──────────┘      └───────────┼─────────────┘ │
│             │                             │               │
├─────────────┼─────────────────────────────┼───────────────┤
│             └──────────────┬──────────────┘               │
│                  公共核心库（Core Library）                 │
│                    kvm-core crate                         │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  · network.rs        — TCP 连接、握手、心跳、重连     │ │
│  │  · state.rs          — 控制权状态机维护与同步         │ │
│  │  · protocol.rs       — 事件帧定义、序列化、键码映射   │ │
│  │  · edge_logic.rs     — 边缘触发判定（坐标计算、防抖） │ │
│  │  · cursor_mapping.rs — 光标跨屏坐标映射              │ │
│  │  · clipboard.rs      — 剪贴板共享协议逻辑            │ │
│  │  · auth.rs           — 密码验证、TLS 配置            │ │
│  │  · config.rs         — 配置数据结构（序列化/反序列化）│ │
│  │  · error.rs          — 统一错误类型                   │ │
│  │  · log.rs            — 统一日志接口                   │ │
│  │  · platform.rs       — 平台适配 trait 定义            │ │
│  └──────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

### 各层职责说明

| 层级 | 模块 | 职责 | 跨平台共享 |
|------|------|------|:---:|
| **kvm-core** | network | TCP 长连接建立/断开/重连，UDP 局域网发现，握手协议，心跳保活 | ✅ |
| | state | 控制权状态机（IDLE / LOCAL_CONTROLLING / REMOTE_CONTROLLING）维护与同步 | ✅ |
| | protocol | 事件帧格式定义，跨平台统一键码映射，序列化/反序列化，事件队列与过期丢弃 | ✅ |
| | edge_logic | 边缘触发的纯逻辑判定（坐标计算、防抖），不涉及系统 API | ✅ |
| | cursor_mapping | 控制权切换时的光标位置映射（Y/X 坐标按比例转换到对方屏幕） | ✅ |
| | clipboard | 剪贴板共享协议（检测变化、序列化内容、发送/接收） | ✅ |
| | auth | 连接密码验证逻辑，TLS 配置 | ✅ |
| | config | 配置数据结构（屏幕布局、密码、对端 IP 等），serde 序列化 | ✅ |
| | error | 统一错误类型（KvmError），涵盖网络、协议、权限等所有错误场景 | ✅ |
| | log | 统一日志 trait 定义，各端可接入不同日志后端 | ✅ |
| | platform trait | 定义平台适配接口（屏幕枚举、键鼠采集/注入/屏蔽、快捷键、剪贴板、光标控制） | ✅ |
| **kvm-mac** | platform/ | 实现 platform trait 的 macOS 版本，调用 CoreGraphics / AppKit API | ❌ Mac 专用 |
| | permission | macOS 辅助功能权限检测与引导 | ❌ Mac 专用 |
| | tray + autostart + 入口 | Mac 端应用入口，组装 kvm-core + mac platform，提供托盘 UI + 开机自启 | ❌ Mac 专用 |
| **kvm-win** | platform/ | 实现 platform trait 的 Windows 版本，调用 Win32 API | ❌ Win 专用 |
| | tray + ui + autostart + 入口 | Win 端应用入口，组装 kvm-core + win platform，提供 Tauri 桌面 UI + 托盘 + 开机自启 | ❌ Win 专用 |

### 关键设计原则

1. **kvm-core 零平台依赖**：不包含任何 `#[cfg(target_os)]`，不调用任何系统 API，纯逻辑 + 网络
2. **平台适配层内嵌在各端 crate 中**：`kvm-mac/src/platform/` 和 `kvm-win/src/platform/` 各自实现 kvm-core 定义的 trait
3. **两端都有 UI**：Mac 端有托盘菜单 UI，Windows 端有 Tauri 桌面 UI + 托盘，各自独立实现
4. **状态机使用通用命名**：`LOCAL_CONTROLLING`（本机控制对方）和 `REMOTE_CONTROLLING`（对方控制本机），完全平台无关
5. **Mac 端技术栈**：CoreGraphics（屏幕枚举、事件注入）、CGEventTap（键鼠采集/屏蔽）、AppKit/NSStatusBar（托盘）、NSPasteboard（剪贴板）
6. **Windows 端技术栈**：Win32 API（EnumDisplayMonitors、SetWindowsHookEx、SendInput、Clipboard）、Tauri（桌面 UI）

---

## 需求

### 需求 1：Cargo Workspace 与公共核心库架构

**用户故事：** 作为开发者，我希望公共逻辑只写一份代码，平台差异通过 trait 接口隔离，以便最大程度减少重复代码和维护成本。

#### 验收标准

1. WHEN 项目构建 THEN 系统 SHALL 使用 Cargo Workspace 组织为三个 crate：kvm-core（公共核心库）、kvm-mac（Mac 端应用）、kvm-win（Windows 端应用）
2. WHEN kvm-core 编译 THEN kvm-core SHALL 不依赖任何平台特定 API，不包含 `#[cfg(target_os)]` 条件编译
3. WHEN kvm-core 需要调用平台功能 THEN kvm-core SHALL 通过 `platform.rs` 中定义的 trait 接口调用，由各端 crate 的 `platform/` 模块提供具体实现
4. WHEN 平台适配层实现 trait THEN 各端 SHALL 为以下功能提供平台实现：
   - `ScreenProvider`：屏幕枚举（获取所有显示器分辨率、位置和 DPI 缩放比例）
   - `InputCapture`：键鼠事件采集（Hook 键盘和鼠标输入）
   - `InputExecutor`：键鼠事件注入（模拟键盘和鼠标操作）
   - `InputBlocker`：键鼠输入屏蔽（屏蔽/恢复本地键鼠）
   - `HotkeyManager`：全局快捷键注册与监听
   - `ClipboardProvider`：剪贴板读写（读取/写入剪贴板内容）
   - `CursorController`：光标控制（显示/隐藏/设置位置）

---

### 需求 2：对等连接模型与局域网自动发现（kvm-core network 模块）

**用户故事：** 作为用户，我希望两台电脑地位完全对等，软件能自动发现局域网内的对方，也支持手动输入 IP 连接，以便不需要关心谁先启动、谁是"服务端"，也不需要手动指定角色。

#### 验收标准

1. WHEN 任意一端启动 THEN 系统 SHALL 在指定端口（默认 23456）监听 TCP 连接
2. WHEN 任意一端启动 THEN 系统 SHALL 同时启动 UDP 广播监听（端口 8888），响应局域网内其他 KVMini 实例的发现请求
3. WHEN 用户点击"扫描设备" THEN 系统 SHALL 发送 UDP 广播包（`KVM_DISCOVERY`），收集局域网内所有 KVMini 实例的响应（设备名、操作系统、TCP 端口），固定等待 5 秒后返回设备列表
4. WHEN 用户从设备列表选择一台设备 THEN 系统 SHALL 使用该设备的 IP 和 TCP 端口建立 TCP 长连接
5. WHEN 用户手动输入对端 IP 并连接 THEN 系统 SHALL 直接建立 TCP 长连接（支持跨网段直连）
6. WHEN 连接建立成功 THEN 系统 SHALL 完成握手（交换协议版本号、屏幕布局信息、DPI 缩放信息），初始状态为双方各自独立（`IDLE`）
7. IF 握手时协议版本号不一致 THEN 系统 SHALL 拒绝连接并提示"版本不兼容，请更新到相同版本"
8. IF 连接意外断开 THEN 系统 SHALL 自动尝试重连，重连间隔为 3 秒，最多重试 5 次
9. IF 重连失败超过上限 THEN 系统 SHALL 显示"连接已断开"提示，等待用户手动重连
10. WHEN 连接保持期间 THEN 系统 SHALL 每 5 秒发送一次心跳包，检测连接是否存活
11. IF 连续 3 次心跳无响应 THEN 系统 SHALL 判定连接断开，触发重连流程
12. WHEN 系统从休眠/睡眠中唤醒 THEN 系统 SHALL 检测连接状态，若连接已断开则自动触发重连流程
13. WHEN 系统锁屏 THEN 系统 SHALL 若处于 `LOCAL_CONTROLLING` 或 `REMOTE_CONTROLLING` 状态，立即切换回 `IDLE` 并恢复本地键鼠

---

### 需求 3：控制权状态机（kvm-core state 模块）

**用户故事：** 作为用户，我希望系统有清晰的控制状态，以便我随时知道当前是各自独立还是某一方在控制另一方。

#### 验收标准

1. WHEN 系统运行 THEN 系统 SHALL 维护以下三种状态之一：
   - `IDLE`：双方各自独立，各自的键鼠只控制本机，互不干扰
   - `LOCAL_CONTROLLING`：本机键鼠正在控制对方（对方处于 `REMOTE_CONTROLLING`）
   - `REMOTE_CONTROLLING`：对方键鼠正在控制本机（对方处于 `LOCAL_CONTROLLING`）
2. WHEN 任意一端触发状态切换 THEN 系统 SHALL 通过控制帧通知对端同步到对应状态
3. WHEN 进入 `LOCAL_CONTROLLING` 状态 THEN 本机 SHALL 开始采集键鼠事件并发送给对方执行
4. WHEN 进入 `REMOTE_CONTROLLING` 状态 THEN 本机 SHALL 屏蔽本地键鼠输入，接收对方事件并执行
5. WHEN 进入 `IDLE` 状态 THEN 双方 SHALL 停止发送键鼠事件，各自恢复本地键鼠输入
6. IF 连接断开 THEN 系统 SHALL 立即回到 `IDLE` 状态，恢复本地键鼠输入

---

### 需求 4：多屏幕识别与布局（平台适配层 ScreenProvider + kvm-core edge_logic）

**用户故事：** 作为用户，我希望软件能识别我的多显示器配置，以便边缘触发方向与实际物理摆放一致，在多屏幕环境下也能正常工作。

#### 验收标准

1. WHEN 程序启动 THEN 平台适配层 ScreenProvider SHALL 枚举本机所有已连接的显示器，获取每块屏幕的分辨率、相对位置和 DPI 缩放比例
2. WHEN 存在多块显示器 THEN kvm-core SHALL 将所有显示器视为一个整体虚拟桌面，边缘检测基于整体虚拟桌面的外边缘
3. WHEN 显示器配置发生变化（插拔显示器） THEN 平台适配层 SHALL 重新枚举并通知 kvm-core 更新屏幕布局
4. WHEN 连接握手阶段 THEN 双方 SHALL 交换各自的屏幕布局信息（包括 DPI 缩放），以便对方知道虚拟桌面的尺寸
5. WHEN 用户配置屏幕布局为"本机在左，对方在右" THEN kvm-core SHALL 以本机虚拟桌面右边缘作为触发边缘
6. WHEN 用户配置屏幕布局为"本机在右，对方在左" THEN kvm-core SHALL 以本机虚拟桌面左边缘作为触发边缘
7. WHEN 用户配置屏幕布局为"本机在上，对方在下" THEN kvm-core SHALL 以本机虚拟桌面下边缘作为触发边缘
8. WHEN 用户配置屏幕布局为"本机在下，对方在上" THEN kvm-core SHALL 以本机虚拟桌面上边缘作为触发边缘
9. IF 用户未配置屏幕布局 THEN 系统 SHALL 默认使用"本机在左，对方在右"

---

### 需求 5：屏幕边缘触发控制权转移（kvm-core edge_logic + 平台适配层）

**用户故事：** 作为用户，我希望鼠标移到屏幕边缘时自动将控制权转移到另一台电脑，以便像操作一台电脑一样无缝切换，不需要任何按钮。

#### 验收标准

1. WHEN 系统处于 `IDLE` 状态且本机鼠标触碰到朝向对方一侧的虚拟桌面边缘 THEN 系统 SHALL 切换到 `LOCAL_CONTROLLING` 状态，本机开始采集键鼠事件发送给对方
2. WHEN 系统处于 `LOCAL_CONTROLLING` 状态且主控方发送的鼠标移动事件使被控方光标到达反向边缘 THEN 系统 SHALL 切换回 `IDLE` 状态（即：控制权回切由主控方的鼠标移动方向决定，而非被控方本地键鼠）
3. WHEN 边缘触发发生 THEN kvm-core SHALL 有防抖处理，50ms 内不重复触发同一边缘事件
4. IF 触发边缘切换时连接不存在 THEN 系统 SHALL 忽略本次触发并提示用户先建立连接

---

### 需求 6：光标位置映射与可见性（kvm-core cursor_mapping + 平台适配层 CursorController）

**用户故事：** 作为用户，我希望控制权切换时鼠标光标能无缝出现在对方屏幕的正确位置，以便操作体验流畅自然。

#### 验收标准

1. WHEN 控制权从 `IDLE` 切换到 `LOCAL_CONTROLLING` THEN kvm-core cursor_mapping SHALL 将本机触发边缘的坐标按比例映射到对方屏幕的对应坐标，作为对方光标的初始位置（左右布局映射 Y 坐标，上下布局映射 X 坐标）
2. WHEN 控制权从 `LOCAL_CONTROLLING` 切换回 `IDLE` THEN kvm-core cursor_mapping SHALL 将对方屏幕的当前坐标按比例映射回本机屏幕的对应坐标，恢复本机光标位置
3. WHEN 进入 `LOCAL_CONTROLLING` 状态 THEN 平台适配层 CursorController SHALL 隐藏本机光标（因为光标逻辑上已"移动"到对方屏幕）
4. WHEN 退出 `LOCAL_CONTROLLING` 状态 THEN 平台适配层 CursorController SHALL 恢复显示本机光标，并将光标设置到映射后的位置
5. WHEN 进入 `REMOTE_CONTROLLING` 状态 THEN 被控方 SHALL 将光标设置到主控方映射过来的初始位置

---

### 需求 7：DPI 缩放与坐标归一化处理（kvm-core protocol + 平台适配层）

**用户故事：** 作为用户，我希望在不同 DPI 缩放的屏幕之间切换时，鼠标移动速度和位置映射都是正确的，以便高分屏和普通屏混用时体验一致。

#### 验收标准

1. WHEN 平台适配层 ScreenProvider 枚举显示器 THEN 系统 SHALL 返回每块显示器的 DPI 缩放比例（如 1.0、1.25、1.5、2.0）
2. WHEN 握手交换屏幕信息 THEN 双方 SHALL 交换各自的 DPI 缩放信息
3. WHEN 主控方发送鼠标移动事件 THEN kvm-core protocol SHALL 使用归一化坐标（基于逻辑像素），而非物理像素
4. WHEN 被控方接收鼠标移动事件 THEN 被控方 SHALL 根据自身 DPI 缩放比例将归一化坐标转换为本地逻辑像素坐标
5. WHEN Windows 端启动 THEN 系统 SHALL 设置 DPI Awareness 为 Per-Monitor Aware V2，确保获取正确的逻辑像素坐标

---

### 需求 8：快捷键触发控制权转移（平台适配层 HotkeyManager + kvm-core state）

**用户故事：** 作为用户，我希望通过快捷键快速切换控制权，以便在不移动鼠标到边缘的情况下也能切换。

#### 验收标准

1. WHEN 用户按下 `Ctrl+Alt+K`（Mac 上为 `Cmd+Option+K`） THEN 系统 SHALL 在 `IDLE` ↔ `LOCAL_CONTROLLING` 之间切换
2. WHEN 切换成功 THEN 系统 SHALL 显示系统通知，内容为当前控制状态
3. IF 切换时连接不存在 THEN 系统 SHALL 显示"未连接，无法切换"提示并保持原状态

---

### 需求 9：被控期间本地键鼠屏蔽（平台适配层 InputBlocker）

**用户故事：** 作为用户，我希望在对方控制我的电脑期间，我本机的键鼠输入被忽略，以便避免误操作干扰对方的操作。

#### 验收标准

1. WHEN 系统进入 `REMOTE_CONTROLLING` 状态 THEN 平台适配层 InputBlocker SHALL 屏蔽本机键盘和鼠标的所有输入事件
2. WHEN 系统退出 `REMOTE_CONTROLLING` 状态 THEN 平台适配层 InputBlocker SHALL 立即恢复本机键鼠输入
3. IF 连接意外断开 THEN 系统 SHALL 立即恢复本机键鼠输入，防止键鼠被永久锁定
4. WHEN 被控期间 THEN 系统 SHALL 保留一个"紧急解锁"快捷键（如 `Ctrl+Alt+Esc` / `Cmd+Option+Esc`），允许被控方强制恢复本地键鼠

---

### 需求 10：键鼠事件传输协议（kvm-core protocol 模块）

**用户故事：** 作为开发者，我希望有统一的事件传输协议，以便 Mac 和 Windows 两端能正确解析彼此发送的键鼠事件。

#### 验收标准

1. WHEN 控制方采集到鼠标移动事件 THEN 系统 SHALL 发送包含归一化坐标的事件帧（基于逻辑像素的相对坐标）
2. WHEN 控制方采集到鼠标点击事件 THEN 系统 SHALL 发送包含按键编号和按下/释放状态的事件帧
3. WHEN 控制方采集到键盘事件 THEN 系统 SHALL 发送包含跨平台统一键码和按下/释放状态的事件帧
4. WHEN 控制方采集到滚轮事件 THEN 系统 SHALL 发送包含滚动方向和幅度的事件帧
5. WHEN 被控方收到事件帧 THEN 平台适配层 InputExecutor SHALL 调用系统 API 模拟执行对应的输入事件
6. WHEN 协议帧传输 THEN 系统 SHALL 使用长度前缀（4字节）+ 二进制编码（serde + bincode），保证帧边界完整
7. WHEN 键码映射 THEN kvm-core protocol SHALL 定义跨平台统一键码表，各平台适配层负责本地键码与统一键码的双向转换
8. WHEN 每个事件帧 THEN 系统 SHALL 包含时间戳字段，用于接收端判断事件是否过期

---

### 需求 11：修饰键状态同步（kvm-core state + protocol 模块）

**用户故事：** 作为用户，我希望控制权切换时不会出现"粘滞键"现象（如 Ctrl 键卡住），以便切换后键盘状态始终正确。

#### 验收标准

1. WHEN 控制权从 `LOCAL_CONTROLLING` 切换回 `IDLE` THEN 主控方 SHALL 检查当前所有修饰键（Ctrl/Shift/Alt/Cmd/Win）的按下状态，对所有处于按下状态的修饰键发送释放事件给被控方
2. WHEN 控制权从 `REMOTE_CONTROLLING` 切换回 `IDLE` THEN 被控方 SHALL 清除所有修饰键的按下状态（发送释放事件），确保本地键盘状态干净
3. WHEN 主控方发送键盘事件 THEN kvm-core protocol SHALL 在事件帧中包含当前修饰键状态位图（Ctrl/Shift/Alt/Cmd 各占 1 bit），以便被控方校验修饰键状态一致性
4. IF 被控方检测到修饰键状态与事件帧中的状态位图不一致 THEN 被控方 SHALL 自动补发修饰键按下/释放事件进行修正

---

### 需求 12：网络延迟与事件队列处理（kvm-core protocol 模块）

**用户故事：** 作为用户，我希望在网络延迟或抖动时，鼠标不会出现"漂移"或"堆积"现象，以便操作体验始终流畅。

#### 验收标准

1. WHEN 被控方接收到事件帧 THEN 系统 SHALL 检查事件时间戳，丢弃超过 200ms 的过期鼠标移动事件
2. WHEN 事件队列中堆积超过 50 个未处理的鼠标移动事件 THEN 系统 SHALL 仅保留最新的 5 个事件，丢弃其余
3. WHEN 键盘按下/释放事件 THEN 系统 SHALL 不丢弃（保证按键状态一致性），但按顺序执行
4. WHEN 鼠标点击事件 THEN 系统 SHALL 不丢弃（保证点击不丢失），但按顺序执行
5. WHEN 网络延迟持续超过 500ms THEN 系统 SHALL 在 UI 上显示"网络延迟较高"警告

---

### 需求 13：剪贴板共享（kvm-core clipboard + 平台适配层 ClipboardProvider）

**用户故事：** 作为用户，我希望在一台电脑上复制的内容可以在另一台电脑上粘贴，以便跨机操作更加便捷。

#### 验收标准

1. WHEN 控制权从 `LOCAL_CONTROLLING` 切换回 `IDLE` THEN 系统 SHALL 自动同步主控方的剪贴板内容到被控方
2. WHEN 剪贴板内容发生变化且处于控制状态 THEN 系统 SHALL 将变化的内容通过协议帧发送给对端
3. WHEN 剪贴板同步 THEN 系统 SHALL 支持纯文本内容的同步
4. IF 剪贴板内容超过 10MB THEN 系统 SHALL 跳过本次同步并在日志中记录
5. WHEN 用户在设置中关闭剪贴板共享 THEN 系统 SHALL 停止所有剪贴板同步行为

---

### 需求 14：授权与安全（kvm-core auth 模块）

**用户故事：** 作为用户，我希望连接需要密码验证，以便防止局域网内其他人未经授权控制我的电脑。

#### 验收标准

1. WHEN 用户首次建立连接 THEN 系统 SHALL 要求输入连接密码，双方密码一致才能建立连接
2. WHEN 连接建立 THEN 系统 SHALL 使用 TLS 加密所有传输数据
3. IF 密码验证失败 THEN 系统 SHALL 拒绝连接并显示"密码错误"提示
4. WHEN 用户修改密码 THEN 系统 SHALL 断开当前连接，要求重新验证

---

### 需求 15：macOS 辅助功能权限处理（kvm-mac permission 模块）

**用户故事：** 作为 Mac 用户，我希望程序能引导我正确授予辅助功能权限，以便键鼠采集和注入功能能正常工作。

#### 验收标准

1. WHEN Mac 端程序启动 THEN 系统 SHALL 检测是否已获得辅助功能权限（Accessibility）
2. IF 未获得辅助功能权限 THEN 系统 SHALL 弹出引导提示，告知用户需要在"系统偏好设置 > 安全性与隐私 > 辅助功能"中授权
3. IF 未获得辅助功能权限 THEN 系统 SHALL 禁用键鼠采集和注入功能，仅保留连接和 UI 功能
4. WHEN 用户授权后重启程序 THEN 系统 SHALL 自动检测到权限已授予，启用全部功能
5. WHEN 权限状态变化 THEN 系统 SHALL 在托盘菜单中显示当前权限状态（已授权 / 未授权）

---

### 需求 16：配置持久化（kvm-core config + 各端 config_store）

**用户故事：** 作为用户，我希望我的设置（屏幕布局、连接密码、对端 IP）在重启后仍然保留，以便不需要每次重新配置。

#### 验收标准

1. WHEN 用户修改配置 THEN 系统 SHALL 立即将配置持久化到本地文件
2. WHEN 程序启动 THEN 系统 SHALL 自动加载上次保存的配置
3. WHEN 配置存储 THEN kvm-core config 模块 SHALL 定义统一的配置数据结构（serde 序列化），各端 config_store 负责具体的文件路径和读写
4. WHEN Mac 端存储配置 THEN 系统 SHALL 存储到 `~/Library/Application Support/KVMini/config.json`
5. WHEN Windows 端存储配置 THEN 系统 SHALL 存储到 `%APPDATA%/KVMini/config.json`
6. IF 配置文件不存在或损坏 THEN 系统 SHALL 使用默认配置并创建新的配置文件

---

### 需求 17：开机自启动（各端 autostart 模块）

**用户故事：** 作为用户，我希望软件能随系统开机自动启动，以便不需要每次手动打开。

#### 验收标准

1. WHEN 用户在设置中开启"开机自启动" THEN 系统 SHALL 注册开机自启动
2. WHEN 用户在设置中关闭"开机自启动" THEN 系统 SHALL 取消开机自启动注册
3. WHEN Mac 端注册自启动 THEN 系统 SHALL 通过 LaunchAgent plist 文件实现（`~/Library/LaunchAgents/com.kvmini.agent.plist`）
4. WHEN Windows 端注册自启动 THEN 系统 SHALL 通过注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 实现
5. IF 开机自启动后 THEN 系统 SHALL 以最小化/托盘模式启动，不弹出主窗口

---

### 需求 18：统一错误处理框架（kvm-core error 模块）

**用户故事：** 作为开发者，我希望有统一的错误类型和用户友好的错误提示，以便两端的错误处理逻辑一致，用户能看到清晰的解决方案引导。

#### 验收标准

1. WHEN kvm-core 定义错误类型 THEN 系统 SHALL 使用统一的 `KvmError` 枚举，涵盖网络错误、协议错误、权限错误、配置错误等所有场景
2. WHEN 错误发生 THEN 每个 `KvmError` 变体 SHALL 提供：错误分类、用户友好的中文描述、建议的解决方案
3. WHEN 两端各自有平台特定错误 THEN 平台适配层 SHALL 将平台错误转换为 `KvmError`，不暴露平台细节给上层
4. WHEN 错误需要展示给用户 THEN UI 层 SHALL 通过 `KvmError` 的 `display()` 方法获取用户友好的提示信息

---

### 需求 19：优雅退出与资源清理（各端 main + kvm-core）

**用户故事：** 作为用户，我希望关闭软件时所有资源都能正确释放，以便不会残留后台进程或锁定键鼠。

#### 验收标准

1. WHEN 用户关闭程序（托盘退出或窗口关闭） THEN 系统 SHALL 按以下顺序清理资源：
   - 恢复本地键鼠输入（如果处于 REMOTE_CONTROLLING 状态）
   - 恢复光标可见性
   - 发送断开通知给对端
   - 关闭 TCP 连接
   - 停止心跳定时器
   - 取消全局快捷键注册
   - 停止剪贴板监听
2. WHEN 程序异常崩溃 THEN 系统 SHALL 通过 panic hook 确保键鼠输入被恢复（防止键鼠被永久锁定）
3. WHEN Mac 端退出 THEN 系统 SHALL 清理 CGEventTap 资源
4. WHEN Windows 端退出 THEN 系统 SHALL 清理 Hook 句柄和 Tauri 资源
5. WHEN 清理遗留进程 THEN 系统 SHALL 在启动时检测并清理上次异常退出残留的进程

---

### 需求 20：日志与调试（kvm-core log + 各端日志后端）

**用户故事：** 作为开发者，我希望系统有统一的日志输出，以便排查跨平台通信和状态切换问题。

#### 验收标准

1. WHEN 系统运行 THEN kvm-core SHALL 通过统一日志接口输出关键事件日志（连接建立/断开、状态切换、边缘触发等）
2. WHEN 各端启动 THEN 各端 SHALL 接入日志后端，将日志输出到文件和控制台
3. WHEN 日志输出 THEN 系统 SHALL 包含时间戳、日志级别、模块名和消息内容
4. WHEN 用户需要排查问题 THEN 系统 SHALL 支持通过配置调整日志级别（debug / info / warn / error）

---

### 需求 21：UI 展示（两端各自实现）

**用户故事：** 作为用户，我希望在两端都能看到当前连接状态和控制权归属，以便随时了解系统状态。

#### 验收标准

1. WHEN 连接未建立 THEN 两端 SHALL 显示"未连接"状态，并提供"扫描设备"和"手动输入 IP"两种连接入口
2. WHEN 连接已建立且处于 `IDLE` 状态 THEN 两端 SHALL 显示"已连接 · 各自独立"
3. WHEN 处于 `LOCAL_CONTROLLING` 状态 THEN 本端 SHALL 显示"正在控制对方"
4. WHEN 处于 `REMOTE_CONTROLLING` 状态 THEN 本端 SHALL 显示"对方正在控制本机"
5. WHEN 状态变化 THEN 两端 SHALL 更新托盘图标颜色以区分状态（绿色=IDLE，蓝色=LOCAL_CONTROLLING，橙色=REMOTE_CONTROLLING）
6. WHEN 界面展示 THEN 两端 SHALL 不包含任何"切换主控/被控"按钮
7. WHEN 用户打开设置 THEN 两端 SHALL 提供以下设置项：
   - 屏幕布局配置（本机在左/右/上/下）
   - 连接密码设置
   - 剪贴板共享开关
   - 开机自启动开关
   - 日志级别选择
8. **Mac 端**：通过 NSStatusBar 系统托盘菜单提供 UI 交互（显示状态、连接/断开、设置）
9. **Windows 端**：通过 Tauri 桌面窗口 + 系统托盘提供 UI 交互（主窗口显示详细状态，托盘提供快捷操作）
10. WHEN 两端 UI 实现 THEN 两端 SHALL 各自独立实现，不共享 UI 代码，但通过 kvm-core 提供的统一状态回调接口获取数据
