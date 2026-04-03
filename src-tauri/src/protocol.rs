/// KVMini 事件传输协议
///
/// 帧格式（固定 16 字节头 + 可变载荷）：
/// ┌─────────────┬──────────┬──────────┬──────────────────────────────┐
/// │ magic(2B)   │ type(1B) │ flags(1B)│ payload_len(4B, little-endian)│
/// ├─────────────┴──────────┴──────────┴──────────────────────────────┤
/// │ payload (payload_len bytes)                                       │
/// └───────────────────────────────────────────────────────────────────┘
///
/// 鼠标移动事件载荷（8 字节）：x(i32 LE) + y(i32 LE)
/// 鼠标点击事件载荷（2 字节）：button(1B) + pressed(1B)
/// 键盘事件载荷（3 字节）：keycode(u16 LE) + pressed(1B)
/// 滚轮事件载荷（8 字节）：dx(i32 LE) + dy(i32 LE)
/// 握手事件载荷（4 字节）：version(u16 LE) + platform(1B) + reserved(1B)
/// 切换控制权载荷（4 字节）：edge(1B) + x_ratio(u16 LE) + reserved(1B)
/// 心跳载荷（0 字节）

/// 协议魔数
pub const MAGIC: u16 = 0x4B56; // "KV"

/// 协议版本
pub const PROTOCOL_VERSION: u16 = 1;

/// 帧头大小（字节）
pub const HEADER_SIZE: usize = 8;

/// 事件类型字节码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// 握手
    Handshake = 0x01,
    /// 握手响应
    HandshakeAck = 0x02,
    /// 鼠标移动
    MouseMove = 0x10,
    /// 鼠标按下/释放
    MouseButton = 0x11,
    /// 滚轮
    MouseScroll = 0x12,
    /// 键盘按下/释放
    KeyEvent = 0x20,
    /// 切换控制权（主控端 → 被控端）
    SwitchControl = 0x30,
    /// 切换控制权确认（被控端 → 主控端）
    SwitchControlAck = 0x31,
    /// 心跳
    Heartbeat = 0x40,
    /// 心跳响应
    HeartbeatAck = 0x41,
}

impl EventType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Handshake),
            0x02 => Some(Self::HandshakeAck),
            0x10 => Some(Self::MouseMove),
            0x11 => Some(Self::MouseButton),
            0x12 => Some(Self::MouseScroll),
            0x20 => Some(Self::KeyEvent),
            0x30 => Some(Self::SwitchControl),
            0x31 => Some(Self::SwitchControlAck),
            0x40 => Some(Self::Heartbeat),
            0x41 => Some(Self::HeartbeatAck),
            _ => None,
        }
    }
}

/// 平台标识
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS = 0x01,
    Windows = 0x02,
    Linux = 0x03,
    Unknown = 0xFF,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        return Self::MacOS;
        #[cfg(target_os = "windows")]
        return Self::Windows;
        #[cfg(target_os = "linux")]
        return Self::Linux;
        #[allow(unreachable_code)]
        Self::Unknown
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0x01 => Self::MacOS,
            0x02 => Self::Windows,
            0x03 => Self::Linux,
            _ => Self::Unknown,
        }
    }
}

/// 屏幕边缘方向
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEdge {
    Left = 0x01,
    Right = 0x02,
    Top = 0x03,
    Bottom = 0x04,
}

impl ScreenEdge {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Left),
            0x02 => Some(Self::Right),
            0x03 => Some(Self::Top),
            0x04 => Some(Self::Bottom),
            _ => None,
        }
    }

    /// 返回对应的反向边缘（切换回来时使用）
    pub fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

