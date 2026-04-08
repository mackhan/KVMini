/// Mac 端系统托盘（菜单栏图标）
///
/// 托盘直接驱动新的 ConnectionManager，支持：
/// - 命令行交互（角色切换、状态查询）
/// - 定期状态打印
/// - macOS 系统通知

use std::sync::{Arc, Mutex};
use crate::connection::{ConnectionManager, ConnectionRole};
use crate::control::{EdgeController, EdgeDirection, EdgeConfig};

/// 启动托盘（阻塞，需在主线程调用）
pub fn run_tray(
    conn_mgr: Arc<ConnectionManager>,
    edge_ctrl: Arc<Mutex<EdgeController>>,
    windows_ip: &'static str,
) {
    #[cfg(target_os = "macos")]
    run_tray_macos(conn_mgr, edge_ctrl, windows_ip);

    #[cfg(not(target_os = "macos"))]
    {
        println!("[Tray] 当前平台不支持系统托盘，以命令行模式运行");
        run_tray_cli(conn_mgr, edge_ctrl, windows_ip);
    }
}

/// macOS 托盘实现
#[cfg(target_os = "macos")]
fn run_tray_macos(
    conn_mgr: Arc<ConnectionManager>,
    edge_ctrl: Arc<Mutex<EdgeController>>,
    windows_ip: &'static str,
) {
    use std::process::Command;

    println!("[Tray] KVM Agent 已启动，运行在菜单栏");
    print_help();

    let _ = Command::new("osascript")
        .arg("-e")
        .arg(r#"display notification "KVM Agent 已启动，默认被控端，等待连接..." with title "KVM Agent""#)
        .spawn();

    // 命令行交互线程
    let cm = Arc::clone(&conn_mgr);
    let ec = Arc::clone(&edge_ctrl);
    std::thread::spawn(move || {
        run_cli_loop(cm, ec, windows_ip);
    });

    // 主循环：定期打印状态
    run_status_loop(conn_mgr);
}

/// 非 macOS 平台的命令行托盘
#[cfg(not(target_os = "macos"))]
fn run_tray_cli(
    conn_mgr: Arc<ConnectionManager>,
    edge_ctrl: Arc<Mutex<EdgeController>>,
    windows_ip: &'static str,
) {
    print_help();
    let cm = Arc::clone(&conn_mgr);
    let ec = Arc::clone(&edge_ctrl);
    std::thread::spawn(move || {
        run_cli_loop(cm, ec, windows_ip);
    });
    run_status_loop(conn_mgr);
}

/// 命令行交互循环
fn run_cli_loop(
    conn_mgr: Arc<ConnectionManager>,
    edge_ctrl: Arc<Mutex<EdgeController>>,
    windows_ip: &'static str,
) {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                let input = input.trim().to_string();
                handle_command(&input, &conn_mgr, &edge_ctrl, windows_ip);
            }
            Err(_) => break,
        }
    }
}

