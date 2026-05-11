//! 状态守卫模块（State Guard）
//!
//! 职责：
//! 1. Watchdog 看门狗：被控端 RemoteControlling 态下，3s 无远端帧自动回 Idle
//! 2. 本地强制恢复快捷键（Win: Ctrl+Alt+S / Mac: Ctrl+Option+S）
//! 3. 统一状态快照推送给前端（消除事件顺序不确定性）
//! 4. 结构化日志

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Notify;
use tauri::Manager;

/// Watchdog 超时阈值（毫秒）：被控端连续 3s 无远端帧 → 自动回 Idle
const WATCHDOG_TIMEOUT_MS: u64 = 3000;

/// Watchdog 检查间隔（毫秒）
const WATCHDOG_CHECK_INTERVAL_MS: u64 = 500;

/// 状态快照（原子推送给前端，确保一致性）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateSnapshot {
    /// 连接状态
    pub connected: bool,
    /// 控制状态 (Idle/Armed/LocalControlling/RemoteControlling)
    pub control_state: String,
    /// Watchdog 是否激活
    pub watchdog_active: bool,
    /// 时间戳（Unix ms）
    pub timestamp_ms: u64,
}

/// 状态守卫（管理 Watchdog 生命周期 + 状态快照）
pub struct StateGuard {
    /// 最后收到远端帧的时间戳（毫秒，0 表示 Watchdog 未激活）
    last_remote_frame_ms: Arc<AtomicU64>,
    /// Watchdog 是否运行中
    watchdog_running: Arc<AtomicBool>,
    /// 停止 Watchdog 的信号
    watchdog_stop: Arc<Notify>,
}

impl StateGuard {
    pub fn new() -> Self {
        Self {
            last_remote_frame_ms: Arc::new(AtomicU64::new(0)),
            watchdog_running: Arc::new(AtomicBool::new(false)),
            watchdog_stop: Arc::new(Notify::new()),
        }
    }

    /// 记录收到远端帧（由 inject_remote_frame 每帧调用）
    ///
    /// 更新 Watchdog 的"最后活跃时间"，防止超时误触发。
    pub fn touch_remote_frame(&self) {
        let now = now_ms();
        self.last_remote_frame_ms.store(now, Ordering::Relaxed);
    }

    /// 启动 Watchdog（进入 RemoteControlling 时调用）
    ///
    /// 每 500ms 检查一次：如果距离上次 touch_remote_frame 超过 3s，
    /// 判定远端已断连/失联，强制调用 on_disconnected 回到 Idle。
    pub fn start_watchdog(&self, app: tauri::AppHandle) {
        if self.watchdog_running.swap(true, Ordering::SeqCst) {
            log::warn!("[StateGuard] Watchdog 已在运行，跳过重复启动");
            return;
        }

        // 初始化时间戳（避免旧时间戳导致立即触发）
        self.touch_remote_frame();

        let last_frame = Arc::clone(&self.last_remote_frame_ms);
        let running = Arc::clone(&self.watchdog_running);
        let stop = Arc::clone(&self.watchdog_stop);

        log::info!(
            "[StateGuard] 🐕 Watchdog 已启动（超时={}ms，检查间隔={}ms）",
            WATCHDOG_TIMEOUT_MS,
            WATCHDOG_CHECK_INTERVAL_MS
        );

        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.notified() => {
                        log::info!("[StateGuard] Watchdog 收到停止信号，退出");
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(WATCHDOG_CHECK_INTERVAL_MS)) => {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }

                        let last = last_frame.load(Ordering::Relaxed);
                        if last == 0 {
                            continue; // 未初始化，跳过
                        }

                        let now = now_ms();
                        let elapsed = now.saturating_sub(last);

                        log::trace!(
                            "[StateGuard] Watchdog tick: elapsed={}ms, threshold={}ms",
                            elapsed,
                            WATCHDOG_TIMEOUT_MS
                        );

                        if elapsed >= WATCHDOG_TIMEOUT_MS {
                            log::warn!(
                                "[StateGuard] ⚠️ Watchdog 超时！{}ms 未收到远端帧，强制回 Idle (trigger=watchdog_timeout)",
                                elapsed
                            );
                            trigger_force_idle(&app).await;
                            break;
                        }
                    }
                }
            }
            running.store(false, Ordering::SeqCst);
            log::info!("[StateGuard] Watchdog 已退出");
        });
    }

    /// 停止 Watchdog（离开 RemoteControlling 时调用）
    pub fn stop_watchdog(&self) {
        if self.watchdog_running.swap(false, Ordering::SeqCst) {
            self.watchdog_stop.notify_waiters();
            self.last_remote_frame_ms.store(0, Ordering::Relaxed);
            log::info!("[StateGuard] Watchdog 已停止");
        }
    }

    /// Watchdog 是否运行中
    pub fn is_watchdog_running(&self) -> bool {
        self.watchdog_running.load(Ordering::SeqCst)
    }
}