/// 协议帧（解码后的结构）
#[derive(Debug, Clone, PartialEq)]
pub enum KvmFrame {
    /// 握手：(version, platform)
    Handshake { version: u16, platform: Platform },
    /// 握手响应：(version, platform)
    HandshakeAck { version: u16, platform: Platform },
    /// 鼠标移动：归一化坐标 [0.0, 1.0]，使用 i32 * 10000 存储
    MouseMove { x: i32, y: i32 },
    /// 鼠标按键：(button_id, pressed)
    MouseButton { button: u8, pressed: bool },
    /// 滚轮：(dx, dy)
    MouseScroll { dx: i32, dy: i32 },
    /// 键盘事件：(keycode, pressed)
    KeyEvent { keycode: u16, pressed: bool },
    /// 切换控制权：(edge, position_ratio * 10000)
    SwitchControl { edge: ScreenEdge, pos_ratio: u16 },
    /// 切换控制权确认
    SwitchControlAck,
    /// 心跳
    Heartbeat,
    /// 心跳响应
    HeartbeatAck,
}

/// 协议编解码错误
#[derive(Debug, PartialEq)]
pub enum ProtocolError {
    /// 数据不足
    Incomplete,
    /// 魔数错误
    InvalidMagic,
    /// 未知事件类型
    UnknownEventType(u8),
    /// 载荷长度不匹配
    PayloadLengthMismatch { expected: usize, got: usize },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => write!(f, "数据不足，需要更多字节"),
            Self::InvalidMagic => write!(f, "魔数错误，非 KVMini 协议"),
            Self::UnknownEventType(t) => write!(f, "未知事件类型: 0x{:02X}", t),
            Self::PayloadLengthMismatch { expected, got } => {
                write!(f, "载荷长度不匹配：期望 {} 字节，实际 {} 字节", expected, got)
            }
        }
    }
}

/// 将 KvmFrame 编码为字节序列（小端序）
pub fn encode(frame: &KvmFrame) -> Vec<u8> {
    let (event_type, payload) = match frame {
        KvmFrame::Handshake { version, platform } => {
            let mut p = Vec::with_capacity(4);
            p.extend_from_slice(&version.to_le_bytes());
            p.push(*platform as u8);
            p.push(0); // reserved
            (EventType::Handshake, p)
        }
        KvmFrame::HandshakeAck { version, platform } => {
            let mut p = Vec::with_capacity(4);
            p.extend_from_slice(&version.to_le_bytes());
            p.push(*platform as u8);
            p.push(0);
            (EventType::HandshakeAck, p)
        }
        KvmFrame::MouseMove { x, y } => {
            let mut p = Vec::with_capacity(8);
            p.extend_from_slice(&x.to_le_bytes());
            p.extend_from_slice(&y.to_le_bytes());
            (EventType::MouseMove, p)
        }
        KvmFrame::MouseButton { button, pressed } => {
            let p = vec![*button, if *pressed { 1 } else { 0 }];
            (EventType::MouseButton, p)
        }
        KvmFrame::MouseScroll { dx, dy } => {
            let mut p = Vec::with_capacity(8);
            p.extend_from_slice(&dx.to_le_bytes());
            p.extend_from_slice(&dy.to_le_bytes());
            (EventType::MouseScroll, p)
        }
        KvmFrame::KeyEvent { keycode, pressed } => {
            let mut p = Vec::with_capacity(3);
            p.extend_from_slice(&keycode.to_le_bytes());
            p.push(if *pressed { 1 } else { 0 });
            (EventType::KeyEvent, p)
        }
        KvmFrame::SwitchControl { edge, pos_ratio } => {
            let mut p = Vec::with_capacity(4);
            p.push(*edge as u8);
            p.extend_from_slice(&pos_ratio.to_le_bytes());
            p.push(0); // reserved
            (EventType::SwitchControl, p)
        }
        KvmFrame::SwitchControlAck => (EventType::SwitchControlAck, vec![]),
        KvmFrame::Heartbeat => (EventType::Heartbeat, vec![]),
        KvmFrame::HeartbeatAck => (EventType::HeartbeatAck, vec![]),
    };

    let payload_len = payload.len() as u32;
    let mut buf = Vec::with_capacity(HEADER_SIZE + payload.len());
    buf.extend_from_slice(&MAGIC.to_le_bytes()); // 2 字节魔数
    buf.push(event_type as u8);                   // 1 字节事件类型
    buf.push(0);                                   // 1 字节 flags（保留）
    buf.extend_from_slice(&payload_len.to_le_bytes()); // 4 字节载荷长度
    buf.extend_from_slice(&payload);
    buf
}

