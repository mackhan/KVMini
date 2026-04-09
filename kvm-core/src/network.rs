//! 网络连接模块
//!
//! TCP 长连接建立/断开/重连，UDP 局域网发现，握手协议，心跳保活。
//! 合并自两端的 connection/manager.rs、server.rs、discovery.rs/network.rs。

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{broadcast, Mutex, Notify};
use tokio::time::{timeout, Duration, Instant};

use crate::error::{KvmError, KvmResult};
use crate::platform::VirtualDesktop;
use crate::protocol::{self, KvmFrame, Platform, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};

/// 默认 TCP 端口
pub const DEFAULT_TCP_PORT: u16 = 23456;

/// 默认 UDP 发现端口
pub const DEFAULT_UDP_PORT: u16 = 8888;

/// UDP 发现魔数
pub const DISCOVERY_MAGIC: &[u8] = b"KVM_DISCOVERY";

/// 发现扫描超时（秒）
pub const SCAN_TIMEOUT_SECS: u64 = 5;

/// 心跳间隔（秒）
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// 心跳超时次数
pub const HEARTBEAT_TIMEOUT_COUNT: u32 = 3;

/// 重连间隔（秒）
pub const RECONNECT_INTERVAL_SECS: u64 = 3;

/// 最大重连次数
pub const MAX_RECONNECT_RETRIES: u32 = 5;

// ── 发现的设备信息 ──────────────────────────────────────────────────────────

/// 发现的设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// 设备名称
    pub name: String,
    /// 操作系统
    pub os: String,
    /// IP 地址
    pub ip: String,
    /// TCP 端口
    pub tcp_port: u16,
}

/// UDP 发现响应数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryResponse {
    magic: String,
    version: u16,
    hostname: String,
    os: String,
    tcp_port: u16,
}

// ── 连接状态 ────────────────────────────────────────────────────────────────

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// 未连接
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 正在重连
    Reconnecting { attempt: u32 },
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "未连接"),
            Self::Connecting => write!(f, "正在连接"),
            Self::Connected => write!(f, "已连接"),
            Self::Reconnecting { attempt } => write!(f, "正在重连（第 {} 次）", attempt),
        }
    }
}

// ── 握手信息 ────────────────────────────────────────────────────────────────

/// 握手信息（连接建立时交换）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeInfo {
    /// 协议版本
    pub version: u16,
    /// 平台
    pub platform: Platform,
    /// 屏幕布局信息
    pub screen_info: VirtualDesktop,
}

impl HandshakeInfo {
    pub fn new(platform: Platform, screen_info: VirtualDesktop) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            platform,
            screen_info,
        }
    }

    /// 序列化为字节（用于握手帧的 screen_info 字段）
    pub fn screen_info_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.screen_info).unwrap_or_default()
    }

    /// 从字节反序列化屏幕信息
    pub fn parse_screen_info(bytes: &[u8]) -> Option<VirtualDesktop> {
        serde_json::from_slice(bytes).ok()
    }
}

// ── 网络事件 ────────────────────────────────────────────────────────────────

/// 网络事件（通过 broadcast channel 发送给上层）
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// 连接状态变化
    ConnectionStateChanged(ConnectionState),
    /// 收到帧
    FrameReceived(KvmFrame),
    /// 对端握手信息
    PeerHandshake(HandshakeInfo),
    /// 网络延迟更新（毫秒）
    LatencyUpdated(u64),
    /// 网络延迟过高警告（>500ms）
    HighLatencyWarning(u64),
    /// 错误
    Error(String),
}

// ── UDP 发现 ────────────────────────────────────────────────────────────────

/// 发送 UDP 广播发现请求，收集局域网内所有 KVMini 实例
///
/// 固定等待 5 秒后返回设备列表。
pub async fn discover_devices(udp_port: u16) -> KvmResult<Vec<DiscoveredDevice>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    let broadcast_addr = format!("255.255.255.255:{}", udp_port);
    socket.send_to(DISCOVERY_MAGIC, &broadcast_addr).await?;

    let mut devices = Vec::new();
    let mut buf = [0u8; 2048];
    let deadline = Instant::now() + Duration::from_secs(SCAN_TIMEOUT_SECS);

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }

        match timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((size, src_addr))) => {
                if let Ok(text) = std::str::from_utf8(&buf[..size]) {
                    if let Ok(resp) = serde_json::from_str::<DiscoveryResponse>(text) {
                        if resp.magic == "KVM_RESPONSE" {
                            devices.push(DiscoveredDevice {
                                name: resp.hostname,
                                os: resp.os,
                                ip: src_addr.ip().to_string(),
                                tcp_port: resp.tcp_port,
                            });
                        }
                    }
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }

    Ok(devices)
}

