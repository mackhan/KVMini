/// 跨平台键码映射模块
///
/// 实现 macOS CGKeyCode ↔ Windows VirtualKey 双向映射。
/// 当 Mac 主控端发送 KeyEvent 时，Windows 被控端通过此映射表转换键码。
/// 反之亦然。
///
/// 平台标识通过 KvmFrame::KeyEvent 中的 platform 字段携带。

use std::collections::HashMap;

/// macOS CGKeyCode（部分常用键）
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
    pub const DELETE: u16 = 51; // Backspace
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

/// Windows Virtual Key Codes（部分常用键）
#[allow(dead_code)]
pub mod vk {
    pub const BACK: u16 = 0x08;    // Backspace
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;  // Enter
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;    // Alt
    pub const CAPITAL: u16 = 0x14; // Caps Lock
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21;   // Page Up
    pub const NEXT: u16 = 0x22;    // Page Down
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
    pub const LWIN: u16 = 0x5B;    // Left Windows key
    pub const RWIN: u16 = 0x5C;    // Right Windows key
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
    pub const LMENU: u16 = 0xA4;   // Left Alt
    pub const RMENU: u16 = 0xA5;   // Right Alt
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

/// 键码映射器
pub struct KeycodeMapper {
    /// CGKeyCode → VirtualKey 映射表
    cg_to_vk: HashMap<u16, u16>,
    /// VirtualKey → CGKeyCode 映射表
    vk_to_cg: HashMap<u16, u16>,
    /// Command 键映射模式
    command_mapping: CommandKeyMapping,
}

impl KeycodeMapper {
    /// 创建默认映射器（Command → Win）
    pub fn new() -> Self {
        Self::with_command_mapping(CommandKeyMapping::Win)
    }

    /// 创建指定 Command 键映射模式的映射器
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
    /// 返回 None 表示映射表中不存在（调用方应丢弃该事件并记录警告）
    pub fn cg_to_vk(&self, cg_keycode: u16) -> Option<u16> {
        self.cg_to_vk.get(&cg_keycode).copied()
    }

    /// 将 Windows VirtualKey 转换为 CGKeyCode
    /// 返回 None 表示映射表中不存在
    pub fn vk_to_cg(&self, vk_keycode: u16) -> Option<u16> {
        self.vk_to_cg.get(&vk_keycode).copied()
    }

