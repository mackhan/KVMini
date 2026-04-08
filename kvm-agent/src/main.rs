mod protocol;
mod discovery;
mod input_executor;
mod tray;

// 连接模块
mod connection;
mod control;

// 保留旧模块（兼容性）
mod server;
mod role_manager;

use std::sync::Arc;
use connection::{ConnectionManager, ConnectionEvent};
use control::{EdgeController, EdgeConfig, EdgeDirection, HotkeyController, HotkeyAction};

/// 对端默认 IP
const DEFAULT_PEER_IP: &str = "30.19.24.43";
/// 本机 IP（仅用于日志显示）
const LOCAL_IP: &str = "30.19.140.39";

fn main() {
    println!("========================================");
    println!("  KVM Agent v0.3.0 - 对等 KVM 程序");
    println!("  本机 IP: {}  对端 IP: {}", LOCAL_IP, DEFAULT_PEER_IP);
    println!("========================================");

    // 检测辅助功能权限
    if !control::InputController::check_permission() {
        control::InputController::print_permission_guide();
        eprintln!("⚠️  未获得辅助功能权限，鼠标/键盘控制将不可用");
    }

    // 获取本机信息
    let hostname = discovery::get_hostname();
    let os_version = discovery::get_os_version();
    println!("[主程序] 主机名: {}，系统: {}", hostname, os_version);

    // ── 创建连接管理器 ────────────────────────────────────────────────────
    let conn_mgr = Arc::new(ConnectionManager::new(|event| {
        match event {
            ConnectionEvent::Connected { peer_ip } => {
                println!("[连接] 已连接到: {}", peer_ip);
                tray::send_notification("KVM Agent", &format!("已连接到 {}", peer_ip));
            }
            ConnectionEvent::Disconnected => {
                println!("[连接] 连接已断开");
                tray::send_notification("KVM Agent", "连接已断开，正在自动重连...");
            }
            ConnectionEvent::Error { error, guide } => {
                eprintln!("[连接] 错误: {}", error);
                for step in &guide {
                    eprintln!("  {}", step);
                }
            }
            ConnectionEvent::Reconnecting { attempt, peer_ip } => {
                println!("[连接] 重连中（第 {} 次）→ {}", attempt, peer_ip);
                if attempt == 11 {
                    tray::send_notification("KVM Agent", "重连失败，仍在后台尝试...");
                }
            }
            ConnectionEvent::ReconnectSuccess { peer_ip } => {
                println!("[连接] 重连成功，已连接到: {}", peer_ip);
                tray::send_notification("KVM Agent", &format!("重连成功，已连接到 {}", peer_ip));
            }
            ConnectionEvent::Listening { port } => {
                println!("[连接] 监听端口 {}", port);
            }
        }
    }));

    // ── 创建屏幕边缘控制器 ────────────────────────────────────────────────
    let mut edge_ctrl = EdgeController::new(1440, 900);
    edge_ctrl.set_config(EdgeConfig {
        trigger_edges: vec![EdgeDirection::Right],
        threshold: 1,
        debounce_ms: 50,
    });
    let edge_ctrl = Arc::new(std::sync::Mutex::new(edge_ctrl));

    // ── 创建快捷键控制器 ──────────────────────────────────────────────────
    let conn_mgr_hk = Arc::clone(&conn_mgr);
    let mut hotkey_ctrl = HotkeyController::new(move |action| {
        match action {
            HotkeyAction::Disconnect => {
                println!("[快捷键] 断开连接");
                conn_mgr_hk.disconnect();
            }
            HotkeyAction::Custom(name) => {
                println!("[快捷键] 自定义动作: {}", name);
            }
            _ => {}
        }
    });
    hotkey_ctrl.register_defaults();

    // ── 启动 UDP 发现监听线程 ──────────────────────────────────────────────
    {
        let hn = hostname.clone();
        let os = os_version.clone();
        std::thread::spawn(move || {
            if let Err(e) = discovery::run_udp_listener(hn, os) {
                eprintln!("[UDP] 监听失败: {}", e);
            }
        });
    }

    // ── 默认启动监听模式 ──────────────────────────────────────────────────
    conn_mgr.start_listening();
    println!("[主程序] KVM Agent 已启动，监听端口 8889，等待对端连接");
    println!("[主程序] 对端默认 IP: {}，输入 'c' 主动发起连接", DEFAULT_PEER_IP);

    // ── 主线程运行托盘（macOS 要求 UI 在主线程）──────────────────────────
    tray::run_tray(conn_mgr, edge_ctrl, DEFAULT_PEER_IP);
}

/// 鼠标边缘检测循环（在独立线程中运行）
///
/// 仅在已连接状态下检测鼠标位置，到达边缘时断开连接（控制权交还本机）。
#[allow(dead_code)]
fn run_edge_detection_loop(
    edge_ctrl: Arc<std::sync::Mutex<EdgeController>>,
    conn_mgr: Arc<ConnectionManager>,
) {
    use std::time::Duration;

    loop {
        std::thread::sleep(Duration::from_millis(8)); // ~120Hz 轮询

        // 仅已连接状态下检测边缘
        if !conn_mgr.is_connected() {
            continue;
        }

        // 获取当前鼠标位置
        let (mx, my) = get_mouse_position();

        // 检测边缘
        let result = edge_ctrl.lock().unwrap().detect(mx, my);

        use control::EdgeDetectResult;
        if let EdgeDetectResult::Triggered { direction, pos_ratio, monitor_id } = result {
            println!(
                "[边缘] 触发切换：方向={:?}，位置比例={}，显示器={}",
                direction, pos_ratio, monitor_id
            );
            // 到达边缘 → 断开连接（控制权交还给本机）
            conn_mgr.disconnect();
            tray::send_notification("KVM Agent", "鼠标回到本机边缘，已断开连接");
        }
    }
}

/// 获取当前鼠标位置（macOS 使用 CGEventSource API）
fn get_mouse_position() -> (i32, i32) {
    #[cfg(target_os = "macos")]
    {
        get_mouse_position_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        (0, 0)
    }
}

/// macOS 鼠标位置获取（通过 CGEvent 获取当前鼠标位置）
#[cfg(target_os = "macos")]
fn get_mouse_position_macos() -> (i32, i32) {
    use core_graphics::event::{CGEvent, CGEventType};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    // 创建一个鼠标移动事件来获取当前鼠标位置
    if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        if let Ok(event) = CGEvent::new(source) {
            let loc = event.location();
            return (loc.x as i32, loc.y as i32);
        }
    }
    (0, 0)
}

#[cfg(not(target_os = "macos"))]
fn get_mouse_position_macos() -> (i32, i32) {
    (0, 0)
}
