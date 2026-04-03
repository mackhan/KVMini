/// 主控端 TCP 连接管理与事件发送模块
///
/// 负责：
/// 1. 建立到被控端的 TCP 连接（含握手）
/// 2. 将捕获到的键鼠事件编码后发送
/// 3. 断线自动重连（指数退避，最大 30 秒）
/// 4. 连接状态通知（通过 broadcast channel）

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio::time::{sleep, timeout, Duration};

use crate::protocol::{self, KvmFrame, Platform, PROTOCOL_VERSION};

/// 客户端连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ClientState {
    /// 未连接
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接
    Connected { peer_addr: String },
    /// 重连中（第 n 次，等待 delay 秒）
    Reconnecting { attempt: u32, delay_secs: u64 },
    /// 已停止（用户主动断开）
    Stopped,
}

/// 客户端事件
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// 连接成功
    Connected { peer_addr: String },
    /// 连接断开
    Disconnected,
    /// 重连中
    Reconnecting { attempt: u32, delay_secs: u64 },
    /// 发送错误
    SendError(String),
    /// 连接错误
    ConnectError(String),
}

/// 重连配置
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// 最大重连次数（0 = 无限重连）
    pub max_attempts: u32,
    /// 初始重连延迟（秒）
    pub initial_delay_secs: u64,
    /// 最大重连延迟（秒）
    pub max_delay_secs: u64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 0,       // 无限重连
            initial_delay_secs: 1,
            max_delay_secs: 30,
        }
    }
}

/// 主控端 TCP 客户端
pub struct KvmClient {
    /// 目标地址
    target_addr: String,
    /// 当前状态
    state: Arc<Mutex<ClientState>>,
    /// 事件广播发送端
    event_tx: broadcast::Sender<ClientEvent>,
    /// 停止信号
    stop_tx: Option<oneshot::Sender<()>>,
    /// 重连配置
    reconnect_config: ReconnectConfig,
    /// 是否启用重连
    auto_reconnect: Arc<AtomicBool>,
}

impl KvmClient {
    /// 创建新的主控端客户端
    pub fn new(target_addr: &str) -> (Self, broadcast::Receiver<ClientEvent>) {
        let (event_tx, event_rx) = broadcast::channel(256);
        let client = Self {
            target_addr: target_addr.to_string(),
            state: Arc::new(Mutex::new(ClientState::Disconnected)),
            event_tx,
            stop_tx: None,
            reconnect_config: ReconnectConfig::default(),
            auto_reconnect: Arc::new(AtomicBool::new(true)),
        };
        (client, event_rx)
    }

    /// 设置重连配置
    pub fn set_reconnect_config(&mut self, config: ReconnectConfig) {
        self.reconnect_config = config;
    }

    /// 启动客户端，返回事件发送端（用于发送键鼠事件）
    /// 调用者将捕获到的 KvmFrame 发送到返回的 sender
    pub async fn start(&mut self) -> Result<mpsc::Sender<KvmFrame>, String> {
        // 如果已在运行，先停止
        self.stop().await;

        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        self.stop_tx = Some(stop_tx);

        let (frame_tx, frame_rx) = mpsc::channel::<KvmFrame>(1024);

        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();
        let target_addr = self.target_addr.clone();
        let reconnect_config = self.reconnect_config.clone();
        let auto_reconnect = Arc::clone(&self.auto_reconnect);

        self.auto_reconnect.store(true, Ordering::SeqCst);

        tokio::spawn(async move {
            client_loop(
                target_addr,
                state,
                event_tx,
                frame_rx,
                stop_rx,
                reconnect_config,
                auto_reconnect,
            )
            .await;
        });

        Ok(frame_tx)
    }

    /// 停止客户端
    pub async fn stop(&mut self) {
        self.auto_reconnect.store(false, Ordering::SeqCst);
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        *self.state.lock().await = ClientState::Stopped;
    }

    /// 获取当前状态
    pub async fn state(&self) -> ClientState {
        self.state.lock().await.clone()
    }

    /// 订阅客户端事件
    pub fn subscribe(&self) -> broadcast::Receiver<ClientEvent> {
        self.event_tx.subscribe()
    }
}