impl Default for StateGuard {
    fn default() -> Self {
        Self::new()
    }
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 构建当前状态快照（供前端 sharing-state-sync 事件使用）
pub fn build_snapshot(connected: bool, control_state: &str, watchdog_active: bool) -> StateSnapshot {
    StateSnapshot {
        connected,
        control_state: control_state.to_string(),
        watchdog_active,
        timestamp_ms: now_ms(),
    }
}

/// Watchdog 超时触发：通过 SharingManager.on_disconnected 强制回 Idle
async fn trigger_force_idle(app: &tauri::AppHandle) {
    if let Some(sm_state) =
        app.try_state::<Arc<crate::sharing::SharingManager>>()
    {
        let sm = sm_state.inner().clone();
        log::warn!("[StateGuard] STATE_CHANGE: RemoteControlling → Idle (trigger=watchdog_timeout)");
        sm.on_disconnected(app).await;
    } else {
        log::error!("[StateGuard] 无法获取 SharingManager，Watchdog 恢复失败！");
    }
}

/// 判断当前是否已连接（从 ConnectionManager 读取状态）
pub async fn is_connected_via_app(app: &tauri::AppHandle) -> bool {
    use crate::connection::KvmConnectionManager;
    if let Some(mgr) = app.try_state::<KvmConnectionManager>() {
        let status = mgr.get_status().await;
        status.connected
    } else {
        false
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_guard_new() {
        let guard = StateGuard::new();
        assert!(!guard.is_watchdog_running());
        assert_eq!(guard.last_remote_frame_ms.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_touch_remote_frame_updates_timestamp() {
        let guard = StateGuard::new();
        guard.touch_remote_frame();
        let ts = guard.last_remote_frame_ms.load(Ordering::Relaxed);
        assert!(ts > 0);
        // 时间戳应该接近当前时间（误差 < 1s）
        let now = now_ms();
        assert!(now - ts < 1000);
    }

    #[test]
    fn test_stop_watchdog_clears_timestamp() {
        let guard = StateGuard::new();
        guard.touch_remote_frame();
        assert!(guard.last_remote_frame_ms.load(Ordering::Relaxed) > 0);

        // 模拟 watchdog_running = true（正常流程由 start_watchdog 设置）
        guard.watchdog_running.store(true, Ordering::SeqCst);
        guard.stop_watchdog();

        assert_eq!(guard.last_remote_frame_ms.load(Ordering::Relaxed), 0);
        assert!(!guard.is_watchdog_running());
    }

    #[test]
    fn test_build_snapshot() {
        let snap = build_snapshot(true, "RemoteControlling", true);
        assert!(snap.connected);
        assert_eq!(snap.control_state, "RemoteControlling");
        assert!(snap.watchdog_active);
        assert!(snap.timestamp_ms > 0);
    }

    #[test]
    fn test_stop_watchdog_idempotent() {
        let guard = StateGuard::new();
        // 未启动时停止不 panic
        guard.stop_watchdog();
        guard.stop_watchdog();
        assert!(!guard.is_watchdog_running());
    }
}
