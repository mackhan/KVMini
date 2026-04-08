//! macOS 键鼠输入屏蔽实现（CGEventTap）
//!
//! 使用 CGEventTap 的拦截模式（非 ListenOnly）屏蔽本地键鼠事件。
//! 屏蔽时所有键鼠事件被丢弃，恢复时移除 Tap。

use kvm_core::error::KvmResult;
use kvm_core::platform::InputBlocker;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct MacInputBlocker {
    blocked: Arc<AtomicBool>,
    /// 用于停止屏蔽的信号句柄（macOS 上存储 tap 引用）
    _tap_handle: Mutex<Option<BlockTapHandle>>,
}

/// 持有 EventTap 生命周期的句柄
struct BlockTapHandle {
    #[cfg(target_os = "macos")]
    _tap: core_graphics::event::CGEventTap,
}

impl MacInputBlocker {
    pub fn new() -> Self {
        Self {
            blocked: Arc::new(AtomicBool::new(false)),
            _tap_handle: Mutex::new(None),
        }
    }
}

impl InputBlocker for MacInputBlocker {
    fn block_input(&self) -> KvmResult<()> {
        if self.blocked.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.blocked.store(true, Ordering::SeqCst);
        log::info!("[Mac] 本地键鼠已屏蔽");

        #[cfg(target_os = "macos")]
        {
            let blocked = Arc::clone(&self.blocked);
            match create_block_tap(blocked) {
                Ok(handle) => {
                    *self._tap_handle.lock().unwrap() = Some(handle);
                }
                Err(e) => {
                    log::warn!("[Mac] 创建屏蔽 EventTap 失败: {}，降级为软屏蔽", e);
                }
            }
        }

        Ok(())
    }

    fn unblock_input(&self) -> KvmResult<()> {
        self.blocked.store(false, Ordering::SeqCst);
        // 释放 tap 句柄即可停止屏蔽
        *self._tap_handle.lock().unwrap() = None;
        log::info!("[Mac] 本地键鼠已恢复");
        Ok(())
    }

    fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "macos")]
fn create_block_tap(blocked: Arc<AtomicBool>) -> KvmResult<BlockTapHandle> {
    use core_graphics::event::{
        CGEventMask, CGEventTap, CGEventTapLocation,
        CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    use core_foundation::runloop::{CFRunLoop, kCFRunLoopDefaultMode};

    let mask = CGEventMask::new(vec![
        CGEventType::MouseMoved,
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseUp,
        CGEventType::RightMouseDown,
        CGEventType::RightMouseUp,
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
        CGEventType::ScrollWheel,
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::FlagsChanged,
    ]);

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default, // 拦截模式：可以丢弃事件
        mask,
        move |_proxy, _event_type, _event| {
            if blocked.load(Ordering::SeqCst) {
                // 返回 None 表示丢弃该事件
                None
            } else {
                // 已恢复，透传事件
                Some(_event.clone())
            }
        },
    ).map_err(|_| kvm_core::error::KvmError::Platform(
        "创建屏蔽 CGEventTap 失败，请检查辅助功能权限".into()
    ))?;

    let loop_source = tap.mach_port.create_runloop_source(0)
        .map_err(|_| kvm_core::error::KvmError::Platform("创建 RunLoop Source 失败".into()))?;

    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&loop_source, unsafe { kCFRunLoopDefaultMode });
    tap.enable();

    Ok(BlockTapHandle { _tap: tap })
}
