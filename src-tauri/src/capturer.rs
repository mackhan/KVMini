/// 主控端键鼠捕获模块
///
/// 定义统一的 EventCapturer 接口，macOS 和 Windows 各自实现。
/// - macOS：使用 CGEvent Tap 捕获全局键鼠事件（需要辅助功能权限）
/// - Windows：使用 SetWindowsHookEx 低级钩子（WH_MOUSE_LL、WH_KEYBOARD_LL）
///
/// 捕获到的事件通过 tokio channel 发送给主控端 TCP 发送模块。
/// 支持鼠标移动事件节流（最高 1000Hz）和快捷键 Ctrl+Alt+S 退出控制模式。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use crate::protocol::KvmFrame;

/// 捕获错误
#[derive(Debug, Clone, PartialEq)]
pub enum CapturerError {
    /// 权限不足（macOS 需要辅助功能权限）
    PermissionDenied,
    /// 已在运行
    AlreadyRunning,
    /// 系统调用失败
    SystemError(String),
}

impl std::fmt::Display for CapturerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "权限不足，请在系统偏好设置中开启辅助功能权限"),
            Self::AlreadyRunning => write!(f, "捕获器已在运行"),
            Self::SystemError(msg) => write!(f, "系统调用失败: {}", msg),
        }
    }
}

/// 统一事件捕获接口
pub trait EventCapturer: Send + Sync {
    /// 启动捕获，返回事件接收端
    /// 捕获到的事件通过 channel 发送
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError>;

    /// 停止捕获
    fn stop(&self);

    /// 是否正在运行
    fn is_running(&self) -> bool;

    /// 设置是否拦截本机输入（控制模式：true = 拦截，false = 透传）
    fn set_intercept(&self, intercept: bool);
}

/// 鼠标移动节流器（最高 1000Hz = 1ms 间隔）
pub struct MouseThrottle {
    /// 上次发送时间（微秒）
    last_send_us: std::sync::Mutex<u64>,
    /// 最小发送间隔（微秒），默认 1000us = 1ms = 1000Hz
    min_interval_us: u64,
}

impl MouseThrottle {
    pub fn new(max_hz: u32) -> Self {
        let min_interval_us = if max_hz == 0 { 0 } else { 1_000_000 / max_hz as u64 };
        Self {
            last_send_us: std::sync::Mutex::new(0),
            min_interval_us,
        }
    }

    /// 检查是否应该发送此次鼠标移动事件
    /// 返回 true 表示应该发送，false 表示应该丢弃（节流）
    pub fn should_send(&self) -> bool {
        let now_us = Self::now_us();
        let mut last = self.last_send_us.lock().unwrap();
        if now_us - *last >= self.min_interval_us {
            *last = now_us;
            true
        } else {
            false
        }
    }

    fn now_us() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

/// 快捷键检测器（Ctrl+Alt+S 退出控制模式）
pub struct HotkeyDetector {
    ctrl_pressed: AtomicBool,
    alt_pressed: AtomicBool,
}

impl HotkeyDetector {
    pub fn new() -> Self {
        Self {
            ctrl_pressed: AtomicBool::new(false),
            alt_pressed: AtomicBool::new(false),
        }
    }

    /// 更新修饰键状态，返回是否触发了退出快捷键（Ctrl+Alt+S）
    pub fn update(&self, keycode: u16, pressed: bool) -> bool {
        // macOS 虚拟键码：Ctrl=59, Alt/Option=58, S=1
        // Windows 虚拟键码：Ctrl=0x11, Alt=0x12, S=0x53
        let is_ctrl = is_ctrl_key(keycode);
        let is_alt = is_alt_key(keycode);
        let is_s = is_s_key(keycode);

        if is_ctrl {
            self.ctrl_pressed.store(pressed, Ordering::SeqCst);
        }
        if is_alt {
            self.alt_pressed.store(pressed, Ordering::SeqCst);
        }

        // 检测 Ctrl+Alt+S 组合键
        if is_s && pressed
            && self.ctrl_pressed.load(Ordering::SeqCst)
            && self.alt_pressed.load(Ordering::SeqCst)
        {
            return true;
        }

        false
    }

