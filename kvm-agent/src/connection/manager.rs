/// 连接生命周期管理子模块
///
/// 管理 Mac 端的连接状态、角色切换、自动重连逻辑。
/// 区分 Mac 端（被控端默认）和 Windows 端（主控端默认）的行为差异。

use std::sync::{Arc, Mutex};
use super::transport::{SharedState, StopSignal, new_shared_state, new_stop_signal};
use super::error::{ConnectError, ErrorGuide};
use super::{DEFAULT_PEER_IP, DEFAULT_PORT};
use crate::protocol::KvmFrame;

/// 连接角色
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionRole {
    /// 被控端：监听 TCP 连接，接收并执行键鼠事件（Mac 端默认）
    Controlled,
    /// 主控端：主动连接对端，采集并发送键鼠事件
    Controller,
}

/// 连接事件（通知上层）
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// 角色已切换
    RoleChanged { role: ConnectionRole },
    /// 连接已建立
    Connected { peer_ip: String },
    /// 连接已断开
    Disconnected,
    /// 连接错误（含用户引导）
    Error { error: ConnectError, guide: Vec<String> },
    /// 重连中
    Reconnecting { attempt: u32, max_attempts: u32, peer_ip: String },
    /// 服务已就绪（被控端监听中）
    Listening { port: u16 },
}

/// 连接管理器
///
/// Mac 端默认为被控端，Windows 端默认为主控端。
/// 支持运行时角色切换和自动重连。
pub struct ConnectionManager {
    /// 当前角色
    role: Arc<Mutex<ConnectionRole>>,
    /// 共享连接状态
    state: SharedState,
    /// 停止信号
    stop_signal: StopSignal,
    /// 事件回调
    event_cb: Arc<dyn Fn(ConnectionEvent) + Send + Sync>,
    /// 主控端帧发送通道
    frame_tx: Arc<Mutex<Option<std::sync::mpsc::SyncSender<KvmFrame>>>>,
    /// 当前连接的对端 IP
    peer_ip: Arc<Mutex<String>>,
}

impl ConnectionManager {
    /// 创建连接管理器（Mac 端默认被控端）
    pub fn new(event_cb: impl Fn(ConnectionEvent) + Send + Sync + 'static) -> Self {
        Self {
            role: Arc::new(Mutex::new(ConnectionRole::Controlled)),
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

    /// 获取当前角色
    pub fn role(&self) -> ConnectionRole {
        self.role.lock().unwrap().clone()
    }

    /// 获取当前对端 IP
    pub fn peer_ip(&self) -> String {
        self.peer_ip.lock().unwrap().clone()
    }

    /// 设置对端 IP
    pub fn set_peer_ip(&self, ip: String) {
        *self.peer_ip.lock().unwrap() = ip;
    }

    /// 切换为被控端模式（Mac 端默认）
    pub fn switch_to_controlled(&self) {
        self.stop_current();
        *self.role.lock().unwrap() = ConnectionRole::Controlled;
        self.emit(ConnectionEvent::RoleChanged { role: ConnectionRole::Controlled });
        self.start_controlled();
    }

    /// 切换为主控端模式，连接指定 IP
    pub fn switch_to_controller(&self, peer_ip: Option<String>) {
        self.stop_current();
        let ip = peer_ip.unwrap_or_else(|| self.peer_ip());
        *self.peer_ip.lock().unwrap() = ip.clone();
        *self.role.lock().unwrap() = ConnectionRole::Controller;
        self.emit(ConnectionEvent::RoleChanged { role: ConnectionRole::Controller });
        self.start_controller(ip);
    }

    /// 发送键鼠帧（主控端模式）
    pub fn send_frame(&self, frame: KvmFrame) -> Result<(), ConnectError> {
        if let Some(tx) = self.frame_tx.lock().unwrap().as_ref() {
            tx.try_send(frame).map_err(|e| ConnectError::Unknown {
                message: format!("发送帧失败: {}", e),
            })
        } else {
            Err(ConnectError::Unknown { message: "未处于主控端模式或未连接".to_string() })
        }
    }

    /// 停止当前服务
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

    fn start_controlled(&self) {
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop_signal);
        let cb = Arc::clone(&self.event_cb);

        std::thread::spawn(move || {
            cb(ConnectionEvent::Listening { port: DEFAULT_PORT });
            loop {
                let result = super::transport::run_server(
                    Arc::clone(&state),
                    Arc::clone(&stop),
                    |frame| crate::input_executor::execute_frame(&frame),
                );
                match result {
                    Ok(_) => break,
                    Err(e) => {
                        let guide = ErrorGuide::guide(&e);
                        cb(ConnectionEvent::Error { error: e, guide });
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        if *stop.lock().unwrap() { break; }
                    }
                }
            }
        });
    }

    fn start_controller(&self, peer_ip: String) {
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop_signal);
        let cb = Arc::clone(&self.event_cb);

        // 帧通道（Arc<Mutex<Receiver>> 使闭包可 Send）
        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<KvmFrame>(256);
        let frame_rx = Arc::new(Mutex::new(frame_rx));
        *self.frame_tx.lock().unwrap() = Some(frame_tx);

        std::thread::spawn(move || {
            const MAX_RETRIES: u32 = 5;
            let mut attempt = 0u32;

            loop {
                if *stop.lock().unwrap() { break; }

                attempt += 1;
                if attempt > 1 {
                    cb(ConnectionEvent::Reconnecting {
                        attempt,
                        max_attempts: MAX_RETRIES,
                        peer_ip: peer_ip.clone(),
                    });
                }

                let rx = Arc::clone(&frame_rx);
                let result = super::transport::run_client(
                    &peer_ip,
                    DEFAULT_PORT,
                    Arc::clone(&state),
                    Arc::clone(&stop),
                    move || rx.lock().unwrap().try_recv().ok(),
                );

                match result {
                    Ok(_) => {
                        cb(ConnectionEvent::Disconnected);
                        break;
                    }
                    Err(e) => {
                        let guide = ErrorGuide::guide(&e);
                        cb(ConnectionEvent::Error { error: e, guide });
                        if attempt >= MAX_RETRIES {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_secs(3));
                    }
                }
            }
        });
    }
}
