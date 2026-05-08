//! 键鼠共享模块（Bug 2 / BUG-019 / BUG-022 / BUG-028）
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
//! BUG-022 修复（v0.1.38）：
//! - StateChanged 时通过 `capture.set_control_state()` 把新状态推送给 CGEventTap 回调，
//!   让回调按状态决定 Some/None（Idle 放行，LocalControlling/RemoteControlling 吞）。
//! - `BlockInput/UnblockInput` 不再是空实现，真正调 `InputBlocker`（Mac 叠加一层屏蔽 Tap、
//!   Win 走 `BlockInput` API），双重保障被对端控制时本机键鼠不受干扰。
//! - `start_capture_and_pump` 的 MouseMove 分支接入 `EdgeDetector`：
//!   按保存的 LayoutConfig + 虚拟桌面构建 detector，鼠标触达布局触发边缘时自动
//!   调 StateMachine.switch_to_idle（LocalControlling → IDLE，交还控制权）。
//!
//! BUG-028 修复（v0.1.41）：
//! - 根因：Armed 状态下 CGEventTap/WinHook 未启动，无法采集鼠标事件，边缘检测永远不触发。
//! - 修复：Idle → Armed 时即启动 capture（监听模式：放行所有事件，同时喂 EdgeDetector）。
//!   pump loop 中按当前状态决定是否转发帧到对端（Armed 不发，LocalControlling 发）。
//! - Armed → Idle 时停止 capture。LocalControlling → Armed 时保持 capture 运行。
//!
//! 平台隔离：MacInputCapture / MacInputExecutor / WinInputCapture / WinInputExecutor 通过 cfg 选择。
//! CGEventTap 的 RunLoop 隔离已经封装在 MacInputCapture::start_capture 内部。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use kvm_core::config::ScreenLayout;
use kvm_core::edge_logic::{EdgeConfig, EdgeDetectResult, EdgeDetector};
use kvm_core::error::KvmResult;
use kvm_core::platform::{CursorController, FocusKeeper, InputBlocker, InputCapture, InputExecutor, MouseButton, RawInputEvent, VirtualDesktop};
use kvm_core::protocol::{KvmFrame, ModifierState, ScreenEdge};
use kvm_core::remote_cursor::{RemoteCursorAction, RemoteCursorState};
use kvm_core::state::{ControlState, StateEvent, StateMachine};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Mutex;

use crate::connection::KvmConnectionManager;

// ── 平台特定 capture / executor / blocker 实现 ──────────────────────────────

#[cfg(target_os = "macos")]
type PlatformCapture = crate::platform::macos::input_capture::MacInputCapture;
#[cfg(target_os = "macos")]
type PlatformExecutor = crate::platform::macos::input_inject::MacInputExecutor;
#[cfg(target_os = "macos")]
type PlatformBlocker = crate::platform::macos::input_block::MacInputBlocker;
#[cfg(target_os = "macos")]
type PlatformCursor = crate::platform::macos::cursor::MacCursorController;
#[cfg(target_os = "macos")]
type PlatformFocusKeeper = crate::platform::macos::focus_keeper::MacFocusKeeper;

#[cfg(target_os = "windows")]
type PlatformCapture = crate::platform::windows::input_capture::WinInputCapture;
#[cfg(target_os = "windows")]
type PlatformExecutor = crate::platform::windows::input_inject::WinInputExecutor;
#[cfg(target_os = "windows")]
type PlatformBlocker = crate::platform::windows::input_block::WinInputBlocker;
#[cfg(target_os = "windows")]
type PlatformCursor = crate::platform::windows::cursor::WinCursorController;
#[cfg(target_os = "windows")]
type PlatformFocusKeeper = crate::platform::windows::focus_keeper::WinFocusKeeper;

// ── 控制状态 → u8（与 platform::*::input_capture 的 CONTROL_STATE_* 常量对齐） ──

fn control_state_to_u8(s: ControlState) -> u8 {
    match s {
        ControlState::Idle => 0,
        ControlState::Armed => 3,
        ControlState::LocalControlling => 1,
        ControlState::RemoteControlling => 2,
    }
}

