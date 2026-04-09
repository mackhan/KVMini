//! KVMini 事件传输协议
//!
//! 帧格式（固定 8 字节头 + 可变载荷）：
//! ┌─────────────┬──────────┬──────────┬──────────────────────────────┐
//! │ magic(2B)   │ type(1B) │ flags(1B)│ payload_len(4B, little-endian)│
//! ├─────────────┴──────────┴──────────┴──────────────────────────────┤
//! │ payload (payload_len bytes)                                       │
//! └───────────────────────────────────────────────────────────────────┘
//!
//! 从现有两端 protocol.rs 合并而来，消除重复代码。

use crate::error::{KvmError, KvmResult};
use serde::{Deserialize, Serialize};

/// 协议魔数
pub const MAGIC: u16 = 0x4B56; // "KV"

/// 协议版本
pub const PROTOCOL_VERSION: u16 = 2;

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
    /// 切换控制权
    SwitchControl = 0x30,
    /// 切换控制权确认
    SwitchControlAck = 0x31,
    /// 心跳
    Heartbeat = 0x40,
    /// 心跳响应
    HeartbeatAck = 0x41,
    /// 剪贴板内容
    Clipboard = 0x50,
    /// Ping（延迟检测）
    Ping = 0x60,
    /// Pong（延迟检测响应）
    Pong = 0x61,
    /// 密码验证请求
    AuthRequest = 0x70,
    /// 密码验证响应
    AuthResponse = 0x71,
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
            0x70 => Some(Self::AuthRequest),
            0x71 => Some(Self::AuthResponse),
            _ => None,
        }
    }
}

/// 平台标识
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    MacOS = 0x01,
    Windows = 0x02,
    Linux = 0x03,
    Unknown = 0xFF,
}

impl Platform {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// 返回对应的反向边缘
    pub fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

/// 修饰键状态位图
///
/// 每个修饰键占 1 bit，用于在事件帧中携带修饰键状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierState {
    bits: u8,
}

impl ModifierState {
    pub const CTRL: u8 = 0b0001;
    pub const SHIFT: u8 = 0b0010;
    pub const ALT: u8 = 0b0100;
    pub const CMD_WIN: u8 = 0b1000;

    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn from_bits(bits: u8) -> Self {
        Self { bits: bits & 0x0F }
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }

    pub fn set(&mut self, modifier: u8, pressed: bool) {
        if pressed {
            self.bits |= modifier;
        } else {
            self.bits &= !modifier;
        }
    }

    pub fn is_set(&self, modifier: u8) -> bool {
        (self.bits & modifier) != 0
    }

    pub fn clear(&mut self) {
        self.bits = 0;
    }

    /// 返回所有处于按下状态的修饰键列表
    pub fn pressed_modifiers(&self) -> Vec<u8> {
        let mut result = Vec::new();
        if self.is_set(Self::CTRL) { result.push(Self::CTRL); }
        if self.is_set(Self::SHIFT) { result.push(Self::SHIFT); }
        if self.is_set(Self::ALT) { result.push(Self::ALT); }
        if self.is_set(Self::CMD_WIN) { result.push(Self::CMD_WIN); }
        result
    }
}

/// 协议帧（解码后的结构）
#[derive(Debug, Clone, PartialEq)]
pub enum KvmFrame {
    /// 握手：(version, platform, screen_info_json)
    Handshake {
        version: u16,
        platform: Platform,
        screen_info: Vec<u8>,
    },
    /// 握手响应
    HandshakeAck {
        version: u16,
        platform: Platform,
        screen_info: Vec<u8>,
    },
    /// 鼠标移动：归一化坐标 [0, 10000]，携带时间戳
    MouseMove {
        x: i32,
        y: i32,
        timestamp_ms: u64,
        modifiers: ModifierState,
    },
    /// 鼠标按键
    MouseButton {
        button: u8,
        pressed: bool,
        timestamp_ms: u64,
        modifiers: ModifierState,
    },
    /// 滚轮
    MouseScroll {
        dx: i32,
        dy: i32,
        timestamp_ms: u64,
        modifiers: ModifierState,
    },
    /// 键盘事件（跨平台统一键码）
    KeyEvent {
        keycode: u16,
        pressed: bool,
        timestamp_ms: u64,
        modifiers: ModifierState,
    },
    /// 切换控制权
    SwitchControl {
        edge: ScreenEdge,
        pos_ratio: u16,
    },
    /// 切换控制权确认
    SwitchControlAck,
    /// 心跳
    Heartbeat,
    /// 心跳响应
    HeartbeatAck,
    /// 剪贴板内容（UTF-8 文本）
    Clipboard { text: String },
    /// Ping
    Ping { timestamp_ms: u64 },
    /// Pong
    Pong { timestamp_ms: u64 },
    /// 密码验证请求（SHA-256 哈希）
    AuthRequest { password_hash: Vec<u8> },
    /// 密码验证响应
    AuthResponse { success: bool },
}

