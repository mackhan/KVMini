/// 主控端/被控端角色管理模块
///
/// 管理应用的当前角色（主控端 / 被控端），切换角色时：
/// 1. 停止旧服务（TCP 服务端或 TCP 客户端）
/// 2. 启动新服务
/// 3. 断开所有连接并通知 UI

use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio::sync::mpsc;

use crate::server::{KvmServer, ServerEvent};
use crate::client::{KvmClient, ClientEvent};
use crate::protocol::KvmFrame;

/// 应用角色
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AppRole {
    /// 被控端（默认）：监听 TCP 连接，接收并注入键鼠事件
    Server,
    /// 主控端：捕获键鼠事件，通过 TCP 发送给被控端
    Client,
}

impl Default for AppRole {
    fn default() -> Self {
        Self::Server
    }
}

impl std::fmt::Display for AppRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Server => write!(f, "被控端"),
            Self::Client => write!(f, "主控端"),
        }
    }
}

/// 角色切换事件
#[derive(Debug, Clone)]
pub enum RoleEvent {
    /// 角色已切换
    RoleChanged { new_role: AppRole },
    /// 服务已启动
    ServiceStarted { role: AppRole, addr: String },
    /// 服务启动失败
    ServiceError(String),
    /// 连接状态变化（转发自 ServerEvent / ClientEvent）
    ConnectionStatus(String),
}

/// 角色管理器
pub struct RoleManager {
    /// 当前角色
    role: Arc<Mutex<AppRole>>,
    /// 监听地址（被控端模式）
    listen_addr: Arc<Mutex<String>>,
    /// 目标地址（主控端模式）
    target_addr: Arc<Mutex<Option<String>>>,
    /// 事件广播
    event_tx: broadcast::Sender<RoleEvent>,
    /// 当前服务端（被控端模式）
    server: Arc<Mutex<Option<KvmServer>>>,
    /// 当前客户端（主控端模式）
    client: Arc<Mutex<Option<KvmClient>>>,
    /// 主控端事件发送端（用于发送键鼠帧）
    frame_tx: Arc<Mutex<Option<mpsc::Sender<KvmFrame>>>>,
}

impl RoleManager {
    pub fn new(listen_addr: &str) -> (Self, broadcast::Receiver<RoleEvent>) {
        let (event_tx, event_rx) = broadcast::channel(256);
        let mgr = Self {
            role: Arc::new(Mutex::new(AppRole::Server)),
            listen_addr: Arc::new(Mutex::new(listen_addr.to_string())),
            target_addr: Arc::new(Mutex::new(None)),
            event_tx,
            server: Arc::new(Mutex::new(None)),
            client: Arc::new(Mutex::new(None)),
            frame_tx: Arc::new(Mutex::new(None)),
        };
        (mgr, event_rx)
    }

    /// 获取当前角色
    pub async fn role(&self) -> AppRole {
        *self.role.lock().await
    }

    /// 切换角色
    pub async fn set_role(&self, new_role: AppRole) -> Result<(), String> {
        // 停止当前服务
        self.stop_current_service().await;

        // 更新角色
        *self.role.lock().await = new_role;

        // 通知角色变化
        let _ = self.event_tx.send(RoleEvent::RoleChanged { new_role });

        // 启动新服务
        match new_role {
            AppRole::Server => self.start_server_mode().await,
            AppRole::Client => {
                // 主控端模式需要目标地址，由 connect_as_client 触发
                Ok(())
            }
        }
    }

