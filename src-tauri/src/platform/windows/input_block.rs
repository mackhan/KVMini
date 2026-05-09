//! Windows 键鼠输入屏蔽实现
//!
//! v0.1.41: 双层屏蔽架构
//! - 第一层：Hook 回调中 should_swallow() 按控制状态返回 1 吞掉事件（无需提权，始终有效）
//! - 第二层：BlockInput Win32 API（需要管理员权限，作为增强，失败不影响第一层）
//!
//! 原设计强依赖 BlockInput（需要 UAC 提权），非管理员运行时完全失效。
//! 新设计让 hook 层作为独立兜底——只要 hook 在跑，就能在回调中吞掉事件。
//!
//! v0.1.44 BUG-036: 增加 force_unblock() 方法，断开连接时强制确保键鼠恢复。

use kvm_core::error::KvmResult;
use kvm_core::platform::InputBlocker;
use std::sync::atomic::{AtomicBool, Ordering};

/// 进程启动时调用：无条件清除可能残留的 BlockInput 状态。
///
/// Win32 BlockInput(1) 是进程级 API，但不会随进程退出自动还原（TerminateProcess 不走 Drop）。
/// 因此每次启动时主动调用一次 BlockInput(0)，确保上次异常退出留下的屏蔽状态被清除。
#[cfg(target_os = "windows")]
pub fn startup_cleanup() {
    use winapi::um::winuser::BlockInput;
    unsafe { BlockInput(0); }
    log::info!("[Win] startup_cleanup: BlockInput(0) 已执行（清除可能残留的屏蔽状态）");
}

/// Non-Windows placeholder
#[cfg(not(target_os = "windows"))]
pub fn startup_cleanup() {}



/// Windows 输入屏蔽器
pub struct WinInputBlocker {
    blocked: AtomicBool,
}

impl WinInputBlocker {
    pub fn new() -> Self {
        Self { blocked: AtomicBool::new(false) }
    }

    /// BUG-036: 强制解除屏蔽（无论当前状态如何都执行 unblock）
    /// 用于断开连接兜底、进程退出前清理等场景
    pub fn force_unblock(&self) {
        self.blocked.store(false, Ordering::SeqCst);
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::BlockInput;
            unsafe { BlockInput(0); }
        }
        log::info!("[Win] force_unblock: 键鼠屏蔽已强制解除");
    }
}

impl InputBlocker for WinInputBlocker {
    fn block_input(&self) -> KvmResult<()> {
        self.blocked.store(true, Ordering::SeqCst);
        log::info!("[Win] 本地键鼠已屏蔽（hook 层吞事件生效）");
        #[cfg(target_os = "windows")]
        {
            // 尝试调用 BlockInput 作为增强层（需要管理员权限）
            // 失败不影响 hook 层的独立屏蔽能力
            use winapi::um::winuser::BlockInput;
            let result = unsafe { BlockInput(1) };
            if result == 0 {
                log::debug!("[Win] BlockInput(true) 返回 0（可能缺少管理员权限，hook 软屏蔽仍有效）");
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
            unsafe { BlockInput(0); }
        }
        Ok(())
    }

    fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }
}

impl Drop for WinInputBlocker {
    fn drop(&mut self) {
        if self.blocked.load(Ordering::SeqCst) {
            let _ = self.unblock_input();
        }
    }
}
