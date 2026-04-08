//! 边缘触发判定（纯坐标计算）
//!
//! 基于虚拟桌面外边缘检测，支持左/右/上/下四个方向。
//! 不涉及系统 API，纯逻辑计算。
//! 详细实现将在任务 5 中完成。

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
}

impl EdgeConfig {
    pub fn from_layout(layout: ScreenLayout) -> Self {
        Self {
            trigger_edge: layout.trigger_edge(),
            threshold: 1,
            debounce_ms: 50,
        }
    }
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            trigger_edge: ScreenEdge::Right,
            threshold: 1,
            debounce_ms: 50,
        }
    }
}

/// 边缘检测结果
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeDetectResult {
    /// 未到达边缘
    None,
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
}

impl EdgeDetector {
    pub fn new(config: EdgeConfig, desktop: VirtualDesktop) -> Self {
        Self {
            config,
            desktop,
            last_trigger: None,
            active: true,
        }
    }

    /// 更新配置
    pub fn set_config(&mut self, config: EdgeConfig) {
        self.config = config;
    }

    /// 更新虚拟桌面信息
    pub fn update_desktop(&mut self, desktop: VirtualDesktop) {
        self.desktop = desktop;
    }

    /// 设置是否激活（控制远端时不检测）
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// 检测鼠标位置是否触发边缘切换
    /// x, y 为虚拟桌面绝对坐标（逻辑像素）
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

        let triggered = match self.config.trigger_edge {
            ScreenEdge::Left => x <= ox + t - 1,
            ScreenEdge::Right => x >= ox + w - t,
            ScreenEdge::Top => y <= oy + t - 1,
            ScreenEdge::Bottom => y >= oy + h - t,
        };

        if triggered {
            let pos_ratio = match self.config.trigger_edge {
                ScreenEdge::Left | ScreenEdge::Right => calc_ratio(y - oy, h),
                ScreenEdge::Top | ScreenEdge::Bottom => calc_ratio(x - ox, w),
            };
            self.last_trigger = Some(std::time::Instant::now());
            EdgeDetectResult::Triggered {
                direction: self.config.trigger_edge,
                pos_ratio,
            }
        } else {
            EdgeDetectResult::None
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
    fn test_right_edge_trigger() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        // 不在边缘
        assert_eq!(detector.detect(960, 540), EdgeDetectResult::None);

        // 在右边缘
        let result = detector.detect(1919, 540);
        assert!(matches!(result, EdgeDetectResult::Triggered { direction: ScreenEdge::Right, .. }));
    }

    #[test]
    fn test_left_edge_trigger() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Left, threshold: 1, debounce_ms: 0 };
        let mut detector = EdgeDetector::new(config, desktop);

        let result = detector.detect(0, 540);
        assert!(matches!(result, EdgeDetectResult::Triggered { direction: ScreenEdge::Left, .. }));
    }

    #[test]
    fn test_inactive_detector() {
        let desktop = make_desktop(1920, 1080);
        let config = EdgeConfig { trigger_edge: ScreenEdge::Right, threshold: 1, debounce_ms: 0 };
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
}
