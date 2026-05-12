//! 跨平台键码映射模块
//!
//! 定义跨平台统一键码表，各平台适配层负责本地键码与统一键码的双向转换。
//! 从现有 src-tauri/keycode_map.rs 合并而来。

use std::collections::HashMap;

/// macOS CGKeyCode 常量
#[allow(dead_code)]
pub mod cg_key {
    pub const A: u16 = 0;
    pub const S: u16 = 1;
    pub const D: u16 = 2;
    pub const F: u16 = 3;
    pub const H: u16 = 4;
    pub const G: u16 = 5;
    pub const Z: u16 = 6;
    pub const X: u16 = 7;
    pub const C: u16 = 8;
    pub const V: u16 = 9;
    pub const B: u16 = 11;
    pub const Q: u16 = 12;
    pub const W: u16 = 13;
    pub const E: u16 = 14;
    pub const R: u16 = 15;
    pub const Y: u16 = 16;
    pub const T: u16 = 17;
    pub const NUM_1: u16 = 18;
    pub const NUM_2: u16 = 19;
    pub const NUM_3: u16 = 20;
    pub const NUM_4: u16 = 21;
    pub const NUM_6: u16 = 22;
    pub const NUM_5: u16 = 23;
    pub const EQUAL: u16 = 24;
    pub const NUM_9: u16 = 25;
    pub const NUM_7: u16 = 26;
    pub const MINUS: u16 = 27;
    pub const NUM_8: u16 = 28;
    pub const NUM_0: u16 = 29;
    pub const RIGHT_BRACKET: u16 = 30;
    pub const O: u16 = 31;
    pub const U: u16 = 32;
    pub const LEFT_BRACKET: u16 = 33;
    pub const I: u16 = 34;
    pub const P: u16 = 35;
    pub const RETURN: u16 = 36;
    pub const L: u16 = 37;
    pub const J: u16 = 38;
    pub const QUOTE: u16 = 39;
    pub const K: u16 = 40;
    pub const SEMICOLON: u16 = 41;
    pub const BACKSLASH: u16 = 42;
    pub const COMMA: u16 = 43;
    pub const SLASH: u16 = 44;
    pub const N: u16 = 45;
    pub const M: u16 = 46;
    pub const PERIOD: u16 = 47;
    pub const TAB: u16 = 48;
    pub const SPACE: u16 = 49;
    pub const GRAVE: u16 = 50;
    pub const DELETE: u16 = 51;
    pub const ESCAPE: u16 = 53;
    pub const RIGHT_CMD: u16 = 54;
    pub const LEFT_CMD: u16 = 55;
    pub const LEFT_SHIFT: u16 = 56;
    pub const CAPS_LOCK: u16 = 57;
    pub const LEFT_ALT: u16 = 58;
    pub const LEFT_CTRL: u16 = 59;
    pub const RIGHT_SHIFT: u16 = 60;
    pub const RIGHT_ALT: u16 = 61;
    pub const RIGHT_CTRL: u16 = 62;
    pub const F17: u16 = 64;
    pub const KEYPAD_DECIMAL: u16 = 65;
    pub const KEYPAD_MULTIPLY: u16 = 67;
    pub const KEYPAD_PLUS: u16 = 69;
    pub const KEYPAD_CLEAR: u16 = 71;
    pub const VOLUME_UP: u16 = 72;
    pub const VOLUME_DOWN: u16 = 73;
    pub const MUTE: u16 = 74;
    pub const KEYPAD_DIVIDE: u16 = 75;
    pub const KEYPAD_ENTER: u16 = 76;
    pub const KEYPAD_MINUS: u16 = 78;
    pub const F18: u16 = 79;
    pub const F19: u16 = 80;
    pub const KEYPAD_EQUAL: u16 = 81;
    pub const KEYPAD_0: u16 = 82;
    pub const KEYPAD_1: u16 = 83;
    pub const KEYPAD_2: u16 = 84;
    pub const KEYPAD_3: u16 = 85;
    pub const KEYPAD_4: u16 = 86;
    pub const KEYPAD_5: u16 = 87;
    pub const KEYPAD_6: u16 = 88;
    pub const KEYPAD_7: u16 = 89;
    pub const F20: u16 = 90;
    pub const KEYPAD_8: u16 = 91;
    pub const KEYPAD_9: u16 = 92;
    pub const F5: u16 = 96;
    pub const F6: u16 = 97;
    pub const F7: u16 = 98;
    pub const F3: u16 = 99;
    pub const F8: u16 = 100;
    pub const F9: u16 = 101;
    pub const F11: u16 = 103;
    pub const F13: u16 = 105;
    pub const F16: u16 = 106;
    pub const F14: u16 = 107;
    pub const F10: u16 = 109;
    pub const F12: u16 = 111;
    pub const F15: u16 = 113;
    pub const HELP: u16 = 114;
    pub const HOME: u16 = 115;
    pub const PAGE_UP: u16 = 116;
    pub const FORWARD_DELETE: u16 = 117;
    pub const F4: u16 = 118;
    pub const END: u16 = 119;
    pub const F2: u16 = 120;
    pub const PAGE_DOWN: u16 = 121;
    pub const F1: u16 = 122;
    pub const LEFT_ARROW: u16 = 123;
    pub const RIGHT_ARROW: u16 = 124;
    pub const DOWN_ARROW: u16 = 125;
    pub const UP_ARROW: u16 = 126;
}

