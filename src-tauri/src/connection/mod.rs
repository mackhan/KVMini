/// src-tauri Windows 端连接模块
///
/// 管理 Windows 端所有连接相关功能：
/// - `error`：连接错误管理，提供用户友好的错误提示和解决方案引导
/// - `manager`：连接生命周期管理（连接/断开/重连）
///
/// Windows 端默认为主控端（IP: 30.19.24.43），Mac 端默认为被控端（IP: 30.19.140.39）

pub mod error;
pub mod manager;

pub use error::{ConnectError, ErrorGuide};
pub use manager::WinConnectionManager;

/// Windows 端默认 IP
pub const WINDOWS_DEFAULT_IP: &str = "30.19.24.43";
/// Mac 端默认 IP（Windows 端连接的目标）
pub const MAC_DEFAULT_IP: &str = "30.19.140.39";
/// 默认端口
pub const DEFAULT_PORT: u16 = 8889;
