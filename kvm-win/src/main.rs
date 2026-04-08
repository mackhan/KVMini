//! KVMini Windows 端应用入口
//!
//! 组装 kvm-core + win platform，启动应用。
//! 启动顺序：
//!   1. 初始化日志
//!   2. 设置 panic hook（确保崩溃时恢复键鼠）
//!   3. 加载配置
//!   4. 创建平台适配器，枚举显示器
//!   5. 启动 Tokio 运行时
//!   6. 启动 UDP 发现监听
//!   7. 启动 TCP 监听
//!   8. 注册全局快捷键
//!   9. 启动边缘触发检测循环
//!  10. 注册优雅退出处理

mod platform;
mod config_store;
mod autostart;
mod tray;

use kvm_core::config::KvmConfig;
use kvm_core::network::{ConnectionManager, HandshakeInfo, NetworkEvent};
use kvm_core::platform::PlatformAdapter;
use kvm_core::protocol::Platform;
use kvm_core::state::StateMachine;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

/// 全局应用状态（Arc 共享给各异步任务）
struct AppState {
    config: KvmConfig,
    adapter: platform::WinPlatformAdapter,
    state_machine: Arc<Mutex<StateMachine>>,
    stop_signal: Arc<Notify>,
}

fn main() {
    // ── 1. 初始化日志 ──────────────────────────────────────────────────────
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    ).init();

    log::info!("KVMini Windows 端启动 v{}", env!("CARGO_PKG_VERSION"));

    // ── 2. 设置 panic hook ─────────────────────────────────────────────────
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("[PANIC] 程序异常崩溃，正在恢复键鼠输入...");
        // 紧急恢复键鼠（直接调用 Win32 BlockInput(FALSE)）
        #[cfg(target_os = "windows")]
        unsafe {
            use winapi::um::winuser::BlockInput;
            BlockInput(0); // FALSE
        }
        default_hook(info);
    }));

    // ── 3. 加载配置 ────────────────────────────────────────────────────────
    let config = config_store::load_config();
    log::info!("[配置] 屏幕布局: {:?}, TCP端口: {}", config.screen_layout, config.tcp_port);

    // 根据配置设置日志级别
    let log_filter = match config.log_level {
        kvm_core::config::LogLevel::Debug => "debug",
        kvm_core::config::LogLevel::Info  => "info",
        kvm_core::config::LogLevel::Warn  => "warn",
        kvm_core::config::LogLevel::Error => "error",
    };
    let _ = log::set_max_level(log_filter.parse().unwrap_or(log::LevelFilter::Info));

    // ── 4. 创建平台适配器，枚举显示器 ──────────────────────────────────────
    let adapter = platform::WinPlatformAdapter::new();

    match adapter.screen_provider().enumerate_monitors() {
        Ok(monitors) => {
            log::info!("检测到 {} 个显示器", monitors.len());
            for m in &monitors {
                log::info!(
                    "  显示器 {}: {}x{} @ ({},{}) DPI={:.1} {}",
                    m.id, m.width, m.height, m.offset_x, m.offset_y,
                    m.dpi_scale,
                    if m.is_primary { "(主)" } else { "" }
                );
            }
        }
        Err(e) => log::error!("枚举显示器失败: {}", e),
    }

    // 检查自启动状态
    if config.autostart && !autostart::is_autostart_enabled() {
        if let Err(e) = autostart::enable_autostart() {
            log::warn!("[自启动] 注册失败: {}", e);
        }
    }

    // ── 5. 启动 Tokio 运行时 ───────────────────────────────────────────────
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("创建 Tokio 运行时失败");

    let config_clone = config.clone();
    let stop_signal = Arc::new(Notify::new());
    let stop_clone = Arc::clone(&stop_signal);

    // ── 6-9. 在 Tokio 运行时中启动所有异步任务 ────────────────────────────
    rt.block_on(async move {
        run_app(config_clone, stop_clone).await;
    });
}

