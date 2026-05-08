//! 控制权状态机
//!
//! 维护四种状态：IDLE、ARMED、LOCAL_CONTROLLING、REMOTE_CONTROLLING。
//! Armed 为中间态：边缘检测已激活但尚未吞键鼠。
//! 状态切换时通知对端同步，连接断开时自动回到 IDLE。

use crate::protocol::{KvmFrame, ModifierState, ScreenEdge};

/// 控制权状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlState {
    /// 双方各自独立，互不干扰
    Idle,
    /// 已激活边缘检测，待撞边切换（不吞键鼠、不隐藏光标）
    Armed,
    /// 本机键鼠正在控制对方
    LocalControlling,
    /// 对方键鼠正在控制本机
    RemoteControlling,
}

impl Default for ControlState {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for ControlState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "各自独立"),
            Self::Armed => write!(f, "已激活·待边缘切换"),
            Self::LocalControlling => write!(f, "正在控制对方"),
            Self::RemoteControlling => write!(f, "对方正在控制本机"),
        }
    }
}

/// 状态切换事件（由状态机产生，通知上层处理）
#[derive(Debug, Clone)]
pub enum StateEvent {
    /// 状态已切换
    StateChanged {
        from: ControlState,
        to: ControlState,
    },
    /// 需要发送帧给对端
    SendFrame(KvmFrame),
    /// 需要开始采集键鼠事件
    StartCapture,
    /// 需要停止采集键鼠事件
    StopCapture,
    /// 需要屏蔽本地键鼠
    BlockInput,
    /// 需要恢复本地键鼠
    UnblockInput,
    /// 需要隐藏光标
    HideCursor,
    /// 需要显示光标
    ShowCursor,
    /// 需要释放所有修饰键
    ReleaseModifiers(ModifierState),
}

/// 控制权状态机
pub struct StateMachine {
    /// 当前状态
    state: ControlState,
    /// 当前修饰键状态
    modifiers: ModifierState,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: ControlState::Idle,
            modifiers: ModifierState::new(),
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> ControlState {
        self.state
    }

    /// 获取当前修饰键状态
    pub fn modifiers(&self) -> &ModifierState {
        &self.modifiers
    }

    /// 更新修饰键状态
    pub fn update_modifier(&mut self, modifier: u8, pressed: bool) {
        self.modifiers.set(modifier, pressed);
    }

    // ─── 新 4 状态机转换 ─────────────────────────────────────────────

    /// Idle → Armed：激活边缘检测，但不吞键鼠、不隐藏光标
    pub fn arm(&mut self) -> Vec<StateEvent> {
        if self.state != ControlState::Idle {
            return vec![];
        }
        let from = self.state;
        self.state = ControlState::Armed;
        vec![StateEvent::StateChanged { from, to: self.state }]
    }

    /// Armed → Idle：取消激活（点击"停止共享"）
    pub fn disarm(&mut self) -> Vec<StateEvent> {
        if self.state != ControlState::Armed {
            return vec![];
        }
        let from = self.state;
        self.state = ControlState::Idle;
        vec![StateEvent::StateChanged { from, to: self.state }]
    }

    /// Armed → LocalControlling：边缘过冲量满足触发条件
    pub fn armed_to_local_controlling(&mut self, edge: ScreenEdge, pos_ratio: u16) -> Vec<StateEvent> {
        if self.state != ControlState::Armed {
            return vec![];
        }
        let from = self.state;
        self.state = ControlState::LocalControlling;
        vec![
            StateEvent::StateChanged { from, to: self.state },
            StateEvent::SendFrame(KvmFrame::SwitchControl { edge, pos_ratio }),
            StateEvent::StartCapture,
            StateEvent::HideCursor,
        ]
    }