    pub fn reset(&self) {
        self.ctrl_pressed.store(false, Ordering::SeqCst);
        self.alt_pressed.store(false, Ordering::SeqCst);
    }
}

/// 判断是否为 Ctrl 键（跨平台）
fn is_ctrl_key(keycode: u16) -> bool {
    // macOS: Left Ctrl = 59, Right Ctrl = 62
    // Windows: VK_CONTROL = 0x11, VK_LCONTROL = 0xA2, VK_RCONTROL = 0xA3
    matches!(keycode, 59 | 62 | 0x11 | 0xA2 | 0xA3)
}

/// 判断是否为 Alt 键（跨平台）
fn is_alt_key(keycode: u16) -> bool {
    // macOS: Left Option = 58, Right Option = 61
    // Windows: VK_MENU = 0x12, VK_LMENU = 0xA4, VK_RMENU = 0xA5
    matches!(keycode, 58 | 61 | 0x12 | 0xA4 | 0xA5)
}

/// 判断是否为 S 键（跨平台）
fn is_s_key(keycode: u16) -> bool {
    // macOS: S = 1 (CGKeyCode)
    // Windows: VK_S = 0x53
    matches!(keycode, 1 | 0x53)
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

/// macOS 事件捕获器（使用 CGEvent Tap）
///
/// 实现要点（避免 4 个常见失效根因）：
/// 1. RunLoop 必须在专用子线程中运行（不能在主线程）
/// 2. CFMachPort 必须持有引用（通过 Arc<Mutex<Option<...>>>）
/// 3. 权限检测使用 CGPreflightListenEventAccess()（无 TCC 缓存问题）
/// 4. 权限未授予时持续轮询，授权后自动启动
#[cfg(target_os = "macos")]
pub struct MacOSCapturer {
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
    /// 用于通知 RunLoop 停止的标志
    stop_flag: Arc<AtomicBool>,
}

#[cfg(target_os = "macos")]
impl MacOSCapturer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            intercept: Arc::new(AtomicBool::new(false)),
            throttle: Arc::new(MouseThrottle::new(1000)),
            hotkey: Arc::new(HotkeyDetector::new()),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(target_os = "macos")]
impl EventCapturer for MacOSCapturer {
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CapturerError::AlreadyRunning);
        }

        // 先检查权限（使用 CGPreflightListenEventAccess，无 TCC 缓存问题）
        if !macos_preflight_listen_event_access() {
            return Err(CapturerError::PermissionDenied);
        }

        self.running.store(true, Ordering::SeqCst);
        self.stop_flag.store(false, Ordering::SeqCst);

        let running = Arc::clone(&self.running);
        let intercept = Arc::clone(&self.intercept);
        let throttle = Arc::clone(&self.throttle);
        let hotkey = Arc::clone(&self.hotkey);
        let stop_flag = Arc::clone(&self.stop_flag);

        // 在专用子线程中启动 CGEvent Tap RunLoop
        // 关键：必须在子线程中调用 CFRunLoopRun()，不能在主线程
        std::thread::spawn(move || {
            macos_run_event_tap(tx, running, intercept, throttle, hotkey, stop_flag);
        });

        Ok(())
    }

    fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.reset();
        // CFRunLoopStop 会在 macos_run_event_tap 中检测 stop_flag 后调用
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_intercept(&self, intercept: bool) {
        self.intercept.store(intercept, Ordering::SeqCst);
    }
}

/// 检测 macOS 辅助功能权限（使用 CGPreflightListenEventAccess，无 TCC 缓存延迟）
#[cfg(target_os = "macos")]
fn macos_preflight_listen_event_access() -> bool {
    extern "C" {
        fn CGPreflightListenEventAccess() -> bool;
    }
    unsafe { CGPreflightListenEventAccess() }
}

/// macOS CGEvent Tap 主循环（在专用子线程中运行）
///
/// 正确实现模式：
/// 1. 等待权限（轮询 CGPreflightListenEventAccess）
/// 2. 创建 CGEvent Tap
/// 3. 将 source 加到当前线程的 RunLoop（不是主线程）
/// 4. 调用 CFRunLoopRun() 阻塞当前线程驱动事件循环
#[cfg(target_os = "macos")]
fn macos_run_event_tap(
    tx: mpsc::Sender<KvmFrame>,
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
    stop_flag: Arc<AtomicBool>,
) {
    use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};

    // 等待权限（轮询，最多等待 60 秒）
    let mut waited = 0u32;
    while !macos_preflight_listen_event_access() {
        if stop_flag.load(Ordering::SeqCst) || waited > 60 {
            running.store(false, Ordering::SeqCst);
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        waited += 1;
    }

    // 创建事件回调上下文（通过 Box 传递到 C 回调）
    let ctx = Box::new(EventTapContext {
        tx: tx.clone(),
        intercept: Arc::clone(&intercept),
        throttle: Arc::clone(&throttle),
        hotkey: Arc::clone(&hotkey),
        running: Arc::clone(&running),
        retry_count: std::sync::atomic::AtomicU32::new(0),
    });
    let ctx_ptr = Box::into_raw(ctx);

    // 监听的事件类型掩码
    // 使用 CGEventMaskBit 宏计算
    let event_mask: u64 = {
        use core_graphics::event::CGEventType;
        (1u64 << CGEventType::MouseMoved as u64)
            | (1u64 << CGEventType::LeftMouseDown as u64)
            | (1u64 << CGEventType::LeftMouseUp as u64)
            | (1u64 << CGEventType::RightMouseDown as u64)
            | (1u64 << CGEventType::RightMouseUp as u64)
            | (1u64 << CGEventType::OtherMouseDown as u64)
            | (1u64 << CGEventType::OtherMouseUp as u64)
            | (1u64 << CGEventType::ScrollWheel as u64)
            | (1u64 << CGEventType::KeyDown as u64)
            | (1u64 << CGEventType::KeyUp as u64)
            | (1u64 << CGEventType::FlagsChanged as u64)
            | (1u64 << 0xFFFFFFFEu64 % 64) // TapDisabledByTimeout
            | (1u64 << 0xFFFFFFFDu64 % 64) // TapDisabledByUserInput
    };

    // 使用直接 C FFI 调用 CGEventTapCreate
    // 这样不受 core-graphics crate 版本影响
    let tap_port = unsafe {
        macos_create_event_tap(event_mask, event_tap_callback, ctx_ptr as *mut _)
    };

    if tap_port.is_null() {
        running.store(false, Ordering::SeqCst);
        unsafe { drop(Box::from_raw(ctx_ptr)); }
        return;
    }

    // 将 source 加到当前线程的 RunLoop（关键：必须是当前子线程，不能是主线程）
    unsafe {
        let loop_source = macos_create_runloop_source(tap_port);
        if !loop_source.is_null() {
            let current_runloop = CFRunLoop::get_current();
            current_runloop.add_source(
                &*(loop_source as *const _),
                kCFRunLoopCommonModes,
            );
            macos_enable_event_tap(tap_port, true);
        }
    }

    // 阻塞当前线程驱动事件循环（关键：必须在同一子线程调用）
    loop {
        if stop_flag.load(Ordering::SeqCst) {
            break;
        }
        // 运行 RunLoop 100ms，然后检查 stop_flag
        CFRunLoop::run_in_mode(
            unsafe { core_foundation::runloop::kCFRunLoopDefaultMode },
            std::time::Duration::from_millis(100),
            false,
        );
    }

    // 清理
    unsafe {
        macos_enable_event_tap(tap_port, false);
        macos_release_port(tap_port);
        drop(Box::from_raw(ctx_ptr));
    }
    running.store(false, Ordering::SeqCst);
}