/// Windows Virtual Key Codes 常量
#[allow(dead_code)]
pub mod vk {
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;
    pub const CAPITAL: u16 = 0x14;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21;
    pub const NEXT: u16 = 0x22;
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const KEY_0: u16 = 0x30;
    pub const KEY_1: u16 = 0x31;
    pub const KEY_2: u16 = 0x32;
    pub const KEY_3: u16 = 0x33;
    pub const KEY_4: u16 = 0x34;
    pub const KEY_5: u16 = 0x35;
    pub const KEY_6: u16 = 0x36;
    pub const KEY_7: u16 = 0x37;
    pub const KEY_8: u16 = 0x38;
    pub const KEY_9: u16 = 0x39;
    pub const KEY_A: u16 = 0x41;
    pub const KEY_B: u16 = 0x42;
    pub const KEY_C: u16 = 0x43;
    pub const KEY_D: u16 = 0x44;
    pub const KEY_E: u16 = 0x45;
    pub const KEY_F: u16 = 0x46;
    pub const KEY_G: u16 = 0x47;
    pub const KEY_H: u16 = 0x48;
    pub const KEY_I: u16 = 0x49;
    pub const KEY_J: u16 = 0x4A;
    pub const KEY_K: u16 = 0x4B;
    pub const KEY_L: u16 = 0x4C;
    pub const KEY_M: u16 = 0x4D;
    pub const KEY_N: u16 = 0x4E;
    pub const KEY_O: u16 = 0x4F;
    pub const KEY_P: u16 = 0x50;
    pub const KEY_Q: u16 = 0x51;
    pub const KEY_R: u16 = 0x52;
    pub const KEY_S: u16 = 0x53;
    pub const KEY_T: u16 = 0x54;
    pub const KEY_U: u16 = 0x55;
    pub const KEY_V: u16 = 0x56;
    pub const KEY_W: u16 = 0x57;
    pub const KEY_X: u16 = 0x58;
    pub const KEY_Y: u16 = 0x59;
    pub const KEY_Z: u16 = 0x5A;
    pub const LWIN: u16 = 0x5B;
    pub const RWIN: u16 = 0x5C;
    pub const NUMPAD0: u16 = 0x60;
    pub const NUMPAD1: u16 = 0x61;
    pub const NUMPAD2: u16 = 0x62;
    pub const NUMPAD3: u16 = 0x63;
    pub const NUMPAD4: u16 = 0x64;
    pub const NUMPAD5: u16 = 0x65;
    pub const NUMPAD6: u16 = 0x66;
    pub const NUMPAD7: u16 = 0x67;
    pub const NUMPAD8: u16 = 0x68;
    pub const NUMPAD9: u16 = 0x69;
    pub const MULTIPLY: u16 = 0x6A;
    pub const ADD: u16 = 0x6B;
    pub const SUBTRACT: u16 = 0x6D;
    pub const DECIMAL: u16 = 0x6E;
    pub const DIVIDE: u16 = 0x6F;
    pub const F1: u16 = 0x70;
    pub const F2: u16 = 0x71;
    pub const F3: u16 = 0x72;
    pub const F4: u16 = 0x73;
    pub const F5: u16 = 0x74;
    pub const F6: u16 = 0x75;
    pub const F7: u16 = 0x76;
    pub const F8: u16 = 0x77;
    pub const F9: u16 = 0x78;
    pub const F10: u16 = 0x79;
    pub const F11: u16 = 0x7A;
    pub const F12: u16 = 0x7B;
    pub const F13: u16 = 0x7C;
    pub const F14: u16 = 0x7D;
    pub const F15: u16 = 0x7E;
    pub const F16: u16 = 0x7F;
    pub const F17: u16 = 0x80;
    pub const F18: u16 = 0x81;
    pub const F19: u16 = 0x82;
    pub const F20: u16 = 0x83;
    pub const NUMLOCK: u16 = 0x90;
    pub const SCROLL: u16 = 0x91;
    pub const LSHIFT: u16 = 0xA0;
    pub const RSHIFT: u16 = 0xA1;
    pub const LCONTROL: u16 = 0xA2;
    pub const RCONTROL: u16 = 0xA3;
    pub const LMENU: u16 = 0xA4;
    pub const RMENU: u16 = 0xA5;
    pub const OEM_SEMICOLON: u16 = 0xBA;
    pub const OEM_PLUS: u16 = 0xBB;
    pub const OEM_COMMA: u16 = 0xBC;
    pub const OEM_MINUS: u16 = 0xBD;
    pub const OEM_PERIOD: u16 = 0xBE;
    pub const OEM_SLASH: u16 = 0xBF;
    pub const OEM_GRAVE: u16 = 0xC0;
    pub const OEM_OPEN_BRACKET: u16 = 0xDB;
    pub const OEM_BACKSLASH: u16 = 0xDC;
    pub const OEM_CLOSE_BRACKET: u16 = 0xDD;
    pub const OEM_QUOTE: u16 = 0xDE;
}

