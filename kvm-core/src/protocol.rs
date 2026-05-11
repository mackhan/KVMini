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
    /// 控制权回切（远端撞反向边缘通知本机）
    SwitchBackControl = 0x32,
    /// 鼠标相对位移（v0.1.40 delta 模式：dx/dy i16，避免 grab 后绝对坐标冻结）
    MouseMoveDelta = 0x33,
    /// v0.1.57: 远端日志同步（Mac 被控端推送日志行到 Win 主控端）
    PeerLog = 0x80,
}

impl EventType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Handshake),
            0x02 => Some(Self::HandshakeAck),
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
            0x32 => Some(Self::SwitchBackControl),
            0x33 => Some(Self::MouseMoveDelta),
            0x80 => Some(Self::PeerLog),
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

/// 控制权回切的原因
///
/// v0.1.43 引入：区分"远端撞到反向边缘的自然回切"与"远端用户物理输入抢占"。
/// 协议载荷为 1 字节；payload 为空时按 `EdgeReturn` 解码，保证对 v0.1.42 及更老版本的向后兼容。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwitchBackReason {
    /// 远端虚拟光标撞到反向边缘，自然回切
    EdgeReturn = 0x00,
    /// 远端用户物理输入抢占（Synergy/Logitech Flow 风格）
    Preempt = 0x01,
}

