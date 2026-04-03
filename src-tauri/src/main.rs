#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{Manager, SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem};

mod types;
mod protocol;
mod server;
mod client;
mod injector;
mod capturer;
mod edge_switch;
mod layout_config;
mod role_manager;
mod accessibility;
mod network;
mod usb;
mod connection_manager;
mod platform;
#[cfg(test)]
mod integration_tests;

use connection_manager::{discover_devices, connect_to_device, disconnect_devices, get_connection_status};
use network::{discover_network_devices, connect_network_device};
use usb::{scan_usb_devices, connect_usb_device};
use platform::{
    get_settings, save_settings,
    get_connection_history, save_connection_history, clear_connection_history,
    show_tray_notification, register_global_hotkey, get_latency, kill_legacy_process,
    get_local_ip, connect_by_ip, get_network_interfaces, set_listen_address, get_listen_config,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 构建系统托盘菜单（两端统一）
fn build_tray_menu() -> SystemTrayMenu {
    let show = CustomMenuItem::new("show".to_string(), "显示窗口");
    let connect = CustomMenuItem::new("connect".to_string(), "连接设备");
    let disconnect = CustomMenuItem::new("disconnect".to_string(), "断开连接");
    let quit = CustomMenuItem::new("quit".to_string(), "退出");

    SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(connect)
        .add_item(disconnect)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit)
}

fn main() {
    let tray_menu = build_tray_menu();
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            use tauri::{SystemTrayEvent};
            match event {
                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "show" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "connect" => {
                        // 通过事件通知前端触发连接
                        let _ = app.emit_all("tray-connect", ());
                    }
                    "disconnect" => {
                        let _ = app.emit_all("tray-disconnect", ());
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                },
                SystemTrayEvent::LeftClick { .. } => {
                    // 左键点击托盘图标：显示/隐藏窗口
                    if let Some(window) = app.get_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            // 设备发现与连接
            discover_devices,
            connect_to_device,
            disconnect_devices,
            get_connection_status,
            discover_network_devices,
            connect_network_device,
            scan_usb_devices,
            connect_usb_device,
            // 平台功能（两端统一接口）
            get_settings,
            save_settings,
            get_connection_history,
            save_connection_history,
            clear_connection_history,
            show_tray_notification,
            register_global_hotkey,
            get_latency,
            kill_legacy_process,
            get_local_ip,
            // 跨网段直连
            connect_by_ip,
            get_network_interfaces,
            set_listen_address,
            get_listen_config,
            // 显示器布局配置
            layout_config::get_layout_config,
            layout_config::set_layout_config,
            // 角色管理
            role_manager::get_current_role,
            role_manager::set_role,
            // 辅助功能权限
            accessibility::check_accessibility,
            accessibility::open_accessibility_prefs,
        ])
        .manage(std::sync::Mutex::new(role_manager::AppRole::default()))
        .setup(|app| {
            // 启动时清理遗留进程
            let _ = platform::kill_legacy_process();

            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}