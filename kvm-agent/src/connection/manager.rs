/// 连接生命周期管理子模块
///
/// 管理连接状态、自动重连逻辑。
/// 两端对等，任意一端都可以发起连接或接受连接，不区分主控/被控角色。

use std::sync::{Arc, Mutex};
use super::transport::{SharedState, StopSignal, new_shared_state, new_stop_signal};
use super::error::{ConnectError, ErrorGuide};
use super::{DEFAULT_PEER_IP, DEFAULT_PORT};
use crate::protocol::KvmFrame;

/// 连接事件（通知上层）
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// 连接已建立
    Connected { peer_ip: String },
    /// 连接已断开
    Disconnected,
    /// 连接错误（含用户引导）
    Error { error: ConnectError, guide: Vec<String> },
    /// 重连中（第 N 次）
    Reconnecting { attempt: u32, peer_ip: String },
    /// 重连成功
    ReconnectSuccess { peer_ip: String },
    /// 服务已就绪（监听中）
    Listening { port: u16 },
}

/// 连接管理器
///
/// 两端对等：任意一端都可以发起连接（connect），任意一端都可以接受连接（listen）。
/// 连接建立后双方逻辑完全对称，不区分主控/被控角色。
pub struct ConnectionManager {
    /// 共享连接状态
    state: SharedState,
    /// 停止信号（停止当前连接/重连循环）
    stop_signal: StopSignal,
    /// 事件回调
    event_cb: Arc<dyn Fn(ConnectionEvent) + Send + Sync>,
    /// 帧发送通道（连接建立后可用）
    frame_tx: Arc<Mutex<Option<std::sync::mpsc::SyncSender<KvmFrame>>>>,
    /// 当前连接的对端 IP
    peer_ip: Arc<Mutex<String>>,
}

impl ConnectionManager {
    /// 创建连接管理器
    pub fn new(event_cb: impl Fn(ConnectionEvent) + Send + Sync + 'static) -> Self {
        Self {
            state: new_shared_state(),
            stop_signal: new_stop_signal(),
            event_cb: Arc::new(event_cb),
            frame_tx: Arc::new(Mutex::new(None)),
            peer_ip: Arc::new(Mutex::new(DEFAULT_PEER_IP.to_string())),
        }
    }

    /// 获取共享连接状态
    pub fn state(&self) -> SharedState {
        Arc::clone(&self.state)
    }

    /// 获取当前对端 IP
    pub fn peer_ip(&self) -> String {
        self.peer_ip.lock().unwrap().clone()
    }

    /// 设置对端 IP
    pub fn set_peer_ip(&self, ip: String) {
        *self.peer_ip.lock().unwrap() = ip;
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.state.lock().unwrap().connected
    }

    /// 启动监听模式（接受方：等待对端连接）
    pub fn start_listening(&self) {
        self.stop_current();
        self.start_server();
    }

    /// 发起连接（发起方：主动连接对端 IP），支持无限自动重连
    pub fn connect(&self, peer_ip: Option<String>) {
        self.stop_current();
        let ip = peer_ip.unwrap_or_else(|| self.peer_ip());
        *self.peer_ip.lock().unwrap() = ip.clone();
        self.start_client(ip);
    }

    /// 手动断开连接，停止自动重连
    pub fn disconnect(&self) {
        self.stop_current();
        *self.frame_tx.lock().unwrap() = None;
        self.emit(ConnectionEvent::Disconnected);
    }

    /// 手动重新连接（使用上次的 IP）
    pub fn reconnect(&self) {
        let ip = self.peer_ip();
        self.connect(Some(ip));
    }

    /// 发送键鼠帧
    pub fn send_frame(&self, frame: KvmFrame) -> Result<(), ConnectError> {
        if let Some(tx) = self.frame_tx.lock().unwrap().as_ref() {
            tx.try_send(frame).map_err(|e| ConnectError::Unknown {
                message: format!("发送帧失败: {}", e),
            })
        } else {
            Err(ConnectError::Unknown { message: "未连接".to_string() })
        }
    }

    /// 停止当前服务（供外部调用）
    pub fn stop(&self) {
        self.stop_current();
    }

    // ── 内部方法 ──────────────────────────────────────────────────────────

    fn emit(&self, event: ConnectionEvent) {
        (self.event_cb)(event);
    }

