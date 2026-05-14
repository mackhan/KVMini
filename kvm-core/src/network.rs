//! 网络连接模块
//!
//! TCP 长连接建立/断开/重连，UDP 局域网发现，握手协议，心跳保活。
//! 合并自两端的 connection/manager.rs、server.rs、discovery.rs/network.rs。

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{broadcast, mpsc, Mutex, Notify};
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

/// 子网 TCP 扫描单个主机超时（毫秒）
pub const SUBNET_SCAN_TIMEOUT_MS: u64 = 500;

/// 子网 TCP 扫描最大并发数
pub const SUBNET_SCAN_CONCURRENCY: usize = 50;

/// 心跳间隔（秒）
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// 心跳超时次数
pub const HEARTBEAT_TIMEOUT_COUNT: u32 = 3;

/// 重连间隔（秒）
pub const RECONNECT_INTERVAL_SECS: u64 = 3;

/// 最大重连次数
pub const MAX_RECONNECT_RETRIES: u32 = 5;

// ── TCP Keepalive 配置（v0.1.59 BUG-043 F3 纵深防御）────────────────────────

/// TCP Keepalive idle 时间（秒）：连接空闲多久后开始发探测包
pub const TCP_KEEPALIVE_IDLE_SECS: u64 = 5;

/// TCP Keepalive 探测间隔（秒）
pub const TCP_KEEPALIVE_INTERVAL_SECS: u64 = 3;

/// TCP Keepalive 失败重试次数
pub const TCP_KEEPALIVE_RETRIES: u32 = 2;

/// 给已建立的 TcpStream 应用 OS 级 keepalive 配置。
///
/// 作用：让远端进程被 kill -9 / 拔网线 / crash 时，本端的 read_loop 能在
/// `idle + interval * retries ≈ 11s` 内通过 TCP RST/超时感知断线，
/// 配合应用层 Watchdog（10s 超时）形成双保险。
///
/// 平台支持：
/// - macOS / Linux / Windows 10+ 均支持（socket2 内部封装 setsockopt）
/// - Windows 7/Server 2008 旧系统会忽略 interval/retries 字段，回退为系统默认 keepalive，不报错
///
/// 失败处理：
/// 该函数失败不应阻塞连接建立流程；调用方仅打 warn 日志即可。
pub fn apply_tcp_keepalive(stream: &TcpStream) -> std::io::Result<()> {
    use socket2::{SockRef, TcpKeepalive};
    use std::time::Duration as StdDuration;

    let sock_ref = SockRef::from(stream);
    let ka = TcpKeepalive::new()
        .with_time(StdDuration::from_secs(TCP_KEEPALIVE_IDLE_SECS))
        .with_interval(StdDuration::from_secs(TCP_KEEPALIVE_INTERVAL_SECS))
        .with_retries(TCP_KEEPALIVE_RETRIES);
    sock_ref.set_tcp_keepalive(&ka)?;
    log::debug!(
        "[Network] TCP Keepalive 已应用：idle={}s, interval={}s, retries={}",
        TCP_KEEPALIVE_IDLE_SECS,
        TCP_KEEPALIVE_INTERVAL_SECS,
        TCP_KEEPALIVE_RETRIES,
    );
    Ok(())
}

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

/// 握手 wire payload（实际通过 KvmFrame::Handshake/HandshakeAck 的 screen_info 字段传输）
///
/// 设计要点：
/// - 在不破坏二进制兼容的前提下，复用 `screen_info: Vec<u8>` 字段承载 JSON。
/// - 旧版本客户端 wire 上只有 `VirtualDesktop` 的 JSON，没有 tls_enabled。
/// - `#[serde(default)]` 让旧 JSON 反序列化时 tls_enabled 默认为 false。
/// - `parse_payload()` 还提供向后回退：解析失败时按旧 VirtualDesktop 格式再试一次。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct HandshakePayload {
    #[serde(default)]
    pub desktop: VirtualDesktop,
    #[serde(default)]
    pub tls_enabled: bool,
    /// v0.1.60 BUG-046: 握手时携带布局方向（发起方为准，被动方自动镜像）
    /// 旧版本无此字段，反序列化时默认 None → 不触发布局同步
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_direction: Option<String>,
}

