use std::sync::Arc;
use tokio::sync::Mutex;
use crate::types::{DeviceInfo, ConnectionType, ConnectionResult};
use crate::network;
use crate::usb;

/// 连接管理器
pub struct ConnectionManager {
    current_connection: Option<ActiveConnection>,
    devices: Vec<DeviceInfo>,
}

/// 活动连接
#[derive(Debug, Clone)]
pub struct ActiveConnection {
    device_id: String,
    connection_type: ConnectionType,
    connected_at: std::time::SystemTime,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            current_connection: None,
            devices: Vec::new(),
        }
    }

    /// 发现所有可用设备
    pub async fn discover_all_devices(&mut self) -> Result<Vec<DeviceInfo>, String> {
        let mut all_devices = Vec::new();
        
        // 网络设备发现
        match network::discover_network_devices().await {
            Ok(network_devices) => {
                all_devices.extend(network_devices);
            }
            Err(e) => {
                println!("Network discovery failed: {}", e);
            }
        }
        
        // USB设备发现
        match usb::scan_usb_devices().await {
            Ok(usb_devices) => {
                all_devices.extend(usb_devices);
            }
            Err(e) => {
                println!("USB discovery failed: {}", e);
            }
        }
        
        self.devices = all_devices.clone();
        Ok(all_devices)
    }

    /// 连接到设备
    pub async fn connect_to_device(&mut self, device_id: &str) -> Result<ConnectionResult, String> {
        // 查找设备
        let device = self.devices.iter()
            .find(|d| d.id == device_id)
            .ok_or("Device not found".to_string())?;
        
        let result = match device.connection_type {
            ConnectionType::Network => {
                network::connect_network_device(device_id.to_string()).await
            }
            ConnectionType::Usb => {
                usb::connect_usb_device(device_id.to_string()).await
            }
        };
        
        if let Ok(ref conn_result) = result {
            if conn_result.success {
                self.current_connection = Some(ActiveConnection {
                    device_id: device_id.to_string(),
                    connection_type: device.connection_type.clone(),
                    connected_at: std::time::SystemTime::now(),
                });
            }
        }
        
        result
    }

    /// 断开连接
    pub fn disconnect(&mut self) -> Result<(), String> {
        self.current_connection = None;
        Ok(())
    }

    /// 获取当前连接状态
    pub fn get_connection_status(&self) -> Option<ActiveConnection> {
        self.current_connection.clone()
    }

    /// 获取所有设备
    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        self.devices.clone()
    }
}

// 全局连接管理器实例
lazy_static::lazy_static! {
    static ref CONNECTION_MANAGER: Arc<Mutex<ConnectionManager>> = 
        Arc::new(Mutex::new(ConnectionManager::new()));
}

#[tauri::command]
/// Tauri命令：发现设备
pub async fn discover_devices() -> Result<Vec<DeviceInfo>, String> {
    let mut manager = CONNECTION_MANAGER.lock().await;
    manager.discover_all_devices().await
}

#[tauri::command]
/// Tauri命令：连接到设备
pub async fn connect_to_device(device_id: String) -> Result<ConnectionResult, String> {
    let mut manager = CONNECTION_MANAGER.lock().await;
    manager.connect_to_device(&device_id).await
}

#[tauri::command]
/// Tauri命令：断开连接
pub async fn disconnect_devices() -> Result<(), String> {
    let mut manager = CONNECTION_MANAGER.lock().await;
    manager.disconnect()
}

#[tauri::command]
/// Tauri命令：获取连接状态
pub async fn get_connection_status() -> Option<String> {
    let manager = CONNECTION_MANAGER.lock().await;
    manager.get_connection_status()
        .map(|conn| format!("Connected to {} via {:?}", conn.device_id, conn.connection_type))
}