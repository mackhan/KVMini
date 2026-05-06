//! 键鼠共享模块（Bug 2 / BUG-019）
//!
//! 串起：StateMachine + InputCapture（本端 → 网络） + InputExecutor（远端 → 本机）。
//!
//! 设计要点：
//! - SharingManager 作为 Tauri 托管单例。
//! - `toggle_sharing` 命令：IDLE ↔ LocalControlling 切换：
//!     IDLE → Local：start capture，把每个 RawInputEvent 通过 mpsc 发给 sender task，
//!                   sender 转换成 KvmFrame 并通过 ConnectionManager.send_frame 发出。
//!     Local → IDLE：stop capture，发 SwitchControlAck（让对端结束 RemoteControlling）。
//! - 接收端：connection 的 start_event_loop 在收到 KvmFrame::MouseMove/Button/Scroll/KeyEvent
//!   时调用 SharingManager.inject_frame，executor 注入到本机。
//! - 接收 SwitchControl 时切换到 RemoteControlling，发出 control-state-changed 事件。
//! - 连接断开时强制回 IDLE。
//!
//! 平台隔离：MacInputCapture / MacInputExecutor / WinInputCapture / WinInputExecutor 通过 cfg 选择。
//! CGEventTap 的 RunLoop 隔离已经封装在 MacInputCapture::start_capture 内部。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use kvm_core::error::KvmResult;
use kvm_core::platform::{InputCapture, InputExecutor, MouseButton, RawInputEvent};
use kvm_core::protocol::{KvmFrame, ModifierState, ScreenEdge};
use kvm_core::state::{ControlState, StateEvent, StateMachine};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Mutex;

use crate::connection::KvmConnectionManager;

// ── 平台特定 capture / executor 实现 ────────────────────────────────────────

#[cfg(target_os = "macos")]
type PlatformCapture = crate::platform::macos::input_capture::MacInputCapture;
#[cfg(target_os = "macos")]
type PlatformExecutor = crate::platform::macos::input_inject::MacInputExecutor;

#[cfg(target_os = "windows")]
type PlatformCapture = crate::platform::windows::input_capture::WinInputCapture;
#[cfg(target_os = "windows")]
type PlatformExecutor = crate::platform::windows::input_inject::WinInputExecutor;

// ── 前端事件 payload ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlStateChange {
    pub from: String,
    pub to: String,
}

fn state_name(s: ControlState) -> &'static str {
    match s {
        ControlState::Idle => "Idle",
        ControlState::LocalControlling => "LocalControlling",
        ControlState::RemoteControlling => "RemoteControlling",
    }
}

// ── SharingManager ──────────────────────────────────────────────────────────

pub struct SharingManager {
    state: Arc<Mutex<StateMachine>>,
    capture: Arc<PlatformCapture>,
    executor: Arc<PlatformExecutor>,
    /// CGEventTap 回调 → 异步 sender 的桥梁；start_capture 时建立，stop 时 take 销毁
    event_tx: Arc<Mutex<Option<UnboundedSender<RawInputEvent>>>>,
}