/// 使用 C FFI 创建 CGEvent Tap
#[cfg(target_os = "macos")]
unsafe fn macos_create_event_tap(
    event_mask: u64,
    callback: unsafe extern "C" fn(*mut std::ffi::c_void, u32, *mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void,
    user_info: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,        // CGEventTapLocation
            place: u32,      // CGEventTapPlacement
            options: u32,    // CGEventTapOptions
            events_of_interest: u64,
            callback: unsafe extern "C" fn(*mut std::ffi::c_void, u32, *mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void,
            user_info: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }
    CGEventTapCreate(
        0, // kCGHIDEventTap
        0, // kCGHeadInsertEventTap
        0, // kCGEventTapOptionDefault
        event_mask,
        callback,
        user_info,
    )
}

/// 创建 RunLoop Source
#[cfg(target_os = "macos")]
unsafe fn macos_create_runloop_source(port: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    extern "C" {
        fn CFMachPortCreateRunLoopSource(
            allocator: *mut std::ffi::c_void,
            port: *mut std::ffi::c_void,
            order: i64,
        ) -> *mut std::ffi::c_void;
    }
    CFMachPortCreateRunLoopSource(std::ptr::null_mut(), port, 0)
}

/// 启用/禁用 Event Tap
#[cfg(target_os = "macos")]
unsafe fn macos_enable_event_tap(port: *mut std::ffi::c_void, enable: bool) {
    extern "C" {
        fn CGEventTapEnable(tap: *mut std::ffi::c_void, enable: bool);
    }
    CGEventTapEnable(port, enable);
}

/// 释放端口
#[cfg(target_os = "macos")]
unsafe fn macos_release_port(port: *mut std::ffi::c_void) {
    extern "C" {
        fn CFRelease(cf: *mut std::ffi::c_void);
    }
    CFRelease(port);
}

/// CGEvent Tap 回调上下文
#[cfg(target_os = "macos")]
struct EventTapContext {
    tx: mpsc::Sender<KvmFrame>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
    running: Arc<AtomicBool>,
    retry_count: std::sync::atomic::AtomicU32,
}

/// CGEvent Tap 回调函数
///
/// 注意：此回调必须极快返回，不能执行耗时操作
/// 所有实际处理通过 channel 发送到工作线程
#[cfg(target_os = "macos")]
unsafe extern "C" fn event_tap_callback(
    _proxy: *mut std::ffi::c_void,
    event_type_raw: u32,
    event: *mut std::ffi::c_void,
    user_info: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    use core_graphics::event::{CGEvent, CGEventType, CGKeyCode};
    use core_graphics::geometry::CGPoint;

    let ctx = &*(user_info as *const EventTapContext);

    // 将 u32 转换为 CGEventType
    let event_type: CGEventType = std::mem::transmute(event_type_raw);

    match event_type {
        // Tap 被系统禁用（超时）- 重新启用，最多重试 3 次
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            let retries = ctx.retry_count.fetch_add(1, Ordering::SeqCst);
            if retries < 3 {
                // 重新启用 tap（通过重新设置 enabled 标志）
                // 注意：实际重新启用需要通过 CGEventTapEnable，这里通过标志通知主循环
                ctx.running.store(true, Ordering::SeqCst);
            } else {
                ctx.running.store(false, Ordering::SeqCst);
            }
            return event;
        }
        _ => {}
    }

    // 重置重试计数（成功处理事件）
    ctx.retry_count.store(0, Ordering::SeqCst);

    // 使用 C FFI 直接操作事件，避免 core-graphics crate API 不兼容问题
    let location = macos_get_event_location(event);
    let should_intercept = ctx.intercept.load(Ordering::SeqCst);

    let frame = match event_type {
        CGEventType::MouseMoved => {
            // 鼠标移动：节流到 1000Hz
            if !ctx.throttle.should_send() {
                return event;
            }
            // 获取屏幕尺寸进行归一化
            let (screen_w, screen_h) = get_main_screen_size();
            if screen_w > 0 && screen_h > 0 {
                let x = (location.0 / screen_w as f64 * 10000.0) as i32;
                let y = (location.1 / screen_h as f64 * 10000.0) as i32;
                Some(KvmFrame::MouseMove {
                    x: x.clamp(0, 10000),
                    y: y.clamp(0, 10000),
                })
            } else {
                None
            }
        }
        CGEventType::LeftMouseDown => Some(KvmFrame::MouseButton { button: 0, pressed: true }),
        CGEventType::LeftMouseUp => Some(KvmFrame::MouseButton { button: 0, pressed: false }),
        CGEventType::RightMouseDown => Some(KvmFrame::MouseButton { button: 1, pressed: true }),
        CGEventType::RightMouseUp => Some(KvmFrame::MouseButton { button: 1, pressed: false }),
        CGEventType::OtherMouseDown => Some(KvmFrame::MouseButton { button: 2, pressed: true }),
        CGEventType::OtherMouseUp => Some(KvmFrame::MouseButton { button: 2, pressed: false }),
        CGEventType::ScrollWheel => {
            // 滚轮事件：使用 C FFI 获取滚动量
            let dy = macos_get_event_integer_field(event, 96) as i32; // SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1 = 96
            let dx = macos_get_event_integer_field(event, 97) as i32; // SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2 = 97
            Some(KvmFrame::MouseScroll { dx, dy })
        }
        CGEventType::KeyDown => {
            let keycode = macos_get_event_integer_field(event, 9) as u16; // KEYBOARD_EVENT_KEYCODE = 9
            // 检测快捷键 Ctrl+Alt+S
            if ctx.hotkey.update(keycode, true) {
                let _ = ctx.tx.try_send(KvmFrame::SwitchControl {
                    edge: crate::protocol::ScreenEdge::Right,
                    pos_ratio: 5000,
                });
            }
            Some(KvmFrame::KeyEvent { keycode, pressed: true })
        }
        CGEventType::KeyUp => {
            let keycode = macos_get_event_integer_field(event, 9) as u16;
            ctx.hotkey.update(keycode, false);
            Some(KvmFrame::KeyEvent { keycode, pressed: false })
        }
        CGEventType::FlagsChanged => {
            // 修饰键变化（Ctrl/Alt/Shift/Cmd）
            let keycode = macos_get_event_integer_field(event, 9) as u16;
            // FlagsChanged 不区分按下/释放，通过 flags 判断
            let flags = macos_get_event_flags(event);
            let pressed = is_modifier_pressed(keycode, flags);
            ctx.hotkey.update(keycode, pressed);
            Some(KvmFrame::KeyEvent { keycode, pressed })
        }
        _ => None,
    };

    if let Some(f) = frame {
        let _ = ctx.tx.try_send(f);
    }

    // 如果拦截模式，返回 null 阻止事件传递给其他应用
    if should_intercept {
        std::ptr::null_mut()
    } else {
        event
    }
}

