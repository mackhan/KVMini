//! 边缘触发判定（纯坐标计算）
//!
//! 基于虚拟桌面外边缘检测，支持左/右/上/下四个方向。
//! 不涉及系统 API，纯逻辑计算。
//! v0.1.40：新增过冲量（overshoot）防误触机制。
//! v0.1.41 BUG-029：macOS 鼠标坐标被系统 clamp 在屏幕内，push_coord 永远 ≤ 0，
//!   overshoot 机制永远无法满足。改为"持续贴边计数"：鼠标连续 N 次 MouseMove 都在
//!   边缘区域内即触发（N = overshoot_px，复用字段）。

use crate::config::ScreenLayout;
use crate::protocol::ScreenEdge;
use crate::platform::VirtualDesktop;

/// 边缘检测配置
#[derive(Debug, Clone)]
pub struct EdgeConfig {
    /// 触发边缘方向
    pub trigger_edge: ScreenEdge,
    /// 边缘触发阈值（像素），默认 1
    pub threshold: i32,
    /// 防抖延迟（毫秒）
    pub debounce_ms: u64,
    /// 过冲量（像素），需推过边缘 N 像素才真正触发，0 = 贴边即切
    pub overshoot_px: i32,
}

impl EdgeConfig {
    pub fn from_layout(layout: ScreenLayout) -> Self {
        Self {
            trigger_edge: layout.trigger_edge(),
            threshold: 1,
            debounce_ms: 50,
            overshoot_px: 20,
        }
    }

    pub fn from_layout_with_overshoot(layout: ScreenLayout, overshoot_px: i32) -> Self {
        Self {
            trigger_edge: layout.trigger_edge(),
            threshold: 1,
            debounce_ms: 50,
            overshoot_px: overshoot_px.clamp(0, 200),
        }
    }
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            trigger_edge: ScreenEdge::Right,
            threshold: 1,
            debounce_ms: 50,
            overshoot_px: 20,
        }
    }
}

/// 边缘检测结果
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeDetectResult {
    /// 未到达边缘
    None,
    /// 到达边缘但过冲量未满足（正在累积中）
    Accumulating {
        /// 当前累积量
        current: i32,
        /// 需要达到的阈值
        required: i32,
    },
    /// 到达边缘，触发切换
    Triggered {
        /// 触发方向
        direction: ScreenEdge,
        /// 沿边缘方向的位置比例（0-10000）
        pos_ratio: u16,
    },
}

/// 边缘检测器（纯逻辑，不依赖平台 API）
pub struct EdgeDetector {
    config: EdgeConfig,
    desktop: VirtualDesktop,
    last_trigger: Option<std::time::Instant>,
    active: bool,
    /// BUG-029: 持续贴边计数（连续在边缘区域内的 MouseMove 次数）
    /// 替代原 overshoot_accumulator，因为 macOS 坐标被系统 clamp 无法超出屏幕
    edge_hit_count: i32,
    /// 是否已贴边（用于判断首次到达边缘）
    at_edge: bool,
}

impl EdgeDetector {
    pub fn new(config: EdgeConfig, desktop: VirtualDesktop) -> Self {
        Self {
            config,
            desktop,
            last_trigger: None,
            active: true,
            edge_hit_count: 0,
            at_edge: false,
        }
    }

    /// 更新配置
    pub fn set_config(&mut self, config: EdgeConfig) {
        self.config = config;
        self.reset_overshoot();
    }

    /// 更新虚拟桌面信息
    pub fn update_desktop(&mut self, desktop: VirtualDesktop) {
        self.desktop = desktop;
    }

