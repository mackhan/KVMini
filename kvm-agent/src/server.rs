use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::protocol::{
    KvmFrame, Platform, PROTOCOL_VERSION,
    read_frame, write_frame,
};

/// 连接状态，供托盘显示使用
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub peer_hostname: Option<String>,
    pub role: AppRole,
}

/// 应用角色
#[derive(Debug, Clone, PartialEq)]
pub enum AppRole {
    /// 主控端：主动连接对端，采集并发送键鼠事件
    Controller,
    /// 被控端：监听端口，接收并执行键鼠事件
    Controlled,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            connected: false,
            peer_ip: None,
            peer_hostname: None,
            role: AppRole::Controlled, // Mac 端默认为被控端
        }
    }
}

/// 全局连接状态（线程安全）
pub type SharedState = Arc<Mutex<ConnectionState>>;

/// 停止信号（用于优雅关闭服务）
pub type StopSignal = Arc<Mutex<bool>>;

/// 启动 TCP 被控端服务，阻塞运行
/// 监听端口 8889，等待主控端连接
/// 每次只接受一个连接，断开后重新等待
pub fn run_tcp_server(
    state: SharedState,
    stop_signal: StopSignal,
    input_handler: impl Fn(KvmFrame) + Send + Sync + 'static,
) -> Result<(), String> {
    let listener = TcpListener::bind("0.0.0.0:8889")
        .map_err(|e| format!("TCP 绑定端口 8889 失败: {}", e))?;

    // 设置非阻塞，以便检查停止信号
    listener.set_nonblocking(true)
        .map_err(|e| format!("设置非阻塞失败: {}", e))?;

    println!("[TCP] 被控端监听端口 8889，等待主控端连接...");

    let input_handler = Arc::new(input_handler);

    loop {
        // 检查停止信号
        {
            let stop = stop_signal.lock().unwrap();
            if *stop {
                println!("[TCP] 收到停止信号，服务端退出");
                return Ok(());
            }
        }

        match listener.accept() {
            Ok((stream, addr)) => {
                let peer_ip = addr.ip().to_string();
                println!("[TCP] 新连接来自: {}", peer_ip);

                // 更新连接状态
                {
                    let mut s = state.lock().unwrap();
                    s.connected = true;
                    s.peer_ip = Some(peer_ip.clone());
                }

                let state_clone = Arc::clone(&state);
                let handler_clone = Arc::clone(&input_handler);
                let stop_clone = Arc::clone(&stop_signal);

                // 处理连接（阻塞直到断开）
                handle_controlled_connection(stream, state_clone, handler_clone, stop_clone);

                // 连接断开后重置状态
                {
                    let mut s = state.lock().unwrap();
                    s.connected = false;
                    s.peer_ip = None;
                    s.peer_hostname = None;
                }
                println!("[TCP] 连接已断开，重新等待...");
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 没有新连接，短暂休眠后继续
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("[TCP] 接受连接失败: {}", e);
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

/// 处理被控端单个 TCP 连接的完整生命周期
fn handle_controlled_connection(
    mut stream: TcpStream,
    state: SharedState,
    input_handler: Arc<impl Fn(KvmFrame) + Send + Sync + 'static>,
    stop_signal: StopSignal,
) {
    // 设置读超时（用于心跳检测）
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(8))) {
        eprintln!("[TCP] 设置读超时失败: {}", e);
        return;
    }

    // ── 握手流程：等待主控端发送 Handshake 帧 ──
    let handshake_frame = match read_frame(&mut stream) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[TCP] 读取握手帧失败: {}", e);
            return;
        }
    };

    match handshake_frame {
        KvmFrame::Handshake { version, platform } => {
            // 版本校验
            if version != PROTOCOL_VERSION {
                eprintln!(
                    "[TCP] 协议版本不兼容：对端版本 {}，本端版本 {}，拒绝连接",
                    version, PROTOCOL_VERSION
                );
                return;
            }
            println!("[TCP] 握手成功，对端平台: {:?}，协议版本: {}", platform, version);
            {
                let mut s = state.lock().unwrap();
                s.peer_hostname = Some(format!("{:?}", platform));
            }
        }
        other => {
            eprintln!("[TCP] 期望握手帧，收到: {:?}，拒绝连接", other);
            return;
        }
    }

    // 回复 HandshakeAck
    let ack = KvmFrame::HandshakeAck {
        version: PROTOCOL_VERSION,
        platform: Platform::current(),
    };
    if let Err(e) = write_frame(&mut stream, &ack) {
        eprintln!("[TCP] 发送 HandshakeAck 失败: {}", e);
        return;
    }
    println!("[TCP] 已发送 HandshakeAck，连接建立");

    // ── 克隆写端，用于心跳发送线程 ──
    let mut write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[TCP] 克隆流失败: {}", e);
            return;
        }
    };

    let stop_for_hb = Arc::clone(&stop_signal);
    let hb_thread = std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(3));
            {
                let stop = stop_for_hb.lock().unwrap();
                if *stop {
                    break;
                }
            }
            if let Err(e) = write_frame(&mut write_stream, &KvmFrame::HeartbeatAck) {
                eprintln!("[TCP] 发送心跳响应失败: {}，心跳线程退出", e);
                break;
            }
        }
    });

    // ── 主循环：读取并处理帧 ──
    let mut last_heartbeat = Instant::now();
    loop {
        // 检查停止信号
        {
            let stop = stop_signal.lock().unwrap();
            if *stop {
                break;
            }
        }

        match read_frame(&mut stream) {
            Ok(frame) => {
                last_heartbeat = Instant::now();
                match &frame {
                    KvmFrame::Heartbeat => {
                        // 心跳正常，不需要额外处理（心跳线程会回复 HeartbeatAck）
                    }
                    KvmFrame::MouseMove { .. }
                    | KvmFrame::MouseButton { .. }
                    | KvmFrame::MouseScroll { .. }
                    | KvmFrame::KeyEvent { .. } => {
                        input_handler(frame);
                    }
                    KvmFrame::SwitchControl { .. } => {
                        // 切换控制权请求，暂时忽略
                        println!("[TCP] 收到切换控制权请求");
                    }
                    KvmFrame::Clipboard { text } => {
                        println!("[TCP] 收到剪贴板内容，长度: {}", text.len());
                        // TODO: 实现剪贴板同步
                    }
                    other => {
                        println!("[TCP] 收到未处理帧: {:?}", other);
                    }
                }

                // 检查心跳超时（10 秒）
                if last_heartbeat.elapsed() > Duration::from_secs(10) {
                    eprintln!("[TCP] 心跳超时，断开连接");
                    break;
                }
            }
            Err(e) => {
                // 读超时或连接断开
                if last_heartbeat.elapsed() > Duration::from_secs(10) {
                    eprintln!("[TCP] 心跳超时，断开连接");
                } else {
                    eprintln!("[TCP] 读取帧失败: {}", e);
                }
                break;
            }
        }
    }

    let _ = hb_thread.join();
}

