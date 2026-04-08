/// 屏幕边缘检测子模块
///
/// 支持上下左右四个方向的边缘检测，支持多显示器虚拟桌面。
/// 当鼠标到达配置的边缘时，触发控制权切换。
///
/// ## 多显示器支持
/// - 使用虚拟桌面坐标系（所有显示器的包围盒）
/// - 每个显示器的边缘都可以独立配置是否触发切换
/// - 支持跨显示器的边缘检测（如左侧显示器的左边缘）

/// 边缘方向（支持上下左右）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeDirection {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeDirection {
    /// 返回所有方向
    pub fn all() -> [EdgeDirection; 4] {
        [Self::Left, Self::Right, Self::Top, Self::Bottom]
    }

    /// 返回对立方向（用于对端入口边缘）
    pub fn opposite(&self) -> EdgeDirection {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

/// 显示器信息（用于多显示器边缘检测）
#[derive(Debug, Clone)]
pub struct MonitorRect {
    /// 显示器 ID
    pub id: u32,
    /// 相对于虚拟桌面的 X 偏移（逻辑像素）
    pub x: i32,
    /// 相对于虚拟桌面的 Y 偏移（逻辑像素）
    pub y: i32,
    /// 宽度（逻辑像素）
    pub width: i32,
    /// 高度（逻辑像素）
    pub height: i32,
    /// 是否为主显示器
    pub is_primary: bool,
}

/// 虚拟桌面（所有显示器的包围盒）
#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    /// 虚拟桌面总宽度
    pub total_width: i32,
    /// 虚拟桌面总高度
    pub total_height: i32,
    /// 虚拟桌面原点 X（多显示器时可能为负数）
    pub origin_x: i32,
    /// 虚拟桌面原点 Y
    pub origin_y: i32,
    /// 所有显示器
    pub monitors: Vec<MonitorRect>,
}

impl VirtualDesktop {
    /// 从显示器列表构建虚拟桌面
    pub fn from_monitors(monitors: Vec<MonitorRect>) -> Self {
        if monitors.is_empty() {
            return Self {
                total_width: 1920,
                total_height: 1080,
                origin_x: 0,
                origin_y: 0,
                monitors,
            };
        }
        let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
        let min_y = monitors.iter().map(|m| m.y).min().unwrap_or(0);
        let max_x = monitors.iter().map(|m| m.x + m.width).max().unwrap_or(1920);
        let max_y = monitors.iter().map(|m| m.y + m.height).max().unwrap_or(1080);
        Self {
            total_width: max_x - min_x,
            total_height: max_y - min_y,
            origin_x: min_x,
            origin_y: min_y,
            monitors,
        }
    }

    /// 获取主显示器
    pub fn primary(&self) -> Option<&MonitorRect> {
        self.monitors.iter().find(|m| m.is_primary)
            .or_else(|| self.monitors.first())
    }

    /// 将绝对坐标归一化为 [0, 10000]
    pub fn normalize(&self, x: i32, y: i32) -> (i32, i32) {
        let rx = x - self.origin_x;
        let ry = y - self.origin_y;
        let nx = (rx as i64 * 10000 / self.total_width as i64) as i32;
        let ny = (ry as i64 * 10000 / self.total_height as i64) as i32;
        (nx.clamp(0, 10000), ny.clamp(0, 10000))
    }

    /// 将归一化坐标还原为绝对坐标
    pub fn denormalize(&self, nx: i32, ny: i32) -> (i32, i32) {
        let x = (nx as i64 * self.total_width as i64 / 10000) as i32 + self.origin_x;
        let y = (ny as i64 * self.total_height as i64 / 10000) as i32 + self.origin_y;
        (x, y)
    }
}

/// 边缘检测配置
#[derive(Debug, Clone)]
pub struct EdgeConfig {
    /// 触发切换的边缘方向（可配置多个）
    pub trigger_edges: Vec<EdgeDirection>,
    /// 边缘触发阈值（像素），默认 1
    pub threshold: i32,
    /// 触发后的防抖延迟（毫秒），防止误触
    pub debounce_ms: u64,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            trigger_edges: vec![EdgeDirection::Right], // 默认右边缘触发（本机在左，远端在右）
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
    /// 到达边缘
    Triggered {
        /// 触发的边缘方向
        direction: EdgeDirection,
        /// 沿边缘方向的位置比例（0-10000）
        pos_ratio: u16,
        /// 触发时所在的显示器 ID（多显示器时有意义）
        monitor_id: u32,
    },
}

/// 屏幕边缘控制器
///
/// 支持多显示器，检测鼠标是否到达配置的屏幕边缘。
pub struct EdgeController {
    /// 虚拟桌面信息
    desktop: VirtualDesktop,
    /// 边缘检测配置
    config: EdgeConfig,
    /// 上次触发时间（防抖）
    last_trigger: Option<std::time::Instant>,
}

impl EdgeController {
    /// 创建边缘控制器（单显示器）
    pub fn new(width: i32, height: i32) -> Self {
        let monitor = MonitorRect { id: 0, x: 0, y: 0, width, height, is_primary: true };
        Self {
            desktop: VirtualDesktop::from_monitors(vec![monitor]),
            config: EdgeConfig::default(),
            last_trigger: None,
        }
    }

    /// 创建边缘控制器（多显示器）
    pub fn with_monitors(monitors: Vec<MonitorRect>) -> Self {
        Self {
            desktop: VirtualDesktop::from_monitors(monitors),
            config: EdgeConfig::default(),
            last_trigger: None,
        }
    }