/// 启动 UDP 发现监听器（响应其他 KVMini 实例的发现请求）
pub async fn start_discovery_listener(
    udp_port: u16,
    tcp_port: u16,
    hostname: String,
    os_name: String,
    stop: Arc<Notify>,
) -> KvmResult<()> {
    let addr = format!("0.0.0.0:{}", udp_port);
    let socket = UdpSocket::bind(&addr).await?;

    log::info!("[UDP] 监听端口 {}，等待发现请求...", udp_port);

    let mut buf = [0u8; 256];
    loop {
        tokio::select! {
            _ = stop.notified() => {
                log::info!("[UDP] 发现监听器已停止");
                break;
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, src)) => {
                        if &buf[..size] == DISCOVERY_MAGIC {
                            log::info!("[UDP] 收到来自 {} 的发现请求", src);
                            let resp = serde_json::json!({
                                "magic": "KVM_RESPONSE",
                                "version": PROTOCOL_VERSION,
                                "hostname": hostname,
                                "os": os_name,
                                "tcp_port": tcp_port,
                            });
                            let resp_bytes = resp.to_string();
                            if let Err(e) = socket.send_to(resp_bytes.as_bytes(), src).await {
                                log::warn!("[UDP] 回复失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[UDP] 接收错误: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

// ── TCP 连接管理 ────────────────────────────────────────────────────────────

/// KVM 连接管理器
///
/// 对等连接模型：两端都监听 TCP，也都可以主动连接对方。
pub struct ConnectionManager {
    /// TCP 监听端口
    tcp_port: u16,
    /// 事件广播发送端
    event_tx: broadcast::Sender<NetworkEvent>,
    /// 当前连接状态
    state: Arc<Mutex<ConnectionState>>,
    /// 写入端（用于发送帧给对端）
    writer: Arc<Mutex<Option<tokio::io::WriteHalf<TcpStream>>>>,
    /// 停止信号
    stop: Arc<Notify>,
    /// 对端地址（用于重连）
    peer_addr: Arc<Mutex<Option<String>>>,
    /// 上次心跳响应时间
    last_heartbeat: Arc<Mutex<Instant>>,
}

impl ConnectionManager {
    /// 创建连接管理器
    pub fn new(tcp_port: u16) -> (Self, broadcast::Receiver<NetworkEvent>) {
        let (event_tx, event_rx) = broadcast::channel(512);
        let mgr = Self {
            tcp_port,
            event_tx,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            writer: Arc::new(Mutex::new(None)),
            stop: Arc::new(Notify::new()),
            peer_addr: Arc::new(Mutex::new(None)),
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
        };
        (mgr, event_rx)
    }

    /// 订阅网络事件
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }

    /// 获取当前连接状态
    pub async fn state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// 启动 TCP 监听（对等模型：等待对方连接）
    pub async fn start_listening(&self) -> KvmResult<SocketAddr> {
        let addr = format!("0.0.0.0:{}", self.tcp_port);
        let listener = TcpListener::bind(&addr).await?;
        let actual_addr = listener.local_addr()?;

        log::info!("[TCP] 监听 {}", actual_addr);

        let event_tx = self.event_tx.clone();
        let state = Arc::clone(&self.state);
        let writer = Arc::clone(&self.writer);
        let stop = Arc::clone(&self.stop);
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let peer_addr = Arc::clone(&self.peer_addr);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.notified() => {
                        log::info!("[TCP] 监听器已停止");
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                log::info!("[TCP] 接受连接: {}", addr);
                                *peer_addr.lock().await = Some(addr.to_string());
                                handle_stream(
                                    stream, &event_tx, &state, &writer,
                                    &last_heartbeat, false,
                                ).await;
                                *state.lock().await = ConnectionState::Disconnected;
                                let _ = event_tx.send(NetworkEvent::ConnectionStateChanged(
                                    ConnectionState::Disconnected,
                                ));
                            }
                            Err(e) => {
                                log::error!("[TCP] 接受连接失败: {}", e);
                            }
                        }
                    }
                }
            }
        });

        Ok(actual_addr)
    }

    /// 主动连接到对端
    pub async fn connect_to(
        &self,
        peer_ip: &str,
        peer_port: u16,
        local_info: HandshakeInfo,
    ) -> KvmResult<HandshakeInfo> {
        let addr = format!("{}:{}", peer_ip, peer_port);
        *self.state.lock().await = ConnectionState::Connecting;
        let _ = self.event_tx.send(NetworkEvent::ConnectionStateChanged(
            ConnectionState::Connecting,
        ));

        let stream = timeout(
            Duration::from_secs(10),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| KvmError::ConnectionTimeout {
            peer_ip: peer_ip.to_string(),
            elapsed_secs: 10,
        })?
        .map_err(|e| KvmError::from_io_with_context(&e, peer_ip, peer_port))?;

        stream.set_nodelay(true).ok();

        *self.peer_addr.lock().await = Some(addr.clone());

        // 执行握手（主动方：先发 Handshake，等 HandshakeAck）
        let (mut reader, mut writer) = tokio::io::split(stream);

        // 发送 Handshake
        let handshake_frame = KvmFrame::Handshake {
            version: PROTOCOL_VERSION,
            platform: local_info.platform,
            screen_info: local_info.screen_info_bytes(),
        };
        protocol::write_frame_async(&mut writer, &handshake_frame).await?;

        // 等待 HandshakeAck
        let ack = timeout(
            Duration::from_secs(5),
            protocol::read_frame_async(&mut reader),
        )
        .await
        .map_err(|_| KvmError::HandshakeFailed {
            reason: "握手超时".to_string(),
        })??;

        let peer_info = match ack {
            KvmFrame::HandshakeAck { version, platform, screen_info } => {
                if version != PROTOCOL_VERSION {
                    return Err(KvmError::ProtocolMismatch {
                        local_version: PROTOCOL_VERSION,
                        remote_version: version,
                    });
                }
                let desktop = HandshakeInfo::parse_screen_info(&screen_info)
                    .unwrap_or_else(|| VirtualDesktop::from_monitors(vec![]));
                HandshakeInfo { version, platform, screen_info: desktop }
            }
            other => {
                return Err(KvmError::HandshakeFailed {
                    reason: format!("期望 HandshakeAck，收到 {:?}", other),
                });
            }
        };

        let _ = self.event_tx.send(NetworkEvent::PeerHandshake(peer_info.clone()));

        // 连接成功
        *self.state.lock().await = ConnectionState::Connected;
        let _ = self.event_tx.send(NetworkEvent::ConnectionStateChanged(
            ConnectionState::Connected,
        ));

        // 保存 writer
        *self.writer.lock().await = Some(writer);
        *self.last_heartbeat.lock().await = Instant::now();

        // 启动读取循环和心跳
        let event_tx = self.event_tx.clone();
        let state = Arc::clone(&self.state);
        let last_heartbeat = Arc::clone(&self.last_heartbeat);

        tokio::spawn(async move {
            read_loop(reader, &event_tx, &state, &last_heartbeat).await;
        });

        self.start_heartbeat();

        Ok(peer_info)
    }

    /// 发送帧给对端
    pub async fn send_frame(&self, frame: &KvmFrame) -> KvmResult<()> {
        let mut guard = self.writer.lock().await;
        match guard.as_mut() {
            Some(writer) => {
                protocol::write_frame_async(writer, frame).await
            }
            None => Err(KvmError::Disconnected),
        }
    }

    /// 断开连接
    pub async fn disconnect(&self) {
        *self.writer.lock().await = None;
        *self.state.lock().await = ConnectionState::Disconnected;
        let _ = self.event_tx.send(NetworkEvent::ConnectionStateChanged(
            ConnectionState::Disconnected,
        ));
    }

    /// 停止所有网络活动
    pub fn stop(&self) {
        self.stop.notify_waiters();
    }

    /// 启动心跳定时器
    fn start_heartbeat(&self) {
        let writer = Arc::clone(&self.writer);
        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let stop = Arc::clone(&self.stop);

        tokio::spawn(async move {
            let mut missed_count: u32 = 0;

            loop {
                tokio::select! {
                    _ = stop.notified() => break,
                    _ = tokio::time::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)) => {
                        // 检查连接状态
                        if *state.lock().await != ConnectionState::Connected {
                            break;
                        }

                        // 发送心跳
                        let mut guard = writer.lock().await;
                        if let Some(w) = guard.as_mut() {
                            if protocol::write_frame_async(w, &KvmFrame::Heartbeat).await.is_err() {
                                break;
                            }
                        } else {
                            break;
                        }
                        drop(guard);

                        // 检查心跳响应
                        let elapsed = last_heartbeat.lock().await.elapsed();
                        if elapsed > Duration::from_secs(HEARTBEAT_INTERVAL_SECS * 2) {
                            missed_count += 1;
                            if missed_count >= HEARTBEAT_TIMEOUT_COUNT {
                                log::warn!("[心跳] 连续 {} 次无响应，判定连接断开", missed_count);
                                *state.lock().await = ConnectionState::Disconnected;
                                let _ = event_tx.send(NetworkEvent::ConnectionStateChanged(
                                    ConnectionState::Disconnected,
                                ));
                                break;
                            }
                        } else {
                            missed_count = 0;
                        }
                    }
                }
            }
        });
    }
}

