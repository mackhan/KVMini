/// KVMini 事件传输协议（与 src-tauri 端完全一致）
///
/// 帧格式（固定 8 字节头 + 可变载荷）：
/// ┌─────────────┬──────────┬──────────┬──────────────────────────────┐
/// │ magic(2B)   │ type(1B) │ flags(1B)│ payload_len(4B, little-endian)│
/// ├─────────────┴──────────┴──────────┴──────────────────────────────┤
/// │ payload (payload_len bytes)                                       │
/// └───────────────────────────────────────────────────────────────────┘

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
    /// 剪贴板内容
    Clipboard = 0x50,
    /// Ping（延迟检测，携带发送时间戳 ms）
    Ping = 0x60,
    /// Pong（延迟检测响应，回传原始时间戳）
    Pong = 0x61,
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
            0x50 => Some(Self::Clipboard),
            0x60 => Some(Self::Ping),
            0x61 => Some(Self::Pong),
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
    /// 剪贴板内容（UTF-8 文本）
    Clipboard { text: String },
    /// Ping：携带发送时间戳（毫秒，u64 小端序）
    Ping { timestamp_ms: u64 },
    /// Pong：回传原始时间戳（毫秒，u64 小端序）
    Pong { timestamp_ms: u64 },
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
        KvmFrame::Clipboard { text } => {
            let p = text.as_bytes().to_vec();
            (EventType::Clipboard, p)
        }
        KvmFrame::Ping { timestamp_ms } => {
            (EventType::Ping, timestamp_ms.to_le_bytes().to_vec())
        }
        KvmFrame::Pong { timestamp_ms } => {
            (EventType::Pong, timestamp_ms.to_le_bytes().to_vec())
        }
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
        EventType::Clipboard => {
            let text = String::from_utf8_lossy(payload).to_string();
            KvmFrame::Clipboard { text }
        }
        EventType::Ping => {
            check_payload_len(payload, 8)?;
            let timestamp_ms = u64::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
                payload[4], payload[5], payload[6], payload[7],
            ]);
            KvmFrame::Ping { timestamp_ms }
        }
        EventType::Pong => {
            check_payload_len(payload, 8)?;
            let timestamp_ms = u64::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
                payload[4], payload[5], payload[6], payload[7],
            ]);
            KvmFrame::Pong { timestamp_ms }
        }
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

/// 从 TCP 流中读取一个完整帧（同步阻塞版本）
pub fn read_frame(stream: &mut impl std::io::Read) -> Result<KvmFrame, String> {
    let mut header = [0u8; HEADER_SIZE];
    stream.read_exact(&mut header)
        .map_err(|e| format!("读取帧头失败: {}", e))?;

    // 验证魔数
    let magic = u16::from_le_bytes([header[0], header[1]]);
    if magic != MAGIC {
        return Err(format!("魔数错误: 0x{:04X}", magic));
    }

    let payload_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    let mut buf = vec![0u8; HEADER_SIZE + payload_len];
    buf[..HEADER_SIZE].copy_from_slice(&header);

    if payload_len > 0 {
        stream.read_exact(&mut buf[HEADER_SIZE..])
            .map_err(|e| format!("读取帧载荷失败: {}", e))?;
    }

    let (frame, _) = decode(&buf)
        .map_err(|e| format!("帧解码失败: {}", e))?;
    Ok(frame)
}

/// 向 TCP 流写入一个帧（同步阻塞版本）
pub fn write_frame(stream: &mut impl std::io::Write, frame: &KvmFrame) -> Result<(), String> {
    let bytes = encode(frame);
    stream.write_all(&bytes)
        .map_err(|e| format!("写入帧失败: {}", e))?;
    Ok(())
}
