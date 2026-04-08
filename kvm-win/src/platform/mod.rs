//! Windows 平台适配层
//!
//! 实现 kvm-core 定义的所有平台 trait。

pub mod screen;
pub mod input_capture;
pub mod input_inject;
pub mod input_block;
pub mod hotkey;
pub mod clipboard;
pub mod cursor;

use kvm_core::error::KvmResult;
use kvm_core::platform::*;

/// Windows 平台适配器
pub struct WinPlatformAdapter {
    pub screen: screen::WinScreenProvider,
    pub input_capture: input_capture::WinInputCapture,
    pub input_inject: input_inject::WinInputExecutor,
    pub input_block: input_block::WinInputBlocker,
    pub hotkey: hotkey::WinHotkeyManager,
    pub clipboard: clipboard::WinClipboardProvider,
    pub cursor: cursor::WinCursorController,
}

impl WinPlatformAdapter {
    pub fn new() -> Self {
        Self {
            screen: screen::WinScreenProvider::new(),
            input_capture: input_capture::WinInputCapture::new(),
            input_inject: input_inject::WinInputExecutor::new(),
            input_block: input_block::WinInputBlocker::new(),
            hotkey: hotkey::WinHotkeyManager::new(),
            clipboard: clipboard::WinClipboardProvider::new(),
            cursor: cursor::WinCursorController::new(),
        }
    }
}

impl PlatformAdapter for WinPlatformAdapter {
    fn screen_provider(&self) -> &dyn ScreenProvider { &self.screen }
    fn input_capture(&self) -> &dyn InputCapture { &self.input_capture }
    fn input_executor(&self) -> &dyn InputExecutor { &self.input_inject }
    fn input_blocker(&self) -> &dyn InputBlocker { &self.input_block }
    fn hotkey_manager(&self) -> &dyn HotkeyManager { &self.hotkey }
    fn clipboard_provider(&self) -> &dyn ClipboardProvider { &self.clipboard }
    fn cursor_controller(&self) -> &dyn CursorController { &self.cursor }
}
