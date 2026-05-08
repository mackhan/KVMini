//! 远端虚拟光标状态机（v0.1.40 delta 模式）
//!
//! 在 `RemoteControlling` 态下维护"对方鼠标在本机屏幕上的位置"。
//! 核心职责：
//! - 入态时根据 `source_edge + pos_ratio` 计算初始落点
//! - 接收 `MouseMoveDelta{dx,dy}` 累加位置 + clamp 屏幕范围
//! - 持续贴**反向边缘**（= source_edge 的 opposite，即入屏那一侧的对面？不，是入屏那一侧本身，用户动作直觉：从哪进就往哪退）
//!
//! 回切语义详解：
//! - 本机撞 `Right` → 发 `SwitchControl{edge: Right}` → 远端入屏位置在远端**左边缘**（Right.opposite() = Left）
//! - 用户想回本机 → 继续往左推 → 远端光标持续贴 **Left 边**
//! - 所以 `return_edge = Left = source_edge.opposite()`（与"入屏那一侧"相同）
//!
//! 这与 `EdgeDetector.detect_return_edge` 的命名一致（trigger_edge.opposite）。

use crate::cursor_mapping::CursorMapper;
use crate::platform::VirtualDesktop;
use crate::protocol::ScreenEdge;

/// 远端虚拟光标状态
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteCursorState {
    /// 当前虚拟光标在本机屏幕上的绝对坐标（逻辑像素）
    pub x: i32,
    pub y: i32,
    /// 本机屏幕原点与尺寸
    pub origin_x: i32,
    pub origin_y: i32,
    pub screen_w: i32,
    pub screen_h: i32,
    /// 反向边缘（连续贴此边达阈值 → SwitchBack）
    pub return_edge: ScreenEdge,
    /// 贴反向边缘的连续计数（复用 EdgeDetector 风格）
    pub edge_hit_count: i32,
    /// 触发阈值（持续贴边次数，复用 overshoot_px 配置）
    pub return_threshold: i32,
}

/// 远端虚拟光标动作
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteCursorAction {
    /// 落点到 (x, y)
    Warp { x: i32, y: i32 },
    /// 触发回切（贴反向边缘持续达阈值）
    SwitchBack,
}

impl RemoteCursorState {
    /// 入态：根据对方发来的 source_edge + pos_ratio 计算本机入屏落点
    ///
    /// source_edge: 对方**出屏**的边缘（也就是对方 EdgeDetector 触发的那个边）
    /// pos_ratio: 沿 source_edge 方向的位置比例 [0, 10000]
    /// local_desktop: 本机虚拟桌面（本机 = 被控方）
    /// return_threshold: 贴反向边缘达到此次数即触发回切（建议与 overshoot_px 一致，默认 20）
    pub fn on_enter(
        source_edge: ScreenEdge,
        pos_ratio: u16,
        local_desktop: &VirtualDesktop,
        return_threshold: i32,
    ) -> Self {
        let (x, y) = CursorMapper::map_to_remote(pos_ratio, source_edge, local_desktop);
        Self {
            x,
            y,
            origin_x: local_desktop.origin_x,
            origin_y: local_desktop.origin_y,
            screen_w: local_desktop.total_width,
            screen_h: local_desktop.total_height,
            return_edge: source_edge.opposite(),
            edge_hit_count: 0,
            return_threshold: return_threshold.max(1),
        }
    }