/// 握手信息（连接建立时交换）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeInfo {
    /// 协议版本
    pub version: u16,
    /// 平台
    pub platform: Platform,
    /// 屏幕布局信息
    pub screen_info: VirtualDesktop,
    /// 是否启用 TLS 加密（用于双端 TLS 状态比较）
    /// 注意：旧版本客户端不携带此字段，反序列化时默认为 false（未开启）。
    /// 如果本机开启了 TLS 但对端是旧版本，会触发 tls_mismatch 提示，这是预期行为。
    #[serde(default)]
    pub tls_enabled: bool,
    /// v0.1.60 BUG-046: 对端的布局方向字符串（如 "LocalLeftOfRemote"）
    /// 旧版本无此字段，默认 None → 不触发布局同步
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_direction: Option<String>,
}

impl HandshakeInfo {
    pub fn new(platform: Platform, screen_info: VirtualDesktop) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            platform,
            screen_info,
            tls_enabled: false,
            layout_direction: None,
        }
    }

    /// 创建携带 TLS 状态的握手信息
    pub fn new_with_tls(platform: Platform, screen_info: VirtualDesktop, tls_enabled: bool) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            platform,
            screen_info,
            tls_enabled,
            layout_direction: None,
        }
    }

    /// 创建携带 TLS 状态和布局方向的握手信息（v0.1.60 BUG-046）
    pub fn new_with_tls_and_layout(
        platform: Platform,
        screen_info: VirtualDesktop,
        tls_enabled: bool,
        layout_direction: Option<String>,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            platform,
            screen_info,
            tls_enabled,
            layout_direction,
        }
    }

    /// 序列化为字节（用于握手帧的 screen_info 字段）
    ///
    /// 新格式：序列化整个 HandshakePayload（含 tls_enabled）。
    /// 旧客户端反序列化为 VirtualDesktop 时，只看到 desktop/tls_enabled 两个 key，
    /// 因为 VirtualDesktop 的 serde 是 deny_unknown_fields=false（默认），多余字段会被忽略，
    /// 但因 wire 格式实际是 `{"desktop":{...},"tls_enabled":bool}`，旧版按 VirtualDesktop 解析会失败。
    /// 旧版客户端的兼容靠 parse_screen_info 的 unwrap_or_else 退化到空 desktop——
    /// 这是可接受的：旧版本本来就不传 desktop 数据。
    pub fn screen_info_bytes(&self) -> Vec<u8> {
        let payload = HandshakePayload {
            desktop: self.screen_info.clone(),
            tls_enabled: self.tls_enabled,
            layout_direction: self.layout_direction.clone(),
        };
        serde_json::to_vec(&payload).unwrap_or_default()
    }

    /// 从字节反序列化屏幕信息（仅 desktop，向后兼容旧调用方）
    pub fn parse_screen_info(bytes: &[u8]) -> Option<VirtualDesktop> {
        let (desktop, _, _) = Self::parse_payload(bytes);
        if desktop.total_width == 0 && desktop.total_height == 0 && bytes.is_empty() {
            None
        } else {
            Some(desktop)
        }
    }

    /// 解析握手 wire payload，向后兼容旧格式
    ///
    /// 解析顺序：
    /// 1. 优先按新版 HandshakePayload 解析
    /// 2. 失败则按旧版 VirtualDesktop 解析（tls_enabled 默认 false）
    /// 3. 都失败返回默认 desktop + false + None
    ///
    /// 返回 (VirtualDesktop, tls_enabled, layout_direction)
    pub fn parse_payload(bytes: &[u8]) -> (VirtualDesktop, bool, Option<String>) {
        // 新版优先
        if let Ok(payload) = serde_json::from_slice::<HandshakePayload>(bytes) {
            // HandshakePayload 反序列化对完全空的 JSON 也成功（全用 default），
            // 必须验证 desktop 字段是否真的存在；简单办法：检查原 bytes 是否含 "desktop"
            if bytes.windows(b"desktop".len()).any(|w| w == b"desktop")
                || bytes.windows(b"tls_enabled".len()).any(|w| w == b"tls_enabled")
            {
                return (payload.desktop, payload.tls_enabled, payload.layout_direction);
            }
        }
        // 旧版回退
        if let Ok(desktop) = serde_json::from_slice::<VirtualDesktop>(bytes) {
            return (desktop, false, None);
        }
        // 全部失败
        (VirtualDesktop::from_monitors(vec![]), false, None)
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
    /// v0.1.60 BUG-046: 布局方向同步完成（被动方镜像后触发）
    LayoutSynced {
        /// 镜像后的新布局方向字符串（如 "LocalRightOfRemote"）
        new_layout: String,
        /// 本端布局是否被改变（true = 之前不互补，已自动调整）
        was_changed: bool,
    },
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

/// 子网扫描探测魔数（放在 screen_info 字段，服务端识别后不进入 Connected 状态）
pub const PROBE_MAGIC: &[u8] = b"KVM_PROBE";

/// 子网 TCP 扫描：对 /24 网段并发 TCP 探测，识别 KVMini 设备
///
/// 当 UDP 广播失败时作为降级方案。发送带探测魔数的握手帧，
/// 只有回复 HandshakeAck 的才是真正的 KVMini 设备，避免误报其他服务。
/// 服务端收到探测魔数后只回包、不进入 Connected 状态。
pub async fn scan_subnet(local_ip: &str, tcp_port: u16) -> KvmResult<Vec<DiscoveredDevice>> {
    // 解析本机 IP，取前三段作为网段前缀（/24）
    let parts: Vec<&str> = local_ip.split('.').collect();
    if parts.len() != 4 {
        return Err(KvmError::NetworkUnreachable { peer_ip: format!("无效的本机 IP: {}", local_ip) });
    }
    let prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);

    log::info!("[SubnetScan] 开始扫描 {}.0/24 端口 {}", prefix, tcp_port);

    let semaphore = Arc::new(tokio::sync::Semaphore::new(SUBNET_SCAN_CONCURRENCY));
    let mut handles = Vec::with_capacity(253);

    for i in 1u8..=254 {
        let ip = format!("{}.{}", prefix, i);
        // 跳过本机 IP
        if ip == local_ip {
            continue;
        }
        let sem = Arc::clone(&semaphore);
        let ip_clone = ip.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            let addr = format!("{}:{}", ip_clone, tcp_port);
            let stream = timeout(
                Duration::from_millis(SUBNET_SCAN_TIMEOUT_MS),
                TcpStream::connect(&addr),
            ).await.ok()?.ok()?;
            stream.set_nodelay(true).ok();

            // 发送带探测魔数的握手帧（screen_info = PROBE_MAGIC）
            let (mut reader, mut writer) = tokio::io::split(stream);
            let probe_frame = KvmFrame::Handshake {
                version: PROTOCOL_VERSION,
                platform: Platform::Unknown,
                screen_info: PROBE_MAGIC.to_vec(),
            };
            protocol::write_frame_async(&mut writer, &probe_frame).await.ok()?;

            // 等待 HandshakeAck，只有 KVMini 设备会回
            let ack = timeout(
                Duration::from_millis(SUBNET_SCAN_TIMEOUT_MS),
                protocol::read_frame_async(&mut reader),
            ).await.ok()?.ok()?;

            if let KvmFrame::HandshakeAck { platform, .. } = ack {
                let os = format!("{:?}", platform);
                log::info!("[SubnetScan] 确认 KVMini 设备: {} ({})", ip_clone, os);
                Some(DiscoveredDevice {
                    name: ip_clone.clone(),
                    os,
                    ip: ip_clone,
                    tcp_port,
                })
            } else {
                None
            }
        });
        handles.push(handle);
    }

    let mut devices = Vec::new();
    for handle in handles {
        if let Ok(Some(device)) = handle.await {
            devices.push(device);
        }
    }

    log::info!("[SubnetScan] 扫描完成，确认 {} 个 KVMini 设备", devices.len());
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
/// 写操作通过 mpsc channel 序列化到单一 write_task，消除锁竞争。
pub struct ConnectionManager {
    /// TCP 监听端口
    tcp_port: u16,
    /// 事件广播发送端
    event_tx: broadcast::Sender<NetworkEvent>,
    /// 当前连接状态
    state: Arc<Mutex<ConnectionState>>,
    /// 写帧 channel（None 表示未连接）
    write_tx: Arc<Mutex<Option<mpsc::Sender<KvmFrame>>>>,
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
            write_tx: Arc::new(Mutex::new(None)),
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