    /// 设置是否激活（控制远端时不检测）
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if !active {
            self.reset_overshoot();
        }
    }

    /// 重置贴边计数
    pub fn reset_overshoot(&mut self) {
        self.edge_hit_count = 0;
        self.at_edge = false;
    }

    /// 检测鼠标位置是否触发边缘切换
    /// x, y 为虚拟桌面绝对坐标（逻辑像素）
    /// BUG-029: macOS 坐标被系统 clamp，改为"连续贴边次数"触发
    pub fn detect(&mut self, x: i32, y: i32) -> EdgeDetectResult {
        if !self.active {
            return EdgeDetectResult::None;
        }

        // 防抖
        if let Some(last) = self.last_trigger {
            if last.elapsed().as_millis() < self.config.debounce_ms as u128 {
                return EdgeDetectResult::None;
            }
        }

        let t = self.config.threshold;
        let ox = self.desktop.origin_x;
        let oy = self.desktop.origin_y;
        let w = self.desktop.total_width;
        let h = self.desktop.total_height;

        // 判断是否在边缘区域内
        let at_edge_now = match self.config.trigger_edge {
            ScreenEdge::Left => x <= ox + t - 1,
            ScreenEdge::Right => x >= ox + w - t,
            ScreenEdge::Top => y <= oy + t - 1,
            ScreenEdge::Bottom => y >= oy + h - t,
        };

        if !at_edge_now {
            // 离开边缘区域，重置计数器
            self.reset_overshoot();
            return EdgeDetectResult::None;
        }

        // overshoot_px 为 0 时立即触发（兼容旧行为）
        if self.config.overshoot_px <= 0 {
            let pos_ratio = match self.config.trigger_edge {
                ScreenEdge::Left | ScreenEdge::Right => calc_ratio(y - oy, h),
                ScreenEdge::Top | ScreenEdge::Bottom => calc_ratio(x - ox, w),
            };
            self.last_trigger = Some(std::time::Instant::now());
            self.reset_overshoot();
            return EdgeDetectResult::Triggered {
                direction: self.config.trigger_edge,
                pos_ratio,
            };
        }

        // BUG-029: 持续贴边计数逻辑
        // macOS 坐标被 clamp 在屏幕内，无法"推过"边缘获得正的 push_coord
        // 改为：每次 MouseMove 在边缘区域内就 +1，连续 N 次后触发
        if !self.at_edge {
            self.at_edge = true;
            self.edge_hit_count = 1;
        } else {
            self.edge_hit_count += 1;
        }

        if self.edge_hit_count >= self.config.overshoot_px {
            // 贴边次数达到阈值，触发！
            let pos_ratio = match self.config.trigger_edge {
                ScreenEdge::Left | ScreenEdge::Right => calc_ratio(y - oy, h),
                ScreenEdge::Top | ScreenEdge::Bottom => calc_ratio(x - ox, w),
            };
            self.last_trigger = Some(std::time::Instant::now());
            self.reset_overshoot();
            EdgeDetectResult::Triggered {
                direction: self.config.trigger_edge,
                pos_ratio,
            }
        } else {
            EdgeDetectResult::Accumulating {
                current: self.edge_hit_count,
                required: self.config.overshoot_px,
            }
        }
    }

    /// 检测被控方光标是否到达反向边缘（用于控制权回切）
    pub fn detect_return_edge(&self, x: i32, y: i32, remote_desktop: &VirtualDesktop) -> bool {
        let return_edge = self.config.trigger_edge.opposite();
        let t = self.config.threshold;
        let ox = remote_desktop.origin_x;
        let oy = remote_desktop.origin_y;
        let w = remote_desktop.total_width;
        let h = remote_desktop.total_height;

        match return_edge {
            ScreenEdge::Left => x <= ox + t - 1,
            ScreenEdge::Right => x >= ox + w - t,
            ScreenEdge::Top => y <= oy + t - 1,
            ScreenEdge::Bottom => y >= oy + h - t,
        }
    }
}