    /// 构建双向映射表
    fn build_tables(&mut self) {
        // 字母键 A-Z
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

        // 数字键 0-9
        let digit_pairs = [
            (cg_key::NUM_0, vk::KEY_0), (cg_key::NUM_1, vk::KEY_1),
            (cg_key::NUM_2, vk::KEY_2), (cg_key::NUM_3, vk::KEY_3),
            (cg_key::NUM_4, vk::KEY_4), (cg_key::NUM_5, vk::KEY_5),
            (cg_key::NUM_6, vk::KEY_6), (cg_key::NUM_7, vk::KEY_7),
            (cg_key::NUM_8, vk::KEY_8), (cg_key::NUM_9, vk::KEY_9),
        ];

        // 功能键 F1-F20
        let fn_pairs = [
            (cg_key::F1, vk::F1), (cg_key::F2, vk::F2), (cg_key::F3, vk::F3),
            (cg_key::F4, vk::F4), (cg_key::F5, vk::F5), (cg_key::F6, vk::F6),
            (cg_key::F7, vk::F7), (cg_key::F8, vk::F8), (cg_key::F9, vk::F9),
            (cg_key::F10, vk::F10), (cg_key::F11, vk::F11), (cg_key::F12, vk::F12),
            (cg_key::F13, vk::F13), (cg_key::F14, vk::F14), (cg_key::F15, vk::F15),
            (cg_key::F16, vk::F16), (cg_key::F17, vk::F17), (cg_key::F18, vk::F18),
            (cg_key::F19, vk::F19), (cg_key::F20, vk::F20),
        ];

        // 方向键
        let arrow_pairs = [
            (cg_key::LEFT_ARROW, vk::LEFT), (cg_key::RIGHT_ARROW, vk::RIGHT),
            (cg_key::UP_ARROW, vk::UP), (cg_key::DOWN_ARROW, vk::DOWN),
        ];

        // 特殊键
        let special_pairs = [
            (cg_key::RETURN, vk::RETURN),
            (cg_key::DELETE, vk::BACK),           // macOS Delete = Windows Backspace
            (cg_key::FORWARD_DELETE, vk::DELETE), // macOS Forward Delete = Windows Delete
            (cg_key::TAB, vk::TAB),
            (cg_key::ESCAPE, vk::ESCAPE),
            (cg_key::SPACE, vk::SPACE),
            (cg_key::HOME, vk::HOME),
            (cg_key::END, vk::END),
            (cg_key::PAGE_UP, vk::PRIOR),
            (cg_key::PAGE_DOWN, vk::NEXT),
            (cg_key::CAPS_LOCK, vk::CAPITAL),
        ];

        // 修饰键
        let modifier_pairs = [
            (cg_key::LEFT_SHIFT, vk::LSHIFT),
            (cg_key::RIGHT_SHIFT, vk::RSHIFT),
            (cg_key::LEFT_CTRL, vk::LCONTROL),
            (cg_key::RIGHT_CTRL, vk::RCONTROL),
            (cg_key::LEFT_ALT, vk::LMENU),
            (cg_key::RIGHT_ALT, vk::RMENU),
        ];

        // 符号键
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

        // 小键盘
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

        // 插入所有映射对
        for pairs in [
            letter_pairs.as_slice(),
            digit_pairs.as_slice(),
            fn_pairs.as_slice(),
            arrow_pairs.as_slice(),
            special_pairs.as_slice(),
            modifier_pairs.as_slice(),
            symbol_pairs.as_slice(),
            numpad_pairs.as_slice(),
        ] {
            for &(cg, vk_code) in pairs {
                self.cg_to_vk.insert(cg, vk_code);
                self.vk_to_cg.insert(vk_code, cg);
            }
        }

        // Command 键映射（根据配置）
        let cmd_vk = match self.command_mapping {
            CommandKeyMapping::Win => vk::LWIN,
            CommandKeyMapping::Ctrl => vk::LCONTROL,
        };
        self.cg_to_vk.insert(cg_key::LEFT_CMD, cmd_vk);
        self.cg_to_vk.insert(cg_key::RIGHT_CMD, match self.command_mapping {
            CommandKeyMapping::Win => vk::RWIN,
            CommandKeyMapping::Ctrl => vk::RCONTROL,
        });
        // 反向映射：Win → Command
        self.vk_to_cg.insert(vk::LWIN, cg_key::LEFT_CMD);
        self.vk_to_cg.insert(vk::RWIN, cg_key::RIGHT_CMD);
    }
}

impl Default for KeycodeMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局单例映射器（使用 lazy_static）
lazy_static::lazy_static! {
    static ref GLOBAL_MAPPER: std::sync::RwLock<KeycodeMapper> =
        std::sync::RwLock::new(KeycodeMapper::new());
}

/// 更新全局映射器的 Command 键映射模式
pub fn set_command_key_mapping(mode: CommandKeyMapping) {
    let mut mapper = GLOBAL_MAPPER.write().unwrap();
    *mapper = KeycodeMapper::with_command_mapping(mode);
}

/// 将 CGKeyCode 转换为 VirtualKey（使用全局映射器）
/// 返回 None 时调用方应丢弃事件并记录警告
pub fn cg_to_vk(cg_keycode: u16) -> Option<u16> {
    GLOBAL_MAPPER.read().unwrap().cg_to_vk(cg_keycode)
}

/// 将 VirtualKey 转换为 CGKeyCode（使用全局映射器）
pub fn vk_to_cg(vk_keycode: u16) -> Option<u16> {
    GLOBAL_MAPPER.read().unwrap().vk_to_cg(vk_keycode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> KeycodeMapper {
        KeycodeMapper::new()
    }

    // ── 字母键映射测试 ───────────────────────────────────────────────────

    #[test]
    fn test_cg_to_vk_letter_a() {
        let m = mapper();
        assert_eq!(m.cg_to_vk(cg_key::A), Some(vk::KEY_A));
    }

    #[test]
    fn test_cg_to_vk_letter_z() {
        let m = mapper();
        assert_eq!(m.cg_to_vk(cg_key::Z), Some(vk::KEY_Z));
    }

    #[test]
    fn test_vk_to_cg_letter_a() {
        let m = mapper();
        assert_eq!(m.vk_to_cg(vk::KEY_A), Some(cg_key::A));
    }

    #[test]
    fn test_all_letters_bidirectional() {
        let m = mapper();
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
        for (cg, vk_code) in letter_pairs {
            assert_eq!(m.cg_to_vk(cg), Some(vk_code), "CGKey {} -> VK {:#04X} 失败", cg, vk_code);
            assert_eq!(m.vk_to_cg(vk_code), Some(cg), "VK {:#04X} -> CGKey {} 失败", vk_code, cg);
        }
    }

    // ── 数字键映射测试 ───────────────────────────────────────────────────

    #[test]
    fn test_all_digits_bidirectional() {
        let m = mapper();
        let digit_pairs = [
            (cg_key::NUM_0, vk::KEY_0), (cg_key::NUM_1, vk::KEY_1),
            (cg_key::NUM_2, vk::KEY_2), (cg_key::NUM_3, vk::KEY_3),
            (cg_key::NUM_4, vk::KEY_4), (cg_key::NUM_5, vk::KEY_5),
            (cg_key::NUM_6, vk::KEY_6), (cg_key::NUM_7, vk::KEY_7),
            (cg_key::NUM_8, vk::KEY_8), (cg_key::NUM_9, vk::KEY_9),
        ];
        for (cg, vk_code) in digit_pairs {
            assert_eq!(m.cg_to_vk(cg), Some(vk_code));
            assert_eq!(m.vk_to_cg(vk_code), Some(cg));
        }
    }

    // ── 功能键映射测试 ───────────────────────────────────────────────────

    #[test]
    fn test_function_keys_f1_to_f12() {
        let m = mapper();
        let fn_pairs = [
            (cg_key::F1, vk::F1), (cg_key::F2, vk::F2), (cg_key::F3, vk::F3),
            (cg_key::F4, vk::F4), (cg_key::F5, vk::F5), (cg_key::F6, vk::F6),
            (cg_key::F7, vk::F7), (cg_key::F8, vk::F8), (cg_key::F9, vk::F9),
            (cg_key::F10, vk::F10), (cg_key::F11, vk::F11), (cg_key::F12, vk::F12),
        ];
        for (cg, vk_code) in fn_pairs {
            assert_eq!(m.cg_to_vk(cg), Some(vk_code), "F键映射失败: CG={} VK={:#04X}", cg, vk_code);
        }
    }

    // ── 方向键映射测试 ───────────────────────────────────────────────────

    #[test]
    fn test_arrow_keys() {
        let m = mapper();
        assert_eq!(m.cg_to_vk(cg_key::LEFT_ARROW), Some(vk::LEFT));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_ARROW), Some(vk::RIGHT));
        assert_eq!(m.cg_to_vk(cg_key::UP_ARROW), Some(vk::UP));
        assert_eq!(m.cg_to_vk(cg_key::DOWN_ARROW), Some(vk::DOWN));
    }

