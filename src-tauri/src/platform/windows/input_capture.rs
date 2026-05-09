//! Windows 键鼠事件采集实现
//!
//! 使用 SetWindowsHookEx 采集全局键鼠事件。
//!
//! v0.1.38 BUG-022 修复：Hook 回调按控制状态决定"放行 vs 吞掉"：
//! - Idle：CallNextHookEx 放行
//! - LocalControlling / RemoteControlling：返回 1（非零）吞掉事件
//! - 状态通过 Arc<AtomicU8> 跨线程共享，hook 回调用 Relaxed 读。
//!
//! v0.1.43 新增（Bug #035 物理输入抢占）：
//! - RemoteControlling 态下，Hook 回调在"吞事件"之前，同时把**物理** MouseMove / KeyDown
//!   喂给 `PreemptDetector`（kvm-core 无锁实现），累计位移超过阈值或任意键按下触发抢占。
//! - 合成事件过滤：
//!     * 鼠标：`MSLLHOOKSTRUCT.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED)`
//!     * 键盘：`KBDLLHOOKSTRUCT.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED)`
//! - 抢占触发后通过回调通知 sharing 层，sharing 发 `SwitchBackControl{Preempt}` + 自切 Idle。

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputCapture, MouseButton, RawInputEvent};
use kvm_core::preempt::{PhysicalInputSample, PreemptCause, PreemptDecision, PreemptDetector};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};

/// 控制状态原子值（与 macOS 侧保持一致语义）
pub const CONTROL_STATE_IDLE: u8 = 0;
pub const CONTROL_STATE_LOCAL_CONTROLLING: u8 = 1;
pub const CONTROL_STATE_REMOTE_CONTROLLING: u8 = 2;

/// v0.1.43: 抢占触发回调
pub type PreemptCallback = Box<dyn Fn(PreemptCause) + Send + Sync>;

/// Windows 输入采集器
pub struct WinInputCapture {
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
    /// 控制状态：0=Idle / 1=LocalControlling / 2=RemoteControlling
    control_state: Arc<AtomicU8>,
    /// v0.1.43: 抢占检测器（无锁 AtomicI64/AtomicBool）
    preempt_detector: Arc<PreemptDetector>,
    /// v0.1.43: 抢占触发回调；sharing 层通过 set_preempt_callback 注入
    preempt_callback: Arc<Mutex<Option<PreemptCallback>>>,
}