/// 处理已建立的 TCP 流（被动接受连接时使用）
async fn handle_stream(
    stream: TcpStream,
    event_tx: &broadcast::Sender<NetworkEvent>,
    state: &Arc<Mutex<ConnectionState>>,
    writer_slot: &Arc<Mutex<Option<tokio::io::WriteHalf<TcpStream>>>>,
    last_heartbeat: &Arc<Mutex<Instant>>,
    _is_initiator: bool,
) {
    stream.set_nodelay(true).ok();

    let (mut reader, mut writer) = tokio::io::split(stream);

    // 被动方：等待 Handshake，回复 HandshakeAck
    let handshake_result = timeout(
        Duration::from_secs(5),
        protocol::read_frame_async(&mut reader),
    ).await;

    let peer_info = match handshake_result {
        Ok(Ok(KvmFrame::Handshake { version, platform, screen_info })) => {
            if version != PROTOCOL_VERSION {
                let _ = event_tx.send(NetworkEvent::Error(format!(
                    "版本不兼容：本端 v{}，对端 v{}", PROTOCOL_VERSION, version
                )));
                return;
            }

            // 回复 HandshakeAck（使用空屏幕信息，后续由上层补充）
            let ack = KvmFrame::HandshakeAck {
                version: PROTOCOL_VERSION,
                platform: Platform::Unknown, // 由上层设置
                screen_info: vec![],
            };
            if protocol::write_frame_async(&mut writer, &ack).await.is_err() {
                return;
            }

            let desktop = HandshakeInfo::parse_screen_info(&screen_info)
                .unwrap_or_else(|| VirtualDesktop::from_monitors(vec![]));
            HandshakeInfo { version, platform, screen_info: desktop }
        }
        _ => {
            let _ = event_tx.send(NetworkEvent::Error("握手失败".to_string()));
            return;
        }
    };

    let _ = event_tx.send(NetworkEvent::PeerHandshake(peer_info));

    *state.lock().await = ConnectionState::Connected;
    let _ = event_tx.send(NetworkEvent::ConnectionStateChanged(ConnectionState::Connected));

    *writer_slot.lock().await = Some(writer);
    *last_heartbeat.lock().await = Instant::now();

    // 读取循环
    read_loop(reader, event_tx, state, last_heartbeat).await;
}

