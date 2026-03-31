use serde::{Deserialize, Serialize};

/// 输入事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    KeyDown { keycode: u16 },
    KeyUp { keycode: u16 },
    Scroll { dx: i32, dy: i32 },
}

/// 鼠标按钮类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Side1,
    Side2,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub os: String,
    pub ip: String,
    pub connection_type: ConnectionType,
}

/// 连接类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Network,
    Usb,
}

/// 连接结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionResult {
    pub success: bool,
    pub connection_type: Option<ConnectionType>,
    pub error: Option<String>,
}

/// 设备发现结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub devices: Vec<DeviceInfo>,
}