    /// 设置边缘检测配置
    pub fn set_config(&mut self, config: EdgeConfig) {
        self.config = config;
    }

    /// 更新显示器布局（显示器变化时调用）
    pub fn update_monitors(&mut self, monitors: Vec<MonitorRect>) {
        self.desktop = VirtualDesktop::from_monitors(monitors);
    }

    /// 获取虚拟桌面信息
    pub fn desktop(&self) -> &VirtualDesktop {
        &self.desktop
    }

    /// 检测鼠标位置是否触发边缘切换
    /// x, y 为虚拟桌面绝对坐标
    pub fn detect(&mut self, x: i32, y: i32) -> EdgeDetectResult {
        // 防抖检测
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

        // 确定鼠标所在显示器
        let monitor_id = self.find_monitor(x, y).unwrap_or(0);

        for &dir in &self.config.trigger_edges {
            let result = match dir {
                EdgeDirection::Left if x <= ox + t - 1 => {
                    let ratio = self.calc_ratio(y - oy, h);
                    Some((dir, ratio))
                }
                EdgeDirection::Right if x >= ox + w - t => {
                    let ratio = self.calc_ratio(y - oy, h);
                    Some((dir, ratio))
                }
                EdgeDirection::Top if y <= oy + t - 1 => {
                    let ratio = self.calc_ratio(x - ox, w);
                    Some((dir, ratio))
                }
                EdgeDirection::Bottom if y >= oy + h - t => {
                    let ratio = self.calc_ratio(x - ox, w);
                    Some((dir, ratio))
                }
                _ => None,
            };

            if let Some((direction, pos_ratio)) = result {
                self.last_trigger = Some(std::time::Instant::now());
                return EdgeDetectResult::Triggered { direction, pos_ratio, monitor_id };
            }
        }

        EdgeDetectResult::None
    }

    /// 计算位置比例（0-10000）
    fn calc_ratio(&self, pos: i32, total: i32) -> u16 {
        if total <= 0 { return 0; }
        ((pos as i64 * 10000 / total as i64).clamp(0, 10000)) as u16
    }

    /// 查找鼠标所在显示器 ID
    fn find_monitor(&self, x: i32, y: i32) -> Option<u32> {
        self.desktop.monitors.iter()
            .find(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
            .map(|m| m.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_monitor_right_edge() {
        let mut ctrl = EdgeController::new(1920, 1080);
        ctrl.set_config(EdgeConfig {
            trigger_edges: vec![EdgeDirection::Right],
            threshold: 1,
            debounce_ms: 0,
        });
        let result = ctrl.detect(1919, 540);
        assert!(matches!(result, EdgeDetectResult::Triggered { direction: EdgeDirection::Right, .. }));
    }

    #[test]
    fn test_single_monitor_no_trigger_center() {
        let mut ctrl = EdgeController::new(1920, 1080);
        ctrl.set_config(EdgeConfig { debounce_ms: 0, ..Default::default() });
        assert_eq!(ctrl.detect(960, 540), EdgeDetectResult::None);
    }

    #[test]
    fn test_multi_monitor_right_edge() {
        // 两个 1920x1080 并排
        let monitors = vec![
            MonitorRect { id: 0, x: 0, y: 0, width: 1920, height: 1080, is_primary: true },
            MonitorRect { id: 1, x: 1920, y: 0, width: 1920, height: 1080, is_primary: false },
        ];
        let mut ctrl = EdgeController::with_monitors(monitors);
        ctrl.set_config(EdgeConfig {
            trigger_edges: vec![EdgeDirection::Right],
            threshold: 1,
            debounce_ms: 0,
        });
        // 右侧显示器的右边缘（x = 3839）
        let result = ctrl.detect(3839, 540);
        assert!(matches!(result, EdgeDetectResult::Triggered { direction: EdgeDirection::Right, .. }));
    }

    #[test]
    fn test_all_four_directions() {
        let mut ctrl = EdgeController::new(1920, 1080);
        ctrl.set_config(EdgeConfig {
            trigger_edges: EdgeDirection::all().to_vec(),
            threshold: 1,
            debounce_ms: 0,
        });
        assert!(matches!(ctrl.detect(0, 540), EdgeDetectResult::Triggered { direction: EdgeDirection::Left, .. }));
        ctrl.last_trigger = None;
        assert!(matches!(ctrl.detect(1919, 540), EdgeDetectResult::Triggered { direction: EdgeDirection::Right, .. }));
        ctrl.last_trigger = None;
        assert!(matches!(ctrl.detect(960, 0), EdgeDetectResult::Triggered { direction: EdgeDirection::Top, .. }));
        ctrl.last_trigger = None;
        assert!(matches!(ctrl.detect(960, 1079), EdgeDetectResult::Triggered { direction: EdgeDirection::Bottom, .. }));
    }

    #[test]
    fn test_virtual_desktop_normalize() {
        let monitors = vec![
            MonitorRect { id: 0, x: 0, y: 0, width: 1920, height: 1080, is_primary: true },
            MonitorRect { id: 1, x: 1920, y: 0, width: 1920, height: 1080, is_primary: false },
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 3840);
        let (nx, ny) = vd.normalize(1920, 540);
        assert_eq!(nx, 5000);
        assert_eq!(ny, 5000);
    }

    #[test]
    fn test_edge_direction_opposite() {
        assert_eq!(EdgeDirection::Left.opposite(), EdgeDirection::Right);
        assert_eq!(EdgeDirection::Top.opposite(), EdgeDirection::Bottom);
    }
}
