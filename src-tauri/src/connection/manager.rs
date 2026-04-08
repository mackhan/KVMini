/// Windows 端连接管理器子模块
///
/// 管理 Windows 端（主控端）的连接生命周期：
/// - 默认连接 Mac 端（30.19.140.39:8889）
/// - 支持角色切换（主控端 ↔ 被控端）
/// - 连接失败时提供用户友好的错误引导
/// - 支持自动重连

use std::sync::Arc;
use tauri::Manager;
use serde::{Serialize, Deserialize};

use super::error::{ConnectError, ErrorGuide};
use super::{MAC_DEFAULT_IP, WINDOWS_DEFAULT_IP, DEFAULT_PORT};

/// Windows 端连接角色
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WinRole {
    /// 主控端（Windows 端默认）：连接 Mac，发送键鼠事件
    Controller,
    /// 被控端：监听连接，接收并注入键鼠事件
    Controlled,
}

impl Default for WinRole {
    fn default() -> Self {
        Self::Controller // Windows 端默认主控端
    }
}

impl std::fmt::Display for WinRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Controller => write!(f, "主控端"),
            Self::Controlled => write!(f, "被控端"),
        }
    }
}

/// 连接状态（序列化给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub role: String,
    pub message: String,
}

/// 连接事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEvent {
    pub event_type: String,
    pub connected: bool,
    pub peer_ip: Option<String>,
    pub message: String,
    /// 错误解决方案（连接失败时）
    pub guide: Option<Vec<String>>,
}

impl ConnectionEvent {
    pub fn connected(peer_ip: String) -> Self {
        Self {
            event_type: "connected".to_string(),
            connected: true,
            peer_ip: Some(peer_ip.clone()),
            message: format!("已连接到 Mac（{}）", peer_ip),
            guide: None,
        }
    }

    pub fn disconnected(message: &str) -> Self {
        Self {
            event_type: "disconnected".to_string(),
            connected: false,
            peer_ip: None,
            message: message.to_string(),
            guide: None,
        }
    }

    pub fn error(error: &ConnectError) -> Self {
        Self {
            event_type: "error".to_string(),
            connected: false,
            peer_ip: None,
            message: error.to_string(),
            guide: Some(ErrorGuide::guide(error)),
        }
    }

    pub fn reconnecting(attempt: u32, max: u32, peer_ip: &str) -> Self {
        Self {
            event_type: "reconnecting".to_string(),
            connected: false,
            peer_ip: Some(peer_ip.to_string()),
            message: format!("重连中（{}/{}）→ {}", attempt, max, peer_ip),
            guide: None,
        }
    }
}

/// Windows 端连接管理器
///
/// 通过 Tauri 命令暴露给前端，管理与 Mac 端的连接。
pub struct WinConnectionManager {
    /// 当前角色
    role: std::sync::Mutex<WinRole>,
    /// 当前连接的对端 IP
    peer_ip: std::sync::Mutex<String>,
    /// 连接状态
    connected: std::sync::Mutex<bool>,
}

impl WinConnectionManager {
    pub fn new() -> Self {
        Self {
            role: std::sync::Mutex::new(WinRole::default()),
            peer_ip: std::sync::Mutex::new(MAC_DEFAULT_IP.to_string()),
            connected: std::sync::Mutex::new(false),
        }
    }

    /// 获取当前角色
    pub fn role(&self) -> WinRole {
        *self.role.lock().unwrap()
    }

    /// 设置角色
    pub fn set_role(&self, role: WinRole) {
        *self.role.lock().unwrap() = role;
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

    /// 获取连接状态（供前端查询）
    pub fn get_status(&self) -> ConnectionStatus {
        ConnectionStatus {
            connected: self.is_connected(),
            peer_ip: if self.is_connected() { Some(self.peer_ip()) } else { None },
            role: self.role().to_string(),
            message: if self.is_connected() {
                format!("已连接到 Mac（{}）", self.peer_ip())
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

/// Tauri 命令：连接到 Mac 端（使用默认或指定 IP）
#[tauri::command]
pub async fn connect_to_mac(
    peer_ip: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let ip = peer_ip.unwrap_or_else(|| MAC_DEFAULT_IP.to_string());
    let addr = format!("{}:{}", ip, DEFAULT_PORT);

    // 通知前端连接中
    let _ = app_handle.emit_all("connection-status", serde_json::json!({
        "event_type": "connecting",
        "connected": false,
        "peer_ip": ip,
        "message": format!("正在连接到 Mac（{}）...", ip),
        "guide": null
    }));

    // 尝试 TCP 连接（简单探测）
    match std::net::TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("无效地址: {}", e))?,
        std::time::Duration::from_secs(10),
    ) {
        Ok(_) => {
            let event = ConnectionEvent::connected(ip.clone());
            let _ = app_handle.emit_all("connection-status", &event);
            Ok(format!("已连接到 Mac（{}）", ip))
        }
        Err(e) => {
            let err = ErrorGuide::from_io_error(&e, &ip, DEFAULT_PORT);
            let event = ConnectionEvent::error(&err);
            let _ = app_handle.emit_all("connection-status", &event);
            Err(event.message)
        }
    }
}

/// Tauri 命令：断开连接
#[tauri::command]
pub async fn disconnect_from_mac(app_handle: tauri::AppHandle) -> Result<(), String> {
    let event = ConnectionEvent::disconnected("已断开与 Mac 的连接");
    let _ = app_handle.emit_all("connection-status", &event);
    Ok(())
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
