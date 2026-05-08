//! Windows 键鼠事件注入实现
//!
//! 使用 SendInput 注入键鼠事件。
//! v0.1.41: 多屏使用 SM_CXVIRTUALSCREEN + MOUSEEVENTF_VIRTUALDESK
//!          XButton (Side1/Side2) 补全
//!          滚轮 mouseData 正确使用 i32 as u32 表达负向滚动

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputExecutor, MouseButton};

/// Windows 输入注入器
pub struct WinInputExecutor;

impl WinInputExecutor {
    pub fn new() -> Self { Self }
}

/// XBUTTON1 / XBUTTON2 常量
#[cfg(target_os = "windows")]
const XBUTTON1: u32 = 0x0001;
#[cfg(target_os = "windows")]
const XBUTTON2: u32 = 0x0002;

impl InputExecutor for WinInputExecutor {
    fn move_mouse(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                // v0.1.41: 使用 VirtualScreen 坐标系支持多屏
                let vscreen_w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let vscreen_h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                let vscreen_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let vscreen_y = GetSystemMetrics(SM_YVIRTUALSCREEN);

                // 将绝对坐标转换为 VirtualScreen 归一化坐标 [0, 65535]
                let rel_x = x - vscreen_x;
                let rel_y = y - vscreen_y;
                let abs_x = if vscreen_w > 0 { (rel_x as i64 * 65535 / vscreen_w as i64) as i32 } else { 0 };
                let abs_y = if vscreen_h > 0 { (rel_y as i64 * 65535 / vscreen_h as i64) as i32 } else { 0 };

                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_MOUSE;
                let mi = input.u.mi_mut();
                mi.dx = abs_x;
                mi.dy = abs_y;
                // MOUSEEVENTF_VIRTUALDESK: 坐标相对于整个虚拟桌面（多显示器）
                mi.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
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
                match button {
                    MouseButton::Left => { mi.dwFlags = MOUSEEVENTF_LEFTDOWN; }
                    MouseButton::Right => { mi.dwFlags = MOUSEEVENTF_RIGHTDOWN; }
                    MouseButton::Middle => { mi.dwFlags = MOUSEEVENTF_MIDDLEDOWN; }
                    // v0.1.41: XButton 支持
                    MouseButton::Side1 => {
                        mi.dwFlags = MOUSEEVENTF_XDOWN;
                        mi.mouseData = XBUTTON1;
                    }
                    MouseButton::Side2 => {
                        mi.dwFlags = MOUSEEVENTF_XDOWN;
                        mi.mouseData = XBUTTON2;
                    }
                }
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
                match button {
                    MouseButton::Left => { mi.dwFlags = MOUSEEVENTF_LEFTUP; }
                    MouseButton::Right => { mi.dwFlags = MOUSEEVENTF_RIGHTUP; }
                    MouseButton::Middle => { mi.dwFlags = MOUSEEVENTF_MIDDLEUP; }
                    // v0.1.41: XButton 支持
                    MouseButton::Side1 => {
                        mi.dwFlags = MOUSEEVENTF_XUP;
                        mi.mouseData = XBUTTON1;
                    }
                    MouseButton::Side2 => {
                        mi.dwFlags = MOUSEEVENTF_XUP;
                        mi.mouseData = XBUTTON2;
                    }
                }
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
                    // v0.1.41: 正确使用 i32 as u32 保留符号位（负值滚动方向）
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let _exec = WinInputExecutor::new();
    }
}
