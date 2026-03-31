#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;

mod types;
mod network;
mod usb;
mod connection_manager;

use connection_manager::{discover_devices, connect_to_device, disconnect_devices, get_connection_status};
use network::{discover_network_devices, connect_network_device};
use usb::{scan_usb_devices, connect_usb_device};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            discover_devices,
            connect_to_device,
            disconnect_devices,
            get_connection_status,
            discover_network_devices,
            connect_network_device,
            scan_usb_devices,
            connect_usb_device
        ])
        .setup(|app| {
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