    fn stop_current(&self) {
        *self.stop_signal.lock().unwrap() = true;
        *self.frame_tx.lock().unwrap() = None;
        std::thread::sleep(std::time::Duration::from_millis(200));
        *self.stop_signal.lock().unwrap() = false;
    }

    /// 启动服务端（接受方），断开后自动重新监听
    fn start_server(&self) {
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop_signal);
        let cb = Arc::clone(&self.event_cb);
        let frame_tx_slot = Arc::clone(&self.frame_tx);

        std::thread::spawn(move || {
            cb(ConnectionEvent::Listening { port: DEFAULT_PORT });
            loop {
                if *stop.lock().unwrap() { break; }

                // 每次连接建立一个新的帧通道
                let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<KvmFrame>(256);
                let frame_rx = Arc::new(Mutex::new(frame_rx));
                *frame_tx_slot.lock().unwrap() = Some(frame_tx);

                let rx = Arc::clone(&frame_rx);
                let result = super::transport::run_peer_connection(
                    super::transport::PeerMode::Server,
                    Arc::clone(&state),
                    Arc::clone(&stop),
                    move || rx.lock().unwrap().try_recv().ok(),
                    |frame| crate::input_executor::execute_frame(&frame),
                );

                *frame_tx_slot.lock().unwrap() = None;

                if *stop.lock().unwrap() { break; }

                match result {
                    Ok(_) => {
                        // 连接正常断开，通知上层，继续等待新连接
                        cb(ConnectionEvent::Disconnected);
                    }
                    Err(e) => {
                        let guide = ErrorGuide::guide(&e);
                        cb(ConnectionEvent::Error { error: e, guide });
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                }
            }
        });
    }

    /// 启动客户端（发起方），无限自动重连
    fn start_client(&self, peer_ip: String) {
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop_signal);
        let cb = Arc::clone(&self.event_cb);
        let frame_tx_slot = Arc::clone(&self.frame_tx);

        // 帧通道（整个重连生命周期共用，避免重连时丢失通道引用）
        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<KvmFrame>(256);
        let frame_rx = Arc::new(Mutex::new(frame_rx));
        *self.frame_tx.lock().unwrap() = Some(frame_tx);

        std::thread::spawn(move || {
            let mut attempt = 0u32;

            loop {
                if *stop.lock().unwrap() { break; }

                attempt += 1;

                if attempt > 1 {
                    // 重连等待 3 秒
                    for _ in 0..30 {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        if *stop.lock().unwrap() { break; }
                    }
                    if *stop.lock().unwrap() { break; }

                    cb(ConnectionEvent::Reconnecting {
                        attempt,
                        peer_ip: peer_ip.clone(),
                    });
                }

                let rx = Arc::clone(&frame_rx);
                let result = super::transport::run_peer_connection(
                    super::transport::PeerMode::Client {
                        peer_ip: peer_ip.clone(),
                        port: DEFAULT_PORT,
                    },
                    Arc::clone(&state),
                    Arc::clone(&stop),
                    move || rx.lock().unwrap().try_recv().ok(),
                    |frame| crate::input_executor::execute_frame(&frame),
                );

                if *stop.lock().unwrap() { break; }

                match result {
                    Ok(_) => {
                        // 连接曾经建立，现在正常断开 → 通知断开，继续重连
                        cb(ConnectionEvent::Disconnected);
                        // 重置重连计数（下次重连从第 1 次开始）
                        attempt = 0;
                    }
                    Err(ConnectError::HeartbeatTimeout { .. }) => {
                        // 心跳超时断开 → 通知断开，继续重连
                        let guide = ErrorGuide::guide(&ConnectError::HeartbeatTimeout { last_seen_secs: 15 });
                        cb(ConnectionEvent::Error {
                            error: ConnectError::HeartbeatTimeout { last_seen_secs: 15 },
                            guide,
                        });
                        cb(ConnectionEvent::Disconnected);
                        attempt = 0;
                    }
                    Err(e) => {
                        // 连接失败 → 通知错误，继续重连（无限重试）
                        let guide = ErrorGuide::guide(&e);
                        cb(ConnectionEvent::Error { error: e, guide });
                        // attempt 不重置，继续累加（用于 UI 显示重连次数）
                    }
                }
            }

            *frame_tx_slot.lock().unwrap() = None;
        });
    }
}
