use std::net::{SocketAddr, UdpSocket};
use std::collections::HashMap;
use tokio::net::TcpStream;
use crate::types::{DeviceInfo, ConnectionType, ConnectionResult};

/// UDP 广播发现协议常量
pub const DISCOVERY_MSG: &[u8] = b"KVM_DISCOVERY";
pub const DISCOVERY_PORT: u16 = 8888;
/// 扫描固定等待时间：5 秒
pub const SCAN_TIMEOUT_SECS: u64 = 5;

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
    /// 发送 KVM_DISCOVERY 广播，固定等待 5 秒收集所有响应
    pub async fn discover_devices(&mut self) -> Result<Vec<DeviceInfo>, String> {
        let broadcast_addr = format!("255.255.255.255:{}", DISCOVERY_PORT);

        self.udp_socket.set_broadcast(true)
            .map_err(|e| format!("Failed to set broadcast: {}", e))?;

        self.udp_socket.send_to(DISCOVERY_MSG, &broadcast_addr)
            .map_err(|e| format!("Failed to send discovery: {}", e))?;

        // 固定等待 5 秒收集所有响应
        self.udp_socket.set_read_timeout(Some(std::time::Duration::from_secs(SCAN_TIMEOUT_SECS)))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;

        let mut buffer = [0u8; 1024];
        let mut devices = Vec::new();

        // 持续接收，直到超时
        while let Ok((size, src_addr)) = self.udp_socket.recv_from(&mut buffer) {
            let data = &buffer[..size];

            // 尝试解析 JSON 格式响应（新格式）
            if let Ok(text) = std::str::from_utf8(data) {
                if text.starts_with("KVM_RESPONSE:") {
                    // 新格式：KVM_RESPONSE:{"name":"...","os":"...","tcp_port":...}
                    let json_part = &text["KVM_RESPONSE:".len()..];
                    if let Ok(info) = serde_json::from_str::<serde_json::Value>(json_part) {
                        let name = info["name"].as_str().unwrap_or("KVMini Device").to_string();
                        let os = info["os"].as_str().unwrap_or("Unknown").to_string();
                        let tcp_port = info["tcp_port"].as_u64().unwrap_or(9999) as u16;
                        let ip = src_addr.ip().to_string();
                        let id = format!("{}:{}", ip, tcp_port);

                        let device = DeviceInfo {
                            id: id.clone(),
                            name,
                            os,
                            ip,
                            connection_type: ConnectionType::Network,
                        };
                        devices.push(device);
                        self.discovered_devices.insert(id, SocketAddr::new(src_addr.ip(), tcp_port));
                    }
                } else if data == b"KVM_RESPONSE" {
                    // 旧格式兼容
                    let device = DeviceInfo {
                        id: src_addr.to_string(),
                        name: format!("KVMini-{}", src_addr.port()),
                        os: "Unknown".to_string(),
                        ip: src_addr.ip().to_string(),
                        connection_type: ConnectionType::Network,
                    };
                    devices.push(device);
                    self.discovered_devices.insert(src_addr.to_string(), src_addr);
                }
            }
        }

        Ok(devices)
    }

    /// 连接到指定设备
    pub async fn connect_to_device(&self, device_id: &str) -> Result<ConnectionResult, String> {
        let addr = self.discovered_devices.get(device_id)
            .ok_or("Device not found".to_string())?;

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