    /// 启动 write_task：从 channel 接收帧并写入 TCP stream
    fn start_write_task(
        mut writer: tokio::io::WriteHalf<TcpStream>,
        mut rx: mpsc::Receiver<KvmFrame>,
    ) {
        tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                if protocol::write_frame_async(&mut writer, &frame).await.is_err() {
                    break;
                }
            }
        });
    }

    /// 启动 TCP 监听（对等模型：等待对方连接）
    ///
    /// `local_info` 提供本端握手信息，用于在 HandshakeAck 中携带本机 TLS 状态等元数据。
    pub async fn start_listening(&self, local_info: HandshakeInfo) -> KvmResult<SocketAddr> {
        let addr = format!("0.0.0.0:{}", self.tcp_port);
        let listener = TcpListener::bind(&addr).await?;
        let actual_addr = listener.local_addr()?;

        log::info!("[TCP] 监听 {}", actual_addr);

        let event_tx = self.event_tx.clone();
        let state = Arc::clone(&self.state);
        let write_tx = Arc::clone(&self.write_tx);
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
                                    stream, &event_tx, &state, &write_tx,
                                    &last_heartbeat, false,
                                    &local_info,
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
        // v0.1.59 BUG-043: 应用 TCP Keepalive（失败不阻塞，仅 warn）
        if let Err(e) = apply_tcp_keepalive(&stream) {
            log::warn!("[Network] connect_to 应用 TCP Keepalive 失败（降级为系统默认）: {}", e);
        }

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
                let (desktop, tls_enabled, layout_direction) = HandshakeInfo::parse_payload(&screen_info);
                log::info!(
                    "[Handshake] 主动方收到 HandshakeAck: peer_platform={:?}, peer_tls={}, peer_layout={:?}",
                    platform, tls_enabled, layout_direction
                );
                HandshakeInfo {
                    version, platform, screen_info: desktop, tls_enabled, layout_direction,
                }
            }
            other => {
                return Err(KvmError::HandshakeFailed {
                    reason: format!("期望 HandshakeAck，收到 {:?}", other),
                });
            }
        };

        let _ = self.event_tx.send(NetworkEvent::PeerHandshake(peer_info.clone()));

        // 连接成功：启动 write_task，存储 write_tx
        *self.state.lock().await = ConnectionState::Connected;
        let _ = self.event_tx.send(NetworkEvent::ConnectionStateChanged(
            ConnectionState::Connected,
        ));

        let (tx, rx) = mpsc::channel::<KvmFrame>(64);
        *self.write_tx.lock().await = Some(tx.clone());
        *self.last_heartbeat.lock().await = Instant::now();

        Self::start_write_task(writer, rx);

        // 启动读取循环和心跳
        let event_tx = self.event_tx.clone();
        let state = Arc::clone(&self.state);
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let write_tx_for_read = tx.clone();

        tokio::spawn(async move {
            read_loop(reader, &event_tx, &state, &last_heartbeat, &write_tx_for_read).await;
        });

        self.start_heartbeat(tx);

        Ok(peer_info)
    }

    /// 发送帧给对端
    pub async fn send_frame(&self, frame: &KvmFrame) -> KvmResult<()> {
        let guard = self.write_tx.lock().await;
        match guard.as_ref() {
            Some(tx) => tx.send(frame.clone()).await.map_err(|_| KvmError::Disconnected),
            None => Err(KvmError::Disconnected),
        }
    }

    /// 断开连接
    pub async fn disconnect(&self) {
        *self.write_tx.lock().await = None;
        *self.state.lock().await = ConnectionState::Disconnected;
        let _ = self.event_tx.send(NetworkEvent::ConnectionStateChanged(
            ConnectionState::Disconnected,
        ));
    }

    /// 发送 Ping 帧（用于测量延迟）
    ///
    /// 返回 Ok(()) 表示帧已入队；连接不存在时返回 Err。
    pub async fn send_ping(&self) -> KvmResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let frame = crate::protocol::KvmFrame::Ping { timestamp_ms: now };
        let guard = self.write_tx.lock().await;
        if let Some(tx) = guard.as_ref() {
            tx.send(frame).await.map_err(|_| crate::error::KvmError::Disconnected)?;
            Ok(())
        } else {
            Err(crate::error::KvmError::Disconnected)
        }
    }

    /// 停止所有网络活动
    pub fn stop(&self) {
        self.stop.notify_waiters();
    }

    /// 启动心跳定时器（使用 write_tx 发送心跳帧，无锁竞争）
    fn start_heartbeat(&self, tx: mpsc::Sender<KvmFrame>) {
        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let stop = Arc::clone(&self.stop);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.notified() => break,
                    _ = tokio::time::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)) => {
                        // 检查连接状态
                        if *state.lock().await != ConnectionState::Connected {
                            break;
                        }

                        // 发送心跳（通过 channel，不持锁）
                        if tx.send(KvmFrame::Heartbeat).await.is_err() {
                            break;
                        }

                        // 检查心跳响应：超过 HEARTBEAT_INTERVAL * TIMEOUT_COUNT 判定断开
                        let elapsed = last_heartbeat.lock().await.elapsed();
                        let timeout_threshold = Duration::from_secs(
                            HEARTBEAT_INTERVAL_SECS * HEARTBEAT_TIMEOUT_COUNT as u64
                        );
                        if elapsed > timeout_threshold {
                            log::warn!("[心跳] {}s 内未收到 HeartbeatAck，判定连接断开", timeout_threshold.as_secs());
                            *state.lock().await = ConnectionState::Disconnected;
                            let _ = event_tx.send(NetworkEvent::ConnectionStateChanged(
                                ConnectionState::Disconnected,
                            ));
                            break;
                        }
                    }
                }
            }
        });
    }
}