/// 从字节序列解码 KvmFrame
/// 返回 (frame, consumed_bytes) 或错误
pub fn decode(buf: &[u8]) -> Result<(KvmFrame, usize), ProtocolError> {
    if buf.len() < HEADER_SIZE {
        return Err(ProtocolError::Incomplete);
    }

    // 验证魔数
    let magic = u16::from_le_bytes([buf[0], buf[1]]);
    if magic != MAGIC {
        return Err(ProtocolError::InvalidMagic);
    }

    let event_type_byte = buf[2];
    // buf[3] 是 flags，暂时忽略
    let payload_len = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;

    let total_len = HEADER_SIZE + payload_len;
    if buf.len() < total_len {
        return Err(ProtocolError::Incomplete);
    }

    let payload = &buf[HEADER_SIZE..total_len];

    let event_type = EventType::from_u8(event_type_byte)
        .ok_or(ProtocolError::UnknownEventType(event_type_byte))?;

    let frame = match event_type {
        EventType::Handshake => {
            check_payload_len(payload, 4)?;
            let version = u16::from_le_bytes([payload[0], payload[1]]);
            let platform = Platform::from_u8(payload[2]);
            KvmFrame::Handshake { version, platform }
        }
        EventType::HandshakeAck => {
            check_payload_len(payload, 4)?;
            let version = u16::from_le_bytes([payload[0], payload[1]]);
            let platform = Platform::from_u8(payload[2]);
            KvmFrame::HandshakeAck { version, platform }
        }
        EventType::MouseMove => {
            check_payload_len(payload, 8)?;
            let x = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let y = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            KvmFrame::MouseMove { x, y }
        }
        EventType::MouseButton => {
            check_payload_len(payload, 2)?;
            KvmFrame::MouseButton { button: payload[0], pressed: payload[1] != 0 }
        }
        EventType::MouseScroll => {
            check_payload_len(payload, 8)?;
            let dx = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let dy = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            KvmFrame::MouseScroll { dx, dy }
        }
        EventType::KeyEvent => {
            check_payload_len(payload, 3)?;
            let keycode = u16::from_le_bytes([payload[0], payload[1]]);
            KvmFrame::KeyEvent { keycode, pressed: payload[2] != 0 }
        }
        EventType::SwitchControl => {
            check_payload_len(payload, 4)?;
            let edge = ScreenEdge::from_u8(payload[0])
                .ok_or(ProtocolError::UnknownEventType(payload[0]))?;
            let pos_ratio = u16::from_le_bytes([payload[1], payload[2]]);
            KvmFrame::SwitchControl { edge, pos_ratio }
        }
        EventType::SwitchControlAck => KvmFrame::SwitchControlAck,
        EventType::Heartbeat => KvmFrame::Heartbeat,
        EventType::HeartbeatAck => KvmFrame::HeartbeatAck,
    };

    Ok((frame, total_len))
}

