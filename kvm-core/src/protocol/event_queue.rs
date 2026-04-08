//! 事件队列与延迟处理
//!
//! 处理网络延迟和事件堆积：
//! - 丢弃超过 200ms 的过期鼠标移动事件
//! - 队列超 50 个鼠标移动事件时仅保留最新 5 个
//! - 键盘和点击事件不丢弃，按顺序执行

use std::collections::VecDeque;
use crate::protocol::KvmFrame;

/// 事件过期阈值（毫秒）
pub const EVENT_EXPIRE_MS: u64 = 200;

/// 鼠标移动事件队列最大长度
pub const MOUSE_MOVE_QUEUE_MAX: usize = 50;

/// 鼠标移动事件队列保留数量（超限时）
pub const MOUSE_MOVE_QUEUE_KEEP: usize = 5;

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// 事件队列（接收端使用）
///
/// 管理接收到的事件帧，处理过期和堆积。
pub struct EventQueue {
    /// 鼠标移动事件队列
    mouse_moves: VecDeque<KvmFrame>,
    /// 其他事件队列（键盘、点击、滚轮等，不丢弃）
    other_events: VecDeque<KvmFrame>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            mouse_moves: VecDeque::with_capacity(64),
            other_events: VecDeque::with_capacity(64),
        }
    }

    /// 将收到的帧加入队列
    pub fn push(&mut self, frame: KvmFrame) {
        match &frame {
            KvmFrame::MouseMove { timestamp_ms, .. } => {
                // 检查是否过期
                let now = now_ms();
                if now.saturating_sub(*timestamp_ms) > EVENT_EXPIRE_MS {
                    // 过期，丢弃
                    return;
                }

                self.mouse_moves.push_back(frame);

                // 队列堆积超限，仅保留最新的
                if self.mouse_moves.len() > MOUSE_MOVE_QUEUE_MAX {
                    let keep_start = self.mouse_moves.len() - MOUSE_MOVE_QUEUE_KEEP;
                    self.mouse_moves.drain(..keep_start);
                }
            }
            // 键盘、点击、滚轮事件不丢弃
            KvmFrame::KeyEvent { .. }
            | KvmFrame::MouseButton { .. }
            | KvmFrame::MouseScroll { .. } => {
                self.other_events.push_back(frame);
            }
            // 其他帧类型也放入 other_events
            _ => {
                self.other_events.push_back(frame);
            }
        }
    }

    /// 取出下一个待执行的事件
    ///
    /// 优先返回非鼠标移动事件（保证按键和点击不延迟），
    /// 然后返回鼠标移动事件。
    pub fn pop(&mut self) -> Option<KvmFrame> {
        // 优先处理键盘/点击/滚轮事件
        if let Some(frame) = self.other_events.pop_front() {
            return Some(frame);
        }
        // 然后处理鼠标移动
        self.mouse_moves.pop_front()
    }

    /// 取出所有待执行的事件
    pub fn drain_all(&mut self) -> Vec<KvmFrame> {
        let mut result = Vec::with_capacity(
            self.other_events.len() + self.mouse_moves.len(),
        );
        // 先取非鼠标移动事件
        result.extend(self.other_events.drain(..));
        // 再取鼠标移动事件
        result.extend(self.mouse_moves.drain(..));
        result
    }

    /// 队列是否为空
    pub fn is_empty(&self) -> bool {
        self.mouse_moves.is_empty() && self.other_events.is_empty()
    }

    /// 队列中的事件总数
    pub fn len(&self) -> usize {
        self.mouse_moves.len() + self.other_events.len()
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.mouse_moves.clear();
        self.other_events.clear();
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ModifierState;

    fn make_mouse_move(x: i32, y: i32, ts: u64) -> KvmFrame {
        KvmFrame::MouseMove {
            x, y,
            timestamp_ms: ts,
            modifiers: ModifierState::new(),
        }
    }

    fn make_key_event(keycode: u16, pressed: bool, ts: u64) -> KvmFrame {
        KvmFrame::KeyEvent {
            keycode, pressed,
            timestamp_ms: ts,
            modifiers: ModifierState::new(),
        }
    }

    fn make_mouse_button(button: u8, pressed: bool, ts: u64) -> KvmFrame {
        KvmFrame::MouseButton {
            button, pressed,
            timestamp_ms: ts,
            modifiers: ModifierState::new(),
        }
    }

    #[test]
    fn test_push_and_pop() {
        let mut q = EventQueue::new();
        let now = now_ms();
        q.push(make_mouse_move(100, 200, now));
        q.push(make_key_event(0x41, true, now));

        // 键盘事件优先
        let first = q.pop().unwrap();
        assert!(matches!(first, KvmFrame::KeyEvent { .. }));

        let second = q.pop().unwrap();
        assert!(matches!(second, KvmFrame::MouseMove { .. }));

        assert!(q.is_empty());
    }

    #[test]
    fn test_expired_mouse_move_dropped() {
        let mut q = EventQueue::new();
        let old_ts = now_ms() - 300; // 300ms 前，超过 200ms 阈值
        q.push(make_mouse_move(100, 200, old_ts));
        assert!(q.is_empty()); // 过期事件被丢弃
    }

    #[test]
    fn test_fresh_mouse_move_kept() {
        let mut q = EventQueue::new();
        let now = now_ms();
        q.push(make_mouse_move(100, 200, now));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_mouse_move_queue_overflow() {
        let mut q = EventQueue::new();
        let now = now_ms();

        // 推入 60 个鼠标移动事件（超过 50 的限制）
        for i in 0..60 {
            q.push(make_mouse_move(i, i, now));
        }

        // 第 51 个推入时触发裁剪到 5 个，之后又推入 9 个 = 14 个
        // 但不超过 50，所以最终有 14 个
        // 实际上：推入 51 个时 len=51>50，裁剪到 5 个，然后继续推入 52-60 共 9 个 = 14 个
        assert!(q.mouse_moves.len() <= MOUSE_MOVE_QUEUE_MAX);
        assert!(q.mouse_moves.len() > 0);

        // 验证最后一个是 x=59
        let all = q.drain_all();
        if let KvmFrame::MouseMove { x, .. } = all.last().unwrap() {
            assert_eq!(*x, 59);
        }
    }

    #[test]
    fn test_key_events_never_dropped() {
        let mut q = EventQueue::new();
        let old_ts = now_ms() - 300; // 过期时间戳

        // 键盘事件即使时间戳过期也不丢弃
        q.push(make_key_event(0x41, true, old_ts));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_mouse_button_never_dropped() {
        let mut q = EventQueue::new();
        let old_ts = now_ms() - 300;

        q.push(make_mouse_button(0, true, old_ts));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_drain_all() {
        let mut q = EventQueue::new();
        let now = now_ms();

        q.push(make_mouse_move(1, 1, now));
        q.push(make_key_event(0x41, true, now));
        q.push(make_mouse_move(2, 2, now));

        let all = q.drain_all();
        assert_eq!(all.len(), 3);
        // 键盘事件在前
        assert!(matches!(all[0], KvmFrame::KeyEvent { .. }));
        assert!(q.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut q = EventQueue::new();
        let now = now_ms();
        q.push(make_mouse_move(1, 1, now));
        q.push(make_key_event(0x41, true, now));
        q.clear();
        assert!(q.is_empty());
    }
}
