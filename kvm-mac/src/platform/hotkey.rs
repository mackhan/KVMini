//! macOS 全局快捷键实现（NSEvent）
//!
//! 使用 NSEvent addGlobalMonitorForEventsMatchingMask 监听全局键盘事件，
//! 匹配注册的快捷键组合后触发回调。

use kvm_core::error::KvmResult;
use kvm_core::platform::HotkeyManager;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// 快捷键描述（修饰键 + 键码）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HotkeySpec {
    /// macOS CGKeyCode
    pub keycode: u16,
    /// 修饰键标志位（NSEventModifierFlags）
    pub modifiers: u64,
}

impl HotkeySpec {
    /// 从字符串解析快捷键，如 "ctrl+alt+s"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.to_lowercase().split('+').collect();
        let mut modifiers: u64 = 0;
        let mut keycode: Option<u16> = None;

        for part in &parts {
            match part.trim() {
                "ctrl" | "control" => modifiers |= 0x0004_0000, // NSEventModifierFlagControl
                "alt" | "option"   => modifiers |= 0x0008_0000, // NSEventModifierFlagOption
                "shift"            => modifiers |= 0x0002_0000, // NSEventModifierFlagShift
                "cmd" | "command"  => modifiers |= 0x0010_0000, // NSEventModifierFlagCommand
                key => {
                    keycode = key_name_to_cg_keycode(key);
                }
            }
        }

        keycode.map(|kc| HotkeySpec { keycode: kc, modifiers })
    }
}

/// 将键名转换为 CGKeyCode
fn key_name_to_cg_keycode(name: &str) -> Option<u16> {
    match name {
        "a" => Some(0), "s" => Some(1), "d" => Some(2), "f" => Some(3),
        "h" => Some(4), "g" => Some(5), "z" => Some(6), "x" => Some(7),
        "c" => Some(8), "v" => Some(9), "b" => Some(11), "q" => Some(12),
        "w" => Some(13), "e" => Some(14), "r" => Some(15), "y" => Some(16),
        "t" => Some(17), "1" => Some(18), "2" => Some(19), "3" => Some(20),
        "4" => Some(21), "6" => Some(22), "5" => Some(23), "9" => Some(25),
        "7" => Some(26), "8" => Some(28), "0" => Some(29), "o" => Some(31),
        "u" => Some(32), "i" => Some(34), "p" => Some(35), "l" => Some(37),
        "j" => Some(38), "k" => Some(40), "n" => Some(45), "m" => Some(46),
        "tab" => Some(48), "space" => Some(49), "escape" | "esc" => Some(53),
        "return" | "enter" => Some(36), "delete" | "backspace" => Some(51),
        "f1" => Some(122), "f2" => Some(120), "f3" => Some(99), "f4" => Some(118),
        "f5" => Some(96), "f6" => Some(97), "f7" => Some(98), "f8" => Some(100),
        "f9" => Some(101), "f10" => Some(109), "f11" => Some(103), "f12" => Some(111),
        _ => None,
    }
}

pub struct MacHotkeyManager {
    /// hotkey_id -> (spec, callback)
    hotkeys: Arc<Mutex<HashMap<String, (HotkeySpec, Box<dyn Fn() + Send + Sync>)>>>,
    /// NSEvent 监听器句柄（macOS 上使用）
    _monitor_handle: Mutex<Option<MonitorHandle>>,
}

struct MonitorHandle {
    #[cfg(target_os = "macos")]
    _id: *mut std::ffi::c_void,
}

// 安全：MonitorHandle 仅在主线程创建和销毁
unsafe impl Send for MonitorHandle {}
unsafe impl Sync for MonitorHandle {}

impl MacHotkeyManager {
    pub fn new() -> Self {
        Self {
            hotkeys: Arc::new(Mutex::new(HashMap::new())),
            _monitor_handle: Mutex::new(None),
        }
    }

    /// 启动全局键盘监听（首次注册快捷键时调用）
    fn ensure_monitor_started(&self) -> KvmResult<()> {
        let mut handle = self._monitor_handle.lock().unwrap();
        if handle.is_some() {
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            let hotkeys = Arc::clone(&self.hotkeys);
            let monitor_id = start_global_key_monitor(hotkeys)?;
            *handle = Some(MonitorHandle { _id: monitor_id });
        }

        Ok(())
    }
}

impl HotkeyManager for MacHotkeyManager {
    fn register_hotkey(&self, hotkey_id: &str, callback: Box<dyn Fn() + Send + Sync>) -> KvmResult<()> {
        let spec = HotkeySpec::parse(hotkey_id).ok_or_else(|| {
            kvm_core::error::KvmError::Config(format!("无法解析快捷键: {}", hotkey_id))
        })?;

        self.hotkeys.lock().unwrap().insert(hotkey_id.to_string(), (spec, callback));
        log::info!("[Mac] 注册快捷键: {}", hotkey_id);

        self.ensure_monitor_started()?;
        Ok(())
    }

    fn unregister_hotkey(&self, hotkey_id: &str) -> KvmResult<()> {
        self.hotkeys.lock().unwrap().remove(hotkey_id);
        log::info!("[Mac] 取消快捷键: {}", hotkey_id);
        Ok(())
    }

    fn unregister_all(&self) -> KvmResult<()> {
        self.hotkeys.lock().unwrap().clear();
        // 停止监听器
        *self._monitor_handle.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn start_global_key_monitor(
    hotkeys: Arc<Mutex<HashMap<String, (HotkeySpec, Box<dyn Fn() + Send + Sync>)>>>,
) -> KvmResult<*mut std::ffi::c_void> {
    use objc::runtime::{Object, YES};
    use objc::{class, msg_send, sel, sel_impl};

    // NSEventMaskKeyDown = 1 << 10
    let mask: u64 = 1 << 10;

    // 使用 block 回调
    let block = block::ConcreteBlock::new(move |event: *mut Object| {
        // 获取 keyCode 和 modifierFlags
        let keycode: u16 = unsafe { msg_send![event, keyCode] };
        let modifiers: u64 = unsafe { msg_send![event, modifierFlags] };
        // 只保留修饰键位
        let modifiers = modifiers & 0x001F_0000;

        let guard = hotkeys.lock().unwrap();
        for (_, (spec, cb)) in guard.iter() {
            if spec.keycode == keycode && spec.modifiers == modifiers {
                cb();
                break;
            }
        }
    });
    let block = block.copy();

    let monitor: *mut Object = unsafe {
        let cls = class!(NSEvent);
        msg_send![cls,
            addGlobalMonitorForEventsMatchingMask: mask
            handler: &*block
        ]
    };

    if monitor.is_null() {
        return Err(kvm_core::error::KvmError::Platform(
            "addGlobalMonitorForEventsMatchingMask 返回 nil，请检查辅助功能权限".into()
        ));
    }

    log::info!("[Mac] NSEvent 全局键盘监听已启动");
    Ok(monitor as *mut std::ffi::c_void)
}
