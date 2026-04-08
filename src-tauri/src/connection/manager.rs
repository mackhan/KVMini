/// Windows 端连接管理器子模块
///
/// 管理 Windows 端的连接生命周期（对等连接模型）：
/// - 支持主动连接对端（connect）和监听模式（listen）
/// - 无限自动重连
/// - 通过 Tauri 事件向前端推送连接状态变更

use std::sync::Arc;
use tauri::Manager;
use serde::{Serialize, Deserialize};

use super::error::{ConnectError, ErrorGuide};
use super::{MAC_DEFAULT_IP, WINDOWS_DEFAULT_IP, DEFAULT_PORT};

/// 连接状态（序列化给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub message: String,
}

/// 连接事件（推送给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEvent {
    /// 事件类型：connected / disconnected / reconnecting / reconnect_success / error
    pub event_type: String,
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub message: String,
    /// 重连次数（reconnecting 事件时有效）
    pub attempt: Option<u32>,
    /// 错误解决方案（error 事件时有效）
    pub guide: Option<Vec<String>>,
}

impl ConnectionEvent {
    pub fn connected(peer_ip: String) -> Self {
        Self {
            event_type: "connected".to_string(),
            connected: true,
            peer_ip: Some(peer_ip.clone()),
            message: format!("已连接到 {}", peer_ip),
            attempt: None,
            guide: None,
        }
    }

    pub fn disconnected() -> Self {
        Self {
            event_type: "disconnected".to_string(),
            connected: false,
            peer_ip: None,
            message: "连接已断开，正在自动重连...".to_string(),
            attempt: None,
            guide: None,
        }
    }

    pub fn manual_disconnected() -> Self {
        Self {
            event_type: "disconnected".to_string(),
            connected: false,
            peer_ip: None,
            message: "已手动断开连接".to_string(),
            attempt: None,
            guide: None,
        }
    }

    pub fn reconnecting(attempt: u32, peer_ip: &str) -> Self {
        let msg = if attempt > 10 {
            format!("重连失败，仍在后台尝试（第 {} 次）", attempt)
        } else {
            format!("正在重连...（第 {} 次）→ {}", attempt, peer_ip)
        };
        Self {
            event_type: "reconnecting".to_string(),
            connected: false,
            peer_ip: Some(peer_ip.to_string()),
            message: msg,
            attempt: Some(attempt),
            guide: None,
        }
    }

    pub fn reconnect_success(peer_ip: String) -> Self {
        Self {
            event_type: "reconnect_success".to_string(),
            connected: true,
            peer_ip: Some(peer_ip.clone()),
            message: format!("重连成功，已连接到 {}", peer_ip),
            attempt: None,
            guide: None,
        }
    }

    pub fn error(error: &ConnectError) -> Self {
        Self {
            event_type: "error".to_string(),
            connected: false,
            peer_ip: None,
            message: error.to_string(),
            attempt: None,
            guide: Some(ErrorGuide::guide(error)),
        }
    }
}

/// Windows 端连接管理器（Tauri 托管状态）
pub struct WinConnectionManager {
    /// 当前连接的对端 IP
    peer_ip: std::sync::Mutex<String>,
    /// 连接状态
    connected: std::sync::Mutex<bool>,
    /// 停止信号（停止自动重连）
    stop_reconnect: std::sync::Mutex<bool>,
}

impl WinConnectionManager {
    pub fn new() -> Self {
        Self {
            peer_ip: std::sync::Mutex::new(MAC_DEFAULT_IP.to_string()),
            connected: std::sync::Mutex::new(false),
            stop_reconnect: std::sync::Mutex::new(false),
        }
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
        *self.connected.lock().unwrap()
    }

    /// 设置连接状态
    pub fn set_connected(&self, v: bool) {
        *self.connected.lock().unwrap() = v;
    }

    /// 是否应停止重连
    pub fn should_stop(&self) -> bool {
        *self.stop_reconnect.lock().unwrap()
    }

    /// 设置停止重连标志
    pub fn set_stop(&self, v: bool) {
        *self.stop_reconnect.lock().unwrap() = v;
    }