impl SwitchBackReason {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x01 => Self::Preempt,
            _ => Self::EdgeReturn, // 未知值按兼容策略退化为 EdgeReturn
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
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
    /// 控制权回切（远端撞到反向边缘或物理输入抢占，通知本机从 LocalControlling 切回 Armed）
    SwitchBackControl { reason: SwitchBackReason },
    /// 鼠标相对位移（v0.1.40 delta 模式）
    ///
    /// 替代 `MouseMove{x,y}` 的绝对坐标路径：采集端从 CGEvent `kCGMouseEventDeltaX/Y`
    /// 读取硬件相对位移，远端用 `RemoteCursorState` 累加后 `CGWarpMouseCursorPosition` 落点。
    /// 这是 Synergy/Barrier/ShareMouse 等竞品的标准方案，不会因 `CGAssociateMouseAndMouseCursorPosition(0)` 冻结。
    MouseMoveDelta { dx: i16, dy: i16 },
    /// v0.1.57: 远端日志推送
    ///
    /// Mac 被控端通过此帧将运行时日志行实时推送到 Win 主控端，
    /// Win 端落盘到 `%LOCALAPPDATA%\KVMini\Logs\peer-mac\runtime.log`。
    /// level: 0=trace 1=debug 2=info 3=warn 4=error
    PeerLog { level: u8, message: String },
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
        KvmFrame::SwitchBackControl { reason } => {
            (EventType::SwitchBackControl, vec![reason.as_u8()])
        }
        KvmFrame::MouseMoveDelta { dx, dy } => {
            let mut p = Vec::with_capacity(4);
            p.extend_from_slice(&dx.to_le_bytes());
            p.extend_from_slice(&dy.to_le_bytes());
            (EventType::MouseMoveDelta, p)
        }
        KvmFrame::PeerLog { level, message } => {
            let msg_bytes = message.as_bytes();
            let mut p = Vec::with_capacity(1 + msg_bytes.len());
            p.push(*level);
            p.extend_from_slice(msg_bytes);
            (EventType::PeerLog, p)
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
        EventType::SwitchBackControl => {
            // 向后兼容：v0.1.42 及更老版本 payload 为空 → 默认 EdgeReturn
            let reason = if payload.is_empty() {
                SwitchBackReason::EdgeReturn
            } else {
                SwitchBackReason::from_u8(payload[0])
            };
            KvmFrame::SwitchBackControl { reason }
        }
        EventType::MouseMoveDelta => {
            check_min_payload(payload, 4)?;
            let dx = i16::from_le_bytes([payload[0], payload[1]]);
            let dy = i16::from_le_bytes([payload[2], payload[3]]);
            KvmFrame::MouseMoveDelta { dx, dy }
        }
        EventType::PeerLog => {
            check_min_payload(payload, 1)?;
            let level = payload[0];
            let message = String::from_utf8_lossy(&payload[1..]).to_string();
            KvmFrame::PeerLog { level, message }
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
    fn test_encode_decode_mouse_move_delta_legacy() {
        // 旧 test_encode_decode_mouse_move 已移除（KvmFrame::MouseMove 已在 v0.1.40 删除）
        // 该用例已迁移到 test_encode_decode_mouse_move_delta（在文件下方），此处仅保留命名兼容
        let frame = KvmFrame::MouseMoveDelta { dx: 5, dy: -3 };
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
        let frame1 = KvmFrame::MouseMoveDelta { dx: 100, dy: 200 };
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

    #[test]
    fn test_encode_decode_switch_back_control() {
        // v0.1.43: payload = 1 字节 reason
        let frame = KvmFrame::SwitchBackControl {
            reason: SwitchBackReason::EdgeReturn,
        };
        let bytes = encode(&frame);
        assert_eq!(bytes.len(), HEADER_SIZE + 1);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, bytes.len());
        // 验证 event_type 字节正确
        assert_eq!(bytes[2], 0x32);
        // reason 字节
        assert_eq!(bytes[HEADER_SIZE], 0x00);
    }

    #[test]
    fn test_switch_back_reason_encode_decode_roundtrip() {
        // EdgeReturn 和 Preempt 都能正确 roundtrip
        for reason in [SwitchBackReason::EdgeReturn, SwitchBackReason::Preempt] {
            let frame = KvmFrame::SwitchBackControl { reason };
            let bytes = encode(&frame);
            let (decoded, _) = decode(&bytes).unwrap();
            assert_eq!(decoded, frame);
        }
    }

    #[test]
    fn test_switch_back_preempt_byte_value() {
        let frame = KvmFrame::SwitchBackControl {
            reason: SwitchBackReason::Preempt,
        };
        let bytes = encode(&frame);
        assert_eq!(bytes[HEADER_SIZE], 0x01);
    }

    #[test]
    fn test_switch_back_legacy_empty_payload_compat() {
        // 手动构造一个 v0.1.42 风格的 SwitchBackControl 帧（0 字节 payload）
        let mut bytes = Vec::with_capacity(HEADER_SIZE);
        bytes.extend_from_slice(&MAGIC.to_le_bytes());
        bytes.push(EventType::SwitchBackControl as u8);
        bytes.push(0); // flags
        bytes.extend_from_slice(&0u32.to_le_bytes()); // payload_len = 0
        assert_eq!(bytes.len(), HEADER_SIZE);

        // 解码应退化为 EdgeReturn，保证对 v0.1.42 及更老版本的兼容
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(
            decoded,
            KvmFrame::SwitchBackControl {
                reason: SwitchBackReason::EdgeReturn,
            }
        );
        assert_eq!(consumed, HEADER_SIZE);
    }

    #[test]
    fn test_switch_back_reason_from_u8_unknown_falls_back_to_edge_return() {
        // 未来若出现未知 reason 值（如新客户端给老客户端发 0x7F），
        // 退化为 EdgeReturn 是安全默认（不会破坏 LocalControlling→Armed 回切主流程）
        assert_eq!(SwitchBackReason::from_u8(0x00), SwitchBackReason::EdgeReturn);
        assert_eq!(SwitchBackReason::from_u8(0x01), SwitchBackReason::Preempt);
        assert_eq!(SwitchBackReason::from_u8(0x7F), SwitchBackReason::EdgeReturn);
        assert_eq!(SwitchBackReason::from_u8(0xFF), SwitchBackReason::EdgeReturn);
    }

    // ── v0.1.40 鼠标相对位移协议（delta 模式）──────────────────────────

    #[test]
    fn test_encode_decode_mouse_move_delta_basic() {
        let frame = KvmFrame::MouseMoveDelta { dx: 5, dy: -3 };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, bytes.len());
        // event_type 字节应为 0x33
        assert_eq!(bytes[2], 0x33);
    }

    #[test]
    fn test_mouse_move_delta_payload_size() {
        // dx (i16) + dy (i16) = 4 bytes
        let frame = KvmFrame::MouseMoveDelta { dx: 0, dy: 0 };
        let bytes = encode(&frame);
        assert_eq!(bytes.len(), HEADER_SIZE + 4);
    }

    #[test]
    fn test_mouse_move_delta_extreme_values() {
        // 边界：i16::MAX / i16::MIN 都能正确 roundtrip
        let frame = KvmFrame::MouseMoveDelta {
            dx: i16::MAX,
            dy: i16::MIN,
        };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);

        let frame = KvmFrame::MouseMoveDelta {
            dx: i16::MIN,
            dy: i16::MAX,
        };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_mouse_move_delta_little_endian() {
        let frame = KvmFrame::MouseMoveDelta { dx: 0x0102, dy: 0x0304 };
        let bytes = encode(&frame);
        // payload = [02, 01, 04, 03]（小端序）
        assert_eq!(&bytes[HEADER_SIZE..HEADER_SIZE + 4], &[0x02, 0x01, 0x04, 0x03]);
    }

    #[test]
    fn test_mouse_move_delta_event_type_opcode() {
        // 确认 EventType::MouseMoveDelta 枚举正确映射到 0x33
        assert_eq!(EventType::MouseMoveDelta as u8, 0x33);
        assert_eq!(EventType::from_u8(0x33), Some(EventType::MouseMoveDelta));
    }

    // ── v0.1.57 远端日志同步（PeerLog）─────────────────────────────────────

    #[test]
    fn test_encode_decode_peer_log_roundtrip() {
        let frame = KvmFrame::PeerLog {
            level: 2, // info
            message: "[Mac/Inject] scroll raw(dx=0 dy=120) → lines(x=0 y=1)".to_string(),
        };
        let bytes = encode(&frame);
        let (decoded, consumed) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(consumed, bytes.len());
        // event_type 字节应为 0x80
        assert_eq!(bytes[2], 0x80);
    }

    #[test]
    fn test_peer_log_level_preserved() {
        for level in [0u8, 1, 2, 3, 4] {
            let frame = KvmFrame::PeerLog { level, message: "test".to_string() };
            let bytes = encode(&frame);
            let (decoded, _) = decode(&bytes).unwrap();
            assert_eq!(decoded, frame);
        }
    }

    #[test]
    fn test_peer_log_empty_message() {
        let frame = KvmFrame::PeerLog { level: 3, message: String::new() };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_peer_log_unicode_message() {
        let frame = KvmFrame::PeerLog {
            level: 2,
            message: "注入滚轮 dy=120 lines=1".to_string(),
        };
        let bytes = encode(&frame);
        let (decoded, _) = decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
    }
}
