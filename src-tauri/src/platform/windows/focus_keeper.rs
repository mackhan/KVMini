//! Windows 焦点保持器（空实现）
//!
//! Windows 不存在 macOS 的"失焦后 CGAssociate 被重置"问题。
//! ClipCursor 在任何焦点状态下都保持有效，WH_MOUSE_LL 也不受焦点影响。
//! 因此 Windows 端给空实现。

use kvm_core::error::KvmResult;
use kvm_core::platform::FocusKeeper;

pub struct WinFocusKeeper;

impl WinFocusKeeper {
    pub fn new() -> Self {
        Self
    }
}

impl FocusKeeper for WinFocusKeeper {
    fn start(&self) -> KvmResult<()> {
        log::trace!("[FocusKeeper] Windows 空实现，无需焦点保持");
        Ok(())
    }

    fn stop(&self) -> KvmResult<()> {
        Ok(())
    }

    fn is_active(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win_focus_keeper_noop() {
        let fk = WinFocusKeeper::new();
        assert!(fk.start().is_ok());
        assert!(!fk.is_active());
        assert!(fk.stop().is_ok());
    }
}
