//! 辅助宏和工具函数

/// 快速创建 KvmError::PlatformError
/// 用法：platform_err!("API名称", "错误原因")
#[macro_export]
macro_rules! platform_err {
    ($api:expr, $reason:expr) => {
        kvm_core::error::KvmError::PlatformError {
            api: $api.to_string(),
            reason: $reason.to_string(),
        }
    };
    ($reason:expr) => {
        kvm_core::error::KvmError::PlatformError {
            api: "platform".to_string(),
            reason: $reason.to_string(),
        }
    };
}
