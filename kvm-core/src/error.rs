//! 统一错误类型定义
//!
//! 涵盖网络、协议、权限、配置等所有错误场景。
//! 每个变体提供用户友好的中文描述和建议的解决方案。

use thiserror::Error;

/// KVMini 统一错误类型
#[derive(Debug, Error)]
pub enum KvmError {
    // ── 网络错误 ──────────────────────────────────────────────────────

    /// 网络不可达（目标 IP 无法连接）
    #[error("无法连接到 {peer_ip}：网络不可达")]
    NetworkUnreachable { peer_ip: String },

    /// 连接被拒绝（对端未启动或防火墙拦截）
    #[error("连接 {peer_ip}:{port} 被拒绝：对端未启动或防火墙拦截")]
    ConnectionRefused { peer_ip: String, port: u16 },

    /// 连接超时
    #[error("连接 {peer_ip} 超时（已等待 {elapsed_secs} 秒）")]
    ConnectionTimeout { peer_ip: String, elapsed_secs: u64 },

    /// 心跳超时（连接中断）
    #[error("连接中断：{last_seen_secs} 秒内未收到心跳")]
    HeartbeatTimeout { last_seen_secs: u64 },

    /// 端口已被占用
    #[error("端口 {port} 已被占用")]
    PortInUse { port: u16 },

    /// 连接已断开
    #[error("连接已断开")]
    Disconnected,

    /// 重连失败（超过最大重试次数）
    #[error("重连失败：已重试 {max_retries} 次")]
    ReconnectFailed { max_retries: u32 },

    // ── 协议错误 ──────────────────────────────────────────────────────

    /// 协议版本不兼容
    #[error("版本不兼容：本端 v{local_version}，对端 v{remote_version}，请更新到相同版本")]
    ProtocolMismatch { local_version: u16, remote_version: u16 },

    /// 握手失败
    #[error("握手失败：{reason}")]
    HandshakeFailed { reason: String },

    /// 协议帧解码错误
    #[error("协议帧解码错误：{reason}")]
    FrameDecodeError { reason: String },

    /// 未知事件类型
    #[error("未知事件类型：0x{event_type:02X}")]
    UnknownEventType { event_type: u8 },

    /// 魔数错误
    #[error("魔数错误，非 KVMini 协议")]
    InvalidMagic,

    /// 载荷长度不匹配
    #[error("载荷长度不匹配：期望 {expected} 字节，实际 {got} 字节")]
    PayloadLengthMismatch { expected: usize, got: usize },

    /// 数据不足
    #[error("数据不足，需要更多字节")]
    Incomplete,

    // ── 权限错误 ──────────────────────────────────────────────────────

    /// 辅助功能权限未授权（macOS）
    #[error("辅助功能权限未授权：无法控制鼠标和键盘")]
    AccessibilityDenied,

    /// 密码验证失败
    #[error("密码错误")]
    AuthFailed,

    // ── 配置错误 ──────────────────────────────────────────────────────

    /// 配置文件读取失败
    #[error("配置文件读取失败：{reason}")]
    ConfigReadError { reason: String },

    /// 配置文件写入失败
    #[error("配置文件写入失败：{reason}")]
    ConfigWriteError { reason: String },

    /// 配置数据无效
    #[error("配置数据无效：{reason}")]
    ConfigInvalid { reason: String },

    // ── 平台错误 ──────────────────────────────────────────────────────

    /// 平台 API 调用失败
    #[error("平台 API 调用失败：{api} - {reason}")]
    PlatformError { api: String, reason: String },

    // ── 通用错误 ──────────────────────────────────────────────────────

    /// IO 错误
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),

    /// 未知错误
    #[error("未知错误：{message}")]
    Unknown { message: String },
}

