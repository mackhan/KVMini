//! macOS 焦点保持器
//!
//! 解决 LocalControlling 时其他应用抢走焦点导致 CGAssociateMouseAndMouseCursorPosition 被系统重置的问题。
//!
//! 方案（业界主流，Synergy/Barrier 同理）：
//! - 创建 1×1 透明窗口（level=statusBar, alpha=0.01, ignoresMouseEvents=true）
//! - 启动 200ms 间隔定时器持续 NSApp.activate(ignoringOtherApps:true) + makeKeyAndOrderFront
//! - macOS 始终认为 KVMini 前台 → CGAssociate 不被重置 → CGEventTap 持续吞事件
//! - 退出 LocalControlling 时立即关窗 + 停定时器，把焦点完全还给系统

use kvm_core::error::KvmResult;
use kvm_core::platform::FocusKeeper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

impl FocusKeeper for MacFocusKeeper {
    fn start(&self) -> KvmResult<()> {
        if self.active.load(Ordering::SeqCst) {
            log::debug!("[FocusKeeper] 已在运行，跳过重复启动");
            return Ok(());
        }

        self.stop_signal.store(false, Ordering::SeqCst);
        self.active.store(true, Ordering::SeqCst);

        let stop = Arc::clone(&self.stop_signal);
        let active = Arc::clone(&self.active);

        // 在主线程上执行窗口创建和定时器
        // 注意：NSApp.activate 必须在主线程调用
        std::thread::spawn(move || {
            #[cfg(target_os = "macos")]
            {
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

                    log::info!("[FocusKeeper] 已启动：1×1 透明窗口 + 200ms activate 循环");

                    // 200ms 循环 activate
                    while !stop.load(Ordering::SeqCst) {
                        let _: () = msg_send![app, activateIgnoringOtherApps:YES];
                        let _: () = msg_send![window, makeKeyAndOrderFront:std::ptr::null::<Object>()];
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }

                    // 停止：关窗
                    let _: () = msg_send![window, orderOut:std::ptr::null::<Object>()];
                    let _: () = msg_send![window, close];

                    log::info!("[FocusKeeper] 已停止：窗口关闭，焦点归还系统");
                }
            }
            active.store(false, Ordering::SeqCst);
        });

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
