mod protocol;
mod discovery;
mod input_executor;
mod tray;

// 新模块结构
mod connection;
mod control;

// 保留旧模块（server.rs 供 tray.rs 的 ConnectionState 类型使用）
mod server;
mod role_manager;

use std::sync::Arc;
use connection::{ConnectionManager, ConnectionEvent, ConnectionRole};
use control::{EdgeController, EdgeConfig, EdgeDirection, HotkeyController, HotkeyAction};

/// Mac 端自身 IP
const MAC_DEFAULT_IP: &str = "30.19.140.39";
/// Windows 端默认 IP（Mac 端连接的目标）
const WINDOWS_DEFAULT_IP: &str = "30.19.24.43";

fn main() {
    println!("========================================");
    println!("  KVM Agent v0.2.0 - Mac 端 KVM 程序");
    println!("  Mac IP: {}  Windows IP: {}", MAC_DEFAULT_IP, WINDOWS_DEFAULT_IP);
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

    // ── 创建连接管理器（Mac 端默认被控端）──────────────────────────────
    let conn_mgr = Arc::new(ConnectionManager::new(|event| {
        match event {
            ConnectionEvent::RoleChanged { role } => {
                let role_name = match role {
                    ConnectionRole::Controlled => "被控端",
                    ConnectionRole::Controller => "主控端",
                };
                println!("[连接] 角色切换为: {}", role_name);
                tray::send_notification("KVM Agent", &format!("已切换为{}", role_name));
            }
            ConnectionEvent::Connected { peer_ip } => {
                println!("[连接] 已连接到: {}", peer_ip);
                tray::send_notification("KVM Agent", &format!("已连接到 {}", peer_ip));
            }
            ConnectionEvent::Disconnected => {
                println!("[连接] 已断开连接");
            }
            ConnectionEvent::Error { error, guide } => {
                eprintln!("[连接] 错误: {}", error);
                eprintln!("[连接] 解决方案:");
                for step in &guide {
                    eprintln!("  {}", step);
                }
            }
            ConnectionEvent::Reconnecting { attempt, max_attempts, peer_ip } => {
                println!("[连接] 重连中（{}/{}）→ {}", attempt, max_attempts, peer_ip);
            }
            ConnectionEvent::Listening { port } => {
                println!("[连接] 被控端监听端口 {}", port);
            }
        }
    }));

    // ── 创建屏幕边缘控制器（支持多显示器）──────────────────────────────
    let mut edge_ctrl = EdgeController::new(1440, 900); // Mac 默认分辨率，运行时会更新
    edge_ctrl.set_config(EdgeConfig {
        trigger_edges: vec![EdgeDirection::Right], // 默认右边缘切换到 Windows
        threshold: 1,
        debounce_ms: 50,
    });
    let edge_ctrl = Arc::new(std::sync::Mutex::new(edge_ctrl));

    // ── 创建快捷键控制器 ──────────────────────────────────────────────
    let conn_mgr_hk = Arc::clone(&conn_mgr);
    let mut hotkey_ctrl = HotkeyController::new(move |action| {
        match action {
            HotkeyAction::ToggleRole => {
                let current = conn_mgr_hk.role();
                match current {
                    ConnectionRole::Controlled => {
                        println!("[快捷键] 切换为主控端，连接 {}", WINDOWS_DEFAULT_IP);
                        conn_mgr_hk.switch_to_controller(Some(WINDOWS_DEFAULT_IP.to_string()));
                    }
                    ConnectionRole::Controller => {
                        println!("[快捷键] 切换为被控端");
                        conn_mgr_hk.switch_to_controlled();
                    }
                }
            }
            HotkeyAction::Disconnect => {
                conn_mgr_hk.stop();
            }
            HotkeyAction::Custom(name) => {
                println!("[快捷键] 自定义动作: {}", name);
            }
        }
    });
    hotkey_ctrl.register_defaults();

    // ── 启动 UDP 发现监听线程 ──────────────────────────────────────────
    {
        let hn = hostname.clone();
        let os = os_version.clone();
        std::thread::spawn(move || {
            if let Err(e) = discovery::run_udp_listener(hn, os) {
                eprintln!("[UDP] 监听失败: {}", e);
            }
        });
    }

    // ── 启动鼠标边缘检测线程（主控端模式下轮询鼠标位置）──────────────
    {
        let edge = Arc::clone(&edge_ctrl);
        let conn = Arc::clone(&conn_mgr);
        std::thread::spawn(move || {
            run_edge_detection_loop(edge, conn);
        });
    }

    // ── 默认启动被控端模式 ────────────────────────────────────────────
    conn_mgr.switch_to_controlled();
    println!("[主程序] Mac 端已启动，默认角色：被控端，监听端口 8889");
    println!("[主程序] Windows 端默认 IP: {}，Mac 端 IP: {}", WINDOWS_DEFAULT_IP, MAC_DEFAULT_IP);
    println!("[主程序] 快捷键 Ctrl+Alt+K 切换主控/被控角色");

    // ── 主线程运行托盘（macOS 要求 UI 在主线程）──────────────────────
    tray::run_tray(conn_mgr, edge_ctrl, WINDOWS_DEFAULT_IP);
}

/// 鼠标边缘检测循环（在独立线程中运行）
///
/// 仅在主控端模式下检测鼠标位置，到达边缘时切换回被控端。
/// 被控端模式下不检测（不需要捕获鼠标）。
fn run_edge_detection_loop(
    edge_ctrl: Arc<std::sync::Mutex<EdgeController>>,
    conn_mgr: Arc<ConnectionManager>,
) {
    use std::time::Duration;

    loop {
        std::thread::sleep(Duration::from_millis(8)); // ~120Hz 轮询

        // 仅主控端模式下检测边缘
        if conn_mgr.role() != ConnectionRole::Controller {
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
            // 到达边缘 → 切换回被控端（控制权交还给本机）
            conn_mgr.switch_to_controlled();
            tray::send_notification("KVM Agent", "已切换回被控端（鼠标回到 Mac）");
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