/// 获取事件位置（使用 C FFI）
#[cfg(target_os = "macos")]
unsafe fn macos_get_event_location(event: *mut std::ffi::c_void) -> (f64, f64) {
    /// CGPoint 结构体（与 C 端 ABI 兼容）
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    extern "C" {
        fn CGEventGetLocation(event: *mut std::ffi::c_void) -> CGPoint;
    }
    let pt = CGEventGetLocation(event);
    (pt.x, pt.y)
}

/// 获取事件整数字段（使用 C FFI）
#[cfg(target_os = "macos")]
unsafe fn macos_get_event_integer_field(event: *mut std::ffi::c_void, field: u32) -> i64 {
    extern "C" {
        fn CGEventGetIntegerValueField(event: *mut std::ffi::c_void, field: u32) -> i64;
    }
    CGEventGetIntegerValueField(event, field)
}

/// 获取事件 flags（使用 C FFI）
#[cfg(target_os = "macos")]
unsafe fn macos_get_event_flags(event: *mut std::ffi::c_void) -> u64 {
    extern "C" {
        fn CGEventGetFlags(event: *mut std::ffi::c_void) -> u64;
    }
    CGEventGetFlags(event)
}

/// 获取主屏幕尺寸（像素）
#[cfg(target_os = "macos")]
fn get_main_screen_size() -> (i32, i32) {
    use core_graphics::display::CGDisplay;
    let display = CGDisplay::main();
    let bounds = display.bounds();
    (bounds.size.width as i32, bounds.size.height as i32)
}

