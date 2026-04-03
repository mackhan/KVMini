use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::protocol::{KvmMessage, HandshakePacket, HeartbeatPacket, encode_message, decode_message};

/// 连接状态，供托盘显示使用
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub peer_hostname: Option<String>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            connected: false,
            peer_ip: None,
            peer_hostname: None,
        }
    }
}

/// 全局连接状态（线程安全）
pub type SharedState = Arc<Mutex<ConnectionState>>;

/// 启动 TCP 服务端，阻塞运行
/// 每次只接受一个连接，断开后重新等待
pub fn run_tcp_server(
    hostname: String,
    os_version: String,
    state: SharedState,
    input_handler: impl Fn(crate::protocol::InputEvent) + Send + Sync + 'static,
) -> Result<(), String> {
    let listener = TcpListener::bind("0.0.0.0:9999")
        .map_err(|e| format!("TCP 绑定端口 9999 失败: {}", e))?;

    println!("[TCP] 监听端口 9999，等待控制端连接...");

    let input_handler = Arc::new(input_handler);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_addr = stream.peer_addr()
                    .map(|a| a.ip().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                println!("[TCP] 新连接来自: {}", peer_addr);

                // 更新连接状态
                {
                    let mut s = state.lock().unwrap();
                    s.connected = true;
                    s.peer_ip = Some(peer_addr.clone());
                }

                let state_clone = Arc::clone(&state);
                let handler_clone = Arc::clone(&input_handler);
                let hn = hostname.clone();
                let os = os_version.clone();

                // 在当前线程处理连接（每次只允许一个连接）
                handle_connection(stream, hn, os, state_clone, handler_clone);

                // 连接断开后重置状态
                {
                    let mut s = state.lock().unwrap();
                    s.connected = false;
                    s.peer_ip = None;
                    s.peer_hostname = None;
                }
                println!("[TCP] 连接已断开，重新等待...");
            }
            Err(e) => {
                eprintln!("[TCP] 接受连接失败: {}", e);
            }
        }
    }
    Ok(())
}

/// 处理单个 TCP 连接的完整生命周期
fn handle_connection(
    stream: TcpStream,
    hostname: String,
    os_version: String,
    state: SharedState,
    input_handler: Arc<impl Fn(crate::protocol::InputEvent) + Send + Sync + 'static>,
) {
    // 设置读超时（用于心跳检测）
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(8))) {
        eprintln!("[TCP] 设置读超时失败: {}", e);
        return;
    }

    let mut write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[TCP] 克隆流失败: {}", e);
            return;
        }
    };

    // 发送握手包
    let handshake = KvmMessage::Handshake(HandshakePacket::agent(hostname, os_version));
    if let Ok(msg) = encode_message(&handshake) {
        if let Err(e) = write_stream.write_all(msg.as_bytes()) {
            eprintln!("[TCP] 发送握手包失败: {}", e);
            return;
        }
    }

    let reader = BufReader::new(stream);
    let mut last_heartbeat = Instant::now();

    // 启动心跳发送线程
    let mut write_for_hb = write_stream.try_clone().ok();
    let hb_thread = std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if let Some(ref mut w) = write_for_hb {
                let hb = KvmMessage::Heartbeat(HeartbeatPacket::now());
                if let Ok(msg) = encode_message(&hb) {
                    if w.write_all(msg.as_bytes()).is_err() {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    });

    // 主循环：读取消息
    for line in reader.lines() {
        match line {
            Ok(text) => {
                last_heartbeat = Instant::now();
                match decode_message(&text) {
                    Ok(KvmMessage::Handshake(pkt)) => {
                        println!("[TCP] 握手成功，对端: {} ({})", pkt.hostname, pkt.os);
                        let mut s = state.lock().unwrap();
                        s.peer_hostname = Some(pkt.hostname);
                    }
                    Ok(KvmMessage::Heartbeat(_)) => {
                        // 心跳正常，不需要额外处理
                    }
                    Ok(KvmMessage::Input(pkt)) => {
                        input_handler(pkt.event);
                    }
                    Ok(KvmMessage::Disconnect { reason }) => {
                        println!("[TCP] 对端主动断开: {}", reason);
                        break;
                    }
                    Err(e) => {
                        eprintln!("[TCP] 消息解析失败: {}", e);
                    }
                }

                // 检查心跳超时（6 秒）
                if last_heartbeat.elapsed() > Duration::from_secs(6) {
                    eprintln!("[TCP] 心跳超时，断开连接");
                    break;
                }
            }
            Err(e) => {
                // 读超时或连接断开
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                {
                    if last_heartbeat.elapsed() > Duration::from_secs(6) {
                        eprintln!("[TCP] 心跳超时，断开连接");
                    }
                } else {
                    eprintln!("[TCP] 读取错误: {}", e);
                }
                break;
            }
        }
    }

    // 等待心跳线程结束（忽略错误）
    let _ = hb_thread.join();
}