impl Default for SharingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SharingManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StateMachine::new())),
            capture: Arc::new(PlatformCapture::new()),
            executor: Arc::new(PlatformExecutor::new()),
            event_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// 当前状态（仅供测试与 get_control_state 命令）
    pub async fn current_state(&self) -> ControlState {
        self.state.lock().await.state()
    }

    /// 由 connection 事件循环调用：远端发来一个输入帧，注入到本机
    pub async fn inject_remote_frame(&self, frame: &KvmFrame) {
        // 注入前确保本端处于 RemoteControlling（首帧到达时自动切换）
        {
            let mut sm = self.state.lock().await;
            if sm.state() == ControlState::Idle {
                let evs = sm.switch_to_remote_controlling();
                drop(sm);
                self.handle_state_events_silent(evs).await;
            }
        }
        if let Err(e) = inject_frame(&*self.executor, frame) {
            log::warn!("[Sharing] 注入帧失败: {}", e);
        }
    }

    /// 由 connection 事件循环调用：远端 SwitchControl 帧
    pub async fn on_remote_switch_control(&self, app: &tauri::AppHandle) {
        let evs = self.state.lock().await.switch_to_remote_controlling();
        self.handle_state_events_with_emit(evs, app).await;
    }

    /// 由 connection 事件循环调用：远端 SwitchControlAck（对端已交还控制权）
    pub async fn on_remote_switch_ack(&self, app: &tauri::AppHandle) {
        let evs = self.state.lock().await.switch_to_idle();
        self.handle_state_events_with_emit(evs, app).await;
        // 本端可能在 LocalControlling，停止 capture
        let _ = self.capture.stop_capture();
        *self.event_tx.lock().await = None;
    }

    /// 连接断开时强制回到 IDLE
    pub async fn on_disconnected(&self, app: &tauri::AppHandle) {
        let evs = self.state.lock().await.on_disconnected();
        self.handle_state_events_with_emit(evs, app).await;
        let _ = self.capture.stop_capture();
        *self.event_tx.lock().await = None;
    }

    /// toggle_sharing 命令的核心逻辑
    pub async fn toggle(&self, app: &tauri::AppHandle) -> Result<String, String> {
        // 默认从右边缘触发（按钮场景没有真实边缘信息）
        let edge = ScreenEdge::Right;
        let pos_ratio: u16 = 5000;

        let evs = {
            let mut sm = self.state.lock().await;
            sm.toggle_control(edge, pos_ratio)
        };
        if evs.is_empty() {
            // 处于 RemoteControlling 时不响应（被对端控制时不能本地切换）
            let now = self.current_state().await;
            log::warn!("[Sharing] toggle 在 {:?} 状态下被忽略", now);
            return Err(format!("当前状态 {:?} 不支持手动切换", now));
        }
        self.handle_state_events_with_emit(evs, app).await;
        let now = self.current_state().await;
        log::info!("[Sharing] toggle → {:?}", now);
        Ok(state_name(now).to_string())
    }

    // ── 内部：处理 StateMachine 产生的事件 ──────────────────────────────────

    async fn handle_state_events_silent(&self, evs: Vec<StateEvent>) {
        for ev in evs {
            self.handle_one(ev, None).await;
        }
    }

    async fn handle_state_events_with_emit(
        &self,
        evs: Vec<StateEvent>,
        app: &tauri::AppHandle,
    ) {
        for ev in evs {
            self.handle_one(ev, Some(app)).await;
        }
    }

    async fn handle_one(&self, ev: StateEvent, app: Option<&tauri::AppHandle>) {
        match ev {
            StateEvent::StateChanged { from, to } => {
                log::info!("[Sharing] StateChanged: {} → {}", from, to);
                if let Some(app) = app {
                    let payload = ControlStateChange {
                        from: state_name(from).to_string(),
                        to: state_name(to).to_string(),
                    };
                    let _ = app.emit("control-state-changed", payload);
                }
            }
            StateEvent::SendFrame(frame) => {
                if let Some(app) = app {
                    if let Err(e) = send_frame_via_app(app, &frame).await {
                        log::warn!("[Sharing] 发送 SendFrame 失败: {}", e);
                    }
                }
            }
            StateEvent::StartCapture => {
                if let Some(app) = app {
                    if let Err(e) = self.start_capture_and_pump(app.clone()).await {
                        log::error!("[Sharing] start_capture 失败: {}", e);
                    }
                }
            }
            StateEvent::StopCapture => {
                let _ = self.capture.stop_capture();
                *self.event_tx.lock().await = None;
                log::info!("[Sharing] capture 已停止");
            }
            StateEvent::ReleaseModifiers(mods) => {
                log::debug!("[Sharing] release modifiers: bits={:#06b}", mods.bits());
            }
            StateEvent::BlockInput
            | StateEvent::UnblockInput
            | StateEvent::HideCursor
            | StateEvent::ShowCursor => {}
        }
    }

    /// 启动 capture，并把 RawInputEvent → KvmFrame → conn.send_frame
    async fn start_capture_and_pump(&self, app: tauri::AppHandle) -> KvmResult<()> {
        let (tx, mut rx) = unbounded_channel::<RawInputEvent>();
        *self.event_tx.lock().await = Some(tx.clone());

        let cb_tx = tx;
        self.capture.start_capture(Box::new(move |raw| {
            let _ = cb_tx.send(raw);
        }))?;

        tauri::async_runtime::spawn(async move {
            let mut sample_count: u64 = 0;
            while let Some(raw) = rx.recv().await {
                let frame = raw_to_frame(raw);
                if sample_count % 100 == 0 {
                    log::debug!("[Sharing] pump frame sample #{}: {:?}", sample_count, frame);
                }
                sample_count += 1;
                if let Err(e) = send_frame_via_app(&app, &frame).await {
                    log::warn!("[Sharing] send_frame 失败: {}", e);
                }
            }
            log::info!("[Sharing] capture pump 结束");
        });

        Ok(())
    }
}

/// 通过 Tauri 应用状态拿到 KvmConnectionManager，发送一帧
async fn send_frame_via_app(
    app: &tauri::AppHandle,
    frame: &KvmFrame,
) -> KvmResult<()> {
    if let Some(state) = app.try_state::<KvmConnectionManager>() {
        state.inner_send_frame(frame).await
    } else {
        Err(kvm_core::error::KvmError::PlatformError {
            api: "tauri::State<KvmConnectionManager>".to_string(),
            reason: "ConnectionManager 未注册".to_string(),
        })
    }
}

