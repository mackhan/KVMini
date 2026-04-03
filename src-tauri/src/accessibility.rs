/// macOS 辅助功能（Accessibility）权限检测与引导模块
///
/// 平台隔离设计：
/// - 定义 `AccessibilityChecker` trait 作为统一接口
/// - macOS 实现：调用 `AXIsProcessTrustedWithOptions` 系统 API
/// - Windows 实现：直接返回 `Granted`（不需要此权限）
/// - Mock 实现：用于单元测试
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

// ── AccessibilityChecker trait（平台隔离接口）──────────────────────────────

/// 辅助功能权限检测接口（平台隔离 trait）
///
/// 上层模块通过此 trait 调用权限检测，不直接调用平台 API。
pub trait AccessibilityChecker: Send + Sync {
    /// 检测辅助功能权限状态
    fn check_permission(&self) -> PermissionStatus;

    /// 打开系统辅助功能设置页面
    fn open_settings(&self) -> Result<(), String>;
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

/// macOS 辅助功能权限检测器
///
/// 调用 `AXIsProcessTrustedWithOptions` 系统 API 真实检测权限状态。
/// 使用 `core-foundation` crate 进行 FFI 调用。
#[cfg(target_os = "macos")]
pub struct MacOSAccessibilityChecker;

#[cfg(target_os = "macos")]
impl MacOSAccessibilityChecker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
impl AccessibilityChecker for MacOSAccessibilityChecker {
    fn check_permission(&self) -> PermissionStatus {
        check_accessibility_macos()
    }

    fn open_settings(&self) -> Result<(), String> {
        open_accessibility_settings_macos()
    }
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// Windows 辅助功能权限检测器（空实现，Windows 不需要此权限）
#[cfg(target_os = "windows")]
pub struct WindowsAccessibilityChecker;

#[cfg(target_os = "windows")]
impl WindowsAccessibilityChecker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
impl AccessibilityChecker for WindowsAccessibilityChecker {
    fn check_permission(&self) -> PermissionStatus {
        // Windows 不需要辅助功能权限，始终返回 Granted
        PermissionStatus::Granted
    }

    fn open_settings(&self) -> Result<(), String> {
        // Windows 无需打开辅助功能设置，空操作
        Ok(())
    }
}

// ── Mock 实现（用于测试）────────────────────────────────────────────────────

/// Mock 辅助功能权限检测器，用于单元测试
pub struct MockAccessibilityChecker {
    pub status: std::sync::Mutex<PermissionStatus>,
    pub open_settings_called: std::sync::Mutex<bool>,
}

impl MockAccessibilityChecker {
    pub fn new(status: PermissionStatus) -> Self {
        Self {
            status: std::sync::Mutex::new(status),
            open_settings_called: std::sync::Mutex::new(false),
        }
    }

    pub fn set_status(&self, status: PermissionStatus) {
        *self.status.lock().unwrap() = status;
    }

    pub fn was_open_settings_called(&self) -> bool {
        *self.open_settings_called.lock().unwrap()
    }
}

impl AccessibilityChecker for MockAccessibilityChecker {
    fn check_permission(&self) -> PermissionStatus {
        self.status.lock().unwrap().clone()
    }

    fn open_settings(&self) -> Result<(), String> {
        *self.open_settings_called.lock().unwrap() = true;
        Ok(())
    }
}

// ── 工厂函数 ────────────────────────────────────────────────────────────────

/// 创建当前平台的辅助功能权限检测器
pub fn create_platform_accessibility_checker() -> Box<dyn AccessibilityChecker> {
    #[cfg(target_os = "macos")]
    return Box::new(MacOSAccessibilityChecker::new());

    #[cfg(target_os = "windows")]
    return Box::new(WindowsAccessibilityChecker::new());

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Box::new(MockAccessibilityChecker::new(PermissionStatus::NotApplicable));
}

// ── macOS 平台内部实现 ──────────────────────────────────────────────────────

/// macOS 辅助功能权限检测（调用真实系统 API）
///
/// 使用 `AXIsProcessTrustedWithOptions(NULL, false)` 检测权限状态。
/// 传入 `false` 表示仅检测，不弹出系统授权对话框。
#[cfg(target_os = "macos")]
fn check_accessibility_macos() -> PermissionStatus {
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    use core_foundation::boolean::CFBoolean;

    // AXIsProcessTrustedWithOptions 的 kAXTrustedCheckOptionPrompt 键
    // 传入 false 表示仅检测，不弹出系统授权对话框
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::false_value();
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

    let trusted = unsafe {
        ax_is_process_trusted_with_options(options.as_concrete_TypeRef())
    };

    if trusted {
        PermissionStatus::Granted
    } else {
        PermissionStatus::Denied
    }
}

/// macOS FFI：AXIsProcessTrustedWithOptions
#[cfg(target_os = "macos")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
}

/// 包装 FFI 调用（便于测试 mock）
#[cfg(target_os = "macos")]
unsafe fn ax_is_process_trusted_with_options(
    options: core_foundation::dictionary::CFDictionaryRef,
) -> bool {
    AXIsProcessTrustedWithOptions(options)
}

/// 打开 macOS 系统偏好设置 → 辅助功能页面
#[cfg(target_os = "macos")]
fn open_accessibility_settings_macos() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map_err(|e| format!("打开系统偏好设置失败: {}", e))?;
    Ok(())
}

