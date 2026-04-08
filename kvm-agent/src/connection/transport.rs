/// TCP 传输层子模块
///
/// 封装底层 TCP 连接的握手、帧收发、双向心跳、TCP Keep-Alive 逻辑。
/// 两端对等：连接建立后双方逻辑完全对称，不区分主控/被控角色。
///
/// 心跳策略（仿付费版 ShareMouse）：
/// - 应用层心跳：每 5 秒发送 Heartbeat，对端回复 HeartbeatAck
/// - 连续 15 秒（3 次）未收到 HeartbeatAck → 判定断线
/// - TCP 读超时设置为 20 秒（大于心跳超时阈值，避免误触发）
/// - TCP Keep-Alive：系统层兜底，探测间隔 30 秒

use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::protocol::{KvmFrame, Platform, PROTOCOL_VERSION, read_frame, write_frame};
use super::error::{ConnectError, ErrorGuide};

// ── 心跳参数 ──────────────────────────────────────────────────────────────
/// 心跳发送间隔（5 秒）
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// 心跳超时阈值（15 秒 = 3 次心跳未回复）
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
/// TCP 读超时（20 秒，大于心跳超时阈值）
const READ_TIMEOUT: Duration = Duration::from_secs(20);

// ── TCP Keep-Alive 参数 ────────────────────────────────────────────────────
/// Keep-Alive 探测间隔（30 秒）
const KEEPALIVE_INTERVAL_SECS: u64 = 30;
/// Keep-Alive 探测次数
const KEEPALIVE_RETRIES: u32 = 3;

/// 连接状态（线程安全共享）
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub peer_platform: Option<String>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self { connected: false, peer_ip: None, peer_platform: None }
    }
}

pub type SharedState = Arc<Mutex<ConnectionState>>;
pub type StopSignal = Arc<Mutex<bool>>;

/// 创建共享状态
pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(ConnectionState::default()))
}

/// 创建停止信号
pub fn new_stop_signal() -> StopSignal {
    Arc::new(Mutex::new(false))
}

/// 对等连接模式
pub enum PeerMode {
    /// 服务端：监听端口，等待对端连接
    Server,
    /// 客户端：主动连接对端
    Client { peer_ip: String, port: u16 },
}

/// 启动对等连接（服务端或客户端），阻塞直到连接断开
///
/// 连接建立后双方逻辑完全对称：
/// - 双方都发送心跳（Heartbeat），收到后回复（HeartbeatAck）
/// - 双方都可以发送键鼠帧
/// - 双方都可以接收并执行键鼠帧
pub fn run_peer_connection(
    mode: PeerMode,
    state: SharedState,
    stop_signal: StopSignal,
    frame_sender: impl Fn() -> Option<KvmFrame> + Send + 'static,
    input_handler: impl Fn(KvmFrame) + Send + Sync + 'static,
) -> Result<(), ConnectError> {
    match mode {
        PeerMode::Server => run_server(state, stop_signal, frame_sender, input_handler),
        PeerMode::Client { peer_ip, port } => {
            run_client(&peer_ip, port, state, stop_signal, frame_sender, input_handler)
        }
    }
}

// ── 服务端（接受方）────────────────────────────────────────────────────────

