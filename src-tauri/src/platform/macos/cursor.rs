//! macOS 光标控制实现（NSCursor + CGWarpMouseCursorPosition + CGAssociateMouseAndMouseCursorPosition）
//!
//! v0.1.42 修复：
//! `warp_cursor` 在旧实现里无条件调 `CGAssociateMouseAndMouseCursorPosition(1)`，
//! 这会立刻把 grab 解除（鼠标重新驱动光标）。当远端发来 MouseMoveAbsolute 或
//! 边缘回切路径调用 `set_position` 时，grab 会被悄悄破坏。
//!
//! v0.1.42 方案：MacCursorController 维护 `is_grabbed: AtomicBool`，
//! `warp_cursor` 在 grab 中**只 warp 不 associate**，非 grab 时保持原有行为。
//!
//! v0.1.43 修复（Bug #034 二修）：
//! 非 grab 分支的 `CGAssociate(1)` 本身就是主根因——被控端 RemoteControlling 下
//! 从不调 grab()，set_position 每次都走到 `!grabbed` 分支，warp 刚落点又立刻被
//! `CGAssociate(1)` 把光标拉回物理鼠标位置，表现为远端虚拟光标完全不动。
//! 修复：彻底删除非 grab 分支的 `CGAssociate(1)`，让 warp 成为纯粹的"落点"原语。
//! LocalControlling 的 `Associate(1)` 恢复仍在 `release()` 中显式处理，不影响主流程。

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

    /// v0.1.43 Bug #034：被控端入 RemoteControlling 时调用。
    /// - 断开物理鼠标驱动光标（`CGAssociate(0)`）
    /// - **保持光标可见**（不 hide），以便远端 warp 的"虚拟光标"对被控用户可见
    fn decouple_physical_mouse(&self) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
            unsafe {
                CGAssociateMouseAndMouseCursorPosition(0);
                // 兜底 unhide：部分场景下光标可能已被其他组件隐藏
                use objc::{class, msg_send, sel, sel_impl};
                let _: () = msg_send![class!(NSCursor), unhide];
            }
            log::info!(
                "[Cursor] macOS decouple_physical_mouse: 断开鼠标↔光标绑定 + 保持光标可见（远端虚拟光标模式）"
            );
        }
        Ok(())
    }

    fn recouple_physical_mouse(&self) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
            unsafe {
                CGAssociateMouseAndMouseCursorPosition(1);
            }
            log::info!("[Cursor] macOS recouple_physical_mouse: 恢复鼠标↔光标绑定");
        }
        Ok(())
    }
}

/// BUG-039 Mac：进程启动时无条件恢复物理鼠标↔光标绑定。
///
/// 进程被强杀时 Drop 不执行，CGAssociateMouseAndMouseCursorPosition(0) 残留，
/// 导致重启后鼠标移动和光标位置不同步。每次启动调用一次确保状态干净。
#[cfg(target_os = "macos")]
pub fn startup_cleanup() {
    use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
    unsafe { CGAssociateMouseAndMouseCursorPosition(1); }
    log::info!("[Cursor] startup_cleanup: CGAssociate(1) 已执行（清除可能残留的解绑状态）");
}

/// Non-macOS placeholder
#[cfg(not(target_os = "macos"))]
pub fn startup_cleanup() {}

/// warp 光标。纯粹的"落点"原语：只调 `CGWarpMouseCursorPosition`，
/// 不再根据 `grabbed` 调 `CGAssociate(1/0)`——
/// - `grab()`/`release()` 里已显式负责解绑/恢复绑定
/// - 非 grab 场景下额外的 Associate(1) 会立即把 warp 的光标拉回物理鼠标位置，
///   导致远端虚拟光标不动（Bug #034 根因）
#[cfg(target_os = "macos")]
fn warp_cursor(x: i32, y: i32, _grabbed: bool) -> KvmResult<()> {
    use core_graphics::display::{CGPoint, CGWarpMouseCursorPosition};
    let point = CGPoint::new(x as f64, y as f64);
    let result = unsafe { CGWarpMouseCursorPosition(point) };
    if result != 0 {
        return Err(platform_err!(
            "CGWarpMouseCursorPosition",
            format!("失败: code={}", result)
        ));
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

    /// Bug #034 真正根因回归测试（v0.1.43）：
    /// 被控端处于 RemoteControlling 时从不调 grab()（grab 只用于本机 LocalControlling 隐藏光标）。
    /// 旧实现在 `warp_cursor(grabbed=false)` 分支无条件 `CGAssociate(1)`，
    /// 会把刚 warp 的光标立即被物理鼠标位置覆盖，表现为虚拟光标完全不动。
    ///
    /// 修复：非 grab 分支取消 `CGAssociate(1)` 调用，warp 成为纯粹的"落点"原语。
    /// set_position 必须在 non-grab 模式下仍保持光标在目标坐标附近（不被物理鼠标瞬间拉回）。
    /// 这个测试不验证光标真实坐标（headless 环境下 CGWarpMouseCursorPosition 可能返回非 0），
    /// 而是验证 set_position 对于 `!is_grabbed` 不会 panic、不改变 grab 标志。
    #[test]
    fn test_set_position_in_non_grab_does_not_panic_or_mutate_state() {
        let c = MacCursorController::new();
        assert!(!c.is_grabbed());
        // 非 grab 路径下多次调用，不应 panic，不应翻转状态
        for (x, y) in [(100, 100), (500, 500), (1920, 1080)] {
            let _ = c.set_position(x, y);
        }
        assert!(
            !c.is_grabbed(),
            "非 grab 场景的 set_position 不允许把 is_grabbed 翻转为 true"
        );
    }
}
