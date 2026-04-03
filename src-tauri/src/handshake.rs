/// KVMini 握手协议模块
///
/// 负责 TCP 连接建立后的握手验证，确保对端确实运行着 KVMini 服务。
/// 此模块是跨平台公共逻辑，不包含任何平台特定代码。
///
/// 握手流程：
/// 1. 客户端发送 `Handshake` 帧（含协议版本和平台标识）
/// 2. 服务端响应 `HandshakeAck` 帧（含协议版本和平台标识）
/// 3. 客户端验证版本是否兼容
///
/// 超时：3 秒内无响应视为握手失败

use std::io::{Read, Write};
use std::time::Duration;

use crate::protocol::{self, KvmFrame, Platform, PROTOCOL_VERSION};

/// 握手错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum HandshakeError {
    /// 对端不是 KVMini 服务（魔数错误或协议不匹配）
    NotKvmService,
    /// 协议版本不兼容
    VersionMismatch { local: u16, remote: u16 },
    /// 握手超时（3 秒内无响应）
    Timeout,
    /// IO 错误
    IoError(String),
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotKvmService => write!(f, "连接失败：对端不是 KVMini 服务"),
            Self::VersionMismatch { local, remote } => write!(
                f,
                "版本不兼容：本端版本 {}，对端版本 {}，请确保两端使用相同版本的 KVM Share",
                local, remote
            ),
            Self::Timeout => write!(f, "握手超时：对端可能不是 KVMini 服务"),
            Self::IoError(msg) => write!(f, "连接错误：{}", msg),
        }
    }
}

/// 握手结果（成功时包含对端平台信息）
#[derive(Debug, Clone, PartialEq)]
pub struct HandshakeResult {
    /// 对端协议版本
    pub remote_version: u16,
    /// 对端平台
    pub remote_platform: Platform,
}

/// 执行客户端握手（主控端发起连接时调用）
///
/// 向对端发送 `Handshake` 帧，等待 `HandshakeAck` 响应（超时 3 秒）。
/// 验证协议版本兼容性。
///
/// # 参数
/// - `stream`: 已建立的 TCP 连接
///
/// # 返回
/// - `Ok(HandshakeResult)`: 握手成功，包含对端信息
/// - `Err(HandshakeError)`: 握手失败，包含具体原因
pub fn perform_client_handshake<S>(stream: &mut S) -> Result<HandshakeResult, HandshakeError>
where
    S: Read + Write,
{
    // 1. 发送 Handshake 帧
    let handshake_frame = KvmFrame::Handshake {
        version: PROTOCOL_VERSION,
        platform: Platform::current(),
    };
    let bytes = protocol::encode(&handshake_frame);
    stream.write_all(&bytes).map_err(|e| HandshakeError::IoError(e.to_string()))?;

    // 2. 读取响应（最多读取 256 字节，足够容纳 HandshakeAck 帧）
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
            HandshakeError::Timeout
        } else {
            HandshakeError::IoError(e.to_string())
        }
    })?;

    if n == 0 {
        return Err(HandshakeError::NotKvmService);
    }

    // 3. 解码响应帧
    let (frame, _) = protocol::decode(&buf[..n]).map_err(|e| {
        match e {
            protocol::ProtocolError::InvalidMagic => HandshakeError::NotKvmService,
            _ => HandshakeError::NotKvmService,
        }
    })?;

    // 4. 验证响应类型
    match frame {
        KvmFrame::HandshakeAck { version, platform } => {
            // 5. 验证版本兼容性
            if version != PROTOCOL_VERSION {
                return Err(HandshakeError::VersionMismatch {
                    local: PROTOCOL_VERSION,
                    remote: version,
                });
            }
            Ok(HandshakeResult {
                remote_version: version,
                remote_platform: platform,
            })
        }
        _ => Err(HandshakeError::NotKvmService),
    }
}

