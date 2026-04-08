/// Mac 端输入事件执行器
/// 仅在 macOS 上编译，使用 CGEvent API 模拟鼠标和键盘操作
use crate::protocol::KvmFrame;

// ============================================================
// 辅助功能权限检测（仅 macOS）
// ============================================================

/// 检测是否已获得辅助功能权限
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    use std::process::Command;
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of every process")
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    false
}

/// 打印辅助功能权限引导文案
pub fn print_accessibility_guide() {
    eprintln!("========================================");
    eprintln!("⚠️  KVM Agent 需要辅助功能权限才能控制鼠标和键盘");
    eprintln!();
    eprintln!("请按以下步骤授权：");
    eprintln!("1. 打开「系统设置」（System Settings）");
    eprintln!("2. 点击「隐私与安全性」（Privacy & Security）");
    eprintln!("3. 点击「辅助功能」（Accessibility）");
    eprintln!("4. 找到「kvm-agent」并开启开关");
    eprintln!("5. 重新启动 kvm-agent");
    eprintln!("========================================");
}

// ============================================================
// Windows keycode → macOS 虚拟键码映射表
// ============================================================

/// 将 Windows 虚拟键码（VK_*）转换为 macOS CGKeyCode
pub fn win_keycode_to_mac(win_vk: u16) -> Option<u16> {
    let mac_code = match win_vk {
        // 字母键 A-Z
        0x41 => 0x00, // A
        0x42 => 0x0B, // B
        0x43 => 0x08, // C
        0x44 => 0x02, // D
        0x45 => 0x0E, // E
        0x46 => 0x03, // F
        0x47 => 0x05, // G
        0x48 => 0x04, // H
        0x49 => 0x22, // I
        0x4A => 0x26, // J
        0x4B => 0x28, // K
        0x4C => 0x25, // L
        0x4D => 0x2E, // M
        0x4E => 0x2D, // N
        0x4F => 0x1F, // O
        0x50 => 0x23, // P
        0x51 => 0x0C, // Q
        0x52 => 0x0F, // R
        0x53 => 0x01, // S
        0x54 => 0x11, // T
        0x55 => 0x20, // U
        0x56 => 0x09, // V
        0x57 => 0x0D, // W
        0x58 => 0x07, // X
        0x59 => 0x10, // Y
        0x5A => 0x06, // Z
        // 数字键 0-9
        0x30 => 0x1D, // 0
        0x31 => 0x12, // 1
        0x32 => 0x13, // 2
        0x33 => 0x14, // 3
        0x34 => 0x15, // 4
        0x35 => 0x17, // 5
        0x36 => 0x16, // 6
        0x37 => 0x1A, // 7
        0x38 => 0x1C, // 8
        0x39 => 0x19, // 9
        // 功能键
        0x70 => 0x7A, // F1
        0x71 => 0x78, // F2
        0x72 => 0x63, // F3
        0x73 => 0x76, // F4
        0x74 => 0x60, // F5
        0x75 => 0x61, // F6
        0x76 => 0x62, // F7
        0x77 => 0x64, // F8
        0x78 => 0x65, // F9
        0x79 => 0x6D, // F10
        0x7A => 0x67, // F11
        0x7B => 0x6F, // F12
        // 控制键
        0x08 => 0x33, // Backspace → Delete
        0x09 => 0x30, // Tab
        0x0D => 0x24, // Enter
        0x1B => 0x35, // Escape
        0x20 => 0x31, // Space
        0x25 => 0x7B, // Left Arrow
        0x26 => 0x7E, // Up Arrow
        0x27 => 0x7C, // Right Arrow
        0x28 => 0x7D, // Down Arrow
        0x2E => 0x75, // Delete (Forward)
        0x24 => 0x73, // Home
        0x23 => 0x77, // End
        0x21 => 0x74, // Page Up
        0x22 => 0x79, // Page Down
        // 修饰键
        0x10 => 0x38, // Shift (Left)
        0x11 => 0x3B, // Ctrl (Left)
        0x12 => 0x3A, // Alt/Option (Left)
        0x5B => 0x37, // Win/Command (Left)
        0xA0 => 0x38, // Left Shift
        0xA1 => 0x3C, // Right Shift
        0xA2 => 0x3B, // Left Ctrl
        0xA3 => 0x3E, // Right Ctrl
        0xA4 => 0x3A, // Left Alt
        0xA5 => 0x3D, // Right Alt
        // 符号键
        0xBB => 0x18, // = / +
        0xBD => 0x1B, // - / _
        0xDB => 0x21, // [ / {
        0xDD => 0x1E, // ] / }
        0xDC => 0x2A, // \ / |
        0xBA => 0x29, // ; / :
        0xDE => 0x27, // ' / "
        0xBC => 0x2B, // , / <
        0xBE => 0x2F, // . / >
        0xBF => 0x2C, // / / ?
        0xC0 => 0x32, // ` / ~
        _ => return None,
    };
    Some(mac_code)
}

