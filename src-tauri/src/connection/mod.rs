//! 连接管理模块
//!
//! 使用 kvm-core 的 ConnectionManager 管理 TCP 连接。
//! 两端对等，任意一端都可以发起连接或接受连接。
//! 通过 Tauri 事件向前端推送连接状态变更。

use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

use kvm_core::network::{ConnectionManager, ConnectionState, NetworkEvent};

/// 默认 TCP 端口
pub const DEFAULT_PORT: u16 = 23456;

/// 连接状态（序列化给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub peer_platform: Option<String>,
    pub message: String,
    pub state: String,
}

/// 连接事件（推送给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEvent {
    pub event_type: String,
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub message: String,
    pub attempt: Option<u32>,
    pub guide: Option<Vec<String>>,
    /// TLS 配置是否不一致（一端开一端未开）
    #[serde(default)]
    pub tls_mismatch: bool,
    /// 本机 TLS 状态
    #[serde(default)]
    pub local_tls: bool,
    /// 对端 TLS 状态
    #[serde(default)]
    pub remote_tls: bool,
}

impl ConnectionEvent {
    pub fn from_state(state: &ConnectionState, peer_ip: Option<&str>) -> Self {
        match state {
            ConnectionState::Disconnected => Self {
                event_type: "disconnected".to_string(),
                connected: false,
                peer_ip: peer_ip.map(|s| s.to_string()),
                message: "未连接".to_string(),
                attempt: None,
                guide: None,
                tls_mismatch: false,
                local_tls: false,
                remote_tls: false,
            },
            ConnectionState::Connecting => Self {
                event_type: "connecting".to_string(),
                connected: false,
                peer_ip: peer_ip.map(|s| s.to_string()),
                message: format!("正在连接到 {}...", peer_ip.unwrap_or("")),
                attempt: None,
                guide: None,
                tls_mismatch: false,
                local_tls: false,
                remote_tls: false,
            },
            ConnectionState::Connected => Self {
                event_type: "connected".to_string(),
                connected: true,
                peer_ip: peer_ip.map(|s| s.to_string()),
                message: format!("已连接到 {}", peer_ip.unwrap_or("")),
                attempt: None,
                guide: None,
                tls_mismatch: false,
                local_tls: false,
                remote_tls: false,
            },
            ConnectionState::Reconnecting { attempt } => Self {
                event_type: "reconnecting".to_string(),
                connected: false,
                peer_ip: peer_ip.map(|s| s.to_string()),
                message: format!("正在重连（第 {} 次）→ {}", attempt, peer_ip.unwrap_or("")),
                attempt: Some(*attempt),
                guide: None,
                tls_mismatch: false,
                local_tls: false,
                remote_tls: false,
            },
        }
    }

    /// 构造 TLS 不一致事件
    pub fn tls_mismatch(peer_ip: Option<&str>, local_tls: bool, remote_tls: bool) -> Self {
        let msg = if local_tls && !remote_tls {
            "本机已开启 TLS 加密，但对端未开启，连接可能失败。请关闭本机 TLS 或通知对端开启。".to_string()
        } else {
            "对端已开启 TLS 加密，但本机未开启，连接可能失败。请开启本机 TLS 或通知对端关闭。".to_string()
        };
        Self {
            event_type: "tls_mismatch".to_string(),
            connected: true,
            peer_ip: peer_ip.map(|s| s.to_string()),
            message: msg,
            attempt: None,
            guide: Some(vec![
                "关闭本机 TLS 后重试".to_string(),
                "或通知对端调整 TLS 设置".to_string(),
            ]),
            tls_mismatch: true,
            local_tls,
            remote_tls,
        }
    }

    pub fn listening(port: u16) -> Self {
        Self {
            event_type: "listening".to_string(),
            connected: false,
            peer_ip: None,
            message: format!("监听端口 {}，等待对端连接...", port),
            attempt: None,
            guide: None,
            tls_mismatch: false,
            local_tls: false,
            remote_tls: false,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            event_type: "error".to_string(),
            connected: false,
            peer_ip: None,
            message: msg.to_string(),
            attempt: None,
            guide: Some(vec![
                "检查对端是否已启动 KVMini".to_string(),
                "检查防火墙是否放行端口".to_string(),
                "确认 IP 地址是否正确".to_string(),
            ]),
            tls_mismatch: false,
            local_tls: false,
            remote_tls: false,
        }
    }
}

/// KVM 连接管理器（Tauri 托管状态）
pub struct KvmConnectionManager {
    /// kvm-core 连接管理器
    inner: Arc<ConnectionManager>,
    /// 当前对端 IP
    peer_ip: Arc<Mutex<String>>,
    /// 停止信号（用于停止事件监听循环）
    stop_tx: Arc<tokio::sync::Notify>,
}