fn layout_string_to_enum(s: &str) -> ScreenLayout {
    match s {
        "LocalRightOfRemote" | "LocalRight" => ScreenLayout::LocalRight,
        "LocalAboveRemote" | "LocalAbove" => ScreenLayout::LocalAbove,
        "LocalBelowRemote" | "LocalBelow" => ScreenLayout::LocalBelow,
        _ => ScreenLayout::LocalLeft, // 默认 + "LocalLeftOfRemote" 兼容
    }
}

// ── 前端事件 payload ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlStateChange {
    pub from: String,
    pub to: String,
}

fn state_name(s: ControlState) -> &'static str {
    match s {
        ControlState::Idle => "Idle",
        ControlState::Armed => "Armed",
        ControlState::LocalControlling => "LocalControlling",
        ControlState::RemoteControlling => "RemoteControlling",
    }
}

// ── SharingManager ──────────────────────────────────────────────────────────

pub struct SharingManager {
    state: Arc<Mutex<StateMachine>>,
    capture: Arc<PlatformCapture>,
    executor: Arc<PlatformExecutor>,
    /// 本地键鼠屏蔽器（BUG-022：RemoteControlling 时阻止本地键鼠干扰）
    blocker: Arc<PlatformBlocker>,
    /// 光标控制器（LocalControlling 时 grab 断开鼠标↔光标绑定）
    cursor: Arc<PlatformCursor>,
    /// v0.1.41: 焦点保持器（macOS: 防止失焦后 CGAssociate 被系统重置）
    focus_keeper: Arc<PlatformFocusKeeper>,
    /// CGEventTap 回调 → 异步 sender 的桥梁；start_capture 时建立，stop 时 take 销毁
    event_tx: Arc<Mutex<Option<UnboundedSender<RawInputEvent>>>>,
    /// 边缘检测器（BUG-022：LocalControlling 时鼠标触达反向边缘回切 IDLE）
    /// 在 start_capture_and_pump 里按当前 LayoutConfig + 虚拟桌面懒初始化
    edge_detector: Arc<Mutex<Option<EdgeDetector>>>,
    /// v0.1.40: 远端虚拟光标（RemoteControlling 态下维护"对方光标在本机屏幕上的位置"）
    /// 由 on_remote_switch_control 入态时初始化，MouseMoveDelta 帧累加，撞反向边缘触发 SwitchBack
    remote_cursor: Arc<Mutex<Option<RemoteCursorState>>>,
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
            blocker: Arc::new(PlatformBlocker::new()),
            cursor: Arc::new(PlatformCursor::new()),
            focus_keeper: Arc::new(PlatformFocusKeeper::new()),
            event_tx: Arc::new(Mutex::new(None)),
            edge_detector: Arc::new(Mutex::new(None)),
            remote_cursor: Arc::new(Mutex::new(None)),
        }
    }

    /// 当前状态（仅供测试与 get_control_state 命令）
    pub async fn current_state(&self) -> ControlState {
        self.state.lock().await.state()
    }

    /// 由 connection 事件循环调用：远端发来一个输入帧，注入到本机
    ///
    /// v0.1.40: MouseMoveDelta 走 RemoteCursorState（累加 → clamp → warp / SwitchBack）；
    /// 其他帧（MouseButton/MouseScroll/KeyEvent）维持原 inject_frame 路径。
    pub async fn inject_remote_frame(&self, frame: &KvmFrame, app: &tauri::AppHandle) {
        // 注入前确保本端处于 RemoteControlling（首帧到达时自动切换）
        {
            let mut sm = self.state.lock().await;
            if sm.state() == ControlState::Idle {
                let evs = sm.switch_to_remote_controlling();
                drop(sm);
                self.handle_state_events_silent(evs).await;
            }
        }

        // v0.1.40: 鼠标 delta 单独处理（维护虚拟光标 + 反向边缘回切）
        if let KvmFrame::MouseMoveDelta { dx, dy } = frame {
            self.handle_remote_mouse_delta(*dx, *dy, app).await;
            return;
        }

        if let Err(e) = inject_frame(&*self.executor, frame) {
            log::warn!("[Sharing] 注入帧失败: {}", e);
        }
    }

    /// 处理远端发来的 MouseMoveDelta：累加虚拟光标位置，必要时发 SwitchBackControl
    async fn handle_remote_mouse_delta(&self, dx: i16, dy: i16, app: &tauri::AppHandle) {
        // 若 RemoteCursorState 未初始化（比如收到 delta 前未收到 SwitchControl），
        // 按当前本机显示器中心懒初始化一个，避免丢帧。
        let action = {
            let mut slot = self.remote_cursor.lock().await;
            if slot.is_none() {
                if let Some(state) = build_remote_cursor_default() {
                    log::warn!("[Sharing] MouseMoveDelta 到达时 RemoteCursor 未初始化，按默认入屏点构建");
                    *slot = Some(state);
                } else {
                    log::warn!("[Sharing] MouseMoveDelta 无法构建 RemoteCursor（显示器读取失败），丢弃");
                    return;
                }
            }
            slot.as_mut().unwrap().on_delta(dx, dy)
        };

        match action {
            RemoteCursorAction::Warp { x, y } => {
                if let Err(e) = self.cursor.set_position(x, y) {
                    log::warn!("[Sharing] RemoteCursor warp 失败: {}", e);
                }
            }
            RemoteCursorAction::SwitchBack => {
                log::info!("[Sharing] 远端虚拟光标撞反向边缘 → 发 SwitchBackControl");
                // 先发 SwitchBackControl 通知对端
                if let Err(e) = send_frame_via_app(app, &KvmFrame::SwitchBackControl).await {
                    log::warn!("[Sharing] SwitchBackControl 发送失败: {}", e);
                }
                // 本端：RemoteControlling → Idle（对端收到后应回到 Armed/LocalControlling）
                let evs = {
                    let mut sm = self.state.lock().await;
                    sm.on_disconnected() // 直接归零，清理 blocker/cursor
                };
                // 清理虚拟光标
                *self.remote_cursor.lock().await = None;
                self.handle_state_events_with_emit(evs, app).await;
            }
        }
    }

    /// 由 connection 事件循环调用：远端 SwitchControl 帧
    ///
    /// v0.1.40: edge/pos_ratio 用于初始化 RemoteCursorState（对方出屏点 → 本机入屏点）
    pub async fn on_remote_switch_control(
        &self,
        app: &tauri::AppHandle,
        edge: ScreenEdge,
        pos_ratio: u16,
    ) {
        let evs = self.state.lock().await.switch_to_remote_controlling();

        // 初始化远端虚拟光标状态
        if let Some(state) = build_remote_cursor_from_edge(edge, pos_ratio) {
            log::info!(
                "[Sharing] RemoteCursor 初始化：edge={:?}, pos_ratio={}, 入屏点=({}, {})",
                edge, pos_ratio, state.x, state.y
            );
            // 立即 warp 到入屏点（给用户视觉反馈：对方光标"刚进来"）
            let _ = self.cursor.set_position(state.x, state.y);
            *self.remote_cursor.lock().await = Some(state);
        } else {
            log::warn!("[Sharing] RemoteCursor 初始化失败（显示器读取失败）");
        }

        self.handle_state_events_with_emit(evs, app).await;
    }

    /// 由 connection 事件循环调用：远端 SwitchControlAck（对端确认已进入 RemoteControlling）
    /// BUG-030c: 收到 Ack 后本端应保持 LocalControlling 继续控制远端，
    /// 不能切回 Idle（之前的实现把 Ack 当作"交还控制权"处理是错误的）。
    /// 只有收到 SwitchBackControl 或用户按快捷键才应停止控制。
    pub async fn on_remote_switch_ack(&self, _app: &tauri::AppHandle) {
        log::info!("[Sharing] 收到远端 SwitchControlAck，确认远端已进入 RemoteControlling，保持 LocalControlling");
        // 不改状态，不停 capture，保持 LocalControlling 状态
    }

    /// 由 connection 事件循环调用：远端 SwitchBackControl（远端撞反向边缘，回切到 Armed）
    pub async fn on_remote_switch_back(&self, app: &tauri::AppHandle) {
        let evs = self.state.lock().await.local_to_armed();
        if evs.is_empty() {
            log::warn!("[Sharing] on_remote_switch_back: 当前状态不是 LocalControlling，忽略");
            return;
        }
        log::info!("[Sharing] on_remote_switch_back: LocalControlling → Armed");
        self.handle_state_events_with_emit(evs, app).await;
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
                // BUG-022：状态镜像到 capture 回调线程，让 CGEventTap / WinHook
                // 按新状态决定 Some(event)（放行）或 None（吞）。
                self.capture.set_control_state(control_state_to_u8(to));
                // BUG-030: EdgeDetector 只在 Armed 状态激活
                // LocalControlling 下禁用（回切靠快捷键或远端 SwitchBackControl）
                // 避免鼠标仍停在边缘区域导致回到 Armed 后立刻重新触发
                {
                    let mut slot = self.edge_detector.lock().await;
                    if let Some(d) = slot.as_mut() {
                        d.set_active(to == ControlState::Armed);
                        // 进入非 Armed 状态时重置计数，回到 Armed 时从零开始
                        if to != ControlState::Armed {
                            d.reset_overshoot();
                        }
                    }
                }

                // BUG-028：Armed 状态需要 capture 运行以监听鼠标边缘
                // Idle → Armed：启动 capture + EdgeDetector（监听模式，不吞事件）
                if from == ControlState::Idle && to == ControlState::Armed {
                    if let Some(app) = app {
                        if let Err(e) = self.start_capture_and_pump(app.clone()).await {
                            log::error!("[Sharing] Armed 启动 capture 失败: {}", e);
                        } else {
                            log::info!("[Sharing] Armed 模式 capture 已启动（监听边缘）");
                            // BUG-034: CGEventTap 启动成功 = 输入监控权限实际可用
                            // 通知前端隐藏可能因 CGPreflightListenEventAccess 进程缓存残留的权限横幅
                            let _ = app.emit("input-monitoring://permission-granted", ());
                        }
                    }
                }
                // Armed → Idle：停止 capture + 清理 EdgeDetector
                if from == ControlState::Armed && to == ControlState::Idle {
                    let _ = self.capture.stop_capture();
                    *self.event_tx.lock().await = None;
                    *self.edge_detector.lock().await = None;
                    log::info!("[Sharing] Armed → Idle: capture 已停止，edge_detector 已清理");
                }

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
                // BUG-028: 如果当前状态是 Armed，不停止 capture（继续监听边缘）
                let current = self.state.lock().await.state();
                if current == ControlState::Armed {
                    log::info!("[Sharing] StopCapture 跳过（Armed 保持 capture 运行以监听边缘）");
                } else {
                    let _ = self.capture.stop_capture();
                    *self.event_tx.lock().await = None;
                    *self.edge_detector.lock().await = None;
                    log::info!("[Sharing] capture 已停止，edge_detector 已清理");
                }
            }
            StateEvent::ReleaseModifiers(mods) => {
                log::debug!("[Sharing] release modifiers: bits={:#06b}", mods.bits());
            }
            StateEvent::BlockInput => {
                // BUG-022：被对端控制时屏蔽本地键鼠，防止本地干扰注入流
                if let Err(e) = self.blocker.block_input() {
                    log::warn!("[Sharing] BlockInput 调用失败: {}", e);
                } else {
                    log::info!("[Sharing] BlockInput 已生效（本地键鼠已屏蔽）");
                }
            }
            StateEvent::UnblockInput => {
                if let Err(e) = self.blocker.unblock_input() {
                    log::warn!("[Sharing] UnblockInput 调用失败: {}", e);
                } else {
                    log::info!("[Sharing] UnblockInput 已生效（本地键鼠已恢复）");
                }
            }
            StateEvent::HideCursor => {
                // LocalControlling 时：断开物理鼠标与屏幕光标的绑定 + 隐藏光标
                // 这样鼠标移动不会驱动本机屏幕光标，用户感知"鼠标被远端接管"
                if let Err(e) = self.cursor.grab() {
                    log::warn!("[Sharing] HideCursor(grab) 失败: {}", e);
                } else {
                    log::info!("[Sharing] HideCursor(grab) 已生效：鼠标↔光标绑定断开");
                }
                // v0.1.41: 启动 FocusKeeper，防止 macOS 失焦后 CGAssociate 被重置
                if let Err(e) = self.focus_keeper.start() {
                    log::warn!("[Sharing] FocusKeeper.start() 失败: {}", e);
                } else {
                    log::info!("[Sharing] FocusKeeper 已启动");
                }
            }
            StateEvent::ShowCursor => {
                // v0.1.41: 先停止 FocusKeeper，把焦点还给系统
                if let Err(e) = self.focus_keeper.stop() {
                    log::warn!("[Sharing] FocusKeeper.stop() 失败: {}", e);
                }
                // 退出控制时：恢复物理鼠标与屏幕光标的绑定 + 显示光标
                if let Err(e) = self.cursor.release() {
                    log::warn!("[Sharing] ShowCursor(release) 失败: {}", e);
                } else {
                    log::info!("[Sharing] ShowCursor(release) 已生效：鼠标↔光标绑定恢复");
                }
            }
        }
    }

    /// 启动 capture，并把 RawInputEvent → KvmFrame → conn.send_frame
    /// 同时构建 EdgeDetector（BUG-022：LocalControlling 下鼠标触达回切边缘自动回 IDLE）
    async fn start_capture_and_pump(&self, app: tauri::AppHandle) -> KvmResult<()> {
        let (tx, mut rx) = unbounded_channel::<RawInputEvent>();
        *self.event_tx.lock().await = Some(tx.clone());

        // ── 构建 EdgeDetector（BUG-022）────────────────────────────
        // 来源：config::LayoutConfig（layout 字符串）+ 本机显示器组成的虚拟桌面
        let edge_detector = build_edge_detector_for_local().await;
        if edge_detector.is_some() {
            log::info!("[Sharing] EdgeDetector 已创建（按布局方向检测回切边缘）");
        } else {
            log::warn!("[Sharing] EdgeDetector 未创建（布局/显示器读取失败），边缘回切不生效");
        }
        *self.edge_detector.lock().await = edge_detector;

        let cb_tx = tx;
        self.capture.start_capture(Box::new(move |raw| {
            let _ = cb_tx.send(raw);
        }))?;

        let edge_detector_arc = Arc::clone(&self.edge_detector);
        let state_arc = Arc::clone(&self.state);
        let capture_arc: Arc<PlatformCapture> = Arc::clone(&self.capture);
        let blocker_arc: Arc<PlatformBlocker> = Arc::clone(&self.blocker);
        let cursor_arc: Arc<PlatformCursor> = Arc::clone(&self.cursor);
        let app_for_edge = app.clone();

        tauri::async_runtime::spawn(async move {
            let mut sample_count: u64 = 0;
            while let Some(raw) = rx.recv().await {
                // BUG-022：MouseMove 分支先喂 EdgeDetector，触发回切
                if let RawInputEvent::MouseMove { x, y, .. } = raw {
                    let result = {
                        let mut slot = edge_detector_arc.lock().await;
                        if let Some(d) = slot.as_mut() {
                            d.detect(x, y)
                        } else {
                            EdgeDetectResult::None
                        }
                    };
                    if let EdgeDetectResult::Triggered { direction, pos_ratio } = result {
                        // BUG-030: 只在 Armed 状态下响应边缘触发（Armed→LocalControlling）
                        // LocalControlling 下不通过本地边缘检测回切（回切靠快捷键或远端 SwitchBackControl）
                        // 原因：进入 LocalControlling 后鼠标坐标仍停留在边缘区域，
                        // 如果响应会立即触发回切，导致 Armed↔LocalControlling 无限振荡
                        let current_state = state_arc.lock().await.state();
                        if current_state == ControlState::Armed {
                            log::info!(
                                "[Sharing] EdgeDetect triggered (Armed→Local): dir={:?} pos_ratio={}",
                                direction,
                                pos_ratio
                            );
                            let events = {
                                let mut sm = state_arc.lock().await;
                                sm.armed_to_local_controlling(direction, pos_ratio)
                            };
                            handle_events_on_edge_trigger(
                                events,
                                &app_for_edge,
                                &capture_arc,
                                &blocker_arc,
                                &cursor_arc,
                            )
                            .await;
                        }
                    }
                }

                let frame = raw_to_frame(raw);
                // BUG-028: 只在 LocalControlling 状态下才转发帧到对端
                // Armed 状态下 capture 只用于边缘检测，不发帧
                let current_state = state_arc.lock().await.state();
                if current_state == ControlState::LocalControlling {
                    if sample_count % 100 == 0 {
                        log::debug!("[Sharing] pump frame sample #{}: {:?}", sample_count, frame);
                    }
                    sample_count += 1;
                    if let Err(e) = send_frame_via_app(&app, &frame).await {
                        log::warn!("[Sharing] send_frame 失败: {}", e);
                    }
                }
            }
            log::info!("[Sharing] capture pump 结束");
        });

        Ok(())
    }
}