/// 将 KvmFrame 编码为字节序列（小端序）
pub fn encode(frame: &KvmFrame) -> Vec<u8> {
    let (event_type, payload) = match frame {
        KvmFrame::Handshake { version, platform, screen_info } => {
            let mut p = Vec::with_capacity(4 + screen_info.len());
            p.extend_from_slice(&version.to_le_bytes());
            p.push(*platform as u8);
            p.push(0); // reserved
            p.extend_from_slice(screen_info);
            (EventType::Handshake, p)
        }
        KvmFrame::HandshakeAck { version, platform, screen_info } => {
            let mut p = Vec::with_capacity(4 + screen_info.len());
            p.extend_from_slice(&version.to_le_bytes());
            p.push(*platform as u8);
            p.push(0);
            p.extend_from_slice(screen_info);
            (EventType::HandshakeAck, p)
        }
        KvmFrame::MouseMove { x, y, timestamp_ms, modifiers } => {
            let mut p = Vec::with_capacity(17);
            p.extend_from_slice(&x.to_le_bytes());
            p.extend_from_slice(&y.to_le_bytes());
            p.extend_from_slice(&timestamp_ms.to_le_bytes());
            p.push(modifiers.bits());
            (EventType::MouseMove, p)
        }
        KvmFrame::MouseButton { button, pressed, timestamp_ms, modifiers } => {
            let mut p = Vec::with_capacity(11);
            p.push(*button);
            p.push(if *pressed { 1 } else { 0 });
            p.extend_from_slice(&timestamp_ms.to_le_bytes());
            p.push(modifiers.bits());
            (EventType::MouseButton, p)
        }
        KvmFrame::MouseScroll { dx, dy, timestamp_ms, modifiers } => {
            let mut p = Vec::with_capacity(17);
            p.extend_from_slice(&dx.to_le_bytes());
            p.extend_from_slice(&dy.to_le_bytes());
            p.extend_from_slice(&timestamp_ms.to_le_bytes());
            p.push(modifiers.bits());
            (EventType::MouseScroll, p)
        }
        KvmFrame::KeyEvent { keycode, pressed, timestamp_ms, modifiers } => {
            let mut p = Vec::with_capacity(12);
            p.extend_from_slice(&keycode.to_le_bytes());
            p.push(if *pressed { 1 } else { 0 });
            p.extend_from_slice(&timestamp_ms.to_le_bytes());
            p.push(modifiers.bits());
            (EventType::KeyEvent, p)
        }
        KvmFrame::SwitchControl { edge, pos_ratio } => {
            let mut p = Vec::with_capacity(4);
            p.push(*edge as u8);
            p.extend_from_slice(&pos_ratio.to_le_bytes());
            p.push(0);
            (EventType::SwitchControl, p)
        }
        KvmFrame::SwitchControlAck => (EventType::SwitchControlAck, vec![]),
        KvmFrame::Heartbeat => (EventType::Heartbeat, vec![]),
        KvmFrame::HeartbeatAck => (EventType::HeartbeatAck, vec![]),
        KvmFrame::Clipboard { text } => {
            (EventType::Clipboard, text.as_bytes().to_vec())
        }
        KvmFrame::Ping { timestamp_ms } => {
            (EventType::Ping, timestamp_ms.to_le_bytes().to_vec())
        }
        KvmFrame::Pong { timestamp_ms } => {
            (EventType::Pong, timestamp_ms.to_le_bytes().to_vec())
        }
        KvmFrame::AuthRequest { password_hash } => {
            (EventType::AuthRequest, password_hash.clone())
        }
        KvmFrame::AuthResponse { success } => {
            (EventType::AuthResponse, vec![if *success { 1 } else { 0 }])
        }
    };

    let payload_len = payload.len() as u32;
    let mut buf = Vec::with_capacity(HEADER_SIZE + payload.len());
    buf.extend_from_slice(&MAGIC.to_le_bytes());
    buf.push(event_type as u8);
    buf.push(0); // flags
    buf.extend_from_slice(&payload_len.to_le_bytes());
    buf.extend_from_slice(&payload);
    buf
}

