/// Windows 端连接错误管理子模块
///
/// 提供用户友好的错误分类、提示信息和解决方案引导。
/// 区分 Windows 端（主控端）和 Mac 端（被控端）的错误场景。

use std::fmt;
use serde::{Serialize, Deserialize};

/// 连接错误类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectError {
    /// 网络不可达（目标 IP 无法连接）
    NetworkUnreachable { peer_ip: String },
    /// 端口被拒绝（Mac 端 kvm-agent 未启动）
    ConnectionRefused { peer_ip: String, port: u16 },
    /// 连接超时
    Timeout { peer_ip: String, elapsed_secs: u64 },
    /// 协议版本不兼容
    ProtocolMismatch { local_version: u8, remote_version: u8 },
    /// 握手失败
    HandshakeFailed { reason: String },
    /// 心跳超时（连接中断）
    HeartbeatTimeout { last_seen_secs: u64 },
    /// 端口已被占用（Windows 端 8889 端口被占用）
    PortInUse { port: u16 },
    /// Mac 端 kvm-agent 未运行
    AgentNotRunning { peer_ip: String },
    /// 未知错误
    Unknown { message: String },
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkUnreachable { peer_ip } => {
                write!(f, "无法连接到 Mac（{}）：网络不可达", peer_ip)
            }
            Self::ConnectionRefused { peer_ip, port } => {
                write!(f, "Mac（{}:{}）拒绝连接：kvm-agent 未启动", peer_ip, port)
            }
            Self::Timeout { peer_ip, elapsed_secs } => {
                write!(f, "连接 Mac（{}）超时（{}秒）", peer_ip, elapsed_secs)
            }
            Self::ProtocolMismatch { local_version, remote_version } => {
                write!(f, "协议版本不兼容：Windows v{}，Mac v{}", local_version, remote_version)
            }
            Self::HandshakeFailed { reason } => {
                write!(f, "握手失败：{}", reason)
            }
            Self::HeartbeatTimeout { last_seen_secs } => {
                write!(f, "连接中断：{}秒内未收到心跳", last_seen_secs)
            }
            Self::PortInUse { port } => {
                write!(f, "端口 {} 已被占用", port)
            }
            Self::AgentNotRunning { peer_ip } => {
                write!(f, "Mac（{}）上的 kvm-agent 未运行", peer_ip)
            }
            Self::Unknown { message } => {
                write!(f, "未知错误：{}", message)
            }
        }
    }
}

/// 错误解决方案引导
pub struct ErrorGuide;

impl ErrorGuide {
    /// 根据错误类型返回用户友好的解决步骤
    pub fn guide(error: &ConnectError) -> Vec<String> {
        match error {
            ConnectError::NetworkUnreachable { peer_ip } => vec![
                format!("1. 确认 Mac 电脑（{}）已开机并联网", peer_ip),
                "2. 确认两台电脑在同一局域网".to_string(),
                format!("3. 在命令提示符执行：ping {}", peer_ip),
                "4. 检查 Mac 防火墙设置".to_string(),
            ],
            ConnectError::ConnectionRefused { peer_ip, port } => vec![
                format!("1. 在 Mac（{}）上启动 kvm-agent", peer_ip),
                format!("2. 确认 kvm-agent 正在监听端口 {}", port),
                "3. 在 Mac 终端执行：lsof -i :8889 确认端口已监听".to_string(),
                "4. 检查 Mac 防火墙是否允许 kvm-agent 接受连接".to_string(),
            ],
            ConnectError::AgentNotRunning { peer_ip } => vec![
                format!("1. 在 Mac（{}）上启动 kvm-agent", peer_ip),
                "2. 确认 kvm-agent 正在监听端口 8889".to_string(),
                "3. 在 Mac 终端执行：lsof -i :8889 确认端口已监听".to_string(),
                "4. 检查 Mac 防火墙是否允许 kvm-agent 接受连接".to_string(),
            ],
            ConnectError::Timeout { peer_ip, .. } => vec![
                format!("1. 检查网络连接质量（ping {}）", peer_ip),
                "2. 确认两台电脑网络延迟 < 100ms".to_string(),
                "3. 尝试切换到有线网络".to_string(),
                "4. 检查是否有 VPN 或代理影响连接".to_string(),
            ],
            ConnectError::ProtocolMismatch { local_version, remote_version } => vec![
                format!("1. 版本不匹配：Windows v{}，Mac v{}", local_version, remote_version),
                "2. 请将两端 KVMini 更新到相同版本".to_string(),
            ],
            ConnectError::HandshakeFailed { reason } => vec![
                format!("1. 握手失败：{}", reason),
                "2. 确认 Mac 端运行的是 kvm-agent（非其他程序）".to_string(),
                "3. 尝试重启两端 KVMini".to_string(),
            ],
            ConnectError::HeartbeatTimeout { last_seen_secs } => vec![
                format!("1. 连接已中断（{}秒无心跳）", last_seen_secs),
                "2. 检查网络是否稳定".to_string(),
                "3. KVMini 将自动尝试重连".to_string(),
            ],
            ConnectError::PortInUse { port } => vec![
                format!("1. 端口 {} 已被占用", port),
                format!("2. 在命令提示符执行：netstat -ano | findstr {}", port),
                "3. 关闭占用该端口的程序后重试".to_string(),
            ],
            ConnectError::Unknown { message } => vec![
                format!("1. 错误详情：{}", message),
                "2. 尝试重启 KVMini".to_string(),
            ],
        }
    }

    /// 将标准 IO 错误转换为 ConnectError
    pub fn from_io_error(e: &std::io::Error, peer_ip: &str, port: u16) -> ConnectError {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::ConnectionRefused => ConnectError::ConnectionRefused {
                peer_ip: peer_ip.to_string(),
                port,
            },
            ErrorKind::TimedOut => ConnectError::Timeout {
                peer_ip: peer_ip.to_string(),
                elapsed_secs: 10,
            },
            ErrorKind::AddrInUse => ConnectError::PortInUse { port },
            _ => {
                let msg = e.to_string();
                if msg.contains("No route to host") || msg.contains("Network is unreachable") {
                    ConnectError::NetworkUnreachable { peer_ip: peer_ip.to_string() }
                } else {
                    ConnectError::Unknown { message: msg }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_connection_refused() {
        let e = ConnectError::ConnectionRefused {
            peer_ip: "30.19.140.39".to_string(),
            port: 8889,
        };
        assert!(e.to_string().contains("30.19.140.39"));
        assert!(e.to_string().contains("kvm-agent"));
    }

    #[test]
    fn test_error_guide_not_empty() {
        let e = ConnectError::AgentNotRunning { peer_ip: "30.19.140.39".to_string() };
        let guide = ErrorGuide::guide(&e);
        assert!(!guide.is_empty());
        assert!(guide[0].contains("kvm-agent"));
    }
}
