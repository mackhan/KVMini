//! macOS 焦点保持器
//!
//! 解决 LocalControlling 时其他应用抢走焦点导致 CGAssociateMouseAndMouseCursorPosition 被系统重置的问题。
//!
//! 方案（业界主流，Synergy/Barrier 同理）：
//! - 创建 1×1 透明窗口（level=statusBar, alpha=0.01, ignoresMouseEvents=true）
//! - 启动 200ms 间隔定时器持续 NSApp.activate(ignoringOtherApps:true) + makeKeyAndOrderFront
//! - macOS 始终认为 KVMini 前台 → CGAssociate 不被重置 → CGEventTap 持续吞事件
//! - 退出 LocalControlling 时立即关窗 + 停定时器，把焦点完全还给系统
//!
//! v0.1.41 fix: 所有 AppKit 操作通过 dispatch_async(dispatch_get_main_queue()) 派发到主线程，
//! 避免在子线程调用 NSApplication/NSWindow 导致未定义行为。

use kvm_core::error::KvmResult;
use kvm_core::platform::FocusKeeper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "macos")]
extern "C" {
    // dispatch_get_main_queue() 在 libdispatch 中实际是全局变量 _dispatch_main_q
    #[link_name = "_dispatch_main_q"]
    static _dispatch_main_q: *mut std::ffi::c_void;

    fn dispatch_async_f(
        queue: *mut std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: extern "C" fn(*mut std::ffi::c_void),
    );
}

/// 获取主队列指针
#[cfg(target_os = "macos")]
#[inline]
unsafe fn get_main_queue() -> *mut std::ffi::c_void {
    // SAFETY: _dispatch_main_q 是 libdispatch 导出的全局指针
    &_dispatch_main_q as *const _ as *mut std::ffi::c_void
}

pub struct MacFocusKeeper {
    active: Arc<AtomicBool>,
    /// 用于通知定时器线程停止
    stop_signal: Arc<AtomicBool>,
}

impl MacFocusKeeper {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// dispatch_async_f 的上下文：启动窗口
#[cfg(target_os = "macos")]
struct FocusKeeperStartCtx {
    stop_signal: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

/// dispatch_async_f 回调：在主线程创建窗口并启动 timer 循环（子线程轮询 stop 信号）
#[cfg(target_os = "macos")]
extern "C" fn focus_keeper_start_on_main(ctx_ptr: *mut std::ffi::c_void) {
    let ctx = unsafe { Box::from_raw(ctx_ptr as *mut FocusKeeperStartCtx) };

    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::{Object, YES, NO};
    use core_graphics::geometry::{CGRect, CGPoint, CGSize};

    unsafe {
        // 获取 NSApp
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];

        // 创建 1×1 透明窗口
        let frame = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 1.0, height: 1.0 },
        };

        // NSWindow with borderless style (NSWindowStyleMaskBorderless = 0)
        let window: *mut Object = msg_send![class!(NSWindow), alloc];
        let window: *mut Object = msg_send![window,
            initWithContentRect:frame
            styleMask:0u64
            backing:2u64  // NSBackingStoreBuffered
            defer:NO
        ];

        // 设置窗口属性
        let _: () = msg_send![window, setAlphaValue:0.01f64];
        let _: () = msg_send![window, setLevel:25i64]; // NSStatusWindowLevel
        let _: () = msg_send![window, setIgnoresMouseEvents:YES];
        let _: () = msg_send![window, setHasShadow:NO];
        let _: () = msg_send![window, setOpaque:NO];
        let _: () = msg_send![window, setCollectionBehavior:1u64 << 0]; // canJoinAllSpaces

        // 显示窗口
        let _: () = msg_send![window, makeKeyAndOrderFront:std::ptr::null::<Object>()];
        let _: () = msg_send![app, activateIgnoringOtherApps:YES];

        log::info!("[FocusKeeper] 已启动：1×1 透明窗口（主线程创建）");

        // 200ms 循环：子线程轮询 stop_signal，主线程派发 activate
        let stop = ctx.stop_signal;
        let active = ctx.active;
        let window_ptr = window as usize; // 跨线程保存指针（安全：仅主线程使用）
        let app_ptr = app as usize;