// ============================================================
// 输入事件执行（仅 macOS）
// ============================================================

#[cfg(target_os = "macos")]
mod macos_input {
    use super::*;
    use core_graphics::event::{
        CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, ScrollEventUnit,
    };
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    /// 执行二进制帧输入事件
    pub fn execute_frame(frame: &KvmFrame) {
        let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[Input] 创建 CGEventSource 失败");
                return;
            }
        };

        match frame {
            KvmFrame::MouseMove { x, y } => {
                // 坐标为归一化值 * 10000，需要转换为屏幕像素坐标
                // 获取屏幕分辨率并还原实际坐标
                let (screen_w, screen_h) = get_screen_size();
                let px = (*x as f64 / 10000.0) * screen_w;
                let py = (*y as f64 / 10000.0) * screen_h;
                let point = CGPoint::new(px, py);
                if let Ok(ev) = CGEvent::new_mouse_event(
                    source,
                    CGEventType::MouseMoved,
                    point,
                    CGMouseButton::Left,
                ) {
                    ev.post(CGEventTapLocation::HID);
                }
            }
            KvmFrame::MouseButton { button, pressed } => {
                let pos = get_current_mouse_pos();
                let (ev_type, cg_btn) = if *pressed {
                    mouse_button_to_cg_down(*button)
                } else {
                    mouse_button_to_cg_up(*button)
                };
                if let Ok(ev) = CGEvent::new_mouse_event(source, ev_type, pos, cg_btn) {
                    ev.post(CGEventTapLocation::HID);
                }
            }
            KvmFrame::MouseScroll { dx, dy } => {
                if let Ok(ev) = CGEvent::new_scroll_event(
                    source,
                    ScrollEventUnit::Pixel,
                    2,
                    *dy,
                    *dx,
                    0,
                ) {
                    ev.post(CGEventTapLocation::HID);
                }
            }
            KvmFrame::KeyEvent { keycode, pressed } => {
                if let Some(mac_code) = win_keycode_to_mac(*keycode) {
                    if let Ok(ev) = CGEvent::new_keyboard_event(source, mac_code, *pressed) {
                        ev.post(CGEventTapLocation::HID);
                    }
                } else {
                    eprintln!("[Input] 未知 keycode: 0x{:04X}", keycode);
                }
            }
            KvmFrame::Heartbeat => {
                // 心跳帧不执行输入操作，仅更新时间戳（由 server.rs 处理）
            }
            other => {
                eprintln!("[Input] 收到非输入帧，忽略: {:?}", other);
            }
        }
    }

    fn get_screen_size() -> (f64, f64) {
        use core_graphics::display::CGDisplay;
        let display = CGDisplay::main();
        let bounds = display.bounds();
        (bounds.size.width, bounds.size.height)
    }

    fn get_current_mouse_pos() -> CGPoint {
        CGPoint::new(0.0, 0.0)
    }

    fn mouse_button_to_cg_down(btn: u8) -> (CGEventType, CGMouseButton) {
        match btn {
            0 => (CGEventType::LeftMouseDown, CGMouseButton::Left),
            1 => (CGEventType::RightMouseDown, CGMouseButton::Right),
            2 => (CGEventType::OtherMouseDown, CGMouseButton::Center),
            _ => (CGEventType::OtherMouseDown, CGMouseButton::Center),
        }
    }

    fn mouse_button_to_cg_up(btn: u8) -> (CGEventType, CGMouseButton) {
        match btn {
            0 => (CGEventType::LeftMouseUp, CGMouseButton::Left),
            1 => (CGEventType::RightMouseUp, CGMouseButton::Right),
            2 => (CGEventType::OtherMouseUp, CGMouseButton::Center),
            _ => (CGEventType::OtherMouseUp, CGMouseButton::Center),
        }
    }
}

/// 公开的帧输入执行入口（平台隔离）
#[cfg(target_os = "macos")]
pub use macos_input::execute_frame;

/// 非 macOS 平台的空实现（编译占位）
#[cfg(not(target_os = "macos"))]
pub fn execute_frame(frame: &KvmFrame) {
    eprintln!("[Input] 当前平台不支持输入事件执行（仅 macOS）: {:?}", frame);
}
