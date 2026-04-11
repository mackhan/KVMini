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

/// 网卡信息结构体
#[derive(serde::Serialize, Clone)]
struct NetworkInterface {
    name: String,
    ip: String,
    is_up: bool,
}

/// 获取本机所有活跃网卡（名称 + IP + 状态）
#[tauri::command]
fn get_network_interfaces() -> Vec<NetworkInterface> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .filter_map(|iface| {
            let ip = iface.ip().to_string();
            // 只返回 IPv4 私有地址
            if ip.starts_with("192.") || ip.starts_with("10.") || ip.starts_with("172.") {
                Some(NetworkInterface {
                    name: iface.name.clone(),
                    ip,
                    is_up: true, // if_addrs 只返回活跃接口
                })
            } else {
                None
            }
        })
        .collect()
}

// ── TLS 证书相关 ──────────────────────────────────────────────────────────────

/// 获取 TLS 证书存储路径（跨平台）
fn get_cert_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("KVMini")
        .join("certs")
}

/// 生成自签名 TLS 证书，返回 DER 格式的证书字节
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), String> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "KVMini");
    dn.push(DnType::OrganizationName, "KVMini");
    params.distinguished_name = dn;

    let key_pair = KeyPair::generate().map_err(|e| format!("生成密钥对失败: {}", e))?;
    let cert = params.self_signed(&key_pair).map_err(|e| format!("生成证书失败: {}", e))?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    Ok((cert_der, key_der))
}

/// 计算证书 DER 字节的 SHA-256 指纹（格式：XX:XX:XX:...）
fn compute_cert_fingerprint(cert_der: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(cert_der);
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// 加载或生成本机 TLS 证书，返回 SHA-256 指纹
#[tauri::command]
fn get_local_tls_fingerprint() -> Result<String, String> {
    let cert_dir = get_cert_dir();
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");

    // 尝试加载已有证书
    if cert_path.exists() && key_path.exists() {
        match std::fs::read(&cert_path) {
            Ok(cert_der) => {
                log::info!("[TLS] 加载已有证书: {:?}", cert_path);
                return Ok(compute_cert_fingerprint(&cert_der));
            }
            Err(e) => {
                // 证书文件损坏，删除并重新生成
                log::warn!("[TLS] 证书文件损坏，重新生成: {}", e);
                let _ = std::fs::remove_file(&cert_path);
                let _ = std::fs::remove_file(&key_path);
            }
        }
    }

    // 生成新证书
    log::info!("[TLS] 生成新的自签名证书");
    std::fs::create_dir_all(&cert_dir)
        .map_err(|e| format!("创建证书目录失败: {}", e))?;

    let (cert_der, key_der) = generate_self_signed_cert()?;

    std::fs::write(&cert_path, &cert_der)
        .map_err(|e| format!("保存证书失败: {}", e))?;
    std::fs::write(&key_path, &key_der)
        .map_err(|e| format!("保存密钥失败: {}", e))?;

    log::info!("[TLS] 新证书已生成并保存: {:?}", cert_path);
    Ok(compute_cert_fingerprint(&cert_der))
}

/// 重新生成 TLS 证书（删除旧证书，生成新证书），返回新指纹
#[tauri::command]
fn regenerate_tls_cert() -> Result<String, String> {
    let cert_dir = get_cert_dir();
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");

    // 删除旧证书
    if cert_path.exists() { let _ = std::fs::remove_file(&cert_path); }
    if key_path.exists() { let _ = std::fs::remove_file(&key_path); }

    log::info!("[TLS] 重新生成证书");
    std::fs::create_dir_all(&cert_dir)
        .map_err(|e| format!("创建证书目录失败: {}", e))?;

    let (cert_der, key_der) = generate_self_signed_cert()?;

    std::fs::write(&cert_path, &cert_der)
        .map_err(|e| format!("保存证书失败: {}", e))?;
    std::fs::write(&key_path, &key_der)
        .map_err(|e| format!("保存密钥失败: {}", e))?;

    log::info!("[TLS] 证书已重新生成: {:?}", cert_path);
    Ok(compute_cert_fingerprint(&cert_der))
}

/// 扫描局域网内的 KVMini 设备（UDP 广播，等待 5 秒）
#[tauri::command]
async fn discover_devices() -> Result<Vec<kvm_core::network::DiscoveredDevice>, String> {
    kvm_core::network::discover_devices(kvm_core::network::DEFAULT_UDP_PORT)
        .await
        .map_err(|e| e.to_string())
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

/// 写入启动诊断日志到 ~/Library/Logs/KVMini/startup.log（macOS 专属）
/// 非 macOS 平台直接返回 Ok
#[tauri::command]
fn write_startup_log(content: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::fs;
        use std::path::PathBuf;

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_dir = PathBuf::from(&home).join("Library").join("Logs").join("KVMini");

        // 自动创建目录
        fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;

        let log_path = log_dir.join("startup.log");

        // 追加写入
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("打开日志文件失败: {}", e))?;

        file.write_all(content.as_bytes())
            .map_err(|e| format!("写入日志失败: {}", e))?;

        log::info!("[Diagnostics] 诊断日志已写入: {:?}", log_path);
    }
    Ok(())
}

