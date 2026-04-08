//! macOS 光标控制实现（NSCursor + CGWarpMouseCursorPosition）
//!
//! - 隐藏/显示光标：NSCursor hide / unhide
//! - 设置光标位置：CGWarpMouseCursorPosition
//! - 获取光标位置：NSEvent mouseLocation

use kvm_core::error::KvmResult;
use kvm_core::platform::CursorController;

pub struct MacCursorController;

impl MacCursorController {
    pub fn new() -> Self {
        Self
    }
}

impl CursorController for MacCursorController {
    fn hide_cursor(&self) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                use objc::{class, msg_send, sel, sel_impl};
                let _: () = msg_send![class!(NSCursor), hide];
            }
            log::debug!("[Mac] 光标已隐藏");
        }
        Ok(())
    }

    fn show_cursor(&self) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                use objc::{class, msg_send, sel, sel_impl};
                let _: () = msg_send![class!(NSCursor), unhide];
            }
            log::debug!("[Mac] 光标已显示");
        }
        Ok(())
    }

    fn set_position(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            warp_cursor(x, y)?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] set_position({}, {})", x, y);
        }
        Ok(())
    }

    fn get_position(&self) -> KvmResult<(i32, i32)> {
        #[cfg(target_os = "macos")]
        {
            get_cursor_position()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok((0, 0))
        }
    }
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn warp_cursor(x: i32, y: i32) -> KvmResult<()> {
    use core_graphics::display::CGPoint;
    use core_graphics::display::CGWarpMouseCursorPosition;

    let point = CGPoint::new(x as f64, y as f64);
    let result = unsafe { CGWarpMouseCursorPosition(point) };

    if result != 0 {
        return Err(kvm_core::error::KvmError::Platform(
            format!("CGWarpMouseCursorPosition 失败: code={}", result)
        ));
    }

    // 同步 HID 光标位置，避免下次鼠标移动时跳跃
    unsafe {
        use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
        CGAssociateMouseAndMouseCursorPosition(1);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn get_cursor_position() -> KvmResult<(i32, i32)> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    // NSPoint point = [NSEvent mouseLocation]
    // NSPoint 在 objc 中是 {f64, f64}
    #[repr(C)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    let point: NSPoint = unsafe { msg_send![class!(NSEvent), mouseLocation] };

    // macOS 坐标系：原点在左下角，需要转换为左上角原点
    // 获取主屏幕高度
    let screen_height = get_main_screen_height();
    let y_flipped = screen_height - point.y as i32;

    Ok((point.x as i32, y_flipped))
}

/// 获取主屏幕高度（用于坐标系转换）
#[cfg(target_os = "macos")]
fn get_main_screen_height() -> i32 {
    use core_graphics::display::CGDisplay;
    let main = CGDisplay::main();
    let bounds = main.bounds();
    bounds.size.height as i32
}
