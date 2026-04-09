//! macOS 光标控制实现（NSCursor + CGWarpMouseCursorPosition）

use kvm_core::error::KvmResult;
use kvm_core::platform::CursorController;

pub struct MacCursorController;

impl MacCursorController {
    pub fn new() -> Self { Self }
}

impl CursorController for MacCursorController {
    fn hide_cursor(&self) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                use objc::{class, msg_send, sel, sel_impl};
                let _: () = msg_send![class!(NSCursor), hide];
            }
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
        }
        Ok(())
    }

    fn set_position(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { warp_cursor(x, y)?; }
        Ok(())
    }

    fn get_position(&self) -> KvmResult<(i32, i32)> {
        #[cfg(target_os = "macos")]
        { return get_cursor_position(); }
        #[cfg(not(target_os = "macos"))]
        Ok((0, 0))
    }
}

#[cfg(target_os = "macos")]
fn warp_cursor(x: i32, y: i32) -> KvmResult<()> {
    use core_graphics::display::{CGPoint, CGWarpMouseCursorPosition, CGAssociateMouseAndMouseCursorPosition};
    let point = CGPoint::new(x as f64, y as f64);
    let result = unsafe { CGWarpMouseCursorPosition(point) };
    if result != 0 {
        return Err(platform_err!("CGWarpMouseCursorPosition",
            format!("失败: code={}", result)
        ));
    }
    unsafe { CGAssociateMouseAndMouseCursorPosition(1); }
    Ok(())
}

#[cfg(target_os = "macos")]
fn get_cursor_position() -> KvmResult<(i32, i32)> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    #[repr(C)]
    struct NSPoint { x: f64, y: f64 }
    let point: NSPoint = unsafe { msg_send![class!(NSEvent), mouseLocation] };
    let screen_height = {
        use core_graphics::display::CGDisplay;
        let main = CGDisplay::main();
        main.bounds().size.height as i32
    };
    let y_flipped = screen_height - point.y as i32;
    Ok((point.x as i32, y_flipped))
}
