//! 配置数据结构定义与序列化
//!
//! 定义统一的配置数据结构，各端 config_store 负责具体的文件路径和读写。

use serde::{Deserialize, Serialize};

/// 屏幕布局方向（本机相对于对方的位置）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenLayout {
    /// 本机在左，对方在右（默认）
    LocalLeft,
    /// 本机在右，对方在左
    LocalRight,
    /// 本机在上，对方在下
    LocalAbove,
    /// 本机在下，对方在上
    LocalBelow,
}

impl Default for ScreenLayout {
    fn default() -> Self {
        Self::LocalLeft
    }
}

impl ScreenLayout {
    /// 获取触发边缘方向
    pub fn trigger_edge(&self) -> crate::protocol::ScreenEdge {
        match self {
            Self::LocalLeft => crate::protocol::ScreenEdge::Right,
            Self::LocalRight => crate::protocol::ScreenEdge::Left,
            Self::LocalAbove => crate::protocol::ScreenEdge::Bottom,
            Self::LocalBelow => crate::protocol::ScreenEdge::Top,
        }
    }

    /// 获取对端入口边缘方向
    pub fn remote_entry_edge(&self) -> crate::protocol::ScreenEdge {
        self.trigger_edge().opposite()
    }
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// KVMini 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmConfig {
    /// 屏幕布局
    pub screen_layout: ScreenLayout,
    /// 连接密码（空字符串表示无密码）
    pub password: String,
    /// 对端 IP 地址（上次连接的 IP）
    pub peer_ip: String,
    /// TCP 端口
    pub tcp_port: u16,
    /// UDP 发现端口
    pub udp_port: u16,
    /// 是否启用剪贴板共享
    pub clipboard_enabled: bool,
    /// 是否开机自启动
    pub autostart: bool,
    /// 日志级别
    pub log_level: LogLevel,
    /// 边缘触发防抖时间（毫秒）
    pub edge_debounce_ms: u64,
    /// 边缘过冲量（像素），需推过边缘 N 像素才切换，0 = 贴边即切，默认 20，范围 0~200
    pub edge_overshoot_px: i32,
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    /// 心跳超时次数
    pub heartbeat_timeout_count: u32,
    /// 重连间隔（秒）
    pub reconnect_interval_secs: u64,
    /// 最大重连次数
    pub max_reconnect_retries: u32,
}

impl Default for KvmConfig {
    fn default() -> Self {
        Self {
            screen_layout: ScreenLayout::default(),
            password: String::new(),
            peer_ip: String::new(),
            tcp_port: 23456,
            udp_port: 8888,
            clipboard_enabled: true,
            autostart: false,
            log_level: LogLevel::default(),
            edge_debounce_ms: 50,
            edge_overshoot_px: 20,
            heartbeat_interval_secs: 5,
            heartbeat_timeout_count: 3,
            reconnect_interval_secs: 3,
            max_reconnect_retries: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KvmConfig::default();
        assert_eq!(config.screen_layout, ScreenLayout::LocalLeft);
        assert_eq!(config.tcp_port, 23456);
        assert_eq!(config.udp_port, 8888);
        assert!(config.clipboard_enabled);
        assert!(!config.autostart);
        assert_eq!(config.edge_debounce_ms, 50);
        assert_eq!(config.edge_overshoot_px, 20);
        assert_eq!(config.heartbeat_interval_secs, 5);
        assert_eq!(config.heartbeat_timeout_count, 3);
        assert_eq!(config.reconnect_interval_secs, 3);
        assert_eq!(config.max_reconnect_retries, 5);
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let config = KvmConfig {
            screen_layout: ScreenLayout::LocalRight,
            password: "test123".to_string(),
            peer_ip: "192.168.1.100".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: KvmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.screen_layout, ScreenLayout::LocalRight);
        assert_eq!(restored.password, "test123");
        assert_eq!(restored.peer_ip, "192.168.1.100");
    }

    #[test]
    fn test_screen_layout_trigger_edge() {
        use crate::protocol::ScreenEdge;
        assert_eq!(ScreenLayout::LocalLeft.trigger_edge(), ScreenEdge::Right);
        assert_eq!(ScreenLayout::LocalRight.trigger_edge(), ScreenEdge::Left);
        assert_eq!(ScreenLayout::LocalAbove.trigger_edge(), ScreenEdge::Bottom);
        assert_eq!(ScreenLayout::LocalBelow.trigger_edge(), ScreenEdge::Top);
    }

    #[test]
    fn test_screen_layout_remote_entry_edge() {
        use crate::protocol::ScreenEdge;
        assert_eq!(ScreenLayout::LocalLeft.remote_entry_edge(), ScreenEdge::Left);
        assert_eq!(ScreenLayout::LocalRight.remote_entry_edge(), ScreenEdge::Right);
        assert_eq!(ScreenLayout::LocalAbove.remote_entry_edge(), ScreenEdge::Top);
        assert_eq!(ScreenLayout::LocalBelow.remote_entry_edge(), ScreenEdge::Bottom);
    }
}
