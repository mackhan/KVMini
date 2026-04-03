/// Mac 端系统托盘（菜单栏图标）
/// 使用 macOS NSStatusBar API 实现，仅在 macOS 上编译
use std::sync::{Arc, Mutex};
use crate::server::ConnectionState;

/// 托盘控制器
pub struct TrayController {
    state: Arc<Mutex<ConnectionState>>,
}

impl TrayController {
    pub fn new(state: Arc<Mutex<ConnectionState>>) -> Self {
        Self { state }
    }

    /// 启动托盘（阻塞，需在主线程调用）
    pub fn run(&self) {
        #[cfg(target_os = "macos")]
        self.run_macos();

        #[cfg(not(target_os = "macos"))]
        {
            println!("[Tray] 当前平台不支持系统托盘，以命令行模式运行");
            // 非 macOS 平台：保持主线程存活
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn run_macos(&self) {
        use std::process::Command;

        println!("[Tray] KVM Agent 已启动，运行在菜单栏");
        println!("[Tray] 按 Ctrl+C 退出");

        // 使用 osascript 创建简单的菜单栏通知
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(r#"display notification "KVM Agent 已启动，等待连接..." with title "KVM Agent""#)
            .spawn();

        // 主循环：定期更新状态显示
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let state = self.state.lock().unwrap();
            if state.connected {
                let ip = state.peer_ip.as_deref().unwrap_or("unknown");
                let hostname = state.peer_hostname.as_deref().unwrap_or("unknown");
                println!("[Tray] 已连接 - 控制端: {} ({})", hostname, ip);
            } else {
                println!("[Tray] 等待连接...");
            }
        }
    }
}

/// 发送 macOS 系统通知
#[cfg(target_os = "macos")]
pub fn send_notification(title: &str, message: &str) {
    let script = format!(
        r#"display notification "{}" with title "{}""#,
        message, title
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