/// Command 键映射模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKeyMapping {
    /// Command → Win 键（默认）
    Win,
    /// Command → Ctrl 键
    Ctrl,
}

impl Default for CommandKeyMapping {
    fn default() -> Self {
        Self::Win
    }
}

/// Windows VirtualKey → X11 KeySym 转换（Win 主控端采集时调用）
///
/// 与 `keysym_to_vk`（Win 被控端注入时用）互为逆向。
/// 采用与 Barrier/Deskflow 相同的 X11 KeySym 作为协议中间层。
///
/// 键位映射统一模式（key_mapping_unified=true）：
///   VK_LCONTROL / VK_RCONTROL → XK_Super_L / XK_Super_R
///   即 Win 端 Ctrl+C → Mac 被控端收到 Cmd+C（实现统一复制快捷键）
pub fn vk_to_keysym(vk: u16, unified: bool) -> u16 {
    // 统一模式：Ctrl → Cmd（Super）
    if unified {
        match vk {
            0xA2 => return 0xFFEB, // VK_LCONTROL → XK_Super_L
            0xA3 => return 0xFFEC, // VK_RCONTROL → XK_Super_R
            0x11 => return 0xFFEB, // VK_CONTROL  → XK_Super_L（泛化 Ctrl）
            _ => {}
        }
    }
    match vk {
        // ── 字母键 ──────────────────────────────────────────────────────
        0x41 => 0x0061, // VK_A → XK_a
        0x42 => 0x0062, // VK_B → XK_b
        0x43 => 0x0063, // VK_C → XK_c
        0x44 => 0x0064, // VK_D → XK_d
        0x45 => 0x0065, // VK_E → XK_e
        0x46 => 0x0066, // VK_F → XK_f
        0x47 => 0x0067, // VK_G → XK_g
        0x48 => 0x0068, // VK_H → XK_h
        0x49 => 0x0069, // VK_I → XK_i
        0x4A => 0x006A, // VK_J → XK_j
        0x4B => 0x006B, // VK_K → XK_k
        0x4C => 0x006C, // VK_L → XK_l
        0x4D => 0x006D, // VK_M → XK_m
        0x4E => 0x006E, // VK_N → XK_n
        0x4F => 0x006F, // VK_O → XK_o
        0x50 => 0x0070, // VK_P → XK_p
        0x51 => 0x0071, // VK_Q → XK_q
        0x52 => 0x0072, // VK_R → XK_r
        0x53 => 0x0073, // VK_S → XK_s
        0x54 => 0x0074, // VK_T → XK_t
        0x55 => 0x0075, // VK_U → XK_u
        0x56 => 0x0076, // VK_V → XK_v
        0x57 => 0x0077, // VK_W → XK_w
        0x58 => 0x0078, // VK_X → XK_x
        0x59 => 0x0079, // VK_Y → XK_y
        0x5A => 0x007A, // VK_Z → XK_z
        // ── 数字键 ──────────────────────────────────────────────────────
        0x30 => 0x0030, // VK_0 → XK_0
        0x31 => 0x0031, // VK_1 → XK_1
        0x32 => 0x0032, // VK_2 → XK_2
        0x33 => 0x0033, // VK_3 → XK_3
        0x34 => 0x0034, // VK_4 → XK_4
        0x35 => 0x0035, // VK_5 → XK_5
        0x36 => 0x0036, // VK_6 → XK_6
        0x37 => 0x0037, // VK_7 → XK_7
        0x38 => 0x0038, // VK_8 → XK_8
        0x39 => 0x0039, // VK_9 → XK_9
        // ── 空格 / 退格 / Tab / Esc / Enter ─────────────────────────────
        0x08 => 0xFF08, // VK_BACK    → XK_BackSpace
        0x09 => 0xFF09, // VK_TAB     → XK_Tab
        0x0D => 0xFF0D, // VK_RETURN  → XK_Return
        0x1B => 0xFF1B, // VK_ESCAPE  → XK_Escape
        0x20 => 0x0020, // VK_SPACE   → XK_space
        // ── 符号键 ──────────────────────────────────────────────────────
        0xBB => 0x003D, // VK_OEM_PLUS         → XK_equal
        0xBD => 0x002D, // VK_OEM_MINUS        → XK_minus
        0xDB => 0x005B, // VK_OEM_4            → XK_bracketleft
        0xDD => 0x005D, // VK_OEM_6            → XK_bracketright
        0xDE => 0x0027, // VK_OEM_7            → XK_apostrophe
        0xBA => 0x003B, // VK_OEM_1            → XK_semicolon
        0xDC => 0x005C, // VK_OEM_5            → XK_backslash
        0xBC => 0x002C, // VK_OEM_COMMA        → XK_comma
        0xBF => 0x002F, // VK_OEM_2            → XK_slash
        0xBE => 0x002E, // VK_OEM_PERIOD       → XK_period
        0xC0 => 0x0060, // VK_OEM_3            → XK_grave
        // ── 方向键 ──────────────────────────────────────────────────────
        0x25 => 0xFF51, // VK_LEFT  → XK_Left
        0x26 => 0xFF52, // VK_UP    → XK_Up
        0x27 => 0xFF53, // VK_RIGHT → XK_Right
        0x28 => 0xFF54, // VK_DOWN  → XK_Down
        // ── 功能键 F1-F15 ────────────────────────────────────────────────
        0x70 => 0xFFBE, // VK_F1
        0x71 => 0xFFBF, // VK_F2
        0x72 => 0xFFC0, // VK_F3
        0x73 => 0xFFC1, // VK_F4
        0x74 => 0xFFC2, // VK_F5
        0x75 => 0xFFC3, // VK_F6
        0x76 => 0xFFC4, // VK_F7
        0x77 => 0xFFC5, // VK_F8
        0x78 => 0xFFC6, // VK_F9
        0x79 => 0xFFC7, // VK_F10
        0x7A => 0xFFC8, // VK_F11
        0x7B => 0xFFC9, // VK_F12
        0x7C => 0xFFCA, // VK_F13
        0x7D => 0xFFCB, // VK_F14
        0x7E => 0xFFCC, // VK_F15
        // ── 修饰键 ──────────────────────────────────────────────────────
        0xA0 => 0xFFE1, // VK_LSHIFT   → XK_Shift_L
        0xA1 => 0xFFE2, // VK_RSHIFT   → XK_Shift_R
        0xA2 => 0xFFE3, // VK_LCONTROL → XK_Control_L（透明模式）
        0xA3 => 0xFFE4, // VK_RCONTROL → XK_Control_R
        0x10 => 0xFFE1, // VK_SHIFT    → XK_Shift_L
        0x11 => 0xFFE3, // VK_CONTROL  → XK_Control_L
        0xA4 => 0xFFE9, // VK_LMENU    → XK_Alt_L
        0xA5 => 0xFFEA, // VK_RMENU    → XK_Alt_R
        0x12 => 0xFFE9, // VK_MENU     → XK_Alt_L
        0x5B => 0xFFEB, // VK_LWIN     → XK_Super_L (= Mac Cmd_L)
        0x5C => 0xFFEC, // VK_RWIN     → XK_Super_R (= Mac Cmd_R)
        0x14 => 0xFFE5, // VK_CAPITAL  → XK_Caps_Lock
        // ── 扩展键 ──────────────────────────────────────────────────────
        0x2D => 0xFF63, // VK_INSERT    → XK_Insert
        0x2E => 0xFFFF, // VK_DELETE    → XK_Delete
        0x24 => 0xFF50, // VK_HOME      → XK_Home
        0x23 => 0xFF57, // VK_END       → XK_End
        0x21 => 0xFF55, // VK_PRIOR     → XK_Page_Up
        0x22 => 0xFF56, // VK_NEXT      → XK_Page_Down
        // ── 数字键盘 ────────────────────────────────────────────────────
        0x60 => 0xFFB0, // VK_NUMPAD0   → XK_KP_0
        0x61 => 0xFFB1, // VK_NUMPAD1   → XK_KP_1
        0x62 => 0xFFB2, // VK_NUMPAD2   → XK_KP_2
        0x63 => 0xFFB3, // VK_NUMPAD3   → XK_KP_3
        0x64 => 0xFFB4, // VK_NUMPAD4   → XK_KP_4
        0x65 => 0xFFB5, // VK_NUMPAD5   → XK_KP_5
        0x66 => 0xFFB6, // VK_NUMPAD6   → XK_KP_6
        0x67 => 0xFFB7, // VK_NUMPAD7   → XK_KP_7
        0x68 => 0xFFB8, // VK_NUMPAD8   → XK_KP_8
        0x69 => 0xFFB9, // VK_NUMPAD9   → XK_KP_9
        0x6A => 0xFFAA, // VK_MULTIPLY  → XK_KP_Multiply
        0x6B => 0xFFAB, // VK_ADD       → XK_KP_Add
        0x6D => 0xFFAD, // VK_SUBTRACT  → XK_KP_Subtract
        0x6E => 0xFFAE, // VK_DECIMAL   → XK_KP_Decimal
        0x6F => 0xFFAF, // VK_DIVIDE    → XK_KP_Divide
        0x90 => 0xFF7F, // VK_NUMLOCK   → XK_Num_Lock
        // ── 未映射：透传 ────────────────────────────────────────────────
        other => {
            log::debug!("[KeyMap/Win→KeySym] 未知 VK {:#06X}, 透传", other);
            other
        }
    }
}

