//! macOS 键鼠事件采集实现（CGEventTap）

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputCapture, RawInputEvent};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

pub struct MacInputCapture {
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
}

impl MacInputCapture {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
            callback: Arc::new(Mutex::new(None)),
        }
    }
}

impl InputCapture for MacInputCapture {
    fn start_capture(&self, callback: Box<dyn Fn(RawInputEvent) + Send + Sync>) -> KvmResult<()> {
        *self.callback.lock().unwrap() = Some(callback);
        self.capturing.store(true, Ordering::SeqCst);
        log::info!("[Mac] 键鼠采集已启动");
        #[cfg(target_os = "macos")]
        {
            let capturing = Arc::clone(&self.capturing);
            let cb = Arc::clone(&self.callback);
            start_event_tap(capturing, cb)?;
        }
        Ok(())
    }

    fn stop_capture(&self) -> KvmResult<()> {
        self.capturing.store(false, Ordering::SeqCst);
        *self.callback.lock().unwrap() = None;
        log::info!("[Mac] 键鼠采集已停止");
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.capturing.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "macos")]
fn start_event_tap(
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
) -> KvmResult<()> {
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation,
        CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    use core_foundation::runloop::{CFRunLoop, kCFRunLoopDefaultMode};

    let mask = vec![
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
    ];

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        mask,
        move |_proxy, event_type, event| {
            if !capturing.load(Ordering::SeqCst) { return None; }
            if let Some(raw_event) = cg_event_to_raw(event_type, event) {
                if let Ok(guard) = callback.lock() {
                    if let Some(cb) = guard.as_ref() { cb(raw_event); }
                }
            }
            None
        },
    ).map_err(|_| kvm_core::error::KvmError::PlatformError {
        api: "CGEventTap::new".to_string(),
        reason: "创建 CGEventTap 失败，请检查辅助功能权限".to_string(),
    })?;

    let loop_source = tap.mach_port.create_runloop_source(0)
        .map_err(|_| kvm_core::error::KvmError::PlatformError {
            api: "create_runloop_source".to_string(),
            reason: "创建 RunLoop Source 失败".to_string(),
        })?;

    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&loop_source, unsafe { kCFRunLoopDefaultMode });
    tap.enable();

    log::info!("[Mac] CGEventTap 已启动");
    Ok(())
}

#[cfg(target_os = "macos")]
fn cg_event_to_raw(
    event_type: core_graphics::event::CGEventType,
    event: &core_graphics::event::CGEvent,
) -> Option<RawInputEvent> {
    use core_graphics::event::CGEventType;
    use kvm_core::platform::MouseButton;

    match event_type {
        CGEventType::MouseMoved => {
            let loc = event.location();
            Some(RawInputEvent::MouseMove { x: loc.x as i32, y: loc.y as i32 })
        }
        CGEventType::LeftMouseDown  => Some(RawInputEvent::MouseDown { button: MouseButton::Left }),
        CGEventType::LeftMouseUp    => Some(RawInputEvent::MouseUp   { button: MouseButton::Left }),
        CGEventType::RightMouseDown => Some(RawInputEvent::MouseDown { button: MouseButton::Right }),
        CGEventType::RightMouseUp   => Some(RawInputEvent::MouseUp   { button: MouseButton::Right }),
        CGEventType::OtherMouseDown => Some(RawInputEvent::MouseDown { button: MouseButton::Middle }),
        CGEventType::OtherMouseUp   => Some(RawInputEvent::MouseUp   { button: MouseButton::Middle }),
        CGEventType::ScrollWheel => {
            // ScrollWheelEventPointDeltaAxis2 = 97, ScrollWheelEventPointDeltaAxis1 = 96
            let dx = event.get_integer_value_field(97) as i32;
            let dy = event.get_integer_value_field(96) as i32;
            Some(RawInputEvent::Scroll { dx, dy })
        }
        CGEventType::KeyDown => {
            // KeyboardEventKeycode = 9
            let keycode = event.get_integer_value_field(9) as u16;
            Some(RawInputEvent::KeyDown { native_keycode: keycode })
        }
        CGEventType::KeyUp => {
            // KeyboardEventKeycode = 9
            let keycode = event.get_integer_value_field(9) as u16;
            Some(RawInputEvent::KeyUp { native_keycode: keycode })
        }
        _ => None,
    }
}
