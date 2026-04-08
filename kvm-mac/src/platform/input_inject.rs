//! macOS 键鼠事件注入实现（CGEvent）
//!
//! 使用 CoreGraphics CGEvent API 模拟鼠标移动、点击、键盘按下/释放和滚轮事件。

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputExecutor, MouseButton};

pub struct MacInputExecutor;

impl MacInputExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl InputExecutor for MacInputExecutor {
    fn move_mouse(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            inject_mouse_move(x, y)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] move_mouse({}, {})", x, y);
            Ok(())
        }
    }

    fn mouse_down(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            inject_mouse_button(button, true)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] mouse_down({:?})", button);
            Ok(())
        }
    }

    fn mouse_up(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            inject_mouse_button(button, false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] mouse_up({:?})", button);
            Ok(())
        }
    }

    fn key_down(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            inject_key(native_keycode, true)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] key_down({})", native_keycode);
            Ok(())
        }
    }

    fn key_up(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            inject_key(native_keycode, false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] key_up({})", native_keycode);
            Ok(())
        }
    }

    fn scroll(&self, dx: i32, dy: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            inject_scroll(dx, dy)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] scroll({}, {})", dx, dy);
            Ok(())
        }
    }
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn inject_mouse_move(x: i32, y: i32) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| kvm_core::error::KvmError::Platform("创建 CGEventSource 失败".into()))?;

    let point = CGPoint::new(x as f64, y as f64);
    let event = CGEvent::new_mouse_event(
        source,
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left,
    ).map_err(|_| kvm_core::error::KvmError::Platform("创建鼠标移动事件失败".into()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_mouse_button(button: MouseButton, pressed: bool) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| kvm_core::error::KvmError::Platform("创建 CGEventSource 失败".into()))?;

    // 获取当前鼠标位置
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
        .map_err(|_| kvm_core::error::KvmError::Platform("创建鼠标按键事件失败".into()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_key(native_keycode: u16, pressed: bool) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| kvm_core::error::KvmError::Platform("创建 CGEventSource 失败".into()))?;

    let event = CGEvent::new_keyboard_event(source, native_keycode as CGKeyCode, pressed)
        .map_err(|_| kvm_core::error::KvmError::Platform(
            format!("创建键盘事件失败: keycode={}", native_keycode)
        ))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_scroll(dx: i32, dy: i32) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGScrollEventUnit};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| kvm_core::error::KvmError::Platform("创建 CGEventSource 失败".into()))?;

    // CGEventCreateScrollWheelEvent: wheel_count=2 支持水平+垂直
    let event = CGEvent::new_scroll_event(
        source,
        CGScrollEventUnit::Line,
        2,
        dy,
        dx,
        0,
    ).map_err(|_| kvm_core::error::KvmError::Platform("创建滚轮事件失败".into()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}
