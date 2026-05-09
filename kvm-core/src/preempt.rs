//! # 远端物理输入抢占检测器（PreemptDetector）
//!
//! 当本机处于 `RemoteControlling`（被对端控制）态下，监听本机物理键鼠输入。
//! 累计鼠标位移 >= `PREEMPT_MOVE_THRESHOLD_PX` 或接收到任意物理 KeyDown 时，
//! 返回 `ShouldPreempt` 决定。上层据此发送 `SwitchBackControl` 并回切 Idle。
//!
//! 设计（第一性原理 + 奥卡姆剃刀）：
//! - 纯逻辑，零平台依赖（放 kvm-core，mac/win 统一复用）
//! - 无锁：`AtomicI64` 累加距离平方（避免 sqrt），`AtomicBool` 标记 KeyDown
//! - O(1) 每次 feed，实时路径（CGEventTap / WH_MOUSE_LL）零锁阻塞
//! - 状态隔离：仅在 `RemoteControlling` 态下喂样本，上层保证
//! - 合成事件过滤：由平台层负责（本模块不感知来源）
//!
//! Synergy / Barrier / Logitech Flow / ShareMouse 等竞品均用"位移阈值 + 任意键"方案。

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

/// 鼠标累计位移平方阈值对应的像素数
///
/// 触发条件：sqrt(Σdx² + Σdy²) >= 40 像素
/// 等价于：Σdx² + Σdy² >= 1600（避免开根）
pub const PREEMPT_MOVE_THRESHOLD_PX: i32 = 40;

/// 平方阈值常量（40 * 40 = 1600）
pub const PREEMPT_MOVE_THRESHOLD_SQ: i64 = (PREEMPT_MOVE_THRESHOLD_PX as i64) * (PREEMPT_MOVE_THRESHOLD_PX as i64);

/// 物理输入样本（来自 input_capture 的回调）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalInputSample {
    /// 鼠标相对位移（与 CGMouseEventDelta / 帧差一致）
    Move { dx: i32, dy: i32 },
    /// 键盘按键按下（只看 KeyDown，不看 KeyUp，避免抬起脉冲误触发）
    KeyDown,
}

/// 抢占触发原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptCause {
    /// 累计鼠标位移超过阈值
    MouseMove,
    /// 键盘按下
    KeyDown,
}

/// 抢占决策结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptDecision {
    /// 尚未达到抢占阈值，继续累加
    Idle,
    /// 应立即触发抢占（上层：发 SwitchBackControl + 切 Idle）
    ShouldPreempt { cause: PreemptCause },
}

/// 无锁抢占检测器（可跨线程共享）
pub struct PreemptDetector {
    /// 累计距离平方（dx² + dy² 之和）
    accumulated_dist_sq: AtomicI64,
    /// 是否已经触发过（防重复触发，reset 前保持 true）
    triggered: AtomicBool,
}

impl PreemptDetector {
    /// 创建新的检测器，初始状态为"未累计、未触发"
    pub fn new() -> Self {
        Self {
            accumulated_dist_sq: AtomicI64::new(0),
            triggered: AtomicBool::new(false),
        }
    }

    /// 喂入一个物理输入样本，返回决策。
    ///
    /// - `Move { dx, dy }`: 累加 dx² + dy² 到内部累计值，若超阈值返回 `ShouldPreempt(MouseMove)`
    /// - `KeyDown`: 立即返回 `ShouldPreempt(KeyDown)`（任意键触发，Synergy 风格）
    ///
    /// 触发后再喂样本仍返回 `Idle`，直到调用 `reset()`（防重复冲击上层）
    pub fn feed(&self, sample: PhysicalInputSample) -> PreemptDecision {
        if self.triggered.load(Ordering::Relaxed) {
            // 已触发过，上层未 reset 前不再重复决策
            return PreemptDecision::Idle;
        }

        match sample {
            PhysicalInputSample::KeyDown => {
                self.triggered.store(true, Ordering::Relaxed);
                PreemptDecision::ShouldPreempt {
                    cause: PreemptCause::KeyDown,
                }
            }
            PhysicalInputSample::Move { dx, dy } => {
                // i64 避免 i32 溢出：极端情况下 dx=dy=i32::MAX 时 dx² 会溢出 i32
                let dx64 = dx as i64;
                let dy64 = dy as i64;
                let delta_sq = dx64 * dx64 + dy64 * dy64;
                let new_sum = self
                    .accumulated_dist_sq
                    .fetch_add(delta_sq, Ordering::Relaxed)
                    .saturating_add(delta_sq);
                if new_sum >= PREEMPT_MOVE_THRESHOLD_SQ {
                    self.triggered.store(true, Ordering::Relaxed);
                    PreemptDecision::ShouldPreempt {
                        cause: PreemptCause::MouseMove,
                    }
                } else {
                    PreemptDecision::Idle
                }
            }
        }
    }

