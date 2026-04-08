//! Windows 系统托盘
//!
//! 通过 Tauri 系统托盘提供快捷操作。
//! 托盘图标颜色区分状态：绿色=IDLE，蓝色=LOCAL_CONTROLLING，橙色=REMOTE_CONTROLLING。

use kvm_core::state::ControlState;
use kvm_core::network::ConnectionState;

/// 托盘状态信息
#[derive(Debug, Clone)]
pub struct TrayStatus {
    pub connection: ConnectionState,
    pub control: ControlState,
    pub peer_name: Option<String>,
}

impl TrayStatus {
    pub fn new() -> Self {
        Self {
            connection: ConnectionState::Disconnected,
            control: ControlState::Idle,
            peer_name: None,
        }
    }

    /// 获取托盘提示文本
    pub fn tooltip(&self) -> String {
        match self.connection {
            ConnectionState::Disconnected => "KVMini - 未连接".to_string(),
            ConnectionState::Connecting => "KVMini - 正在连接...".to_string(),
            ConnectionState::Connected => {
                let peer = self.peer_name.as_deref().unwrap_or("对方");
                match self.control {
                    ControlState::Idle => format!("KVMini - 已连接 · 各自独立"),
                    ControlState::LocalControlling => format!("KVMini - 正在控制{}", peer),
                    ControlState::RemoteControlling => format!("KVMini - {}正在控制本机", peer),
                }
            }
            ConnectionState::Reconnecting { attempt } => {
                format!("KVMini - 正在重连（第 {} 次）", attempt)
            }
        }
    }

    /// 获取托盘图标颜色标识
    pub fn icon_color(&self) -> &'static str {
        match self.control {
            ControlState::Idle => "green",
            ControlState::LocalControlling => "blue",
            ControlState::RemoteControlling => "orange",
        }
    }
}

/// 托盘菜单项
#[derive(Debug, Clone)]
pub enum TrayAction {
    /// 扫描设备
    ScanDevices,
    /// 手动输入 IP 连接
    ManualConnect,
    /// 断开连接
    Disconnect,
    /// 打开设置
    OpenSettings,
    /// 退出
    Quit,
}
