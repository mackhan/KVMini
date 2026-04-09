//! macOS 辅助功能权限检测与引导

use kvm_core::error::KvmResult;

/// 权限状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityStatus {
    Granted,
    Denied,
    Unknown,
}

impl std::fmt::Display for AccessibilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Granted => write!(f, "已授权"),
            Self::Denied  => write!(f, "未授权"),
            Self::Unknown => write!(f, "未知"),
        }
    }
}

/// 检测辅助功能权限（不弹出系统对话框）
pub fn check_accessibility() -> AccessibilityStatus {
    #[cfg(target_os = "macos")]
    {
        if is_trusted(false) { AccessibilityStatus::Granted } else { AccessibilityStatus::Denied }
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

/// 轮询等待权限授予（最多等待 timeout_secs 秒）
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
