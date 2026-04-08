/// TCP 传输层子模块
///
/// 封装底层 TCP 连接的握手、帧收发、心跳检测逻辑。
/// 与平台无关，Mac 和 Windows 共用相同的传输协议。

use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::protocol::{KvmFrame, Platform, PROTOCOL_VERSION, read_frame, write_frame};
use super::error::{ConnectError, ErrorGuide};

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

// ── 被控端（TCP 服务端）────────────────────────────────────────────────────

/// 启动 TCP 被控端监听，阻塞运行
/// 端口：8889，每次只接受一个连接，断开后重新等待
pub fn run_server(
    state: SharedState,
    stop_signal: StopSignal,
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

    println!("[传输层] 被控端监听端口 8889...");
    let input_handler = Arc::new(input_handler);

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
                handle_server_connection(
                    stream,
                    Arc::clone(&state),
                    Arc::clone(&input_handler),
                    Arc::clone(&stop_signal),
                );
                {
                    let mut s = state.lock().unwrap();
                    s.connected = false;
                    s.peer_ip = None;
                    s.peer_platform = None;
                }
                println!("[传输层] 连接断开，重新等待...");
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

/// 处理单个被控端连接
fn handle_server_connection(
    mut stream: TcpStream,
    state: SharedState,
    input_handler: Arc<impl Fn(KvmFrame) + Send + Sync + 'static>,
    stop_signal: StopSignal,
) {
    if stream.set_read_timeout(Some(Duration::from_secs(8))).is_err() {
        return;
    }

    // 握手：等待主控端发送 Handshake
    match read_frame(&mut stream) {
        Ok(KvmFrame::Handshake { version, platform }) => {
            if version != PROTOCOL_VERSION {
                eprintln!("[传输层] 协议版本不兼容: 对端 v{}，本端 v{}", version, PROTOCOL_VERSION);
                return;
            }
            println!("[传输层] 握手成功，对端平台: {:?}", platform);
            state.lock().unwrap().peer_platform = Some(format!("{:?}", platform));
        }
        Ok(other) => {
            eprintln!("[传输层] 期望握手帧，收到: {:?}", other);
            return;
        }
        Err(e) => {
            eprintln!("[传输层] 读取握手帧失败: {}", e);
            return;
        }
    }

    // 回复 HandshakeAck
    let ack = KvmFrame::HandshakeAck { version: PROTOCOL_VERSION, platform: Platform::current() };
    if write_frame(&mut stream, &ack).is_err() {
        return;
    }

    // 心跳发送线程
    let mut write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let stop_hb = Arc::clone(&stop_signal);
    let hb_thread = std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(3));
            if *stop_hb.lock().unwrap() { break; }
            if write_frame(&mut write_stream, &KvmFrame::HeartbeatAck).is_err() { break; }
        }
    });

    // 主循环：读取并执行输入帧
    let mut last_hb = Instant::now();
    loop {
        if *stop_signal.lock().unwrap() { break; }

        match read_frame(&mut stream) {
            Ok(frame) => {
                last_hb = Instant::now();
                match &frame {
                    KvmFrame::Heartbeat => {}
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
                if last_hb.elapsed() > Duration::from_secs(10) {
                    eprintln!("[传输层] 心跳超时，断开连接");
                    break;
                }
            }
            Err(_) => {
                if last_hb.elapsed() > Duration::from_secs(10) {
                    eprintln!("[传输层] 心跳超时，断开连接");
                }
                break;
            }
        }
    }
    let _ = hb_thread.join();
}

// ── 主控端（TCP 客户端）────────────────────────────────────────────────────

/// 启动 TCP 主控端连接，主动连接对端
pub fn run_client(
    peer_ip: &str,
    port: u16,
    state: SharedState,
    stop_signal: StopSignal,
    event_receiver: impl Fn() -> Option<KvmFrame> + Send + 'static,
) -> Result<(), ConnectError> {
    let addr = format!("{}:{}", peer_ip, port);
    println!("[传输层] 主控端连接: {}", addr);

    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|_| ConnectError::Unknown {
            message: format!("无效地址: {}", addr),
        })?,
        Duration::from_secs(10),
    ).map_err(|e| ErrorGuide::from_io_error(&e, peer_ip, port))?;

    {
        let mut s = state.lock().unwrap();
        s.connected = true;
        s.peer_ip = Some(peer_ip.to_string());
    }

    // 握手：主控端先发 Handshake
    let handshake = KvmFrame::Handshake { version: PROTOCOL_VERSION, platform: Platform::current() };
    write_frame(&mut stream, &handshake)
        .map_err(|e| ConnectError::HandshakeFailed { reason: e.to_string() })?;

    match read_frame(&mut stream)
        .map_err(|e| ConnectError::HandshakeFailed { reason: e.to_string() })?
    {
        KvmFrame::HandshakeAck { version, platform } => {
            if version != PROTOCOL_VERSION {
                return Err(ConnectError::ProtocolMismatch {
                    local_version: PROTOCOL_VERSION,
                    remote_version: version,
                });
            }
            println!("[传输层] 握手完成，对端平台: {:?}", platform);
            state.lock().unwrap().peer_platform = Some(format!("{:?}", platform));
        }
        other => {
            return Err(ConnectError::HandshakeFailed {
                reason: format!("期望 HandshakeAck，收到: {:?}", other),
            });
        }
    }

    // 主循环：发送键鼠事件
    loop {
        if *stop_signal.lock().unwrap() { break; }
        if let Some(frame) = event_receiver() {
            if write_frame(&mut stream, &frame).is_err() {
                break;
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    {
        let mut s = state.lock().unwrap();
        s.connected = false;
        s.peer_ip = None;
        s.peer_platform = None;
    }
    Ok(())
}