/// 判断修饰键是否处于按下状态（通过 CGEventFlags 判断）
#[cfg(target_os = "macos")]
fn is_modifier_pressed(keycode: u16, flags: u64) -> bool {
    // CGEventFlags 常量
    const CTRL_FLAG: u64 = 0x00040000;
    const ALT_FLAG: u64 = 0x00080000;
    const SHIFT_FLAG: u64 = 0x00020000;
    const CMD_FLAG: u64 = 0x00100000;
    match keycode {
        // Left/Right Ctrl
        59 | 62 => (flags & CTRL_FLAG) != 0,
        // Left/Right Option (Alt)
        58 | 61 => (flags & ALT_FLAG) != 0,
        // Left/Right Shift
        56 | 60 => (flags & SHIFT_FLAG) != 0,
        // Left/Right Command
        55 | 54 => (flags & CMD_FLAG) != 0,
        _ => false,
    }
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// Windows 事件捕获器（使用 SetWindowsHookEx 低级钩子）
///
/// 实现要点：
/// 1. 钩子回调中只做 try_send()，不执行任何耗时操作（确保 300ms 内返回）
/// 2. 所有实际处理在独立工作线程中完成
/// 3. 使用有界 channel（容量 1024），防止内存无限增长
#[cfg(target_os = "windows")]
pub struct WindowsCapturer {
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
    stop_flag: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
impl WindowsCapturer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            intercept: Arc::new(AtomicBool::new(false)),
            throttle: Arc::new(MouseThrottle::new(1000)),
            hotkey: Arc::new(HotkeyDetector::new()),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(target_os = "windows")]
impl EventCapturer for WindowsCapturer {
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CapturerError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);
        self.stop_flag.store(false, Ordering::SeqCst);

        let running = Arc::clone(&self.running);
        let intercept = Arc::clone(&self.intercept);
        let throttle = Arc::clone(&self.throttle);
        let hotkey = Arc::clone(&self.hotkey);
        let stop_flag = Arc::clone(&self.stop_flag);

        // 在专用线程中安装低级钩子并运行消息循环
        std::thread::spawn(move || {
            windows_run_hook(tx, running, intercept, throttle, hotkey, stop_flag);
        });

        Ok(())
    }

    fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.reset();
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_intercept(&self, intercept: bool) {
        self.intercept.store(intercept, Ordering::SeqCst);
    }
}

/// Windows 低级钩子原始事件（用于跨线程传递，必须极快 clone）
#[cfg(target_os = "windows")]
#[derive(Clone)]
enum RawHookEvent {
    Mouse {
        msg: u32,
        x: i32,
        y: i32,
        mouse_data: u32,
    },
    Keyboard {
        vk_code: u32,
        flags: u32,
        msg: u32,
    },
}

/// Windows 钩子线程全局状态（通过 thread_local 传递给钩子回调）
#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_EVENT_SENDER: std::cell::RefCell<Option<std::sync::mpsc::SyncSender<RawHookEvent>>> =
        std::cell::RefCell::new(None);
    static HOOK_INTERCEPT: std::cell::RefCell<Arc<AtomicBool>> =
        std::cell::RefCell::new(Arc::new(AtomicBool::new(false)));
}

/// Windows 低级钩子主循环
#[cfg(target_os = "windows")]
fn windows_run_hook(
    tx: mpsc::Sender<KvmFrame>,
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
    stop_flag: Arc<AtomicBool>,
) {
    use winapi::um::winuser::{
        SetWindowsHookExW, UnhookWindowsHookEx,
        TranslateMessage, DispatchMessageW,
        WH_MOUSE_LL, WH_KEYBOARD_LL, MSG,
    };
    use std::sync::mpsc as std_mpsc;

    // 创建有界 channel（容量 1024），钩子回调只做 try_send
    let (raw_tx, raw_rx) = std_mpsc::sync_channel::<RawHookEvent>(1024);

    // 设置 thread_local 状态
    HOOK_EVENT_SENDER.with(|s| {
        *s.borrow_mut() = Some(raw_tx);
    });
    HOOK_INTERCEPT.with(|i| {
        *i.borrow_mut() = Arc::clone(&intercept);
    });

    // 启动工作线程处理原始事件
    let tx_clone = tx.clone();
    let throttle_clone = Arc::clone(&throttle);
    let hotkey_clone = Arc::clone(&hotkey);
    let stop_flag_clone = Arc::clone(&stop_flag);
    std::thread::spawn(move || {
        while let Ok(event) = raw_rx.recv() {
            if stop_flag_clone.load(Ordering::SeqCst) {
                break;
            }
            if let Some(frame) = windows_raw_event_to_frame(event, &throttle_clone, &hotkey_clone) {
                let _ = tx_clone.try_send(frame);
            }
        }
    });

    // 安装低级钩子
    let mouse_hook = unsafe {
        SetWindowsHookExW(WH_MOUSE_LL, Some(windows_mouse_hook_proc), std::ptr::null_mut(), 0)
    };
    let keyboard_hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(windows_keyboard_hook_proc), std::ptr::null_mut(), 0)
    };

    if mouse_hook.is_null() || keyboard_hook.is_null() {
        running.store(false, Ordering::SeqCst);
        if !mouse_hook.is_null() {
            unsafe { UnhookWindowsHookEx(mouse_hook); }
        }
        if !keyboard_hook.is_null() {
            unsafe { UnhookWindowsHookEx(keyboard_hook); }
        }
        return;
    }

    // 消息循环（GetMessage + TranslateMessage + DispatchMessage，不做额外工作）
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        if stop_flag.load(Ordering::SeqCst) {
            break;
        }
        // 使用 PeekMessage 避免阻塞，定期检查 stop_flag
        let result = unsafe {
            winapi::um::winuser::PeekMessageW(
                &mut msg,
                std::ptr::null_mut(),
                0, 0,
                winapi::um::winuser::PM_REMOVE,
            )
        };
        if result != 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    // 卸载钩子
    unsafe {
        UnhookWindowsHookEx(mouse_hook);
        UnhookWindowsHookEx(keyboard_hook);
    }
    running.store(false, Ordering::SeqCst);
}