/// v0.1.40: 按对端发来的 edge + pos_ratio 构建本机远端虚拟光标
///
/// 对端 EdgeDetector 触发 → 发 SwitchControl{edge, pos_ratio}
/// 本机收到后用这里构建 RemoteCursorState：入屏点 = 本机 edge.opposite() 边按 pos_ratio
fn build_remote_cursor_from_edge(edge: ScreenEdge, pos_ratio: u16) -> Option<RemoteCursorState> {
    let monitors = match crate::platform::screen::get_monitors_impl() {
        Ok(list) if !list.is_empty() => list,
        _ => return None,
    };
    let desktop = VirtualDesktop::from_monitors(monitors);
    // 返回阈值复用 edge_overshoot_px（与本地 Armed → LocalControlling 触发阈值对称）
    let app_config = crate::config::load_config();
    let threshold = app_config.edge_overshoot_px.clamp(1, 200);
    Some(RemoteCursorState::on_enter(edge, pos_ratio, &desktop, threshold))
}

/// v0.1.40: MouseMoveDelta 先到、SwitchControl 还没到的兜底构造
/// 用屏幕中心作入屏点，return_edge 默认 Left（任意选一个，后续 SwitchControl 会覆盖）
fn build_remote_cursor_default() -> Option<RemoteCursorState> {
    build_remote_cursor_from_edge(ScreenEdge::Left, 5000)
}

