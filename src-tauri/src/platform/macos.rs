//! macOS 平台适配层
//!
//! 实现 kvm-core 定义的所有平台 trait。
//! 所有 CoreGraphics/NSEvent API 调用通过 #[cfg(target_os = "macos")] 隔离。

pub mod screen;
pub mod input_capture;
pub mod input_inject;
pub mod input_block;
pub mod hotkey;
pub mod clipboard;
pub mod cursor;
pub mod permission;
pub mod focus_keeper;

use kvm_core::platform::*;

/// macOS 平台适配器
pub struct MacPlatformAdapter {
    pub screen: screen::MacScreenProvider,
    pub input_capture: input_capture::MacInputCapture,
    pub input_inject: input_inject::MacInputExecutor,
    pub input_block: input_block::MacInputBlocker,
    pub hotkey: hotkey::MacHotkeyManager,
    pub clipboard: clipboard::MacClipboardProvider,
    pub cursor: cursor::MacCursorController,
    pub focus_keeper: focus_keeper::MacFocusKeeper,
    pub permission_prov: MacPermissionProvider,
}

impl MacPlatformAdapter {
    pub fn new() -> Self {
        Self {
            screen: screen::MacScreenProvider::new(),
            input_capture: input_capture::MacInputCapture::new(),
            input_inject: input_inject::MacInputExecutor::new(),
            input_block: input_block::MacInputBlocker::new(),
            hotkey: hotkey::MacHotkeyManager::new(),
            clipboard: clipboard::MacClipboardProvider::new(),
            cursor: cursor::MacCursorController::new(),
            focus_keeper: focus_keeper::MacFocusKeeper::new(),
            permission_prov: MacPermissionProvider,
        }
    }
}

impl PlatformAdapter for MacPlatformAdapter {
    fn screen_provider(&self) -> &dyn ScreenProvider { &self.screen }
    fn input_capture(&self) -> &dyn InputCapture { &self.input_capture }
    fn input_executor(&self) -> &dyn InputExecutor { &self.input_inject }
    fn input_blocker(&self) -> &dyn InputBlocker { &self.input_block }
    fn hotkey_manager(&self) -> &dyn HotkeyManager { &self.hotkey }
    fn clipboard_provider(&self) -> &dyn ClipboardProvider { &self.clipboard }
    fn cursor_controller(&self) -> &dyn CursorController { &self.cursor }
    fn focus_keeper(&self) -> &dyn FocusKeeper { &self.focus_keeper }
    fn permission_provider(&self) -> &dyn PermissionProvider { &self.permission_prov }
}

/// macOS 权限提供者（包装现有 permission.rs 的功能）
pub struct MacPermissionProvider;

impl PermissionProvider for MacPermissionProvider {
    fn is_elevated(&self) -> bool {
        true // macOS 不需要 UAC 提权
    }

    fn request_elevation(&self) -> kvm_core::error::KvmResult<()> {
        Ok(()) // macOS 不需要提权
    }

    fn permission_status(&self) -> PermissionStatus {
        use permission::AccessibilityStatus;
        PermissionStatus {
            elevated: true,
            accessibility: permission::check_accessibility() == AccessibilityStatus::Granted,
            input_monitoring: permission::check_input_monitoring() == AccessibilityStatus::Granted,
            admin_required: false,
        }
    }
}