/// Mac CGKeyCode 采集时的键位统一模式转换（Mac 主控端采集时调用）
///
/// 统一模式（key_mapping_unified=true）：
///   LEFT_CMD / RIGHT_CMD → Ctrl KeySym（即发 XK_Control_L/R）
///   让 Mac 端 Cmd+C 在 Win 被控端注入为 Ctrl+C
pub fn cg_cmd_to_keysym_unified(cg_keycode: u16) -> Option<u16> {
    match cg_keycode {
        55 => Some(0xFFE3), // LEFT_CMD  → XK_Control_L
        54 => Some(0xFFE4), // RIGHT_CMD → XK_Control_R
        _ => None,
    }
}

/// 键码映射器
pub struct KeycodeMapper {
    cg_to_vk: HashMap<u16, u16>,
    vk_to_cg: HashMap<u16, u16>,
    command_mapping: CommandKeyMapping,
}

impl KeycodeMapper {
    pub fn new() -> Self {
        Self::with_command_mapping(CommandKeyMapping::Win)
    }

    pub fn with_command_mapping(command_mapping: CommandKeyMapping) -> Self {
        let mut mapper = Self {
            cg_to_vk: HashMap::new(),
            vk_to_cg: HashMap::new(),
            command_mapping,
        };
        mapper.build_tables();
        mapper
    }

