//! macOS 键鼠输入屏蔽实现
//!
//! v0.1.52: 简化设计——屏蔽不需要额外的 CGEventTap，
//! 已由 input_capture.rs 中 CONTROL_STATE_REMOTE_CONTROLLING 分支的 `return None` 实现。
//! MacInputBlocker 仅做日志标记，确保 blocker.is_blocked() 状态语义正确。
//! 旧的 create_block_tap 在 get_current() RunLoop 上注册 source 但不运行该 RunLoop，
//! 永远无效，已删除。

use kvm_core::error::KvmResult;
use kvm_core::platform::InputBlocker;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct MacInputBlocker {
    blocked: Arc<AtomicBool>,
}

impl MacInputBlocker {
    pub fn new() -> Self {
        Self {
            blocked: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for MacInputBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBlocker for MacInputBlocker {
    fn block_input(&self) -> KvmResult<()> {
        if self.blocked.load(Ordering::SeqCst) { return Ok(()); }
        self.blocked.store(true, Ordering::SeqCst);
        // 实际屏蔽由 input_capture.rs 的 control_state == REMOTE_CONTROLLING 分支完成
        // （CGEventTap 回调返回 None 吞掉事件）
        log::info!("[Mac] 本地键鼠已屏蔽（由 CGEventTap control_state 保证）");
        Ok(())
    }

    fn unblock_input(&self) -> KvmResult<()> {
        self.blocked.store(false, Ordering::SeqCst);
        log::info!("[Mac] 本地键鼠已恢复");
        Ok(())
    }

    fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }
}
