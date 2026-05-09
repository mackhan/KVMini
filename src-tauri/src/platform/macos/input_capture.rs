//! macOS 键鼠事件采集实现（CGEventTap）
//!
//! 关键设计（按 macos-cgevent-tap-hotkey skill）：
//! - CGEventTap 必须在专用子线程的 RunLoop 中运行（不能用主线程，
//!   主线程被 Tauri/NSApplication 占用，事件回调不会触发）。
//! - CFRunLoopAddSource(CFRunLoopGetCurrent(), ...) + CFRunLoopRun() 在子线程内调用。
//! - 通过保存子线程的 CFRunLoopRef，stop_capture 时调 CFRunLoop::stop 让子线程退出。
//!
//! v0.1.38 BUG-022 修复：
//! - 采用 CGEventTapOptions::Default（不是 ListenOnly），让回调返回值生效：
//!     - 返回 Some(event) → 事件继续传递到本机
//!     - 返回 None        → 事件被吞掉，本机不感知
//! - 引入 Arc<AtomicU8> 控制状态（0=Idle, 1=LocalControlling, 2=RemoteControlling）
//! - 回调用 Relaxed 原子读（禁止锁，避免实时输入路径阻塞）
//! - SharingManager 在状态切换时调用 set_control_state 同步

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputCapture, RawInputEvent};
use kvm_core::preempt::{PhysicalInputSample, PreemptCause, PreemptDecision, PreemptDetector};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};

#[cfg(target_os = "macos")]
use core_foundation::runloop::CFRunLoop;

/// 控制状态原子值
/// 0 = Idle（放行所有输入）
/// 1 = LocalControlling（本机控制对端，吞掉本机输入）
/// 2 = RemoteControlling（被对端控制，吞掉本机输入防止干扰）
pub const CONTROL_STATE_IDLE: u8 = 0;
pub const CONTROL_STATE_LOCAL_CONTROLLING: u8 = 1;
pub const CONTROL_STATE_REMOTE_CONTROLLING: u8 = 2;

/// v0.1.43：注入事件的 magic user data，用于 CGEventTap 回调过滤自我产生的合成事件。
///
/// `input_inject.rs` 在 `event.post()` 前调 `set_integer_value_field(kCGEventSourceUserData=42, ...)`
/// 写入此值；tap 回调读到相同值即认为是自注入事件，跳过抢占检测。
/// 值取 b"KVMI" 的 i64，碰撞概率极低。
pub const KVMINI_INJECT_USER_DATA: i64 = 0x4B56_4D49; // "KVMI"

/// macOS Core Graphics 事件字段 `kCGEventSourceUserData` 的 raw 值
#[cfg(target_os = "macos")]
pub const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;

/// 抢占回调类型别名
pub type PreemptCallback = Box<dyn Fn(PreemptCause) + Send + Sync>;

pub struct MacInputCapture {
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
    /// 控制状态，CGEventTap 回调按此决定是否吞事件。
    /// 初始为 Idle，绝不在未明确切到 Local/Remote 时吞事件（防止误吞用户输入）。
    control_state: Arc<AtomicU8>,
    /// v0.1.43 抢占检测器（仅在 RemoteControlling 态下喂样本）
    preempt_detector: Arc<PreemptDetector>,
    /// v0.1.43 抢占触发回调（sharing 层设置），触发后 detector.reset()
    preempt_callback: Arc<Mutex<Option<PreemptCallback>>>,
    /// 持有捕获子线程的 RunLoop 引用，用于 stop 时唤醒
    #[cfg(target_os = "macos")]
    run_loop: Arc<Mutex<Option<RunLoopHandle>>>,
}

/// 跨线程持有的 CFRunLoop 句柄（CFRunLoop 本身就是引用计数对象）
#[cfg(target_os = "macos")]
struct RunLoopHandle(CFRunLoop);

#[cfg(target_os = "macos")]
unsafe impl Send for RunLoopHandle {}
#[cfg(target_os = "macos")]
unsafe impl Sync for RunLoopHandle {}