/// 启动 TCP 主控端连接，主动连接对端
/// 连接成功后发送 Handshake 帧，等待 HandshakeAck
pub fn run_tcp_client(
    peer_addr: String,
    state: SharedState,
    stop_signal: StopSignal,
    event_receiver: impl Fn() -> Option<KvmFrame> + Send + 'static,
) -> Result<(), String> {
    let addr = format!("{}:8889", peer_addr);
    println!("[TCP] 主控端尝试连接: {}", addr);

    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("连接 {} 失败: {}", addr, e))?;

    println!("[TCP] 已连接到: {}", addr);

    // 更新连接状态
    {
        let mut s = state.lock().unwrap();
        s.connected = true;
        s.peer_ip = Some(peer_addr.clone());
    }

    // ── 握手流程：主控端先发 Handshake ──
    let handshake = KvmFrame::Handshake {
        version: PROTOCOL_VERSION,
        platform: Platform::current(),
    };
    write_frame(&mut stream, &handshake)
        .map_err(|e| format!("发送握手帧失败: {}", e))?;

    // 等待 HandshakeAck
    let ack_frame = read_frame(&mut stream)
        .map_err(|e| format!("读取握手响应失败: {}", e))?;

    match ack_frame {
        KvmFrame::HandshakeAck { version, platform } => {
            if version != PROTOCOL_VERSION {
                return Err(format!(
                    "协议版本不兼容：对端版本 {}，本端版本 {}",
                    version, PROTOCOL_VERSION
                ));
            }
            println!("[TCP] 握手完成，对端平台: {:?}", platform);
            {
                let mut s = state.lock().unwrap();
                s.peer_hostname = Some(format!("{:?}", platform));
            }
        }
        other => {
            return Err(format!("期望 HandshakeAck，收到: {:?}", other));
        }
    }

    // ── 主循环：发送键鼠事件 ──
    loop {
        {
            let stop = stop_signal.lock().unwrap();
            if *stop {
                break;
            }
        }

        if let Some(frame) = event_receiver() {
            if let Err(e) = write_frame(&mut stream, &frame) {
                eprintln!("[TCP] 发送事件帧失败: {}，断开连接", e);
                break;
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    // 更新断开状态
    {
        let mut s = state.lock().unwrap();
        s.connected = false;
        s.peer_ip = None;
        s.peer_hostname = None;
    }

    Ok(())
}