/// 应用主异步入口
async fn run_app(config: KvmConfig, stop_signal: Arc<Notify>) {
    let tcp_port = config.tcp_port;
    let udp_port = config.udp_port;

    // 创建连接管理器
    let (conn_mgr, mut event_rx) = ConnectionManager::new(tcp_port);
    let conn_mgr = Arc::new(conn_mgr);

    // 创建控制状态机
    let state_machine = Arc::new(Mutex::new(StateMachine::new()));

    // ── 6. 启动 UDP 发现监听 ───────────────────────────────────────────────
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Windows-PC".to_string());

    let udp_stop = Arc::new(Notify::new());
    let udp_stop_clone = Arc::clone(&udp_stop);
    tokio::spawn(async move {
        if let Err(e) = kvm_core::network::start_discovery_listener(
            udp_port,
            tcp_port,
            hostname,
            "Windows".to_string(),
            udp_stop_clone,
        ).await {
            log::warn!("[UDP] 发现监听启动失败: {}", e);
        }
    });

    // ── 7. 启动 TCP 监听 ───────────────────────────────────────────────────
    match conn_mgr.start_listening().await {
        Ok(addr) => log::info!("[TCP] 监听地址: {}", addr),
        Err(e) => log::error!("[TCP] 监听启动失败: {}", e),
    }

    // ── 8. 注册全局快捷键（Ctrl+Alt+S 切换控制权） ────────────────────────
    let adapter = platform::WinPlatformAdapter::new();
    let sm_for_hotkey = Arc::clone(&state_machine);
    let hotkey_result = adapter.hotkey_manager().register_hotkey(
        "ctrl+alt+s",
        Box::new(move || {
            let mut sm = sm_for_hotkey.lock().unwrap();
            use kvm_core::protocol::ScreenEdge;
            let events = sm.toggle_control(ScreenEdge::Right, 5000);
            log::info!("[快捷键] 切换控制权，产生 {} 个事件", events.len());
        }),
    );
    if let Err(e) = hotkey_result {
        log::warn!("[快捷键] 注册失败: {}", e);
    }

    // ── 9. 网络事件处理循环 ────────────────────────────────────────────────
    let sm_for_events = Arc::clone(&state_machine);
    let conn_for_events = Arc::clone(&conn_mgr);
    tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    handle_network_event(event, &sm_for_events, &conn_for_events).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    log::info!("[事件] 网络事件通道已关闭");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("[事件] 丢失 {} 个网络事件（处理过慢）", n);
                }
            }
        }
    });

    log::info!("KVMini Windows 端就绪，等待连接...");

    // ── 10. 等待退出信号 ───────────────────────────────────────────────────
    let stop_clone = Arc::clone(&stop_signal);
    ctrlc::set_handler(move || {
        log::info!("[退出] 收到退出信号，正在清理...");
        stop_clone.notify_waiters();
    }).ok();

    stop_signal.notified().await;

    // ── 优雅退出：按顺序清理资源 ──────────────────────────────────────────
    log::info!("[退出] 步骤1: 恢复本地键鼠输入");
    let _ = adapter.input_blocker().unblock_input();

    log::info!("[退出] 步骤2: 恢复光标可见性");
    let _ = adapter.cursor_controller().show_cursor();

    log::info!("[退出] 步骤3: 通知对端断开");
    conn_mgr.disconnect().await;

    log::info!("[退出] 步骤4: 停止网络监听");
    conn_mgr.stop();
    udp_stop.notify_waiters();

    log::info!("[退出] 步骤5: 取消全局快捷键");
    let _ = adapter.hotkey_manager().unregister_all();

    log::info!("[退出] 步骤6: 停止剪贴板监听");
    let _ = adapter.clipboard_provider().stop_watching();

    log::info!("[退出] 清理完成，程序退出");
    std::process::exit(0);
}

/// 处理网络事件
async fn handle_network_event(
    event: NetworkEvent,
    state_machine: &Arc<Mutex<StateMachine>>,
    _conn_mgr: &Arc<ConnectionManager>,
) {
    match event {
        NetworkEvent::ConnectionStateChanged(state) => {
            log::info!("[连接] 状态变化: {}", state);
            if matches!(state, kvm_core::network::ConnectionState::Disconnected) {
                // 连接断开时重置控制状态
                let mut sm = state_machine.lock().unwrap();
                sm.on_disconnected();
            }
        }
        NetworkEvent::PeerHandshake(info) => {
            log::info!(
                "[握手] 对端: {:?} 屏幕 {}x{}",
                info.platform,
                info.screen_info.total_width,
                info.screen_info.total_height
            );
        }
        NetworkEvent::LatencyUpdated(ms) => {
            log::debug!("[延迟] {}ms", ms);
        }
        NetworkEvent::HighLatencyWarning(ms) => {
            log::warn!("[延迟] 网络延迟过高: {}ms", ms);
        }
        NetworkEvent::FrameReceived(frame) => {
            log::trace!("[帧] 收到: {:?}", frame);
        }
        NetworkEvent::Error(msg) => {
            log::error!("[网络错误] {}", msg);
        }
    }
}