/// 按当前 LayoutConfig 构建 EdgeDetector（失败时返回 None，不阻塞 capture 流程）
async fn build_edge_detector_for_local() -> Option<EdgeDetector> {
    // 1. 读取布局（同步 IO，量极小，不需要异步）
    let layout_cfg = crate::config::load_layout_config();
    let layout = layout_string_to_enum(&layout_cfg.layout);
    // 2. 枚举本机显示器 → VirtualDesktop
    let monitors = match crate::platform::screen::get_monitors_impl() {
        Ok(list) if !list.is_empty() => list,
        Ok(_) => {
            log::warn!("[Sharing] 未枚举到显示器，EdgeDetector 跳过");
            return None;
        }
        Err(e) => {
            log::warn!("[Sharing] 获取显示器失败 ({}), EdgeDetector 跳过", e);
            return None;
        }
    };
    let desktop = VirtualDesktop::from_monitors(monitors);

    // 3. 读取 AppConfig 获取 overshoot_px
    let app_config = crate::config::load_config();
    let overshoot_px = app_config.edge_overshoot_px.clamp(0, 200);

    let mut config = EdgeConfig::from_layout_with_overshoot(layout, overshoot_px);
    // 阈值（像素）从 AppConfig 读取，容忍读失败
    config.threshold = app_config.edge_threshold.max(1);

    log::info!(
        "[Sharing] EdgeDetector 配置: edge={:?}, threshold={}, overshoot_px={}",
        config.trigger_edge, config.threshold, config.overshoot_px
    );
    let detector = EdgeDetector::new(config, desktop);
    Some(detector)
}

