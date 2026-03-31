use std::net::{SocketAddr, UdpSocket};
use std::collections::HashMap;
use tokio::net::TcpStream;
use crate::types::{DeviceInfo, ConnectionType, ConnectionResult};

/// 网络管理器
pub struct NetworkManager {
    udp_socket: UdpSocket,
    discovered_devices: HashMap<String, SocketAddr>,
}

impl NetworkManager {
    pub fn new() -> Result<Self, String> {
        let udp_socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;
        
        Ok(Self {
            udp_socket,
            discovered_devices: HashMap::new(),
        })
    }

    /// 发现局域网内的设备
    pub async fn discover_devices(&mut self) -> Result<Vec<DeviceInfo>, String> {
        // 发送广播发现消息
        let broadcast_addr = "255.255.255.255:8888";
        let discovery_msg = b"KVM_DISCOVERY";
        
        self.udp_socket.set_broadcast(true)
            .map_err(|e| format!("Failed to set broadcast: {}", e))?;
        
        self.udp_socket.send_to(discovery_msg, broadcast_addr)
            .map_err(|e| format!("Failed to send discovery: {}", e))?;
        
        // 等待响应（简化实现）
        let mut buffer = [0u8; 1024];
        let mut devices = Vec::new();
        
        // 设置超时
        self.udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;
        
        // 接收响应
        while let Ok((size, src_addr)) = self.udp_socket.recv_from(&mut buffer) {
            if &buffer[..size] == b"KVM_RESPONSE" {
                let device = DeviceInfo {
                    id: src_addr.to_string(),
                    name: format!("Device-{}", src_addr.port()),
                    os: "Unknown".to_string(),
                    ip: src_addr.ip().to_string(),
                    connection_type: ConnectionType::Network,
                };
                devices.push(device);
                self.discovered_devices.insert(src_addr.to_string(), src_addr);
            }
        }
        
        Ok(devices)
    }

    /// 连接到指定设备
    pub async fn connect_to_device(&self, device_id: &str) -> Result<ConnectionResult, String> {
        let addr = self.discovered_devices.get(device_id)
            .ok_or("Device not found".to_string())?;
        
        // 尝试TCP连接
        match TcpStream::connect(addr).await {
            Ok(_stream) => {
                Ok(ConnectionResult {
                    success: true,
                    connection_type: Some(ConnectionType::Network),
                    error: None,
                })
            }
            Err(e) => {
                Ok(ConnectionResult {
                    success: false,
                    connection_type: None,
                    error: Some(format!("Connection failed: {}", e)),
                })
            }
        }
    }
}

#[tauri::command]
/// Tauri命令：发现网络设备
pub async fn discover_network_devices() -> Result<Vec<DeviceInfo>, String> {
    let mut manager = NetworkManager::new()?;
    manager.discover_devices().await
}

#[tauri::command]
/// Tauri命令：通过网络连接到设备
pub async fn connect_network_device(device_id: String) -> Result<ConnectionResult, String> {
    let manager = NetworkManager::new()?;
    manager.connect_to_device(&device_id).await
}