    /// 累加 delta，检测反向边缘，返回动作
    pub fn on_delta(&mut self, dx: i16, dy: i16) -> RemoteCursorAction {
        // 累加并 clamp 到屏幕范围（i64 防止 i32 溢出）
        let new_x = (self.x as i64 + dx as i64)
            .clamp(self.origin_x as i64, (self.origin_x + self.screen_w - 1) as i64)
            as i32;
        let new_y = (self.y as i64 + dy as i64)
            .clamp(self.origin_y as i64, (self.origin_y + self.screen_h - 1) as i64)
            as i32;

        self.x = new_x;
        self.y = new_y;

        // 检测是否持续贴反向边缘
        let at_return_edge = match self.return_edge {
            ScreenEdge::Left => new_x <= self.origin_x,
            ScreenEdge::Right => new_x >= self.origin_x + self.screen_w - 1,
            ScreenEdge::Top => new_y <= self.origin_y,
            ScreenEdge::Bottom => new_y >= self.origin_y + self.screen_h - 1,
        };

        if at_return_edge {
            self.edge_hit_count += 1;
            if self.edge_hit_count >= self.return_threshold {
                return RemoteCursorAction::SwitchBack;
            }
        } else {
            self.edge_hit_count = 0;
        }

        RemoteCursorAction::Warp { x: new_x, y: new_y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::MonitorInfo;

    fn make_desktop(w: i32, h: i32) -> VirtualDesktop {
        VirtualDesktop::from_monitors(vec![MonitorInfo {
            id: 0,
            width: w,
            height: h,
            dpi_scale: 1.0,
            offset_x: 0,
            offset_y: 0,
            is_primary: true,
        }])
    }

    // ── on_enter 初始化 ─────────────────────────────────────────────────

    #[test]
    fn test_on_enter_from_right_edge_center() {
        // 对方撞右边缘 pos_ratio=5000（中间）→ 本机（被控方）光标应落在左边缘中间
        let desktop = make_desktop(1920, 1080);
        let state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 20);
        assert_eq!(state.x, 0, "远端入屏 x 应是本机左边缘 0");
        assert_eq!(state.y, 540, "pos_ratio=5000 → y=540 (1080/2)");
        assert_eq!(state.return_edge, ScreenEdge::Left, "反向边缘 = source.opposite()");
    }

    #[test]
    fn test_on_enter_from_left_edge() {
        // 对方撞左边缘 → 本机光标落在右边缘
        let desktop = make_desktop(1920, 1080);
        let state = RemoteCursorState::on_enter(ScreenEdge::Left, 2500, &desktop, 20);
        assert_eq!(state.x, 1919);
        assert_eq!(state.y, 270);
        assert_eq!(state.return_edge, ScreenEdge::Right);
    }

    #[test]
    fn test_on_enter_from_bottom_edge() {
        // 对方撞下边缘 → 本机光标落在上边缘
        let desktop = make_desktop(1920, 1080);
        let state = RemoteCursorState::on_enter(ScreenEdge::Bottom, 7500, &desktop, 20);
        assert_eq!(state.x, 1440);
        assert_eq!(state.y, 0);
        assert_eq!(state.return_edge, ScreenEdge::Top);
    }

    #[test]
    fn test_on_enter_threshold_min_1() {
        // return_threshold = 0 应被提升到 1（避免除零/立即触发 bug）
        let desktop = make_desktop(1920, 1080);
        let state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 0);
        assert_eq!(state.return_threshold, 1);
    }

    // ── on_delta 累加 ──────────────────────────────────────────────────