    /// LocalControlling → Armed：快捷键回切或收到 SwitchBackControl
    /// 不回 Idle，Armed 继续监听下次撞边
    pub fn local_to_armed(&mut self) -> Vec<StateEvent> {
        if self.state != ControlState::LocalControlling {
            return vec![];
        }
        let from = self.state;
        self.state = ControlState::Armed;

        let mut events = vec![
            StateEvent::StateChanged { from, to: self.state },
        ];
        // 释放修饰键
        if self.modifiers.bits() != 0 {
            events.push(StateEvent::ReleaseModifiers(self.modifiers));
            self.modifiers.clear();
        }
        events.push(StateEvent::StopCapture);
        events.push(StateEvent::ShowCursor);
        events
    }

    // ─── 兼容旧路径（保留供渐进迁移使用）─────────────────────────────

    /// 触发切换到 LOCAL_CONTROLLING（从 Idle 或 Armed 触发）
    /// 返回需要执行的事件列表
    pub fn switch_to_local_controlling(&mut self, edge: ScreenEdge, pos_ratio: u16) -> Vec<StateEvent> {
        match self.state {
            ControlState::Idle => {
                let from = self.state;
                self.state = ControlState::LocalControlling;
                vec![
                    StateEvent::StateChanged { from, to: self.state },
                    StateEvent::SendFrame(KvmFrame::SwitchControl { edge, pos_ratio }),
                    StateEvent::StartCapture,
                    StateEvent::HideCursor,
                ]
            }
            ControlState::Armed => self.armed_to_local_controlling(edge, pos_ratio),
            _ => vec![],
        }
    }

    /// 触发切换到 REMOTE_CONTROLLING（收到对端的 SwitchControl 帧）
    pub fn switch_to_remote_controlling(&mut self) -> Vec<StateEvent> {
        if self.state != ControlState::Idle {
            return vec![];
        }

        let from = self.state;
        self.state = ControlState::RemoteControlling;

        vec![
            StateEvent::StateChanged { from, to: self.state },
            StateEvent::SendFrame(KvmFrame::SwitchControlAck),
            StateEvent::BlockInput,
        ]
    }

    /// 切换回 IDLE（连接断开、主动停止共享等）
    pub fn switch_to_idle(&mut self) -> Vec<StateEvent> {
        let from = self.state;
        if from == ControlState::Idle {
            return vec![];
        }

        self.state = ControlState::Idle;

        let mut events = vec![
            StateEvent::StateChanged { from, to: self.state },
        ];

        match from {
            ControlState::Armed => {
                // Armed → Idle：无需释放资源，边缘检测由上层关闭
            }
            ControlState::LocalControlling => {
                // 释放所有修饰键
                if self.modifiers.bits() != 0 {
                    events.push(StateEvent::ReleaseModifiers(self.modifiers));
                    self.modifiers.clear();
                }
                events.push(StateEvent::StopCapture);
                events.push(StateEvent::ShowCursor);
            }
            ControlState::RemoteControlling => {
                // 清除修饰键状态
                self.modifiers.clear();
                events.push(StateEvent::UnblockInput);
            }
            ControlState::Idle => {}
        }

        events
    }

    /// 连接断开时强制回到 IDLE
    pub fn on_disconnected(&mut self) -> Vec<StateEvent> {
        self.switch_to_idle()
    }