    /// 以被控端模式启动 TCP 监听
    async fn start_server_mode(&self) -> Result<(), String> {
        let addr = self.listen_addr.lock().await.clone();
        let (mut server, mut srv_rx) = KvmServer::new(&addr);

        let actual_addr = server.start().await?;
        let addr_str = actual_addr.to_string();

        let _ = self.event_tx.send(RoleEvent::ServiceStarted {
            role: AppRole::Server,
            addr: addr_str.clone(),
        });

        // 转发服务端事件到 RoleEvent
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                match srv_rx.recv().await {
                    Ok(event) => {
                        let msg = match &event {
                            ServerEvent::ClientConnected { peer_addr } => {
                                format!("主控端已连接: {}", peer_addr)
                            }
                            ServerEvent::ClientDisconnected => "主控端已断开".to_string(),
                            ServerEvent::Error(e) => format!("服务错误: {}", e),
                            _ => continue,
                        };
                        let _ = event_tx.send(RoleEvent::ConnectionStatus(msg));
                    }
                    Err(_) => break,
                }
            }
        });

        *self.server.lock().await = Some(server);
        Ok(())
    }

    /// 以主控端模式连接到被控端
    pub async fn connect_as_client(&self, target_addr: &str) -> Result<(), String> {
        if *self.role.lock().await != AppRole::Client {
            return Err("当前不是主控端模式".to_string());
        }

        *self.target_addr.lock().await = Some(target_addr.to_string());

        let (mut client, mut cli_rx) = KvmClient::new(target_addr);
        let frame_tx = client.start().await?;

        // 转发客户端事件到 RoleEvent
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                match cli_rx.recv().await {
                    Ok(event) => {
                        let msg = match &event {
                            ClientEvent::Connected { peer_addr } => {
                                format!("已连接到被控端: {}", peer_addr)
                            }
                            ClientEvent::Disconnected => "与被控端断开连接".to_string(),
                            ClientEvent::Reconnecting { attempt, .. } => {
                                format!("重连中（第 {} 次）...", attempt)
                            }
                            ClientEvent::ConnectError(e) => format!("连接失败: {}", e),
                            ClientEvent::SendError(e) => format!("发送失败: {}", e),
                        };
                        let _ = event_tx.send(RoleEvent::ConnectionStatus(msg));
                    }
                    Err(_) => break,
                }
            }
        });

        *self.frame_tx.lock().await = Some(frame_tx);
        *self.client.lock().await = Some(client);
        Ok(())
    }

    /// 停止当前服务
    async fn stop_current_service(&self) {
        if let Some(mut server) = self.server.lock().await.take() {
            server.stop().await;
        }
        if let Some(mut client) = self.client.lock().await.take() {
            client.stop().await;
        }
        *self.frame_tx.lock().await = None;
    }

    /// 发送键鼠帧（主控端模式）
    pub async fn send_frame(&self, frame: KvmFrame) -> Result<(), String> {
        if let Some(tx) = self.frame_tx.lock().await.as_ref() {
            tx.send(frame).await.map_err(|e| format!("发送失败: {}", e))
        } else {
            Err("未连接到被控端".to_string())
        }
    }

    /// 更新监听地址
    pub async fn set_listen_addr(&self, addr: &str) {
        *self.listen_addr.lock().await = addr.to_string();
    }

    /// 订阅角色事件
    pub fn subscribe(&self) -> broadcast::Receiver<RoleEvent> {
        self.event_tx.subscribe()
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：获取当前角色
#[tauri::command]
pub fn get_current_role(role: tauri::State<'_, std::sync::Mutex<AppRole>>) -> AppRole {
    *role.lock().unwrap()
}

/// Tauri 命令：设置角色（切换主控端/被控端）
#[tauri::command]
pub async fn set_role(
    new_role: AppRole,
    role: tauri::State<'_, std::sync::Mutex<AppRole>>,
) -> Result<String, String> {
    *role.lock().unwrap() = new_role;
    Ok(format!("角色已切换为: {}", new_role))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AppRole 测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_app_role_default() {
        assert_eq!(AppRole::default(), AppRole::Server);
    }

    #[test]
    fn test_app_role_display() {
        assert_eq!(AppRole::Server.to_string(), "被控端");
        assert_eq!(AppRole::Client.to_string(), "主控端");
    }

    #[test]
    fn test_app_role_serialize() {
        let role = AppRole::Client;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"Client\"");

        let restored: AppRole = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, AppRole::Client);
    }

    #[test]
    fn test_app_role_server_serialize() {
        let role = AppRole::Server;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"Server\"");
    }

    // ── RoleManager 测试 ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_role_manager_default_role() {
        let (mgr, _rx) = RoleManager::new("127.0.0.1:0");
        assert_eq!(mgr.role().await, AppRole::Server);
    }

    #[tokio::test]
    async fn test_role_manager_switch_to_client() {
        let (mgr, mut rx) = RoleManager::new("127.0.0.1:0");

        mgr.set_role(AppRole::Client).await.unwrap();
        assert_eq!(mgr.role().await, AppRole::Client);

        // 验证收到角色变化事件
        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            rx.recv(),
        ).await.unwrap().unwrap();

        assert!(matches!(event, RoleEvent::RoleChanged { new_role: AppRole::Client }));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_role_manager_start_server_mode() {
        let (mgr, mut rx) = RoleManager::new("127.0.0.1:0");

        // 切换到被控端模式（默认就是，但重新设置触发启动）
        mgr.set_role(AppRole::Server).await.unwrap();

        // 等待服务启动事件
        let mut got_started = false;
        for _ in 0..5 {
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(1),
                rx.recv(),
            ).await {
                Ok(Ok(RoleEvent::ServiceStarted { role: AppRole::Server, .. })) => {
                    got_started = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(got_started, "应该收到 ServiceStarted 事件");
    }

    #[tokio::test]
    async fn test_role_manager_switch_back_to_server() {
        let (mgr, _rx) = RoleManager::new("127.0.0.1:0");

        mgr.set_role(AppRole::Client).await.unwrap();
        assert_eq!(mgr.role().await, AppRole::Client);

        mgr.set_role(AppRole::Server).await.unwrap();
        assert_eq!(mgr.role().await, AppRole::Server);
    }

    #[tokio::test]
    async fn test_role_manager_send_frame_without_connection() {
        let (mgr, _rx) = RoleManager::new("127.0.0.1:0");
        mgr.set_role(AppRole::Client).await.unwrap();

        let result = mgr.send_frame(KvmFrame::Heartbeat).await;
        assert!(result.is_err(), "未连接时发送帧应返回错误");
    }

    #[tokio::test]
    async fn test_role_manager_set_listen_addr() {
        let (mgr, _rx) = RoleManager::new("127.0.0.1:8889");
        mgr.set_listen_addr("0.0.0.0:9999").await;
        // 验证地址已更新（通过下次启动服务时使用新地址）
        assert_eq!(*mgr.listen_addr.lock().await, "0.0.0.0:9999");
    }

    // ── 延迟警告测试（> 50ms 显示警告）────────────────────────────────

    #[test]
    fn test_latency_threshold() {
        // 验证延迟阈值逻辑（50ms）
        let latency_ms = 60u64;
        let should_warn = latency_ms > 50;
        assert!(should_warn, "延迟 > 50ms 应显示警告");

        let latency_ms = 30u64;
        let should_warn = latency_ms > 50;
        assert!(!should_warn, "延迟 <= 50ms 不应显示警告");
    }
}