// ── 授权管理 ──────────────────────────────────────────────────────────────

/// 授权检查结果
#[derive(serde::Serialize)]
struct LicenseCheckResult {
    licensed: bool,
    status: String,
    message: Option<String>,
}

/// 激活结果
#[derive(serde::Serialize)]
struct LicenseActivateResult {
    success: bool,
    message: Option<String>,
}

/// 检查授权状态
/// 当前实现：从本地配置文件读取 license key 并验证
/// 待 tauri-commerce SDK (https://github.com/mackhan/tauri-commerce) 可用后替换实现
#[tauri::command]
fn check_license_status(app: tauri::AppHandle) -> LicenseCheckResult {
    let data_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let license_path = data_dir.join("license.key");

    if license_path.exists() {
        match std::fs::read_to_string(&license_path) {
            Ok(key) if !key.trim().is_empty() => {
                log::info!("[License] 找到本地授权文件");
                LicenseCheckResult {
                    licensed: true,
                    status: "licensed".to_string(),
                    message: None,
                }
            }
            _ => {
                log::info!("[License] 授权文件为空");
                LicenseCheckResult {
                    licensed: false,
                    status: "unlicensed".to_string(),
                    message: None,
                }
            }
        }
    } else {
        log::info!("[License] 未找到授权文件");
        LicenseCheckResult {
            licensed: false,
            status: "unlicensed".to_string(),
            message: None,
        }
    }
}

/// 激活 License Key
/// 当前实现：将 key 写入本地文件（简单验证格式）
/// 待 tauri-commerce SDK 可用后替换为在线验证
#[tauri::command]
fn activate_license(app: tauri::AppHandle, key: String) -> LicenseActivateResult {
    let key = key.trim().to_string();
    if key.is_empty() {
        return LicenseActivateResult {
            success: false,
            message: Some("授权码不能为空".to_string()),
        };
    }

    // 简单格式验证：XXXX-XXXX-XXXX-XXXX
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 4 || parts.iter().any(|p| p.len() != 4) {
        return LicenseActivateResult {
            success: false,
            message: Some("授权码格式无效，请使用 XXXX-XXXX-XXXX-XXXX 格式".to_string()),
        };
    }

    let data_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        log::error!("[License] 创建数据目录失败: {}", e);
        return LicenseActivateResult {
            success: false,
            message: Some(format!("激活失败: {}", e)),
        };
    }

    let license_path = data_dir.join("license.key");
    match std::fs::write(&license_path, &key) {
        Ok(_) => {
            log::info!("[License] 授权码已保存");
            LicenseActivateResult {
                success: true,
                message: None,
            }
        }
        Err(e) => {
            log::error!("[License] 保存授权码失败: {}", e);
            LicenseActivateResult {
                success: false,
                message: Some(format!("激活失败: {}", e)),
            }
        }
    }
}

// ── 全局快捷键管理 ────────────────────────────────────────────────────────────

/// 全局快捷键状态（通过 tauri::State 管理）
pub struct HotkeyState {
    /// 当前注册的快捷键字符串（空字符串表示未注册）
    current_hotkey: std::sync::Mutex<String>,
    /// 平台快捷键管理器
    #[cfg(target_os = "macos")]
    manager: platform::macos::hotkey::MacHotkeyManager,
    #[cfg(target_os = "windows")]
    manager: platform::windows::hotkey::WinHotkeyManager,
}

impl HotkeyState {
    pub fn new() -> Self {
        Self {
            current_hotkey: std::sync::Mutex::new(String::new()),
            #[cfg(target_os = "macos")]
            manager: platform::macos::hotkey::MacHotkeyManager::new(),
            #[cfg(target_os = "windows")]
            manager: platform::windows::hotkey::WinHotkeyManager::new(),
        }
    }
}