impl MacInputCapture {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
            callback: Arc::new(Mutex::new(None)),
            control_state: Arc::new(AtomicU8::new(CONTROL_STATE_IDLE)),
            preempt_detector: Arc::new(PreemptDetector::new()),
            preempt_callback: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "macos")]
            run_loop: Arc::new(Mutex::new(None)),
        }
    }

    /// 由 SharingManager 在状态切换时调用，推送新状态到 CGEventTap 回调。
    /// 使用 Relaxed 足够：回调线程下一次读能看到新值即可，无需内存序强约束。
    pub fn set_control_state(&self, new_state: u8) {
        let old = self.control_state.swap(new_state, Ordering::Relaxed);
        if old != new_state {
            log::info!(
                "[Mac] control_state: {} → {} ({})",
                old,
                new_state,
                describe_state(new_state)
            );
        }
        // v0.1.43 抢占检测器：只在 RemoteControlling 态下工作，切态时 reset 避免残留累计
        if new_state == CONTROL_STATE_REMOTE_CONTROLLING {
            self.preempt_detector.reset();
            log::debug!("[Mac] preempt_detector.reset()（进入 RemoteControlling）");
        }
    }

    /// 获取当前控制状态（主要用于测试与诊断）
    pub fn control_state(&self) -> u8 {
        self.control_state.load(Ordering::Relaxed)
    }

    /// 暴露控制状态原子给 SharingManager 做共享（可选用法：
    /// 直接让 SharingManager 持同一个 Arc，避免反复 setter 调用）
    pub fn control_state_arc(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.control_state)
    }

    /// v0.1.43 设置抢占触发回调（sharing 层调用）
    ///
    /// 回调会在被控端检测到物理鼠标累计位移 ≥40px 或任意 KeyDown 时触发，
    /// 上层负责发 SwitchBackControl + 切 Idle。触发后 detector 已自动 reset。
    pub fn set_preempt_callback(&self, cb: PreemptCallback) {
        *self.preempt_callback.lock().unwrap() = Some(cb);
    }

    /// v0.1.43 清除抢占回调（disconnect / stop 时调用）
    pub fn clear_preempt_callback(&self) {
        *self.preempt_callback.lock().unwrap() = None;
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

impl Default for MacInputCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl InputCapture for MacInputCapture {
    fn start_capture(
        &self,
        callback: Box<dyn Fn(RawInputEvent) + Send + Sync>,
    ) -> KvmResult<()> {
        // 已经在采集时直接返回（避免重复 spawn 子线程）
        if self.capturing.load(Ordering::SeqCst) {
            log::warn!("[Mac] start_capture 重复调用，忽略");
            return Ok(());
        }

        *self.callback.lock().unwrap() = Some(callback);
        self.capturing.store(true, Ordering::SeqCst);
        log::info!("[Mac] 键鼠采集启动（子线程模式）");

        #[cfg(target_os = "macos")]
        {
            let capturing = Arc::clone(&self.capturing);
            let cb = Arc::clone(&self.callback);
            let run_loop_slot = Arc::clone(&self.run_loop);
            let control_state = Arc::clone(&self.control_state);
            let preempt_detector = Arc::clone(&self.preempt_detector);
            let preempt_callback = Arc::clone(&self.preempt_callback);
            let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<KvmResult<()>>(1);

            // 专用子线程：创建 Tap → 注册到本线程 RunLoop → CFRunLoopRun() 阻塞
            std::thread::Builder::new()
                .name("kvm-cgeventtap".to_string())
                .spawn(move || {
                    match start_event_tap_in_current_thread(
                        capturing,
                        cb,
                        control_state,
                        preempt_detector,
                        preempt_callback,
                    ) {
                        Ok(_) => {
                            // 把当前线程的 RunLoop 引用回传给主控制端
                            let rl = CFRunLoop::get_current();
                            *run_loop_slot.lock().unwrap() = Some(RunLoopHandle(rl));
                            let _ = started_tx.send(Ok(()));
                            log::info!("[Mac] CGEventTap 子线程进入 CFRunLoopRun()");
                            CFRunLoop::run_current();
                            log::info!("[Mac] CGEventTap 子线程已退出 CFRunLoopRun()");
                        }
                        Err(e) => {
                            log::error!("[Mac] CGEventTap 启动失败: {}", e);
                            let _ = started_tx.send(Err(e));
                        }
                    }
                })
                .map_err(|e| kvm_core::error::KvmError::PlatformError {
                    api: "spawn cgeventtap thread".to_string(),
                    reason: e.to_string(),
                })?;

            // 等待子线程完成 Tap 创建（最多 2 秒）
            match started_rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => {
                    self.capturing.store(false, Ordering::SeqCst);
                    *self.callback.lock().unwrap() = None;
                    Err(e)
                }
                Err(_) => {
                    self.capturing.store(false, Ordering::SeqCst);
                    *self.callback.lock().unwrap() = None;
                    Err(kvm_core::error::KvmError::PlatformError {
                        api: "CGEventTap startup".to_string(),
                        reason: "等待 CGEventTap 子线程启动超时（2s）".to_string(),
                    })
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        Ok(())
    }

    fn stop_capture(&self) -> KvmResult<()> {
        if !self.capturing.swap(false, Ordering::SeqCst) {
            return Ok(());
        }
        *self.callback.lock().unwrap() = None;
        // 停止采集时强制回 Idle，防止后续还在吞事件
        self.control_state
            .store(CONTROL_STATE_IDLE, Ordering::Relaxed);
        log::info!("[Mac] 键鼠采集停止，control_state → Idle");

        #[cfg(target_os = "macos")]
        {
            // 唤醒子线程的 CFRunLoopRun，让其退出
            if let Some(handle) = self.run_loop.lock().unwrap().take() {
                CFRunLoop::stop(&handle.0);
                log::info!("[Mac] 已请求 CGEventTap 子线程退出");
            }
        }
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.capturing.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;

// FFI: CGEventTapEnable 用于 re-enable 被系统自动 disable 的 tap
#[cfg(target_os = "macos")]
extern "C" {
    fn CGEventTapEnable(tap: core_foundation::mach_port::CFMachPortRef, enable: bool);
}

fn start_event_tap_in_current_thread(
    capturing: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(RawInputEvent) + Send + Sync>>>>,
    control_state: Arc<AtomicU8>,
    preempt_detector: Arc<PreemptDetector>,
    preempt_callback: Arc<Mutex<Option<PreemptCallback>>>,
) -> KvmResult<()> {
    // 克隆 capturing 给守护线程使用（退出条件）
    let capturing_for_guard = Arc::clone(&capturing);
    use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };

    let mut mask = vec![
        CGEventType::MouseMoved,
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseUp,
        CGEventType::RightMouseDown,
        CGEventType::RightMouseUp,
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
        CGEventType::ScrollWheel,
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::FlagsChanged,
        // v0.1.41: 手势屏蔽 — 避免 LocalControlling 时触发 Mission Control/Launchpad/Pinch
        CGEventType::LeftMouseDragged,
        CGEventType::RightMouseDragged,
        CGEventType::OtherMouseDragged,
    ];

    // v0.1.41: 额外的手势事件类型（core-graphics CGEventType 枚举不含这些）
    // 通过 transmute 注入 raw event type 值到 mask 列表
    // kCGEventGesture = 29, NSEventTypeMagnify/Pinch = 30, NSEventTypeSwipe = 31
    unsafe {
        mask.push(std::mem::transmute::<u32, CGEventType>(29)); // kCGEventGesture
        mask.push(std::mem::transmute::<u32, CGEventType>(30)); // Magnify/Pinch
        mask.push(std::mem::transmute::<u32, CGEventType>(31)); // Swipe
    }

    // CGEventTapOptions::Default 允许回调决定事件放行/吞掉；
    // 必须在本机授予"输入监控"权限，否则 CGEventTap::new 会失败。
    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        mask,
        move |_proxy, event_type, event| {
            // BUG-030: 检测 tap 被系统超时禁用，立即 re-enable
            // macOS 会在回调返回 None 过于频繁/耗时过长时自动 disable tap
            // CGEventType 没有实现 PartialEq，用 raw 值比较
            // TapDisabledByTimeout = 0xFFFFFFFE, TapDisabledByUserInput = 0xFFFFFFFF
            let event_type_raw = unsafe { std::mem::transmute::<_, u32>(event_type) };
            if event_type_raw == 0xFFFFFFFE || event_type_raw == 0xFFFFFFFF {
                log::warn!("[Mac] CGEventTap 被系统禁用（type={:#X}），守护线程将自动 re-enable", event_type_raw);
                return Some(event.to_owned());
            }

            // 1. 未在 capturing 状态时，一律放行（安全兜底）
            if !capturing.load(Ordering::SeqCst) {
                return Some(event.to_owned());
            }

            // 读取控制状态（提前到这里，用于日志和决策）
            let state = control_state.load(Ordering::Relaxed);

            // ── v0.1.43 抢占检测 ─────────────────────────────────────
            // 仅在 RemoteControlling 态、且事件非自注入时喂 PreemptDetector
            if state == CONTROL_STATE_REMOTE_CONTROLLING {
                // 过滤自注入事件：input_inject 写入的 KVMINI_INJECT_USER_DATA magic
                let user_data = event.get_integer_value_field(K_CG_EVENT_SOURCE_USER_DATA);
                let is_synthetic = user_data == KVMINI_INJECT_USER_DATA;

                if !is_synthetic {
                    let sample = match event_type {
                        CGEventType::MouseMoved
                        | CGEventType::LeftMouseDragged
                        | CGEventType::RightMouseDragged
                        | CGEventType::OtherMouseDragged => {
                            use core_graphics::event::EventField;
                            let dx = event
                                .get_integer_value_field(EventField::MOUSE_EVENT_DELTA_X)
                                as i32;
                            let dy = event
                                .get_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y)
                                as i32;
                            if dx != 0 || dy != 0 {
                                Some(PhysicalInputSample::Move { dx, dy })
                            } else {
                                None
                            }
                        }
                        CGEventType::KeyDown => Some(PhysicalInputSample::KeyDown),
                        _ => None,
                    };

                    if let Some(s) = sample {
                        if let PreemptDecision::ShouldPreempt { cause } =
                            preempt_detector.feed(s)
                        {
                            log::info!(
                                "[Mac][Preempt] physical input preempt triggered (cause={:?})",
                                cause
                            );
                            // 触发回调（在 tap 回调线程；sharing 层应尽快 spawn async 异步发帧）
                            if let Ok(guard) = preempt_callback.lock() {
                                if let Some(cb) = guard.as_ref() {
                                    cb(cause);
                                }
                            }
                            // detector 在内部已自动置位 triggered，本次调用后不会重复触发
                        }
                    }
                }
            }

            // 2. 先把事件送给 sender（无论最后是否吞）——
            //    只有 LocalControlling 状态下这些事件才有意义转发给对端，
            //    其他状态日志里 sender 会被静默丢弃（event_tx 已被 take）。
            if let Some(raw_event) = cg_event_to_raw(event_type, event) {
                if let Ok(guard) = callback.lock() {
                    if let Some(cb) = guard.as_ref() {
                        cb(raw_event);
                    }
                }
            }

            // 3. 根据控制状态决定是否吞事件
            match state {
                CONTROL_STATE_LOCAL_CONTROLLING | CONTROL_STATE_REMOTE_CONTROLLING => {
                    // 吞掉：本机不感知此事件
                    // 用 static 变量限频日志，只打一次
                    use std::sync::atomic::AtomicBool;
                    static LOGGED_FIRST_SWALLOW: AtomicBool = AtomicBool::new(false);
                    if !LOGGED_FIRST_SWALLOW.swap(true, Ordering::Relaxed) {
                        log::info!("[Mac] 首次吞掉本机输入事件（state={}）", state);
                    }
                    None
                }
                _ => {
                    // Idle/Armed：完全放行（防误吞，是最关键的安全兜底）
                    Some(event.to_owned())
                }
            }
        },
    )
    .map_err(|_| kvm_core::error::KvmError::PlatformError {
        api: "CGEventTap::new".to_string(),
        reason: "创建 CGEventTap 失败，请检查输入监控/辅助功能权限".to_string(),
    })?;

    let loop_source = tap
        .mach_port
        .create_runloop_source(0)
        .map_err(|_| kvm_core::error::KvmError::PlatformError {
            api: "create_runloop_source".to_string(),
            reason: "创建 RunLoop Source 失败".to_string(),
        })?;

    // 保存 mach_port 的 raw 指针用于 re-enable（在 tap 被 forget 后仍可用）
    // 保存 mach_port 的 raw 指针（as usize）用于守护线程 re-enable
    let mach_port_raw = tap.mach_port.as_concrete_TypeRef() as usize;

    // CRITICAL：必须用 GetCurrent（即子线程 RunLoop），不能用 GetMain
    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&loop_source, unsafe { kCFRunLoopDefaultMode });
    tap.enable();

    // tap 必须 forget，否则函数返回后 tap 析构 → CFMachPort 释放 → Tap 失效（根因 2）
    // CFRunLoop 已经持有 source → mach_port，把 tap 自身泄漏即可保持强引用
    std::mem::forget(tap);

    // BUG-030: 启动一个定时器，每 5 秒检测并 re-enable tap（防止被系统 disable 后无法恢复）
    // 这是因为 TapDisabledByTimeout 事件有时不会被回调收到（tap 已经 disabled 了）
    // v0.1.41 fix: 守护线程在 capturing=false 时退出，避免 stop_capture 后线程泄漏和潜在 UAF
    std::thread::Builder::new()
        .name("kvm-tap-guard".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                // 退出条件：capture 已停止
                if !capturing_for_guard.load(Ordering::SeqCst) {
                    log::info!("[Mac] tap 守护线程检测到 capturing=false，正常退出");
                    break;
                }
                unsafe {
                    let port_ref = mach_port_raw as core_foundation::mach_port::CFMachPortRef;
                    CGEventTapEnable(port_ref, true);
                }
            }
        })
        .ok();

    log::info!("[Mac] CGEventTap 已启动（子线程 RunLoop, CGEventTapOptions::Default, 含自动 re-enable 守护）");
    Ok(())
}