// ── 公共接口函数（向后兼容）────────────────────────────────────────────────

/// 检测辅助功能权限状态（公共接口，内部使用平台实现）
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

/// 打开辅助功能设置页面（公共接口）
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        open_accessibility_settings_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
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

    // ── MockAccessibilityChecker 测试（跨平台，任意平台均可运行）────────

    #[test]
    fn test_mock_checker_granted() {
        let checker = MockAccessibilityChecker::new(PermissionStatus::Granted);
        assert_eq!(checker.check_permission(), PermissionStatus::Granted);
    }

    #[test]
    fn test_mock_checker_denied() {
        let checker = MockAccessibilityChecker::new(PermissionStatus::Denied);
        assert_eq!(checker.check_permission(), PermissionStatus::Denied);
    }

    #[test]
    fn test_mock_checker_set_status() {
        let checker = MockAccessibilityChecker::new(PermissionStatus::Denied);
        checker.set_status(PermissionStatus::Granted);
        assert_eq!(checker.check_permission(), PermissionStatus::Granted);
    }

    #[test]
    fn test_mock_checker_open_settings_tracked() {
        let checker = MockAccessibilityChecker::new(PermissionStatus::Denied);
        assert!(!checker.was_open_settings_called());
        checker.open_settings().unwrap();
        assert!(checker.was_open_settings_called());
    }

    #[test]
    fn test_mock_checker_open_settings_returns_ok() {
        let checker = MockAccessibilityChecker::new(PermissionStatus::Denied);
        assert!(checker.open_settings().is_ok());
    }

    // ── 通过 trait 接口调用（验证平台隔离）──────────────────────────────

    #[test]
    fn test_trait_interface_check_permission() {
        // 通过 trait 接口调用，不依赖具体平台实现
        let checker: Box<dyn AccessibilityChecker> =
            Box::new(MockAccessibilityChecker::new(PermissionStatus::Granted));
        assert_eq!(checker.check_permission(), PermissionStatus::Granted);
    }

    #[test]
    fn test_trait_interface_open_settings() {
        let checker: Box<dyn AccessibilityChecker> =
            Box::new(MockAccessibilityChecker::new(PermissionStatus::Denied));
        assert!(checker.open_settings().is_ok());
    }

    // ── macOS 平台特定测试 ───────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn test_check_accessibility_macos_returns_valid_status() {
        // macOS 上应返回 Granted 或 Denied（不应返回 NotApplicable）
        let status = check_accessibility_permission();
        assert!(
            matches!(status, PermissionStatus::Granted | PermissionStatus::Denied),
            "macOS 上应返回 Granted 或 Denied，实际: {:?}",
            status
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_checker_returns_valid_status() {
        let checker = MacOSAccessibilityChecker::new();
        let status = checker.check_permission();
        assert!(
            matches!(status, PermissionStatus::Granted | PermissionStatus::Denied),
            "MacOSAccessibilityChecker 应返回 Granted 或 Denied，实际: {:?}",
            status
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_open_accessibility_settings_does_not_panic() {
        // 验证打开设置不会 panic（实际是否打开取决于系统）
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
    fn test_windows_checker_always_granted() {
        let checker = WindowsAccessibilityChecker::new();
        assert_eq!(checker.check_permission(), PermissionStatus::Granted,
            "WindowsAccessibilityChecker 应始终返回 Granted");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_open_accessibility_settings_windows_noop() {
        let checker = WindowsAccessibilityChecker::new();
        assert!(checker.open_settings().is_ok());
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

    // ── 平台隔离验证测试 ─────────────────────────────────────────────────

    /// 验证 macOS 模块不包含 Windows 特定符号
    /// 此测试通过编译时 cfg 确保隔离
    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_module_isolation() {
        // 如果此测试能编译通过，说明 macOS 模块中没有 Windows 特定代码
        // MacOSAccessibilityChecker 存在
        let _checker = MacOSAccessibilityChecker::new();
        // WindowsAccessibilityChecker 不应在 macOS 上存在（编译时隔离）
        // 以下代码在 macOS 上不会编译：
        // let _win_checker = WindowsAccessibilityChecker::new(); // 编译错误
    }

    /// 验证 Windows 模块不包含 macOS 特定符号
    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_module_isolation() {
        // WindowsAccessibilityChecker 存在
        let _checker = WindowsAccessibilityChecker::new();
        // MacOSAccessibilityChecker 不应在 Windows 上存在（编译时隔离）
        // 以下代码在 Windows 上不会编译：
        // let _mac_checker = MacOSAccessibilityChecker::new(); // 编译错误
    }
}
