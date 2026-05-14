//! Windows 键鼠事件注入实现
//!
//! 使用 SendInput 注入键鼠事件。
//! v0.1.41: 多屏使用 SM_CXVIRTUALSCREEN + MOUSEEVENTF_VIRTUALDESK
//!          XButton (Side1/Side2) 补全
//!          滚轮 mouseData 正确使用 i32 as u32 表达负向滚动
//! v0.1.50: 键码体系改为 X11 KeySym（与 Barrier/Synergy 行业标准对齐）
//!          keysym_to_vk() 替换旧的 cg_keycode_to_vk()

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

                // v0.1.59 M1：注入路径全链路日志（trace 级别，不影响 release 性能）
                log::trace!(
                    "[Win/Inject] move_mouse x={} y={} → vscreen({},{},{}x{}) abs=({},{})",
                    x, y, vscreen_x, vscreen_y, vscreen_w, vscreen_h, abs_x, abs_y
                );

                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_MOUSE;
                let mi = input.u.mi_mut();
                mi.dx = abs_x;
                mi.dy = abs_y;
                // MOUSEEVENTF_VIRTUALDESK: 坐标相对于整个虚拟桌面（多显示器）
                mi.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
                let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if n != 1 {
                    log::warn!("[Win/Inject] move_mouse SendInput 返回 {}（期望 1）", n);
                }
            }
        }
        Ok(())
    }

    fn mouse_down(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                // v0.1.59 M1：按键注入日志
                log::debug!("[Win/Inject] mouse_down {:?}", button);
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
                let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if n != 1 {
                    log::warn!("[Win/Inject] mouse_down SendInput 返回 {}（期望 1）", n);
                }
            }
        }
        Ok(())
    }

    fn mouse_up(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                log::debug!("[Win/Inject] mouse_up {:?}", button);
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
                let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if n != 1 {
                    log::warn!("[Win/Inject] mouse_up SendInput 返回 {}（期望 1）", n);
                }
            }
        }
        Ok(())
    }

    fn key_down(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                // v0.1.50: native_keycode 是 X11 KeySym，转换为 VK 后注入
                let vk = keysym_to_vk(native_keycode);
                log::debug!("[Win/Inject] key_down keysym={:#06X} vk={:#04X}", native_keycode, vk);
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_KEYBOARD;
                let ki = input.u.ki_mut();
                ki.wVk = vk;
                ki.dwFlags = 0;
                let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if n != 1 {
                    log::warn!("[Win/Inject] key_down SendInput 返回 {}（期望 1）", n);
                }
            }
        }
        Ok(())
    }

    fn key_up(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                // v0.1.50: native_keycode 是 X11 KeySym，转换为 VK 后注入
                let vk = keysym_to_vk(native_keycode);
                log::debug!("[Win/Inject] key_up keysym={:#06X} vk={:#04X}", native_keycode, vk);
                let mut input: INPUT = std::mem::zeroed();
                input.type_ = INPUT_KEYBOARD;
                let ki = input.u.ki_mut();
                ki.wVk = vk;
                ki.dwFlags = KEYEVENTF_KEYUP;
                let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if n != 1 {
                    log::warn!("[Win/Inject] key_up SendInput 返回 {}（期望 1）", n);
                }
            }
        }
        Ok(())
    }

    fn scroll(&self, dx: i32, dy: i32) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::*;
            unsafe {
                // v0.1.59 M1：F1 滚轮诊断的核心埋点
                // 协议层 dx/dy 已是 ±120 的 wheel delta（与 WHEEL_DELTA 一致），
                // SendInput MOUSEEVENTF_WHEEL/HWHEEL 期望 mouseData = WHEEL_DELTA 倍数。
                log::debug!("[Win/Inject] scroll dx={} dy={}", dx, dy);
                if dy != 0 {
                    let mut input: INPUT = std::mem::zeroed();
                    input.type_ = INPUT_MOUSE;
                    let mi = input.u.mi_mut();
                    // v0.1.41: 正确使用 i32 as u32 保留符号位（负值滚动方向）
                    mi.mouseData = dy as u32;
                    mi.dwFlags = MOUSEEVENTF_WHEEL;
                    let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                    if n != 1 {
                        log::warn!("[Win/Inject] scroll(WHEEL) SendInput 返回 {}（期望 1，dy={}）", n, dy);
                    } else {
                        log::trace!("[Win/Inject] scroll(WHEEL) 注入成功 dy={}", dy);
                    }
                }
                if dx != 0 {
                    let mut input: INPUT = std::mem::zeroed();
                    input.type_ = INPUT_MOUSE;
                    let mi = input.u.mi_mut();
                    mi.mouseData = dx as u32;
                    mi.dwFlags = MOUSEEVENTF_HWHEEL;
                    let n = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                    if n != 1 {
                        log::warn!("[Win/Inject] scroll(HWHEEL) SendInput 返回 {}（期望 1，dx={}）", n, dx);
                    } else {
                        log::trace!("[Win/Inject] scroll(HWHEEL) 注入成功 dx={}", dx);
                    }
                }
            }
        }
        Ok(())
    }
}