/// 从字节序列解码 KvmFrame
/// 返回 (frame, consumed_bytes) 或错误
pub fn decode(buf: &[u8]) -> KvmResult<(KvmFrame, usize)> {
    if buf.len() < HEADER_SIZE {
        return Err(KvmError::Incomplete);
    }

    let magic = u16::from_le_bytes([buf[0], buf[1]]);
    if magic != MAGIC {
        return Err(KvmError::InvalidMagic);
    }

    let event_type_byte = buf[2];
    let payload_len = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;

    let total_len = HEADER_SIZE + payload_len;
    if buf.len() < total_len {
        return Err(KvmError::Incomplete);
    }

    let payload = &buf[HEADER_SIZE..total_len];

    let event_type = EventType::from_u8(event_type_byte)
        .ok_or(KvmError::UnknownEventType { event_type: event_type_byte })?;

    let frame = match event_type {
        EventType::Handshake => {
            if payload.len() < 4 {
                return Err(KvmError::PayloadLengthMismatch { expected: 4, got: payload.len() });
            }
            let version = u16::from_le_bytes([payload[0], payload[1]]);
            let platform = Platform::from_u8(payload[2]);
            let screen_info = payload[4..].to_vec();
            KvmFrame::Handshake { version, platform, screen_info }
        }
        EventType::HandshakeAck => {
            if payload.len() < 4 {
                return Err(KvmError::PayloadLengthMismatch { expected: 4, got: payload.len() });
            }
            let version = u16::from_le_bytes([payload[0], payload[1]]);
            let platform = Platform::from_u8(payload[2]);
            let screen_info = payload[4..].to_vec();
            KvmFrame::HandshakeAck { version, platform, screen_info }
        }
        EventType::MouseMove => {
            check_min_payload(payload, 17)?;
            let x = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let y = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            let timestamp_ms = u64::from_le_bytes([
                payload[8], payload[9], payload[10], payload[11],
                payload[12], payload[13], payload[14], payload[15],
            ]);
            let modifiers = ModifierState::from_bits(payload[16]);
            KvmFrame::MouseMove { x, y, timestamp_ms, modifiers }
        }
        EventType::MouseButton => {
            check_min_payload(payload, 11)?;
            let button = payload[0];
            let pressed = payload[1] != 0;
            let timestamp_ms = u64::from_le_bytes([
                payload[2], payload[3], payload[4], payload[5],
                payload[6], payload[7], payload[8], payload[9],
            ]);
            let modifiers = ModifierState::from_bits(payload[10]);
            KvmFrame::MouseButton { button, pressed, timestamp_ms, modifiers }
        }
        EventType::MouseScroll => {
            check_min_payload(payload, 17)?;
            let dx = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let dy = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            let timestamp_ms = u64::from_le_bytes([
                payload[8], payload[9], payload[10], payload[11],
                payload[12], payload[13], payload[14], payload[15],
            ]);
            let modifiers = ModifierState::from_bits(payload[16]);
            KvmFrame::MouseScroll { dx, dy, timestamp_ms, modifiers }
        }
        EventType::KeyEvent => {
            check_min_payload(payload, 12)?;
            let keycode = u16::from_le_bytes([payload[0], payload[1]]);
            let pressed = payload[2] != 0;
            let timestamp_ms = u64::from_le_bytes([
                payload[3], payload[4], payload[5], payload[6],
                payload[7], payload[8], payload[9], payload[10],
            ]);
            let modifiers = ModifierState::from_bits(payload[11]);
            KvmFrame::KeyEvent { keycode, pressed, timestamp_ms, modifiers }
        }
        EventType::SwitchControl => {
            check_min_payload(payload, 4)?;
            let edge = ScreenEdge::from_u8(payload[0])
                .ok_or(KvmError::UnknownEventType { event_type: payload[0] })?;
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
            check_min_payload(payload, 8)?;
            let timestamp_ms = u64::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
                payload[4], payload[5], payload[6], payload[7],
            ]);
            KvmFrame::Ping { timestamp_ms }
        }
        EventType::Pong => {
            check_min_payload(payload, 8)?;
            let timestamp_ms = u64::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
                payload[4], payload[5], payload[6], payload[7],
            ]);
            KvmFrame::Pong { timestamp_ms }
        }
        EventType::AuthRequest => {
            KvmFrame::AuthRequest { password_hash: payload.to_vec() }
        }
        EventType::AuthResponse => {
            check_min_payload(payload, 1)?;
            KvmFrame::AuthResponse { success: payload[0] != 0 }
        }
    };

    Ok((frame, total_len))
}

