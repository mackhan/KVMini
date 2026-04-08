/// kvm-agent 连接模块
///
/// 管理所有连接相关功能：
/// - `transport`：TCP 传输层（握手、帧收发、双向心跳、TCP Keep-Alive）
/// - `error`：连接错误管理，提供用户友好的错误提示和解决方案
/// - `manager`：连接生命周期管理（连接/断开/无限重连）

pub mod transport;
pub mod error;
pub mod manager;

pub use manager::{ConnectionManager, ConnectionEvent};
pub use error::{ConnectError, ErrorGuide};
pub use transport::{ConnectionState, PeerMode};

/// 默认连接配置
pub const DEFAULT_PEER_IP: &str = "30.19.24.43";  // 对端默认 IP
pub const DEFAULT_PORT: u16 = 8889;
