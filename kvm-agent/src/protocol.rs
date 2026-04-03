use serde::{Deserialize, Serialize};

// ============================================================
// 协议版本
// ============================================================
pub const PROTOCOL_VERSION: u8 = 1;

// ============================================================
// UDP 发现协议
// ============================================================

/// UDP 广播发现消息（Windows 端发送）
pub const DISCOVER_MAGIC: &[u8] = b"KVM_DISCOVER";

/// UDP 发现响应（Mac 端回复，JSON 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResponse {
    pub magic: String,
    pub version: u8,
    pub hostname: String,
    pub os: String,
    pub tcp_port: u16,
}

impl DiscoverResponse {
    pub fn new(hostname: String, os: String) -> Self {
        Self {
            magic: "KVM_RESPONSE".to_string(),
            version: PROTOCOL_VERSION,
            hostname,
            os,
            tcp_port: 9999,
        }
    }
}

// ============================================================
// TCP 通信协议消息
// ============================================================

/// 鼠标按钮类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Side1,
    Side2,
}

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

/// KVM 主消息枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KvmMessage {
    Handshake(HandshakePacket),
    Heartbeat(HeartbeatPacket),
    Input(InputEventPacket),
    Disconnect { reason: String },
}

/// 握手包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakePacket {
    pub version: u8,
    pub hostname: String,
    pub os: String,
    pub role: String,
}

impl HandshakePacket {
    pub fn agent(hostname: String, os: String) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            hostname,
            os,
            role: "agent".to_string(),
        }
    }
}

/// 心跳包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPacket {
    pub timestamp_ms: u64,
}

impl HeartbeatPacket {
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self { timestamp_ms: ts }
    }
}

/// 输入事件包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEventPacket {
    pub event: InputEvent,
}

// ============================================================
// 消息编解码
// ============================================================

pub fn encode_message(msg: &KvmMessage) -> Result<String, String> {
    let mut s = serde_json::to_string(msg)
        .map_err(|e| format!("消息序列化失败: {}", e))?;
    s.push('\n');
    Ok(s)
}

pub fn decode_message(line: &str) -> Result<KvmMessage, String> {
    serde_json::from_str(line.trim())
        .map_err(|e| format!("消息反序列化失败: {}", e))
}
