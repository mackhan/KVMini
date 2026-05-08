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

    fn grab(&self) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::{ClipCursor, GetCursorPos, ShowCursor};
            use winapi::shared::windef::{POINT, RECT};
            unsafe {
                // 获取当前光标位置，将光标锁定在 1x1 像素区域（不可移动）
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                let rect = RECT { left: pt.x, top: pt.y, right: pt.x + 1, bottom: pt.y + 1 };
                ClipCursor(&rect);
                ShowCursor(0);
            }
            log::info!("[Cursor] Windows grab: ClipCursor 锁定 + 隐藏光标");
        }
        Ok(())
    }

    fn release(&self) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::{ClipCursor, ShowCursor};
            use std::ptr;
            unsafe {
                // 解除光标区域限制
                ClipCursor(ptr::null());
                ShowCursor(1);
            }
            log::info!("[Cursor] Windows release: ClipCursor 解除 + 显示光标");
        }
        Ok(())
    }
}