/// 执行服务端握手（被控端接受连接时调用）
///
/// 等待客户端发送 `Handshake` 帧，验证后响应 `HandshakeAck`。
///
/// # 参数
/// - `stream`: 已建立的 TCP 连接
///
/// # 返回
/// - `Ok(HandshakeResult)`: 握手成功，包含对端信息
/// - `Err(HandshakeError)`: 握手失败
pub fn perform_server_handshake<S>(stream: &mut S) -> Result<HandshakeResult, HandshakeError>
where
    S: Read + Write,
{
    // 1. 读取客户端的 Handshake 帧
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
            HandshakeError::Timeout
        } else {
            HandshakeError::IoError(e.to_string())
        }
    })?;

    if n == 0 {
        return Err(HandshakeError::NotKvmService);
    }

    // 2. 解码 Handshake 帧
    let (frame, _) = protocol::decode(&buf[..n]).map_err(|_| HandshakeError::NotKvmService)?;

    match frame {
        KvmFrame::Handshake { version, platform } => {
            // 3. 验证版本兼容性
            if version != PROTOCOL_VERSION {
                return Err(HandshakeError::VersionMismatch {
                    local: PROTOCOL_VERSION,
                    remote: version,
                });
            }

            // 4. 发送 HandshakeAck 响应
            let ack_frame = KvmFrame::HandshakeAck {
                version: PROTOCOL_VERSION,
                platform: Platform::current(),
            };
            let ack_bytes = protocol::encode(&ack_frame);
            stream.write_all(&ack_bytes).map_err(|e| HandshakeError::IoError(e.to_string()))?;

            Ok(HandshakeResult {
                remote_version: version,
                remote_platform: platform,
            })
        }
        _ => Err(HandshakeError::NotKvmService),
    }
}

