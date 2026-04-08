/// Windows 端屏幕边缘检测子模块
///
/// 支持上下左右四个方向的边缘检测，支持多显示器虚拟桌面。
/// 与 kvm-agent 的 edge 模块共享相同的逻辑，但使用 Windows 坐标系。
///
/// ## 多显示器支持
/// - 使用虚拟桌面坐标系（所有显示器的包围盒）
/// - 支持跨显示器边缘检测
/// - 与 monitor.rs 集成，自动更新显示器布局

use serde::{Serialize, Deserialize};
use crate::monitor::{MonitorInfo, VirtualDesktop};

/// 边缘方向（支持上下左右）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeDirection {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeDirection {
    pub fn all() -> [EdgeDirection; 4] {
        [Self::Left, Self::Right, Self::Top, Self::Bottom]
    }

    /// 对立方向（用于对端入口边缘）
    pub fn opposite(&self) -> EdgeDirection {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }

    /// 转换为协议层 ScreenEdge
    pub fn to_screen_edge(&self) -> crate::protocol::ScreenEdge {
        match self {
            Self::Left => crate::protocol::ScreenEdge::Left,
            Self::Right => crate::protocol::ScreenEdge::Right,
            Self::Top => crate::protocol::ScreenEdge::Top,
            Self::Bottom => crate::protocol::ScreenEdge::Bottom,
        }
    }
}

/// 边缘检测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    /// 触发切换的边缘方向列表
    pub trigger_edges: Vec<EdgeDirection>,
    /// 边缘触发阈值（像素），默认 1
    pub threshold: i32,
    /// 防抖延迟（毫秒）
    pub debounce_ms: u64,
    /// 是否启用多显示器边缘检测
    pub multi_monitor: bool,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            trigger_edges: vec![EdgeDirection::Right], // Windows 默认右边缘切换到 Mac
            threshold: 1,
            debounce_ms: 50,
            multi_monitor: true,
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
        direction: EdgeDirection,
        /// 沿边缘方向的位置比例（0-10000）
        pos_ratio: u16,
        /// 触发时所在的显示器 ID
        monitor_id: u32,
    },
}

/// Windows 端屏幕边缘控制器
///
/// 与 monitor.rs 集成，支持多显示器动态更新。
pub struct WinEdgeController {
    /// 虚拟桌面（从 monitor.rs 获取）
    desktop: VirtualDesktop,
    /// 边缘检测配置
    config: EdgeConfig,
    /// 上次触发时间（防抖）
    last_trigger: Option<std::time::Instant>,
    /// 当前是否正在控制远端
    controlling_remote: bool,
}

impl WinEdgeController {
    /// 创建边缘控制器（自动获取当前显示器信息）
    pub fn new() -> Self {
        let desktop = crate::monitor::get_virtual_desktop();
        Self {
            desktop,
            config: EdgeConfig::default(),
            last_trigger: None,
            controlling_remote: false,
        }
    }

    /// 设置边缘检测配置
    pub fn set_config(&mut self, config: EdgeConfig) {
        self.config = config;
    }

    /// 更新显示器布局（显示器变化时调用）
    pub fn update_monitors(&mut self) {
        self.desktop = crate::monitor::get_virtual_desktop();
    }

    /// 获取虚拟桌面信息
    pub fn desktop(&self) -> &VirtualDesktop {
        &self.desktop
    }

    /// 是否正在控制远端
    pub fn is_controlling_remote(&self) -> bool {
        self.controlling_remote
    }

    /// 设置控制远端状态
    pub fn set_controlling_remote(&mut self, v: bool) {
        self.controlling_remote = v;
    }

    /// 检测鼠标位置是否触发边缘切换
    /// x, y 为虚拟桌面绝对坐标（Windows 逻辑像素）
    pub fn detect(&mut self, x: i32, y: i32) -> EdgeDetectResult {
        // 已在控制远端时不检测本机边缘
        if self.controlling_remote {
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

        let monitor_id = self.find_monitor(x, y);

        for &dir in &self.config.trigger_edges {
            let result = match dir {
                EdgeDirection::Left if x <= ox + t - 1 => {
                    Some((dir, self.calc_ratio(y - oy, h)))
                }
                EdgeDirection::Right if x >= ox + w - t => {
                    Some((dir, self.calc_ratio(y - oy, h)))
                }
                EdgeDirection::Top if y <= oy + t - 1 => {
                    Some((dir, self.calc_ratio(x - ox, w)))
                }
                EdgeDirection::Bottom if y >= oy + h - t => {
                    Some((dir, self.calc_ratio(x - ox, w)))
                }
                _ => None,
            };

            if let Some((direction, pos_ratio)) = result {
                self.last_trigger = Some(std::time::Instant::now());
                self.controlling_remote = true;
                return EdgeDetectResult::Triggered { direction, pos_ratio, monitor_id };
            }
        }

        EdgeDetectResult::None
    }

    fn calc_ratio(&self, pos: i32, total: i32) -> u16 {
        if total <= 0 { return 0; }
        ((pos as i64 * 10000 / total as i64).clamp(0, 10000)) as u16
    }

    fn find_monitor(&self, x: i32, y: i32) -> u32 {
        self.desktop.monitors.iter()
            .find(|m| {
                x >= m.offset_x && x < m.offset_x + m.width
                    && y >= m.offset_y && y < m.offset_y + m.height
            })
            .map(|m| m.id)
            .unwrap_or(0)
    }
}

impl Default for WinEdgeController {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：获取边缘检测配置
#[tauri::command]
pub fn get_edge_config() -> EdgeConfig {
    EdgeConfig::default()
}

/// Tauri 命令：设置边缘检测配置
#[tauri::command]
pub fn set_edge_config(config: EdgeConfig) -> Result<(), String> {
    // 配置验证
    if config.trigger_edges.is_empty() {
        return Err("至少需要配置一个触发边缘".to_string());
    }
    if config.threshold < 1 || config.threshold > 10 {
        return Err("边缘阈值应在 1-10 像素之间".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_desktop(w: i32, h: i32) -> VirtualDesktop {
        VirtualDesktop {
            total_width: w,
            total_height: h,
            origin_x: 0,
            origin_y: 0,
            monitors: vec![MonitorInfo {
                id: 0, width: w, height: h, dpi_scale: 1.0,
                offset_x: 0, offset_y: 0, is_primary: true,
            }],
        }
    }

    #[test]
    fn test_edge_direction_opposite() {
        assert_eq!(EdgeDirection::Left.opposite(), EdgeDirection::Right);
        assert_eq!(EdgeDirection::Top.opposite(), EdgeDirection::Bottom);
    }

    #[test]
    fn test_edge_config_default() {
        let cfg = EdgeConfig::default();
        assert_eq!(cfg.trigger_edges, vec![EdgeDirection::Right]);
        assert_eq!(cfg.threshold, 1);
    }
}
