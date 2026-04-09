//! Windows 全局快捷键实现
//!
//! 使用 RegisterHotKey Win32 API 注册全局快捷键。

use kvm_core::error::KvmResult;
use kvm_core::platform::HotkeyManager;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Windows 快捷键管理器
pub struct WinHotkeyManager {
    hotkeys: Arc<Mutex<HashMap<String, Box<dyn Fn() + Send + Sync>>>>,
    running: Arc<AtomicBool>,
}

impl WinHotkeyManager {
    pub fn new() -> Self {
        Self {
            hotkeys: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn ensure_listener_running(&self) {
        if self.running.load(Ordering::SeqCst) { return; }
        self.running.store(true, Ordering::SeqCst);
        let hotkeys = Arc::clone(&self.hotkeys);
        let running = Arc::clone(&self.running);
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            hotkey_listener_thread(hotkeys, running);
        });
    }
}

impl HotkeyManager for WinHotkeyManager {
    fn register_hotkey(&self, hotkey_id: &str, callback: Box<dyn Fn() + Send + Sync>) -> KvmResult<()> {
        self.hotkeys.lock().unwrap().insert(hotkey_id.to_string(), callback);
        log::info!("[Win] 注册快捷键: {}", hotkey_id);
        #[cfg(target_os = "windows")]
        {
            if let Some((modifiers, vk)) = parse_hotkey(hotkey_id) {
                let id = hotkey_id_to_int(hotkey_id);
                unsafe {
                    use winapi::um::winuser::RegisterHotKey;
                    RegisterHotKey(std::ptr::null_mut(), id, modifiers, vk as u32);
                }
            }
        }
        self.ensure_listener_running();
        Ok(())
    }

    fn unregister_hotkey(&self, hotkey_id: &str) -> KvmResult<()> {
        self.hotkeys.lock().unwrap().remove(hotkey_id);
        log::info!("[Win] 取消快捷键: {}", hotkey_id);
        #[cfg(target_os = "windows")]
        {
            let id = hotkey_id_to_int(hotkey_id);
            unsafe {
                use winapi::um::winuser::UnregisterHotKey;
                UnregisterHotKey(std::ptr::null_mut(), id);
            }
        }
        Ok(())
    }

    fn unregister_all(&self) -> KvmResult<()> {
        let ids: Vec<String> = self.hotkeys.lock().unwrap().keys().cloned().collect();
        for id in ids { self.unregister_hotkey(&id)?; }
        self.hotkeys.lock().unwrap().clear();
        Ok(())
    }
}

fn hotkey_id_to_int(hotkey_id: &str) -> i32 {
    let mut hash: i32 = 0;
    for b in hotkey_id.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(b as i32);
    }
    hash.abs() % 0xBFFF + 1
}

#[cfg(target_os = "windows")]
fn parse_hotkey(hotkey_id: &str) -> Option<(u32, u16)> {
    use winapi::um::winuser::*;
    let parts: Vec<&str> = hotkey_id.split('+').collect();
    if parts.is_empty() { return None; }
    let mut modifiers: u32 = 0;
    let mut vk: u16 = 0;
    for part in &parts {
        match part.trim().to_uppercase().as_str() {
            "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL as u32,
            "ALT" | "OPTION"   => modifiers |= MOD_ALT as u32,
            "SHIFT"            => modifiers |= MOD_SHIFT as u32,
            "WIN" | "CMD" | "COMMAND" => modifiers |= MOD_WIN as u32,
            key => { vk = key_name_to_vk(key); }
        }
    }
    if vk == 0 { None } else { Some((modifiers, vk)) }
}

#[cfg(target_os = "windows")]
fn key_name_to_vk(key: &str) -> u16 {
    match key {
        "A"=>0x41,"B"=>0x42,"C"=>0x43,"D"=>0x44,"E"=>0x45,"F"=>0x46,"G"=>0x47,
        "H"=>0x48,"I"=>0x49,"J"=>0x4A,"K"=>0x4B,"L"=>0x4C,"M"=>0x4D,"N"=>0x4E,
        "O"=>0x4F,"P"=>0x50,"Q"=>0x51,"R"=>0x52,"S"=>0x53,"T"=>0x54,"U"=>0x55,
        "V"=>0x56,"W"=>0x57,"X"=>0x58,"Y"=>0x59,"Z"=>0x5A,
        "0"=>0x30,"1"=>0x31,"2"=>0x32,"3"=>0x33,"4"=>0x34,
        "5"=>0x35,"6"=>0x36,"7"=>0x37,"8"=>0x38,"9"=>0x39,
        "F1"=>0x70,"F2"=>0x71,"F3"=>0x72,"F4"=>0x73,"F5"=>0x74,"F6"=>0x75,
        "F7"=>0x76,"F8"=>0x77,"F9"=>0x78,"F10"=>0x79,"F11"=>0x7A,"F12"=>0x7B,
        "ESC"|"ESCAPE"=>0x1B,"TAB"=>0x09,"SPACE"=>0x20,"ENTER"|"RETURN"=>0x0D,
        "BACKSPACE"=>0x08,"DELETE"|"DEL"=>0x2E,"INSERT"|"INS"=>0x2D,
        "HOME"=>0x24,"END"=>0x23,"PAGEUP"|"PGUP"=>0x21,"PAGEDOWN"|"PGDN"=>0x22,
        "LEFT"=>0x25,"UP"=>0x26,"RIGHT"=>0x27,"DOWN"=>0x28,
        _ => 0,
    }
}

#[cfg(target_os = "windows")]
fn hotkey_listener_thread(
    hotkeys: Arc<Mutex<HashMap<String, Box<dyn Fn() + Send + Sync>>>>,
    running: Arc<AtomicBool>,
) {
    use winapi::um::winuser::*;
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while running.load(Ordering::SeqCst) {
            let result = MsgWaitForMultipleObjects(0, std::ptr::null(), 0, 100, QS_HOTKEY);
            if result == 0xFFFFFFFF { break; }
            while PeekMessageW(&mut msg, std::ptr::null_mut(), WM_HOTKEY, WM_HOTKEY, PM_REMOVE) != 0 {
                let hotkey_id_int = msg.wParam as i32;
                let guard = hotkeys.lock().unwrap();
                for (id_str, callback) in guard.iter() {
                    if hotkey_id_to_int(id_str) == hotkey_id_int {
                        callback();
                        break;
                    }
                }
            }
        }
    }
}
