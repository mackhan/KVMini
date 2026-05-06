//! Windows 键鼠事件采集实现
//!
//! 使用 SetWindowsHookEx 采集全局键鼠事件。
//!
//! v0.1.38 BUG-022 修复：Hook 回调按控制状态决定"放行 vs 吞掉"：
//! - Idle：CallNextHookEx 放行
//! - LocalControlling / RemoteControlling：返回 1（非零）吞掉事件
//! - 状态通过 Arc<AtomicU8> 跨线程共享，hook 回调用 Relaxed 读。

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputCapture, MouseButton, RawInputEvent};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};

/// 控制状态原子值（与 macOS 侧保持一致语义）
pub const CONTROL_STATE_IDLE: u8 = 0;
pub const CONTROL_STATE_LOCAL_CONTROLLING: u8 = 1;
pub const CONTROL_STATE_REMOTE_CONTROLLING: u8 = 2;

/// Windows 输入采集器
pub struct WinInputCapture {
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
    /// 控制状态：0=Idle / 1=LocalControlling / 2=RemoteControlling
    control_state: Arc<AtomicU8>,
}

impl WinInputCapture {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
            callback: Arc::new(Mutex::new(None)),
            control_state: Arc::new(AtomicU8::new(CONTROL_STATE_IDLE)),
        }
    }

    /// SharingManager 切换状态时调用
    pub fn set_control_state(&self, new_state: u8) {
        let old = self.control_state.swap(new_state, Ordering::Relaxed);
        if old != new_state {
            log::info!(
                "[Win] control_state: {} → {}",
                describe_state(old),
                describe_state(new_state)
            );
        }
    }

    pub fn control_state(&self) -> u8 {
        self.control_state.load(Ordering::Relaxed)
    }

    pub fn control_state_arc(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.control_state)
    }
}

impl Default for WinInputCapture {
    fn default() -> Self {
        Self::new()
    }
}

fn describe_state(v: u8) -> &'static str {
    match v {
        CONTROL_STATE_IDLE => "Idle",
        CONTROL_STATE_LOCAL_CONTROLLING => "LocalControlling",
        CONTROL_STATE_REMOTE_CONTROLLING => "RemoteControlling",
        _ => "Unknown",
    }
}

impl InputCapture for WinInputCapture {
    fn start_capture(&self, callback: Box<dyn Fn(RawInputEvent) + Send + Sync>) -> KvmResult<()> {
        *self.callback.lock().unwrap() = Some(callback);
        self.capturing.store(true, Ordering::SeqCst);
        log::info!("[Win] 键鼠采集已启动");

        #[cfg(target_os = "windows")]
        {
            let capturing = Arc::clone(&self.capturing);
            let cb = Arc::clone(&self.callback);
            let control_state = Arc::clone(&self.control_state);
            std::thread::spawn(move || {
                win32_hook_thread(capturing, cb, control_state);
            });
        }

        Ok(())
    }

    fn stop_capture(&self) -> KvmResult<()> {
        self.capturing.store(false, Ordering::SeqCst);
        *self.callback.lock().unwrap() = None;
        // 安全兜底：停止采集时强制回 Idle
        self.control_state
            .store(CONTROL_STATE_IDLE, Ordering::Relaxed);
        log::info!("[Win] 键鼠采集已停止，control_state → Idle");
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.capturing.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "windows")]
fn win32_hook_thread(
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
    control_state: Arc<AtomicU8>,
) {
    use winapi::um::winuser::*;

    HOOK_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(callback);
    });
    HOOK_CAPTURING.with(|cell| {
        *cell.borrow_mut() = Some(capturing);
    });
    HOOK_CONTROL_STATE.with(|cell| {
        *cell.borrow_mut() = Some(control_state);
    });

    unsafe {
        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            std::ptr::null_mut(),
            0,
        );
        let keyboard_hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            std::ptr::null_mut(),
            0,
        );

        if mouse_hook.is_null() || keyboard_hook.is_null() {
            log::error!("[Win] 安装 Hook 失败");
            return;
        }

        let mut msg: MSG = std::mem::zeroed();
        loop {
            let should_stop = HOOK_CAPTURING
                .with(|cell| cell.borrow().as_ref().map(|c| !c.load(Ordering::SeqCst)).unwrap_or(true));
            if should_stop { break; }

            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        if !mouse_hook.is_null() { UnhookWindowsHookEx(mouse_hook); }
        if !keyboard_hook.is_null() { UnhookWindowsHookEx(keyboard_hook); }
    }
}