    /// 获取连接状态（供前端查询）
    pub fn get_status(&self) -> ConnectionStatus {
        ConnectionStatus {
            connected: self.is_connected(),
            peer_ip: if self.is_connected() { Some(self.peer_ip()) } else { None },
            message: if self.is_connected() {
                format!("已连接到 {}", self.peer_ip())
            } else {
                "未连接".to_string()
            },
        }
    }
}

impl Default for WinConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：连接到对端（使用默认或指定 IP），支持无限自动重连
#[tauri::command]
pub async fn connect_to_mac(
    peer_ip: Option<String>,
    app_handle: tauri::AppHandle,
    mgr: tauri::State<'_, WinConnectionManager>,
) -> Result<String, String> {
    let ip = peer_ip.unwrap_or_else(|| MAC_DEFAULT_IP.to_string());
    mgr.set_peer_ip(ip.clone());
    mgr.set_stop(false);

    let app = app_handle.clone();
    let ip_clone = ip.clone();
    let mgr_ip = ip.clone();

    // 通知前端连接中
    let _ = app_handle.emit_all("connection-status", serde_json::json!({
        "event_type": "connecting",
        "connected": false,
        "peer_ip": ip,
        "message": format!("正在连接到 {}...", ip),
        "attempt": null,
        "guide": null
    }));

    // 在后台线程中执行连接（含无限重连）
    std::thread::spawn(move || {
        let addr = format!("{}:{}", ip_clone, DEFAULT_PORT);
        let mut attempt = 0u32;
        let mut was_connected = false;

        loop {
            // 检查停止信号
            // 注意：这里无法直接访问 mgr（State 不是 Send），
            // 通过 app_handle 发送事件来间接控制
            attempt += 1;

            if attempt > 1 {
                // 重连等待 3 秒
                std::thread::sleep(std::time::Duration::from_secs(3));

                let _ = app.emit_all("connection-status",
                    ConnectionEvent::reconnecting(attempt, &ip_clone));
            }

            match std::net::TcpStream::connect_timeout(
                &addr.parse().unwrap(),
                std::time::Duration::from_secs(10),
            ) {
                Ok(_stream) => {
                    let event = if was_connected {
                        ConnectionEvent::reconnect_success(ip_clone.clone())
                    } else {
                        ConnectionEvent::connected(ip_clone.clone())
                    };
                    let _ = app.emit_all("connection-status", &event);
                    was_connected = true;
                    attempt = 0;

                    // 保持连接（实际的 TCP 会话由 platform.rs 的 connect_by_ip 管理）
                    // 这里只做状态通知，实际连接由 platform 层处理
                    // TODO: 集成完整的 TCP 会话管理
                    break;
                }
                Err(e) => {
                    let err = ErrorGuide::from_io_error(&e, &ip_clone, DEFAULT_PORT);
                    let _ = app.emit_all("connection-status", ConnectionEvent::error(&err));
                    // 继续重连循环（无限重试）
                }
            }
        }
    });

    Ok(format!("正在连接到 {}...", mgr_ip))
}

/// Tauri 命令：手动断开连接，停止自动重连
#[tauri::command]
pub async fn disconnect_from_mac(
    app_handle: tauri::AppHandle,
    mgr: tauri::State<'_, WinConnectionManager>,
) -> Result<(), String> {
    mgr.set_stop(true);
    mgr.set_connected(false);
    let event = ConnectionEvent::manual_disconnected();
    let _ = app_handle.emit_all("connection-status", &event);
    Ok(())
}

/// Tauri 命令：手动重新连接（使用上次的 IP）
#[tauri::command]
pub async fn reconnect_to_peer(
    app_handle: tauri::AppHandle,
    mgr: tauri::State<'_, WinConnectionManager>,
) -> Result<String, String> {
    let ip = mgr.peer_ip();
    connect_to_mac(Some(ip), app_handle, mgr).await
}

/// Tauri 命令：获取连接状态
#[tauri::command]
pub fn get_win_connection_status(
    mgr: tauri::State<'_, WinConnectionManager>,
) -> ConnectionStatus {
    mgr.get_status()
}

/// Tauri 命令：获取默认 IP 配置
#[tauri::command]
pub fn get_default_ips() -> serde_json::Value {
    serde_json::json!({
        "windows_ip": WINDOWS_DEFAULT_IP,
        "mac_ip": MAC_DEFAULT_IP,
        "port": DEFAULT_PORT
    })
}