impl KvmConnectionManager {
    pub fn new() -> Self {
        let (mgr, _event_rx) = ConnectionManager::new(DEFAULT_PORT);
        Self {
            inner: Arc::new(mgr),
            peer_ip: Arc::new(Mutex::new(String::new())),
            stop_tx: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// 获取当前对端 IP
    pub async fn peer_ip(&self) -> String {
        self.peer_ip.lock().await.clone()
    }

    /// 设置对端 IP
    pub async fn set_peer_ip(&self, ip: String) {
        *self.peer_ip.lock().await = ip;
    }

    /// 获取连接状态
    pub async fn get_status(&self) -> ConnectionStatus {
        let state = self.inner.state().await;
        let peer_ip = self.peer_ip.lock().await.clone();
        let peer_ip_opt = if peer_ip.is_empty() { None } else { Some(peer_ip.as_str()) };
        let event = ConnectionEvent::from_state(&state, peer_ip_opt);
        ConnectionStatus {
            connected: event.connected,
            peer_ip: event.peer_ip,
            peer_platform: None,
            message: event.message,
            state: event.event_type,
        }
    }

    /// 启动事件监听循环，将 NetworkEvent 转发给前端
    pub fn start_event_loop(&self, app: tauri::AppHandle) {
        let mut rx = self.inner.subscribe();
        let peer_ip = Arc::clone(&self.peer_ip);
        let stop = Arc::clone(&self.stop_tx);

        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.notified() => break,
                    result = rx.recv() => {
                        match result {
                            Ok(event) => {
                                let ip = peer_ip.lock().await.clone();
                                let ip_opt = if ip.is_empty() { None } else { Some(ip.as_str()) };
                                if let Some(ev) = network_event_to_connection_event(&event, ip_opt) {
                                    let _ = app.emit("connection-status", ev);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }

    /// 停止所有网络活动
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.inner.stop();
        self.stop_tx.notify_waiters();
    }
}

impl Default for KvmConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── 纯函数：NetworkEvent → ConnectionEvent ────────────────────────────────

/// 将 NetworkEvent 转换为 ConnectionEvent（纯函数，便于测试）
pub fn network_event_to_connection_event(
    event: &NetworkEvent,
    ip_opt: Option<&str>,
) -> Option<ConnectionEvent> {
    match event {
        NetworkEvent::ConnectionStateChanged(state) => {
            Some(ConnectionEvent::from_state(state, ip_opt))
        }
        NetworkEvent::Error(msg) => {
            Some(ConnectionEvent::error(msg))
        }
        NetworkEvent::PeerHandshake(info) => {
            let platform_str = format!("{:?}", info.platform);
            Some(ConnectionEvent {
                event_type: "peer_info".to_string(),
                connected: true,
                peer_ip: ip_opt.map(|s| s.to_string()),
                message: format!("对端平台: {}", platform_str),
                attempt: None,
                guide: None,
                tls_mismatch: false,
                local_tls: false,
                remote_tls: false,
            })
        }
        NetworkEvent::LatencyUpdated(ms) => {
            Some(ConnectionEvent {
                event_type: "latency".to_string(),
                connected: true,
                peer_ip: ip_opt.map(|s| s.to_string()),
                message: format!("延迟: {}ms", ms),
                attempt: None,
                guide: None,
                tls_mismatch: false,
                local_tls: false,
                remote_tls: false,
            })
        }
        _ => None,
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：启动监听模式（等待对端连接）
#[tauri::command]
pub async fn start_listening(
    app_handle: tauri::AppHandle,
    mgr: tauri::State<'_, KvmConnectionManager>,
) -> Result<String, String> {
    let addr = mgr.inner.start_listening().await
        .map_err(|e| e.to_string())?;
    let _ = app_handle.emit("connection-status", ConnectionEvent::listening(addr.port()));
    Ok(format!("已启动监听，端口 {}", addr.port()))
}

/// Tauri 命令：主动连接对端
#[tauri::command]
pub async fn connect_to_peer(
    peer_ip: Option<String>,
    app_handle: tauri::AppHandle,
    mgr: tauri::State<'_, KvmConnectionManager>,
) -> Result<String, String> {
    let ip = match peer_ip {
        Some(ip) if !ip.is_empty() => ip,
        _ => mgr.peer_ip().await,
    };

    if ip.is_empty() {
        return Err("请输入对端 IP 地址".to_string());
    }

    mgr.set_peer_ip(ip.clone()).await;

    let local_info = crate::platform::local_handshake_info();
    let _ = app_handle.emit("connection-status", ConnectionEvent::from_state(
        &ConnectionState::Connecting, Some(&ip)
    ));

    // 读取本机 TLS 配置
    let local_tls = crate::config::load_config().enable_tls;

    let inner = Arc::clone(&mgr.inner);
    let ip_clone = ip.clone();
    let app_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        match inner.connect_to(&ip_clone, DEFAULT_PORT, local_info).await {
            Ok(peer_info) => {
                log::info!("[连接] 已连接到 {}", ip_clone);
                // 检测 TLS 状态：从对端握手信息中读取 tls_enabled
                let remote_tls = peer_info.tls_enabled;
                if local_tls != remote_tls {
                    log::warn!(
                        "[TLS] 不一致：本机 tls={}, 对端 tls={}",
                        local_tls, remote_tls
                    );
                    let _ = app_clone.emit(
                        "connection-status",
                        ConnectionEvent::tls_mismatch(Some(&ip_clone), local_tls, remote_tls),
                    );
                }
            }
            Err(e) => {
                log::error!("[连接] 连接失败: {}", e);
                let _ = app_clone.emit("connection-status", ConnectionEvent::error(&e.to_string()));
            }
        }
    });

    Ok(format!("正在连接到 {}...", ip))
}

/// Tauri 命令：断开连接
#[tauri::command]
pub async fn disconnect_peer(
    app_handle: tauri::AppHandle,
    mgr: tauri::State<'_, KvmConnectionManager>,
) -> Result<(), String> {
    mgr.inner.disconnect().await;
    let _ = app_handle.emit("connection-status", ConnectionEvent {
        event_type: "disconnected".to_string(),
        connected: false,
        peer_ip: None,
        message: "已手动断开连接".to_string(),
        attempt: None,
        guide: None,
        tls_mismatch: false,
        local_tls: false,
        remote_tls: false,
    });
    Ok(())
}

/// Tauri 命令：获取连接状态
#[tauri::command]
pub async fn get_connection_status(
    mgr: tauri::State<'_, KvmConnectionManager>,
) -> Result<ConnectionStatus, String> {
    Ok(mgr.get_status().await)
}
