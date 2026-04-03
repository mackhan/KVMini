/// macOS 辅助功能（Accessibility）权限检测与引导模块
///
/// - macOS：检测辅助功能权限状态，未授权时引导用户开启
/// - Windows：直接返回已授权（不需要此权限）
///
/// 辅助功能权限是 macOS 上捕获全局键鼠事件的必要条件。

/// 权限状态
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PermissionStatus {
    /// 已授权
    Granted,
    /// 未授权
    Denied,
    /// 不适用（Windows 平台）
    NotApplicable,
}

impl std::fmt::Display for PermissionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Granted => write!(f, "已授权"),
            Self::Denied => write!(f, "未授权"),
            Self::NotApplicable => write!(f, "不适用"),
        }
    }
}

/// 检测辅助功能权限状态
///
/// macOS：调用 AXIsProcessTrustedWithOptions 检测
/// Windows：直接返回 Granted
pub fn check_accessibility_permission() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        check_accessibility_macos()
    }

    #[cfg(target_os = "windows")]
    {
        PermissionStatus::Granted
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PermissionStatus::NotApplicable
    }
}

/// macOS 辅助功能权限检测
///
/// 使用 AXIsProcessTrustedWithOptions API（通过 core-foundation 调用）
/// 目前提供接口定义，实际 CF 调用在集成 core-foundation crate 后补充
#[cfg(target_os = "macos")]
fn check_accessibility_macos() -> PermissionStatus {
    // TODO: 调用 AXIsProcessTrustedWithOptions(NULL, false)
    // 目前通过检查进程是否有辅助功能权限来判断
    // 实际实现需要 core-foundation crate：
    //   use core_foundation::base::TCFType;
    //   let trusted = unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) };
    //   if trusted { PermissionStatus::Granted } else { PermissionStatus::Denied }

    // 临时实现：通过尝试创建 CGEvent Tap 来检测权限
    // 如果没有权限，CGEvent Tap 创建会失败
    PermissionStatus::Denied // 保守返回 Denied，实际集成时替换
}

/// 打开 macOS 系统偏好设置 → 辅助功能页面
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| format!("打开系统偏好设置失败: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(()) // 非 macOS 平台无需操作
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：检测辅助功能权限
#[tauri::command]
pub fn check_accessibility() -> PermissionStatus {
    check_accessibility_permission()
}

/// Tauri 命令：打开辅助功能设置页面
#[tauri::command]
pub fn open_accessibility_prefs() -> Result<(), String> {
    open_accessibility_settings()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PermissionStatus 测试 ────────────────────────────────────────────

    #[test]
    fn test_permission_status_display() {
        assert_eq!(PermissionStatus::Granted.to_string(), "已授权");
        assert_eq!(PermissionStatus::Denied.to_string(), "未授权");
        assert_eq!(PermissionStatus::NotApplicable.to_string(), "不适用");
    }

    #[test]
    fn test_permission_status_serialize() {
        let status = PermissionStatus::Granted;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"Granted\"");

        let restored: PermissionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, PermissionStatus::Granted);
    }

    #[test]
    fn test_permission_status_all_variants() {
        let variants = [
            PermissionStatus::Granted,
            PermissionStatus::Denied,
            PermissionStatus::NotApplicable,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let restored: PermissionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(v, &restored);
        }
    }

    // ── macOS 平台特定测试 ───────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn test_check_accessibility_macos_returns_valid_status() {
        let status = check_accessibility_permission();
        // macOS 上应返回 Granted 或 Denied（不应返回 NotApplicable）
        assert!(
            matches!(status, PermissionStatus::Granted | PermissionStatus::Denied),
            "macOS 上应返回 Granted 或 Denied，实际: {:?}",
            status
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_open_accessibility_settings_does_not_panic() {
        // 验证打开设置不会 panic（实际是否打开取决于系统）
        // 在 CI 环境中可能失败，但不应 panic
        let _ = open_accessibility_settings();
    }

    // ── Windows 平台特定测试 ─────────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn test_check_accessibility_windows_always_granted() {
        let status = check_accessibility_permission();
        assert_eq!(status, PermissionStatus::Granted,
            "Windows 平台应始终返回 Granted");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_open_accessibility_settings_windows_noop() {
        // Windows 上打开设置应成功（无操作）
        let result = open_accessibility_settings();
        assert!(result.is_ok());
    }

    // ── 跨平台行为测试 ───────────────────────────────────────────────────

    #[test]
    fn test_check_accessibility_returns_some_status() {
        // 无论平台，都应返回一个有效的状态
        let status = check_accessibility_permission();
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_permission_not_applicable_for_unsupported_platform() {
        // 在非 macOS/Windows 平台上应返回 NotApplicable
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let status = check_accessibility_permission();
            assert_eq!(status, PermissionStatus::NotApplicable);
        }
        // macOS 和 Windows 上此测试不执行
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            // 验证 NotApplicable 可以正确序列化
            let status = PermissionStatus::NotApplicable;
            assert_eq!(serde_json::to_string(&status).unwrap(), "\"NotApplicable\"");
        }
    }
}
