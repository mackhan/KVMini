//! 控制权状态机
//!
//! 维护三种状态：IDLE、LOCAL_CONTROLLING、REMOTE_CONTROLLING。
//! 状态切换时通知对端同步，连接断开时自动回到 IDLE。
//! 
//! 详细实现将在任务 3 中完成。

use crate::protocol::{KvmFrame, ModifierState, ScreenEdge};

/// 控制权状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlState {
    /// 双方各自独立，互不干扰
    Idle,
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

    /// 触发切换到 LOCAL_CONTROLLING（边缘触发或快捷键触发）
    /// 返回需要执行的事件列表
    pub fn switch_to_local_controlling(&mut self, edge: ScreenEdge, pos_ratio: u16) -> Vec<StateEvent> {
        if self.state != ControlState::Idle {
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

    /// 切换回 IDLE（边缘回切、快捷键、连接断开等）
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

    /// 快捷键触发切换（IDLE ↔ LOCAL_CONTROLLING）
    pub fn toggle_control(&mut self, edge: ScreenEdge, pos_ratio: u16) -> Vec<StateEvent> {
        match self.state {
            ControlState::Idle => self.switch_to_local_controlling(edge, pos_ratio),
            ControlState::LocalControlling => self.switch_to_idle(),
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

    #[test]
    fn test_switch_to_local_controlling() {
        let mut sm = StateMachine::new();
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
    fn test_switch_back_to_idle() {
        let mut sm = StateMachine::new();
        sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.switch_to_idle();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_cannot_switch_from_non_idle() {
        let mut sm = StateMachine::new();
        sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        // 尝试再次切换到 LOCAL_CONTROLLING 应该无效
        let events = sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        assert!(events.is_empty());
    }

    #[test]
    fn test_on_disconnected() {
        let mut sm = StateMachine::new();
        sm.switch_to_remote_controlling();
        let events = sm.on_disconnected();
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_toggle_control() {
        let mut sm = StateMachine::new();
        // IDLE → LOCAL_CONTROLLING
        let events = sm.toggle_control(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::LocalControlling);
        assert!(!events.is_empty());

        // LOCAL_CONTROLLING → IDLE
        let events = sm.toggle_control(ScreenEdge::Right, 5000);
        assert_eq!(sm.state(), ControlState::Idle);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_modifier_tracking() {
        let mut sm = StateMachine::new();
        sm.update_modifier(ModifierState::CTRL, true);
        assert!(sm.modifiers().is_set(ModifierState::CTRL));

        sm.switch_to_local_controlling(ScreenEdge::Right, 5000);
        let events = sm.switch_to_idle();
        // 应该有 ReleaseModifiers 事件
        assert!(events.iter().any(|e| matches!(e, StateEvent::ReleaseModifiers(_))));
    }

    #[test]
    fn test_control_state_display() {
        assert_eq!(format!("{}", ControlState::Idle), "各自独立");
        assert_eq!(format!("{}", ControlState::LocalControlling), "正在控制对方");
        assert_eq!(format!("{}", ControlState::RemoteControlling), "对方正在控制本机");
    }
}