fn check_payload_len(payload: &[u8], expected: usize) -> Result<(), ProtocolError> {
    if payload.len() != expected {
        Err(ProtocolError::PayloadLengthMismatch { expected, got: payload.len() })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_handshake() {
        let frame = KvmFrame::Handshake {
            version: PROTOCOL_VERSION,
            platform: Platform::MacOS,
        };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn test_encode_decode_handshake_windows() {
        let frame = KvmFrame::HandshakeAck {
            version: PROTOCOL_VERSION,
            platform: Platform::Windows,
        };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_mouse_move() {
        let frame = KvmFrame::MouseMove { x: 5000, y: -3000 };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, HEADER_SIZE + 8);
    }

    #[test]
    fn test_encode_decode_mouse_move_boundary() {
        let frame = KvmFrame::MouseMove { x: i32::MAX, y: i32::MIN };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_mouse_button() {
        let frame = KvmFrame::MouseButton { button: 0, pressed: true };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, HEADER_SIZE + 2);
    }

    #[test]
    fn test_encode_decode_mouse_scroll() {
        let frame = KvmFrame::MouseScroll { dx: 0, dy: -120 };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_key_event() {
        let frame = KvmFrame::KeyEvent { keycode: 0x0041, pressed: true };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, HEADER_SIZE + 3);
    }

    #[test]
    fn test_encode_decode_key_event_release() {
        let frame = KvmFrame::KeyEvent { keycode: 0x0041, pressed: false };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_switch_control() {
        let frame = KvmFrame::SwitchControl { edge: ScreenEdge::Right, pos_ratio: 5000 };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_heartbeat() {
        let frame = KvmFrame::Heartbeat;
        let bytes = encode(&frame);
        assert_eq!(bytes.len(), HEADER_SIZE);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_little_endian_mouse_move() {
        let frame = KvmFrame::MouseMove { x: 1, y: 2 };
        let bytes = encode(&frame);
        assert_eq!(bytes[HEADER_SIZE], 0x01);
        assert_eq!(bytes[HEADER_SIZE + 1], 0x00);
        assert_eq!(bytes[HEADER_SIZE + 2], 0x00);
        assert_eq!(bytes[HEADER_SIZE + 3], 0x00);
        assert_eq!(bytes[HEADER_SIZE + 4], 0x02);
    }

    #[test]
    fn test_little_endian_key_event() {
        let frame = KvmFrame::KeyEvent { keycode: 0x0102, pressed: true };
        let bytes = encode(&frame);
        assert_eq!(bytes[HEADER_SIZE], 0x02);
        assert_eq!(bytes[HEADER_SIZE + 1], 0x01);
    }

    #[test]
    fn test_decode_incomplete_header() {
        let bytes = vec![0x56, 0x4B];
        assert_eq!(decode(&bytes), Err(ProtocolError::Incomplete));
    }

    #[test]
    fn test_decode_invalid_magic() {
        let mut bytes = encode(&KvmFrame::Heartbeat);
        bytes[0] = 0xFF;
        assert_eq!(decode(&bytes), Err(ProtocolError::InvalidMagic));
    }

    #[test]
    fn test_decode_unknown_event_type() {
        let mut bytes = encode(&KvmFrame::Heartbeat);
        bytes[2] = 0xFF;
        assert_eq!(decode(&bytes), Err(ProtocolError::UnknownEventType(0xFF)));
    }

    #[test]
    fn test_decode_incomplete_payload() {
        let bytes = encode(&KvmFrame::MouseMove { x: 100, y: 200 });
        let truncated = &bytes[..HEADER_SIZE + 4];
        assert_eq!(decode(truncated), Err(ProtocolError::Incomplete));
    }

    #[test]
    fn test_decode_multiple_frames_in_buffer() {
        let frame1 = KvmFrame::MouseMove { x: 100, y: 200 };
        let frame2 = KvmFrame::KeyEvent { keycode: 0x41, pressed: true };
        let frame3 = KvmFrame::Heartbeat;

        let mut buf = encode(&frame1);
        buf.extend(encode(&frame2));
        buf.extend(encode(&frame3));

        let (decoded1, consumed1) = decode(&buf).unwrap();
        assert_eq!(decoded1, frame1);

        let (decoded2, consumed2) = decode(&buf[consumed1..]).unwrap();
        assert_eq!(decoded2, frame2);

        let (decoded3, _) = decode(&buf[consumed1 + consumed2..]).unwrap();
        assert_eq!(decoded3, frame3);
    }

    #[test]
    fn test_screen_edge_opposite() {
        assert_eq!(ScreenEdge::Left.opposite(), ScreenEdge::Right);
        assert_eq!(ScreenEdge::Right.opposite(), ScreenEdge::Left);
        assert_eq!(ScreenEdge::Top.opposite(), ScreenEdge::Bottom);
        assert_eq!(ScreenEdge::Bottom.opposite(), ScreenEdge::Top);
    }

    #[test]
    fn test_magic_bytes() {
        let bytes = encode(&KvmFrame::Heartbeat);
        assert_eq!(bytes[0], 0x56); // 'V'
        assert_eq!(bytes[1], 0x4B); // 'K'
    }
}