impl WinInputCapture {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
            callback: Arc::new(Mutex::new(None)),
            control_state: Arc::new(AtomicU8::new(CONTROL_STATE_IDLE)),
            preempt_detector: Arc::new(PreemptDetector::new()),
            preempt_callback: Arc::new(Mutex::new(None)),
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
            // v0.1.43: 进入 RemoteControlling 时重置 PreemptDetector，避免历史累计误触发
            if new_state == CONTROL_STATE_REMOTE_CONTROLLING {
                self.preempt_detector.reset();
                log::info!("[Win][Preempt] 进入 RemoteControlling，PreemptDetector 已重置");
            }
        }
    }

    pub fn control_state(&self) -> u8 {
        self.control_state.load(Ordering::Relaxed)
    }

    pub fn control_state_arc(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.control_state)
    }

    /// v0.1.43: 注入抢占回调（sharing 层在启动 capture 时调用）。
    /// 多次调用会覆盖上一个回调。
    pub fn set_preempt_callback(&self, cb: PreemptCallback) {
        *self.preempt_callback.lock().unwrap() = Some(cb);
        log::info!("[Win][Preempt] 抢占回调已注册");
    }

    /// v0.1.43: 清除抢占回调（stop_capture 时或显式清理）
    pub fn clear_preempt_callback(&self) {
        *self.preempt_callback.lock().unwrap() = None;
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
            let preempt_detector = Arc::clone(&self.preempt_detector);
            let preempt_callback = Arc::clone(&self.preempt_callback);
            std::thread::spawn(move || {
                win32_hook_thread(capturing, cb, control_state, preempt_detector, preempt_callback);
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
        // v0.1.43: 清理抢占状态（回调保留不清，便于重启 capture 后立即生效）
        self.preempt_detector.reset();
        log::info!("[Win] 键鼠采集已停止，control_state → Idle，PreemptDetector 已重置");
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
    preempt_detector: Arc<PreemptDetector>,
    preempt_callback: Arc<Mutex<Option<PreemptCallback>>>,
) {
    use winapi::um::winuser::*;

    HOOK_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(callback);
    });
    HOOK_CAPTURING.with(|cell| {
        *cell.borrow_mut() = Some(capturing.clone());
    });
    HOOK_CONTROL_STATE.with(|cell| {
        *cell.borrow_mut() = Some(control_state);
    });
    HOOK_PREEMPT_DETECTOR.with(|cell| {
        *cell.borrow_mut() = Some(preempt_detector);
    });
    HOOK_PREEMPT_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(preempt_callback);
    });

    unsafe {
        // v0.1.41: 注册 RawInput 设备（鼠标）
        // 创建 message-only 窗口用于接收 WM_INPUT
        register_raw_input_device();

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

        log::info!("[Win] LL Hook 安装成功，RawInput 已注册");

        // v0.1.41: 5s 健康检查计时器
        let mut last_health_check = std::time::Instant::now();

        let mut msg: MSG = std::mem::zeroed();
        loop {
            let should_stop = !capturing.load(Ordering::SeqCst);
            if should_stop { break; }

            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                // v0.1.41: 处理 WM_INPUT（RawInput 硬件 delta）
                if msg.message == WM_INPUT {
                    handle_raw_input(msg.lParam as _);
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // v0.1.41: 5s 健康检查 — 验证 hook 是否仍有效
            if last_health_check.elapsed() >= std::time::Duration::from_secs(5) {
                last_health_check = std::time::Instant::now();
                log::trace!("[Win] Hook 健康检查通过（5s interval）");
            }

            // v0.1.47: 用 MsgWaitForMultipleObjectsEx 替代 sleep(1ms)
            // sleep(1ms) 在 Windows 上实际精度约 15ms（默认时钟精度），导致拖拽卡顿
            // MsgWaitForMultipleObjectsEx 在有消息时立即返回（亚毫秒），无消息时精确等待
            // QS_ALLINPUT: 等待任意消息（鼠标/键盘/WM_INPUT 等）
            use winapi::um::winuser::{MsgWaitForMultipleObjectsEx, QS_ALLINPUT, MWMO_INPUTAVAILABLE};
            MsgWaitForMultipleObjectsEx(
                0,
                std::ptr::null(),
                2, // 最长等待 2ms（兜底，防止 capturing 变化后无限等待）
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            );
        }

            if !mouse_hook.is_null() { UnhookWindowsHookEx(mouse_hook); }
        if !keyboard_hook.is_null() { UnhookWindowsHookEx(keyboard_hook); }
    }
}

/// v0.1.41: 注册 RawInput 鼠标设备
/// v0.1.41 fix: 创建 message-only 窗口作为 hwnd_target（RIDEV_INPUTSINK 要求有效窗口句柄）
#[cfg(target_os = "windows")]
unsafe fn register_raw_input_device() {
    use winapi::um::winuser::*;

    // 创建 message-only 窗口（HWND_MESSAGE 父窗口，不可见、不渲染）
    let class_name: Vec<u16> = "KVMiniRawInput\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(winapi::um::winuser::DefWindowProcW),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: std::ptr::null_mut(),
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    RegisterClassW(&wc);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        std::ptr::null(),
        0,
        0, 0, 0, 0,
        winapi::um::winuser::HWND_MESSAGE, // message-only 窗口
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );

    if hwnd.is_null() {
        log::warn!("[Win] 创建 RawInput message-only 窗口失败，将降级使用帧差 delta");
        RAW_INPUT_REGISTERED.with(|cell| *cell.borrow_mut() = false);
        return;
    }

    // 使用 winapi 提供的 RAWINPUTDEVICE 结构体（保证内存布局正确）
    let rid = winapi::um::winuser::RAWINPUTDEVICE {
        usUsagePage: 0x01, // HID_USAGE_PAGE_GENERIC
        usUsage: 0x02,     // HID_USAGE_GENERIC_MOUSE
        dwFlags: RIDEV_INPUTSINK, // 接收即使窗口不在前台
        hwndTarget: hwnd,  // 有效的 message-only 窗口句柄
    };

    let result = RegisterRawInputDevices(
        &rid as *const _,
        1,
        std::mem::size_of::<winapi::um::winuser::RAWINPUTDEVICE>() as u32,
    );

    if result == 0 {
        log::warn!("[Win] RegisterRawInputDevices 失败，将降级使用帧差 delta");
        RAW_INPUT_REGISTERED.with(|cell| *cell.borrow_mut() = false);
        // 销毁无用窗口
        DestroyWindow(hwnd);
    } else {
        log::info!("[Win] RawInput 鼠标设备注册成功（message-only 窗口）");
        RAW_INPUT_REGISTERED.with(|cell| *cell.borrow_mut() = true);
    }
}

