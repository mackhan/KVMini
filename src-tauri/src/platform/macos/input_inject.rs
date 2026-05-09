//! macOS 键鼠事件注入实现（CGEvent）
//!
//! v0.1.43: 所有注入事件在 `post()` 前会在 `kCGEventSourceUserData` (field 42) 上写入
//! magic `KVMINI_INJECT_USER_DATA`，供 input_capture.rs 的 CGEventTap 回调过滤
//! "自我注入事件"，避免被 PreemptDetector 误识别为物理抢占。
//!
//! v0.1.50:
//! - inject_key: native_keycode 改为 X11 KeySym，通过 keysym_to_cg_keycode() 转换后注入
//! - inject_scroll: 改用 SCROLL_UNIT_PIXEL 模式（±120px 合理，旧 LINE 模式 ±120 = 滚120行）
//! - inject_mouse_button: 改用 CombinedSessionState 读取准确光标位置（修复 DecoupleCursor 下坐标偏移）

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputExecutor, MouseButton};

#[cfg(target_os = "macos")]
use crate::platform::macos::input_capture::{KVMINI_INJECT_USER_DATA, K_CG_EVENT_SOURCE_USER_DATA};

/// 给一个即将 post 的 CGEvent 打上 KVMini 合成事件 magic tag。
/// 仅 macOS 可用；失败不 panic，静默忽略（最坏情况是 PreemptDetector 可能误触发一次，
/// 用户重新撞边即可恢复，无数据风险）。
#[cfg(target_os = "macos")]
#[inline]
fn tag_synthetic(event: &core_graphics::event::CGEvent) {
    event.set_integer_value_field(K_CG_EVENT_SOURCE_USER_DATA, KVMINI_INJECT_USER_DATA);
}

pub struct MacInputExecutor;

impl MacInputExecutor {
    pub fn new() -> Self { Self }
}

impl InputExecutor for MacInputExecutor {
    fn move_mouse(&self, x: i32, y: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_mouse_move(x, y); }
        #[cfg(not(target_os = "macos"))]
        { log::debug!("[Mac-stub] move_mouse({}, {})", x, y); Ok(()) }
    }

    fn mouse_down(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_mouse_button(button, true); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn mouse_up(&self, button: MouseButton) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_mouse_button(button, false); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn key_down(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_key(native_keycode, true); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn key_up(&self, native_keycode: u16) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_key(native_keycode, false); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn scroll(&self, dx: i32, dy: i32) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { return inject_scroll(dx, dy); }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }
}

#[cfg(target_os = "macos")]
fn inject_mouse_move(x: i32, y: i32) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let point = CGPoint::new(x as f64, y as f64);
    let event = CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left)
        .map_err(|_| platform_err!("CGEvent::new_mouse_event", "创建鼠标移动事件失败"))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_mouse_button(button: MouseButton, pressed: bool) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    // v0.1.50: 用 CombinedSessionState 读取屏幕光标当前位置
    // HIDSystemState 在 DecoupleCursor 状态下返回物理鼠标位置，而非光标位置
    // CombinedSessionState 返回 CGWarpMouseCursorPosition 后的合成光标位置（准确）
    let current_pos = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .ok()
        .and_then(|s| CGEvent::new(s).ok())
        .map(|e| e.location())
        .unwrap_or(CGPoint::new(0.0, 0.0));

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let (event_type, cg_button) = match (button, pressed) {
        (MouseButton::Left, true)   => (CGEventType::LeftMouseDown,  CGMouseButton::Left),
        (MouseButton::Left, false)  => (CGEventType::LeftMouseUp,    CGMouseButton::Left),
        (MouseButton::Right, true)  => (CGEventType::RightMouseDown, CGMouseButton::Right),
        (MouseButton::Right, false) => (CGEventType::RightMouseUp,   CGMouseButton::Right),
        (_, true)                   => (CGEventType::OtherMouseDown, CGMouseButton::Center),
        (_, false)                  => (CGEventType::OtherMouseUp,   CGMouseButton::Center),
    };
    let event = CGEvent::new_mouse_event(source, event_type, current_pos, cg_button)
        .map_err(|_| platform_err!("CGEvent::new_mouse_event", "创建鼠标按键事件失败"))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_key(native_keycode: u16, pressed: bool) -> KvmResult<()> {
    use core_graphics::event::{CGEvent, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    // v0.1.50: native_keycode 是 X11 KeySym，需转换为 CGKeyCode 再注入
    let cg_code = keysym_to_cg_keycode(native_keycode);
    let event = CGEvent::new_keyboard_event(source, cg_code as CGKeyCode, pressed)
        .map_err(|_| platform_err!("CGEvent::new_keyboard_event",
            format!("创建键盘事件失败: keysym={:#06X} cg_code={}", native_keycode, cg_code)))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn inject_scroll(dx: i32, dy: i32) -> KvmResult<()> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // v0.1.52: 改回 SCROLL_UNIT_LINE(=1) + 除以 120（Barrier 行业标准方案）
    // 协议层滚轮单位为 ±120（与 WM_MOUSEWHEEL WHEEL_DELTA 一致），
    // macOS LINE 单位 = 1 行；传入 ±1 为自然一格；传 ±120 = 滚 120 行（过快）。
    // 除以 120 将协议单位还原为 macOS 行数，感受与本机鼠标一致。
    const SCROLL_UNIT_LINE: u32 = 1;
    // 至少 ±1 行，避免小 delta 完全丢失
    let lines_y = if dy == 0 { 0 } else { (dy / 120).max(1).min(20) * dy.signum() };
    let lines_x = if dx == 0 { 0 } else { (dx / 120).max(1).min(20) * dx.signum() };

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| platform_err!("CGEventSource::new", "创建 CGEventSource 失败"))?;
    let event = CGEvent::new_scroll_event(source, SCROLL_UNIT_LINE, 2, lines_y, lines_x, 0)
        .map_err(|_| platform_err!("CGEvent::new_scroll_event", "创建滚轮事件失败"))?;
    tag_synthetic(&event);
    event.post(core_graphics::event::CGEventTapLocation::HID);
    Ok(())
}