/// 客户端主循环（含断线重连）
async fn client_loop(
    target_addr: String,
    state: Arc<Mutex<ClientState>>,
    event_tx: broadcast::Sender<ClientEvent>,
    mut frame_rx: mpsc::Receiver<KvmFrame>,
    mut stop_rx: oneshot::Receiver<()>,
    config: ReconnectConfig,
    auto_reconnect: Arc<AtomicBool>,
) {
    let mut attempt = 0u32;
    let mut delay_secs = config.initial_delay_secs;

    loop {
        // 检查停止信号
        if stop_rx.try_recv().is_ok() || !auto_reconnect.load(Ordering::SeqCst) {
            break;
        }

        // 更新状态为连接中
        *state.lock().await = ClientState::Connecting;

        // 尝试连接
        match connect_and_run(
            &target_addr,
            &state,
            &event_tx,
            &mut frame_rx,
            &mut stop_rx,
        )
        .await
        {
            ConnectionResult::Stopped => {
                // 用户主动停止
                break;
            }
            ConnectionResult::Disconnected => {
                // 连接断开，准备重连
                let _ = event_tx.send(ClientEvent::Disconnected);
            }
            ConnectionResult::ConnectFailed(e) => {
                let _ = event_tx.send(ClientEvent::ConnectError(e));
            }
        }

        // 检查是否需要重连
        if !auto_reconnect.load(Ordering::SeqCst) {
            break;
        }

        attempt += 1;
        if config.max_attempts > 0 && attempt >= config.max_attempts {
            break;
        }

        // 通知重连状态
        let _ = event_tx.send(ClientEvent::Reconnecting {
            attempt,
            delay_secs,
        });
        *state.lock().await = ClientState::Reconnecting { attempt, delay_secs };

        // 等待重连延迟（可被停止信号中断）
        tokio::select! {
            _ = sleep(Duration::from_secs(delay_secs)) => {}
            _ = &mut stop_rx => { break; }
        }

        // 指数退避
        delay_secs = (delay_secs * 2).min(config.max_delay_secs);
    }

    *state.lock().await = ClientState::Stopped;
}

/// 连接结果
enum ConnectionResult {
    Stopped,
    Disconnected,
    ConnectFailed(String),
}

/// 建立连接并运行发送循环
async fn connect_and_run(
    target_addr: &str,
    state: &Arc<Mutex<ClientState>>,
    event_tx: &broadcast::Sender<ClientEvent>,
    frame_rx: &mut mpsc::Receiver<KvmFrame>,
    stop_rx: &mut oneshot::Receiver<()>,
) -> ConnectionResult {
    // 建立 TCP 连接（5 秒超时）
    let mut stream = match timeout(
        Duration::from_secs(5),
        TcpStream::connect(target_addr),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return ConnectionResult::ConnectFailed(format!("连接失败: {}", e)),
        Err(_) => return ConnectionResult::ConnectFailed("连接超时".to_string()),
    };

    // 设置 TCP_NODELAY
    if let Err(e) = stream.set_nodelay(true) {
        return ConnectionResult::ConnectFailed(format!("设置 TCP_NODELAY 失败: {}", e));
    }

    // 执行握手
    if let Err(e) = perform_client_handshake(&mut stream).await {
        return ConnectionResult::ConnectFailed(e);
    }

    let peer_addr = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| target_addr.to_string());

    *state.lock().await = ClientState::Connected { peer_addr: peer_addr.clone() };
    let _ = event_tx.send(ClientEvent::Connected { peer_addr });

    // 将 stream 拆分为读写两半，同时监听：
    // 1. 读取端：检测连接关闭（服务端断开时会收到 EOF）
    // 2. 写入端：发送键鼠事件帧
    let (mut read_half, mut write_half) = stream.into_split();
    let mut read_buf = [0u8; 64];

    loop {
        tokio::select! {
            biased;
            _ = &mut *stop_rx => {
                return ConnectionResult::Stopped;
            }
            // 监听读取端（检测连接关闭）
            read_result = read_half.read(&mut read_buf) => {
                match read_result {
                    Ok(0) => {
                        // 服务端关闭连接（EOF）
                        return ConnectionResult::Disconnected;
                    }
                    Ok(_) => {
                        // 收到服务端数据（心跳响应等），忽略
                    }
                    Err(_) => {
                        return ConnectionResult::Disconnected;
                    }
                }
            }
            // 监听发送队列
            frame = frame_rx.recv() => {
                match frame {
                    Some(f) => {
                        let bytes = protocol::encode(&f);
                        if let Err(e) = write_half.write_all(&bytes).await {
                            let _ = event_tx.send(ClientEvent::SendError(format!("发送失败: {}", e)));
                            return ConnectionResult::Disconnected;
                        }
                    }
                    None => {
                        // channel 关闭，停止
                        return ConnectionResult::Stopped;
                    }
                }
            }
        }
    }
}

