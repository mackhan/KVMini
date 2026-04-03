/// Windows 端全局输入钩子与事件转发
/// 使用 SetWindowsHookEx 捕获全局鼠标和键盘事件
/// 仅在 Windows 上编译，使用 #[cfg(target_os = "windows")] 隔离
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::types::{InputEvent, MouseButton};
use crate::protocol::{KvmMessage, InputEventPacket, encode_message};

// ============================================================
// 转发模式状态（全局原子变量，线程安全）
// ============================================================

/// 是否处于转发模式（鼠标已锁定，输入转发给 Mac）
static FORWARDING: AtomicBool = AtomicBool::new(false);

/// 是否已安装钩子
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

/// 消息发送通道（用于将捕获的事件发送到 TCP 写入线程）
lazy_static::lazy_static! {
    static ref EVENT_SENDER: Arc<Mutex<Option<std::sync::mpsc::Sender<KvmMessage>>>> =
        Arc::new(Mutex::new(None));
}

// ============================================================
// 公开 API
// ============================================================

/// 检查当前是否处于转发模式
pub fn is_forwarding() -> bool {
    FORWARDING.load(Ordering::SeqCst)
}

/// 手动切换到转发模式（连接成功后调用）
pub fn enter_forwarding_mode() {
    FORWARDING.store(true, Ordering::SeqCst);
    println!("[InputCapture] 进入转发模式，鼠标已锁定");
    #[cfg(target_os = "windows")]
    windows_impl::lock_cursor();
}

/// 退出转发模式，释放鼠标锁定
pub fn exit_forwarding_mode() {
    FORWARDING.store(false, Ordering::SeqCst);
    println!("[InputCapture] 退出转发模式，鼠标已释放");
    #[cfg(target_os = "windows")]
    windows_impl::unlock_cursor();
}

/// 设置事件发送通道（连接建立后调用）
pub fn set_event_sender(sender: std::sync::mpsc::Sender<KvmMessage>) {
    let mut s = EVENT_SENDER.lock().unwrap();
    *s = Some(sender);
}

/// 清除事件发送通道（连接断开后调用）
pub fn clear_event_sender() {
    let mut s = EVENT_SENDER.lock().unwrap();
    *s = None;
    // 连接断开时立即退出转发模式
    exit_forwarding_mode();
}

/// 安装全局输入钩子
pub fn install_hooks() -> Result<(), String> {
    if HOOK_INSTALLED.load(Ordering::SeqCst) {
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        windows_impl::install_hooks()?;
        HOOK_INSTALLED.store(true, Ordering::SeqCst);
        println!("[InputCapture] 全局输入钩子已安装");
    }
    #[cfg(not(target_os = "windows"))]
    {
        println!("[InputCapture] 非 Windows 平台，跳过钩子安装");
    }
    Ok(())
}

/// 卸载全局输入钩子
pub fn uninstall_hooks() {
    #[cfg(target_os = "windows")]
    {
        windows_impl::uninstall_hooks();
        HOOK_INSTALLED.store(false, Ordering::SeqCst);
        println!("[InputCapture] 全局输入钩子已卸载");
    }
}

/// 发送输入事件到 TCP 通道
pub(crate) fn send_input_event(event: InputEvent) {
    let sender = EVENT_SENDER.lock().unwrap();
    if let Some(ref tx) = *sender {
        let msg = KvmMessage::Input(InputEventPacket { event });
        let _ = tx.send(msg);
    }
}

