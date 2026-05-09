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
                        mi.mouseData = XBUTTON1 as u32;
                    }
                    MouseButton::Side2 => {
                        mi.dwFlags = MOUSEEVENTF_XDOWN;
                        mi.mouseData = XBUTTON2 as u32;
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
                        mi.mouseData = XBUTTON1 as u32;
                    }
                    MouseButton::Side2 => {
                        mi.dwFlags = MOUSEEVENTF_XUP;
                        mi.mouseData = XBUTTON2 as u32;
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
                let vk = cg_keycode_to_vk(native_keycode);
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_KEYBOARD;
                let ki = input.u.ki_mut();
                ki.wVk = vk;
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
                let vk = cg_keycode_to_vk(native_keycode);
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_KEYBOARD;
                let ki = input.u.ki_mut();
                ki.wVk = vk;
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

/// macOS CGKeyCode → Windows Virtual Key Code 转换表
///
/// macOS 使用 CGKeyCode（USB HID 键位编号），Windows 使用 VK（虚拟键码）。
/// 两套体系完全不同，需要在 Win 被控端转换后才能正确注入。
/// 参考：https://eastmanreference.com/complete-list-of-applescript-key-codes
///       https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
pub(crate) fn cg_keycode_to_vk(cg: u16) -> u16 {
    match cg {
        // ── 字母键 ──────────────────────────────────────────
        0  => 0x41, // a → VK_A
        1  => 0x53, // s → VK_S
        2  => 0x44, // d → VK_D
        3  => 0x46, // f → VK_F
        4  => 0x48, // h → VK_H
        5  => 0x47, // g → VK_G
        6  => 0x5A, // z → VK_Z
        7  => 0x58, // x → VK_X
        8  => 0x43, // c → VK_C
        9  => 0x56, // v → VK_V
        11 => 0x42, // b → VK_B
        12 => 0x51, // q → VK_Q
        13 => 0x57, // w → VK_W
        14 => 0x45, // e → VK_E
        15 => 0x52, // r → VK_R
        16 => 0x59, // y → VK_Y
        17 => 0x54, // t → VK_T
        31 => 0x4F, // o → VK_O
        32 => 0x55, // u → VK_U
        34 => 0x49, // i → VK_I
        35 => 0x50, // p → VK_P
        37 => 0x4C, // l → VK_L
        38 => 0x4A, // j → VK_J
        40 => 0x4B, // k → VK_K
        45 => 0x4E, // n → VK_N
        46 => 0x4D, // m → VK_M
        // ── 数字键（主键盘行）──────────────────────────────
        18 => 0x31, // 1 → VK_1
        19 => 0x32, // 2
        20 => 0x33, // 3
        21 => 0x34, // 4
        22 => 0x36, // 6
        23 => 0x35, // 5
        25 => 0x39, // 9
        26 => 0x37, // 7
        28 => 0x38, // 8
        29 => 0x30, // 0
        // ── 符号键 ──────────────────────────────────────────
        24 => 0xBB, // = → VK_OEM_PLUS
        27 => 0xBD, // - → VK_OEM_MINUS
        30 => 0xDB, // [ → VK_OEM_4
        33 => 0xDD, // ] → VK_OEM_6
        36 => 0x0D, // Return → VK_RETURN
        39 => 0xDE, // ' → VK_OEM_7
        41 => 0xBA, // ; → VK_OEM_1
        42 => 0xDC, // \ → VK_OEM_5
        43 => 0xBC, // , → VK_OEM_COMMA
        44 => 0xBF, // / → VK_OEM_2
        47 => 0xBE, // . → VK_OEM_PERIOD
        50 => 0xC0, // ` → VK_OEM_3
        // ── 空格 / 退格 / Tab / Esc ────────────────────────
        48 => 0x09, // Tab → VK_TAB
        49 => 0x20, // Space → VK_SPACE
        51 => 0x08, // Delete(Backspace) → VK_BACK
        53 => 0x1B, // Escape → VK_ESCAPE
        // ── 修饰键 ──────────────────────────────────────────
        54 => 0xA3, // Right Cmd → VK_RWIN (映射为右 Win 键)
        55 => 0x5B, // Left Cmd  → VK_LWIN
        56 => 0xA0, // Left Shift → VK_LSHIFT
        57 => 0x14, // Caps Lock → VK_CAPITAL
        58 => 0xA2, // Left Option(Alt) → VK_LMENU
        59 => 0xA2, // Left Control → VK_LCONTROL (macOS ctrl→win ctrl)
        60 => 0xA1, // Right Shift → VK_RSHIFT
        61 => 0xA5, // Right Option → VK_RMENU
        62 => 0xA3, // Right Control → VK_RCONTROL
        63 => 0x5D, // Fn → VK_APPS (无对应，映射为菜单键)
        // ── 方向键 ──────────────────────────────────────────
        123 => 0x25, // Left  → VK_LEFT
        124 => 0x27, // Right → VK_RIGHT
        125 => 0x28, // Down  → VK_DOWN
        126 => 0x26, // Up    → VK_UP
        // ── 功能键 F1-F20 ────────────────────────────────────
        122 => 0x70, // F1
        120 => 0x71, // F2
        99  => 0x72, // F3
        118 => 0x73, // F4
        96  => 0x74, // F5
        97  => 0x75, // F6
        98  => 0x76, // F7
        100 => 0x77, // F8
        101 => 0x78, // F9
        109 => 0x79, // F10
        103 => 0x7A, // F11
        111 => 0x7B, // F12
        105 => 0x7C, // F13
        107 => 0x7D, // F14
        113 => 0x7E, // F15
        // ── 数字键盘 ────────────────────────────────────────
        65 => 0x6E, // Numpad .  → VK_DECIMAL
        67 => 0x6A, // Numpad *  → VK_MULTIPLY
        69 => 0x6B, // Numpad +  → VK_ADD
        71 => 0x90, // Num Lock  → VK_NUMLOCK
        75 => 0x6F, // Numpad /  → VK_DIVIDE
        76 => 0x0D, // Numpad Enter → VK_RETURN
        78 => 0x6D, // Numpad -  → VK_SUBTRACT
        81 => 0xBB, // Numpad =
        82 => 0x60, // Numpad 0
        83 => 0x61, // Numpad 1
        84 => 0x62, // Numpad 2
        85 => 0x63, // Numpad 3
        86 => 0x64, // Numpad 4
        87 => 0x65, // Numpad 5
        88 => 0x66, // Numpad 6
        89 => 0x67, // Numpad 7
        91 => 0x68, // Numpad 8
        92 => 0x69, // Numpad 9
        // ── 扩展键 ─────────────────────────────────────────
        114 => 0x2D, // Insert → VK_INSERT
        115 => 0x24, // Home → VK_HOME
        116 => 0x21, // Page Up → VK_PRIOR
        117 => 0x2E, // Forward Delete → VK_DELETE
        119 => 0x23, // End → VK_END
        121 => 0x22, // Page Down → VK_NEXT
        // ── 未映射：透传（Win 接收端按原值，可能无效但不崩溃）
        other => {
            log::debug!("[KeyMap] 未知 CGKeyCode {}, 透传", other);
            other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let _exec = WinInputExecutor::new();
    }

    #[test]
    fn test_cg_keycode_to_vk_alpha() {
        assert_eq!(cg_keycode_to_vk(0), 0x41);  // a → VK_A
        assert_eq!(cg_keycode_to_vk(13), 0x57); // w → VK_W
        assert_eq!(cg_keycode_to_vk(12), 0x51); // q → VK_Q
    }

    #[test]
    fn test_cg_keycode_to_vk_special() {
        assert_eq!(cg_keycode_to_vk(36), 0x0D);  // Return
        assert_eq!(cg_keycode_to_vk(49), 0x20);  // Space
        assert_eq!(cg_keycode_to_vk(51), 0x08);  // Backspace
        assert_eq!(cg_keycode_to_vk(53), 0x1B);  // Escape
        assert_eq!(cg_keycode_to_vk(48), 0x09);  // Tab
    }

    #[test]
    fn test_cg_keycode_to_vk_arrows() {
        assert_eq!(cg_keycode_to_vk(123), 0x25); // Left
        assert_eq!(cg_keycode_to_vk(124), 0x27); // Right
        assert_eq!(cg_keycode_to_vk(125), 0x28); // Down
        assert_eq!(cg_keycode_to_vk(126), 0x26); // Up
    }

    #[test]
    fn test_cg_keycode_to_vk_digits() {
        assert_eq!(cg_keycode_to_vk(18), 0x31); // 1
        assert_eq!(cg_keycode_to_vk(29), 0x30); // 0
    }
}

