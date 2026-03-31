# 跨平台鼠标键盘共享软件实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 开发一个支持Mac和Windows之间鼠标键盘共享的软件，支持USB-C直连和网络连接

**Architecture:** 基于Tauri + Rust的跨平台架构，使用混合连接模式（优先网络，失败时回退到USB），采用事件驱动的输入设备处理

**Tech Stack:** Tauri, Rust, libusb, winapi (Windows), IOKit (macOS), tokio (异步运行时)

---

## 系统架构设计

### 核心组件
1. **连接管理器** - 负责设备发现和连接管理
2. **输入捕获层** - 捕获本地鼠标键盘输入事件
3. **事件传输层** - 通过网络或USB传输输入事件
4. **输入重放层** - 在远程设备上重放输入事件
5. **GUI界面** - 用户配置和状态显示

### 连接策略
- **优先网络连接**: UDP广播发现 + TCP通信
- **备用USB连接**: USB-C数据线建立网络隧道
- **智能切换**: 自动检测连接质量并切换

### 性能目标
- 输入延迟: <5ms
- 帧率: 1000Hz (鼠标), 125Hz (键盘)
- 内存占用: <50MB

## 安全考虑
- 输入事件加密传输
- 设备配对认证
- 防止输入注入攻击

## 测试策略
- 单元测试: 核心算法和协议
- 集成测试: 跨平台连接测试
- 性能测试: 延迟和吞吐量测试
- 兼容性测试: 不同OS版本和设备

## 实现任务分解

### Task 1: 项目初始化和Tauri设置

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `package.json`

**Step 1: 初始化Tauri项目结构**
```bash
cargo tauri init
npm init
```

**Step 2: 配置Cargo.toml依赖**
```toml
[package]
name = "kvm-share"
version = "0.1.0"

[dependencies]
tauri = { version = "1.0", features = ["api-all"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
libusb = "0.3"

[target.x86_64-pc-windows-gnu.dependencies]
winapi = { version = "0.3", features = ["winuser", "winbase"] }

[target.x86_64-apple-darwin.dependencies]
core-foundation = "0.9"
core-graphics = "0.22"
```

### Task 2: 核心数据结构定义

**Files:**
- Create: `src-tauri/src/types.rs`

**Step 1: 定义输入事件类型**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    KeyDown { keycode: u16 },
    KeyUp { keycode: u16 },
    Scroll { dx: i32, dy: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Side1,
    Side2,
}
```

### Task 3: 网络连接管理器

**Files:**
- Create: `src-tauri/src/network.rs`

**Step 1: 实现UDP广播发现**
```rust
pub struct NetworkManager {
    udp_socket: UdpSocket,
    discovered_devices: HashMap<String, SocketAddr>,
}

impl NetworkManager {
    pub async fn discover_devices() -> Result<Vec<DeviceInfo>> {
        // UDP广播发现逻辑
    }
}
```

### Task 4: USB连接管理器

**Files:**
- Create: `src-tauri/src/usb.rs`

**Step 1: USB设备检测和连接**
```rust
pub struct UsbManager {
    context: libusb::Context,
}

impl UsbManager {
    pub fn find_kvm_devices() -> Result<Vec<UsbDevice>> {
        // 检测支持USB网络共享的设备
    }
}
```

### Task 5: 输入捕获层 (平台特定)

**Files:**
- Create: `src-tauri/src/capture/windows.rs`
- Create: `src-tauri/src/capture/macos.rs`

**Step 1: Windows输入捕获**
```rust
// Windows使用SetWindowsHookEx捕获全局输入
```

**Step 2: macOS输入捕获**
```rust
// macOS使用CGEventTap捕获全局输入
```

### Task 6: 输入重放层 (平台特定)

**Files:**
- Create: `src-tauri/src/replay/windows.rs`
- Create: `src-tauri/src/replay/macos.rs`

**Step 1: Windows输入重放**
```rust
// 使用SendInput模拟输入
```

**Step 2: macOS输入重放**
```rust
// 使用CGEventPost模拟输入
```

### Task 7: 连接策略管理器

**Files:**
- Create: `src-tauri/src/connection_manager.rs`

**Step 1: 智能连接选择**
```rust
pub struct ConnectionManager {
    network: NetworkManager,
    usb: UsbManager,
    current_connection: ConnectionType,
}

impl ConnectionManager {
    pub async fn connect() -> Result<()> {
        // 优先尝试网络，失败时尝试USB
    }
}
```

### Task 8: 前端界面

**Files:**
- Create: `src/index.html`
- Create: `src/styles.css`
- Create: `src/main.js`

**Step 1: 基本UI界面**
```html
<div class="container">
    <h1>KVM共享</h1>
    <div class="status">连接状态: <span id="status">断开</span></div>
    <button id="connect-btn">连接</button>
</div>
```

### Task 9: 测试套件

**Files:**
- Create: `tests/integration_test.rs`
- Create: `tests/performance_test.rs`

**Step 1: 集成测试**
```rust
#[test]
fn test_network_discovery() {
    // 测试设备发现功能
}
```

## 开发流程

1. **设置开发环境**: Rust, Node.js, Tauri CLI
2. **核心协议开发**: 输入事件序列化和传输
3. **平台特定实现**: Windows和macOS的输入捕获/重放
4. **连接管理**: 网络和USB连接实现
5. **UI开发**: 前端界面和状态管理
6. **测试和优化**: 性能测试和bug修复
7. **打包发布**: 跨平台安装包制作

## 风险评估

- **权限问题**: macOS需要辅助功能权限
- **网络限制**: 公司防火墙可能阻断连接
- **USB驱动**: 可能需要自定义USB驱动
- **输入延迟**: 需要优化传输协议

## 后续优化

- 支持多设备连接
- 剪贴板共享
- 文件传输功能
- 安全加密传输