fn check_min_payload(payload: &[u8], min: usize) -> KvmResult<()> {
    if payload.len() < min {
        Err(KvmError::PayloadLengthMismatch { expected: min, got: payload.len() })
    } else {
        Ok(())
    }
}

/// 从 TCP 流中异步读取一个完整帧
pub async fn read_frame_async<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
) -> KvmResult<KvmFrame> {
    let mut header = [0u8; HEADER_SIZE];
    reader.read_exact(&mut header).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            KvmError::Disconnected
        } else {
            KvmError::Io(e)
        }
    })?;

    let magic = u16::from_le_bytes([header[0], header[1]]);
    if magic != MAGIC {
        return Err(KvmError::InvalidMagic);
    }

    let payload_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    let mut buf = vec![0u8; HEADER_SIZE + payload_len];
    buf[..HEADER_SIZE].copy_from_slice(&header);

    if payload_len > 0 {
        reader.read_exact(&mut buf[HEADER_SIZE..]).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                KvmError::Disconnected
            } else {
                KvmError::Io(e)
            }
        })?;
    }

    let (frame, _) = decode(&buf)?;
    Ok(frame)
}

/// 向 TCP 流异步写入一个帧
pub async fn write_frame_async<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    frame: &KvmFrame,
) -> KvmResult<()> {
    let bytes = encode(frame);
    writer.write_all(&bytes).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            KvmError::Disconnected
        } else {
            KvmError::Io(e)
        }
    })?;
    Ok(())
}

/// 从 TCP 流中同步读取一个完整帧
pub fn read_frame(stream: &mut impl std::io::Read) -> KvmResult<KvmFrame> {
    let mut header = [0u8; HEADER_SIZE];
    stream.read_exact(&mut header)?;

    let magic = u16::from_le_bytes([header[0], header[1]]);
    if magic != MAGIC {
        return Err(KvmError::InvalidMagic);
    }

    let payload_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    let mut buf = vec![0u8; HEADER_SIZE + payload_len];
    buf[..HEADER_SIZE].copy_from_slice(&header);

    if payload_len > 0 {
        stream.read_exact(&mut buf[HEADER_SIZE..])?;
    }

    let (frame, _) = decode(&buf)?;
    Ok(frame)
}

/// 向 TCP 流同步写入一个帧
pub fn write_frame(stream: &mut impl std::io::Write, frame: &KvmFrame) -> KvmResult<()> {
    let bytes = encode(frame);
    stream.write_all(&bytes)?;
    Ok(())
}

// ── 键码映射 ────────────────────────────────────────────────────────────────

pub mod keycode_map;
pub mod event_queue;