    #[test]
    fn test_on_delta_simple_accumulation() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 20);
        // 初始 (0, 540)
        let action = state.on_delta(10, 5);
        assert_eq!(action, RemoteCursorAction::Warp { x: 10, y: 545 });
        assert_eq!(state.x, 10);
        assert_eq!(state.y, 545);
    }

    #[test]
    fn test_on_delta_multiple_accumulates() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 20);
        // 初始 (0, 540)
        state.on_delta(100, 50);
        state.on_delta(50, -30);
        assert_eq!(state.x, 150);
        assert_eq!(state.y, 560);
    }

    // ── on_delta clamp 屏幕边界 ────────────────────────────────────────

    #[test]
    fn test_on_delta_clamp_right_edge() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Left, 5000, &desktop, 20);
        // 初始 (1919, 540)
        state.on_delta(1000, 0);
        // clamp 到 1919
        assert_eq!(state.x, 1919);
    }

    #[test]
    fn test_on_delta_clamp_left_edge() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 20);
        // 初始 (0, 540)
        state.on_delta(-1000, 0);
        assert_eq!(state.x, 0);
    }

    #[test]
    fn test_on_delta_clamp_top_bottom() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 20);
        state.on_delta(100, -1000);
        assert_eq!(state.y, 0);
        state.on_delta(0, 10000);
        assert_eq!(state.y, 1079);
    }

    #[test]
    fn test_on_delta_i16_extremes_no_overflow() {
        // 极端 delta 不应导致 i32 溢出
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 20);
        state.x = i32::MAX - 10;
        state.y = i32::MAX - 10;
        // i16::MAX + (i32::MAX - 10) 会溢出 → 必须通过 i64 中间结果安全处理，最终 clamp 到屏幕
        let _ = state.on_delta(i16::MAX, i16::MAX);
        // clamp 到 screen_w-1 / screen_h-1
        assert_eq!(state.x, 1919);
        assert_eq!(state.y, 1079);
    }

    // ── on_delta 反向边缘检测 ──────────────────────────────────────────

    #[test]
    fn test_on_delta_hit_return_edge_accumulates() {
        // 从 Right 进入 → return_edge = Left；光标移到 x=0 后持续 dx<0 应累加
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 3);
        // 初始 (0, 540)，已经贴 Left 边
        let a1 = state.on_delta(-1, 0);
        assert_eq!(a1, RemoteCursorAction::Warp { x: 0, y: 540 });
        assert_eq!(state.edge_hit_count, 1);

        let a2 = state.on_delta(-1, 0);
        assert_eq!(a2, RemoteCursorAction::Warp { x: 0, y: 540 });
        assert_eq!(state.edge_hit_count, 2);

        // 第 3 次贴边达到阈值 → SwitchBack
        let a3 = state.on_delta(-1, 0);
        assert_eq!(a3, RemoteCursorAction::SwitchBack);
    }

    #[test]
    fn test_on_delta_leave_return_edge_resets_counter() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 5);
        // 贴 Left 边两次
        state.on_delta(-1, 0);
        state.on_delta(-1, 0);
        assert_eq!(state.edge_hit_count, 2);

        // 离开 Left 边 → 重置
        state.on_delta(50, 0);
        assert_eq!(state.edge_hit_count, 0);
        assert_eq!(state.x, 50);
    }

    #[test]
    fn test_on_delta_return_edge_right() {
        // 从 Left 进入 → return_edge = Right
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Left, 5000, &desktop, 2);
        // 初始 (1919, 540)
        assert_eq!(state.return_edge, ScreenEdge::Right);

        state.on_delta(1, 0);
        assert_eq!(state.edge_hit_count, 1);
        let a = state.on_delta(1, 0);
        assert_eq!(a, RemoteCursorAction::SwitchBack);
    }

    #[test]
    fn test_on_delta_return_edge_bottom() {
        // 从 Top 进入 → return_edge = Bottom
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Top, 5000, &desktop, 2);
        // 初始 (x=960, y=1079)
        assert_eq!(state.return_edge, ScreenEdge::Bottom);
        assert_eq!(state.y, 1079);

        state.on_delta(0, 1);
        let a = state.on_delta(0, 1);
        assert_eq!(a, RemoteCursorAction::SwitchBack);
    }

    #[test]
    fn test_on_delta_not_at_return_edge_no_trigger() {
        let desktop = make_desktop(1920, 1080);
        let mut state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 2);
        // 初始 (0, 540) 已经贴 Left，但移开到中间
        state.on_delta(500, 0);
        assert_eq!(state.edge_hit_count, 0);
        // 在中间继续移动不应触发
        let a = state.on_delta(10, 10);
        assert!(matches!(a, RemoteCursorAction::Warp { .. }));
    }

    #[test]
    fn test_on_delta_initial_entry_does_not_count_as_return_hit() {
        // 入态时虽然已经贴在 return_edge 上（比如从 Right 进入初始 x=0 = Left 边）
        // 但 edge_hit_count 初始为 0，第一次 on_delta 才开始计数
        // 这个测试验证这一点：不会因为入态位置恰好在 return_edge 上就"立即回切"
        let desktop = make_desktop(1920, 1080);
        let state = RemoteCursorState::on_enter(ScreenEdge::Right, 5000, &desktop, 1);
        assert_eq!(state.edge_hit_count, 0);
        assert_eq!(state.x, 0); // 确实贴在 Left（return_edge）上
        // 此时如果阈值=1，下一次 on_delta 会触发（这是合理行为）
    }
}