/// 启动 TCP 服务端监听，阻塞直到接受一个连接并处理完毕
fn run_server(
    state: SharedState,
    stop_signal: StopSignal,
    frame_sender: impl Fn() -> Option<KvmFrame> + Send + 'static,
    input_handler: impl Fn(KvmFrame) + Send + Sync + 'static,
) -> Result<(), ConnectError> {
    let listener = TcpListener::bind("0.0.0.0:8889")
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AddrInUse {
                ConnectError::PortInUse { port: 8889 }
            } else {
                ConnectError::Unknown { message: format!("绑定端口 8889 失败: {}", e) }
            }
        })?;

    listener.set_nonblocking(true)
        .map_err(|e| ConnectError::Unknown { message: e.to_string() })?;

    println!("[传输层] 监听端口 8889，等待对端连接...");

    loop {
        if *stop_signal.lock().unwrap() {
            return Ok(());
        }

        match listener.accept() {
            Ok((stream, addr)) => {
                let peer_ip = addr.ip().to_string();
                println!("[传输层] 新连接来自: {}", peer_ip);
                {
                    let mut s = state.lock().unwrap();
                    s.connected = true;
                    s.peer_ip = Some(peer_ip.clone());
                }

                // 启用 TCP Keep-Alive
                apply_keepalive(&stream);

                // 设置读超时
                if stream.set_read_timeout(Some(READ_TIMEOUT)).is_err() {
                    let mut s = state.lock().unwrap();
                    s.connected = false;
                    s.peer_ip = None;
                    return Err(ConnectError::Unknown { message: "设置读超时失败".to_string() });
                }

                // 服务端握手：先读 Handshake，再发 HandshakeAck
                let peer_platform = match perform_handshake_server(&stream) {
                    Ok(p) => p,
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.connected = false;
                        s.peer_ip = None;
                        return Err(e);
                    }
                };
                state.lock().unwrap().peer_platform = Some(peer_platform);

                // 运行对等会话
                let result = run_session(
                    stream,
                    Arc::clone(&state),
                    Arc::clone(&stop_signal),
                    frame_sender,
                    input_handler,
                );

                {
                    let mut s = state.lock().unwrap();
                    s.connected = false;
                    s.peer_ip = None;
                    s.peer_platform = None;
                }

                return result;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("[传输层] 接受连接失败: {}", e);
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

// ── 客户端（发起方）────────────────────────────────────────────────────────

/// 启动 TCP 客户端，主动连接对端，阻塞直到连接断开
fn run_client(
    peer_ip: &str,
    port: u16,
    state: SharedState,
    stop_signal: StopSignal,
    frame_sender: impl Fn() -> Option<KvmFrame> + Send + 'static,
    input_handler: impl Fn(KvmFrame) + Send + Sync + 'static,
) -> Result<(), ConnectError> {
    let addr = format!("{}:{}", peer_ip, port);
    println!("[传输层] 连接对端: {}", addr);

    let stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|_| ConnectError::Unknown {
            message: format!("无效地址: {}", addr),
        })?,
        Duration::from_secs(10),
    ).map_err(|e| ErrorGuide::from_io_error(&e, peer_ip, port))?;

    // 启用 TCP Keep-Alive
    apply_keepalive(&stream);

    // 设置读超时
    stream.set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|e| ConnectError::Unknown { message: format!("设置读超时失败: {}", e) })?;

    {
        let mut s = state.lock().unwrap();
        s.connected = true;
        s.peer_ip = Some(peer_ip.to_string());
    }

    // 客户端握手：先发 Handshake，再读 HandshakeAck
    let peer_platform = match perform_handshake_client(&stream) {
        Ok(p) => p,
        Err(e) => {
            let mut s = state.lock().unwrap();
            s.connected = false;
            s.peer_ip = None;
            return Err(e);
        }
    };
    state.lock().unwrap().peer_platform = Some(peer_platform);

    let result = run_session(
        stream,
        Arc::clone(&state),
        Arc::clone(&stop_signal),
        frame_sender,
        input_handler,
    );

    {
        let mut s = state.lock().unwrap();
        s.connected = false;
        s.peer_ip = None;
        s.peer_platform = None;
    }

    result
}

// ── 握手 ──────────────────────────────────────────────────────────────────

/// 客户端握手：发 Handshake → 读 HandshakeAck
fn perform_handshake_client(stream: &TcpStream) -> Result<String, ConnectError> {
    let mut s = stream.try_clone()
        .map_err(|e| ConnectError::Unknown { message: format!("克隆流失败: {}", e) })?;

    // 发送 Handshake
    let handshake = KvmFrame::Handshake {
        version: PROTOCOL_VERSION,
        platform: Platform::current(),
    };
    write_frame(&mut s, &handshake)
        .map_err(|e| ConnectError::HandshakeFailed { reason: e })?;

    // 读取 HandshakeAck
    match read_frame(&mut s).map_err(|e| ConnectError::HandshakeFailed { reason: e })? {
        KvmFrame::HandshakeAck { version, platform } => {
            if version != PROTOCOL_VERSION {
                return Err(ConnectError::ProtocolMismatch {
                    local_version: PROTOCOL_VERSION,
                    remote_version: version,
                });
            }
            println!("[传输层] 握手完成，对端平台: {:?}", platform);
            Ok(format!("{:?}", platform))
        }
        other => Err(ConnectError::HandshakeFailed {
            reason: format!("期望 HandshakeAck，收到: {:?}", other),
        }),
    }
}

/// 服务端握手：读 Handshake → 发 HandshakeAck
fn perform_handshake_server(stream: &TcpStream) -> Result<String, ConnectError> {
    let mut s = stream.try_clone()
        .map_err(|e| ConnectError::Unknown { message: format!("克隆流失败: {}", e) })?;

    match read_frame(&mut s).map_err(|e| ConnectError::HandshakeFailed { reason: e })? {
        KvmFrame::Handshake { version, platform } => {
            if version != PROTOCOL_VERSION {
                return Err(ConnectError::ProtocolMismatch {
                    local_version: PROTOCOL_VERSION,
                    remote_version: version,
                });
            }
            let ack = KvmFrame::HandshakeAck {
                version: PROTOCOL_VERSION,
                platform: Platform::current(),
            };
            write_frame(&mut s, &ack)
                .map_err(|e| ConnectError::HandshakeFailed { reason: e })?;
            println!("[传输层] 握手完成，对端平台: {:?}", platform);
            Ok(format!("{:?}", platform))
        }
        other => Err(ConnectError::HandshakeFailed {
            reason: format!("期望 Handshake，收到: {:?}", other),
        }),
    }
}