#[cfg(target_os = "macos")]
fn cg_event_to_raw(
    event_type: core_graphics::event::CGEventType,
    event: &core_graphics::event::CGEvent,
) -> Option<RawInputEvent> {
    use core_graphics::event::CGEventType;
    use kvm_core::platform::MouseButton;

    match event_type {
        CGEventType::MouseMoved => {
            let loc = event.location();
            // v0.1.40: 读取硬件 delta（CGEventField 93/94 = kCGMouseEventDeltaX/Y）
            // 绕过 grab 后 event.location() 冻结的问题（根本修复远端鼠标不动 bug）
            use core_graphics::event::EventField;
            let dx = event.get_integer_value_field(EventField::MOUSE_EVENT_DELTA_X) as i16;
            let dy = event.get_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y) as i16;
            Some(RawInputEvent::MouseMove {
                x: loc.x as i32,
                y: loc.y as i32,
                dx,
                dy,
            })
        }
        CGEventType::LeftMouseDown => Some(RawInputEvent::MouseDown {
            button: MouseButton::Left,
        }),
        CGEventType::LeftMouseUp => Some(RawInputEvent::MouseUp {
            button: MouseButton::Left,
        }),
        CGEventType::RightMouseDown => Some(RawInputEvent::MouseDown {
            button: MouseButton::Right,
        }),
        CGEventType::RightMouseUp => Some(RawInputEvent::MouseUp {
            button: MouseButton::Right,
        }),
        CGEventType::OtherMouseDown => Some(RawInputEvent::MouseDown {
            button: MouseButton::Middle,
        }),
        CGEventType::OtherMouseUp => Some(RawInputEvent::MouseUp {
            button: MouseButton::Middle,
        }),
        CGEventType::ScrollWheel => {
            // ScrollWheelEventPointDeltaAxis2 = 97, ScrollWheelEventPointDeltaAxis1 = 96
            let dx = event.get_integer_value_field(97) as i32;
            let dy = event.get_integer_value_field(96) as i32;
            Some(RawInputEvent::Scroll { dx, dy })
        }
        CGEventType::KeyDown => {
            let keycode = event.get_integer_value_field(9) as u16;
            Some(RawInputEvent::KeyDown {
                native_keycode: keycode,
            })
        }
        CGEventType::KeyUp => {
            let keycode = event.get_integer_value_field(9) as u16;
            Some(RawInputEvent::KeyUp {
                native_keycode: keycode,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_state() {
        let cap = MacInputCapture::new();
        assert!(!cap.is_capturing());
    }

    #[test]
    fn test_stop_when_not_capturing_is_noop() {
        let cap = MacInputCapture::new();
        // 未开始时 stop 应该幂等且不报错
        assert!(cap.stop_capture().is_ok());
    }

    #[test]
    fn test_default_control_state_is_idle() {
        // 关键安全兜底：默认状态必须是 Idle，绝不能初始就吞事件
        let cap = MacInputCapture::new();
        assert_eq!(cap.control_state(), CONTROL_STATE_IDLE);
    }

    #[test]
    fn test_set_control_state_roundtrip() {
        let cap = MacInputCapture::new();
        cap.set_control_state(CONTROL_STATE_LOCAL_CONTROLLING);
        assert_eq!(cap.control_state(), CONTROL_STATE_LOCAL_CONTROLLING);
        cap.set_control_state(CONTROL_STATE_REMOTE_CONTROLLING);
        assert_eq!(cap.control_state(), CONTROL_STATE_REMOTE_CONTROLLING);
        cap.set_control_state(CONTROL_STATE_IDLE);
        assert_eq!(cap.control_state(), CONTROL_STATE_IDLE);
    }

    #[test]
    fn test_control_state_arc_shared() {
        // SharingManager 可以持有同一份 Arc，直接更新能被 capture 回调看到
        let cap = MacInputCapture::new();
        let shared = cap.control_state_arc();
        shared.store(CONTROL_STATE_LOCAL_CONTROLLING, Ordering::Relaxed);
        assert_eq!(cap.control_state(), CONTROL_STATE_LOCAL_CONTROLLING);
    }

    #[test]
    fn test_stop_capture_resets_state_to_idle() {
        // 安全兜底：stop_capture 必须强制回 Idle，防止后续误吞
        let cap = MacInputCapture::new();
        cap.set_control_state(CONTROL_STATE_LOCAL_CONTROLLING);
        // 模拟：虽然未真实 start_capture，但 stop_capture 路径仍应重置
        // 注意：is_capturing=false 时 stop 是 noop，这里显式校验 reset 逻辑
        cap.capturing.store(true, Ordering::SeqCst);
        assert!(cap.stop_capture().is_ok());
        assert_eq!(cap.control_state(), CONTROL_STATE_IDLE);
    }

    /// 真值表测试：验证"状态 → 回调返回值"的语义与现实预期一致
    /// （这是纯逻辑决策，不依赖 CGEventTap；如果这个逻辑被破坏会导致
    /// 用户要么完全无法操作电脑，要么 KVM 无法生效）
    #[test]
    fn test_state_to_decision_truth_table() {
        fn should_swallow(state: u8) -> bool {
            matches!(
                state,
                CONTROL_STATE_LOCAL_CONTROLLING | CONTROL_STATE_REMOTE_CONTROLLING
            )
        }
        // Idle → 放行（返回 Some）
        assert!(!should_swallow(CONTROL_STATE_IDLE), "Idle 必须放行");
        // Local → 吞
        assert!(
            should_swallow(CONTROL_STATE_LOCAL_CONTROLLING),
            "LocalControlling 必须吞"
        );
        // Remote → 吞
        assert!(
            should_swallow(CONTROL_STATE_REMOTE_CONTROLLING),
            "RemoteControlling 必须吞"
        );
        // 未知状态 → 放行（安全兜底）
        assert!(!should_swallow(255), "未知状态必须放行（安全兜底）");
    }
}
