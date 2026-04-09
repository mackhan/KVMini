//! Windows 光标控制实现

use kvm_core::error::KvmResult;
use kvm_core::platform::CursorController;

pub struct WinCursorController;

impl WinCursorController {
    pub fn new() -> Self { Self }
}

impl CursorController for WinCursorController {
    fn hide_cursor(&self) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        { use winapi::um::winuser::ShowCursor; unsafe { ShowCursor(0); } }
        Ok(())
    }

    fn show_cursor(&self) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        { use winapi::um::winuser::ShowCursor; unsafe { ShowCursor(1); } }
        Ok(())
    }

    fn set_position(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        { use winapi::um::winuser::SetCursorPos; unsafe { SetCursorPos(x, y); } }
        Ok(())
    }

    fn get_position(&self) -> KvmResult<(i32, i32)> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::GetCursorPos;
            use winapi::shared::windef::POINT;
            unsafe {
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                return Ok((pt.x, pt.y));
            }
        }
        #[cfg(not(target_os = "windows"))]
        Ok((0, 0))
    }
}