/// 处理命令行命令
fn handle_command(
    input: &str,
    conn_mgr: &Arc<ConnectionManager>,
    edge_ctrl: &Arc<Mutex<EdgeController>>,
    windows_ip: &'static str,
) {
    if input.starts_with("c ") {
        // c <IP> - 切换为主控端并连接指定 IP
        let ip = input[2..].trim().to_string();
        println!("[Tray] 切换为主控端，连接: {}", ip);
        conn_mgr.switch_to_controller(Some(ip));

    } else if input == "c" {
        // c - 切换为主控端，使用默认 Windows IP
        println!("[Tray] 切换为主控端，连接默认 Windows IP: {}", windows_ip);
        conn_mgr.switch_to_controller(Some(windows_ip.to_string()));

    } else if input == "s" {
        // s - 切换为被控端
        println!("[Tray] 切换为被控端");
        conn_mgr.switch_to_controlled();

    } else if input == "toggle" || input == "t" {
        // toggle - 切换角色
        match conn_mgr.role() {
            ConnectionRole::Controlled => {
                println!("[Tray] 切换为主控端，连接: {}", windows_ip);
                conn_mgr.switch_to_controller(Some(windows_ip.to_string()));
            }
            ConnectionRole::Controller => {
                println!("[Tray] 切换为被控端");
                conn_mgr.switch_to_controlled();
            }
        }

    } else if input == "status" {
        // status - 显示当前状态
        print_status(conn_mgr);

    } else if input.starts_with("edge ") {
        // edge <left|right|top|bottom> - 设置触发边缘
        let dir_str = input[5..].trim();
        let dir = match dir_str {
            "left" => Some(EdgeDirection::Left),
            "right" => Some(EdgeDirection::Right),
            "top" => Some(EdgeDirection::Top),
            "bottom" => Some(EdgeDirection::Bottom),
            _ => None,
        };
        if let Some(d) = dir {
            edge_ctrl.lock().unwrap().set_config(EdgeConfig {
                trigger_edges: vec![d],
                threshold: 1,
                debounce_ms: 50,
            });
            println!("[Tray] 边缘触发方向已设置为: {:?}", d);
        } else {
            println!("[Tray] 未知方向: {}（可选: left/right/top/bottom）", dir_str);
        }

    } else if input == "help" || input == "?" {
        print_help();

    } else if !input.is_empty() {
        println!("[Tray] 未知命令，输入 help 查看帮助");
    }
}

/// 打印帮助信息
fn print_help() {
    println!("────────────────────────────────────────");
    println!("  KVM Agent 命令行控制");
    println!("────────────────────────────────────────");
    println!("  c [IP]          切换为主控端（可选指定 IP）");
    println!("  s               切换为被控端");
    println!("  t / toggle      切换角色");
    println!("  status          显示当前状态");
    println!("  edge <方向>     设置触发边缘（left/right/top/bottom）");
    println!("  help / ?        显示帮助");
    println!("────────────────────────────────────────");
    println!("  快捷键: Ctrl+Alt+K 切换主控/被控角色");
    println!("────────────────────────────────────────");
}

/// 打印当前状态
fn print_status(conn_mgr: &Arc<ConnectionManager>) {
    let role = conn_mgr.role();
    let peer_ip = conn_mgr.peer_ip();
    let role_str = match role {
        ConnectionRole::Controlled => "被控端（等待 Windows 连接）",
        ConnectionRole::Controller => "主控端（控制 Windows）",
    };
    println!("[状态] 角色: {}", role_str);
    println!("[状态] 对端 IP: {}", peer_ip);
}

/// 状态打印主循环
fn run_status_loop(conn_mgr: Arc<ConnectionManager>) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let role = conn_mgr.role();
        let role_str = match role {
            ConnectionRole::Controlled => "被控端",
            ConnectionRole::Controller => "主控端",
        };
        let peer_ip = conn_mgr.peer_ip();
        println!("[Tray] [{}] 对端: {}", role_str, peer_ip);
    }
}

// ── 兼容旧接口（供 main.rs 中旧代码过渡使用）──────────────────────────────

/// 托盘控制器（旧接口，保留兼容性）
pub struct TrayController {
    state: Arc<Mutex<crate::server::ConnectionState>>,
    role_manager: Arc<crate::role_manager::RoleManager>,
}

impl TrayController {
    pub fn new(
        state: Arc<Mutex<crate::server::ConnectionState>>,
        role_manager: Arc<crate::role_manager::RoleManager>,
    ) -> Self {
        Self { state, role_manager }
    }

    pub fn run(&self) {
        println!("[Tray] 旧接口 TrayController::run() 已废弃，请使用 run_tray()");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}

/// 发送 macOS 系统通知
#[cfg(target_os = "macos")]
pub fn send_notification(title: &str, message: &str) {
    let script = format!(
        r#"display notification "{}" with title "{}""#,
        message.replace('"', "\\\""),
        title.replace('"', "\\\"")
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn send_notification(title: &str, message: &str) {
    println!("[通知] {}: {}", title, message);
}