/// 读取循环：持续从 TCP 流读取帧并分发
async fn read_loop(
    mut reader: tokio::io::ReadHalf<TcpStream>,
    event_tx: &broadcast::Sender<NetworkEvent>,
    state: &Arc<Mutex<ConnectionState>>,
    last_heartbeat: &Arc<Mutex<Instant>>,
) {
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 4096];

    loop {
        match reader.read(&mut tmp).await {
            Ok(0) => {
                // 连接关闭
                log::info!("[TCP] 连接已关闭");
                break;
            }
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);

                // 解码所有完整帧
                loop {
                    match protocol::decode(&buf) {
                        Ok((frame, consumed)) => {
                            dispatch_frame(&frame, event_tx, last_heartbeat).await;
                            buf.drain(..consumed);
                        }
                        Err(KvmError::Incomplete) => break,
                        Err(e) => {
                            log::error!("[TCP] 协议解码错误: {}", e);
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("[TCP] 读取错误: {}", e);
                break;
            }
        }
    }

    *state.lock().await = ConnectionState::Disconnected;
    let _ = event_tx.send(NetworkEvent::ConnectionStateChanged(ConnectionState::Disconnected));
}

/// 分发解码后的帧
async fn dispatch_frame(
    frame: &KvmFrame,
    event_tx: &broadcast::Sender<NetworkEvent>,
    last_heartbeat: &Arc<Mutex<Instant>>,
) {
    match frame {
        KvmFrame::HeartbeatAck => {
            *last_heartbeat.lock().await = Instant::now();
        }
        KvmFrame::Heartbeat => {
            // 收到心跳，标记活跃（回复由上层处理或自动回复）
            *last_heartbeat.lock().await = Instant::now();
        }
        KvmFrame::Pong { timestamp_ms } => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let latency = now.saturating_sub(*timestamp_ms);
            let _ = event_tx.send(NetworkEvent::LatencyUpdated(latency));
            if latency > 500 {
                let _ = event_tx.send(NetworkEvent::HighLatencyWarning(latency));
            }
        }
        _ => {
            let _ = event_tx.send(NetworkEvent::FrameReceived(frame.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_display() {
        assert_eq!(format!("{}", ConnectionState::Disconnected), "未连接");
        assert_eq!(format!("{}", ConnectionState::Connected), "已连接");
        assert_eq!(
            format!("{}", ConnectionState::Reconnecting { attempt: 3 }),
            "正在重连（第 3 次）"
        );
    }

    #[test]
    fn test_handshake_info() {
        let info = HandshakeInfo::new(
            Platform::Windows,
            VirtualDesktop::from_monitors(vec![]),
        );
        assert_eq!(info.version, PROTOCOL_VERSION);
        assert_eq!(info.platform, Platform::Windows);
    }

    #[test]
    fn test_handshake_screen_info_roundtrip() {
        use crate::platform::MonitorInfo;
        let desktop = VirtualDesktop::from_monitors(vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }]);
        let info = HandshakeInfo::new(Platform::MacOS, desktop.clone());
        let bytes = info.screen_info_bytes();
        let parsed = HandshakeInfo::parse_screen_info(&bytes).unwrap();
        assert_eq!(parsed.total_width, desktop.total_width);
        assert_eq!(parsed.total_height, desktop.total_height);
    }

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let (mgr, _rx) = ConnectionManager::new(0);
        assert_eq!(mgr.state().await, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_discover_devices_returns_empty_on_no_response() {
        // 使用随机端口避免冲突
        let result = discover_devices(19999).await;
        assert!(result.is_ok());
        // 没有其他设备响应，应该返回空列表
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tcp_listen_and_connect() {
        let (mgr, mut rx) = ConnectionManager::new(0);
        let addr = mgr.start_listening().await.unwrap();

        // 在 Windows 上，0.0.0.0 不能直接连接，需要用 127.0.0.1
        let connect_addr = SocketAddr::new(
            "127.0.0.1".parse().unwrap(),
            addr.port(),
        );

        // 等待监听器就绪
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 模拟客户端连接
        let client = TcpStream::connect(connect_addr).await.unwrap();
        client.set_nodelay(true).unwrap();

        let (mut reader, mut writer) = tokio::io::split(client);

        // 发送握手
        let handshake = KvmFrame::Handshake {
            version: PROTOCOL_VERSION,
            platform: Platform::MacOS,
            screen_info: vec![],
        };
        protocol::write_frame_async(&mut writer, &handshake).await.unwrap();

        // 等待握手响应
        let ack = timeout(
            Duration::from_secs(3),
            protocol::read_frame_async(&mut reader),
        ).await.unwrap().unwrap();

        assert!(matches!(ack, KvmFrame::HandshakeAck { .. }));

        // 等待连接状态变化事件（可能有多个事件）
        let mut got_connected = false;
        for _ in 0..5 {
            match timeout(Duration::from_secs(2), rx.recv()).await {
                Ok(Ok(NetworkEvent::ConnectionStateChanged(ConnectionState::Connected))) => {
                    got_connected = true;
                    break;
                }
                Ok(Ok(_)) => continue, // 其他事件，继续等
                _ => break,
            }
        }
        assert!(got_connected, "应该收到 Connected 状态事件");

        mgr.stop();
    }
}