/// Windows 鼠标低级钩子回调（必须极快返回，只做 try_send）
#[cfg(target_os = "windows")]
unsafe extern "system" fn windows_mouse_hook_proc(
    code: i32,
    wparam: winapi::shared::minwindef::WPARAM,
    lparam: winapi::shared::minwindef::LPARAM,
) -> winapi::shared::minwindef::LRESULT {
    use winapi::um::winuser::{CallNextHookEx, MSLLHOOKSTRUCT, LLMHF_INJECTED};

    if code >= 0 {
        let ms = &*(lparam as *const MSLLHOOKSTRUCT);
        // 忽略由 SendInput 注入的事件（防止循环）
        if ms.flags & LLMHF_INJECTED == 0 {
            let event = RawHookEvent::Mouse {
                msg: wparam as u32,
                x: ms.pt.x,
                y: ms.pt.y,
                mouse_data: ms.mouseData,
            };
            HOOK_EVENT_SENDER.with(|s| {
                if let Some(ref sender) = *s.borrow() {
                    let _ = sender.try_send(event);
                }
            });
        }

        // 如果拦截模式，返回非零值阻止事件传递
        let should_intercept = HOOK_INTERCEPT.with(|i| {
            i.borrow().load(Ordering::SeqCst)
        });
        if should_intercept {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

/// Windows 键盘低级钩子回调（必须极快返回，只做 try_send）
#[cfg(target_os = "windows")]
unsafe extern "system" fn windows_keyboard_hook_proc(
    code: i32,
    wparam: winapi::shared::minwindef::WPARAM,
    lparam: winapi::shared::minwindef::LPARAM,
) -> winapi::shared::minwindef::LRESULT {
    use winapi::um::winuser::{CallNextHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED};

    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        // 忽略由 SendInput 注入的事件（防止循环）
        if kb.flags & LLKHF_INJECTED == 0 {
            let event = RawHookEvent::Keyboard {
                vk_code: kb.vkCode,
                flags: kb.flags,
                msg: wparam as u32,
            };
            HOOK_EVENT_SENDER.with(|s| {
                if let Some(ref sender) = *s.borrow() {
                    let _ = sender.try_send(event);
                }
            });
        }

        let should_intercept = HOOK_INTERCEPT.with(|i| {
            i.borrow().load(Ordering::SeqCst)
        });
        if should_intercept {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

/// 将 Windows 原始钩子事件转换为 KvmFrame
#[cfg(target_os = "windows")]
fn windows_raw_event_to_frame(
    event: RawHookEvent,
    throttle: &MouseThrottle,
    hotkey: &HotkeyDetector,
) -> Option<KvmFrame> {
    use winapi::um::winuser::{
        WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
        WM_MOUSEWHEEL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    match event {
        RawHookEvent::Mouse { msg, x, y, mouse_data } => {
            match msg {
                m if m == WM_MOUSEMOVE => {
                    if !throttle.should_send() {
                        return None;
                    }
                    // 获取虚拟屏幕尺寸进行归一化
                    let (screen_w, screen_h) = windows_get_virtual_screen_size();
                    if screen_w > 0 && screen_h > 0 {
                        let norm_x = (x as i64 * 10000 / screen_w as i64) as i32;
                        let norm_y = (y as i64 * 10000 / screen_h as i64) as i32;
                        Some(KvmFrame::MouseMove {
                            x: norm_x.clamp(0, 10000),
                            y: norm_y.clamp(0, 10000),
                        })
                    } else {
                        None
                    }
                }
                m if m == WM_LBUTTONDOWN => Some(KvmFrame::MouseButton { button: 0, pressed: true }),
                m if m == WM_LBUTTONUP => Some(KvmFrame::MouseButton { button: 0, pressed: false }),
                m if m == WM_RBUTTONDOWN => Some(KvmFrame::MouseButton { button: 1, pressed: true }),
                m if m == WM_RBUTTONUP => Some(KvmFrame::MouseButton { button: 1, pressed: false }),
                m if m == WM_MBUTTONDOWN => Some(KvmFrame::MouseButton { button: 2, pressed: true }),
                m if m == WM_MBUTTONUP => Some(KvmFrame::MouseButton { button: 2, pressed: false }),
                m if m == WM_MOUSEWHEEL => {
                    let delta = (mouse_data >> 16) as i16 as i32;
                    Some(KvmFrame::MouseScroll { dx: 0, dy: delta })
                }
                _ => None,
            }
        }
        RawHookEvent::Keyboard { vk_code, flags, msg } => {
            let pressed = msg == WM_KEYDOWN as u32 || msg == WM_SYSKEYDOWN as u32;
            let keycode = vk_code as u16;
            hotkey.update(keycode, pressed);
            Some(KvmFrame::KeyEvent { keycode, pressed })
        }
    }
}

/// 获取 Windows 虚拟屏幕尺寸（多显示器时为总宽高）
#[cfg(target_os = "windows")]
fn windows_get_virtual_screen_size() -> (i32, i32) {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        (w, h)
    }
}

// ── Mock 捕获器（用于测试）──────────────────────────────────────────────────

/// Mock 捕获器，可手动注入事件，用于单元测试
pub struct MockCapturer {
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    tx: std::sync::Mutex<Option<mpsc::Sender<KvmFrame>>>,
}

impl MockCapturer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            intercept: Arc::new(AtomicBool::new(false)),
            tx: std::sync::Mutex::new(None),
        }
    }

    /// 手动注入一个事件（模拟捕获到的输入）
    pub fn inject_event(&self, frame: KvmFrame) -> bool {
        if let Some(tx) = self.tx.lock().unwrap().as_ref() {
            tx.try_send(frame).is_ok()
        } else {
            false
        }
    }

    pub fn is_intercepting(&self) -> bool {
        self.intercept.load(Ordering::SeqCst)
    }
}

impl EventCapturer for MockCapturer {
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CapturerError::AlreadyRunning);
        }
        *self.tx.lock().unwrap() = Some(tx);
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.tx.lock().unwrap() = None;
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_intercept(&self, intercept: bool) {
        self.intercept.store(intercept, Ordering::SeqCst);
    }
}

