use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio::time::{timeout, Duration};

use crate::protocol::{
    self, KvmFrame, Platform, PROTOCOL_VERSION,
};

/// 被控端服务器状态
#[derive(Debug, Clone, PartialEq)]
pub enum ServerState {
    /// 未启动
    Stopped,
    /// 监听中（等待连接）
    Listening,
    /// 已连接（主控端已接入）
    Connected { peer_addr: String },
    /// 错误
    Error(String),
}

/// 被控端服务器事件（通过 broadcast channel 发送给 UI）
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// 主控端已连接
    ClientConnected { peer_addr: String },
    /// 主控端已断开
    ClientDisconnected,
    /// 收到键鼠事件帧（转发给注入模块）
    InputFrame(KvmFrame),
    /// 收到切换控制权请求
    SwitchControl { edge: crate::protocol::ScreenEdge, pos_ratio: u16 },
    /// 服务器错误
    Error(String),
}

/// 被控端 TCP 服务器
pub struct KvmServer {
    /// 监听地址
    listen_addr: String,
    /// 当前状态
    state: Arc<Mutex<ServerState>>,
    /// 事件广播发送端
    event_tx: broadcast::Sender<ServerEvent>,
    /// 停止信号发送端（oneshot，发送后 accept_loop 退出）
    stop_tx: Option<oneshot::Sender<()>>,
}

impl KvmServer {
    /// 创建新的被控端服务器
    pub fn new(listen_addr: &str) -> (Self, broadcast::Receiver<ServerEvent>) {
        let (event_tx, event_rx) = broadcast::channel(256);
        let server = Self {
            listen_addr: listen_addr.to_string(),
            state: Arc::new(Mutex::new(ServerState::Stopped)),
            event_tx,
            stop_tx: None,
        };
        (server, event_rx)
    }

    /// 启动 TCP 监听服务，返回实际监听地址（支持 :0 动态端口）
    pub async fn start(&mut self) -> Result<SocketAddr, String> {
        // 如果已在运行，先停止
        self.stop().await;

        let listener = TcpListener::bind(&self.listen_addr)
            .await
            .map_err(|e| format!("绑定地址 {} 失败: {}", self.listen_addr, e))?;

        let actual_addr = listener
            .local_addr()
            .map_err(|e| format!("获取监听地址失败: {}", e))?;

        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        self.stop_tx = Some(stop_tx);

        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        *state.lock().await = ServerState::Listening;

        tokio::spawn(async move {
            accept_loop(listener, state, event_tx, stop_rx).await;
        });

        Ok(actual_addr)
    }

    /// 停止服务器
    pub async fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        *self.state.lock().await = ServerState::Stopped;
    }

    /// 获取当前状态
    pub async fn state(&self) -> ServerState {
        self.state.lock().await.clone()
    }

    /// 订阅服务器事件
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.event_tx.subscribe()
    }
}

/// 接受连接的主循环
async fn accept_loop(
    listener: TcpListener,
    state: Arc<Mutex<ServerState>>,
    event_tx: broadcast::Sender<ServerEvent>,
    mut stop_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            biased;
            _ = &mut stop_rx => {
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer_addr)) => {
                        let peer_str = peer_addr.to_string();
                        *state.lock().await = ServerState::Connected {
                            peer_addr: peer_str.clone(),
                        };
                        let _ = event_tx.send(ServerEvent::ClientConnected {
                            peer_addr: peer_str.clone(),
                        });

                        // 处理单个连接（KVMini 同时只支持一个主控端）
                        let (conn_stop_tx, conn_stop_rx) = oneshot::channel::<()>();
                        handle_connection(stream, peer_addr, &event_tx, conn_stop_rx).await;
                        drop(conn_stop_tx);

                        *state.lock().await = ServerState::Listening;
                        let _ = event_tx.send(ServerEvent::ClientDisconnected);
                    }
                    Err(e) => {
                        let _ = event_tx.send(ServerEvent::Error(format!("接受连接失败: {}", e)));
                        break;
                    }
                }
            }
        }
    }
}