/// X11 KeySym → Windows Virtual Key Code 转换表（Win 被控端，注入前调用）
///
/// v0.1.50: 采用 Barrier/Synergy 行业标准，协议层使用 X11 KeySym 作为通用键码。
/// Mac 控制端将 CGKeyCode 转为 KeySym 发送，Win 被控端此函数将 KeySym 转回 VK 注入。
///
/// 参考：
///   https://www.x.org/releases/X11R7.7/doc/xproto/x11protocol.html#keysym_encoding
///   https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
pub(crate) fn keysym_to_vk(ks: u16) -> u16 {
    match ks {
        // ── 字母键 (XK_lowercase → VK) ──────────────────────────────────
        0x0061 => 0x41, // XK_a → VK_A
        0x0062 => 0x42, // XK_b → VK_B
        0x0063 => 0x43, // XK_c → VK_C
        0x0064 => 0x44, // XK_d → VK_D
        0x0065 => 0x45, // XK_e → VK_E
        0x0066 => 0x46, // XK_f → VK_F
        0x0067 => 0x47, // XK_g → VK_G
        0x0068 => 0x48, // XK_h → VK_H
        0x0069 => 0x49, // XK_i → VK_I
        0x006A => 0x4A, // XK_j → VK_J
        0x006B => 0x4B, // XK_k → VK_K
        0x006C => 0x4C, // XK_l → VK_L
        0x006D => 0x4D, // XK_m → VK_M
        0x006E => 0x4E, // XK_n → VK_N
        0x006F => 0x4F, // XK_o → VK_O
        0x0070 => 0x50, // XK_p → VK_P
        0x0071 => 0x51, // XK_q → VK_Q
        0x0072 => 0x52, // XK_r → VK_R
        0x0073 => 0x53, // XK_s → VK_S
        0x0074 => 0x54, // XK_t → VK_T
        0x0075 => 0x55, // XK_u → VK_U
        0x0076 => 0x56, // XK_v → VK_V
        0x0077 => 0x57, // XK_w → VK_W
        0x0078 => 0x58, // XK_x → VK_X
        0x0079 => 0x59, // XK_y → VK_Y
        0x007A => 0x5A, // XK_z → VK_Z
        // ── 数字键 ──────────────────────────────────────────────────────
        0x0030 => 0x30, // XK_0 → VK_0
        0x0031 => 0x31, // XK_1 → VK_1
        0x0032 => 0x32, // XK_2 → VK_2
        0x0033 => 0x33, // XK_3 → VK_3
        0x0034 => 0x34, // XK_4 → VK_4
        0x0035 => 0x35, // XK_5 → VK_5
        0x0036 => 0x36, // XK_6 → VK_6
        0x0037 => 0x37, // XK_7 → VK_7
        0x0038 => 0x38, // XK_8 → VK_8
        0x0039 => 0x39, // XK_9 → VK_9
        // ── 空格 / 退格 / Tab / Esc / Enter ────────────────────────────
        0xFF08 => 0x08, // XK_BackSpace → VK_BACK
        0xFF09 => 0x09, // XK_Tab       → VK_TAB
        0xFF0D => 0x0D, // XK_Return    → VK_RETURN
        0xFF1B => 0x1B, // XK_Escape    → VK_ESCAPE
        0x0020 => 0x20, // XK_space     → VK_SPACE
        // ── 符号键 ──────────────────────────────────────────────────────
        0x003D => 0xBB, // XK_equal         → VK_OEM_PLUS
        0x002D => 0xBD, // XK_minus         → VK_OEM_MINUS
        0x005B => 0xDB, // XK_bracketleft   → VK_OEM_4
        0x005D => 0xDD, // XK_bracketright  → VK_OEM_6
        0x0027 => 0xDE, // XK_apostrophe    → VK_OEM_7
        0x003B => 0xBA, // XK_semicolon     → VK_OEM_1
        0x005C => 0xDC, // XK_backslash     → VK_OEM_5
        0x002C => 0xBC, // XK_comma         → VK_OEM_COMMA
        0x002F => 0xBF, // XK_slash         → VK_OEM_2
        0x002E => 0xBE, // XK_period        → VK_OEM_PERIOD
        0x0060 => 0xC0, // XK_grave         → VK_OEM_3
        // ── 方向键 ──────────────────────────────────────────────────────
        0xFF51 => 0x25, // XK_Left  → VK_LEFT
        0xFF52 => 0x26, // XK_Up    → VK_UP
        0xFF53 => 0x27, // XK_Right → VK_RIGHT
        0xFF54 => 0x28, // XK_Down  → VK_DOWN
        // ── 功能键 F1-F15 ───────────────────────────────────────────────
        0xFFBE => 0x70, // XK_F1
        0xFFBF => 0x71, // XK_F2
        0xFFC0 => 0x72, // XK_F3
        0xFFC1 => 0x73, // XK_F4
        0xFFC2 => 0x74, // XK_F5
        0xFFC3 => 0x75, // XK_F6
        0xFFC4 => 0x76, // XK_F7
        0xFFC5 => 0x77, // XK_F8
        0xFFC6 => 0x78, // XK_F9
        0xFFC7 => 0x79, // XK_F10
        0xFFC8 => 0x7A, // XK_F11
        0xFFC9 => 0x7B, // XK_F12
        0xFFCA => 0x7C, // XK_F13
        0xFFCB => 0x7D, // XK_F14
        0xFFCC => 0x7E, // XK_F15
        // ── 修饰键 ──────────────────────────────────────────────────────
        0xFFE1 => 0xA0, // XK_Shift_L   → VK_LSHIFT
        0xFFE2 => 0xA1, // XK_Shift_R   → VK_RSHIFT
        0xFFE3 => 0xA2, // XK_Control_L → VK_LCONTROL
        0xFFE4 => 0xA3, // XK_Control_R → VK_RCONTROL
        0xFFE9 => 0xA4, // XK_Alt_L     → VK_LMENU
        0xFFEA => 0xA5, // XK_Alt_R     → VK_RMENU
        0xFFEB => 0x5B, // XK_Super_L   → VK_LWIN (= Mac Cmd_L)
        0xFFEC => 0x5C, // XK_Super_R   → VK_RWIN (= Mac Cmd_R)
        0xFFE5 => 0x14, // XK_Caps_Lock → VK_CAPITAL
        // ── 扩展键 ──────────────────────────────────────────────────────
        0xFF63 => 0x2D, // XK_Insert    → VK_INSERT
        0xFFFF => 0x2E, // XK_Delete    → VK_DELETE
        0xFF50 => 0x24, // XK_Home      → VK_HOME
        0xFF57 => 0x23, // XK_End       → VK_END
        0xFF55 => 0x21, // XK_Page_Up   → VK_PRIOR
        0xFF56 => 0x22, // XK_Page_Down → VK_NEXT
        // ── 数字键盘 ────────────────────────────────────────────────────
        0xFFB0 => 0x60, // XK_KP_0        → VK_NUMPAD0
        0xFFB1 => 0x61, // XK_KP_1        → VK_NUMPAD1
        0xFFB2 => 0x62, // XK_KP_2        → VK_NUMPAD2
        0xFFB3 => 0x63, // XK_KP_3        → VK_NUMPAD3
        0xFFB4 => 0x64, // XK_KP_4        → VK_NUMPAD4
        0xFFB5 => 0x65, // XK_KP_5        → VK_NUMPAD5
        0xFFB6 => 0x66, // XK_KP_6        → VK_NUMPAD6
        0xFFB7 => 0x67, // XK_KP_7        → VK_NUMPAD7
        0xFFB8 => 0x68, // XK_KP_8        → VK_NUMPAD8
        0xFFB9 => 0x69, // XK_KP_9        → VK_NUMPAD9
        0xFFAA => 0x6A, // XK_KP_Multiply → VK_MULTIPLY
        0xFFAB => 0x6B, // XK_KP_Add      → VK_ADD
        0xFFAD => 0x6D, // XK_KP_Subtract → VK_SUBTRACT
        0xFFAE => 0x6E, // XK_KP_Decimal  → VK_DECIMAL
        0xFFAF => 0x6F, // XK_KP_Divide   → VK_DIVIDE
        0xFF7F => 0x90, // XK_Num_Lock    → VK_NUMLOCK
        // ── 未映射：透传（保守降级）──────────────────────────────────────
        other => {
            log::debug!("[KeyMap/Win] 未知 KeySym {:#06X}, 透传", other);
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
    fn test_keysym_to_vk_alpha() {
        assert_eq!(keysym_to_vk(0x0061), 0x41); // XK_a → VK_A
        assert_eq!(keysym_to_vk(0x0077), 0x57); // XK_w → VK_W
        assert_eq!(keysym_to_vk(0x0071), 0x51); // XK_q → VK_Q
    }

    #[test]
    fn test_keysym_to_vk_special() {
        assert_eq!(keysym_to_vk(0xFF0D), 0x0D); // XK_Return    → VK_RETURN
        assert_eq!(keysym_to_vk(0x0020), 0x20); // XK_space     → VK_SPACE
        assert_eq!(keysym_to_vk(0xFF08), 0x08); // XK_BackSpace → VK_BACK
        assert_eq!(keysym_to_vk(0xFF1B), 0x1B); // XK_Escape    → VK_ESCAPE
        assert_eq!(keysym_to_vk(0xFF09), 0x09); // XK_Tab       → VK_TAB
    }

    #[test]
    fn test_keysym_to_vk_arrows() {
        assert_eq!(keysym_to_vk(0xFF51), 0x25); // XK_Left  → VK_LEFT
        assert_eq!(keysym_to_vk(0xFF52), 0x26); // XK_Up    → VK_UP
        assert_eq!(keysym_to_vk(0xFF53), 0x27); // XK_Right → VK_RIGHT
        assert_eq!(keysym_to_vk(0xFF54), 0x28); // XK_Down  → VK_DOWN
    }

    #[test]
    fn test_keysym_to_vk_digits() {
        assert_eq!(keysym_to_vk(0x0031), 0x31); // XK_1 → VK_1
        assert_eq!(keysym_to_vk(0x0030), 0x30); // XK_0 → VK_0
    }

    #[test]
    fn test_keysym_to_vk_function_keys() {
        assert_eq!(keysym_to_vk(0xFFBE), 0x70); // XK_F1  → VK_F1
        assert_eq!(keysym_to_vk(0xFFC9), 0x7B); // XK_F12 → VK_F12
    }

    #[test]
    fn test_keysym_to_vk_modifiers() {
        assert_eq!(keysym_to_vk(0xFFE1), 0xA0); // XK_Shift_L   → VK_LSHIFT
        assert_eq!(keysym_to_vk(0xFFE3), 0xA2); // XK_Control_L → VK_LCONTROL
        assert_eq!(keysym_to_vk(0xFFE9), 0xA4); // XK_Alt_L     → VK_LMENU
        assert_eq!(keysym_to_vk(0xFFEB), 0x5B); // XK_Super_L   → VK_LWIN
    }
}
