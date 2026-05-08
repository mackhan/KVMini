//! macOS 权限检测与引导（辅助功能 + 输入监控）
//!
//! - Accessibility（辅助功能）：AXIsProcessTrustedWithOptions
//! - Input Monitoring（输入监控）：CGPreflightListenEventAccess / CGRequestListenEventAccess
//!
//! v0.1.36 起同时管理两种权限（BUG-027 / BUG-021）。

use kvm_core::error::KvmResult;

/// 权限状态（辅助功能 + 输入监控通用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityStatus {
    Granted,
    Denied,
    Unknown,
}

impl std::fmt::Display for AccessibilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Granted => write!(f, "Granted"),
            Self::Denied  => write!(f, "Denied"),
            Self::Unknown => write!(f, "NA"),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 辅助功能（Accessibility）
// ══════════════════════════════════════════════════════════════════════════

/// 检测辅助功能权限（不弹出系统对话框）
pub fn check_accessibility() -> AccessibilityStatus {
    #[cfg(target_os = "macos")]
    {
        let result = is_trusted(false);
        log::debug!(
            "[Mac] AXIsProcessTrustedWithOptions(prompt=false) = {}, pid={}",
            result,
            std::process::id()
        );
        if result { AccessibilityStatus::Granted } else { AccessibilityStatus::Denied }
    }
    #[cfg(not(target_os = "macos"))]
    { AccessibilityStatus::Unknown }
}

/// 请求辅助功能权限（弹出系统对话框引导用户授权）
pub fn request_accessibility() -> KvmResult<()> {
    log::info!("[Mac] 请求辅助功能权限");
    #[cfg(target_os = "macos")]
    { is_trusted(true); }
    Ok(())
}

/// 打开系统偏好设置 → 辅助功能页面
pub fn open_accessibility_settings() -> KvmResult<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        Command::new("open").arg(url).spawn()
            .map_err(|e| platform_err!("open", format!("打开系统设置失败: {}", e)))?;
        log::info!("[Mac] 已打开辅助功能设置页面");
    }
    Ok(())
}

/// 轮询等待辅助功能权限授予（最多等待 timeout_secs 秒）
pub fn wait_for_accessibility(timeout_secs: u64) -> bool {
    use std::time::{Duration, Instant};
    use std::thread;
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if check_accessibility() == AccessibilityStatus::Granted {
            log::info!("[Mac] 辅助功能权限已授予");
            return true;
        }
        thread::sleep(Duration::from_millis(500));
    }
    log::warn!("[Mac] 等待辅助功能权限超时（{}s）", timeout_secs);
    false
}

#[cfg(target_os = "macos")]
fn is_trusted(prompt: bool) -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = if prompt { CFBoolean::true_value() } else { CFBoolean::false_value() };
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
    }

    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
}

// ══════════════════════════════════════════════════════════════════════════
// 输入监控（Input Monitoring / ListenEvent Access）
// ══════════════════════════════════════════════════════════════════════════
//
// BUG-027 / BUG-021：macOS 15+ 对 CGEventTap 有独立的 TCC 权限（输入监控）。
// 仅授予辅助功能权限不够，CGEventTap::new 仍会失败。
//
// 关键点（参考 OnenoteCopy v1.0.12 经验）：
// 1. CGPreflightListenEventAccess 只读检查，无副作用，但进程内结果会缓存整个生命周期
// 2. 必须主动调用 CGRequestListenEventAccess 才能让 App 注册到 TCC 数据库
//    否则用户在系统设置的"输入监控"列表里根本看不到 KVMini 的开关
// 3. 授权后必须重启 App 才能让 Preflight 返回 true（进程内缓存不刷新）

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

/// 检测输入监控权限（只读，无副作用）
/// 注意：此结果在进程内被 CoreGraphics 缓存，授权后必须重启 App 才能刷新
pub fn check_input_monitoring() -> AccessibilityStatus {
    #[cfg(target_os = "macos")]
    {
        let granted = unsafe { CGPreflightListenEventAccess() };
        if granted { AccessibilityStatus::Granted } else { AccessibilityStatus::Denied }
    }
    #[cfg(not(target_os = "macos"))]
    { AccessibilityStatus::Unknown }
}

/// 主动请求输入监控权限
/// - 首次调用会把 KVMini 注册到 TCC 数据库的"输入监控"列表
/// - 用户未授权时系统可能弹出对话框（取决于 macOS 版本）
/// - 返回值：是否已授权（但由于进程内缓存，授权后仍需重启才会变 true）
pub fn request_input_monitoring() -> AccessibilityStatus {
    log::info!("[Mac] 请求输入监控权限（注册到 TCC）");
    #[cfg(target_os = "macos")]
    {
        let granted = unsafe { CGRequestListenEventAccess() };
        if granted { AccessibilityStatus::Granted } else { AccessibilityStatus::Denied }
    }
    #[cfg(not(target_os = "macos"))]
    { AccessibilityStatus::Unknown }
}

/// 打开系统偏好设置 → 输入监控页面
pub fn open_input_monitoring_settings() -> KvmResult<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent";
        Command::new("open").arg(url).spawn()
            .map_err(|e| platform_err!("open", format!("打开系统设置失败: {}", e)))?;
        log::info!("[Mac] 已打开输入监控设置页面");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_display() {
        assert_eq!(AccessibilityStatus::Granted.to_string(), "Granted");
        assert_eq!(AccessibilityStatus::Denied.to_string(), "Denied");
        assert_eq!(AccessibilityStatus::Unknown.to_string(), "NA");
    }

    #[test]
    fn test_check_input_monitoring_no_crash() {
        // 只验证调用不崩溃；实际权限状态因 CI/本地环境差异不固定
        let _ = check_input_monitoring();
    }

    #[test]
    fn test_check_accessibility_no_crash() {
        let _ = check_accessibility();
    }
}