// ── 帧转换 ─────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn mouse_button_to_u8(b: MouseButton) -> u8 {
    match b {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Side1 => 3,
        MouseButton::Side2 => 4,
    }
}

fn u8_to_mouse_button(b: u8) -> MouseButton {
    match b {
        0 => MouseButton::Left,
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        3 => MouseButton::Side1,
        _ => MouseButton::Side2,
    }
}

pub(crate) fn raw_to_frame(raw: RawInputEvent) -> KvmFrame {
    let ts = now_ms();
    let mods = ModifierState::new();
    match raw {
        RawInputEvent::MouseMove { x, y } => KvmFrame::MouseMove {
            x,
            y,
            timestamp_ms: ts,
            modifiers: mods,
        },
        RawInputEvent::MouseDown { button } => KvmFrame::MouseButton {
            button: mouse_button_to_u8(button),
            pressed: true,
            timestamp_ms: ts,
            modifiers: mods,
        },
        RawInputEvent::MouseUp { button } => KvmFrame::MouseButton {
            button: mouse_button_to_u8(button),
            pressed: false,
            timestamp_ms: ts,
            modifiers: mods,
        },
        RawInputEvent::Scroll { dx, dy } => KvmFrame::MouseScroll {
            dx,
            dy,
            timestamp_ms: ts,
            modifiers: mods,
        },
        RawInputEvent::KeyDown { native_keycode } => KvmFrame::KeyEvent {
            keycode: native_keycode,
            pressed: true,
            timestamp_ms: ts,
            modifiers: mods,
        },
        RawInputEvent::KeyUp { native_keycode } => KvmFrame::KeyEvent {
            keycode: native_keycode,
            pressed: false,
            timestamp_ms: ts,
            modifiers: mods,
        },
    }
}

/// 注入：远端 KvmFrame → 本机 InputExecutor 调用
pub(crate) fn inject_frame(executor: &dyn InputExecutor, frame: &KvmFrame) -> KvmResult<()> {
    match frame {
        KvmFrame::MouseMove { x, y, .. } => executor.move_mouse(*x, *y),
        KvmFrame::MouseButton { button, pressed, .. } => {
            let b = u8_to_mouse_button(*button);
            if *pressed {
                executor.mouse_down(b)
            } else {
                executor.mouse_up(b)
            }
        }
        KvmFrame::MouseScroll { .. } => {
            // 当前 InputExecutor trait 暂未提供滚轮接口，记日志跳过（MVP）
            log::debug!("[Sharing] MouseScroll 注入未实现，跳过");
            Ok(())
        }
        KvmFrame::KeyEvent { keycode, pressed, .. } => {
            if *pressed {
                executor.key_down(*keycode)
            } else {
                executor.key_up(*keycode)
            }
        }
        _ => Ok(()),
    }
}

// ── KvmConnectionManager 上的辅助方法（在 connection/mod.rs 中加 send_frame 转发） ──

impl KvmConnectionManager {
    pub async fn send_frame(&self, frame: &KvmFrame) -> KvmResult<()> {
        self.inner_send_frame(frame).await
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn toggle_sharing(
    app: tauri::AppHandle,
    sm: tauri::State<'_, Arc<SharingManager>>,
) -> Result<String, String> {
    sm.toggle(&app).await
}

#[tauri::command]
pub async fn get_control_state(
    sm: tauri::State<'_, Arc<SharingManager>>,
) -> Result<String, String> {
    Ok(state_name(sm.current_state().await).to_string())
}

// ── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_mouse_move_to_frame() {
        let f = raw_to_frame(RawInputEvent::MouseMove { x: 100, y: 200 });
        match f {
            KvmFrame::MouseMove { x, y, .. } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("应转换为 MouseMove"),
        }
    }

    #[test]
    fn test_raw_key_down_to_frame() {
        let f = raw_to_frame(RawInputEvent::KeyDown { native_keycode: 0x41 });
        match f {
            KvmFrame::KeyEvent { keycode, pressed, .. } => {
                assert_eq!(keycode, 0x41);
                assert!(pressed);
            }
            _ => panic!("应转换为 KeyEvent"),
        }
    }

    #[test]
    fn test_raw_mouse_down_to_frame_pressed_true() {
        let f = raw_to_frame(RawInputEvent::MouseDown {
            button: MouseButton::Left,
        });
        match f {
            KvmFrame::MouseButton { button, pressed, .. } => {
                assert_eq!(button, 0);
                assert!(pressed);
            }
            _ => panic!("应转换为 MouseButton"),
        }
    }

    #[test]
    fn test_state_name_roundtrip() {
        assert_eq!(state_name(ControlState::Idle), "Idle");
        assert_eq!(state_name(ControlState::LocalControlling), "LocalControlling");
        assert_eq!(state_name(ControlState::RemoteControlling), "RemoteControlling");
    }
}

// 单元测试结束
