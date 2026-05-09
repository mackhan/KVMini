//! macOS 键鼠事件注入实现（CGEvent）
//!
//! v0.1.43: 所有注入事件在 `post()` 前会在 `kCGEventSourceUserData` (field 42) 上写入
//! magic `KVMINI_INJECT_USER_DATA`，供 input_capture.rs 的 CGEventTap 回调过滤
//! "自我注入事件"，避免被 PreemptDetector 误识别为物理抢占。

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputExecutor, MouseButton};

#[cfg(target_os = "macos")]
use crate::platform::macos::input_capture::{KVMINI_INJECT_USER_DATA, K_CG_EVENT_SOURCE_USER_DATA};

/// 给一个即将 post 的 CGEvent 打上 KVMini 合成事件 magic tag。
/// 仅 macOS 可用；失败不 panic，静默忽略（最坏情况是 PreemptDetector 可能误触发一次，
/// 用户重新撞边即可恢复，无数据风险）。
#[cfg(target_os = "macos")]
#[inline]
fn tag_synthetic(event: &core_graphics::event::CGEvent) {
    event.set_integer_value_field(K_CG_EVENT_SOURCE_USER_DATA, KVMINI_INJECT_USER_DATA);
}

pub struct MacInputExecutor;

impl MacInputExecutor {
    pub fn new() -> Self { Self }
}

impl InputExecutor for MacInputExecutor {
    fn move_mouse(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_mouse_move(x, y); }
        #[cfg(not(target_os = "macos"))]
        { log::debug!("[Mac-stub] move_mouse({}, {})", x, y); Ok(()) }
    }

    fn mouse_down(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_mouse_button(button, true); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn mouse_up(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_mouse_button(button, false); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn key_down(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_key(native_keycode, true); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn key_up(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_key(native_keycode, false); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn scroll(&self, dx: i32, dy: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_scroll(dx, dy); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }
}

#[cfg(target_os = "macos")]
fn inject_mouse_move(x: i32, y: i32) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let point = CGPoint::new(x as f64, y as f64);
    let event = CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left)
        .map_err(|_| platform_err!("CGEvent::new_mouse_event", "创建鼠标移动事件失败"))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_mouse_button(button: MouseButton, pressed: bool) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let current_pos = CGEvent::new(source.clone())
        .map(|e| e.location())
        .unwrap_or(CGPoint::new(0.0, 0.0));
    let (event_type, cg_button) = match (button, pressed) {
        (MouseButton::Left, true)   => (CGEventType::LeftMouseDown,  CGMouseButton::Left),
        (MouseButton::Left, false)  => (CGEventType::LeftMouseUp,    CGMouseButton::Left),
        (MouseButton::Right, true)  => (CGEventType::RightMouseDown, CGMouseButton::Right),
        (MouseButton::Right, false) => (CGEventType::RightMouseUp,   CGMouseButton::Right),
        (_, true)                   => (CGEventType::OtherMouseDown, CGMouseButton::Center),
        (_, false)                  => (CGEventType::OtherMouseUp,   CGMouseButton::Center),
    };
    let event = CGEvent::new_mouse_event(source, event_type, current_pos, cg_button)
        .map_err(|_| platform_err!("CGEvent::new_mouse_event", "创建鼠标按键事件失败"))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_key(native_keycode: u16, pressed: bool) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let event = CGEvent::new_keyboard_event(source, native_keycode as CGKeyCode, pressed)
        .map_err(|_| platform_err!("CGEvent::new_keyboard_event",
            format!("创建键盘事件失败: keycode={}", native_keycode)))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_scroll(dx: i32, dy: i32) -> KvmResult<()> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // core-graphics 0.23 中 CGScrollEventUnit 是 u32 类型别名
    // Line = 1, Pixel = 2
    const SCROLL_UNIT_LINE: u32 = 1;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let event = CGEvent::new_scroll_event(source, SCROLL_UNIT_LINE, 2, dy, dx, 0)
        .map_err(|_| platform_err!("CGEvent::new_scroll_event", "创建滚轮事件失败"))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}
