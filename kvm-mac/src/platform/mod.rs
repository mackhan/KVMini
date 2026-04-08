//! Mac 平台适配层
//!
//! 实现 kvm-core 定义的所有平台 trait。

pub mod screen;
pub mod input_capture;
pub mod input_inject;
pub mod input_block;
pub mod hotkey;
pub mod clipboard;
pub mod cursor;
pub mod permission;

use kvm_core::platform::*;

/// Mac 平台适配器
pub struct MacPlatformAdapter {
    pub screen: screen::MacScreenProvider,
    pub input_capture: input_capture::MacInputCapture,
    pub input_inject: input_inject::MacInputExecutor,
    pub input_block: input_block::MacInputBlocker,
    pub hotkey: hotkey::MacHotkeyManager,
    pub clipboard: clipboard::MacClipboardProvider,
    pub cursor: cursor::MacCursorController,
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
}
