/// 屏幕边缘检测与控制权切换逻辑
///
/// 当主控端鼠标到达屏幕边缘时，根据显示器布局配置决定是否切换控制权。
/// 切换时：
/// 1. 主控端隐藏本机鼠标光标，停止向本机注入事件
/// 2. 向被控端发送 SwitchControl 协议帧（含边缘位置比例）
/// 3. 被控端在对应边缘位置显示鼠标光标
///
/// 支持 macOS 和 Windows 坐标系（均使用归一化坐标 * 10000）

use crate::protocol::ScreenEdge;

/// 屏幕尺寸
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSize {
    pub width: i32,
    pub height: i32,
}

impl ScreenSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

/// 边缘检测结果
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeDetectResult {
    /// 未到达边缘
    None,
    /// 到达边缘，包含边缘类型和位置比例（0-10000）
    Edge {
        edge: ScreenEdge,
        /// 沿边缘方向的位置比例（0-10000）
        /// 左/右边缘：y 方向比例；上/下边缘：x 方向比例
        pos_ratio: u16,
    },
}

/// 边缘检测器
///
/// 检测鼠标是否到达屏幕边缘（考虑 1 像素的边缘阈值）
pub struct EdgeDetector {
    screen: ScreenSize,
    /// 边缘触发阈值（像素），默认 1
    threshold: i32,
}

impl EdgeDetector {
    pub fn new(screen: ScreenSize) -> Self {
        Self { screen, threshold: 1 }
    }

    pub fn with_threshold(mut self, threshold: i32) -> Self {
        self.threshold = threshold;
        self
    }

    /// 检测鼠标位置是否到达屏幕边缘
    /// x, y 为屏幕像素坐标（macOS 和 Windows 均适用）
    pub fn detect(&self, x: i32, y: i32) -> EdgeDetectResult {
        let w = self.screen.width;
        let h = self.screen.height;
        let t = self.threshold;

        // 优先检测角落（左右优先于上下）
        if x <= t - 1 {
            // 左边缘
            let pos_ratio = self.calc_pos_ratio(y, h);
            return EdgeDetectResult::Edge { edge: ScreenEdge::Left, pos_ratio };
        }
        if x >= w - t {
            // 右边缘
            let pos_ratio = self.calc_pos_ratio(y, h);
            return EdgeDetectResult::Edge { edge: ScreenEdge::Right, pos_ratio };
        }
        if y <= t - 1 {
            // 上边缘
            let pos_ratio = self.calc_pos_ratio(x, w);
            return EdgeDetectResult::Edge { edge: ScreenEdge::Top, pos_ratio };
        }
        if y >= h - t {
            // 下边缘
            let pos_ratio = self.calc_pos_ratio(x, w);
            return EdgeDetectResult::Edge { edge: ScreenEdge::Bottom, pos_ratio };
        }

        EdgeDetectResult::None
    }

    /// 计算位置比例（0-10000）
    fn calc_pos_ratio(&self, pos: i32, total: i32) -> u16 {
        if total <= 0 {
            return 0;
        }
        let ratio = (pos as i64 * 10000 / total as i64).clamp(0, 10000) as u16;
        ratio
    }
}

/// 显示器布局方向（本机相对于远端的位置）
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DisplayLayout {
    /// 本机在远端的左侧（鼠标超出右边缘切换到远端）
    LocalLeftOfRemote,
    /// 本机在远端的右侧（鼠标超出左边缘切换到远端）
    LocalRightOfRemote,
    /// 本机在远端的上方（鼠标超出下边缘切换到远端）
    LocalAboveRemote,
    /// 本机在远端的下方（鼠标超出上边缘切换到远端）
    LocalBelowRemote,
}

impl DisplayLayout {
    /// 根据布局，判断哪个边缘触发切换到远端
    pub fn trigger_edge(&self) -> ScreenEdge {
        match self {
            Self::LocalLeftOfRemote => ScreenEdge::Right,
            Self::LocalRightOfRemote => ScreenEdge::Left,
            Self::LocalAboveRemote => ScreenEdge::Bottom,
            Self::LocalBelowRemote => ScreenEdge::Top,
        }
    }

    /// 切换到远端后，鼠标应出现在远端的哪个边缘
    pub fn remote_entry_edge(&self) -> ScreenEdge {
        match self {
            Self::LocalLeftOfRemote => ScreenEdge::Left,
            Self::LocalRightOfRemote => ScreenEdge::Right,
            Self::LocalAboveRemote => ScreenEdge::Top,
            Self::LocalBelowRemote => ScreenEdge::Bottom,
        }
    }

    /// 从远端返回时，鼠标超出远端的哪个边缘触发返回
    pub fn remote_return_edge(&self) -> ScreenEdge {
        // 与 remote_entry_edge 相反
        match self {
            Self::LocalLeftOfRemote => ScreenEdge::Right,
            Self::LocalRightOfRemote => ScreenEdge::Left,
            Self::LocalAboveRemote => ScreenEdge::Bottom,
            Self::LocalBelowRemote => ScreenEdge::Top,
        }
    }
}