/// 注册全局快捷键
/// - hotkey: 快捷键字符串，如 "Ctrl+Alt+S"；空字符串表示注销当前快捷键
#[tauri::command]
fn register_global_hotkey(
    hotkey: String,
    state: tauri::State<HotkeyState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use platform::HotkeyManager;

    let mut current = state.current_hotkey.lock().unwrap();

    // 先注销旧的快捷键
    if !current.is_empty() {
        if let Err(e) = state.manager.unregister_hotkey(&current) {
            log::warn!("[Hotkey] 注销旧快捷键 {} 失败: {}", current, e);
        }
        log::info!("[Hotkey] 已注销旧快捷键: {}", current);
    }

    // 如果新快捷键为空，只注销不注册
    if hotkey.trim().is_empty() {
        *current = String::new();
        log::info!("[Hotkey] 快捷键已清空");
        return Ok(());
    }

    // 注册新快捷键，触发时向前端发送 hotkey-triggered 事件
    let app_clone = app.clone();
    let hotkey_clone = hotkey.clone();
    let result = state.manager.register_hotkey(
        &hotkey,
        Box::new(move || {
            log::info!("[Hotkey] 快捷键触发: {}", hotkey_clone);
            let _ = app_clone.emit("hotkey-triggered", ());
        }),
    );

    match result {
        Ok(_) => {
            *current = hotkey.clone();
            log::info!("[Hotkey] 全局快捷键已注册: {}", hotkey);
            Ok(())
        }
        Err(e) => {
            log::error!("[Hotkey] 注册快捷键 {} 失败: {}", hotkey, e);
            Err(format!("{}", e))
        }
    }
}

/// 注销当前全局快捷键
#[tauri::command]
fn unregister_global_hotkey(state: tauri::State<HotkeyState>) -> Result<(), String> {
    use platform::HotkeyManager;

    let mut current = state.current_hotkey.lock().unwrap();
    if current.is_empty() {
        return Ok(());
    }

    if let Err(e) = state.manager.unregister_hotkey(&current) {
        log::warn!("[Hotkey] 注销快捷键 {} 失败: {}", current, e);
        return Err(format!("{}", e));
    }

    log::info!("[Hotkey] 全局快捷键已注销: {}", current);
    *current = String::new();
    Ok(())
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
        tls_mismatch: false,
        local_tls: false,
        remote_tls: false,
    });
}

/// 检查 macOS 辅助功能权限，并通知前端，权限不足时启动后台轮询
fn check_macos_permission(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use platform::macos::permission::{check_accessibility, AccessibilityStatus};
        let status = check_accessibility();
        log::info!("[Mac] 辅助功能权限状态: {:?}", status);

        if status == AccessibilityStatus::Granted {
            // 已授权：通知前端隐藏遮罩
            let _ = app.emit("accessibility://permission-granted", ());
            log::info!("[Mac] 辅助功能权限已授权，通知前端");
        } else {
            // 未授权：通知前端显示引导遮罩，并启动后台轮询
            log::warn!("[Mac] 辅助功能权限未授权，通知前端引导用户");
            let _ = app.emit("accessibility://permission-denied", ());
            start_accessibility_polling(app.clone());
        }
    }
    // 非 macOS 平台无需权限检查
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// 启动后台权限轮询（每 2 秒检测一次，授权后发送 granted 事件并停止）
#[cfg(target_os = "macos")]
fn start_accessibility_polling(app: tauri::AppHandle) {
    use platform::macos::permission::{check_accessibility, AccessibilityStatus};
    use std::thread;
    use std::time::Duration;

    thread::spawn(move || {
        log::info!("[Mac] 启动辅助功能权限轮询（每 2 秒）");
        loop {
            thread::sleep(Duration::from_secs(2));
            match check_accessibility() {
                AccessibilityStatus::Granted => {
                    log::info!("[Mac] 轮询检测到权限已授权，通知前端");
                    let _ = app.emit("accessibility://permission-granted", ());
                    break;
                }
                _ => {
                    // 继续轮询
                }
            }
        }
        log::info!("[Mac] 辅助功能权限轮询已停止");
    });
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
            get_network_interfaces,
            get_local_tls_fingerprint,
            regenerate_tls_cert,
            get_platform,
            discover_devices,
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
            // 诊断日志
            write_startup_log,
            // 授权管理
            check_license_status,
            activate_license,
            // 全局快捷键
            register_global_hotkey,
            unregister_global_hotkey,
        ])
        // 托管连接管理器状态
        .manage(connection::KvmConnectionManager::new())
        // 托管全局快捷键状态
        .manage(HotkeyState::new())
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

            // 启动时自动注册已保存的全局快捷键
            {
                let config = config::load_config();
                let saved_hotkey = config.switch_hotkey.clone();
                if !saved_hotkey.is_empty() {
                    let hotkey_state = app_handle.state::<HotkeyState>();
                    let app_for_hotkey = app_handle.clone();
                    match register_global_hotkey(saved_hotkey.clone(), hotkey_state, app_for_hotkey) {
                        Ok(_) => log::info!("[Hotkey] 启动时自动注册快捷键: {}", saved_hotkey),
                        Err(e) => log::warn!("[Hotkey] 启动时注册快捷键失败: {}", e),
                    }
                }
            }

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