/// 处理已建立的 TCP 流（被动接受连接时使用）
///
/// `local_info` 用于构造 HandshakeAck 中携带的本端 wire payload（含 TLS 状态）。
async fn handle_stream(
    stream: TcpStream,
    event_tx: &broadcast::Sender<NetworkEvent>,
    state: &Arc<Mutex<ConnectionState>>,
    write_tx_slot: &Arc<Mutex<Option<mpsc::Sender<KvmFrame>>>>,
    last_heartbeat: &Arc<Mutex<Instant>>,
    _is_initiator: bool,
    local_info: &HandshakeInfo,
) {
    stream.set_nodelay(true).ok();
    // v0.1.59 BUG-043: 应用 TCP Keepalive（失败不阻塞，仅 warn）
    if let Err(e) = apply_tcp_keepalive(&stream) {
        log::warn!("[Network] handle_stream 应用 TCP Keepalive 失败（降级为系统默认）: {}", e);
    }

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

            // 探测模式：screen_info == PROBE_MAGIC，只回 HandshakeAck，不建立连接
            if screen_info == PROBE_MAGIC {
                log::debug!("[TCP] 收到探测握手，回复 HandshakeAck 后断开");
                let ack = KvmFrame::HandshakeAck {
                    version: PROTOCOL_VERSION,
                    platform: local_info.platform,
                    screen_info: vec![],
                };
                let _ = protocol::write_frame_async(&mut writer, &ack).await;
                return; // 不进入 Connected 状态
            }

            // 正常握手：HandshakeAck 必须携带本端 wire payload（含 TLS 状态），
            // 让主动方能正确读出本端 tls_enabled，否则会触发 BUG-018 误报。
            let ack = KvmFrame::HandshakeAck {
                version: PROTOCOL_VERSION,
                platform: local_info.platform,
                screen_info: local_info.screen_info_bytes(),
            };

            let (tx, rx) = mpsc::channel::<KvmFrame>(64);
            if tx.send(ack).await.is_err() {
                return;
            }
            *write_tx_slot.lock().await = Some(tx.clone());
            // SAFETY: writer 在探测分支已 return，此处独占所有权
            ConnectionManager::start_write_task(writer, rx);

            let (desktop, peer_tls, peer_layout) = HandshakeInfo::parse_payload(&screen_info);
            log::info!(
                "[Handshake] 被动方收到 Handshake: peer_platform={:?}, peer_tls={}, local_tls={}, peer_layout={:?}",
                platform, peer_tls, local_info.tls_enabled, peer_layout
            );

            // v0.1.60 BUG-046: 被动方自动镜像布局方向
            // 发起连接方的布局为准，被动方强制接受镜像值
            let mut layout_was_changed = false;
            let mut new_layout_str: Option<String> = None;
            if let Some(ref peer_layout_str) = peer_layout {
                let peer_layout_enum = crate::config::ScreenLayout::from_layout_string(peer_layout_str);
                let mirrored = peer_layout_enum.mirror();
                let mirrored_str = mirrored.to_layout_string().to_string();
                // 读取本端当前布局
                let local_layout_enum = crate::config::ScreenLayout::from_layout_string(
                    &local_info.layout_direction.clone().unwrap_or_default()
                );
                if local_layout_enum != mirrored {
                    log::info!(
                        "[Handshake][LayoutSync] 被动方布局自动镜像: {:?} → {:?} (对端={:?})",
                        local_layout_enum, mirrored, peer_layout_enum
                    );
                    layout_was_changed = true;
                } else {
                    log::info!(
                        "[Handshake][LayoutSync] 被动方布局已互补，无需调整: local={:?}, peer={:?}",
                        local_layout_enum, peer_layout_enum
                    );
                }
                new_layout_str = Some(mirrored_str);
            }

            (
                HandshakeInfo {
                    version,
                    platform,
                    screen_info: desktop,
                    tls_enabled: peer_tls,
                    layout_direction: peer_layout,
                },
                tx,
                layout_was_changed,
                new_layout_str,
            )
        }
        _ => {
            let _ = event_tx.send(NetworkEvent::Error("握手失败".to_string()));
            return;
        }
    };

    let (peer_handshake_info, write_tx, layout_was_changed, new_layout_str) = peer_info;
    let _ = event_tx.send(NetworkEvent::PeerHandshake(peer_handshake_info));

    // v0.1.60 BUG-046: 如果布局被镜像调整，发送 LayoutSynced 事件
    if let Some(layout_str) = new_layout_str {
        let _ = event_tx.send(NetworkEvent::LayoutSynced {
            new_layout: layout_str,
            was_changed: layout_was_changed,
        });
    }

    *state.lock().await = ConnectionState::Connected;
    let _ = event_tx.send(NetworkEvent::ConnectionStateChanged(ConnectionState::Connected));

    *last_heartbeat.lock().await = Instant::now();

    // 读取循环
    read_loop(reader, event_tx, state, last_heartbeat, &write_tx).await;
}

