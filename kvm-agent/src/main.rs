mod protocol;
mod discovery;
mod server;
mod input_executor;
mod tray;

use std::sync::{Arc, Mutex};
use server::{ConnectionState, SharedState};

fn main() {
    println!("========================================");
    println!("  KVM Agent v0.1.0 - Mac 端被控程序");
    println!("========================================");

    // 检测辅助功能权限
    if !input_executor::check_accessibility_permission() {
        input_executor::print_accessibility_guide();
        eprintln!("⚠️  未获得辅助功能权限，鼠标/键盘控制将不可用");
        eprintln!("   授权后请重新启动 kvm-agent");
        // 不退出，允许程序继续运行（仍可接受连接，但无法执行输入）
    }

    // 获取本机信息
    let hostname = discovery::get_hostname();
    let os_version = discovery::get_os_version();
    println!("[主程序] 主机名: {}", hostname);
    println!("[主程序] 系统版本: {}", os_version);

    // 创建共享连接状态
    let state: SharedState = Arc::new(Mutex::new(ConnectionState::default()));

    // 启动 UDP 发现监听线程
    {
        let hn = hostname.clone();
        let os = os_version.clone();
        std::thread::spawn(move || {
            if let Err(e) = discovery::run_udp_listener(hn, os) {
                eprintln!("[UDP] 监听失败: {}", e);
            }
        });
    }

    // 启动 TCP 服务端线程
    {
        let hn = hostname.clone();
        let os = os_version.clone();
        let state_clone = Arc::clone(&state);
        std::thread::spawn(move || {
            loop {
                if let Err(e) = server::run_tcp_server(
                    hn.clone(),
                    os.clone(),
                    Arc::clone(&state_clone),
                    |event| {
                        input_executor::execute_input_event(&event);
                    },
                ) {
                    eprintln!("[TCP] 服务端错误: {}，5 秒后重试...", e);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }
        });
    }

    // 主线程运行托盘（macOS 要求 UI 在主线程）
    let tray = tray::TrayController::new(Arc::clone(&state));
    tray.run();
}