// ============================================================
// Windows 特定实现
// ============================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM, POINT, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx,
        GetSystemMetrics, GetCursorPos, ClipCursor, SetCursorPos,
        HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WH_MOUSE_LL,
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
        WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
        SM_CXSCREEN, SM_CYSCREEN,
    };    use std::sync::atomic::{AtomicIsize, Ordering};

    /// 鼠标钩子句柄
    static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
    /// 键盘钩子句柄
    static KEYBOARD_HOOK: AtomicIsize = AtomicIsize::new(0);

    /// 上一次鼠标位置（用于计算相对移动量）
    static LAST_MOUSE_X: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
    static LAST_MOUSE_Y: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

    /// Ctrl+Alt+Tab 的按键状态追踪
    static CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
    static ALT_PRESSED: AtomicBool = AtomicBool::new(false);

    pub fn install_hooks() -> Result<(), String> {
        unsafe {
            let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), 0, 0);
            if mouse_hook == 0 {
                return Err("安装鼠标钩子失败".to_string());
            }
            MOUSE_HOOK.store(mouse_hook as isize, Ordering::SeqCst);

            let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), 0, 0);
            if kb_hook == 0 {
                UnhookWindowsHookEx(mouse_hook);
                return Err("安装键盘钩子失败".to_string());
            }
            KEYBOARD_HOOK.store(kb_hook as isize, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn uninstall_hooks() {
        unsafe {
            let mh = MOUSE_HOOK.swap(0, Ordering::SeqCst) as HHOOK;
            if mh != 0 {
                UnhookWindowsHookEx(mh);
            }
            let kh = KEYBOARD_HOOK.swap(0, Ordering::SeqCst) as HHOOK;
            if kh != 0 {
                UnhookWindowsHookEx(kh);
            }
        }
    }

    /// 锁定鼠标到屏幕右侧区域（防止鼠标移出）
    pub fn lock_cursor() {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            // 将鼠标限制在屏幕右边缘 1 像素区域
            let rect = RECT {
                left: w - 2,
                top: 0,
                right: w,
                bottom: h,
            };
            ClipCursor(&rect);
        }
    }

    /// 释放鼠标锁定
    pub fn unlock_cursor() {
        unsafe {
            ClipCursor(std::ptr::null());
        }
    }

    /// 获取屏幕宽度
    fn screen_width() -> i32 {
        unsafe { GetSystemMetrics(SM_CXSCREEN) }
    }

    /// 鼠标低级钩子回调
    unsafe extern "system" fn mouse_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let info = &*(l_param as *const MSLLHOOKSTRUCT);
            let x = info.pt.x;
            let y = info.pt.y;

            // 检测鼠标是否到达屏幕右边缘（触发转发模式）
            if !FORWARDING.load(Ordering::SeqCst) {
                let sw = screen_width();
                if x >= sw - 2 {
                    super::enter_forwarding_mode();
                }
            }

            if FORWARDING.load(Ordering::SeqCst) {
                let last_x = LAST_MOUSE_X.load(Ordering::SeqCst);
                let last_y = LAST_MOUSE_Y.load(Ordering::SeqCst);

                match w_param as u32 {
                    WM_MOUSEMOVE => {
                        let dx = x - last_x;
                        let dy = y - last_y;
                        if dx != 0 || dy != 0 {
                            super::send_input_event(InputEvent::MouseMove { x: dx, y: dy });
                        }
                    }
                    WM_LBUTTONDOWN => {
                        super::send_input_event(InputEvent::MouseDown { button: MouseButton::Left });
                    }
                    WM_LBUTTONUP => {
                        super::send_input_event(InputEvent::MouseUp { button: MouseButton::Left });
                    }
                    WM_RBUTTONDOWN => {
                        super::send_input_event(InputEvent::MouseDown { button: MouseButton::Right });
                    }
                    WM_RBUTTONUP => {
                        super::send_input_event(InputEvent::MouseUp { button: MouseButton::Right });
                    }
                    WM_MBUTTONDOWN => {
                        super::send_input_event(InputEvent::MouseDown { button: MouseButton::Middle });
                    }
                    WM_MBUTTONUP => {
                        super::send_input_event(InputEvent::MouseUp { button: MouseButton::Middle });
                    }
                    WM_MOUSEWHEEL => {
                        // 高位字节为滚轮增量
                        let delta = ((info.mouseData >> 16) as i16) as i32;
                        super::send_input_event(InputEvent::Scroll { dx: 0, dy: delta });
                    }
                    _ => {}
                }

                LAST_MOUSE_X.store(x, Ordering::SeqCst);
                LAST_MOUSE_Y.store(y, Ordering::SeqCst);

                // 处于转发模式时，吞掉鼠标事件（不传递给本地系统）
                return 1;
            }

            LAST_MOUSE_X.store(x, Ordering::SeqCst);
            LAST_MOUSE_Y.store(y, Ordering::SeqCst);
        }

        let hook = MOUSE_HOOK.load(Ordering::SeqCst) as HHOOK;
        CallNextHookEx(hook, n_code, w_param, l_param)
    }

    /// 键盘低级钩子回调
    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let info = &*(l_param as *const KBDLLHOOKSTRUCT);
            let vk = info.vkCode as u16;
            let is_down = matches!(w_param as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let is_up = matches!(w_param as u32, WM_KEYUP | WM_SYSKEYUP);

            // 追踪 Ctrl/Alt 状态（用于检测 Ctrl+Alt+Tab 切换快捷键）
            match vk {
                0xA2 | 0xA3 | 0x11 => { // Ctrl
                    CTRL_PRESSED.store(is_down, Ordering::SeqCst);
                }
                0xA4 | 0xA5 | 0x12 => { // Alt
                    ALT_PRESSED.store(is_down, Ordering::SeqCst);
                }
                _ => {}
            }

            // 检测 Ctrl+Alt+Tab：退出转发模式
            if is_down && vk == 0x09 // Tab
                && CTRL_PRESSED.load(Ordering::SeqCst)
                && ALT_PRESSED.load(Ordering::SeqCst)
            {
                if FORWARDING.load(Ordering::SeqCst) {
                    super::exit_forwarding_mode();
                    println!("[InputCapture] Ctrl+Alt+Tab 触发，切换回本地控制");
                    return 1; // 吞掉此快捷键
                }
            }

            // 处于转发模式时，转发键盘事件
            if FORWARDING.load(Ordering::SeqCst) {
                if is_down {
                    super::send_input_event(InputEvent::KeyDown { keycode: vk });
                } else if is_up {
                    super::send_input_event(InputEvent::KeyUp { keycode: vk });
                }
                return 1; // 吞掉键盘事件
            }
        }

        let hook = KEYBOARD_HOOK.load(Ordering::SeqCst) as HHOOK;
        CallNextHookEx(hook, n_code, w_param, l_param)
    }
}

// ============================================================
// Tauri 命令
// ============================================================

#[tauri::command]
/// Tauri命令：安装全局输入钩子（连接成功后调用）
pub fn start_input_capture() -> Result<(), String> {
    install_hooks()
}

#[tauri::command]
/// Tauri命令：卸载全局输入钩子（断开连接后调用）
pub fn stop_input_capture() {
    uninstall_hooks();
    exit_forwarding_mode();
}

#[tauri::command]
/// Tauri命令：查询当前是否处于转发模式
pub fn get_forwarding_status() -> bool {
    is_forwarding()
}
