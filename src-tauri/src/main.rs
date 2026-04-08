#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![allow(dead_code, unused_imports, unused_variables, unused_doc_comments)]

use tauri::{Manager, SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem};
use tauri::GlobalShortcutManager;

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
mod handshake;
mod keycode_map;
mod clipboard;
mod tls;
mod monitor;
// 新模块结构
mod connection;
mod control;
#[cfg(test)]
mod integration_tests;

use connection_manager::{discover_devices, connect_to_device, disconnect_devices, get_connection_status};
use connection::manager::{connect_to_mac, disconnect_from_mac, reconnect_to_peer, get_win_connection_status, get_default_ips};
use control::edge::{get_edge_config, set_edge_config};
use control::hotkey::{get_hotkey_config, check_hotkey_available};
use network::{discover_network_devices, connect_network_device};
use usb::{scan_usb_devices, connect_usb_device};
use platform::{
    get_settings, save_settings,
    get_connection_history, save_connection_history, clear_connection_history,
    show_tray_notification, register_global_hotkey, get_latency, kill_legacy_process,
    get_local_ip, connect_by_ip, get_network_interfaces, set_listen_address, get_listen_config,
    disconnect_tcp, get_tcp_connection_status, start_heartbeat_monitor,
    check_hotkey_conflict, update_tray_language, update_tray_connection_state,
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
            disconnect_tcp,
            get_tcp_connection_status,
            start_heartbeat_monitor,
            // 显示器布局配置
            layout_config::get_layout_config,
            layout_config::set_layout_config,
            // 角色管理
            role_manager::get_current_role,
            role_manager::set_role,
            // 辅助功能权限
            accessibility::check_accessibility,
            accessibility::open_accessibility_prefs,
            // 快捷键冲突检测
            check_hotkey_conflict,
            // 托盘菜单国际化
            update_tray_language,
            update_tray_connection_state,
            // TLS 证书管理
            tls::get_local_tls_fingerprint,
            tls::trust_peer_certificate,
            // 多显示器支持
            monitor::get_monitors,
            monitor::get_virtual_desktop_info,
            // 连接模块（新）
            connect_to_mac,
            disconnect_from_mac,
            reconnect_to_peer,
            get_win_connection_status,
            get_default_ips,
            // 控制模块（新）
            get_edge_config,
            set_edge_config,
            get_hotkey_config,
            check_hotkey_available,
        ])
        .manage(std::sync::Mutex::new(role_manager::AppRole::default()))
        .manage(connection::manager::WinConnectionManager::new())
        .setup(|app| {
            // 启动时清理遗留进程
            let _ = platform::kill_legacy_process();

            // 通知前端初始连接状态
            let app_handle = app.handle();
            let _ = app_handle.emit_all("connection-status", serde_json::json!({
                "event_type": "disconnected",
                "connected": false,
                "peer_ip": null,
                "message": "未连接",
                "attempt": null,
                "guide": null
            }));
            log::info!("KVM Share 启动，等待连接");

            // 启动被控端 UDP 广播监听（所有设备启动时都监听，主控端扫描时能发现）
            platform::start_udp_discovery_listener();

            // 启动显示器变化监听（每 3 秒检测一次，变化时通知前端）
            monitor::start_monitor_watcher(app.handle());

            // 注册已保存的全局快捷键
            if let Ok(settings) = platform::get_settings() {
                if !settings.hotkey.is_empty() {
                    let app_handle = app.handle();
                    let hotkey = settings.hotkey.clone();
                    match app.global_shortcut_manager().register(&hotkey, move || {
                        let _ = app_handle.emit_all("hotkey-triggered", ());
                    }) {
                        Ok(_) => log::info!("全局快捷键已注册: {}", hotkey),
                        Err(e) => log::warn!("全局快捷键注册失败: {} ({})", hotkey, e),
                    }
                }
            }

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