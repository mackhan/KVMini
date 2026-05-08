//! macOS 焦点保持器（v0.1.42 重写版）
//!
//! ## v0.1.41 → v0.1.42 重大变更
//!
//! v0.1.41 实现的"创建 1×1 透明窗口 + 200ms NSApp.activate(ignoringOtherApps:YES) +
//! makeKeyAndOrderFront"会把 KVMini 强制抢成 key application，副作用极其严重：
//!
//! 1. KVMini 成为 key app → Cocoa first responder 链开始吃键盘事件 → 远端键盘失效
//! 2. 点击落到 KVMini 主窗口而非被 swallow → 本机闪烁
//! 3. activate 间隙 mouseDown 被 CGEventTap 吞，但 mouseUp 落到 KVMini 主窗口
//!    → 远端"按住没松"=误触发拖拽
//! 4. 事件路由在"前台 app"和"HID tap"之间反复切换 → 点击丢失
//!
//! ## v0.1.42 正确做法（业界主流，Synergy/Barrier/Logitech Flow 实测路径）
//!
//! - CGEventTap 工作在 `kCGHIDEventTap`（HID 层），其吞噬能力**本来就不依赖 key window**
//! - 唯一需要"维持"的是 `CGAssociateMouseAndMouseCursorPosition(0)` —— 系统在
//!   显示模式变化、其它 app 调相关 API 等时机会重置它
//! - 解法：后台线程 200ms 周期重新调一次 `Associate(0)`，**不创建任何窗口、不调 activate**
//!
//! ## 接口契约（保持不变）
//!
//! - `start()` / `stop()` / `is_active()` 签名零改动
//! - sharing.rs 调用方式无须修改
//!
//! ## 状态机安全（v0.1.42 code review 修复）
//!
//! - `start()` 使用 `compare_exchange` 避免并发 double-spawn
//! - `stop()` 立即把 `active` 置 false，避免"快速 toggle"窗口期内新 start 被幂等分支跳过
//! - JoinHandle 保存在结构体中，stop() 时 join 让线程确定性退出（最多等待 1 个 tick）

use kvm_core::error::KvmResult;
use kvm_core::platform::FocusKeeper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// 200ms keep-alive 周期。提取为常量便于测试用更短周期覆盖循环体。
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(200);

pub struct MacFocusKeeper {
    /// 对外可见的运行状态。`start()` 成功后立即为 true，`stop()` 调用后立即为 false。
    active: Arc<AtomicBool>,
    /// 通知后台线程退出。线程每 tick 前读一次。
    stop_signal: Arc<AtomicBool>,
    /// 后台线程句柄，`stop()` 时 take + join，保证线程退出确定性。
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl MacFocusKeeper {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }

    /// 通用启动逻辑：spawn 一个 daemon 线程，每 `interval` 调一次 `tick()`，
    /// 直到 `stop_signal` 为 true。
    ///
    /// 提取为关联函数让测试可注入自定义闭包和更短周期。
    fn spawn_keep_alive<F>(
        stop_signal: Arc<AtomicBool>,
        tick: F,
        interval: Duration,
    ) -> std::io::Result<JoinHandle<()>>
    where
        F: Fn() + Send + 'static,
    {
        std::thread::Builder::new()
            .name("kvm-focus-keeper".to_string())
            .spawn(move || {
                log::info!(
                    "[FocusKeeper] keep-alive 线程启动（interval={:?}）",
                    interval
                );
                while !stop_signal.load(Ordering::SeqCst) {
                    tick();
                    std::thread::sleep(interval);
                }
                log::info!("[FocusKeeper] keep-alive 线程退出");
            })
    }
}

impl Default for MacFocusKeeper {
    fn default() -> Self {
        Self::new()
    }
}

/// 生产实现使用的 tick 动作：macOS 上调 `CGAssociateMouseAndMouseCursorPosition(0)`；
/// 其它平台 no-op（保证编译通过，测试也能跑）。
#[inline]
fn real_tick() {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::CGAssociateMouseAndMouseCursorPosition;
        unsafe {
            // 重新断开物理鼠标 ↔ 屏幕光标的绑定。
            // 即使系统在某个时机把它重置成 1，下一次 tick 也会立刻拉回 0。
            // 该调用纯 syscall，无 GUI 副作用，不会抢前台焦点。
            CGAssociateMouseAndMouseCursorPosition(0);
        }
    }
}

impl FocusKeeper for MacFocusKeeper {
    fn start(&self) -> KvmResult<()> {
        // compare_exchange 避免并发 double-spawn：
        // 若旧值本来就是 true，直接幂等返回；
        // 若旧值为 false 则原子翻转到 true 并继续 spawn。
        if self
            .active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            log::debug!("[FocusKeeper] 已在运行，跳过重复启动");
            return Ok(());
        }

        // 清零 stop_signal，防止前一次 stop 留下的 true 直接让新线程退出
        self.stop_signal.store(false, Ordering::SeqCst);