#[cfg(test)]
mod tests {
    use super::*;

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[test]
    fn test_encode_decode_handshake() {
        let frame = KvmFrame::Handshake {
            version: PROTOCOL_VERSION,
            platform: Platform::MacOS,
            screen_info: vec![],
        };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn test_encode_decode_mouse_move() {
        let ts = now_ms();
        let frame = KvmFrame::MouseMove {
            x: 5000, y: 3000,
            timestamp_ms: ts,
            modifiers: ModifierState::from_bits(0b0101),
        };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_key_event() {
        let ts = now_ms();
        let frame = KvmFrame::KeyEvent {
            keycode: 0x0041, pressed: true,
            timestamp_ms: ts,
            modifiers: ModifierState::from_bits(0b0001),
        };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_switch_control() {
        let frame = KvmFrame::SwitchControl {
            edge: ScreenEdge::Right, pos_ratio: 5000,
        };
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
    fn test_encode_decode_clipboard() {
        let frame = KvmFrame::Clipboard { text: "Hello 你好".to_string() };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_encode_decode_auth() {
        let frame = KvmFrame::AuthRequest { password_hash: vec![1, 2, 3, 4] };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);

        let frame = KvmFrame::AuthResponse { success: true };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_decode_incomplete() {
        let bytes = vec![0x56, 0x4B];
        assert!(matches!(decode(&bytes), Err(KvmError::Incomplete)));
    }

    #[test]
    fn test_decode_invalid_magic() {
        let mut bytes = encode(&KvmFrame::Heartbeat);
        bytes[0] = 0xFF;
        assert!(matches!(decode(&bytes), Err(KvmError::InvalidMagic)));
    }

    #[test]
    fn test_decode_unknown_event_type() {
        let mut bytes = encode(&KvmFrame::Heartbeat);
        bytes[2] = 0xFF;
        assert!(matches!(decode(&bytes), Err(KvmError::UnknownEventType { .. })));
    }

    #[test]
    fn test_screen_edge_opposite() {
        assert_eq!(ScreenEdge::Left.opposite(), ScreenEdge::Right);
        assert_eq!(ScreenEdge::Right.opposite(), ScreenEdge::Left);
        assert_eq!(ScreenEdge::Top.opposite(), ScreenEdge::Bottom);
        assert_eq!(ScreenEdge::Bottom.opposite(), ScreenEdge::Top);
    }

    #[test]
    fn test_modifier_state() {
        let mut m = ModifierState::new();
        assert_eq!(m.bits(), 0);

        m.set(ModifierState::CTRL, true);
        assert!(m.is_set(ModifierState::CTRL));
        assert!(!m.is_set(ModifierState::SHIFT));

        m.set(ModifierState::SHIFT, true);
        assert!(m.is_set(ModifierState::SHIFT));

        let pressed = m.pressed_modifiers();
        assert_eq!(pressed.len(), 2);

        m.set(ModifierState::CTRL, false);
        assert!(!m.is_set(ModifierState::CTRL));

        m.clear();
        assert_eq!(m.bits(), 0);
    }

    #[test]
    fn test_multiple_frames_in_buffer() {
        let ts = now_ms();
        let frame1 = KvmFrame::MouseMove {
            x: 100, y: 200, timestamp_ms: ts,
            modifiers: ModifierState::new(),
        };
        let frame2 = KvmFrame::KeyEvent {
            keycode: 0x41, pressed: true, timestamp_ms: ts,
            modifiers: ModifierState::new(),
        };
        let frame3 = KvmFrame::Heartbeat;

        let mut buf = encode(&frame1);
        buf.extend(encode(&frame2));
        buf.extend(encode(&frame3));

        let (d1, c1) = decode(&buf).unwrap();
        assert_eq!(d1, frame1);

        let (d2, c2) = decode(&buf[c1..]).unwrap();
        assert_eq!(d2, frame2);

        let (d3, _) = decode(&buf[c1 + c2..]).unwrap();
        assert_eq!(d3, frame3);
    }

    #[test]
    fn test_magic_bytes() {
        let bytes = encode(&KvmFrame::Heartbeat);
        assert_eq!(bytes[0], 0x56); // 'V'
        assert_eq!(bytes[1], 0x4B); // 'K'
    }
}
