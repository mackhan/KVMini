//! 统一日志接口定义
//!
//! 各端可接入不同日志后端（文件、控制台等）。
//! kvm-core 通过标准 `log` crate 宏输出日志。

/// 日志级别过滤器
pub use log::LevelFilter;

/// 初始化日志系统的配置
pub struct LogConfig {
    /// 日志级别
    pub level: LevelFilter,
    /// 是否输出到控制台
    pub console: bool,
    /// 日志文件路径（None 表示不输出到文件）
    pub file_path: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::Info,
            console: true,
            file_path: None,
        }
    }
}

/// 日志初始化 trait
///
/// 各端实现此 trait 来配置日志后端。
pub trait LogInitializer {
    /// 初始化日志系统
    fn init(config: LogConfig) -> Result<(), String>;
}