        let handle = Self::spawn_keep_alive(
            Arc::clone(&self.stop_signal),
            real_tick,
            DEFAULT_TICK_INTERVAL,
        )
        .map_err(|e| kvm_core::error::KvmError::PlatformError {
            api: "spawn focus_keeper thread".to_string(),
            reason: e.to_string(),
        })?;

        // 存 handle 便于 stop 时 join
        *self.thread.lock().unwrap() = Some(handle);

        log::info!("[FocusKeeper] 已启动（CGAssociate(0) keep-alive 模式，无窗口、无前台抢占）");
        Ok(())
    }

    fn stop(&self) -> KvmResult<()> {
        // 用 swap 立即翻转 active=false，保证后续任何 is_active() / start() 的读都看到最新值
        if !self.active.swap(false, Ordering::SeqCst) {
            return Ok(());
        }
        self.stop_signal.store(true, Ordering::SeqCst);

        // join 旧线程，保证真正退出（最多等待 1 个 tick interval）
        if let Some(handle) = self.thread.lock().unwrap().take() {
            // 在 Mutex guard 外 join 防死锁，这里 take() 后 guard 已释放
            let _ = handle.join();
        }

        log::info!("[FocusKeeper] 已停止（线程已 join）");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn test_focus_keeper_initial_state() {
        let fk = MacFocusKeeper::new();
        assert!(!fk.is_active());
    }

    #[test]
    fn test_focus_keeper_stop_when_not_active() {
        let fk = MacFocusKeeper::new();
        // 未激活时 stop 必须幂等不报错
        assert!(fk.stop().is_ok());
        assert!(!fk.is_active());
    }

    #[test]
    fn test_keep_alive_thread_ticks_repeatedly() {
        // 用 1ms 间隔验证：50ms 内至少 tick 多次
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = Arc::clone(&counter);

        let handle = MacFocusKeeper::spawn_keep_alive(
            Arc::clone(&stop),
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            },
            Duration::from_millis(1),
        )
        .expect("spawn 失败");

        std::thread::sleep(Duration::from_millis(50));
        stop.store(true, Ordering::SeqCst);
        handle.join().expect("线程 join 失败");

        let n = counter.load(Ordering::SeqCst);
        assert!(n >= 5, "1ms 周期 50ms 至少 5 次 tick，实际 {}", n);
    }

    #[test]
    fn test_stop_signal_terminates_thread() {
        // 验证 stop_signal 能让线程在合理时间内退出
        let stop = Arc::new(AtomicBool::new(false));

        let handle = MacFocusKeeper::spawn_keep_alive(
            Arc::clone(&stop),
            || {},
            Duration::from_millis(5),
        )
        .expect("spawn 失败");

        std::thread::sleep(Duration::from_millis(20));
        stop.store(true, Ordering::SeqCst);

        let join_result = std::thread::Builder::new()
            .name("test-join-watcher".into())
            .spawn(move || handle.join())
            .unwrap()
            .join();

        assert!(join_result.is_ok(), "线程应在合理时间内退出");
    }

    #[test]
    fn test_double_start_is_idempotent() {
        let fk = MacFocusKeeper::new();
        assert!(fk.start().is_ok());
        assert!(fk.is_active());
        // 第二次 start 必须不重复 spawn 线程，is_active 仍为 true
        assert!(fk.start().is_ok());
        assert!(fk.is_active());
        assert!(fk.stop().is_ok());
        assert!(!fk.is_active());
    }

    /// v0.1.42 code review 1.3 回归保护：
    /// 快速 toggle（start → stop → start）必须每次都成功重启 keep-alive 线程，
    /// 不能因幂等分支把新 start 跳过。
    #[test]
    fn test_rapid_toggle_always_restarts() {
        let fk = MacFocusKeeper::new();

        // 第一轮 start/stop
        assert!(fk.start().is_ok());
        assert!(fk.is_active());
        assert!(fk.stop().is_ok());
        assert!(!fk.is_active(), "stop 后 is_active 必须立即为 false");

        // 第二轮：关键 — 上一轮的 stop_signal 可能还是 true，
        // 新 start 必须清零 stop_signal 并真的 spawn 新线程
        assert!(fk.start().is_ok());
        assert!(fk.is_active(), "第二次 start 必须真正激活");

        // 第三轮：验证可以持续切换
        assert!(fk.stop().is_ok());
        assert!(fk.start().is_ok());
        assert!(fk.is_active());
        assert!(fk.stop().is_ok());
    }

    /// v0.1.42 code review 3.2：
    /// stop() 必须 join 后台线程（不允许 fire-and-forget 泄漏）。
    /// 验证方式：stop() 返回后立即查 thread slot，必须为 None（已被 take+join）。
    #[test]
    fn test_stop_joins_thread() {
        let fk = MacFocusKeeper::new();
        assert!(fk.start().is_ok());
        assert!(fk.thread.lock().unwrap().is_some(), "start 后应持有 handle");
        assert!(fk.stop().is_ok());
        assert!(
            fk.thread.lock().unwrap().is_none(),
            "stop 后 thread handle 必须已被 take"
        );
    }
}
