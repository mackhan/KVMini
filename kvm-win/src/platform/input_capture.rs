//! Windows 键鼠事件采集实现
//!
//! 使用 SetWindowsHookEx 采集全局键鼠事件。
//! 采集到的事件通过回调传递给 kvm-core 处理。

use kvm_core::error::{KvmError, KvmResult};
use kvm_core::platform::{InputCapture, MouseButton, RawInputEvent};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Windows 输入采集器
pub struct WinInputCapture {
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
}

impl WinInputCapture {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
            callback: Arc::new(Mutex::new(None)),
        }
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

            // 在独立线程中运行消息循环（Hook 需要消息循环）
            std::thread::spawn(move || {
                win32_hook_thread(capturing, cb);
            });
        }

        Ok(())
    }

    fn stop_capture(&self) -> KvmResult<()> {
        self.capturing.store(false, Ordering::SeqCst);
        *self.callback.lock().unwrap() = None;
        log::info!("[Win] 键鼠采集已停止");
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
) {
    use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
    use winapi::shared::windef::HHOOK;
    use winapi::um::winuser::*;

    // 将回调存入线程局部变量（Hook 回调必须是 C 函数，无法直接捕获闭包）
    HOOK_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(callback);
    });
    HOOK_CAPTURING.with(|cell| {
        *cell.borrow_mut() = Some(capturing);
    });

    unsafe {
        // 安装鼠标 Hook
        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            std::ptr::null_mut(),
            0,
        );

        // 安装键盘 Hook
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

        log::debug!("[Win] Hook 已安装，进入消息循环");

        // 消息循环（Hook 需要消息循环才能工作）
        let mut msg: MSG = std::mem::zeroed();
        loop {
            // 检查是否应该停止
            let should_stop = HOOK_CAPTURING
                .with(|cell| cell.borrow().as_ref().map(|c| !c.load(Ordering::SeqCst)).unwrap_or(true));

            if should_stop {
                break;
            }

            // 非阻塞消息处理
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // 短暂休眠避免 CPU 占用过高
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // 卸载 Hook
        if !mouse_hook.is_null() {
            UnhookWindowsHookEx(mouse_hook);
        }
        if !keyboard_hook.is_null() {
            UnhookWindowsHookEx(keyboard_hook);
        }

        log::debug!("[Win] Hook 已卸载");
    }
}

#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_CALLBACK: std::cell::RefCell<Option<Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>>> =
        std::cell::RefCell::new(None);
    static HOOK_CAPTURING: std::cell::RefCell<Option<Arc<AtomicBool>>> =
        std::cell::RefCell::new(None);
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
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}
