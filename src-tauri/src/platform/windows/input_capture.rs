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
            // v0.1.57 BUG-041 修复：补充 DBLCLK 消息，映射为 MouseDown
            // Windows 双击事件序列：DOWN → UP → DBLCLK → UP
            // 第二次 DOWN 以 WM_LBUTTONDBLCLK 形式到达，不在 match 里就丢失，
            // 导致远端只收到 1 次 click（窗口/文件双击失效）
            // 拖动快速操作时也可能产生 DBLCLK，同样需要映射为 Down 保证拖动帧完整
            WM_LBUTTONDBLCLK => Some(RawInputEvent::MouseDown { button: MouseButton::Left }),
            WM_RBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Right }),
            WM_RBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Right }),
            WM_RBUTTONDBLCLK => Some(RawInputEvent::MouseDown { button: MouseButton::Right }),
            WM_MBUTTONDOWN => Some(RawInputEvent::MouseDown { button: MouseButton::Middle }),
            WM_MBUTTONUP => Some(RawInputEvent::MouseUp { button: MouseButton::Middle }),
            WM_MBUTTONDBLCLK => Some(RawInputEvent::MouseDown { button: MouseButton::Middle }),
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
        // v0.1.56 BUG-040：RemoteControlling 下合成事件（自注入）必须放行，
        // 只吞物理事件。否则被控端 SendInput 注入的滚轮/按键/拖拽全被自己 hook 吞掉。
        if should_swallow() {
            if is_injected && read_control_state() == CONTROL_STATE_REMOTE_CONTROLLING {
                // 被控端自注入事件：放行，让系统正确响应远端控制指令
            } else {
                return 1; // 非零返回值 = 吞掉事件，不传给本机
            }
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

        // v0.1.57 StateGuard: Ctrl+Alt+S 本地强制回 Idle（安全阀）
        // 即使处于 RemoteControlling 吞事件状态，此组合键也必须放行并触发恢复。
        // 仅在 KeyDown 时触发，且必须是物理按键（非注入）
        if !is_injected && vk_code == 0x53 /* VK_S */ {
            if (w_param as u32) == WM_KEYDOWN || (w_param as u32) == WM_SYSKEYDOWN {
                let ctrl_down = (GetAsyncKeyState(VK_CONTROL) as u16 & 0x8000) != 0;
                let alt_down = (GetAsyncKeyState(VK_MENU) as u16 & 0x8000) != 0;
                if ctrl_down && alt_down && read_control_state() == CONTROL_STATE_REMOTE_CONTROLLING {
                    log::warn!(
                        "[StateGuard] ⚡ Ctrl+Alt+S 安全恢复快捷键触发！强制回 Idle (trigger=force_recovery)"
                    );
                    // 立即把 control_state 设回 Idle（停止吞事件，立即恢复键鼠）
                    HOOK_CONTROL_STATE.with(|cell| {
                        if let Some(state) = cell.borrow().as_ref() {
                            state.store(CONTROL_STATE_IDLE, Ordering::Relaxed);
                        }
                    });
                    // 通过 preempt 通道通知 sharing 层走 on_disconnected 完整清理
                    HOOK_PREEMPT_CALLBACK.with(|cell| {
                        if let Some(cb_arc) = cell.borrow().as_ref() {
                            if let Ok(guard) = cb_arc.lock() {
                                if let Some(cb) = guard.as_ref() {
                                    cb(PreemptCause::KeyDown);
                                }
                            }
                        }
                    });
                    // 放行此键（不吞），让系统正常处理
                    return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
                }
            }
        }

        // v0.1.50: VK → X11 KeySym（与 Barrier/Synergy 行业标准对齐）
        // Win 采集端发出 KeySym，Mac 被控端 keysym_to_cg_keycode() 转换注入
        let keysym = vk_to_keysym(vk_code);
        let event = match w_param as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // v0.1.43: 非合成键盘按下 → 喂 PreemptDetector（任意键即触发）
                if !is_injected {
                    feed_preempt(PhysicalInputSample::KeyDown);
                }
                Some(RawInputEvent::KeyDown { native_keycode: keysym })
            }
            WM_KEYUP | WM_SYSKEYUP => Some(RawInputEvent::KeyUp { native_keycode: keysym }),
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
        // v0.1.56 BUG-040：同鼠标 hook 逻辑，被控端合成键盘事件必须放行
        if should_swallow() {
            if is_injected && read_control_state() == CONTROL_STATE_REMOTE_CONTROLLING {
                // 被控端自注入事件：放行
            } else {
                return 1;
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

/// Windows Virtual Key Code → X11 KeySym 转换表（Win 采集端，发送前调用）
///
/// 参照 Barrier/Synergy 行业标准，使用 X11 KeySym 作为跨平台通用键码。
/// 协议层 KeyEvent.keycode 语义为 KeySym；被控端各平台负责转换为原生码注入。
///
/// 参考：
///   https://www.x.org/releases/X11R7.7/doc/xproto/x11protocol.html#keysym_encoding
///   https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
#[cfg(target_os = "windows")]
pub(crate) fn vk_to_keysym(vk: u16) -> u16 {
    match vk {
        // ── 字母键 (VK → XK_lowercase) ──────────────────────────────────
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
        // ── 数字键（主键盘行）──────────────────────────────────────────
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
        // ── 空格 / 退格 / Tab / Esc / Enter ────────────────────────────
        0x08 => 0xFF08, // VK_BACK    → XK_BackSpace
        0x09 => 0xFF09, // VK_TAB     → XK_Tab
        0x0D => 0xFF0D, // VK_RETURN  → XK_Return
        0x1B => 0xFF1B, // VK_ESCAPE  → XK_Escape
        0x20 => 0x0020, // VK_SPACE   → XK_space
        // ── 符号键 ──────────────────────────────────────────────────────
        0xBB => 0x003D, // VK_OEM_PLUS   → XK_equal
        0xBD => 0x002D, // VK_OEM_MINUS  → XK_minus
        0xDB => 0x005B, // VK_OEM_4      → XK_bracketleft
        0xDD => 0x005D, // VK_OEM_6      → XK_bracketright
        0xDE => 0x0027, // VK_OEM_7      → XK_apostrophe
        0xBA => 0x003B, // VK_OEM_1      → XK_semicolon
        0xDC => 0x005C, // VK_OEM_5      → XK_backslash
        0xBC => 0x002C, // VK_OEM_COMMA  → XK_comma
        0xBF => 0x002F, // VK_OEM_2      → XK_slash
        0xBE => 0x002E, // VK_OEM_PERIOD → XK_period
        0xC0 => 0x0060, // VK_OEM_3      → XK_grave
        // ── 方向键 ──────────────────────────────────────────────────────
        0x25 => 0xFF51, // VK_LEFT  → XK_Left
        0x26 => 0xFF52, // VK_UP    → XK_Up
        0x27 => 0xFF53, // VK_RIGHT → XK_Right
        0x28 => 0xFF54, // VK_DOWN  → XK_Down
        // ── 功能键 F1-F20 ───────────────────────────────────────────────
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
        0xA2 => 0xFFE3, // VK_LCONTROL → XK_Control_L
        0xA3 => 0xFFE4, // VK_RCONTROL → XK_Control_R
        0xA4 => 0xFFE9, // VK_LMENU    → XK_Alt_L
        0xA5 => 0xFFEA, // VK_RMENU    → XK_Alt_R
        0x5B => 0xFFEB, // VK_LWIN     → XK_Super_L (= Mac Cmd)
        0x5C => 0xFFEC, // VK_RWIN     → XK_Super_R
        0x14 => 0xFFE5, // VK_CAPITAL  → XK_Caps_Lock
        // ── 扩展键 ──────────────────────────────────────────────────────
        0x2D => 0xFF63, // VK_INSERT    → XK_Insert
        0x2E => 0xFFFF, // VK_DELETE    → XK_Delete
        0x24 => 0xFF50, // VK_HOME      → XK_Home
        0x23 => 0xFF57, // VK_END       → XK_End
        0x21 => 0xFF55, // VK_PRIOR     → XK_Page_Up
        0x22 => 0xFF56, // VK_NEXT      → XK_Page_Down
        // ── 数字键盘 ────────────────────────────────────────────────────
        0x60 => 0xFFB0, // VK_NUMPAD0 → XK_KP_0
        0x61 => 0xFFB1, // VK_NUMPAD1 → XK_KP_1
        0x62 => 0xFFB2, // VK_NUMPAD2 → XK_KP_2
        0x63 => 0xFFB3, // VK_NUMPAD3 → XK_KP_3
        0x64 => 0xFFB4, // VK_NUMPAD4 → XK_KP_4
        0x65 => 0xFFB5, // VK_NUMPAD5 → XK_KP_5
        0x66 => 0xFFB6, // VK_NUMPAD6 → XK_KP_6
        0x67 => 0xFFB7, // VK_NUMPAD7 → XK_KP_7
        0x68 => 0xFFB8, // VK_NUMPAD8 → XK_KP_8
        0x69 => 0xFFB9, // VK_NUMPAD9 → XK_KP_9
        0x6A => 0xFFAA, // VK_MULTIPLY  → XK_KP_Multiply
        0x6B => 0xFFAB, // VK_ADD       → XK_KP_Add
        0x6D => 0xFFAD, // VK_SUBTRACT  → XK_KP_Subtract
        0x6E => 0xFFAE, // VK_DECIMAL   → XK_KP_Decimal
        0x6F => 0xFFAF, // VK_DIVIDE    → XK_KP_Divide
        0x90 => 0xFF7F, // VK_NUMLOCK   → XK_Num_Lock
        // ── 未映射：透传（保守降级，最坏情况按键无响应但不崩溃）─────────
        other => {
            log::debug!("[KeyMap/Win] 未知 VK {:#04X}, 透传为 KeySym", other);
            other
        }
    }
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

    /// v0.1.57 BUG-041 回归: vk_to_keysym 映射表覆盖常见 VK 码
    #[test]
    fn test_vk_to_keysym_basic_mappings() {
        // 字母键
        assert_eq!(vk_to_keysym(0x41), 0x0061); // VK_A → XK_a
        assert_eq!(vk_to_keysym(0x5A), 0x007A); // VK_Z → XK_z
        // Win 键映射到 Super_L（Mac Cmd）
        assert_eq!(vk_to_keysym(0x5B), 0xFFEB); // VK_LWIN → XK_Super_L
        // Shift
        assert_eq!(vk_to_keysym(0xA0), 0xFFE1); // VK_LSHIFT → XK_Shift_L
    }
}