    // ── 特殊键映射测试 ───────────────────────────────────────────────────

    #[test]
    fn test_special_keys() {
        let m = mapper();
        assert_eq!(m.cg_to_vk(cg_key::RETURN), Some(vk::RETURN));
        assert_eq!(m.cg_to_vk(cg_key::DELETE), Some(vk::BACK));       // macOS Delete = Backspace
        assert_eq!(m.cg_to_vk(cg_key::FORWARD_DELETE), Some(vk::DELETE));
        assert_eq!(m.cg_to_vk(cg_key::TAB), Some(vk::TAB));
        assert_eq!(m.cg_to_vk(cg_key::ESCAPE), Some(vk::ESCAPE));
        assert_eq!(m.cg_to_vk(cg_key::SPACE), Some(vk::SPACE));
    }

    // ── 修饰键映射测试 ───────────────────────────────────────────────────

    #[test]
    fn test_modifier_keys() {
        let m = mapper();
        assert_eq!(m.cg_to_vk(cg_key::LEFT_CTRL), Some(vk::LCONTROL));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_CTRL), Some(vk::RCONTROL));
        assert_eq!(m.cg_to_vk(cg_key::LEFT_ALT), Some(vk::LMENU));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_ALT), Some(vk::RMENU));
        assert_eq!(m.cg_to_vk(cg_key::LEFT_SHIFT), Some(vk::LSHIFT));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_SHIFT), Some(vk::RSHIFT));
    }

    // ── Command 键映射测试 ───────────────────────────────────────────────

    #[test]
    fn test_command_key_maps_to_win_by_default() {
        let m = KeycodeMapper::new();
        assert_eq!(m.cg_to_vk(cg_key::LEFT_CMD), Some(vk::LWIN));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_CMD), Some(vk::RWIN));
    }

    #[test]
    fn test_command_key_maps_to_ctrl_when_configured() {
        let m = KeycodeMapper::with_command_mapping(CommandKeyMapping::Ctrl);
        assert_eq!(m.cg_to_vk(cg_key::LEFT_CMD), Some(vk::LCONTROL));
        assert_eq!(m.cg_to_vk(cg_key::RIGHT_CMD), Some(vk::RCONTROL));
    }

    #[test]
    fn test_win_key_maps_to_command() {
        let m = mapper();
        assert_eq!(m.vk_to_cg(vk::LWIN), Some(cg_key::LEFT_CMD));
        assert_eq!(m.vk_to_cg(vk::RWIN), Some(cg_key::RIGHT_CMD));
    }

    // ── 未知键码测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_unknown_cg_keycode_returns_none() {
        let m = mapper();
        // 0xFFFF 不在映射表中
        assert_eq!(m.cg_to_vk(0xFFFF), None);
    }

    #[test]
    fn test_unknown_vk_keycode_returns_none() {
        let m = mapper();
        // 0xFF 不在映射表中
        assert_eq!(m.vk_to_cg(0xFF), None);
    }

    // ── 全局函数测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_global_cg_to_vk() {
        assert_eq!(cg_to_vk(cg_key::A), Some(vk::KEY_A));
        assert_eq!(cg_to_vk(0xFFFF), None);
    }

    #[test]
    fn test_global_vk_to_cg() {
        assert_eq!(vk_to_cg(vk::KEY_A), Some(cg_key::A));
        assert_eq!(vk_to_cg(0xFF), None);
    }

    #[test]
    fn test_set_command_key_mapping() {
        // 先设置为 Ctrl 模式
        set_command_key_mapping(CommandKeyMapping::Ctrl);
        assert_eq!(cg_to_vk(cg_key::LEFT_CMD), Some(vk::LCONTROL));

        // 恢复为 Win 模式
        set_command_key_mapping(CommandKeyMapping::Win);
        assert_eq!(cg_to_vk(cg_key::LEFT_CMD), Some(vk::LWIN));
    }

    // ── 映射完整性测试 ───────────────────────────────────────────────────

    #[test]
    fn test_mapper_has_all_26_letters() {
        let m = mapper();
        let cg_letters = [
            cg_key::A, cg_key::B, cg_key::C, cg_key::D, cg_key::E,
            cg_key::F, cg_key::G, cg_key::H, cg_key::I, cg_key::J,
            cg_key::K, cg_key::L, cg_key::M, cg_key::N, cg_key::O,
            cg_key::P, cg_key::Q, cg_key::R, cg_key::S, cg_key::T,
            cg_key::U, cg_key::V, cg_key::W, cg_key::X, cg_key::Y, cg_key::Z,
        ];
        for cg in cg_letters {
            assert!(m.cg_to_vk(cg).is_some(), "字母键 CGKey={} 缺少映射", cg);
        }
    }

    #[test]
    fn test_mapper_has_all_10_digits() {
        let m = mapper();
        let cg_digits = [
            cg_key::NUM_0, cg_key::NUM_1, cg_key::NUM_2, cg_key::NUM_3, cg_key::NUM_4,
            cg_key::NUM_5, cg_key::NUM_6, cg_key::NUM_7, cg_key::NUM_8, cg_key::NUM_9,
        ];
        for cg in cg_digits {
            assert!(m.cg_to_vk(cg).is_some(), "数字键 CGKey={} 缺少映射", cg);
        }
    }

    #[test]
    fn test_mapper_has_f1_to_f12() {
        let m = mapper();
        let fn_keys = [
            cg_key::F1, cg_key::F2, cg_key::F3, cg_key::F4,
            cg_key::F5, cg_key::F6, cg_key::F7, cg_key::F8,
            cg_key::F9, cg_key::F10, cg_key::F11, cg_key::F12,
        ];
        for cg in fn_keys {
            assert!(m.cg_to_vk(cg).is_some(), "功能键 CGKey={} 缺少映射", cg);
        }
    }
}