    /// 将 CGKeyCode 转换为 Windows VirtualKey
    pub fn cg_to_vk(&self, cg_keycode: u16) -> Option<u16> {
        self.cg_to_vk.get(&cg_keycode).copied()
    }

    /// 将 Windows VirtualKey 转换为 CGKeyCode
    pub fn vk_to_cg(&self, vk_keycode: u16) -> Option<u16> {
        self.vk_to_cg.get(&vk_keycode).copied()
    }

    fn build_tables(&mut self) {
        let letter_pairs = [
            (cg_key::A, vk::KEY_A), (cg_key::B, vk::KEY_B), (cg_key::C, vk::KEY_C),
            (cg_key::D, vk::KEY_D), (cg_key::E, vk::KEY_E), (cg_key::F, vk::KEY_F),
            (cg_key::G, vk::KEY_G), (cg_key::H, vk::KEY_H), (cg_key::I, vk::KEY_I),
            (cg_key::J, vk::KEY_J), (cg_key::K, vk::KEY_K), (cg_key::L, vk::KEY_L),
            (cg_key::M, vk::KEY_M), (cg_key::N, vk::KEY_N), (cg_key::O, vk::KEY_O),
            (cg_key::P, vk::KEY_P), (cg_key::Q, vk::KEY_Q), (cg_key::R, vk::KEY_R),
            (cg_key::S, vk::KEY_S), (cg_key::T, vk::KEY_T), (cg_key::U, vk::KEY_U),
            (cg_key::V, vk::KEY_V), (cg_key::W, vk::KEY_W), (cg_key::X, vk::KEY_X),
            (cg_key::Y, vk::KEY_Y), (cg_key::Z, vk::KEY_Z),
        ];

        let digit_pairs = [
            (cg_key::NUM_0, vk::KEY_0), (cg_key::NUM_1, vk::KEY_1),
            (cg_key::NUM_2, vk::KEY_2), (cg_key::NUM_3, vk::KEY_3),
            (cg_key::NUM_4, vk::KEY_4), (cg_key::NUM_5, vk::KEY_5),
            (cg_key::NUM_6, vk::KEY_6), (cg_key::NUM_7, vk::KEY_7),
            (cg_key::NUM_8, vk::KEY_8), (cg_key::NUM_9, vk::KEY_9),
        ];

        let fn_pairs = [
            (cg_key::F1, vk::F1), (cg_key::F2, vk::F2), (cg_key::F3, vk::F3),
            (cg_key::F4, vk::F4), (cg_key::F5, vk::F5), (cg_key::F6, vk::F6),
            (cg_key::F7, vk::F7), (cg_key::F8, vk::F8), (cg_key::F9, vk::F9),
            (cg_key::F10, vk::F10), (cg_key::F11, vk::F11), (cg_key::F12, vk::F12),
            (cg_key::F13, vk::F13), (cg_key::F14, vk::F14), (cg_key::F15, vk::F15),
            (cg_key::F16, vk::F16), (cg_key::F17, vk::F17), (cg_key::F18, vk::F18),
            (cg_key::F19, vk::F19), (cg_key::F20, vk::F20),
        ];

        let arrow_pairs = [
            (cg_key::LEFT_ARROW, vk::LEFT), (cg_key::RIGHT_ARROW, vk::RIGHT),
            (cg_key::UP_ARROW, vk::UP), (cg_key::DOWN_ARROW, vk::DOWN),
        ];

        let special_pairs = [
            (cg_key::RETURN, vk::RETURN),
            (cg_key::DELETE, vk::BACK),
            (cg_key::FORWARD_DELETE, vk::DELETE),
            (cg_key::TAB, vk::TAB),
            (cg_key::ESCAPE, vk::ESCAPE),
            (cg_key::SPACE, vk::SPACE),
            (cg_key::HOME, vk::HOME),
            (cg_key::END, vk::END),
            (cg_key::PAGE_UP, vk::PRIOR),
            (cg_key::PAGE_DOWN, vk::NEXT),
            (cg_key::CAPS_LOCK, vk::CAPITAL),
        ];

        let modifier_pairs = [
            (cg_key::LEFT_SHIFT, vk::LSHIFT),
            (cg_key::RIGHT_SHIFT, vk::RSHIFT),
            (cg_key::LEFT_CTRL, vk::LCONTROL),
            (cg_key::RIGHT_CTRL, vk::RCONTROL),
            (cg_key::LEFT_ALT, vk::LMENU),
            (cg_key::RIGHT_ALT, vk::RMENU),
        ];

        let symbol_pairs = [
            (cg_key::SEMICOLON, vk::OEM_SEMICOLON),
            (cg_key::EQUAL, vk::OEM_PLUS),
            (cg_key::COMMA, vk::OEM_COMMA),
            (cg_key::MINUS, vk::OEM_MINUS),
            (cg_key::PERIOD, vk::OEM_PERIOD),
            (cg_key::SLASH, vk::OEM_SLASH),
            (cg_key::GRAVE, vk::OEM_GRAVE),
            (cg_key::LEFT_BRACKET, vk::OEM_OPEN_BRACKET),
            (cg_key::BACKSLASH, vk::OEM_BACKSLASH),
            (cg_key::RIGHT_BRACKET, vk::OEM_CLOSE_BRACKET),
            (cg_key::QUOTE, vk::OEM_QUOTE),
        ];

        let numpad_pairs = [
            (cg_key::KEYPAD_0, vk::NUMPAD0), (cg_key::KEYPAD_1, vk::NUMPAD1),
            (cg_key::KEYPAD_2, vk::NUMPAD2), (cg_key::KEYPAD_3, vk::NUMPAD3),
            (cg_key::KEYPAD_4, vk::NUMPAD4), (cg_key::KEYPAD_5, vk::NUMPAD5),
            (cg_key::KEYPAD_6, vk::NUMPAD6), (cg_key::KEYPAD_7, vk::NUMPAD7),
            (cg_key::KEYPAD_8, vk::NUMPAD8), (cg_key::KEYPAD_9, vk::NUMPAD9),
            (cg_key::KEYPAD_MULTIPLY, vk::MULTIPLY),
            (cg_key::KEYPAD_PLUS, vk::ADD),
            (cg_key::KEYPAD_MINUS, vk::SUBTRACT),
            (cg_key::KEYPAD_DECIMAL, vk::DECIMAL),
            (cg_key::KEYPAD_DIVIDE, vk::DIVIDE),
            (cg_key::KEYPAD_ENTER, vk::RETURN),
        ];

        for pairs in [
            letter_pairs.as_slice(), digit_pairs.as_slice(), fn_pairs.as_slice(),
            arrow_pairs.as_slice(), special_pairs.as_slice(), modifier_pairs.as_slice(),
            symbol_pairs.as_slice(), numpad_pairs.as_slice(),
        ] {
            for &(cg, vk_code) in pairs {
                self.cg_to_vk.insert(cg, vk_code);
                self.vk_to_cg.insert(vk_code, cg);
            }
        }

        // Command 键映射
        let cmd_vk = match self.command_mapping {
            CommandKeyMapping::Win => vk::LWIN,
            CommandKeyMapping::Ctrl => vk::LCONTROL,
        };
        self.cg_to_vk.insert(cg_key::LEFT_CMD, cmd_vk);
        self.cg_to_vk.insert(cg_key::RIGHT_CMD, match self.command_mapping {
            CommandKeyMapping::Win => vk::RWIN,
            CommandKeyMapping::Ctrl => vk::RCONTROL,
        });
        self.vk_to_cg.insert(vk::LWIN, cg_key::LEFT_CMD);
        self.vk_to_cg.insert(vk::RWIN, cg_key::RIGHT_CMD);
    }
}