#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_CALLBACK: std::cell::RefCell<Option<Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>>> =
        std::cell::RefCell::new(None);
    static HOOK_CAPTURING: std::cell::RefCell<Option<Arc<AtomicBool>>> =
        std::cell::RefCell::new(None);
    static HOOK_CONTROL_STATE: std::cell::RefCell<Option<Arc<AtomicU8>>> =
        std::cell::RefCell::new(None);
}

/// 读取当前控制状态（Hook 回调线程专用，用 Relaxed 读避免锁）
#[cfg(target_os = "windows")]
fn read_control_state() -> u8 {
    HOOK_CONTROL_STATE
        .with(|cell| cell.borrow().as_ref().map(|s| s.load(Ordering::Relaxed)))
        .unwrap_or(CONTROL_STATE_IDLE)
}

/// 是否应该吞掉事件（LocalControlling / RemoteControlling 时吞）
#[cfg(target_os = "windows")]
fn should_swallow() -> bool {
    matches!(
        read_control_state(),
        CONTROL_STATE_LOCAL_CONTROLLING | CONTROL_STATE_REMOTE_CONTROLLING
    )
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: winapi::shared::minwindef::WPARAM,
    l_param: winapi::shared::minwindef::LPARAM,
) -> winapi::shared::minwindef::LRESULT {
    use winapi::um::winuser::*;

    if n_code >= 0 {
        let ms_hook = &*(l_param as *const MSLLHOOKSTRUCT);
        let x = ms_hook.pt.x;
        let y = ms_hook.pt.y;

        let event = match w_param as u32 {
            WM_MOUSEMOVE => Some(RawInputEvent::MouseMove { x, y }),
            WM_LBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Left }),
            WM_LBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Left }),
            WM_RBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Right }),
            WM_RBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Right }),
            WM_MBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Middle }),
            WM_MBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Middle }),
            WM_MOUSEWHEEL => {
                let delta = (ms_hook.mouseData >> 16) as i16 as i32;
                Some(RawInputEvent::Scroll { dx: 0, dy: delta })
            }
            WM_MOUSEHWHEEL => {
                let delta = (ms_hook.mouseData >> 16) as i16 as i32;
                Some(RawInputEvent::Scroll { dx: delta, dy: 0 })
            }
            _ => None,
        };

        if let Some(ev) = event {
            HOOK_CALLBACK.with(|cell| {
                if let Some(cb_arc) = cell.borrow().as_ref() {
                    if let Ok(guard) = cb_arc.lock() {
                        if let Some(cb) = guard.as_ref() {
                            cb(ev);
                        }
                    }
                }
            });
        }

        // 按状态吞事件（BUG-022 关键修复）
        if should_swallow() {
            return 1; // 非零返回值 = 吞掉事件，不传给本机
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: winapi::shared::minwindef::WPARAM,
    l_param: winapi::shared::minwindef::LPARAM,
) -> winapi::shared::minwindef::LRESULT {
    use winapi::um::winuser::*;

    if n_code >= 0 {
        let kb_hook = &*(l_param as *const KBDLLHOOKSTRUCT);
        let vk_code = kb_hook.vkCode as u16;

        let event = match w_param as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(RawInputEvent::KeyDown { native_keycode: vk_code }),
            WM_KEYUP | WM_SYSKEYUP => Some(RawInputEvent::KeyUp { native_keycode: vk_code }),
            _ => None,
        };

        if let Some(ev) = event {
            HOOK_CALLBACK.with(|cell| {
                if let Some(cb_arc) = cell.borrow().as_ref() {
                    if let Ok(guard) = cb_arc.lock() {
                        if let Some(cb) = guard.as_ref() {
                            cb(ev);
                        }
                    }
                }
            });
        }

        // 按状态吞事件（BUG-022 关键修复）
        if should_swallow() {
            return 1;
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state_is_idle() {
        let cap = WinInputCapture::new();
        assert_eq!(cap.control_state(), CONTROL_STATE_IDLE);
    }

    #[test]
    fn test_set_control_state_roundtrip() {
        let cap = WinInputCapture::new();
        cap.set_control_state(CONTROL_STATE_LOCAL_CONTROLLING);
        assert_eq!(cap.control_state(), CONTROL_STATE_LOCAL_CONTROLLING);
    }

    #[test]
    fn test_stop_capture_resets_state() {
        let cap = WinInputCapture::new();
        cap.set_control_state(CONTROL_STATE_LOCAL_CONTROLLING);
        cap.capturing.store(true, Ordering::SeqCst);
        let _ = cap.stop_capture();
        assert_eq!(cap.control_state(), CONTROL_STATE_IDLE);
    }
}