/// 控制权切换管理器
///
/// 管理主控端的控制权状态，决定何时切换到被控端
pub struct SwitchManager {
    /// 本机屏幕尺寸
    local_screen: ScreenSize,
    /// 显示器布局
    layout: DisplayLayout,
    /// 边缘检测器
    detector: EdgeDetector,
    /// 当前是否在控制远端（true = 控制远端，false = 控制本机）
    controlling_remote: bool,
}

impl SwitchManager {
    pub fn new(local_screen: ScreenSize, layout: DisplayLayout) -> Self {
        let detector = EdgeDetector::new(local_screen);
        Self {
            local_screen,
            layout,
            detector,
            controlling_remote: false,
        }
    }

    /// 处理鼠标移动事件，返回是否需要切换控制权
    pub fn on_mouse_move(&mut self, x: i32, y: i32) -> SwitchAction {
        if self.controlling_remote {
            // 当前控制远端，不需要检测本机边缘
            return SwitchAction::None;
        }

        let result = self.detector.detect(x, y);
        match result {
            EdgeDetectResult::Edge { edge, pos_ratio } if edge == self.layout.trigger_edge() => {
                self.controlling_remote = true;
                SwitchAction::SwitchToRemote {
                    remote_entry_edge: self.layout.remote_entry_edge(),
                    pos_ratio,
                }
            }
            _ => SwitchAction::None,
        }
    }

    /// 处理从被控端收到的"返回控制权"事件
    pub fn on_return_from_remote(&mut self) {
        self.controlling_remote = false;
    }

    /// 是否正在控制远端
    pub fn is_controlling_remote(&self) -> bool {
        self.controlling_remote
    }

    /// 获取当前布局
    pub fn layout(&self) -> DisplayLayout {
        self.layout
    }

    /// 更新布局配置
    pub fn set_layout(&mut self, layout: DisplayLayout) {
        self.layout = layout;
    }
}

