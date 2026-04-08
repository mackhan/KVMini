//! 光标跨屏坐标映射
//!
//! 控制权切换时的光标位置映射（Y/X 坐标按比例转换到对方屏幕）。
//! 详细实现将在任务 5 中完成。

use crate::platform::VirtualDesktop;
use crate::protocol::ScreenEdge;

/// 光标映射器
pub struct CursorMapper;

impl CursorMapper {
    /// 将本机触发边缘的坐标按比例映射到对方屏幕的对应坐标
    ///
    /// - 左右布局：映射 Y 坐标
    /// - 上下布局：映射 X 坐标
    ///
    /// pos_ratio: 沿边缘方向的位置比例 [0, 10000]
    /// edge: 触发边缘方向
    /// remote_desktop: 对方的虚拟桌面信息
    ///
    /// 返回对方屏幕上的 (x, y) 绝对坐标
    pub fn map_to_remote(
        pos_ratio: u16,
        edge: ScreenEdge,
        remote_desktop: &VirtualDesktop,
    ) -> (i32, i32) {
        let ratio = pos_ratio as i64;
        let rx = remote_desktop.origin_x;
        let ry = remote_desktop.origin_y;
        let rw = remote_desktop.total_width;
        let rh = remote_desktop.total_height;

        match edge {
            ScreenEdge::Right => {
                // 本机右边缘 → 对方左边缘入口
                let y = (ratio * rh as i64 / 10000) as i32 + ry;
                (rx, y)
            }
            ScreenEdge::Left => {
                // 本机左边缘 → 对方右边缘入口
                let y = (ratio * rh as i64 / 10000) as i32 + ry;
                (rx + rw - 1, y)
            }
            ScreenEdge::Bottom => {
                // 本机下边缘 → 对方上边缘入口
                let x = (ratio * rw as i64 / 10000) as i32 + rx;
                (x, ry)
            }
            ScreenEdge::Top => {
                // 本机上边缘 → 对方下边缘入口
                let x = (ratio * rw as i64 / 10000) as i32 + rx;
                (x, ry + rh - 1)
            }
        }
    }

    /// 将对方屏幕的当前坐标按比例映射回本机屏幕
    ///
    /// remote_x, remote_y: 对方屏幕上的当前坐标
    /// edge: 原始触发边缘方向
    /// local_desktop: 本机的虚拟桌面信息
    /// remote_desktop: 对方的虚拟桌面信息
    ///
    /// 返回本机屏幕上的 (x, y) 绝对坐标
    pub fn map_to_local(
        remote_x: i32,
        remote_y: i32,
        edge: ScreenEdge,
        local_desktop: &VirtualDesktop,
        remote_desktop: &VirtualDesktop,
    ) -> (i32, i32) {
        let lx = local_desktop.origin_x;
        let ly = local_desktop.origin_y;
        let lw = local_desktop.total_width;
        let lh = local_desktop.total_height;
        let rw = remote_desktop.total_width;
        let rh = remote_desktop.total_height;
        let ry = remote_desktop.origin_y;
        let rx = remote_desktop.origin_x;

        match edge {
            ScreenEdge::Right => {
                // 从对方左边缘回来 → 本机右边缘
                let y_ratio = if rh > 0 {
                    ((remote_y - ry) as i64 * lh as i64 / rh as i64) as i32
                } else { 0 };
                (lx + lw - 1, ly + y_ratio)
            }
            ScreenEdge::Left => {
                // 从对方右边缘回来 → 本机左边缘
                let y_ratio = if rh > 0 {
                    ((remote_y - ry) as i64 * lh as i64 / rh as i64) as i32
                } else { 0 };
                (lx, ly + y_ratio)
            }
            ScreenEdge::Bottom => {
                // 从对方上边缘回来 → 本机下边缘
                let x_ratio = if rw > 0 {
                    ((remote_x - rx) as i64 * lw as i64 / rw as i64) as i32
                } else { 0 };
                (lx + x_ratio, ly + lh - 1)
            }
            ScreenEdge::Top => {
                // 从对方下边缘回来 → 本机上边缘
                let x_ratio = if rw > 0 {
                    ((remote_x - rx) as i64 * lw as i64 / rw as i64) as i32
                } else { 0 };
                (lx + x_ratio, ly)
            }
        }
    }
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
    fn test_map_to_remote_right_edge_center() {
        let remote = make_desktop(2560, 1440);
        let (x, y) = CursorMapper::map_to_remote(5000, ScreenEdge::Right, &remote);
        assert_eq!(x, 0); // 对方左边缘
        assert_eq!(y, 720); // 中间位置
    }

    #[test]
    fn test_map_to_remote_left_edge() {
        let remote = make_desktop(1920, 1080);
        let (x, y) = CursorMapper::map_to_remote(5000, ScreenEdge::Left, &remote);
        assert_eq!(x, 1919); // 对方右边缘
        assert_eq!(y, 540);
    }

    #[test]
    fn test_map_to_remote_bottom_edge() {
        let remote = make_desktop(1920, 1080);
        let (x, y) = CursorMapper::map_to_remote(5000, ScreenEdge::Bottom, &remote);
        assert_eq!(x, 960);
        assert_eq!(y, 0); // 对方上边缘
    }

    #[test]
    fn test_map_to_local_right_edge() {
        let local = make_desktop(1920, 1080);
        let remote = make_desktop(2560, 1440);
        let (x, y) = CursorMapper::map_to_local(0, 720, ScreenEdge::Right, &local, &remote);
        assert_eq!(x, 1919); // 本机右边缘
        assert_eq!(y, 540); // 按比例映射
    }

    #[test]
    fn test_roundtrip_mapping() {
        let local = make_desktop(1920, 1080);
        let remote = make_desktop(1920, 1080);

        // 从本机右边缘中间出发
        let (rx, ry) = CursorMapper::map_to_remote(5000, ScreenEdge::Right, &remote);
        // 从对方映射回来
        let (lx, ly) = CursorMapper::map_to_local(rx, ry, ScreenEdge::Right, &local, &remote);
        assert_eq!(lx, 1919);
        assert_eq!(ly, 540);
    }
}
