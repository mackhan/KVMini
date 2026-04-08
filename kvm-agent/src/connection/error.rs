/// 连接错误管理子模块
///
/// 提供用户友好的错误分类、提示信息和解决方案引导。
/// 每种错误类型都附带中文说明和操作建议。

use std::fmt;

/// 连接错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectError {
    /// 网络不可达（目标 IP 无法 ping 通）
    NetworkUnreachable { peer_ip: String },
    /// 端口被拒绝（对端未启动或防火墙拦截）
    ConnectionRefused { peer_ip: String, port: u16 },
    /// 连接超时（网络延迟过高或目标不响应）
    Timeout { peer_ip: String, elapsed_secs: u64 },
    /// 协议版本不兼容
    ProtocolMismatch { local_version: u16, remote_version: u16 },
    /// 握手失败（收到意外帧）
    HandshakeFailed { reason: String },
    /// 心跳超时（连接中断）
    HeartbeatTimeout { last_seen_secs: u64 },
    /// 权限不足（macOS 辅助功能权限未授权）
    AccessibilityDenied,
    /// 端口已被占用（本机 8889 端口被其他程序占用）
    PortInUse { port: u16 },
    /// 未知错误
    Unknown { message: String },
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkUnreachable { peer_ip } => {
                write!(f, "无法连接到 {}：网络不可达", peer_ip)
            }
            Self::ConnectionRefused { peer_ip, port } => {
                write!(f, "连接 {}:{} 被拒绝：对端未启动或防火墙拦截", peer_ip, port)
            }
            Self::Timeout { peer_ip, elapsed_secs } => {
                write!(f, "连接 {} 超时（已等待 {} 秒）", peer_ip, elapsed_secs)
            }
            Self::ProtocolMismatch { local_version, remote_version } => {
                write!(
                    f,
                    "协议版本不兼容：本端 v{}，对端 v{}，请更新到相同版本",
                    local_version, remote_version
                )
            }
            Self::HandshakeFailed { reason } => {
                write!(f, "握手失败：{}", reason)
            }
            Self::HeartbeatTimeout { last_seen_secs } => {
                write!(f, "连接中断：{} 秒内未收到心跳", last_seen_secs)
            }
            Self::AccessibilityDenied => {
                write!(f, "辅助功能权限未授权：无法控制鼠标和键盘")
            }
            Self::PortInUse { port } => {
                write!(f, "端口 {} 已被占用：请关闭占用该端口的程序", port)
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
                format!("1. 确认 Windows 电脑（{}）已开机并联网", peer_ip),
                "2. 确认两台电脑在同一局域网（Wi-Fi 或有线网络）".to_string(),
                "3. 尝试在终端执行：ping ".to_string() + peer_ip,
                "4. 检查 Windows 防火墙是否允许 KVMini 通信".to_string(),
            ],
            ConnectError::ConnectionRefused { peer_ip, port } => vec![
                format!("1. 确认 Windows 端 KVMini 已启动（IP: {}，端口: {}）", peer_ip, port),
                "2. 检查 Windows 防火墙：允许 KVMini 的入站规则（TCP 8889）".to_string(),
                "3. 在 Windows 上执行：netstat -an | findstr 8889 确认端口已监听".to_string(),
                "4. 尝试重启 Windows 端 KVMini".to_string(),
            ],
            ConnectError::Timeout { peer_ip, .. } => vec![
                format!("1. 检查网络连接质量（ping {}）", peer_ip),
                "2. 确认两台电脑网络延迟 < 100ms".to_string(),
                "3. 尝试切换到有线网络连接".to_string(),
                "4. 检查是否有 VPN 或代理影响连接".to_string(),
            ],
            ConnectError::ProtocolMismatch { local_version, remote_version } => vec![
                format!(
                    "1. 版本不匹配：Mac 端 v{}，Windows 端 v{}",
                    local_version, remote_version
                ),
                "2. 请将两端 KVMini 更新到相同版本".to_string(),
                "3. 访问 GitHub 下载最新版本".to_string(),
            ],
            ConnectError::HandshakeFailed { reason } => vec![
                format!("1. 握手失败原因：{}", reason),
                "2. 确认对端运行的是 KVMini（非其他程序占用 8889 端口）".to_string(),
                "3. 尝试重启两端 KVMini".to_string(),
            ],
            ConnectError::HeartbeatTimeout { last_seen_secs } => vec![
                format!("1. 连接已中断（{} 秒无心跳）", last_seen_secs),
                "2. 检查网络是否稳定".to_string(),
                "3. KVMini 将自动尝试重连，请稍候".to_string(),
            ],
            ConnectError::AccessibilityDenied => vec![
                "1. 打开「系统设置」→「隐私与安全性」→「辅助功能」".to_string(),
                "2. 找到「kvm-agent」并开启开关".to_string(),
                "3. 重新启动 kvm-agent".to_string(),
                "4. 如未看到 kvm-agent，点击「+」手动添加".to_string(),
            ],
            ConnectError::PortInUse { port } => vec![
                format!("1. 端口 {} 已被占用", port),
                format!("2. 执行：lsof -i :{} 查看占用程序", port),
                "3. 关闭占用该端口的程序后重试".to_string(),
                "4. 或修改 KVMini 配置使用其他端口".to_string(),
            ],
            ConnectError::Unknown { message } => vec![
                format!("1. 错误详情：{}", message),
                "2. 尝试重启 kvm-agent".to_string(),
                "3. 查看日志获取更多信息".to_string(),
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
                if msg.contains("Network is unreachable") || msg.contains("No route to host") {
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
    fn test_error_display_network_unreachable() {
        let e = ConnectError::NetworkUnreachable { peer_ip: "30.19.24.43".to_string() };
        assert!(e.to_string().contains("30.19.24.43"));
        assert!(e.to_string().contains("网络不可达"));
    }

    #[test]
    fn test_error_guide_accessibility_denied() {
        let e = ConnectError::AccessibilityDenied;
        let guide = ErrorGuide::guide(&e);
        assert!(!guide.is_empty());
        assert!(guide[0].contains("辅助功能"));
    }

    #[test]
    fn test_from_io_error_connection_refused() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let err = ErrorGuide::from_io_error(&io_err, "30.19.24.43", 8889);
        assert_eq!(err, ConnectError::ConnectionRefused {
            peer_ip: "30.19.24.43".to_string(),
            port: 8889,
        });
    }
}