/// 边缘触发后对状态机产生的 events 做最小闭环处理
/// BUG-028 改造：Armed 状态下 capture 常驻运行，边缘触发时不重启/不停止 capture
async fn handle_events_on_edge_trigger(
    events: Vec<StateEvent>,
    app: &tauri::AppHandle,
    capture: &Arc<PlatformCapture>,
    blocker: &Arc<PlatformBlocker>,
    cursor: &Arc<PlatformCursor>,
) {
    // 先扫一遍确定目标状态（用于判断是否保持 capture 运行）
    let mut target_state = None;
    for ev in &events {
        if let StateEvent::StateChanged { to, .. } = ev {
            target_state = Some(*to);
        }
    }

    for ev in events {
        match ev {
            StateEvent::StateChanged { from, to } => {
                log::info!("[Sharing/EdgeReturn] StateChanged: {} → {}", from, to);
                capture.set_control_state(control_state_to_u8(to));
                let payload = ControlStateChange {
                    from: state_name(from).to_string(),
                    to: state_name(to).to_string(),
                };
                let _ = app.emit("control-state-changed", payload);
            }
            StateEvent::SendFrame(frame) => {
                if let Err(e) = send_frame_via_app(app, &frame).await {
                    log::warn!("[Sharing/EdgeReturn] send_frame 失败: {}", e);
                }
            }
            StateEvent::StopCapture => {
                // BUG-028: 如果目标是 Armed，不停止 capture（继续监听边缘）
                if target_state == Some(ControlState::Armed) {
                    log::info!("[Sharing/EdgeReturn] StopCapture 跳过（Armed 保持 capture 运行）");
                } else {
                    let _ = capture.stop_capture();
                }
            }
            StateEvent::StartCapture => {
                // BUG-028: capture 已经在 Armed 阶段运行，无需重新启动
                log::info!("[Sharing/EdgeReturn] StartCapture 跳过（capture 已在运行）");
            }
            StateEvent::UnblockInput => {
                let _ = blocker.unblock_input();
            }
            StateEvent::HideCursor => {
                if let Err(e) = cursor.grab() {
                    log::warn!("[Sharing/EdgeReturn] HideCursor(grab) 失败: {}", e);
                } else {
                    log::info!("[Sharing/EdgeReturn] HideCursor(grab) 已生效");
                }
            }
            StateEvent::ShowCursor => {
                if let Err(e) = cursor.release() {
                    log::warn!("[Sharing/EdgeReturn] ShowCursor(release) 失败: {}", e);
                } else {
                    log::info!("[Sharing/EdgeReturn] ShowCursor(release) 已生效");
                }
            }
            other => log::debug!("[Sharing/EdgeReturn] event ignored: {:?}", other),
        }
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
        // v0.1.40: LocalControlling 态发送相对位移（避免 grab 后绝对坐标冻结）
        // x/y 保留用于 Armed 态边缘检测（在 pump 中独立消费），这里 raw_to_frame 只负责网络帧
        RawInputEvent::MouseMove { dx, dy, .. } => KvmFrame::MouseMoveDelta { dx, dy },
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
///
/// v0.1.40: MouseMoveDelta 不走这里，由 SharingManager.handle_remote_mouse_delta 专门处理
/// （需要访问 RemoteCursorState + cursor.set_position + 发 SwitchBackControl）。
pub(crate) fn inject_frame(executor: &dyn InputExecutor, frame: &KvmFrame) -> KvmResult<()> {
    match frame {
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
        // v0.1.40: MouseMove → MouseMoveDelta（delta 模式）
        let f = raw_to_frame(RawInputEvent::MouseMove { x: 100, y: 200, dx: 5, dy: -3 });
        match f {
            KvmFrame::MouseMoveDelta { dx, dy } => {
                assert_eq!(dx, 5);
                assert_eq!(dy, -3);
            }
            _ => panic!("应转换为 MouseMoveDelta"),
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