    /// 快捷键触发切换（v0.1.40 语义：Idle↔Armed，LocalControlling→Armed）
    pub fn toggle_control(&mut self, _edge: ScreenEdge, _pos_ratio: u16) -> Vec<StateEvent> {
        match self.state {
            ControlState::Idle => self.arm(),
            ControlState::Armed => self.disarm(),
            ControlState::LocalControlling => self.local_to_armed(),
            ControlState::RemoteControlling => vec![], // 被控时不响应快捷键切换
        }
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_idle() {
        let sm = StateMachine::new();
        assert_eq!(sm.state(), ControlState::Idle);
    }

    // ─── Armed 状态转换测试 ───────────────────────────────────────

    #[test]
    fn test_arm_from_idle() {
        let mut sm = StateMachine::new();
        let events = sm.arm();
        assert_eq!(sm.state(), ControlState::Armed);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StateEvent::StateChanged { from: ControlState::Idle, to: ControlState::Armed }));
    }

    #[test]
    fn test_arm_from_non_idle_fails() {
        let mut sm = StateMachine::new();
        sm.arm();
        // Armed 状态下再 arm 无效
        let events = sm.arm();
        assert!(events.is_empty());
        assert_eq!(sm.state(), ControlState::Armed);
    }

    #[test]
    fn test_disarm_from_armed() {
        let mut sm = StateMachine::new();
        sm.arm();
        let events = sm.disarm();
        assert_eq!(sm.state(), ControlState::Idle);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StateEvent::StateChanged { from: ControlState::Armed, to: ControlState::Idle }));
    }

    #[test]
    fn test_disarm_from_non_armed_fails() {
        let mut sm = StateMachine::new();
        let events = sm.disarm();
        assert!(events.is_empty());
        assert_eq!(sm.state(), ControlState::Idle);
    }

    #[test]
    fn test_armed_to_local_controlling() {
        let mut sm = StateMachine::new();
        sm.arm();
        let events = sm.armed_to_local_controlling(ScreenEdge::Right, 3724);
        assert_eq!(sm.state(), ControlState::LocalControlling);
        assert!(events.len() >= 4);
        // 检查事件类型
        assert!(matches!(&events[0], StateEvent::StateChanged { from: ControlState::Armed, to: ControlState::LocalControlling }));
        assert!(matches!(&events[1], StateEvent::SendFrame(KvmFrame::SwitchControl { edge: ScreenEdge::Right, pos_ratio: 3724 })));
        assert!(matches!(&events[2], StateEvent::StartCapture));
        assert!(matches!(&events[3], StateEvent::HideCursor));
    }

    #[test]
    fn test_armed_to_local_controlling_from_non_armed_fails() {
        let mut sm = StateMachine::new();
        // 从 Idle 直接调用 armed_to_local_controlling 应该失败
        let events = sm.armed_to_local_controlling(ScreenEdge::Right, 5000);
        assert!(events.is_empty());
        assert_eq!(sm.state(), ControlState::Idle);
    }

    #[test]
    fn test_local_to_armed() {
        let mut sm = StateMachine::new();
        sm.arm();
        sm.armed_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.local_to_armed();
        assert_eq!(sm.state(), ControlState::Armed);
        // 应包含：StateChanged, StopCapture, ShowCursor
        assert!(events.iter().any(|e| matches!(e, StateEvent::StateChanged { from: ControlState::LocalControlling, to: ControlState::Armed })));
        assert!(events.iter().any(|e| matches!(e, StateEvent::StopCapture)));
        assert!(events.iter().any(|e| matches!(e, StateEvent::ShowCursor)));
    }

    #[test]
    fn test_local_to_armed_releases_modifiers() {
        let mut sm = StateMachine::new();
        sm.arm();
        sm.armed_to_local_controlling(ScreenEdge::Right, 5000);
        sm.update_modifier(ModifierState::CTRL, true);
        sm.update_modifier(ModifierState::ALT, true);
        let events = sm.local_to_armed();
        assert_eq!(sm.state(), ControlState::Armed);
        assert!(events.iter().any(|e| matches!(e, StateEvent::ReleaseModifiers(_))));
    }

    #[test]
    fn test_local_to_armed_from_non_local_fails() {
        let mut sm = StateMachine::new();
        let events = sm.local_to_armed();
        assert!(events.is_empty());
        assert_eq!(sm.state(), ControlState::Idle);
    }

    // ─── 完整状态转换循环测试 ───────────────────────────────────────

    #[test]
    fn test_full_cycle_arm_trigger_return_disarm() {
        let mut sm = StateMachine::new();
        // Idle → Armed
        sm.arm();
        assert_eq!(sm.state(), ControlState::Armed);
        // Armed → LocalControlling (边缘触发)
        sm.armed_to_local_controlling(ScreenEdge::Right, 7000);
        assert_eq!(sm.state(), ControlState::LocalControlling);
        // LocalControlling → Armed (快捷键回切)
        sm.local_to_armed();
        assert_eq!(sm.state(), ControlState::Armed);
        // Armed → LocalControlling (再次撞边)
        sm.armed_to_local_controlling(ScreenEdge::Right, 2000);
        assert_eq!(sm.state(), ControlState::LocalControlling);
        // LocalControlling → Armed (远端回切)
        sm.local_to_armed();
        assert_eq!(sm.state(), ControlState::Armed);
        // Armed → Idle (用户点停止共享)
        sm.disarm();
        assert_eq!(sm.state(), ControlState::Idle);
    }

    // ─── 旧路径兼容测试 ───────────────────────────────────────────

    #[test]
    fn test_switch_to_local_controlling_from_idle() {
        let mut sm = StateMachine::new();
        let events = sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::LocalControlling);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_switch_to_local_controlling_from_armed() {
        let mut sm = StateMachine::new();
        sm.arm();
        let events = sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::LocalControlling);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_switch_to_remote_controlling() {
        let mut sm = StateMachine::new();
        let events = sm.switch_to_remote_controlling();
        assert_eq!(sm.state(), ControlState::RemoteControlling);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_switch_back_to_idle_from_local() {
        let mut sm = StateMachine::new();
        sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.switch_to_idle();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_switch_back_to_idle_from_armed() {
        let mut sm = StateMachine::new();
        sm.arm();
        let events = sm.switch_to_idle();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_cannot_switch_to_local_from_local() {
        let mut sm = StateMachine::new();
        sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        assert!(events.is_empty());
    }

    #[test]
    fn test_on_disconnected_from_armed() {
        let mut sm = StateMachine::new();
        sm.arm();
        let events = sm.on_disconnected();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_on_disconnected_from_local_controlling() {
        let mut sm = StateMachine::new();
        sm.arm();
        sm.armed_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.on_disconnected();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_on_disconnected_from_remote_controlling() {
        let mut sm = StateMachine::new();
        sm.switch_to_remote_controlling();
        let events = sm.on_disconnected();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_toggle_control_new_semantics() {
        let mut sm = StateMachine::new();
        // Idle → Armed (不再直接到 LocalControlling)
        let events = sm.toggle_control(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::Armed);
        assert!(!events.is_empty());

        // Armed → Idle (取消激活)
        let events = sm.toggle_control(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_toggle_control_from_local_goes_to_armed() {
        let mut sm = StateMachine::new();
        sm.arm();
        sm.armed_to_local_controlling(ScreenEdge::Right, 5000);
        // LocalControlling → Armed (快捷键)
        let events = sm.toggle_control(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::Armed);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_toggle_control_from_remote_no_op() {
        let mut sm = StateMachine::new();
        sm.switch_to_remote_controlling();
        let events = sm.toggle_control(ScreenEdge::Right, 5000);
        assert!(events.is_empty());
        assert_eq!(sm.state(), ControlState::RemoteControlling);
    }

    #[test]
    fn test_modifier_tracking() {
        let mut sm = StateMachine::new();
        sm.update_modifier(ModifierState::CTRL, true);
        assert!(sm.modifiers().is_set(ModifierState::CTRL));

        sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.switch_to_idle();
        assert!(events.iter().any(|e| matches!(e, StateEvent::ReleaseModifiers(_))));
    }

    #[test]
    fn test_control_state_display() {
        assert_eq!(format!("{}", ControlState::Idle), "各自独立");
        assert_eq!(format!("{}", ControlState::Armed), "已激活·待边缘切换");
        assert_eq!(format!("{}", ControlState::LocalControlling), "正在控制对方");
        assert_eq!(format!("{}", ControlState::RemoteControlling), "对方正在控制本机");
    }
}