        std::thread::Builder::new()
            .name("kvm-focus-keeper".to_string())
            .spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(200));

                    // 通过 dispatch_async 派发 activate 到主线程
                    let activate_ctx = Box::new(FocusKeeperActivateCtx {
                        window_ptr,
                        app_ptr,
                    });
                    dispatch_async_f(
                        get_main_queue(),
                        Box::into_raw(activate_ctx) as *mut std::ffi::c_void,
                        focus_keeper_activate_on_main,
                    );
                }

                // 停止：在主线程关窗
                let close_ctx = Box::new(FocusKeeperCloseCtx { window_ptr });
                dispatch_async_f(
                    get_main_queue(),
                    Box::into_raw(close_ctx) as *mut std::ffi::c_void,
                    focus_keeper_close_on_main,
                );

                active.store(false, Ordering::SeqCst);
                log::info!("[FocusKeeper] 已停止：窗口将在主线程关闭");
            })
            .ok();
    }
}

/// 主线程激活回调上下文
#[cfg(target_os = "macos")]
struct FocusKeeperActivateCtx {
    window_ptr: usize,
    app_ptr: usize,
}

/// 主线程关闭窗口回调上下文
#[cfg(target_os = "macos")]
struct FocusKeeperCloseCtx {
    window_ptr: usize,
}

/// dispatch_async_f 回调：主线程执行 activate
#[cfg(target_os = "macos")]
extern "C" fn focus_keeper_activate_on_main(ctx_ptr: *mut std::ffi::c_void) {
    let ctx = unsafe { Box::from_raw(ctx_ptr as *mut FocusKeeperActivateCtx) };
    use objc::{msg_send, sel, sel_impl};
    use objc::runtime::{Object, YES};

    unsafe {
        let app = ctx.app_ptr as *mut Object;
        let window = ctx.window_ptr as *mut Object;
        let _: () = msg_send![app, activateIgnoringOtherApps:YES];
        let _: () = msg_send![window, makeKeyAndOrderFront:std::ptr::null::<Object>()];
    }
}

/// dispatch_async_f 回调：主线程关闭窗口
#[cfg(target_os = "macos")]
extern "C" fn focus_keeper_close_on_main(ctx_ptr: *mut std::ffi::c_void) {
    let ctx = unsafe { Box::from_raw(ctx_ptr as *mut FocusKeeperCloseCtx) };
    use objc::{msg_send, sel, sel_impl};
    use objc::runtime::Object;

    unsafe {
        let window = ctx.window_ptr as *mut Object;
        let _: () = msg_send![window, orderOut:std::ptr::null::<Object>()];
        let _: () = msg_send![window, close];
    }
    log::info!("[FocusKeeper] 窗口已在主线程关闭，焦点归还系统");
}

impl FocusKeeper for MacFocusKeeper {
    fn start(&self) -> KvmResult<()> {
        if self.active.load(Ordering::SeqCst) {
            log::debug!("[FocusKeeper] 已在运行，跳过重复启动");
            return Ok(());
        }

        self.stop_signal.store(false, Ordering::SeqCst);
        self.active.store(true, Ordering::SeqCst);

        #[cfg(target_os = "macos")]
        {
            let ctx = Box::new(FocusKeeperStartCtx {
                stop_signal: Arc::clone(&self.stop_signal),
                active: Arc::clone(&self.active),
            });
            unsafe {
                dispatch_async_f(
                    get_main_queue(),
                    Box::into_raw(ctx) as *mut std::ffi::c_void,
                    focus_keeper_start_on_main,
                );
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // 非 macOS 平台无操作
            self.active.store(false, Ordering::SeqCst);
        }

        Ok(())
    }

    fn stop(&self) -> KvmResult<()> {
        if !self.active.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.stop_signal.store(true, Ordering::SeqCst);
        log::info!("[FocusKeeper] 发出停止信号");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_keeper_initial_state() {
        let fk = MacFocusKeeper::new();
        assert!(!fk.is_active());
    }

    #[test]
    fn test_focus_keeper_stop_when_not_active() {
        let fk = MacFocusKeeper::new();
        // stop 在未激活时不应 panic
        assert!(fk.stop().is_ok());
    }
}
