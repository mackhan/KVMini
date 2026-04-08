/// kvm-agent 角色管理模块
///
/// 管理 Mac 端的当前角色（主控端 / 被控端），切换角色时：
/// 1. 停止旧服务（TCP 服务端或 TCP 客户端）
/// 2. 启动新服务
/// 3. 通知托盘更新显示

use std::sync::{Arc, Mutex};
use crate::server::{AppRole, ConnectionState, SharedState, StopSignal};
use crate::protocol::KvmFrame;

/// 角色切换事件
#[derive(Debug, Clone)]
pub enum RoleEvent {
    /// 角色已切换
    RoleChanged { new_role: AppRole },
    /// 服务已启动
    ServiceStarted { role: AppRole },
    /// 服务启动失败
    ServiceError(String),
    /// 连接状态变化
    ConnectionStatus(String),
}

/// 角色管理器
pub struct RoleManager {
    /// 共享连接状态
    state: SharedState,
    /// 当前服务停止信号
    stop_signal: StopSignal,
    /// 事件回调（通知托盘等）
    event_callback: Arc<dyn Fn(RoleEvent) + Send + Sync>,
    /// 主控端事件发送端（用于发送键鼠帧）
    frame_tx: Arc<Mutex<Option<std::sync::mpsc::SyncSender<KvmFrame>>>>,
}

impl RoleManager {
    /// 创建角色管理器，Mac 端默认角色为被控端
    pub fn new(
        event_callback: impl Fn(RoleEvent) + Send + Sync + 'static,
    ) -> Self {
        let state: SharedState = Arc::new(Mutex::new(ConnectionState::default()));
        let stop_signal: StopSignal = Arc::new(Mutex::new(false));
        Self {
            state,
            stop_signal,
            event_callback: Arc::new(event_callback),
            frame_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// 获取共享连接状态
    pub fn state(&self) -> SharedState {
        Arc::clone(&self.state)
    }

    /// 获取当前角色
    pub fn role(&self) -> AppRole {
        self.state.lock().unwrap().role.clone()
    }

    /// 切换为被控端模式
    /// 停止当前服务，启动 TCP 监听（端口 8889）
    pub fn switch_to_controlled(&self) {
        // 优雅停止当前服务
        self.stop_current_service();

        // 更新角色
        {
            let mut s = self.state.lock().unwrap();
            s.role = AppRole::Controlled;
            s.connected = false;
            s.peer_ip = None;
            s.peer_hostname = None;
        }

        let cb = Arc::clone(&self.event_callback);
        cb(RoleEvent::RoleChanged { new_role: AppRole::Controlled });

        // 启动被控端 TCP 监听
        let state = Arc::clone(&self.state);
        let stop = Arc::new(Mutex::new(false));
        *self.stop_signal.lock().unwrap() = false;
        let stop_clone = Arc::clone(&stop);
        let cb2 = Arc::clone(&self.event_callback);

        std::thread::spawn(move || {
            cb2(RoleEvent::ServiceStarted { role: AppRole::Controlled });
            loop {
                let state2 = Arc::clone(&state);
                let stop2 = Arc::clone(&stop_clone);
                let result = crate::server::run_tcp_server(
                    state2,
                    stop2,
                    |frame| {
                        crate::input_executor::execute_frame(&frame);
                    },
                );
                match result {
                    Ok(_) => {
                        // 收到停止信号，退出
                        break;
                    }
                    Err(e) => {
                        eprintln!("[RoleManager] 被控端服务错误: {}，5 秒后重试...", e);
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        // 检查是否已停止
                        if *stop_clone.lock().unwrap() {
                            break;
                        }
                    }
                }
            }
        });

        // 保存新的停止信号
        *self.stop_signal.lock().unwrap() = false;
    }

    /// 切换为主控端模式
    /// 停止当前服务，主动连接对端 IP:8889
    pub fn switch_to_controller(&self, peer_ip: String) {
        // 优雅停止当前服务
        self.stop_current_service();

        // 更新角色
        {
            let mut s = self.state.lock().unwrap();
            s.role = AppRole::Controller;
            s.connected = false;
            s.peer_ip = None;
            s.peer_hostname = None;
        }

        let cb = Arc::clone(&self.event_callback);
        cb(RoleEvent::RoleChanged { new_role: AppRole::Controller });

        // 创建帧发送通道（用 Arc<Mutex<>> 包装 Receiver 使其可以在线程间共享）
        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<KvmFrame>(256);
        let frame_rx = Arc::new(Mutex::new(frame_rx));
        *self.frame_tx.lock().unwrap() = Some(frame_tx);

        let state = Arc::clone(&self.state);
        let stop = Arc::new(Mutex::new(false));
        let stop_clone = Arc::clone(&stop);
        let cb2 = Arc::clone(&self.event_callback);

        std::thread::spawn(move || {
            cb2(RoleEvent::ServiceStarted { role: AppRole::Controller });

            // 重连逻辑：最多 5 次，间隔 3 秒
            let mut retry_count = 0;
            const MAX_RETRIES: u32 = 5;

            loop {
                if *stop_clone.lock().unwrap() {
                    break;
                }

                let result = crate::server::run_tcp_client(
                    peer_ip.clone(),
                    Arc::clone(&state),
                    Arc::clone(&stop_clone),
                    {
                        let rx = Arc::clone(&frame_rx);
                        move || rx.lock().unwrap().try_recv().ok()
                    },
                );

                match result {
                    Ok(_) => {
                        // 正常断开（收到停止信号）
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        if retry_count > MAX_RETRIES {
                            let msg = format!("连接失败，已重试 {} 次，停止重连", MAX_RETRIES);
                            eprintln!("[RoleManager] {}", msg);
                            cb2(RoleEvent::ServiceError(msg));
                            break;
                        }
                        let msg = format!("连接失败: {}，3 秒后重试（第 {}/{} 次）", e, retry_count, MAX_RETRIES);
                        eprintln!("[RoleManager] {}", msg);
                        cb2(RoleEvent::ConnectionStatus(msg));
                        std::thread::sleep(std::time::Duration::from_secs(3));
                    }
                }
            }
        });
    }

    /// 发送键鼠帧（主控端模式）
    pub fn send_frame(&self, frame: KvmFrame) -> Result<(), String> {
        if let Some(tx) = self.frame_tx.lock().unwrap().as_ref() {
            tx.try_send(frame).map_err(|e| format!("发送帧失败: {}", e))
        } else {
            Err("未处于主控端模式或未连接".to_string())
        }
    }

    /// 停止当前服务（优雅关闭）
    fn stop_current_service(&self) {
        *self.stop_signal.lock().unwrap() = true;
        // 清空帧发送端
        *self.frame_tx.lock().unwrap() = None;
        // 短暂等待服务线程响应停止信号
        std::thread::sleep(std::time::Duration::from_millis(200));
        // 重置停止信号，供下次使用
        *self.stop_signal.lock().unwrap() = false;
    }
}
