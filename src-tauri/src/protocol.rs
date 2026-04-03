use serde::{Deserialize, Serialize};
use crate::types::{InputEvent, MouseButton};

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
    /// 魔术字符串，固定为 "KVM_RESPONSE"
    pub magic: String,
    /// 协议版本
    pub version: u8,
    /// 设备主机名
    pub hostname: String,
    /// 操作系统类型，如 "macOS 14.0"
    pub os: String,
    /// TCP 监听端口
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
// TCP 通信协议消息（每条消息为一行 JSON，以 '\n' 结尾）
// ============================================================

/// KVM 主消息枚举，涵盖所有通信场景
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KvmMessage {
    /// 握手包：连接建立后双端互发
    Handshake(HandshakePacket),
    /// 心跳包：每 2 秒发送一次
    Heartbeat(HeartbeatPacket),
    /// 输入事件：鼠标/键盘/滚轮
    Input(InputEventPacket),
    /// 断开连接通知
    Disconnect { reason: String },
}

// ============================================================
// 握手包
// ============================================================

/// 握手包：连接建立后由 Windows 端发送，Mac 端回复
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakePacket {
    /// 协议版本
    pub version: u8,
    /// 发送方主机名
    pub hostname: String,
    /// 发送方操作系统
    pub os: String,
    /// 发送方角色："controller"（Windows）或 "agent"（Mac）
    pub role: String,
}

impl HandshakePacket {
    pub fn controller(hostname: String, os: String) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            hostname,
            os,
            role: "controller".to_string(),
        }
    }

    pub fn agent(hostname: String, os: String) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            hostname,
            os,
            role: "agent".to_string(),
        }
    }
}

// ============================================================
// 心跳包
// ============================================================

/// 心跳包：每 2 秒发送，超过 6 秒未收到则判定断开
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPacket {
    /// 发送时的时间戳（毫秒，用于计算延迟）
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

// ============================================================
// 输入事件包
// ============================================================

/// 输入事件包：封装具体的鼠标/键盘/滚轮事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEventPacket {
    pub event: InputEvent,
}

impl InputEventPacket {
    pub fn mouse_move(x: i32, y: i32) -> Self {
        Self { event: InputEvent::MouseMove { x, y } }
    }

    pub fn mouse_down(button: MouseButton) -> Self {
        Self { event: InputEvent::MouseDown { button } }
    }

    pub fn mouse_up(button: MouseButton) -> Self {
        Self { event: InputEvent::MouseUp { button } }
    }

    pub fn key_down(keycode: u16) -> Self {
        Self { event: InputEvent::KeyDown { keycode } }
    }

    pub fn key_up(keycode: u16) -> Self {
        Self { event: InputEvent::KeyUp { keycode } }
    }

    pub fn scroll(dx: i32, dy: i32) -> Self {
        Self { event: InputEvent::Scroll { dx, dy } }
    }
}

// ============================================================
// 消息编解码工具函数
// ============================================================

/// 将 KvmMessage 序列化为一行 JSON（末尾附加 '\n'）
pub fn encode_message(msg: &KvmMessage) -> Result<String, String> {
    let mut s = serde_json::to_string(msg)
        .map_err(|e| format!("消息序列化失败: {}", e))?;
    s.push('\n');
    Ok(s)
}

/// 将一行 JSON 字符串反序列化为 KvmMessage
pub fn decode_message(line: &str) -> Result<KvmMessage, String> {
    serde_json::from_str(line.trim())
        .map_err(|e| format!("消息反序列化失败: {}", e))
}