impl KvmError {
    /// 获取错误分类名称
    pub fn category(&self) -> &'static str {
        match self {
            Self::NetworkUnreachable { .. }
            | Self::ConnectionRefused { .. }
            | Self::ConnectionTimeout { .. }
            | Self::HeartbeatTimeout { .. }
            | Self::PortInUse { .. }
            | Self::Disconnected
            | Self::ReconnectFailed { .. } => "网络",

            Self::ProtocolMismatch { .. }
            | Self::HandshakeFailed { .. }
            | Self::FrameDecodeError { .. }
            | Self::UnknownEventType { .. }
            | Self::InvalidMagic
            | Self::PayloadLengthMismatch { .. }
            | Self::Incomplete => "协议",

            Self::AccessibilityDenied | Self::AuthFailed => "权限",

            Self::ConfigReadError { .. }
            | Self::ConfigWriteError { .. }
            | Self::ConfigInvalid { .. } => "配置",

            Self::PlatformError { .. } => "平台",

            Self::Io(_) | Self::Unknown { .. } => "通用",
        }
    }

    /// 获取用户友好的解决方案建议
    pub fn suggestion(&self) -> Vec<String> {
        match self {
            Self::NetworkUnreachable { peer_ip } => vec![
                format!("确认对方电脑（{}）已开机并联网", peer_ip),
                "确认两台电脑在同一局域网".to_string(),
                "检查防火墙设置".to_string(),
            ],
            Self::ConnectionRefused { peer_ip, port } => vec![
                format!("确认对方电脑（{}）上的 KVMini 已启动", peer_ip),
                format!("确认 KVMini 正在监听端口 {}", port),
                "检查防火墙是否允许 KVMini 通信".to_string(),
            ],
            Self::ConnectionTimeout { peer_ip, .. } => vec![
                format!("检查网络连接质量（ping {}）", peer_ip),
                "尝试切换到有线网络".to_string(),
                "检查是否有 VPN 或代理影响连接".to_string(),
            ],
            Self::HeartbeatTimeout { .. } => vec![
                "检查网络是否稳定".to_string(),
                "KVMini 将自动尝试重连".to_string(),
            ],
            Self::PortInUse { port } => vec![
                format!("端口 {} 已被占用", port),
                "关闭占用该端口的程序后重试".to_string(),
            ],
            Self::Disconnected => vec![
                "连接已断开，请重新连接".to_string(),
            ],
            Self::ReconnectFailed { .. } => vec![
                "自动重连失败，请手动重新连接".to_string(),
            ],
            Self::ProtocolMismatch { .. } => vec![
                "请将两端 KVMini 更新到相同版本".to_string(),
            ],
            Self::HandshakeFailed { .. } => vec![
                "确认对端运行的是 KVMini".to_string(),
                "尝试重启两端 KVMini".to_string(),
            ],
            Self::AccessibilityDenied => vec![
                "打开「系统设置」→「隐私与安全性」→「辅助功能」".to_string(),
                "找到 KVMini 并开启开关".to_string(),
                "重新启动 KVMini".to_string(),
            ],
            Self::AuthFailed => vec![
                "请确认两端输入了相同的连接密码".to_string(),
            ],
            Self::ConfigReadError { .. } | Self::ConfigWriteError { .. } => vec![
                "检查配置文件权限".to_string(),
                "尝试删除配置文件后重启 KVMini".to_string(),
            ],
            _ => vec![
                "尝试重启 KVMini".to_string(),
            ],
        }
    }

    /// 从标准 IO 错误转换（附带连接上下文）
    pub fn from_io_with_context(e: &std::io::Error, peer_ip: &str, port: u16) -> Self {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::ConnectionRefused => Self::ConnectionRefused {
                peer_ip: peer_ip.to_string(),
                port,
            },
            ErrorKind::TimedOut => Self::ConnectionTimeout {
                peer_ip: peer_ip.to_string(),
                elapsed_secs: 10,
            },
            ErrorKind::AddrInUse => Self::PortInUse { port },
            _ => {
                let msg = e.to_string();
                if msg.contains("No route to host") || msg.contains("Network is unreachable") {
                    Self::NetworkUnreachable {
                        peer_ip: peer_ip.to_string(),
                    }
                } else {
                    Self::Unknown { message: msg }
                }
            }
        }
    }
}

/// KvmResult 类型别名
pub type KvmResult<T> = Result<T, KvmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = KvmError::ConnectionRefused {
            peer_ip: "192.168.1.100".to_string(),
            port: 23456,
        };
        assert!(e.to_string().contains("192.168.1.100"));
        assert!(e.to_string().contains("23456"));
    }

    #[test]
    fn test_error_category() {
        let e = KvmError::HeartbeatTimeout { last_seen_secs: 15 };
        assert_eq!(e.category(), "网络");

        let e = KvmError::InvalidMagic;
        assert_eq!(e.category(), "协议");

        let e = KvmError::AuthFailed;
        assert_eq!(e.category(), "权限");
    }

    #[test]
    fn test_error_suggestion_not_empty() {
        let e = KvmError::AccessibilityDenied;
        let suggestions = e.suggestion();
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("辅助功能"));
    }

    #[test]
    fn test_from_io_with_context() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let err = KvmError::from_io_with_context(&io_err, "192.168.1.100", 23456);
        match err {
            KvmError::ConnectionRefused { peer_ip, port } => {
                assert_eq!(peer_ip, "192.168.1.100");
                assert_eq!(port, 23456);
            }
            _ => panic!("期望 ConnectionRefused"),
        }
    }
}