/// 读取循环：持续从 TCP 流读取帧并分发
async fn read_loop(
    mut reader: tokio::io::ReadHalf<TcpStream>,
    event_tx: &broadcast::Sender<NetworkEvent>,
    state: &Arc<Mutex<ConnectionState>>,
    last_heartbeat: &Arc<Mutex<Instant>>,
    write_tx: &mpsc::Sender<KvmFrame>,
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
                            dispatch_frame(&frame, event_tx, last_heartbeat, write_tx).await;
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
    write_tx: &mpsc::Sender<KvmFrame>,
) {
    match frame {
        KvmFrame::HeartbeatAck => {
            *last_heartbeat.lock().await = Instant::now();
        }
        KvmFrame::Heartbeat => {
            // 收到心跳：更新活跃时间并通过 channel 回复 HeartbeatAck（无锁）
            *last_heartbeat.lock().await = Instant::now();
            if let Err(e) = write_tx.send(KvmFrame::HeartbeatAck).await {
                log::warn!("[心跳] 回复 HeartbeatAck 失败: {}", e);
            }
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

    // ── Bug 1（BUG-018）: 握手 TLS 状态正确互传 TDD ──────────────────────────

    #[test]
    fn test_handshake_payload_carries_tls_enabled() {
        // 期望：screen_info_bytes 序列化的内容能被 parse_payload 还原出 tls_enabled
        let info = HandshakeInfo::new_with_tls(
            Platform::MacOS,
            VirtualDesktop::from_monitors(vec![]),
            true, // ← TLS 开启
        );
        let bytes = info.screen_info_bytes();
        let (_, tls_enabled, _) = HandshakeInfo::parse_payload(&bytes);
        assert!(tls_enabled, "wire payload 必须携带 tls_enabled=true");
    }

    #[test]
    fn test_handshake_payload_tls_false_roundtrip() {
        let info = HandshakeInfo::new_with_tls(
            Platform::Windows,
            VirtualDesktop::from_monitors(vec![]),
            false,
        );
        let bytes = info.screen_info_bytes();
        let (_, tls_enabled, _) = HandshakeInfo::parse_payload(&bytes);
        assert!(!tls_enabled);
    }

    #[test]
    fn test_handshake_payload_legacy_compat_old_format() {
        // 旧版客户端只发 VirtualDesktop JSON，不含 tls_enabled 字段
        // 新 parse_payload 必须能优雅回退，tls_enabled 默认 false
        use crate::platform::MonitorInfo;
        let legacy_desktop = VirtualDesktop::from_monitors(vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }]);
        let legacy_bytes = serde_json::to_vec(&legacy_desktop).unwrap();
        let (desktop, tls_enabled, layout_dir) = HandshakeInfo::parse_payload(&legacy_bytes);
        assert_eq!(desktop.total_width, legacy_desktop.total_width);
        assert!(!tls_enabled, "旧格式 wire 默认 tls_enabled=false");
        assert!(layout_dir.is_none(), "旧格式无 layout_direction");
    }

    #[test]
    fn test_handshake_payload_garbage_returns_default() {
        // 完全无法解析的字节，返回默认 desktop + false
        let garbage = b"not a valid json";
        let (_desktop, tls_enabled, layout_dir) = HandshakeInfo::parse_payload(garbage);
        assert!(!tls_enabled);
        assert!(layout_dir.is_none());
    }

    #[test]
    fn test_handshake_payload_carries_layout_direction() {
        // v0.1.60 BUG-046: 握手 payload 正确携带并还原 layout_direction
        let info = HandshakeInfo::new_with_tls_and_layout(
            Platform::MacOS,
            VirtualDesktop::from_monitors(vec![]),
            true,
            Some("LocalLeftOfRemote".to_string()),
        );
        let bytes = info.screen_info_bytes();
        let (_, tls_enabled, layout_dir) = HandshakeInfo::parse_payload(&bytes);
        assert!(tls_enabled);
        assert_eq!(layout_dir, Some("LocalLeftOfRemote".to_string()));
    }

    #[test]
    fn test_handshake_payload_layout_direction_none_compat() {
        // layout_direction = None 时，序列化不包含该字段（skip_serializing_if）
        // 反序列化旧版 payload 应得 None
        let info = HandshakeInfo::new_with_tls(
            Platform::Windows,
            VirtualDesktop::from_monitors(vec![]),
            false,
        );
        assert!(info.layout_direction.is_none());
        let bytes = info.screen_info_bytes();
        let (_, _, layout_dir) = HandshakeInfo::parse_payload(&bytes);
        assert!(layout_dir.is_none(), "None 不应序列化，反序列化也应得 None");
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

    // ── BUG-043 (v0.1.59) F3 纵深防御回归测试 ──────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_apply_tcp_keepalive_succeeds_on_live_stream() {
        // 起一个临时监听器，建立真实 TcpStream 后调 apply_tcp_keepalive，必须返回 Ok
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = listener.accept().await;
        });
        let stream = TcpStream::connect(addr).await.unwrap();
        let res = apply_tcp_keepalive(&stream);
        assert!(res.is_ok(), "apply_tcp_keepalive 在真实 TcpStream 上应成功，得到 {:?}", res);
    }

    #[test]
    fn test_keepalive_constants_match_plan_spec() {
        // plan 要求 idle=5s / interval=3s / retries=2，硬编码守门
        assert_eq!(TCP_KEEPALIVE_IDLE_SECS, 5);
        assert_eq!(TCP_KEEPALIVE_INTERVAL_SECS, 3);
        assert_eq!(TCP_KEEPALIVE_RETRIES, 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tcp_listen_and_connect() {
        let (mgr, mut rx) = ConnectionManager::new(0);
        let local_info = HandshakeInfo::new_with_tls(
            Platform::MacOS,
            VirtualDesktop::from_monitors(vec![]),
            false,
        );
        let addr = mgr.start_listening(local_info).await.unwrap();

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