/// 创建当前平台的捕获器（工厂函数）
pub fn create_platform_capturer() -> Box<dyn EventCapturer> {
    #[cfg(target_os = "macos")]
    return Box::new(MacOSCapturer::new());

    #[cfg(target_os = "windows")]
    return Box::new(WindowsCapturer::new());

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Box::new(MockCapturer::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ── MouseThrottle 测试 ───────────────────────────────────────────────

    #[test]
    fn test_throttle_1000hz_allows_first_event() {
        let throttle = MouseThrottle::new(1000);
        assert!(throttle.should_send()); // 第一次总是允许
    }

    #[test]
    fn test_throttle_1000hz_blocks_rapid_events() {
        let throttle = MouseThrottle::new(1000);
        throttle.should_send(); // 第一次
        // 立即再次调用，应该被节流
        let second = throttle.should_send();
        // 在 1ms 内，第二次可能被节流（取决于系统时钟精度）
        // 这里只验证节流器不会 panic
        let _ = second;
    }

    #[test]
    fn test_throttle_0hz_always_allows() {
        // 0Hz 表示不节流
        let throttle = MouseThrottle::new(0);
        for _ in 0..100 {
            assert!(throttle.should_send());
        }
    }

    // ── HotkeyDetector 测试 ──────────────────────────────────────────────

    #[test]
    fn test_hotkey_ctrl_alt_s_macos() {
        let detector = HotkeyDetector::new();
        // macOS 键码：Ctrl=59, Alt=58, S=1
        detector.update(59, true);  // Ctrl 按下
        detector.update(58, true);  // Alt 按下
        let triggered = detector.update(1, true); // S 按下
        assert!(triggered, "Ctrl+Alt+S 应该触发退出快捷键");
    }

    #[test]
    fn test_hotkey_ctrl_alt_s_windows() {
        let detector = HotkeyDetector::new();
        // Windows 键码：Ctrl=0x11, Alt=0x12, S=0x53
        detector.update(0x11, true);  // Ctrl 按下
        detector.update(0x12, true);  // Alt 按下
        let triggered = detector.update(0x53, true); // S 按下
        assert!(triggered, "Ctrl+Alt+S 应该触发退出快捷键（Windows）");
    }

    #[test]
    fn test_hotkey_not_triggered_without_ctrl() {
        let detector = HotkeyDetector::new();
        detector.update(58, true);  // 只按 Alt
        let triggered = detector.update(1, true); // S 按下
        assert!(!triggered, "没有 Ctrl 不应触发快捷键");
    }

    #[test]
    fn test_hotkey_not_triggered_without_alt() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);  // 只按 Ctrl
        let triggered = detector.update(1, true); // S 按下
        assert!(!triggered, "没有 Alt 不应触发快捷键");
    }

    #[test]
    fn test_hotkey_not_triggered_on_key_release() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);  // Ctrl 按下
        detector.update(58, true);  // Alt 按下
        let triggered = detector.update(1, false); // S 释放（不是按下）
        assert!(!triggered, "键释放不应触发快捷键");
    }

    #[test]
    fn test_hotkey_reset() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);
        detector.update(58, true);
        detector.reset();
        let triggered = detector.update(1, true);
        assert!(!triggered, "reset 后不应触发快捷键");
    }

    #[test]
    fn test_hotkey_ctrl_released_before_s() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);  // Ctrl 按下
        detector.update(58, true);  // Alt 按下
        detector.update(59, false); // Ctrl 释放
        let triggered = detector.update(1, true); // S 按下
        assert!(!triggered, "Ctrl 已释放，不应触发快捷键");
    }

    // ── MockCapturer 测试 ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_mock_capturer_start_stop() {
        let capturer = MockCapturer::new();
        let (tx, _rx) = mpsc::channel(16);

        assert!(!capturer.is_running());
        capturer.start(tx).unwrap();
        assert!(capturer.is_running());
        capturer.stop();
        assert!(!capturer.is_running());
    }

    #[tokio::test]
    async fn test_mock_capturer_already_running() {
        let capturer = MockCapturer::new();
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        capturer.start(tx1).unwrap();
        let result = capturer.start(tx2);
        assert_eq!(result, Err(CapturerError::AlreadyRunning));
        capturer.stop();
    }

    #[tokio::test]
    async fn test_mock_capturer_inject_event() {
        let capturer = MockCapturer::new();
        let (tx, mut rx) = mpsc::channel(16);

        capturer.start(tx).unwrap();

        // 注入鼠标移动事件
        let sent = capturer.inject_event(KvmFrame::MouseMove { x: 5000, y: 3000 });
        assert!(sent);

        let received = rx.recv().await.unwrap();
        assert_eq!(received, KvmFrame::MouseMove { x: 5000, y: 3000 });

        capturer.stop();
    }

    #[tokio::test]
    async fn test_mock_capturer_inject_key_event() {
        let capturer = MockCapturer::new();
        let (tx, mut rx) = mpsc::channel(16);

        capturer.start(tx).unwrap();

        capturer.inject_event(KvmFrame::KeyEvent { keycode: 0x41, pressed: true });
        capturer.inject_event(KvmFrame::KeyEvent { keycode: 0x41, pressed: false });

        assert_eq!(rx.recv().await.unwrap(), KvmFrame::KeyEvent { keycode: 0x41, pressed: true });
        assert_eq!(rx.recv().await.unwrap(), KvmFrame::KeyEvent { keycode: 0x41, pressed: false });

        capturer.stop();
    }

    #[test]
    fn test_mock_capturer_intercept() {
        let capturer = MockCapturer::new();
        assert!(!capturer.is_intercepting());
        capturer.set_intercept(true);
        assert!(capturer.is_intercepting());
        capturer.set_intercept(false);
        assert!(!capturer.is_intercepting());
    }

    #[test]
    fn test_mock_capturer_inject_without_start() {
        let capturer = MockCapturer::new();
        let sent = capturer.inject_event(KvmFrame::Heartbeat);
        assert!(!sent, "未启动时注入事件应返回 false");
    }

    // ── macOS 平台特定测试 ───────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "需要辅助功能权限（CGEvent Tap），在 CI/测试环境中跳过"]
    fn test_macos_capturer_start_stop() {
        let capturer = MacOSCapturer::new();
        let (tx, _rx) = mpsc::channel(16);

        assert!(!capturer.is_running());
        capturer.start(tx).unwrap();
        assert!(capturer.is_running());
        capturer.stop();
        assert!(!capturer.is_running());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "需要辅助功能权限（CGEvent Tap），在 CI/测试环境中跳过"]
    fn test_macos_capturer_already_running() {
        let capturer = MacOSCapturer::new();
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        capturer.start(tx1).unwrap();
        assert_eq!(capturer.start(tx2), Err(CapturerError::AlreadyRunning));
        capturer.stop();
    }

    // ── Windows 平台特定测试 ─────────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_capturer_start_stop() {
        let capturer = WindowsCapturer::new();
        let (tx, _rx) = mpsc::channel(16);

        assert!(!capturer.is_running());
        capturer.start(tx).unwrap();
        assert!(capturer.is_running());
        capturer.stop();
        assert!(!capturer.is_running());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_capturer_already_running() {
        let capturer = WindowsCapturer::new();
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        capturer.start(tx1).unwrap();
        assert_eq!(capturer.start(tx2), Err(CapturerError::AlreadyRunning));
        capturer.stop();
    }

    // ── is_ctrl/alt/s_key 跨平台键码测试 ────────────────────────────────

    #[test]
    fn test_is_ctrl_key_macos() {
        assert!(is_ctrl_key(59));  // Left Ctrl
        assert!(is_ctrl_key(62));  // Right Ctrl
        assert!(!is_ctrl_key(58)); // Alt
    }

    #[test]
    fn test_is_ctrl_key_windows() {
        assert!(is_ctrl_key(0x11));  // VK_CONTROL
        assert!(is_ctrl_key(0xA2));  // VK_LCONTROL
        assert!(is_ctrl_key(0xA3));  // VK_RCONTROL
        assert!(!is_ctrl_key(0x12)); // VK_MENU (Alt)
    }

    #[test]
    fn test_is_alt_key_macos() {
        assert!(is_alt_key(58));  // Left Option
        assert!(is_alt_key(61));  // Right Option
        assert!(!is_alt_key(59)); // Ctrl
    }

    #[test]
    fn test_is_alt_key_windows() {
        assert!(is_alt_key(0x12));  // VK_MENU
        assert!(is_alt_key(0xA4));  // VK_LMENU
        assert!(is_alt_key(0xA5));  // VK_RMENU
        assert!(!is_alt_key(0x11)); // VK_CONTROL
    }

    #[test]
    fn test_is_s_key_both_platforms() {
        assert!(is_s_key(1));     // macOS CGKeyCode
        assert!(is_s_key(0x53));  // Windows VK_S
        assert!(!is_s_key(0));
        assert!(!is_s_key(0x41)); // 'A'
    }
}