fn calc_ratio(pos: i32, total: i32) -> u16 {
    if total <= 0 { return 0; }
    ((pos as i64 * 10000 / total as i64).clamp(0, 10000)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::MonitorInfo;

    fn make_desktop(w: i32, h: i32) -> VirtualDesktop {
        VirtualDesktop::from_monitors(vec![MonitorInfo {
            id: 0, width: w, height: h, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }])
    }

    #[test]
    fn test_right_edge_trigger_with_zero_overshoot() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        // 不在边缘
        assert_eq!(detector.detect(960, 540), EdgeDetectResult::None);

        // 在右边缘，overshoot=0 应立即触发
        let result = detector.detect(1919, 540);
        assert!(matches!(result, EdgeDetectResult::Triggered { direction: ScreenEdge::Right, .. }));
    }

    #[test]
    fn test_right_edge_overshoot_accumulation() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 20 };
        let mut detector = EdgeDetector::new(config, desktop);

        // BUG-029: 改为持续贴边计数——第一次贴边 count=1
        let result = detector.detect(1919, 540);
        assert!(matches!(result, EdgeDetectResult::Accumulating { current: 1, required: 20 }));

        // 第二次贴边 count=2
        let result = detector.detect(1919, 540);
        assert!(matches!(result, EdgeDetectResult::Accumulating { current: 2, required: 20 }));

        // 连续贴边到第 20 次应触发
        for _ in 3..20 {
            let result = detector.detect(1919, 540);
            assert!(matches!(result, EdgeDetectResult::Accumulating { .. }));
        }
        let result = detector.detect(1919, 540);
        assert!(matches!(result, EdgeDetectResult::Triggered { direction: ScreenEdge::Right, .. }));
    }

    #[test]
    fn test_left_edge_trigger_with_overshoot() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Left, threshold: 1, debounce_ms: 0, overshoot_px: 5 };
        let mut detector = EdgeDetector::new(config, desktop);

        // BUG-029: 第一次贴边 count=1
        let result = detector.detect(0, 540);
        assert!(matches!(result, EdgeDetectResult::Accumulating { current: 1, .. }));

        // 不在边缘，重置
        detector.detect(100, 540);

        // 再次贴边时是全新一轮，count=1
        let result = detector.detect(0, 540);
        assert!(matches!(result, EdgeDetectResult::Accumulating { current: 1, .. }));
    }

    #[test]
    fn test_overshoot_reset_on_leave_edge() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 10 };
        let mut detector = EdgeDetector::new(config, desktop);

        // 贴边
        detector.detect(1919, 540);
        assert!(detector.at_edge);

        // 离开边缘
        let result = detector.detect(1900, 540);
        assert_eq!(result, EdgeDetectResult::None);
        assert!(!detector.at_edge);
        assert_eq!(detector.edge_hit_count, 0);
    }

    #[test]
    fn test_inactive_detector() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let mut detector = EdgeDetector::new(config, desktop);
        detector.set_active(false);

        assert_eq!(detector.detect(1919, 540), EdgeDetectResult::None);
    }

    #[test]
    fn test_pos_ratio_calculation() {
        assert_eq!(calc_ratio(540, 1080), 5000);
        assert_eq!(calc_ratio(0, 1080), 0);
        assert_eq!(calc_ratio(1080, 1080), 10000);
    }

    #[test]
    fn test_detect_return_edge() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let detector = EdgeDetector::new(config, desktop.clone());

        // 反向边缘是 Left
        assert!(detector.detect_return_edge(0, 540, &desktop));
        assert!(!detector.detect_return_edge(100, 540, &desktop));
    }

    #[test]
    fn test_overshoot_zero_equals_immediate_trigger() {
        let desktop = make_desktop(1920, 1080);
        // overshoot = 0 等价于旧版立即触发行为
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        let result = detector.detect(1919, 270);
        match result {
            EdgeDetectResult::Triggered { direction, pos_ratio } => {
                assert_eq!(direction, ScreenEdge::Right);
                assert_eq!(pos_ratio, 2500); // 270/1080 * 10000
            }
            _ => panic!("Expected Triggered"),
        }
    }

    #[test]
    fn test_y_ratio_calculation_at_trigger() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        // Y=810 → ratio = 810/1080 * 10000 = 7500
        let result = detector.detect(1919, 810);
        match result {
            EdgeDetectResult::Triggered { pos_ratio, .. } => {
                assert_eq!(pos_ratio, 7500);
            }
            _ => panic!("Expected Triggered"),
        }
    }

    #[test]
    fn test_x_ratio_calculation_at_bottom_edge() {
        // v0.1.40 pos_ratio 对 Top/Bottom 边应计算 X 比例
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Bottom, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        // X=1440, Y=1079（贴底边）→ pos_ratio = 1440/1920 * 10000 = 7500
        let result = detector.detect(1440, 1079);
        match result {
            EdgeDetectResult::Triggered { direction, pos_ratio } => {
                assert_eq!(direction, ScreenEdge::Bottom);
                assert_eq!(pos_ratio, 7500);
            }
            _ => panic!("Expected Triggered on Bottom edge"),
        }
    }

    #[test]
    fn test_pos_ratio_on_top_edge() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Top, threshold: 1, debounce_ms: 0, overshoot_px: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        // X=480, Y=0 → pos_ratio = 480/1920 * 10000 = 2500
        let result = detector.detect(480, 0);
        match result {
            EdgeDetectResult::Triggered { direction, pos_ratio } => {
                assert_eq!(direction, ScreenEdge::Top);
                assert_eq!(pos_ratio, 2500);
            }
            _ => panic!("Expected Triggered on Top edge"),
        }
    }
}