/// 控制权切换动作
#[derive(Debug, Clone, PartialEq)]
pub enum SwitchAction {
    /// 无需切换
    None,
    /// 切换到远端
    SwitchToRemote {
        /// 鼠标应出现在远端的哪个边缘
        remote_entry_edge: ScreenEdge,
        /// 沿边缘方向的位置比例（0-10000）
        pos_ratio: u16,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── EdgeDetector 测试（macOS 和 Windows 坐标系均适用）──────────────

    #[test]
    fn test_detect_no_edge_center() {
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        assert_eq!(detector.detect(960, 540), EdgeDetectResult::None);
    }

    #[test]
    fn test_detect_left_edge() {
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(0, 540);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Left,
            pos_ratio: 5000, // 540 / 1080 * 10000 = 5000
        });
    }

    #[test]
    fn test_detect_right_edge() {
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(1919, 270);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Right,
            pos_ratio: 2500, // 270 / 1080 * 10000 = 2500
        });
    }

    #[test]
    fn test_detect_top_edge() {
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(960, 0);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Top,
            pos_ratio: 5000, // 960 / 1920 * 10000 = 5000
        });
    }

    #[test]
    fn test_detect_bottom_edge() {
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(1440, 1079);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Bottom,
            pos_ratio: 7500, // 1440 / 1920 * 10000 = 7500
        });
    }

    #[test]
    fn test_detect_left_edge_top_corner() {
        // 左上角：左边缘优先
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(0, 0);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Left,
            pos_ratio: 0,
        });
    }

    #[test]
    fn test_detect_right_edge_bottom_corner() {
        // 右下角：右边缘优先
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(1919, 1079);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Right,
            pos_ratio: 9990, // 1079 * 10000 / 1080 = 9990（整数除法）
        });
    }

    #[test]
    fn test_detect_4k_screen() {
        // 4K 屏幕（2560x1440）
        let detector = EdgeDetector::new(ScreenSize::new(2560, 1440));
        let result = detector.detect(2559, 720);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Right,
            pos_ratio: 5000, // 720 / 1440 * 10000 = 5000
        });
    }

    #[test]
    fn test_detect_with_threshold_2() {
        // 阈值为 2 时，x=1 也应触发左边缘
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080))
            .with_threshold(2);
        assert_eq!(
            detector.detect(1, 540),
            EdgeDetectResult::Edge { edge: ScreenEdge::Left, pos_ratio: 5000 }
        );
        // x=2 不触发
        assert_eq!(detector.detect(2, 540), EdgeDetectResult::None);
    }

    // ── DisplayLayout 测试 ───────────────────────────────────────────────

    #[test]
    fn test_layout_local_left_of_remote() {
        let layout = DisplayLayout::LocalLeftOfRemote;
        assert_eq!(layout.trigger_edge(), ScreenEdge::Right);
        assert_eq!(layout.remote_entry_edge(), ScreenEdge::Left);
        assert_eq!(layout.remote_return_edge(), ScreenEdge::Right);
    }

    #[test]
    fn test_layout_local_right_of_remote() {
        let layout = DisplayLayout::LocalRightOfRemote;
        assert_eq!(layout.trigger_edge(), ScreenEdge::Left);
        assert_eq!(layout.remote_entry_edge(), ScreenEdge::Right);
        assert_eq!(layout.remote_return_edge(), ScreenEdge::Left);
    }

    #[test]
    fn test_layout_local_above_remote() {
        let layout = DisplayLayout::LocalAboveRemote;
        assert_eq!(layout.trigger_edge(), ScreenEdge::Bottom);
        assert_eq!(layout.remote_entry_edge(), ScreenEdge::Top);
        assert_eq!(layout.remote_return_edge(), ScreenEdge::Bottom);
    }

    #[test]
    fn test_layout_local_below_remote() {
        let layout = DisplayLayout::LocalBelowRemote;
        assert_eq!(layout.trigger_edge(), ScreenEdge::Top);
        assert_eq!(layout.remote_entry_edge(), ScreenEdge::Bottom);
        assert_eq!(layout.remote_return_edge(), ScreenEdge::Top);
    }

    // ── SwitchManager 测试 ───────────────────────────────────────────────

    #[test]
    fn test_switch_manager_no_switch_in_center() {
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalLeftOfRemote,
        );
        assert_eq!(mgr.on_mouse_move(960, 540), SwitchAction::None);
        assert!(!mgr.is_controlling_remote());
    }

    #[test]
    fn test_switch_manager_switch_to_remote_right_edge() {
        // 本机在左，远端在右 → 鼠标到右边缘切换
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalLeftOfRemote,
        );
        let action = mgr.on_mouse_move(1919, 540);
        assert_eq!(action, SwitchAction::SwitchToRemote {
            remote_entry_edge: ScreenEdge::Left,
            pos_ratio: 5000,
        });
        assert!(mgr.is_controlling_remote());
    }

    #[test]
    fn test_switch_manager_no_switch_wrong_edge() {
        // 本机在左，远端在右 → 鼠标到左边缘不切换
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalLeftOfRemote,
        );
        assert_eq!(mgr.on_mouse_move(0, 540), SwitchAction::None);
        assert!(!mgr.is_controlling_remote());
    }

    #[test]
    fn test_switch_manager_return_from_remote() {
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalLeftOfRemote,
        );
        // 切换到远端
        mgr.on_mouse_move(1919, 540);
        assert!(mgr.is_controlling_remote());

        // 返回本机
        mgr.on_return_from_remote();
        assert!(!mgr.is_controlling_remote());
    }

    #[test]
    fn test_switch_manager_no_switch_when_controlling_remote() {
        // 已在控制远端时，不再检测本机边缘
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalLeftOfRemote,
        );
        mgr.on_mouse_move(1919, 540); // 切换到远端

        // 即使鼠标在边缘，也不再触发切换
        assert_eq!(mgr.on_mouse_move(1919, 540), SwitchAction::None);
    }

    #[test]
    fn test_switch_manager_above_remote() {
        // 本机在上，远端在下 → 鼠标到下边缘切换
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalAboveRemote,
        );
        let action = mgr.on_mouse_move(960, 1079);
        assert_eq!(action, SwitchAction::SwitchToRemote {
            remote_entry_edge: ScreenEdge::Top,
            pos_ratio: 5000,
        });
    }

    #[test]
    fn test_switch_manager_update_layout() {
        let mut mgr = SwitchManager::new(
            ScreenSize::new(1920, 1080),
            DisplayLayout::LocalLeftOfRemote,
        );
        // 更新布局
        mgr.set_layout(DisplayLayout::LocalRightOfRemote);
        assert_eq!(mgr.layout(), DisplayLayout::LocalRightOfRemote);

        // 现在左边缘触发切换
        let action = mgr.on_mouse_move(0, 540);
        assert_eq!(action, SwitchAction::SwitchToRemote {
            remote_entry_edge: ScreenEdge::Right,
            pos_ratio: 5000,
        });
    }

    // ── Windows 坐标系特定测试 ───────────────────────────────────────────

    #[test]
    fn test_detect_windows_high_dpi_screen() {
        // Windows 高 DPI 屏幕（逻辑像素 1280x800，DPI 缩放 150%）
        let detector = EdgeDetector::new(ScreenSize::new(1280, 800));
        let result = detector.detect(1279, 400);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Right,
            pos_ratio: 5000, // 400 / 800 * 10000 = 5000
        });
    }

    // ── macOS 坐标系特定测试 ─────────────────────────────────────────────

    #[test]
    fn test_detect_macos_retina_screen() {
        // macOS Retina 屏幕（逻辑像素 1440x900）
        let detector = EdgeDetector::new(ScreenSize::new(1440, 900));
        let result = detector.detect(720, 0);
        assert_eq!(result, EdgeDetectResult::Edge {
            edge: ScreenEdge::Top,
            pos_ratio: 5000, // 720 / 1440 * 10000 = 5000
        });
    }
}