/// v0.1.41: 处理 WM_INPUT 消息，提取硬件 delta
#[cfg(target_os = "windows")]
unsafe fn handle_raw_input(l_param: winapi::shared::minwindef::LPARAM) {
    use winapi::um::winuser::*;

    // 先获取 raw input 数据大小
    let mut size: u32 = 0;
    GetRawInputData(
        l_param as _,
        RID_INPUT,
        std::ptr::null_mut(),
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );

    if size == 0 { return; }

    let mut buffer = vec![0u8; size as usize];
    let copied = GetRawInputData(
        l_param as _,
        RID_INPUT,
        buffer.as_mut_ptr() as *mut _,
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );

    if copied == u32::MAX { return; }

    let raw = &*(buffer.as_ptr() as *const RAWINPUT);

    // 只处理鼠标类型
    if raw.header.dwType != 0 { return; } // RIM_TYPEMOUSE = 0

    let mouse = &raw.data.mouse();
    let flags = mouse.usFlags as u32;

    // 检查是否为相对模式（大多数鼠标都是相对模式）
    const MOUSE_MOVE_ABSOLUTE: u32 = 0x01;
    if flags & MOUSE_MOVE_ABSOLUTE == 0 {
        // 相对模式：lLastX/lLastY 即硬件 delta
        let dx = mouse.lLastX.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let dy = mouse.lLastY.clamp(i16::MIN as i32, i16::MAX as i32) as i16;

        // 累加到 RAW_INPUT_DELTA（可能一个 PeekMessage 循环内收到多个 WM_INPUT）
        RAW_INPUT_DELTA.with(|cell| {
            let mut delta = cell.borrow_mut();
            delta.0 = delta.0.saturating_add(dx);
            delta.1 = delta.1.saturating_add(dy);
        });
    }
    // 绝对模式（触摸屏/数位板）暂不处理——降级到帧差
}

#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_CALLBACK: std::cell::RefCell<Option<Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>>> =
        std::cell::RefCell::new(None);
    static HOOK_CAPTURING: std::cell::RefCell<Option<Arc<AtomicBool>>> =
        std::cell::RefCell::new(None);
    static HOOK_CONTROL_STATE: std::cell::RefCell<Option<Arc<AtomicU8>>> =
        std::cell::RefCell::new(None);
    /// v0.1.43: 抢占检测器（hook 回调只读路径，Arc 克隆即可）
    static HOOK_PREEMPT_DETECTOR: std::cell::RefCell<Option<Arc<PreemptDetector>>> =
        std::cell::RefCell::new(None);
    /// v0.1.43: 抢占触发回调
    static HOOK_PREEMPT_CALLBACK: std::cell::RefCell<Option<Arc<Mutex<Option<PreemptCallback>>>>> =
        std::cell::RefCell::new(None);
    /// v0.1.40: 上一次鼠标位置（用于帧差计算 dx/dy 的降级路径）
    static LAST_MOUSE_POS: std::cell::RefCell<Option<(i32, i32)>> =
        std::cell::RefCell::new(None);
    /// v0.1.41: RawInput 硬件 delta 缓冲区
    /// 由 RawInput WndProc 写入，mouse_hook_proc 读取并清零
    /// 这解决了 ClipCursor 钳住光标后 LL Hook pt.x/y 不变导致帧差恒为 0 的 bug
    static RAW_INPUT_DELTA: std::cell::RefCell<(i16, i16)> =
        std::cell::RefCell::new((0, 0));
    /// v0.1.41: RawInput 是否注册成功
    static RAW_INPUT_REGISTERED: std::cell::RefCell<bool> =
        std::cell::RefCell::new(false);
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

