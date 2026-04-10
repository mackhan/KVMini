#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

// ── 模块声明 ──────────────────────────────────────────────────────────────
#[macro_use]
mod macros;
mod config;
mod connection;
mod platform;

// ── 通用 Tauri 命令 ────────────────────────────────────────────────────────

/// 获取应用版本号（从 Cargo.toml 读取，两端保持一致）
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 获取本机所有局域网 IP 列表
#[tauri::command]
fn get_local_ip() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .map(|iface| iface.ip().to_string())
        .filter(|ip| ip.starts_with("192.") || ip.starts_with("10.") || ip.starts_with("172."))
        .collect()
}

/// 获取当前平台名称
#[tauri::command]
fn get_platform() -> String {
    platform::platform_name().to_string()
}

/// 检查 macOS 辅助功能权限
#[tauri::command]
fn check_accessibility_permission() -> String {
    #[cfg(target_os = "macos")]
    {
        let status = platform::macos::permission::check_accessibility();
        return status.to_string();
    }
    #[cfg(not(target_os = "macos"))]
    "N/A".to_string()
}

/// 请求 macOS 辅助功能权限（弹出系统对话框）
#[tauri::command]
fn request_accessibility_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { platform::macos::permission::request_accessibility().map_err(|e| e.to_string())?; }
    Ok(())
}

/// 打开 macOS 辅助功能设置页面
#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { platform::macos::permission::open_accessibility_settings().map_err(|e| e.to_string())?; }
    Ok(())
}

/// 获取自启动状态
#[tauri::command]
fn get_autostart_status() -> bool {
    platform::autostart::is_autostart_enabled()
}

/// 设置自启动
#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        platform::autostart::enable_autostart()
    } else {
        platform::autostart::disable_autostart()
    }
}

// ── 托盘构建 ──────────────────────────────────────────────────────────────

/// 构建系统托盘菜单和图标
fn build_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show       = MenuItem::with_id(app, "show",       "显示窗口", true, None::<&str>)?;
    let connect    = MenuItem::with_id(app, "connect",    "连接设备", true, None::<&str>)?;
    let disconnect = MenuItem::with_id(app, "disconnect", "断开连接", true, None::<&str>)?;
    let separator  = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit       = MenuItem::with_id(app, "quit",       "退出",     true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &show,
        &separator,
        &connect,
        &disconnect,
        &separator2,
        &quit,
    ])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "connect"    => { let _ = app.emit("tray-connect", ()); }
            "disconnect" => { let _ = app.emit("tray-disconnect", ()); }
            "quit"       => { std::process::exit(0); }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// 推送初始连接状态到前端
fn emit_initial_status(app: &tauri::AppHandle) {
    let _ = app.emit("connection-status", connection::ConnectionEvent {
        event_type: "disconnected".to_string(),
        connected: false,
        peer_ip: None,
        message: "未连接".to_string(),
        attempt: None,
        guide: None,
    });
}

/// 检查 macOS 辅助功能权限，未授权时通知前端引导用户
fn check_macos_permission(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use platform::macos::permission::{check_accessibility, AccessibilityStatus};
        if check_accessibility() != AccessibilityStatus::Granted {
            log::warn!("[Mac] 辅助功能权限未授权，通知前端引导用户");
            let _ = app.emit("accessibility-required", serde_json::json!({
                "message": "KVMini 需要辅助功能权限才能采集和注入键鼠事件",
                "guide": [
                    "打开「系统设置」→「隐私与安全性」→「辅助功能」",
                    "找到 KVMini 并开启权限",
                    "重启 KVMini 使权限生效"
                ]
            }));
        }
    }
    // 非 macOS 平台无需权限检查
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

// ── 主函数 ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 调试：捕获 panic 消息
    #[cfg(debug_assertions)]
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            eprintln!("[PANIC HOOK] panic info: {}", info);
            if let Some(location) = info.location() {
                eprintln!("[PANIC HOOK] location: {}:{}:{}", location.file(), location.line(), location.column());
            }
            if let Some(msg) = info.payload().downcast_ref::<&str>() {
                eprintln!("[PANIC HOOK] message: {}", msg);
            } else if let Some(msg) = info.payload().downcast_ref::<String>() {
                eprintln!("[PANIC HOOK] message: {}", msg);
            }
            default_hook(info);
        }));
    }

    tauri::Builder::default()
        // 注册 Tauri 2.x 插件
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // 注册所有 Tauri 命令
        .invoke_handler(tauri::generate_handler![
            // 通用
            get_app_version,
            get_local_ip,
            get_platform,
            // 连接管理
            connection::start_listening,
            connection::connect_to_peer,
            connection::disconnect_peer,
            connection::get_connection_status,
            // 配置
            config::get_settings,
            config::save_settings,
            config::reset_settings,
            // 自启动
            get_autostart_status,
            set_autostart,
            // macOS 权限
            check_accessibility_permission,
            request_accessibility_permission,
            open_accessibility_settings,
        ])
        // 托管连接管理器状态
        .manage(connection::KvmConnectionManager::new())
        .setup(|app| {
            // 构建系统托盘
            build_tray(app)?;

            let app_handle = app.handle().clone();

            // 启动连接事件监听循环
            {
                let mgr = app_handle.state::<connection::KvmConnectionManager>();
                mgr.start_event_loop(app_handle.clone());
            }

            // 推送初始连接状态
            emit_initial_status(&app_handle);

            // macOS：检查辅助功能权限，未授权时引导用户
            check_macos_permission(&app_handle);

            log::info!("KVMini v{} 已启动（{}）",
                env!("CARGO_PKG_VERSION"),
                platform::platform_name()
            );

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("KVMini 启动失败");
}