/// X11 KeySym → macOS CGKeyCode 转换表（Mac 被控端，注入前调用）
///
/// v0.1.50: 采用 Barrier/Synergy 行业标准，协议层使用 X11 KeySym 作为通用键码。
/// Win 控制端将 VK 转为 KeySym 发送，Mac 被控端此函数将 KeySym 转回 CGKeyCode 注入。
///
/// 参考：
///   https://www.x.org/releases/X11R7.7/doc/xproto/x11protocol.html#keysym_encoding
///   https://eastmanreference.com/complete-list-of-applescript-key-codes
pub(crate) fn keysym_to_cg_keycode(ks: u16) -> u16 {
    match ks {
        // ── 字母键 (XK_lowercase → CGKeyCode) ──────────────────────────
        0x0061 => 0,   // XK_a → a
        0x0073 => 1,   // XK_s → s
        0x0064 => 2,   // XK_d → d
        0x0066 => 3,   // XK_f → f
        0x0068 => 4,   // XK_h → h
        0x0067 => 5,   // XK_g → g
        0x007A => 6,   // XK_z → z
        0x0078 => 7,   // XK_x → x
        0x0063 => 8,   // XK_c → c
        0x0076 => 9,   // XK_v → v
        0x0062 => 11,  // XK_b → b
        0x0071 => 12,  // XK_q → q
        0x0077 => 13,  // XK_w → w
        0x0065 => 14,  // XK_e → e
        0x0072 => 15,  // XK_r → r
        0x0079 => 16,  // XK_y → y
        0x0074 => 17,  // XK_t → t
        0x006F => 31,  // XK_o → o
        0x0075 => 32,  // XK_u → u
        0x0069 => 34,  // XK_i → i
        0x0070 => 35,  // XK_p → p
        0x006C => 37,  // XK_l → l
        0x006A => 38,  // XK_j → j
        0x006B => 40,  // XK_k → k
        0x006E => 45,  // XK_n → n
        0x006D => 46,  // XK_m → m
        // ── 数字键 ──────────────────────────────────────────────────────
        0x0031 => 18,  // XK_1 → 1
        0x0032 => 19,  // XK_2 → 2
        0x0033 => 20,  // XK_3 → 3
        0x0034 => 21,  // XK_4 → 4
        0x0036 => 22,  // XK_6 → 6
        0x0035 => 23,  // XK_5 → 5
        0x0039 => 25,  // XK_9 → 9
        0x0037 => 26,  // XK_7 → 7
        0x0038 => 28,  // XK_8 → 8
        0x0030 => 29,  // XK_0 → 0
        // ── 符号键 ──────────────────────────────────────────────────────
        0x003D => 24,  // XK_equal         → =
        0x002D => 27,  // XK_minus         → -
        0x005B => 30,  // XK_bracketleft   → [
        0x005D => 33,  // XK_bracketright  → ]
        0xFF0D => 36,  // XK_Return        → Return
        0x0027 => 39,  // XK_apostrophe    → '
        0x003B => 41,  // XK_semicolon     → ;
        0x005C => 42,  // XK_backslash     → \
        0x002C => 43,  // XK_comma         → ,
        0x002F => 44,  // XK_slash         → /
        0x002E => 47,  // XK_period        → .
        0x0060 => 50,  // XK_grave         → `
        // ── 空格 / 退格 / Tab / Esc ────────────────────────────────────
        0xFF09 => 48,  // XK_Tab       → Tab
        0x0020 => 49,  // XK_space     → Space
        0xFF08 => 51,  // XK_BackSpace → Delete(Backspace)
        0xFF1B => 53,  // XK_Escape    → Escape
        // ── 修饰键 ──────────────────────────────────────────────────────
        0xFFEC => 54,  // XK_Super_R   → Right Cmd
        0xFFEB => 55,  // XK_Super_L   → Left Cmd
        0xFFE1 => 56,  // XK_Shift_L   → Left Shift
        0xFFE5 => 57,  // XK_Caps_Lock → Caps Lock
        0xFFE9 => 58,  // XK_Alt_L     → Left Option
        0xFFE3 => 59,  // XK_Control_L → Left Control
        0xFFE2 => 60,  // XK_Shift_R   → Right Shift
        0xFFEA => 61,  // XK_Alt_R     → Right Option
        0xFFE4 => 62,  // XK_Control_R → Right Control
        // ── 方向键 ──────────────────────────────────────────────────────
        0xFF51 => 123, // XK_Left  → Left
        0xFF53 => 124, // XK_Right → Right
        0xFF54 => 125, // XK_Down  → Down
        0xFF52 => 126, // XK_Up    → Up
        // ── 功能键 F1-F15 ───────────────────────────────────────────────
        0xFFBE => 122, // XK_F1
        0xFFBF => 120, // XK_F2
        0xFFC0 => 99,  // XK_F3
        0xFFC1 => 118, // XK_F4
        0xFFC2 => 96,  // XK_F5
        0xFFC3 => 97,  // XK_F6
        0xFFC4 => 98,  // XK_F7
        0xFFC5 => 100, // XK_F8
        0xFFC6 => 101, // XK_F9
        0xFFC7 => 109, // XK_F10
        0xFFC8 => 103, // XK_F11
        0xFFC9 => 111, // XK_F12
        0xFFCA => 105, // XK_F13
        0xFFCB => 107, // XK_F14
        0xFFCC => 113, // XK_F15
        // ── 数字键盘 ────────────────────────────────────────────────────
        0xFFAE => 65,  // XK_KP_Decimal  → Numpad .
        0xFFAA => 67,  // XK_KP_Multiply → Numpad *
        0xFFAB => 69,  // XK_KP_Add      → Numpad +
        0xFF7F => 71,  // XK_Num_Lock    → Num Lock
        0xFFAF => 75,  // XK_KP_Divide   → Numpad /
        0xFF8D => 76,  // XK_KP_Enter    → Numpad Enter
        0xFFAD => 78,  // XK_KP_Subtract → Numpad -
        0xFFB0 => 82,  // XK_KP_0 → Numpad 0
        0xFFB1 => 83,  // XK_KP_1 → Numpad 1
        0xFFB2 => 84,  // XK_KP_2 → Numpad 2
        0xFFB3 => 85,  // XK_KP_3 → Numpad 3
        0xFFB4 => 86,  // XK_KP_4 → Numpad 4
        0xFFB5 => 87,  // XK_KP_5 → Numpad 5
        0xFFB6 => 88,  // XK_KP_6 → Numpad 6
        0xFFB7 => 89,  // XK_KP_7 → Numpad 7
        0xFFB8 => 91,  // XK_KP_8 → Numpad 8
        0xFFB9 => 92,  // XK_KP_9 → Numpad 9
        // ── 扩展键 ──────────────────────────────────────────────────────
        0xFF63 => 114, // XK_Insert    → Insert
        0xFF50 => 115, // XK_Home      → Home
        0xFF55 => 116, // XK_Page_Up   → Page Up
        0xFFFF => 117, // XK_Delete    → Forward Delete
        0xFF57 => 119, // XK_End       → End
        0xFF56 => 121, // XK_Page_Down → Page Down
        // ── 未映射：透传（保守降级）──────────────────────────────────────
        other => {
            log::debug!("[KeyMap/Mac] 未知 KeySym {:#06X}, 透传", other);
            other
        }
    }
}