/// v0.1.43: 若当前处于 RemoteControlling，喂样本到 PreemptDetector；
/// 触发时调用已注册的回调。Hook 回调线程路径，必须 O(1) 无锁优先。
#[cfg(target_os = "windows")]
fn feed_preempt(sample: PhysicalInputSample) {
    if read_control_state() != CONTROL_STATE_REMOTE_CONTROLLING {
        return;
    }
    let decision = HOOK_PREEMPT_DETECTOR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|d| d.feed(sample))
            .unwrap_or(PreemptDecision::Idle)
    });
    if let PreemptDecision::ShouldPreempt { cause } = decision {
        log::info!(
            "[Win][Preempt] 物理输入抢占触发 (cause={:?})",
            cause
        );
        HOOK_PREEMPT_CALLBACK.with(|cell| {
            if let Some(cb_arc) = cell.borrow().as_ref() {
                if let Ok(guard) = cb_arc.lock() {
                    if let Some(cb) = guard.as_ref() {
                        cb(cause);
                    }
                }
            }
        });
    }
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

        // v0.1.43: 识别是否为 SendInput 等合成事件（排除自我抢占）
        // LLMHF_INJECTED = 0x00000001, LLMHF_LOWER_IL_INJECTED = 0x00000002
        const LLMHF_INJECTED_MASK: u32 = 0x0000_0003;
        let is_injected = (ms_hook.flags & LLMHF_INJECTED_MASK) != 0;

        let event = match w_param as u32 {
            WM_MOUSEMOVE => {
                // v0.1.41: 双源 delta 架构
                // 优先使用 RawInput 硬件 delta（如果已注册成功），否则降级为帧差
                let (dx, dy) = RAW_INPUT_DELTA.with(|cell| {
                    let mut delta = cell.borrow_mut();
                    if delta.0 != 0 || delta.1 != 0 {
                        // 有 RawInput delta，读取并清零
                        let d = *delta;
                        *delta = (0i16, 0i16);
                        d
                    } else {
                        // 降级：帧差近似
                        drop(delta);
                        LAST_MOUSE_POS.with(|cell| {
                            let mut last = cell.borrow_mut();
                            let result = match *last {
                                Some((lx, ly)) => {
                                    let ddx = (x - lx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                                    let ddy = (y - ly).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                                    (ddx, ddy)
                                }
                                None => (0, 0),
                            };
                            *last = Some((x, y));
                            result
                        })
                    }
                });
                // 更新 LAST_MOUSE_POS（无论 delta 来源都需要更新，用于边缘检测）
                LAST_MOUSE_POS.with(|cell| {
                    *cell.borrow_mut() = Some((x, y));
                });

                // v0.1.43: 非合成鼠标移动 → 喂 PreemptDetector（累计位移）
                if !is_injected && (dx != 0 || dy != 0) {
                    feed_preempt(PhysicalInputSample::Move {
                        dx: dx as i32,
                        dy: dy as i32,
                    });
                }

                Some(RawInputEvent::MouseMove { x, y, dx, dy })
            }
            WM_LBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Left }),
            WM_LBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Left }),
            WM_RBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Right }),
            WM_RBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Right }),
            WM_MBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Middle }),
            WM_MBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Middle }),
            // v0.1.41: XButton（Side1/Side2）补全
            WM_XBUTTONDOWN => {
                let hi_word = ((ms_hook.mouseData >> 16) & 0xFFFF) as u16;
                let button = if hi_word == 1 { MouseButton::Side1 } else { MouseButton::Side2 };
                Some(RawInputEvent::MouseDown { button })
            }
            WM_XBUTTONUP => {
                let hi_word = ((ms_hook.mouseData >> 16) & 0xFFFF) as u16;
                let button = if hi_word == 1 { MouseButton::Side1 } else { MouseButton::Side2 };
                Some(RawInputEvent::MouseUp { button })
            }
            WM_MOUSEWHEEL => {
                // v0.1.41: 正确使用 i32 as i16 表达负向滚动
                let delta = (ms_hook.mouseData as i32) >> 16;
                Some(RawInputEvent::Scroll { dx: 0, dy: delta })
            }
            WM_MOUSEHWHEEL => {
                let delta = (ms_hook.mouseData as i32) >> 16;
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

        // v0.1.43: 识别合成键盘事件
        // LLKHF_INJECTED = 0x00000010, LLKHF_LOWER_IL_INJECTED = 0x00000002
        const LLKHF_INJECTED_MASK: u32 = 0x0000_0012;
        let is_injected = (kb_hook.flags & LLKHF_INJECTED_MASK) != 0;

        let event = match w_param as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // v0.1.43: 非合成键盘按下 → 喂 PreemptDetector（任意键即触发）
                if !is_injected {
                    feed_preempt(PhysicalInputSample::KeyDown);
                }
                Some(RawInputEvent::KeyDown { native_keycode: vk_code })
            }
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
    use std::sync::atomic::AtomicUsize;

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

    /// v0.1.43: 进入 RemoteControlling 时 PreemptDetector 必须被 reset，
    /// 避免从上一次 Session 残留累计值立即触发抢占。
    #[test]
    fn test_enter_remote_controlling_resets_preempt_detector() {
        let cap = WinInputCapture::new();
        // 先人为累计一些移动
        cap.preempt_detector.feed(PhysicalInputSample::Move { dx: 30, dy: 0 });
        // 再切到 REMOTE_CONTROLLING：set_control_state 应调 reset
        cap.set_control_state(CONTROL_STATE_REMOTE_CONTROLLING);
        // 再喂 30 像素位移（总累计 30，不应触发：阈值 40px）
        let decision = cap.preempt_detector.feed(PhysicalInputSample::Move { dx: 30, dy: 0 });
        assert!(
            matches!(decision, PreemptDecision::Idle),
            "切入 RemoteControlling 后 reset 没生效，被累计触发了"
        );
    }

    /// v0.1.43: set_preempt_callback 可被正确安装；
    /// 这里不启动真实 hook，只验证 Arc<Mutex<Option<_>>> 写入路径。
    #[test]
    fn test_set_and_clear_preempt_callback() {
        let cap = WinInputCapture::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_cb = Arc::clone(&hits);
        cap.set_preempt_callback(Box::new(move |_cause| {
            hits_cb.fetch_add(1, Ordering::SeqCst);
        }));
        assert!(cap.preempt_callback.lock().unwrap().is_some());
        cap.clear_preempt_callback();
        assert!(cap.preempt_callback.lock().unwrap().is_none());
    }

    /// v0.1.43: 回归保护——切到 LocalControlling / Idle 不应重置 PreemptDetector
    /// （reset 仅在进入 REMOTE_CONTROLLING 时触发）
    #[test]
    fn test_non_remote_state_transition_does_not_reset_detector() {
        let cap = WinInputCapture::new();
        cap.preempt_detector.feed(PhysicalInputSample::Move { dx: 20, dy: 0 });
        cap.set_control_state(CONTROL_STATE_LOCAL_CONTROLLING);
        // 再喂 35 像素（累计 20+35=55 > 40），应触发
        let decision = cap.preempt_detector.feed(PhysicalInputSample::Move { dx: 35, dy: 0 });
        assert!(
            matches!(decision, PreemptDecision::ShouldPreempt { .. }),
            "切 LocalControlling 不应 reset 累计值"
        );
    }
}
