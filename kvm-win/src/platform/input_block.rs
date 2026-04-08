//! Windows 键鼠输入屏蔽实现
//!
//! 通过 BlockInput Win32 API 屏蔽/恢复本地键鼠。
//! 注意：BlockInput 需要管理员权限或 UIPI 豁免。

use kvm_core::error::KvmResult;
use kvm_core::platform::InputBlocker;
use std::sync::atomic::{AtomicBool, Ordering};

/// Windows 输入屏蔽器
pub struct WinInputBlocker {
    blocked: AtomicBool,
}

impl WinInputBlocker {
    pub fn new() -> Self {
        Self {
            blocked: AtomicBool::new(false),
        }
    }
}

impl InputBlocker for WinInputBlocker {
    fn block_input(&self) -> KvmResult<()> {
        self.blocked.store(true, Ordering::SeqCst);
        log::info!("[Win] 本地键鼠已屏蔽");

        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::BlockInput;
            unsafe {
                BlockInput(1); // TRUE
            }
        }

        Ok(())
    }

    fn unblock_input(&self) -> KvmResult<()> {
        self.blocked.store(false, Ordering::SeqCst);
        log::info!("[Win] 本地键鼠已恢复");

        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::BlockInput;
            unsafe {
                BlockInput(0); // FALSE
            }
        }

        Ok(())
    }

    fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }
}

impl Drop for WinInputBlocker {
    fn drop(&mut self) {
        // 确保析构时恢复键鼠输入
        if self.blocked.load(Ordering::SeqCst) {
            let _ = self.unblock_input();
        }
    }
}

impl InputBlocker for WinInputBlocker {
    fn block_input(&self) -> KvmResult<()> {
        self.blocked.store(true, Ordering::SeqCst);
        log::info!("[Win] 本地键鼠已屏蔽");
        // TODO: 实现 BlockInput(TRUE) 或 Hook 拦截
        Ok(())
    }

    fn unblock_input(&self) -> KvmResult<()> {
        self.blocked.store(false, Ordering::SeqCst);
        log::info!("[Win] 本地键鼠已恢复");
        // TODO: 实现 BlockInput(FALSE) 或移除 Hook
        Ok(())
    }

    fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }
}
