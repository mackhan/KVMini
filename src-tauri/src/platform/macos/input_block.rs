//! macOS 键鼠输入屏蔽实现（CGEventTap）

use kvm_core::error::KvmResult;
use kvm_core::platform::InputBlocker;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct MacInputBlocker {
    blocked: Arc<AtomicBool>,
    _tap_handle: Mutex<Option<BlockTapHandle>>,
}

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
        if self.blocked.load(Ordering::SeqCst) { return Ok(()); }
        self.blocked.store(true, Ordering::SeqCst);
        log::info!("[Mac] 本地键鼠已屏蔽");
        #[cfg(target_os = "macos")]
        {
            let blocked = Arc::clone(&self.blocked);
            match create_block_tap(blocked) {
                Ok(handle) => { *self._tap_handle.lock().unwrap() = Some(handle); }
                Err(e) => { log::warn!("[Mac] 创建屏蔽 EventTap 失败: {}，降级为软屏蔽", e); }
            }
        }
        Ok(())
    }

    fn unblock_input(&self) -> KvmResult<()> {
        self.blocked.store(false, Ordering::SeqCst);
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
        CGEventType::LeftMouseDown, CGEventType::LeftMouseUp,
        CGEventType::RightMouseDown, CGEventType::RightMouseUp,
        CGEventType::OtherMouseDown, CGEventType::OtherMouseUp,
        CGEventType::ScrollWheel,
        CGEventType::KeyDown, CGEventType::KeyUp,
        CGEventType::FlagsChanged,
    ]);

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        mask,
        move |_proxy, _event_type, event| {
            if blocked.load(Ordering::SeqCst) { None } else { Some(event.clone()) }
        },
    ).map_err(|_| platform_err!("CGEventTap::new", "创建屏蔽 CGEventTap 失败，请检查辅助功能权限"))?;

    let loop_source = tap.mach_port.create_runloop_source(0)
        .map_err(|_| platform_err!("create_runloop_source", "创建 RunLoop Source 失败"))?;

    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&loop_source, unsafe { kCFRunLoopDefaultMode });
    tap.enable();

    Ok(BlockTapHandle { _tap: tap })
}