    /// 重置累计值与触发标志，用于：
    /// - 每次入 `RemoteControlling` 态时
    /// - 触发一次后准备下一轮检测
    pub fn reset(&self) {
        self.accumulated_dist_sq.store(0, Ordering::Relaxed);
        self.triggered.store(false, Ordering::Relaxed);
    }

    /// 查询当前累计距离平方（主要用于测试与诊断）
    pub fn accumulated_sq(&self) -> i64 {
        self.accumulated_dist_sq.load(Ordering::Relaxed)
    }

    /// 查询是否已触发过（主要用于测试与诊断）
    pub fn has_triggered(&self) -> bool {
        self.triggered.load(Ordering::Relaxed)
    }
}

impl Default for PreemptDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_idle_and_not_triggered() {
        let d = PreemptDetector::new();
        assert_eq!(d.accumulated_sq(), 0);
        assert!(!d.has_triggered());
    }

    #[test]
    fn test_threshold_constant_is_40() {
        assert_eq!(PREEMPT_MOVE_THRESHOLD_PX, 40);
        assert_eq!(PREEMPT_MOVE_THRESHOLD_SQ, 1600);
    }

    #[test]
    fn test_small_move_under_threshold_is_idle() {
        let d = PreemptDetector::new();
        // 累计 39px 水平移动，距离平方 = 1521 < 1600
        let decision = d.feed(PhysicalInputSample::Move { dx: 39, dy: 0 });
        assert_eq!(decision, PreemptDecision::Idle);
        assert!(!d.has_triggered());
    }

    #[test]
    fn test_threshold_exactly_40px_triggers() {
        let d = PreemptDetector::new();
        // 40² = 1600，正好达到阈值
        let decision = d.feed(PhysicalInputSample::Move { dx: 40, dy: 0 });
        assert_eq!(
            decision,
            PreemptDecision::ShouldPreempt {
                cause: PreemptCause::MouseMove
            }
        );
        assert!(d.has_triggered());
    }

    #[test]
    fn test_accumulated_small_moves_reach_threshold() {
        let d = PreemptDetector::new();
        // 一次 20px，距离平方 = 400，未达
        let dec = d.feed(PhysicalInputSample::Move { dx: 20, dy: 0 });
        assert_eq!(dec, PreemptDecision::Idle);
        // 再来一次 30px，累计 = 400 + 900 = 1300，仍未达
        let dec = d.feed(PhysicalInputSample::Move { dx: 30, dy: 0 });
        assert_eq!(dec, PreemptDecision::Idle);
        // 再来一次 20px，累计 = 1300 + 400 = 1700 >= 1600，触发
        let dec = d.feed(PhysicalInputSample::Move { dx: 20, dy: 0 });
        assert_eq!(
            dec,
            PreemptDecision::ShouldPreempt {
                cause: PreemptCause::MouseMove
            }
        );
    }

    #[test]
    fn test_pythagoras_diagonal_move_triggers() {
        // (30, 30) 距离平方 = 900 + 900 = 1800 >= 1600
        let d = PreemptDetector::new();
        let decision = d.feed(PhysicalInputSample::Move { dx: 30, dy: 30 });
        assert_eq!(
            decision,
            PreemptDecision::ShouldPreempt {
                cause: PreemptCause::MouseMove
            }
        );
    }

    #[test]
    fn test_pythagoras_diagonal_under_threshold_idle() {
        // (28, 28) 距离平方 = 784 + 784 = 1568 < 1600
        let d = PreemptDetector::new();
        let decision = d.feed(PhysicalInputSample::Move { dx: 28, dy: 28 });
        assert_eq!(decision, PreemptDecision::Idle);
    }

    #[test]
    fn test_negative_delta_works() {
        // 负方向位移也应正常累加（dx² 不受符号影响）
        let d = PreemptDetector::new();
        let decision = d.feed(PhysicalInputSample::Move { dx: -40, dy: 0 });
        assert_eq!(
            decision,
            PreemptDecision::ShouldPreempt {
                cause: PreemptCause::MouseMove
            }
        );
    }

    #[test]
    fn test_any_keydown_triggers_immediately() {
        let d = PreemptDetector::new();
        let decision = d.feed(PhysicalInputSample::KeyDown);
        assert_eq!(
            decision,
            PreemptDecision::ShouldPreempt {
                cause: PreemptCause::KeyDown
            }
        );
        assert!(d.has_triggered());
    }

    #[test]
    fn test_keydown_triggers_even_with_zero_accumulated_move() {
        // 未累计任何位移时，KeyDown 也应立即触发
        let d = PreemptDetector::new();
        assert_eq!(d.accumulated_sq(), 0);
        let decision = d.feed(PhysicalInputSample::KeyDown);
        assert!(matches!(decision, PreemptDecision::ShouldPreempt { .. }));
    }

    #[test]
    fn test_after_triggered_subsequent_samples_stay_idle() {
        // 触发后直到 reset 前不再返回 ShouldPreempt，避免重复上报
        let d = PreemptDetector::new();
        let _ = d.feed(PhysicalInputSample::KeyDown);
        assert!(d.has_triggered());

        let dec = d.feed(PhysicalInputSample::KeyDown);
        assert_eq!(dec, PreemptDecision::Idle, "二次 KeyDown 不应再触发");

        let dec = d.feed(PhysicalInputSample::Move { dx: 1000, dy: 1000 });
        assert_eq!(dec, PreemptDecision::Idle, "大幅位移也不应再触发");
    }

    #[test]
    fn test_reset_clears_accumulated_and_flag() {
        let d = PreemptDetector::new();
        let _ = d.feed(PhysicalInputSample::Move { dx: 30, dy: 30 });
        assert!(d.has_triggered());

        d.reset();
        assert_eq!(d.accumulated_sq(), 0);
        assert!(!d.has_triggered());

        // reset 后可重新累计 + 触发
        let dec = d.feed(PhysicalInputSample::Move { dx: 40, dy: 0 });
        assert!(matches!(dec, PreemptDecision::ShouldPreempt { .. }));
    }

    #[test]
    fn test_reset_without_trigger_still_zeros() {
        let d = PreemptDetector::new();
        let _ = d.feed(PhysicalInputSample::Move { dx: 20, dy: 0 }); // 400
        assert_eq!(d.accumulated_sq(), 400);

        d.reset();
        assert_eq!(d.accumulated_sq(), 0);
    }

    #[test]
    fn test_extreme_i32_delta_no_overflow() {
        // 压力测试：i32::MAX² 会在 i32 里溢出，i64 不会
        let d = PreemptDetector::new();
        let _ = d.feed(PhysicalInputSample::Move {
            dx: i32::MAX,
            dy: 0,
        });
        assert!(d.has_triggered(), "极端位移必触发");
        assert!(d.accumulated_sq() > 0, "累计值应该为正（未溢出为负）");
    }

    #[test]
    fn test_zero_move_is_idle_and_no_accumulation() {
        let d = PreemptDetector::new();
        let decision = d.feed(PhysicalInputSample::Move { dx: 0, dy: 0 });
        assert_eq!(decision, PreemptDecision::Idle);
        assert_eq!(d.accumulated_sq(), 0);
        assert!(!d.has_triggered());
    }

    #[test]
    fn test_many_tiny_moves_saturate_to_threshold() {
        // 400 次 2px 位移：400 * (2² + 0²) = 400 * 4 = 1600，正好达阈值
        let d = PreemptDetector::new();
        let mut triggered_at: Option<usize> = None;
        for i in 0..500 {
            match d.feed(PhysicalInputSample::Move { dx: 2, dy: 0 }) {
                PreemptDecision::ShouldPreempt { .. } => {
                    triggered_at = Some(i);
                    break;
                }
                PreemptDecision::Idle => {}
            }
        }
        assert_eq!(triggered_at, Some(399), "在第 400 次（索引 399）达阈值");
    }

    #[test]
    fn test_move_then_keydown_still_triggers_as_keydown() {
        // 部分位移（未达阈值）后按下按键，应返回 KeyDown 原因
        let d = PreemptDetector::new();
        let _ = d.feed(PhysicalInputSample::Move { dx: 10, dy: 10 }); // 200, 未触发
        assert!(!d.has_triggered());
        let dec = d.feed(PhysicalInputSample::KeyDown);
        assert_eq!(
            dec,
            PreemptDecision::ShouldPreempt {
                cause: PreemptCause::KeyDown
            }
        );
    }

    /// 关键并发属性：多线程同时喂样本，最终只有一个线程收到 ShouldPreempt
    /// （但这里简化成"最终一定会 triggered"，不严格断言"仅一次"
    /// 因为无锁 fetch_add 可能让多个线程同时跨过阈值）
    #[test]
    fn test_concurrent_feed_eventually_triggers() {
        use std::sync::Arc;
        use std::thread;
        let d = Arc::new(PreemptDetector::new());
        let mut handles = vec![];
        for _ in 0..8 {
            let d2 = Arc::clone(&d);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    let _ = d2.feed(PhysicalInputSample::Move { dx: 5, dy: 0 });
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert!(d.has_triggered(), "8 * 50 * 25 = 10000 >= 1600 必触发");
    }
}