/// 客户端握手（与服务端握手协议对称）
async fn perform_client_handshake(stream: &mut TcpStream) -> Result<(), String> {
    // 等待服务端握手帧（5 秒超时）
    let mut buf = vec![0u8; 256];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .map_err(|_| "等待握手超时".to_string())?
        .map_err(|e| format!("读取握手失败: {}", e))?;

    if n == 0 {
        return Err("握手时连接关闭".to_string());
    }

    match protocol::decode(&buf[..n]) {
        Ok((KvmFrame::Handshake { version, .. }, _)) => {
            if version != PROTOCOL_VERSION {
                return Err(format!(
                    "协议版本不兼容：本端 {}，对端 {}",
                    PROTOCOL_VERSION, version
                ));
            }
        }
        Ok((frame, _)) => return Err(format!("握手帧类型错误: {:?}", frame)),
        Err(e) => return Err(format!("握手解码失败: {}", e)),
    }

    // 发送握手响应
    let ack = protocol::encode(&KvmFrame::HandshakeAck {
        version: PROTOCOL_VERSION,
        platform: Platform::current(),
    });
    stream
        .write_all(&ack)
        .await
        .map_err(|e| format!("发送握手响应失败: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::KvmServer;

    /// 辅助：启动一个测试服务端
    async fn start_test_server() -> (KvmServer, broadcast::Receiver<crate::server::ServerEvent>, std::net::SocketAddr) {
        let (mut server, rx) = KvmServer::new("127.0.0.1:0");
        let addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        (server, rx, addr)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_connects_to_server() {
        let (mut server, _srv_rx, addr) = start_test_server().await;

        let (mut client, mut client_rx) = KvmClient::new(&addr.to_string());
        let _frame_tx = client.start().await.unwrap();

        // 等待连接成功事件
        let event = timeout(Duration::from_secs(3), client_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, ClientEvent::Connected { .. }));
        assert!(matches!(client.state().await, ClientState::Connected { .. }));

        client.stop().await;
        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_sends_frame_to_server() {
        let (mut server, mut srv_rx, addr) = start_test_server().await;

        let (mut client, mut client_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        // 等待连接成功
        let _ = timeout(Duration::from_secs(3), client_rx.recv()).await;
        // 消费服务端 ClientConnected 事件
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        // 发送鼠标移动事件
        frame_tx.send(KvmFrame::MouseMove { x: 1000, y: 2000 }).await.unwrap();

        // 等待服务端收到事件
        let srv_event = timeout(Duration::from_secs(2), srv_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            srv_event,
            crate::server::ServerEvent::InputFrame(KvmFrame::MouseMove { x: 1000, y: 2000 })
        ));

        client.stop().await;
        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_sends_key_event() {
        let (mut server, mut srv_rx, addr) = start_test_server().await;

        let (mut client, mut client_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        let _ = timeout(Duration::from_secs(3), client_rx.recv()).await;
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        frame_tx.send(KvmFrame::KeyEvent { keycode: 0x41, pressed: true }).await.unwrap();

        let srv_event = timeout(Duration::from_secs(2), srv_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            srv_event,
            crate::server::ServerEvent::InputFrame(KvmFrame::KeyEvent { keycode: 0x41, pressed: true })
        ));

        client.stop().await;
        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_reconnects_after_server_restart() {
        // 测试重连逻辑：连接到不存在的地址，验证重连事件
        let (mut client, mut client_rx) = KvmClient::new("127.0.0.1:19997");
        let mut config = ReconnectConfig::default();
        config.max_attempts = 2;       // 最多重连 2 次
        config.initial_delay_secs = 0; // 立即重连
        config.max_delay_secs = 0;
        client.set_reconnect_config(config);

        let _frame_tx = client.start().await.unwrap();

        // 收集事件，验证有 ConnectError 和 Reconnecting 事件
        let mut got_connect_error = false;
        let mut got_reconnecting = false;

        for _ in 0..10 {
            match timeout(Duration::from_secs(3), client_rx.recv()).await {
                Ok(Ok(event)) => {
                    match event {
                        ClientEvent::ConnectError(_) => got_connect_error = true,
                        ClientEvent::Reconnecting { .. } => got_reconnecting = true,
                        _ => {}
                    }
                    if got_connect_error && got_reconnecting {
                        break;
                    }
                }
                _ => break,
            }
        }

        assert!(got_connect_error, "应该收到 ConnectError 事件");
        assert!(got_reconnecting, "应该收到 Reconnecting 事件");

        client.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_connect_failure() {
        // 连接到不存在的地址
        let (mut client, mut client_rx) = KvmClient::new("127.0.0.1:19999");
        let mut config = ReconnectConfig::default();
        config.max_attempts = 1; // 只重试一次
        config.initial_delay_secs = 0;
        client.set_reconnect_config(config);

        let _frame_tx = client.start().await.unwrap();

        // 等待连接失败事件
        let event = timeout(Duration::from_secs(10), client_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, ClientEvent::ConnectError(_)));

        client.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_stop() {
        let (mut client, _rx) = KvmClient::new("127.0.0.1:19998");
        let _frame_tx = client.start().await.unwrap();

        client.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(client.state().await, ClientState::Stopped);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_sends_multiple_frames() {
        let (mut server, mut srv_rx, addr) = start_test_server().await;

        let (mut client, mut client_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        let _ = timeout(Duration::from_secs(3), client_rx.recv()).await;
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        // 发送多个事件
        let frames = vec![
            KvmFrame::MouseMove { x: 1000, y: 2000 },
            KvmFrame::KeyEvent { keycode: 0x41, pressed: true },
            KvmFrame::MouseButton { button: 0, pressed: true },
        ];

        for frame in &frames {
            frame_tx.send(frame.clone()).await.unwrap();
        }

        // 验证服务端收到所有事件
        for expected in &frames {
            let srv_event = timeout(Duration::from_secs(2), srv_rx.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(srv_event, crate::server::ServerEvent::InputFrame(_)));
            if let crate::server::ServerEvent::InputFrame(f) = srv_event {
                assert_eq!(&f, expected);
            }
        }

        client.stop().await;
        server.stop().await;
    }
}