impl Default for KeycodeMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_letters_bidirectional() {
        let m = KeycodeMapper::new();
        let pairs = [
            (cg_key::A, vk::KEY_A), (cg_key::Z, vk::KEY_Z),
            (cg_key::M, vk::KEY_M),
        ];
        for (cg, vk_code) in pairs {
            assert_eq!(m.cg_to_vk(cg), Some(vk_code));
            assert_eq!(m.vk_to_cg(vk_code), Some(cg));
        }
    }

    #[test]
    fn test_command_key_default_maps_to_win() {
        let m = KeycodeMapper::new();
        assert_eq!(m.cg_to_vk(cg_key::LEFT_CMD), Some(vk::LWIN));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_CMD), Some(vk::RWIN));
    }

    #[test]
    fn test_command_key_maps_to_ctrl() {
        let m = KeycodeMapper::with_command_mapping(CommandKeyMapping::Ctrl);
        assert_eq!(m.cg_to_vk(cg_key::LEFT_CMD), Some(vk::LCONTROL));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_CMD), Some(vk::RCONTROL));
    }

    #[test]
    fn test_unknown_keycode_returns_none() {
        let m = KeycodeMapper::new();
        assert_eq!(m.cg_to_vk(0xFFFF), None);
        assert_eq!(m.vk_to_cg(0xFF), None);
    }

    #[test]
    fn test_special_keys() {
        let m = KeycodeMapper::new();
        assert_eq!(m.cg_to_vk(cg_key::DELETE), Some(vk::BACK));
        assert_eq!(m.cg_to_vk(cg_key::FORWARD_DELETE), Some(vk::DELETE));
        assert_eq!(m.cg_to_vk(cg_key::ESCAPE), Some(vk::ESCAPE));
    }

    // ── vk_to_keysym 双向完整性测试 ────────────────────────────────────────

    #[test]
    fn test_vk_to_keysym_alpha_transparent() {
        // 透明模式：字母键 VK→KeySym
        assert_eq!(vk_to_keysym(0x41, false), 0x0061); // VK_A → XK_a
        assert_eq!(vk_to_keysym(0x5A, false), 0x007A); // VK_Z → XK_z
        assert_eq!(vk_to_keysym(0x43, false), 0x0063); // VK_C → XK_c
    }

    #[test]
    fn test_vk_to_keysym_special_transparent() {
        assert_eq!(vk_to_keysym(0x08, false), 0xFF08); // VK_BACK  → XK_BackSpace
        assert_eq!(vk_to_keysym(0x0D, false), 0xFF0D); // VK_RETURN → XK_Return
        assert_eq!(vk_to_keysym(0x20, false), 0x0020); // VK_SPACE → XK_space
        assert_eq!(vk_to_keysym(0x1B, false), 0xFF1B); // VK_ESCAPE → XK_Escape
    }

    #[test]
    fn test_vk_to_keysym_modifiers_transparent() {
        // 透明模式：Ctrl→Ctrl，Win键→Super
        assert_eq!(vk_to_keysym(0xA2, false), 0xFFE3); // VK_LCONTROL → XK_Control_L
        assert_eq!(vk_to_keysym(0x5B, false), 0xFFEB); // VK_LWIN → XK_Super_L
        assert_eq!(vk_to_keysym(0xA0, false), 0xFFE1); // VK_LSHIFT → XK_Shift_L
        assert_eq!(vk_to_keysym(0xA4, false), 0xFFE9); // VK_LMENU → XK_Alt_L
    }

    #[test]
    fn test_vk_to_keysym_unified_ctrl_becomes_cmd() {
        // 统一模式：Ctrl → Cmd(Super)
        assert_eq!(vk_to_keysym(0xA2, true), 0xFFEB); // VK_LCONTROL → XK_Super_L
        assert_eq!(vk_to_keysym(0xA3, true), 0xFFEC); // VK_RCONTROL → XK_Super_R
        assert_eq!(vk_to_keysym(0x11, true), 0xFFEB); // VK_CONTROL  → XK_Super_L
    }

    #[test]
    fn test_vk_to_keysym_unified_other_keys_unchanged() {
        // 统一模式：其他键不变
        assert_eq!(vk_to_keysym(0x41, true), 0x0061); // VK_A 不变
        assert_eq!(vk_to_keysym(0x5B, true), 0xFFEB); // VK_LWIN → XK_Super_L
        assert_eq!(vk_to_keysym(0xA0, true), 0xFFE1); // VK_LSHIFT 不变
    }

    #[test]
    fn test_cg_cmd_to_keysym_unified() {
        // Mac 统一模式：Cmd→Ctrl
        assert_eq!(cg_cmd_to_keysym_unified(55), Some(0xFFE3)); // LEFT_CMD → XK_Control_L
        assert_eq!(cg_cmd_to_keysym_unified(54), Some(0xFFE4)); // RIGHT_CMD → XK_Control_R
        assert_eq!(cg_cmd_to_keysym_unified(0), None);           // 非 Cmd 键
    }

    #[test]
    fn test_vk_to_keysym_arrows() {
        assert_eq!(vk_to_keysym(0x25, false), 0xFF51); // VK_LEFT → XK_Left
        assert_eq!(vk_to_keysym(0x26, false), 0xFF52); // VK_UP   → XK_Up
        assert_eq!(vk_to_keysym(0x27, false), 0xFF53); // VK_RIGHT → XK_Right
        assert_eq!(vk_to_keysym(0x28, false), 0xFF54); // VK_DOWN  → XK_Down
    }

    #[test]
    fn test_vk_to_keysym_fn_keys() {
        assert_eq!(vk_to_keysym(0x70, false), 0xFFBE); // VK_F1 → XK_F1
        assert_eq!(vk_to_keysym(0x7B, false), 0xFFC9); // VK_F12 → XK_F12
    }
}