/// 设置 TCP stream 的读写超时（3 秒）
pub fn set_handshake_timeout(stream: &std::net::TcpStream) -> Result<(), HandshakeError> {
    let timeout = Duration::from_secs(3);
    stream.set_read_timeout(Some(timeout))
        .map_err(|e| HandshakeError::IoError(e.to_string()))?;
    stream.set_write_timeout(Some(timeout))
        .map_err(|e| HandshakeError::IoError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ── 辅助函数：创建模拟的握手响应数据 ────────────────────────────────

    /// 创建一个包含 HandshakeAck 帧的 Cursor（模拟服务端响应）
    fn make_handshake_ack_bytes(version: u16, platform: Platform) -> Vec<u8> {
        protocol::encode(&KvmFrame::HandshakeAck { version, platform })
    }

    /// 创建一个包含 Handshake 帧的 Cursor（模拟客户端请求）
    fn make_handshake_bytes(version: u16, platform: Platform) -> Vec<u8> {
        protocol::encode(&KvmFrame::Handshake { version, platform })
    }

    // ── 客户端握手测试 ───────────────────────────────────────────────────

    /// 模拟双向 IO 的 Stream（用于测试）
    struct MockStream {
        /// 读取数据（模拟服务端响应）
        read_data: Cursor<Vec<u8>>,
        /// 写入数据（记录客户端发送的内容）
        write_data: Vec<u8>,
    }

    impl MockStream {
        fn new(response: Vec<u8>) -> Self {
            Self {
                read_data: Cursor::new(response),
                write_data: Vec::new(),
            }
        }

        fn written_bytes(&self) -> &[u8] {
            &self.write_data
        }
    }

    impl Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.read_data.read(buf)
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.write_data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_client_handshake_success() {
        // 模拟服务端返回正确的 HandshakeAck
        let ack_bytes = make_handshake_ack_bytes(PROTOCOL_VERSION, Platform::Windows);
        let mut stream = MockStream::new(ack_bytes);

        let result = perform_client_handshake(&mut stream);

        assert!(result.is_ok(), "握手应该成功，实际: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.remote_version, PROTOCOL_VERSION);
        assert_eq!(info.remote_platform, Platform::Windows);
    }

    #[test]
    fn test_client_handshake_sends_correct_frame() {
        // 验证客户端发送了正确的 Handshake 帧
        let ack_bytes = make_handshake_ack_bytes(PROTOCOL_VERSION, Platform::MacOS);
        let mut stream = MockStream::new(ack_bytes);

        perform_client_handshake(&mut stream).ok();

        // 验证发送的字节是有效的 Handshake 帧
        let written = stream.written_bytes();
        assert!(!written.is_empty(), "应该发送了握手帧");
        let (frame, _) = protocol::decode(written).unwrap();
        assert!(matches!(frame, KvmFrame::Handshake { version: PROTOCOL_VERSION, .. }));
    }

    #[test]
    fn test_client_handshake_version_mismatch() {
        // 模拟服务端返回不兼容的版本
        let ack_bytes = make_handshake_ack_bytes(PROTOCOL_VERSION + 1, Platform::Windows);
        let mut stream = MockStream::new(ack_bytes);

        let result = perform_client_handshake(&mut stream);

        assert_eq!(
            result,
            Err(HandshakeError::VersionMismatch {
                local: PROTOCOL_VERSION,
                remote: PROTOCOL_VERSION + 1,
            })
        );
    }

    #[test]
    fn test_client_handshake_not_kvm_service_invalid_magic() {
        // 模拟对端返回非 KVMini 协议数据
        let garbage = b"HTTP/1.1 200 OK\r\n".to_vec();
        let mut stream = MockStream::new(garbage);

        let result = perform_client_handshake(&mut stream);

        assert_eq!(result, Err(HandshakeError::NotKvmService));
    }

    #[test]
    fn test_client_handshake_empty_response() {
        // 模拟对端立即关闭连接（返回空数据）
        let mut stream = MockStream::new(vec![]);

        let result = perform_client_handshake(&mut stream);

        assert_eq!(result, Err(HandshakeError::NotKvmService));
    }

    #[test]
    fn test_client_handshake_wrong_frame_type() {
        // 模拟对端返回了 Heartbeat 而不是 HandshakeAck
        let heartbeat_bytes = protocol::encode(&KvmFrame::Heartbeat);
        let mut stream = MockStream::new(heartbeat_bytes);

        let result = perform_client_handshake(&mut stream);

        assert_eq!(result, Err(HandshakeError::NotKvmService));
    }

    // ── 服务端握手测试 ───────────────────────────────────────────────────

    #[test]
    fn test_server_handshake_success() {
        // 模拟客户端发送正确的 Handshake 帧
        let handshake_bytes = make_handshake_bytes(PROTOCOL_VERSION, Platform::MacOS);
        let mut stream = MockStream::new(handshake_bytes);

        let result = perform_server_handshake(&mut stream);

        assert!(result.is_ok(), "服务端握手应该成功，实际: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.remote_version, PROTOCOL_VERSION);
        assert_eq!(info.remote_platform, Platform::MacOS);
    }

    #[test]
    fn test_server_handshake_sends_ack() {
        // 验证服务端发送了正确的 HandshakeAck 帧
        let handshake_bytes = make_handshake_bytes(PROTOCOL_VERSION, Platform::Windows);
        let mut stream = MockStream::new(handshake_bytes);

        perform_server_handshake(&mut stream).ok();

        // 验证发送的字节是有效的 HandshakeAck 帧
        let written = stream.written_bytes();
        assert!(!written.is_empty(), "服务端应该发送了 HandshakeAck");
        let (frame, _) = protocol::decode(written).unwrap();
        assert!(matches!(frame, KvmFrame::HandshakeAck { version: PROTOCOL_VERSION, .. }));
    }

    #[test]
    fn test_server_handshake_version_mismatch() {
        // 模拟客户端发送不兼容的版本
        let handshake_bytes = make_handshake_bytes(PROTOCOL_VERSION + 1, Platform::Windows);
        let mut stream = MockStream::new(handshake_bytes);

        let result = perform_server_handshake(&mut stream);

        assert_eq!(
            result,
            Err(HandshakeError::VersionMismatch {
                local: PROTOCOL_VERSION,
                remote: PROTOCOL_VERSION + 1,
            })
        );
    }

    #[test]
    fn test_server_handshake_not_kvm_client() {
        // 模拟非 KVMini 客户端连接
        let garbage = b"GET / HTTP/1.1\r\n".to_vec();
        let mut stream = MockStream::new(garbage);

        let result = perform_server_handshake(&mut stream);

        assert_eq!(result, Err(HandshakeError::NotKvmService));
    }

    // ── 错误消息测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_handshake_error_display_not_kvm_service() {
        let err = HandshakeError::NotKvmService;
        assert!(err.to_string().contains("对端不是 KVMini 服务"));
    }

    #[test]
    fn test_handshake_error_display_version_mismatch() {
        let err = HandshakeError::VersionMismatch { local: 1, remote: 2 };
        let msg = err.to_string();
        assert!(msg.contains("版本不兼容"));
        assert!(msg.contains("1"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn test_handshake_error_display_timeout() {
        let err = HandshakeError::Timeout;
        assert!(err.to_string().contains("握手超时"));
    }

    // ── 跨平台握手测试（macOS ↔ Windows）────────────────────────────────

    #[test]
    fn test_cross_platform_handshake_macos_to_windows() {
        // 模拟 macOS 主控端连接 Windows 被控端
        let ack_bytes = make_handshake_ack_bytes(PROTOCOL_VERSION, Platform::Windows);
        let mut stream = MockStream::new(ack_bytes);

        let result = perform_client_handshake(&mut stream);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().remote_platform, Platform::Windows);
    }

    #[test]
    fn test_cross_platform_handshake_windows_to_macos() {
        // 模拟 Windows 主控端连接 macOS 被控端
        let ack_bytes = make_handshake_ack_bytes(PROTOCOL_VERSION, Platform::MacOS);
        let mut stream = MockStream::new(ack_bytes);

        let result = perform_client_handshake(&mut stream);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().remote_platform, Platform::MacOS);
    }
}
