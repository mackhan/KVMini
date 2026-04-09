//! Windows 键鼠事件注入实现
//!
//! 使用 SendInput 注入键鼠事件。

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputExecutor, MouseButton};

/// Windows 输入注入器
pub struct WinInputExecutor;

impl WinInputExecutor {
    pub fn new() -> Self { Self }
}

impl InputExecutor for WinInputExecutor {
    fn move_mouse(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::{SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE};
            use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
            unsafe {
                let screen_w = GetSystemMetrics(SM_CXSCREEN);
                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                let abs_x = (x as i64 * 65535 / screen_w as i64) as i32;
                let abs_y = (y as i64 * 65535 / screen_h as i64) as i32;
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_MOUSE;
                let mi = input.u.mi_mut();
                mi.dx = abs_x;
                mi.dy = abs_y;
                mi.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }
        Ok(())
    }

    fn mouse_down(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_MOUSE;
                let mi = input.u.mi_mut();
                mi.dwFlags = match button {
                    MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
                    MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
                    MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
                    _ => MOUSEEVENTF_LEFTDOWN,
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }
        Ok(())
    }

    fn mouse_up(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_MOUSE;
                let mi = input.u.mi_mut();
                mi.dwFlags = match button {
                    MouseButton::Left => MOUSEEVENTF_LEFTUP,
                    MouseButton::Right => MOUSEEVENTF_RIGHTUP,
                    MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
                    _ => MOUSEEVENTF_LEFTUP,
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }
        Ok(())
    }

    fn key_down(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_KEYBOARD;
                let ki = input.u.ki_mut();
                ki.wVk = native_keycode;
                ki.dwFlags = 0;
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }
        Ok(())
    }

    fn key_up(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_KEYBOARD;
                let ki = input.u.ki_mut();
                ki.wVk = native_keycode;
                ki.dwFlags = KEYEVENTF_KEYUP;
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }
        Ok(())
    }

    fn scroll(&self, dx: i32, dy: i32) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                if dy != 0 {
                    let mut input: INPUT = std::mem::zeroed();
                    input.type_ = INPUT_MOUSE;
                    let mi = input.u.mi_mut();
                    mi.mouseData = dy as u32;
                    mi.dwFlags = MOUSEEVENTF_WHEEL;
                    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                }
                if dx != 0 {
                    let mut input: INPUT = std::mem::zeroed();
                    input.type_ = INPUT_MOUSE;
                    let mi = input.u.mi_mut();
                    mi.mouseData = dx as u32;
                    mi.dwFlags = MOUSEEVENTF_HWHEEL;
                    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                }
            }
        }
        Ok(())
    }
}