// ── 会话层（握手完成后，双方对称）────────────────────────────────────────

/// 运行对等会话（握手完成后调用）
///
/// 启动三个并发任务：
/// 1. 心跳发送线程：每 5 秒发送 Heartbeat（有数据时跳过）
/// 2. 帧发送线程：从 frame_sender 取帧并发送
/// 3. 主线程（接收循环）：读取帧，处理心跳/输入，检测超时
///
/// 所有写操作通过统一的写通道（write_tx）序列化，避免多线程写冲突。
fn run_session(
    stream: TcpStream,
    state: SharedState,
    stop_signal: StopSignal,
    frame_sender: impl Fn() -> Option<KvmFrame> + Send + 'static,
    input_handler: impl Fn(KvmFrame) + Send + Sync + 'static,
) -> Result<(), ConnectError> {
    // 统一写通道：所有写操作（心跳、帧发送、HeartbeatAck）都通过此通道
    let (write_tx, write_rx) = std::sync::mpsc::sync_channel::<KvmFrame>(512);
    let write_tx_hb = write_tx.clone();   // 心跳线程用
    let write_tx_ack = write_tx.clone();  // 接收循环回复 ACK 用

    // 克隆读流
    let mut read_stream = stream.try_clone()
        .map_err(|e| ConnectError::Unknown { message: format!("克隆流失败: {}", e) })?;
    let mut write_stream = stream;

    // 共享：最后收到 HeartbeatAck 的时间
    let last_ack = Arc::new(Mutex::new(Instant::now()));
    let last_ack_recv = Arc::clone(&last_ack);

    // 共享：最后通过 write_tx 发送任何帧的时间（有数据时不需要额外心跳）
    let last_send = Arc::new(Mutex::new(Instant::now()));
    let last_send_hb = Arc::clone(&last_send);
    let last_send_tx = Arc::clone(&last_send);

    // ── 统一写线程 ────────────────────────────────────────────────────────
    let stop_write = Arc::clone(&stop_signal);
    let write_thread = std::thread::spawn(move || {
        loop {
            match write_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    if write_frame(&mut write_stream, &frame).is_err() {
                        eprintln!("[写线程] 写入失败，连接可能已断开");
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if *stop_write.lock().unwrap() { break; }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    // ── 心跳发送线程 ──────────────────────────────────────────────────────
    let stop_hb = Arc::clone(&stop_signal);
    let hb_thread = std::thread::spawn(move || {
        loop {
            std::thread::sleep(HEARTBEAT_INTERVAL);
            if *stop_hb.lock().unwrap() { break; }

            // 如果最近有数据发送，跳过本次心跳（有数据时不需要额外心跳）
            if last_send_hb.lock().unwrap().elapsed() < HEARTBEAT_INTERVAL {
                continue;
            }

            if write_tx_hb.send(KvmFrame::Heartbeat).is_err() {
                eprintln!("[心跳] 发送通道已关闭");
                break;
            }
            *last_send_hb.lock().unwrap() = Instant::now();
        }
    });

    // ── 帧发送线程 ────────────────────────────────────────────────────────
    let stop_tx = Arc::clone(&stop_signal);
    let tx_thread = std::thread::spawn(move || {
        loop {
            if *stop_tx.lock().unwrap() { break; }
            if let Some(frame) = frame_sender() {
                if write_tx.send(frame).is_err() {
                    eprintln!("[发送] 写通道已关闭");
                    break;
                }
                *last_send_tx.lock().unwrap() = Instant::now();
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    });

    // ── 接收主循环 ────────────────────────────────────────────────────────
    let input_handler = Arc::new(input_handler);
    let mut disconnect_reason: Option<ConnectError> = None;

    loop {
        if *stop_signal.lock().unwrap() { break; }

        // 检查心跳超时（15 秒未收到 HeartbeatAck）
        let elapsed_since_ack = last_ack.lock().unwrap().elapsed();
        if elapsed_since_ack > HEARTBEAT_TIMEOUT {
            let elapsed = elapsed_since_ack.as_secs();
            eprintln!("[心跳] 超时（{} 秒未收到 ACK），断开连接", elapsed);
            disconnect_reason = Some(ConnectError::HeartbeatTimeout { last_seen_secs: elapsed });
            break;
        }

        match read_frame(&mut read_stream) {
            Ok(frame) => {
                match frame {
                    KvmFrame::Heartbeat => {
                        // 收到心跳，立即通过写通道回复 HeartbeatAck
                        let _ = write_tx_ack.send(KvmFrame::HeartbeatAck);
                    }
                    KvmFrame::HeartbeatAck => {
                        // 收到心跳响应，更新时间戳
                        *last_ack_recv.lock().unwrap() = Instant::now();
                    }
                    KvmFrame::MouseMove { .. }
                    | KvmFrame::MouseButton { .. }
                    | KvmFrame::MouseScroll { .. }
                    | KvmFrame::KeyEvent { .. } => {
                        input_handler(frame);
                    }
                    KvmFrame::SwitchControl { .. } => {
                        println!("[传输层] 收到切换控制权请求");
                    }
                    KvmFrame::Clipboard { text } => {
                        println!("[传输层] 收到剪贴板内容，长度: {}", text.len());
                    }
                    _ => {}
                }
            }
            Err(e) => {
                // 读超时（20 秒）或连接断开
                let elapsed = last_ack.lock().unwrap().elapsed().as_secs();
                if elapsed > HEARTBEAT_TIMEOUT.as_secs() {
                    disconnect_reason = Some(ConnectError::HeartbeatTimeout { last_seen_secs: elapsed });
                } else {
                    eprintln!("[传输层] 读取帧失败: {}", e);
                }
                break;
            }
        }
    }

    // 等待子线程退出
    let _ = hb_thread.join();
    let _ = tx_thread.join();
    let _ = write_thread.join();

    if let Some(err) = disconnect_reason {
        Err(err)
    } else {
        Ok(())
    }
}

// ── TCP Keep-Alive ────────────────────────────────────────────────────────

/// 在 TCP 连接上启用系统层 Keep-Alive 保活
///
/// 设置探测间隔 30 秒、探测次数 3 次，覆盖系统默认的 2 小时间隔。
/// 作为应用层心跳（5 秒）的兜底机制。
fn apply_keepalive(stream: &TcpStream) {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    apply_keepalive_unix(stream);

    #[cfg(target_os = "windows")]
    apply_keepalive_windows(stream);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn apply_keepalive_unix(stream: &TcpStream) {
    use std::os::unix::io::AsRawFd;
    let fd = stream.as_raw_fd();

    unsafe {
        // 启用 SO_KEEPALIVE
        let optval: libc::c_int = 1;
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_KEEPALIVE,
            &optval as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );

        // TCP_KEEPIDLE（macOS）/ TCP_KEEPIDLE（Linux）：空闲多久后开始探测（30 秒）
        let idle: libc::c_int = KEEPALIVE_INTERVAL_SECS as libc::c_int;
        #[cfg(target_os = "macos")]
        libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPALIVE,
            &idle as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
        #[cfg(target_os = "linux")]
        libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPIDLE,
            &idle as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );

        // TCP_KEEPINTVL：探测间隔（10 秒）
        let intvl: libc::c_int = 10;
        libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPINTVL,
            &intvl as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );

        // TCP_KEEPCNT：探测次数（3 次）
        let cnt: libc::c_int = KEEPALIVE_RETRIES as libc::c_int;
        libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPCNT,
            &cnt as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
    }
    println!("[传输层] TCP Keep-Alive 已启用（间隔 {}s，探测 {} 次）",
        KEEPALIVE_INTERVAL_SECS, KEEPALIVE_RETRIES);
}

#[cfg(target_os = "windows")]
fn apply_keepalive_windows(stream: &TcpStream) {
    use std::os::windows::io::AsRawSocket;

    let socket = stream.as_raw_socket();

    // Windows 使用 SIO_KEEPALIVE_VALS ioctl 设置 Keep-Alive 参数
    // tcp_keepalive 结构体：on_off(4B) + keepalivetime(4B,ms) + keepaliveinterval(4B,ms)
    let keepalive_data: [u32; 3] = [
        1,                                              // on_off = 1（启用）
        (KEEPALIVE_INTERVAL_SECS * 1000) as u32,       // keepalivetime（毫秒）
        10_000,                                         // keepaliveinterval = 10 秒（毫秒）
    ];

    unsafe {
        let mut bytes_returned: u32 = 0;
        // SIO_KEEPALIVE_VALS = 0x98000004
        let result = windows_sys::Win32::Networking::WinSock::WSAIoctl(
            socket as _,
            0x98000004u32,
            keepalive_data.as_ptr() as *const _,
            (std::mem::size_of::<u32>() * 3) as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
            None,
        );
        if result == 0 {
            println!("[传输层] TCP Keep-Alive 已启用（间隔 {}s，探测 {} 次）",
                KEEPALIVE_INTERVAL_SECS, KEEPALIVE_RETRIES);
        } else {
            eprintln!("[传输层] TCP Keep-Alive 设置失败");
        }
    }
}
