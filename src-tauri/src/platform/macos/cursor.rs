//! macOS 光标控制实现（NSCursor + CGWarpMouseCursorPosition + CGAssociateMouseAndMouseCursorPosition）
//!
//! v0.1.42 修复：
//! `warp_cursor` 在旧实现里无条件调 `CGAssociateMouseAndMouseCursorPosition(1)`，
//! 这会立刻把 grab 解除（鼠标重新驱动光标）。当远端发来 MouseMoveAbsolute 或
//! 边缘回切路径调用 `set_position` 时，grab 会被悄悄破坏。
//!
//! 修复方案：MacCursorController 维护 `is_grabbed: AtomicBool`，
//! `grab()` / `release()` 维护其状态，`warp_cursor` 在 grab 中**只 warp 不 associate**，
//! 非 grab 时保持原有行为。

use kvm_core::error::KvmResult;
use kvm_core::platform::CursorController;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct MacCursorController {
    /// grab 状态。true 表示当前处于"光标已 grab 隐藏"模式，
    /// 此时 warp_cursor 不应再调 CGAssociate(1) 解除绑定。
    is_grabbed: Arc<AtomicBool>,
}

impl MacCursorController {
    pub fn new() -> Self {
        Self {
            is_grabbed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 测试可见的 grab 状态查询
    #[cfg(test)]
    fn is_grabbed(&self) -> bool {
        self.is_grabbed.load(Ordering::SeqCst)
    }
}

impl Default for MacCursorController {
    fn default() -> Self {
        Self::new()
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
        {
            warp_cursor(x, y, self.is_grabbed.load(Ordering::SeqCst))?;
        }
        // 非 macOS: no-op（参数显式忽略以静默 unused warning）
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (x, y);
        }
        Ok(())
    }

    fn get_position(&self) -> KvmResult<(i32, i32)> {
        #[cfg(target_os = "macos")]
        {
            return get_cursor_position();
        }
        #[cfg(not(target_os = "macos"))]
        Ok((0, 0))
    }

    fn grab(&self) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
            unsafe {
                // 断开物理鼠标与屏幕光标的绑定——鼠标移动不再驱动光标
                CGAssociateMouseAndMouseCursorPosition(0);
                // 隐藏光标，给用户明确的视觉反馈
                use objc::{class, msg_send, sel, sel_impl};
                let _: () = msg_send![class!(NSCursor), hide];
            }
            log::info!("[Cursor] macOS grab: 已断开鼠标↔光标绑定 + 隐藏光标");
        }
        // 标记 grab 状态：set_position 中 warp_cursor 据此跳过 Associate(1)
        self.is_grabbed.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn release(&self) -> KvmResult<()> {
        // 先清状态，确保任何并发的 set_position 立即看到非 grab，恢复正常 warp 行为
        self.is_grabbed.store(false, Ordering::SeqCst);
        #[cfg(target_os = "macos")]
        {
            use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
            unsafe {
                // 恢复物理鼠标与屏幕光标的绑定
                CGAssociateMouseAndMouseCursorPosition(1);
                // 显示光标
                use objc::{class, msg_send, sel, sel_impl};
                let _: () = msg_send![class!(NSCursor), unhide];
            }
            log::info!("[Cursor] macOS release: 已恢复鼠标↔光标绑定 + 显示光标");
        }
        Ok(())
    }
}

/// warp 光标。`grabbed=true` 时**仅**调用 CGWarpMouseCursorPosition，不再调 Associate(1)，
/// 否则会破坏当前 grab 状态。
#[cfg(target_os = "macos")]
fn warp_cursor(x: i32, y: i32, grabbed: bool) -> KvmResult<()> {
    use core_graphics::display::{
        CGAssociateMouseAndMouseCursorPosition, CGPoint, CGWarpMouseCursorPosition,
    };
    let point = CGPoint::new(x as f64, y as f64);
    let result = unsafe { CGWarpMouseCursorPosition(point) };
    if result != 0 {
        return Err(platform_err!(
            "CGWarpMouseCursorPosition",
            format!("失败: code={}", result)
        ));
    }
    if !grabbed {
        // 非 grab 状态下保持原有行为：warp 后立即重新绑定，避免某些场景下的轻微漂移
        unsafe {
            CGAssociateMouseAndMouseCursorPosition(1);
        }
    } else {
        log::debug!("[Cursor] warp 在 grab 状态，跳过 CGAssociate(1) 以保持 grab 有效");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn get_cursor_position() -> KvmResult<(i32, i32)> {
    use objc::{class, msg_send, sel, sel_impl};
    #[repr(C)]
    struct NSPoint {
        x: f64,
        y: f64,
    }
    let point: NSPoint = unsafe { msg_send![class!(NSEvent), mouseLocation] };
    let screen_height = {
        use core_graphics::display::CGDisplay;
        let main = CGDisplay::main();
        main.bounds().size.height as i32
    };
    let y_flipped = screen_height - point.y as i32;
    Ok((point.x as i32, y_flipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_is_not_grabbed() {
        let c = MacCursorController::new();
        assert!(!c.is_grabbed());
    }

    #[test]
    fn test_grab_sets_flag_true() {
        // grab() 在测试环境下也会调系统 API，但 macOS 单测能跑（unsafe FFI 不会 panic）
        // 这里只关心状态变化，不检验 system call 副作用
        let c = MacCursorController::new();
        let _ = c.grab();
        assert!(c.is_grabbed(), "grab() 必须把 is_grabbed 设为 true");
    }

    #[test]
    fn test_release_clears_flag() {
        let c = MacCursorController::new();
        let _ = c.grab();
        assert!(c.is_grabbed());
        let _ = c.release();
        assert!(!c.is_grabbed(), "release() 必须把 is_grabbed 设回 false");
    }

    #[test]
    fn test_grab_release_grab_cycle() {
        let c = MacCursorController::new();
        let _ = c.grab();
        let _ = c.release();
        let _ = c.grab();
        assert!(c.is_grabbed(), "再次 grab 后必须为 true");
        let _ = c.release();
        assert!(!c.is_grabbed());
    }

    /// 关键回归测试：
    /// 验证 `is_grabbed` 这一布尔状态会被 set_position 的 warp 路径正确读取并跳过 Associate(1)。
    /// 这里我们无法直接 mock `CGAssociateMouseAndMouseCursorPosition`，
    /// 但可以保证状态切换不会在 set_position 内被反向修改。
    #[test]
    fn test_set_position_does_not_clear_grab_flag() {
        let c = MacCursorController::new();
        let _ = c.grab();
        assert!(c.is_grabbed());
        // set_position 在 macOS 真实环境下会调 CGWarpMouseCursorPosition，
        // 在某些 CI/headless 环境下可能返回非 0；但无论返回值如何，
        // 都不应改变 is_grabbed 标志。
        let _ = c.set_position(100, 100);
        assert!(
            c.is_grabbed(),
            "set_position 不允许改变 is_grabbed 状态（v0.1.42 BUG-034 回归保护）"
        );
    }
}