/// 处理单个主控端连接
async fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    event_tx: &broadcast::Sender<ServerEvent>,
    mut stop_rx: oneshot::Receiver<()>,
) {
    // 设置 TCP_NODELAY（禁用 Nagle 算法，减少延迟）
    if let Err(e) = stream.set_nodelay(true) {
        let _ = event_tx.send(ServerEvent::Error(format!("设置 TCP_NODELAY 失败: {}", e)));
    }

    // 执行握手，返回握手后剩余的缓冲区数据
    let leftover = match perform_handshake(&mut stream).await {
        Ok(leftover) => leftover,
        Err(e) => {
            let _ = event_tx.send(ServerEvent::Error(format!(
                "与 {} 握手失败: {}",
                peer_addr, e
            )));
            return;
        }
    };

    // 接收事件流
    let mut read_buf = leftover; // 将握手阶段多读的数据放入缓冲区
    let mut tmp = [0u8; 4096];

    // 先处理握手阶段可能已读取的数据
    loop {
        match protocol::decode(&read_buf) {
            Ok((frame, consumed)) => {
                dispatch_frame(frame, event_tx);
                read_buf.drain(..consumed);
            }
            Err(protocol::ProtocolError::Incomplete) => break,
            Err(e) => {
                let _ = event_tx.send(ServerEvent::Error(format!("协议解码错误: {}", e)));
                return;
            }
        }
    }

    loop {
        tokio::select! {
            biased;
            _ = &mut stop_rx => {
                break;
            }
            read_result = timeout(Duration::from_secs(30), stream.read(&mut tmp)) => {
                match read_result {
                    Ok(Ok(0)) => {
                        // 连接正常关闭
                        break;
                    }
                    Ok(Ok(n)) => {
                        read_buf.extend_from_slice(&tmp[..n]);
                        // 解码缓冲区中的所有完整帧
                        loop {
                            match protocol::decode(&read_buf) {
                                Ok((frame, consumed)) => {
                                    dispatch_frame(frame, event_tx);
                                    read_buf.drain(..consumed);
                                }
                                Err(protocol::ProtocolError::Incomplete) => break,
                                Err(e) => {
                                    let _ = event_tx.send(ServerEvent::Error(
                                        format!("协议解码错误: {}", e)
                                    ));
                                    return;
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = event_tx.send(ServerEvent::Error(format!("读取错误: {}", e)));
                        break;
                    }
                    Err(_) => {
                        // 30 秒超时，连接可能已断开
                        let _ = event_tx.send(ServerEvent::Error("连接超时".to_string()));
                        break;
                    }
                }
            }
        }
    }
}

/// 执行握手协议，返回握手后多读的剩余数据
async fn perform_handshake(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    // 发送握手帧
    let handshake = KvmFrame::Handshake {
        version: PROTOCOL_VERSION,
        platform: Platform::current(),
    };
    let bytes = protocol::encode(&handshake);
    stream
        .write_all(&bytes)
        .await
        .map_err(|e| format!("发送握手失败: {}", e))?;

    // 等待握手响应（5 秒超时）
    let mut buf = vec![0u8; 256];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .map_err(|_| "握手超时".to_string())?
        .map_err(|e| format!("读取握手响应失败: {}", e))?;

    if n == 0 {
        return Err("握手时连接关闭".to_string());
    }

    match protocol::decode(&buf[..n]) {
        Ok((KvmFrame::HandshakeAck { version, .. }, consumed)) => {
            if version != PROTOCOL_VERSION {
                return Err(format!(
                    "协议版本不兼容：本端 {}，对端 {}",
                    PROTOCOL_VERSION, version
                ));
            }
            // 返回握手响应之后的剩余数据
            Ok(buf[consumed..n].to_vec())
        }
        Ok((frame, _)) => Err(format!("握手响应类型错误: {:?}", frame)),
        Err(e) => Err(format!("握手响应解码失败: {}", e)),
    }
}

/// 将解码后的帧分发给事件总线
fn dispatch_frame(frame: KvmFrame, event_tx: &broadcast::Sender<ServerEvent>) {
    match &frame {
        KvmFrame::SwitchControl { edge, pos_ratio } => {
            let _ = event_tx.send(ServerEvent::SwitchControl {
                edge: *edge,
                pos_ratio: *pos_ratio,
            });
        }
        KvmFrame::Heartbeat => {
            // 心跳不需要转发给注入模块，直接忽略
        }
        KvmFrame::MouseMove { .. }
        | KvmFrame::MouseButton { .. }
        | KvmFrame::MouseScroll { .. }
        | KvmFrame::KeyEvent { .. } => {
            let _ = event_tx.send(ServerEvent::InputFrame(frame));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{encode, KvmFrame, Platform, PROTOCOL_VERSION};
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    /// 辅助函数：执行客户端握手（模拟主控端）
    async fn client_handshake(stream: &mut TcpStream) -> Result<(), String> {
        // 读取服务端握手帧
        let mut buf = vec![0u8; 64];
        let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
            .await
            .map_err(|_| "握手超时")?
            .map_err(|e| e.to_string())?;

        match protocol::decode(&buf[..n]) {
            Ok((KvmFrame::Handshake { version, .. }, _)) => {
                if version != PROTOCOL_VERSION {
                    return Err("版本不兼容".to_string());
                }
            }
            _ => return Err("握手帧类型错误".to_string()),
        }

        // 发送握手响应
        let ack = encode(&KvmFrame::HandshakeAck {
            version: PROTOCOL_VERSION,
            platform: Platform::MacOS,
        });
        stream.write_all(&ack).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_start_and_stop() {
        let (mut server, _rx) = KvmServer::new("127.0.0.1:0");
        server.stop().await;
        assert_eq!(server.state().await, ServerState::Stopped);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_listen_state() {
        let (mut server, _rx) = KvmServer::new("127.0.0.1:0");

        server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(server.state().await, ServerState::Listening);

        server.stop().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(server.state().await, ServerState::Stopped);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_accepts_connection_and_handshake() {
        let (mut server, mut event_rx) = KvmServer::new("127.0.0.1:0");
        let actual_addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 模拟主控端连接
        let mut client = TcpStream::connect(actual_addr).await.unwrap();
        client_handshake(&mut client).await.unwrap();

        // 等待 ClientConnected 事件
        let event = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, ServerEvent::ClientConnected { .. }));

        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_receives_mouse_move_event() {
        let (mut server, mut event_rx) = KvmServer::new("127.0.0.1:0");
        let actual_addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(actual_addr).await.unwrap();
        client.set_nodelay(true).unwrap();
        client_handshake(&mut client).await.unwrap();

        // 消费 ClientConnected 事件
        let _ = timeout(Duration::from_secs(2), event_rx.recv()).await;

        // 发送鼠标移动事件
        let frame = KvmFrame::MouseMove { x: 1000, y: 2000 };
        client.write_all(&encode(&frame)).await.unwrap();

        // 等待 InputFrame 事件
        let event = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, ServerEvent::InputFrame(KvmFrame::MouseMove { x: 1000, y: 2000 })));

        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_receives_key_event() {
        let (mut server, mut event_rx) = KvmServer::new("127.0.0.1:0");
        let actual_addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(actual_addr).await.unwrap();
        client.set_nodelay(true).unwrap();
        client_handshake(&mut client).await.unwrap();

        // 消费 ClientConnected 事件
        let _ = timeout(Duration::from_secs(2), event_rx.recv()).await;

        // 发送键盘事件
        let frame = KvmFrame::KeyEvent { keycode: 0x41, pressed: true };
        client.write_all(&encode(&frame)).await.unwrap();

        let event = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            event,
            ServerEvent::InputFrame(KvmFrame::KeyEvent { keycode: 0x41, pressed: true })
        ));

        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_disconnect_event() {
        let (mut server, mut event_rx) = KvmServer::new("127.0.0.1:0");
        let actual_addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(actual_addr).await.unwrap();
        client_handshake(&mut client).await.unwrap();

        // 消费 ClientConnected 事件
        let _ = timeout(Duration::from_secs(2), event_rx.recv()).await;

        // 主动断开连接
        drop(client);

        // 等待 ClientDisconnected 事件
        let event = timeout(Duration::from_secs(3), event_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, ServerEvent::ClientDisconnected));

        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_server_switch_control_event() {
        let (mut server, mut event_rx) = KvmServer::new("127.0.0.1:0");
        let actual_addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(actual_addr).await.unwrap();
        client.set_nodelay(true).unwrap();
        client_handshake(&mut client).await.unwrap();

        // 消费 ClientConnected 事件
        let _ = timeout(Duration::from_secs(2), event_rx.recv()).await;

        // 发送切换控制权帧
        let frame = KvmFrame::SwitchControl {
            edge: crate::protocol::ScreenEdge::Right,
            pos_ratio: 5000,
        };
        client.write_all(&encode(&frame)).await.unwrap();

        let event = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            event,
            ServerEvent::SwitchControl {
                edge: crate::protocol::ScreenEdge::Right,
                pos_ratio: 5000
            }
        ));

        server.stop().await;
    }
}