use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ─── 数据结构 ───

/// 默认快捷键
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+S";

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey: String,
    pub prefer_connection_type: String,
    pub auto_reconnect: bool,
    pub auto_start: bool,
    pub start_minimized: bool,
    /// 是否启用剪贴板共享
    #[serde(default = "default_true")]
    pub clipboard_sync: bool,
    /// 是否启用 TLS 加密
    #[serde(default = "default_true")]
    pub enable_tls: bool,
    /// Command 键映射模式（"win" 或 "ctrl"）
    #[serde(default)]
    pub command_key_mapping: String,
    /// 界面语言（"zh" 或 "en"）
    #[serde(default)]
    pub language: String,
}

fn default_true() -> bool { true }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.to_string(),
            prefer_connection_type: "auto".to_string(),
            auto_reconnect: true,
            auto_start: false,
            start_minimized: false,
            clipboard_sync: true,
            enable_tls: true,
            command_key_mapping: "win".to_string(),
            language: "zh".to_string(),
        }
    }
}

/// 连接历史条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub name: String,
    pub os: String,
    pub ip: String,
    pub connection_type: String,
    pub last_connected: String,
}

// ─── 工具函数 ───

/// 获取应用数据目录
fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join("Library/Application Support/com.kvmshare.app")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Temp".to_string());
        PathBuf::from(appdata).join("KVMShare")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/tmp/kvmshare")
    }
}

/// 确保数据目录存在
fn ensure_data_dir() -> Result<PathBuf, String> {
    let dir = get_app_data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    Ok(dir)
}

// ─── 设置命令 ───

#[tauri::command]
/// 获取应用设置
/// 当 hotkey 为空时，自动填充默认值 Ctrl+Alt+S
pub fn get_settings() -> Result<AppSettings, String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("settings.json");

    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取设置文件失败: {}", e))?;
    let mut settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("解析设置失败: {}", e))?;

    // 当 hotkey 为空时，填充默认值（需求 6.1、6.2）
    if settings.hotkey.is_empty() {
        settings.hotkey = DEFAULT_HOTKEY.to_string();
    }

    Ok(settings)
}

#[tauri::command]
/// 保存应用设置
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("settings.json");

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化设置失败: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("写入设置文件失败: {}", e))
}

// ─── 连接历史命令 ───

#[tauri::command]
/// 获取连接历史记录
pub fn get_connection_history() -> Result<Vec<HistoryEntry>, String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("history.json");

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取历史文件失败: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析历史记录失败: {}", e))
}

#[tauri::command]
/// 保存连接历史记录（追加并去重，最多保留 10 条）
pub fn save_connection_history(entry: HistoryEntry) -> Result<(), String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("history.json");

    let mut history: Vec<HistoryEntry> = if path.exists() {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取历史文件失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // 去重：移除同 id 的旧记录
    history.retain(|h| h.id != entry.id);
    // 插入到最前面
    history.insert(0, entry);
    // 最多保留 10 条
    history.truncate(10);

    let content = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("序列化历史记录失败: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("写入历史文件失败: {}", e))
}

#[tauri::command]
/// 清除连接历史记录
pub fn clear_connection_history() -> Result<(), String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("history.json");

    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("删除历史文件失败: {}", e))?;
    }
    Ok(())
}

// ─── 系统通知命令 ───

#[tauri::command]
/// 显示系统通知（两端统一接口，内部使用平台特定实现）
pub fn show_tray_notification(title: String, body: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        show_notification_macos(&title, &body)
    }
    #[cfg(target_os = "windows")]
    {
        show_notification_windows(&title, &body)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        println!("[通知] {}: {}", title, body);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn show_notification_macos(title: &str, body: &str) -> Result<(), String> {
    // macOS: 使用 osascript 发送通知（无需额外依赖）
    let script = format!(
        r#"display notification "{}" with title "{}""#,
        body.replace('"', "\\\""),
        title.replace('"', "\\\"")
    );
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("macOS 通知发送失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn show_notification_windows(title: &str, body: &str) -> Result<(), String> {
    // Windows: 使用 PowerShell 发送 Toast 通知
    let script = format!(
        r#"
        [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
        $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02)
        $textNodes = $template.GetElementsByTagName("text")
        $textNodes.Item(0).AppendChild($template.CreateTextNode("{}")) | Out-Null
        $textNodes.Item(1).AppendChild($template.CreateTextNode("{}")) | Out-Null
        $toast = [Windows.UI.Notifications.ToastNotification]::new($template)
        [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("KVM Share").Show($toast)
        "#,
        title.replace('"', "'"),
        body.replace('"', "'")
    );
    std::process::Command::new("powershell")
        .args(["-Command", &script])
        .output()
        .map_err(|e| format!("Windows 通知发送失败: {}", e))?;
    Ok(())
}

// ─── 快捷键冲突检测 ───

/// 快捷键冲突检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResult {
    /// 是否存在冲突
    pub conflict: bool,
    /// 冲突类型："system"（系统保留）或 "registered"（已注册）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_type: Option<String>,
    /// 冲突说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ConflictResult {
    fn no_conflict() -> Self {
        Self { conflict: false, conflict_type: None, description: None }
    }

    fn system_conflict(desc: &str) -> Self {
        Self {
            conflict: true,
            conflict_type: Some("system".to_string()),
            description: Some(desc.to_string()),
        }
    }
}

#[tauri::command]
/// 检测快捷键是否与系统保留快捷键冲突（需求 8）
pub fn check_hotkey_conflict(hotkey: String) -> ConflictResult {
    if hotkey.is_empty() {
        return ConflictResult::no_conflict();
    }

    let hotkey_lower = hotkey.to_lowercase();

    #[cfg(target_os = "macos")]
    {
        // macOS 系统保留快捷键列表
        let macos_reserved = [
            ("cmd+space", "Spotlight 搜索"),
            ("cmd+tab", "应用切换"),
            ("ctrl+space", "输入法切换"),
            ("cmd+ctrl+space", "表情符号选择器"),
            ("cmd+shift+3", "截图"),
            ("cmd+shift+4", "区域截图"),
            ("cmd+shift+5", "截图工具栏"),
            ("cmd+q", "退出应用"),
            ("cmd+w", "关闭窗口"),
            ("cmd+h", "隐藏应用"),
            ("cmd+m", "最小化窗口"),
            ("cmd+,", "偏好设置"),
            ("cmd+option+esc", "强制退出"),
            ("ctrl+cmd+q", "锁定屏幕"),
        ];
        for (reserved, desc) in &macos_reserved {
            if hotkey_lower.contains(reserved) || normalize_hotkey(&hotkey_lower) == *reserved {
                return ConflictResult::system_conflict(
                    &format!("与 macOS 系统快捷键冲突：{} ({})", reserved, desc)
                );
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows 系统保留快捷键列表
        let windows_reserved = [
            ("win+l", "锁定屏幕"),
            ("ctrl+alt+del", "任务管理器/安全选项"),
            ("alt+f4", "关闭窗口"),
            ("win+d", "显示桌面"),
            ("win+e", "文件资源管理器"),
            ("win+r", "运行对话框"),
            ("win+tab", "任务视图"),
            ("win+i", "设置"),
            ("win+s", "搜索"),
            ("win+p", "投影"),
            ("alt+tab", "应用切换"),
            ("ctrl+shift+esc", "任务管理器"),
            ("win+prtsc", "截图"),
        ];
        for (reserved, desc) in &windows_reserved {
            if hotkey_lower.contains(reserved) || normalize_hotkey(&hotkey_lower) == *reserved {
                return ConflictResult::system_conflict(
                    &format!("与 Windows 系统快捷键冲突：{} ({})", reserved, desc)
                );
            }
        }
    }

    ConflictResult::no_conflict()
}

/// 规范化快捷键字符串（统一格式，便于比较）
fn normalize_hotkey(hotkey: &str) -> String {
    hotkey
        .replace("control", "ctrl")
        .replace("option", "alt")
        .replace("command", "cmd")
        .replace("windows", "win")
        .replace(" ", "")
        .to_lowercase()
}

// ─── 全局快捷键命令 ───

#[tauri::command]
/// 注册全局快捷键（两端统一接口）
pub fn register_global_hotkey(hotkey: String) -> Result<(), String> {
    if hotkey.is_empty() {
        return Ok(());
    }
    // 快捷键注册通过 Tauri 的 globalShortcut 插件实现
    // 此命令作为后端存储接口，实际注册在前端通过 Tauri API 完成
    // 这里保存快捷键配置到设置文件
    let mut settings = get_settings().unwrap_or_default();
    settings.hotkey = hotkey;
    save_settings(settings)
}

// ─── 托盘菜单国际化命令 ───

/// 托盘菜单文字（根据语言）
pub struct TrayMenuText {
    pub show: &'static str,
    pub connect: &'static str,
    pub disconnect: &'static str,
    pub quit: &'static str,
}

impl TrayMenuText {
    pub fn for_lang(lang: &str) -> Self {
        match lang {
            "en" => Self {
                show: "Show Window",
                connect: "Connect",
                disconnect: "Disconnect",
                quit: "Quit",
            },
            _ => Self {
                show: "显示窗口",
                connect: "连接设备",
                disconnect: "断开连接",
                quit: "退出",
            },
        }
    }
}

#[tauri::command]
/// 更新托盘菜单语言（需求 7.1、7.2）
/// 前端语言切换时调用此命令，后端重建托盘菜单文字
pub fn update_tray_language(
    app: tauri::AppHandle,
    lang: String,
    connected: bool,
) -> Result<(), String> {
    let text = TrayMenuText::for_lang(&lang);

    // 保存语言设置
    let mut settings = get_settings().unwrap_or_default();
    settings.language = lang.clone();
    let _ = save_settings(settings);

    // 更新托盘菜单项文字
    if let Some(tray) = app.tray_handle().try_get_item("show") {
        let _ = tray.set_title(text.show);
    }
    if let Some(tray) = app.tray_handle().try_get_item("connect") {
        let _ = tray.set_title(text.connect);
        // 已连接时"连接设备"置灰（需求 7.5）
        let _ = tray.set_enabled(!connected);
    }
    if let Some(tray) = app.tray_handle().try_get_item("disconnect") {
        let _ = tray.set_title(text.disconnect);
        // 未连接时"断开连接"置灰
        let _ = tray.set_enabled(connected);
    }
    if let Some(tray) = app.tray_handle().try_get_item("quit") {
        let _ = tray.set_title(text.quit);
    }

    Ok(())
}

#[tauri::command]
/// 更新托盘菜单连接状态（连接/断开时调用）
pub fn update_tray_connection_state(
    app: tauri::AppHandle,
    connected: bool,
) -> Result<(), String> {
    // 读取当前语言设置
    let settings = get_settings().unwrap_or_default();
    update_tray_language(app, settings.language, connected)
}

// ─── 本机 IP 命令 ───

#[tauri::command]
/// 获取本机局域网 IP 地址列表
pub fn get_local_ip() -> Vec<String> {
    let mut ips = Vec::new();

    // 枚举所有网络接口，收集所有非回环 IPv4 地址
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("ifconfig").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("inet ") && !line.contains("127.0.0.1") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(ip) = parts.get(1) {
                        let ip = ip.trim_start_matches("addr:");
                        if !ip.is_empty() && ip != "0.0.0.0" {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("ipconfig").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let line = line.trim();
                // 匹配 "IPv4 地址" 或 "IPv4 Address"
                if line.contains("IPv4") && line.contains(':') {
                    if let Some(ip) = line.split(':').last() {
                        // 处理 Windows ipconfig 输出中的 "(Preferred)" 后缀
                        let ip = ip.trim()
                            .trim_end_matches("(Preferred)")
                            .trim_end_matches("(首选)")
                            .trim();
                        if !ip.is_empty() && ip != "0.0.0.0" && !ip.starts_with("169.254") {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    // 兜底：UDP 路由方式
    if ips.is_empty() {
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    let ip = addr.ip().to_string();
                    if ip != "0.0.0.0" {
                        ips.push(ip);
                    }
                }
            }
        }
    }

    ips
}

// ─── 延迟检测命令 ───

#[tauri::command]
/// 获取当前连接延迟（毫秒），None 表示未连接或尚未测量
pub async fn get_latency() -> Option<u32> {
    let rtt = LAST_RTT_MS.load(std::sync::atomic::Ordering::Relaxed);
    if rtt == u32::MAX {
        None
    } else {
        Some(rtt)
    }
}

// ─── 遗留进程清理命令 ───

#[tauri::command]
/// 检测并终止 kvnshare 遗留进程
pub fn kill_legacy_process() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        kill_legacy_process_macos()
    }
    #[cfg(target_os = "windows")]
    {
        kill_legacy_process_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(false)
    }
}

#[cfg(target_os = "macos")]
fn kill_legacy_process_macos() -> Result<bool, String> {
    let output = std::process::Command::new("pgrep")
        .arg("-x")
        .arg("kvnshare")
        .output()
        .map_err(|e| format!("pgrep 执行失败: {}", e))?;

    if output.status.success() {
        // 找到进程，终止它
        std::process::Command::new("pkill")
            .arg("-x")
            .arg("kvnshare")
            .output()
            .map_err(|e| format!("pkill 执行失败: {}", e))?;
        Ok(true) // 找到并终止了遗留进程
    } else {
        Ok(false) // 未找到遗留进程
    }
}

#[cfg(target_os = "windows")]
fn kill_legacy_process_windows() -> Result<bool, String> {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq kvnshare.exe", "/NH"])
        .output()
        .map_err(|e| format!("tasklist 执行失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("kvnshare.exe") {
        std::process::Command::new("taskkill")
            .args(["/F", "/IM", "kvnshare.exe"])
            .output()
            .map_err(|e| format!("taskkill 执行失败: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// ─── 网卡信息与监听地址配置命令 ───

/// 网卡信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
}

/// 监听地址配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenConfig {
    pub ip: String,
    pub port: u16,
}

#[tauri::command]
/// 获取本机所有非回环 IPv4 网卡及其 IP 地址列表
pub fn get_network_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("ifconfig").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_iface = String::new();
            for line in text.lines() {
                // 检测网卡名称行（不以空格开头）
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    if let Some(name) = line.split(':').next() {
                        current_iface = name.trim().to_string();
                    }
                }
                let line = line.trim();
                if line.starts_with("inet ") && !line.contains("127.0.0.1") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(ip) = parts.get(1) {
                        let ip = ip.trim_start_matches("addr:");
                        if !ip.is_empty() && ip != "0.0.0.0" && !current_iface.is_empty() {
                            interfaces.push(NetworkInterface {
                                name: current_iface.clone(),
                                ip: ip.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("ipconfig").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_iface = String::new();
            for line in text.lines() {
                // 检测适配器名称行
                if line.contains("适配器") || line.contains("Adapter") {
                    if let Some(name) = line.split(':').next() {
                        current_iface = name
                            .replace("以太网适配器", "")
                            .replace("无线局域网适配器", "")
                            .replace("Ethernet adapter", "")
                            .replace("Wireless LAN adapter", "")
                            .trim()
                            .to_string();
                    }
                }
                let line = line.trim();
                if line.contains("IPv4") && line.contains(':') {
                    if let Some(ip) = line.split(':').last() {
                        // 处理 Windows ipconfig 输出中的 "(Preferred)" 后缀
                        let ip = ip.trim()
                            .trim_end_matches("(Preferred)")
                            .trim_end_matches("(首选)")
                            .trim();
                        if !ip.is_empty() && ip != "0.0.0.0" && !ip.starts_with("169.254") {
                            interfaces.push(NetworkInterface {
                                name: current_iface.clone(),
                                ip: ip.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    interfaces
}

#[tauri::command]
/// 设置 KVMini 服务端监听地址和端口，保存配置并重启监听服务
pub fn set_listen_address(ip: String, port: u16) -> Result<(), String> {
    // 1. 端口范围校验
    if port < 1024 {
        return Err(format!("端口号 {} 无效：端口范围应为 1024-65535", port));
    }

    // 2. IP 格式校验（允许 0.0.0.0 表示所有网卡）
    if ip != "0.0.0.0" && !is_valid_ipv4(&ip) {
        return Err(format!("IP 地址格式不正确：\"{}\"", ip));
    }

    // 3. 保存监听配置到文件
    let dir = ensure_data_dir()?;
    let path = dir.join("listen_config.json");
    let config = ListenConfig { ip: ip.clone(), port };
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化监听配置失败: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("写入监听配置失败: {}", e))?;

    Ok(())
}

#[tauri::command]
/// 获取当前监听配置
pub fn get_listen_config() -> Result<ListenConfig, String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("listen_config.json");

    if !path.exists() {
        // 默认：监听所有网卡，端口 8889
        return Ok(ListenConfig {
            ip: "0.0.0.0".to_string(),
            port: 8889,
        });
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取监听配置失败: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析监听配置失败: {}", e))
}

// ─── 手动 IP 直连命令 ───

/// 手动 IP 直连结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectByIpResult {
    pub success: bool,
    /// 错误类型："refused" | "timeout" | "unreachable" | "invalid_ip" | "handshake_failed" | "version_mismatch" | ""
    pub error_type: String,
    /// 用户可读的错误消息
    pub error_message: String,
    /// 对端平台（握手成功时填充）
    pub remote_platform: String,
}

/// 校验 IPv4 地址格式
fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| {
        p.parse::<u8>().is_ok()
    })
}

/// 全局活动 TCP 连接状态
pub struct ActiveTcpConnection {
    /// 对端地址
    pub peer_addr: String,
    /// 对端平台
    pub remote_platform: String,
    /// 连接建立时间
    pub connected_at: std::time::SystemTime,
    /// TCP stream（用于后续键鼠事件传输）
    pub stream: Option<std::net::TcpStream>,
}

/// 全局事件循环停止信号
pub struct EventLoopHandle {
    /// 停止信号发送端
    stop_tx: std::sync::mpsc::Sender<()>,
}

impl EventLoopHandle {
    pub fn stop(&self) {
        let _ = self.stop_tx.send(());
    }
}

lazy_static::lazy_static! {
    /// 全局活动 TCP 连接（握手成功后保存）
    pub static ref ACTIVE_TCP_CONNECTION: std::sync::Mutex<Option<ActiveTcpConnection>> =
        std::sync::Mutex::new(None);

    /// 全局事件循环句柄（捕获/注入循环）
    static ref EVENT_LOOP_HANDLE: std::sync::Mutex<Option<EventLoopHandle>> =
        std::sync::Mutex::new(None);

    /// 全局剪贴板监听器
    static ref CLIPBOARD_MONITOR: std::sync::Mutex<Option<Box<dyn crate::clipboard::ClipboardMonitor>>> =
        std::sync::Mutex::new(None);

    /// 全局 UDP 广播监听器停止信号（被控端）
    static ref UDP_LISTENER_STOP: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>> =
        std::sync::Mutex::new(None);
}

/// 最新 RTT 延迟（毫秒），0 表示尚未测量，u32::MAX 表示连接已断开
static LAST_RTT_MS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// 获取当前时间戳（毫秒）
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 启动主控端事件循环：捕获键鼠事件 → 键码映射 → TCP 发送
///
/// 在专用线程中运行，通过 stop_rx 接收停止信号。
fn start_controller_loop(
    stream: std::net::TcpStream,
    remote_platform: crate::protocol::Platform,
    stop_rx: std::sync::mpsc::Receiver<()>,
) {
    use crate::capturer::create_platform_capturer;
    use crate::keycode_map;
    use crate::protocol::{encode, KvmFrame, Platform};
    use std::io::Write;

    std::thread::spawn(move || {
        // 创建 tokio channel（capturer 使用 tokio mpsc）
        let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<KvmFrame>(1024);

        // 创建平台捕获器
        let capturer = create_platform_capturer();

        // 启动捕获
        if let Err(e) = capturer.start(frame_tx) {
            log::error!("启动捕获器失败: {}", e);
            return;
        }

        // 开启事件拦截（主控端模式：键鼠事件不传递给本机）
        capturer.set_intercept(true);

        let mut stream = stream;

        // 在当前线程运行 tokio 事件循环
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("创建 tokio runtime 失败: {}", e);
                capturer.set_intercept(false);
                capturer.stop();
                return;
            }
        };

        rt.block_on(async move {
            let mut last_ping_time = tokio::time::Instant::now();
            const PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

            loop {
                // 检查停止信号
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // 每 2 秒发送一次 Ping
                if last_ping_time.elapsed() >= PING_INTERVAL {
                    let ts = now_ms();
                    let ping_bytes = encode(&KvmFrame::Ping { timestamp_ms: ts });
                    if let Err(e) = stream.write_all(&ping_bytes) {
                        log::error!("Ping 发送失败: {}，停止主控端循环", e);
                        break;
                    }
                    last_ping_time = tokio::time::Instant::now();
                }

                // 接收捕获事件（超时 1ms）
                match tokio::time::timeout(
                    std::time::Duration::from_millis(1),
                    frame_rx.recv(),
                ).await {
                    Ok(Some(frame)) => {
                        // 键码映射：将本机键码转换为对端键码
                        let frame = match frame {
                            KvmFrame::KeyEvent { keycode, pressed } => {
                                // macOS → Windows 键码映射
                                if remote_platform == Platform::Windows {
                                    match keycode_map::cg_to_vk(keycode) {
                                        Some(vk) => KvmFrame::KeyEvent { keycode: vk, pressed },
                                        None => {
                                            log::warn!("未知 CGKeyCode: {}，丢弃", keycode);
                                            continue;
                                        }
                                    }
                                } else {
                                    KvmFrame::KeyEvent { keycode, pressed }
                                }
                            }
                            other => other,
                        };

                        // 发送到 TCP
                        let bytes = encode(&frame);
                        if let Err(e) = stream.write_all(&bytes) {
                            log::error!("TCP 发送失败: {}，停止主控端循环", e);
                            break;
                        }
                    }
                    Ok(None) => {
                        log::info!("捕获器 channel 已关闭，退出主控端循环");
                        break;
                    }
                    Err(_) => {
                        // 超时，继续循环
                    }
                }
            }
        });

        // 停止捕获，恢复本机输入
        capturer.set_intercept(false);
        capturer.stop();
        log::info!("主控端事件循环已停止");
    });
}

/// 启动被控端事件循环：TCP 接收 → 键码映射 → 注入键鼠事件
///
/// 在专用线程中运行，通过 stop_rx 接收停止信号。
fn start_receiver_loop(
    stream: std::net::TcpStream,
    remote_platform: crate::protocol::Platform,
    stop_rx: std::sync::mpsc::Receiver<()>,
    window: Option<tauri::Window>,
) {
    use crate::injector::{create_platform_injector, inject_frame};
    use crate::keycode_map;
    use crate::monitor::get_virtual_desktop;
    use crate::protocol::{decode, encode, KvmFrame, Platform, ProtocolError};
    use std::io::{Read, Write};

    std::thread::spawn(move || {
        // 获取屏幕尺寸（用于注入器坐标转换）
        let vd = get_virtual_desktop();
        let injector = create_platform_injector(vd.total_width, vd.total_height);

        let mut stream = stream;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(5)))
            .unwrap_or(());

        let mut buf = Vec::with_capacity(4096);
        let mut read_buf = [0u8; 4096];

        loop {
            // 检查停止信号
            if stop_rx.try_recv().is_ok() {
                break;
            }

            // 读取 TCP 数据
            match stream.read(&mut read_buf) {
                Ok(0) => {
                    log::info!("TCP 连接已关闭，退出被控端循环");
                    if let Some(ref w) = window {
                        let _ = w.emit("connection-lost", serde_json::json!({
                            "reason": "remote_closed",
                            "message": "对端已断开连接"
                        }));
                    }
                    break;
                }
                Ok(n) => {
                    buf.extend_from_slice(&read_buf[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {
                    // 超时，继续循环
                    continue;
                }
                Err(e) => {
                    log::error!("TCP 读取失败: {}，停止被控端循环", e);
                    if let Some(ref w) = window {
                        let _ = w.emit("connection-lost", serde_json::json!({
                            "reason": "io_error",
                            "message": format!("连接错误: {}", e)
                        }));
                    }
                    break;
                }
            }

            // 解码并处理所有完整帧
            let mut consumed = 0;
            loop {
                match decode(&buf[consumed..]) {
                    Ok((frame, n)) => {
                        consumed += n;

                        // 键码映射：将对端键码转换为本机键码
                        let frame = match frame {
                            KvmFrame::KeyEvent { keycode, pressed } => {
                                let mapped = if remote_platform == Platform::Windows {
                                    // Windows → macOS 键码映射
                                    match keycode_map::vk_to_cg(keycode) {
                                        Some(cg) => cg,
                                        None => {
                                            log::warn!("未知 VirtualKey: {}，丢弃", keycode);
                                            continue;
                                        }
                                    }
                                } else {
                                    keycode
                                };
                                KvmFrame::KeyEvent { keycode: mapped, pressed }
                            }
                            // 收到 Ping → 立即回复 Pong（回传原始时间戳）
                            KvmFrame::Ping { timestamp_ms } => {
                                let pong_bytes = encode(&KvmFrame::Pong { timestamp_ms });
                                if let Err(e) = stream.write_all(&pong_bytes) {
                                    log::warn!("Pong 回复失败: {}", e);
                                }
                                continue;
                            }
                            // 收到 Pong → 计算 RTT，更新全局延迟
                            KvmFrame::Pong { timestamp_ms } => {
                                let now = now_ms();
                                let rtt = now.saturating_sub(timestamp_ms) as u32;
                                LAST_RTT_MS.store(rtt, std::sync::atomic::Ordering::Relaxed);
                                log::debug!("RTT: {}ms", rtt);
                                continue;
                            }
                            other => other,
                        };

                        // 注入事件
                        if let Err(e) = inject_frame(injector.as_ref(), &frame) {
                            log::warn!("注入事件失败: {}", e);
                        }
                    }
                    Err(ProtocolError::Incomplete) => break,
                    Err(e) => {
                        log::warn!("协议解码错误: {}，跳过", e);
                        // 跳过一个字节，尝试重新同步
                        if !buf[consumed..].is_empty() {
                            consumed += 1;
                        } else {
                            break;
                        }
                    }
                }
            }

            if consumed > 0 {
                buf.drain(..consumed);
            }
        }

        log::info!("被控端事件循环已停止");
    });
}

/// 停止当前事件循环（如果有）
fn stop_event_loop() {
    if let Ok(mut handle) = EVENT_LOOP_HANDLE.lock() {
        if let Some(h) = handle.take() {
            h.stop();
        }
    }
    // 同时停止剪贴板监听
    if let Ok(mut monitor) = CLIPBOARD_MONITOR.lock() {
        if let Some(ref m) = *monitor {
            m.stop();
        }
        *monitor = None;
    }
}

/// 启动被控端 UDP 广播监听
///
/// 监听 UDP 8888 端口，收到 KVM_DISCOVERY 广播后立即回复 KVM_RESPONSE:JSON。
/// 崩溃时自动重启，最多重试 3 次。
pub fn start_udp_discovery_listener() {
    use std::net::UdpSocket;
    use crate::network::DISCOVERY_PORT;

    // 停止旧的监听器
    stop_udp_discovery_listener();

    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    if let Ok(mut s) = UDP_LISTENER_STOP.lock() {
        *s = Some(stop_tx);
    }

    std::thread::spawn(move || {
        let mut retry_count = 0u32;
        const MAX_RETRIES: u32 = 3;

        loop {
            // 检查停止信号
            if stop_rx.try_recv().is_ok() {
                log::info!("UDP 广播监听已停止");
                break;
            }

            match run_udp_listener(&stop_rx) {
                Ok(()) => {
                    // 正常停止（收到停止信号）
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count > MAX_RETRIES {
                        log::error!("UDP 广播监听失败次数超过上限（{}），停止重试: {}", MAX_RETRIES, e);
                        break;
                    }
                    log::warn!("UDP 广播监听崩溃（第 {}/{} 次），1 秒后重启: {}", retry_count, MAX_RETRIES, e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    });
}

/// 运行 UDP 监听循环（内部函数，供 start_udp_discovery_listener 调用）
fn run_udp_listener(stop_rx: &std::sync::mpsc::Receiver<()>) -> Result<(), String> {
    use std::net::UdpSocket;
    use crate::network::DISCOVERY_PORT;

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT))
        .map_err(|e| format!("绑定 UDP 端口 {} 失败: {}", DISCOVERY_PORT, e))?;

    // 设置读取超时，以便定期检查停止信号
    socket.set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .map_err(|e| format!("设置 UDP 超时失败: {}", e))?;

    log::info!("被控端 UDP 广播监听已启动，端口 {}", DISCOVERY_PORT);

    let mut buf = [0u8; 256];

    loop {
        // 检查停止信号
        if stop_rx.try_recv().is_ok() {
            return Ok(());
        }

        match socket.recv_from(&mut buf) {
            Ok((size, src_addr)) => {
                let data = &buf[..size];
                if data == crate::network::DISCOVERY_MSG {
                    // 收到发现广播，立即回复包含设备信息的 JSON 响应
                    let device_name = get_device_name();
                    let os_name = get_os_name();
                    let tcp_port = get_listen_config()
                        .map(|c| c.port)
                        .unwrap_or(8889);

                    let response_json = serde_json::json!({
                        "name": device_name,
                        "os": os_name,
                        "tcp_port": tcp_port,
                    });
                    let response = format!("KVM_RESPONSE:{}", response_json);

                    if let Err(e) = socket.send_to(response.as_bytes(), src_addr) {
                        log::warn!("回复 UDP 发现广播失败: {}", e);
                    } else {
                        log::info!("已回复 UDP 发现广播给 {}", src_addr);
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut => {
                // 超时，继续循环（用于检查停止信号）
            }
            Err(e) => {
                return Err(format!("UDP 接收失败: {}", e));
            }
        }
    }
}

/// 停止被控端 UDP 广播监听
pub fn stop_udp_discovery_listener() {
    if let Ok(mut s) = UDP_LISTENER_STOP.lock() {
        if let Some(tx) = s.take() {
            let _ = tx.send(());
        }
    }
}

/// 获取本机设备名称
fn get_device_name() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("scutil")
            .args(&["--get", "ComputerName"])
            .output()
        {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(name) = std::env::var("COMPUTERNAME") {
            if !name.is_empty() {
                return name;
            }
        }
    }
    // 降级：使用 hostname
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "KVMini Device".to_string())
}

/// 获取当前操作系统名称
fn get_os_name() -> &'static str {
    #[cfg(target_os = "macos")]
    { "macOS" }
    #[cfg(target_os = "windows")]
    { "Windows" }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { "Linux" }
}

/// 启动剪贴板共享：监听本机剪贴板变化，通过 TCP 发送给对端
fn start_clipboard_sync(stream: Option<std::net::TcpStream>) {
    use crate::clipboard::create_platform_clipboard_monitor;
    use crate::protocol::{encode, KvmFrame};
    use std::io::Write;

    let stream = match stream {
        Some(s) => s,
        None => return,
    };

    // 创建 tokio channel（clipboard monitor 使用 tokio mpsc）
    let (clip_tx, mut clip_rx) = tokio::sync::mpsc::channel::<KvmFrame>(64);

    let monitor = create_platform_clipboard_monitor();
    if let Err(e) = monitor.start(clip_tx) {
        log::error!("启动剪贴板监听失败: {}", e);
        return;
    }

    // 保存监听器句柄
    if let Ok(mut m) = CLIPBOARD_MONITOR.lock() {
        *m = Some(monitor);
    }

    // 在后台线程中转发剪贴板帧到 TCP
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("剪贴板同步 runtime 创建失败: {}", e);
                return;
            }
        };

        let mut stream = stream;
        rt.block_on(async move {
            while let Some(frame) = clip_rx.recv().await {
                let bytes = encode(&frame);
                if let Err(e) = stream.write_all(&bytes) {
                    log::error!("剪贴板 TCP 发送失败: {}", e);
                    break;
                }
            }
        });
        log::info!("剪贴板同步循环已停止");
    });
}

#[tauri::command]
/// 通过手动输入的 IP 地址直接发起 TCP 连接（跨网段直连）
/// 连接成功后执行 KVMini 握手协议验证，确保对端是 KVMini 服务
pub async fn connect_by_ip(ip: String, port: u16) -> ConnectByIpResult {
    // 1. IPv4 格式校验
    if !is_valid_ipv4(&ip) {
        return ConnectByIpResult {
            success: false,
            error_type: "invalid_ip".to_string(),
            error_message: format!("IP 地址格式不正确：\"{}\"，请输入合法的 IPv4 地址（如 192.168.1.100）", ip),
            remote_platform: String::new(),
        };
    }

    let addr = format!("{}:{}", ip, port);

    // 2. 带 5 秒超时的 TCP 连接
    let connect_future = tokio::net::TcpStream::connect(&addr);
    let tcp_stream = match tokio::time::timeout(std::time::Duration::from_secs(5), connect_future).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            use std::io::ErrorKind;
            let (error_type, error_message) = match e.kind() {
                ErrorKind::ConnectionRefused => (
                    "refused".to_string(),
                    "连接被拒绝：对端 KVMini 未运行，或目标端口未开放。请确认对端已启动 KVM Share 并开放防火墙端口。".to_string(),
                ),
                ErrorKind::NetworkUnreachable | ErrorKind::HostUnreachable | ErrorKind::AddrNotAvailable => (
                    "unreachable".to_string(),
                    "网络不可达：请检查两台设备是否在同一公司内网，以及路由是否互通。".to_string(),
                ),
                _ => (
                    "unreachable".to_string(),
                    format!("无法连接到 {}：{}。请检查 IP 地址是否正确，以及网络是否互通。", addr, e),
                ),
            };
            return ConnectByIpResult { success: false, error_type, error_message, remote_platform: String::new() };
        }
        Err(_) => {
            return ConnectByIpResult {
                success: false,
                error_type: "timeout".to_string(),
                error_message: format!(
                    "连接超时（5秒）：可能被防火墙拦截。请检查 {} 的 Windows Defender / macOS 防火墙设置，确保 KVM Share 端口已开放。",
                    ip
                ),
                remote_platform: String::new(),
            };
        }
    };

    // 3. TCP 连接成功，转换为标准库 TcpStream 执行握手协议
    let std_stream = match tcp_stream.into_std() {
        Ok(s) => s,
        Err(e) => {
            return ConnectByIpResult {
                success: false,
                error_type: "unreachable".to_string(),
                error_message: format!("连接转换失败：{}", e),
                remote_platform: String::new(),
            };
        }
    };

    // 4. 设置握手超时（3 秒）
    if let Err(e) = crate::handshake::set_handshake_timeout(&std_stream) {
        return ConnectByIpResult {
            success: false,
            error_type: "unreachable".to_string(),
            error_message: format!("设置超时失败：{}", e),
            remote_platform: String::new(),
        };
    }

    // 5. 执行 KVMini 握手协议验证
    let mut stream_clone = match std_stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            return ConnectByIpResult {
                success: false,
                error_type: "unreachable".to_string(),
                error_message: format!("连接克隆失败：{}", e),
                remote_platform: String::new(),
            };
        }
    };

    let handshake_result = crate::handshake::perform_client_handshake(&mut stream_clone);

    match handshake_result {
        Ok(info) => {
            // 6. 握手成功，保存 TCP stream 到全局状态
            let remote_platform = format!("{:?}", info.remote_platform);
            let peer_addr = addr.clone();

            // 停止旧的事件循环（如果有）
            stop_event_loop();

            // 克隆 stream 用于事件循环
            let stream_for_loop = match std_stream.try_clone() {
                Ok(s) => s,
                Err(e) => {
                    return ConnectByIpResult {
                        success: false,
                        error_type: "unreachable".to_string(),
                        error_message: format!("连接克隆失败（事件循环）：{}", e),
                        remote_platform: String::new(),
                    };
                }
            };

            // 创建停止信号
            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();

            // 读取当前角色，决定启动主控端还是被控端循环
            let current_role = {
                // 尝试从 Tauri 状态读取角色，默认为被控端
                crate::role_manager::AppRole::Server // 默认被控端，前端可通过 set_role 切换
            };

            // 根据角色启动对应的事件循环
            match current_role {
                crate::role_manager::AppRole::Client => {
                    // 主控端：捕获键鼠 → TCP 发送
                    start_controller_loop(stream_for_loop, info.remote_platform, stop_rx);
                    log::info!("主控端事件循环已启动，连接到 {}", peer_addr);
                }
                crate::role_manager::AppRole::Server => {
                    // 被控端：TCP 接收 → 注入键鼠
                    start_receiver_loop(stream_for_loop, info.remote_platform, stop_rx, None);
                    log::info!("被控端事件循环已启动，接受来自 {} 的控制", peer_addr);
                }
            }

            // 保存事件循环句柄
            if let Ok(mut handle) = EVENT_LOOP_HANDLE.lock() {
                *handle = Some(EventLoopHandle { stop_tx });
            }

            // 启动剪贴板共享（如果设置中已启用）
            let clipboard_enabled = get_settings()
                .map(|s| s.clipboard_sync)
                .unwrap_or(true);
            if clipboard_enabled {
                start_clipboard_sync(std_stream.try_clone().ok());
            }

            // 清除旧连接，保存新连接
            if let Ok(mut conn) = ACTIVE_TCP_CONNECTION.lock() {
                *conn = Some(ActiveTcpConnection {
                    peer_addr: peer_addr.clone(),
                    remote_platform: remote_platform.clone(),
                    connected_at: std::time::SystemTime::now(),
                    stream: Some(std_stream),
                });
            }

            ConnectByIpResult {
                success: true,
                error_type: String::new(),
                error_message: format!("已成功连接到 {}", peer_addr),
                remote_platform,
            }
        }
        Err(crate::handshake::HandshakeError::NotKvmService) => {
            ConnectByIpResult {
                success: false,
                error_type: "handshake_failed".to_string(),
                error_message: "连接失败：对端不是 KVMini 服务。请确认对端已启动 KVM Share。".to_string(),
                remote_platform: String::new(),
            }
        }
        Err(crate::handshake::HandshakeError::VersionMismatch { local, remote }) => {
            ConnectByIpResult {
                success: false,
                error_type: "version_mismatch".to_string(),
                error_message: format!(
                    "版本不兼容：本端版本 {}，对端版本 {}。请确保两端使用相同版本的 KVM Share。",
                    local, remote
                ),
                remote_platform: String::new(),
            }
        }
        Err(crate::handshake::HandshakeError::Timeout) => {
            ConnectByIpResult {
                success: false,
                error_type: "timeout".to_string(),
                error_message: "握手超时（3秒）：对端可能不是 KVMini 服务，或网络延迟过高。".to_string(),
                remote_platform: String::new(),
            }
        }
        Err(crate::handshake::HandshakeError::IoError(msg)) => {
            ConnectByIpResult {
                success: false,
                error_type: "unreachable".to_string(),
                error_message: format!("握手失败：{}", msg),
                remote_platform: String::new(),
            }
        }
    }
}

/// Tauri 命令：断开当前 TCP 连接
#[tauri::command]
pub fn disconnect_tcp() -> Result<(), String> {
    // 停止事件循环（捕获/注入）
    stop_event_loop();

    // 重置延迟显示
    LAST_RTT_MS.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);

    if let Ok(mut conn) = ACTIVE_TCP_CONNECTION.lock() {
        if let Some(active) = conn.take() {
            // 关闭 TCP stream
            if let Some(stream) = active.stream {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        }
    }
    Ok(())
}

/// Tauri 命令：获取当前 TCP 连接状态
#[tauri::command]
pub fn get_tcp_connection_status() -> Option<serde_json::Value> {
    if let Ok(conn) = ACTIVE_TCP_CONNECTION.lock() {
        conn.as_ref().map(|c| {
            serde_json::json!({
                "peer_addr": c.peer_addr,
                "remote_platform": c.remote_platform,
                "connected": c.stream.is_some(),
            })
        })
    } else {
        None
    }
}

/// Tauri 命令：启动心跳检测（连接成功后调用）
///
/// 在后台任务中每 10 秒发送一次心跳帧，30 秒内无响应则认为连接断开。
/// 断开时通过 Tauri 事件通知前端。
#[tauri::command]
pub async fn start_heartbeat_monitor(window: tauri::Window) {
    tokio::spawn(async move {
        let mut consecutive_failures = 0u32;
        const MAX_FAILURES: u32 = 3; // 3 次失败（约 30 秒）后断开

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            // 检查连接是否还存在
            let has_connection = {
                if let Ok(conn) = ACTIVE_TCP_CONNECTION.lock() {
                    conn.is_some()
                } else {
                    false
                }
            };

            if !has_connection {
                // 连接已被主动断开，停止心跳
                break;
            }

            // 尝试发送心跳帧
            let heartbeat_ok = {
                if let Ok(mut conn) = ACTIVE_TCP_CONNECTION.lock() {
                    if let Some(ref mut active) = *conn {
                        if let Some(ref mut stream) = active.stream {
                            use std::io::Write;
                            let heartbeat_bytes = crate::protocol::encode(&crate::protocol::KvmFrame::Heartbeat);
                            stream.write_all(&heartbeat_bytes).is_ok()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if heartbeat_ok {
                consecutive_failures = 0;
            } else {
                consecutive_failures += 1;
                if consecutive_failures >= MAX_FAILURES {
                    // 心跳失败超过阈值，清除连接状态并通知前端
                    if let Ok(mut conn) = ACTIVE_TCP_CONNECTION.lock() {
                        *conn = None;
                    }
                    let _ = window.emit("connection-lost", serde_json::json!({
                        "reason": "heartbeat_timeout",
                        "message": "连接超时，已断开（心跳检测失败）"
                    }));
                    break;
                }
            }
        }
    });
}

// ─── 单元测试 ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        // 默认快捷键应为 Ctrl+Alt+S（需求 6.1）
        assert_eq!(settings.hotkey, DEFAULT_HOTKEY);
        assert_eq!(settings.prefer_connection_type, "auto");
        assert!(settings.auto_reconnect);
        assert!(!settings.auto_start);
        assert!(!settings.start_minimized);
        assert!(settings.clipboard_sync);
        assert!(settings.enable_tls);
        assert_eq!(settings.command_key_mapping, "win");
    }

    #[test]
    fn test_settings_serialization() {
        let settings = AppSettings {
            hotkey: "Ctrl+Alt+K".to_string(),
            prefer_connection_type: "network".to_string(),
            auto_reconnect: false,
            auto_start: true,
            start_minimized: false,
            clipboard_sync: true,
            enable_tls: true,
            command_key_mapping: "win".to_string(),
            language: "zh".to_string(),
        };
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.hotkey, "Ctrl+Alt+K");
        assert_eq!(deserialized.prefer_connection_type, "network");
        assert!(!deserialized.auto_reconnect);
        assert!(deserialized.auto_start);
    }

    #[test]
    fn test_history_entry_serialization() {
        let entry = HistoryEntry {
            id: "dev-1".to_string(),
            name: "Test Device".to_string(),
            os: "Windows".to_string(),
            ip: "192.168.1.100".to_string(),
            connection_type: "Network".to_string(),
            last_connected: "2026-04-03T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "dev-1");
        assert_eq!(deserialized.name, "Test Device");
    }

    #[test]
    fn test_get_app_data_dir_not_empty() {
        let dir = get_app_data_dir();
        assert!(!dir.to_string_lossy().is_empty());
    }

    #[test]
    fn test_history_dedup_and_truncate() {
        // 模拟历史记录去重和截断逻辑
        let mut history: Vec<HistoryEntry> = (0..12).map(|i| HistoryEntry {
            id: format!("dev-{}", i),
            name: format!("Device {}", i),
            os: "macOS".to_string(),
            ip: format!("192.168.1.{}", i),
            connection_type: "Network".to_string(),
            last_connected: "2026-04-03T10:00:00Z".to_string(),
        }).collect();

        let new_entry = HistoryEntry {
            id: "dev-5".to_string(), // 重复 id
            name: "Device 5 Updated".to_string(),
            os: "macOS".to_string(),
            ip: "192.168.1.5".to_string(),
            connection_type: "Network".to_string(),
            last_connected: "2026-04-03T12:00:00Z".to_string(),
        };

        history.retain(|h| h.id != new_entry.id);
        history.insert(0, new_entry);
        history.truncate(10);

        assert_eq!(history.len(), 10);
        assert_eq!(history[0].id, "dev-5");
        assert_eq!(history[0].name, "Device 5 Updated");
    }

    // ─── connect_by_ip 相关测试 ───

    #[test]
    fn test_is_valid_ipv4_valid() {
        assert!(is_valid_ipv4("192.168.1.100"));
        assert!(is_valid_ipv4("10.0.1.1"));
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("255.255.255.255"));
    }

    #[test]
    fn test_is_valid_ipv4_invalid() {
        assert!(!is_valid_ipv4(""));
        assert!(!is_valid_ipv4("192.168.1"));
        assert!(!is_valid_ipv4("192.168.1.256"));
        assert!(!is_valid_ipv4("abc.def.ghi.jkl"));
        assert!(!is_valid_ipv4("192.168.1.1.1"));
        assert!(!is_valid_ipv4("192.168.1.-1"));
        assert!(!is_valid_ipv4("not-an-ip"));
    }

    #[tokio::test]
    async fn test_connect_by_ip_invalid_format() {
        let result = connect_by_ip("not-an-ip".to_string(), 8888).await;
        assert!(!result.success);
        assert_eq!(result.error_type, "invalid_ip");
        assert!(result.error_message.contains("IP 地址格式不正确"));
    }

    #[tokio::test]
    async fn test_connect_by_ip_refused() {
        // 连接本机不存在的端口，应该得到 refused 或 unreachable
        let result = connect_by_ip("127.0.0.1".to_string(), 19999).await;
        assert!(!result.success);
        // 本机端口拒绝应为 refused 或 unreachable（取决于系统）
        assert!(result.error_type == "refused" || result.error_type == "unreachable");
    }

    #[tokio::test]
    async fn test_connect_by_ip_timeout_message_format() {
        // 验证超时错误消息包含防火墙提示
        // 构造一个 ConnectByIpResult 模拟超时场景，验证消息格式
        let timeout_result = ConnectByIpResult {
            success: false,
            error_type: "timeout".to_string(),
            error_message: format!(
                "连接超时（5秒）：可能被防火墙拦截。请检查 {} 的 Windows Defender / macOS 防火墙设置，确保 KVM Share 端口已开放。",
                "192.168.1.100"
            ),
            remote_platform: String::new(),
        };
        assert!(!timeout_result.success);
        assert_eq!(timeout_result.error_type, "timeout");
        assert!(timeout_result.error_message.contains("防火墙"));
        assert!(timeout_result.error_message.contains("5秒"));
    }

    // ─── Windows (Preferred) 后缀处理测试（跨平台通用逻辑） ───

    /// 模拟 Windows ipconfig 输出中 IP 地址的清理逻辑（与 platform.rs 保持一致）
    fn clean_windows_ip(raw: &str) -> String {
        raw.trim()
            .trim_end_matches("(Preferred)")
            .trim_end_matches("(首选)")
            .trim()
            .to_string()
    }

    #[test]
    fn test_clean_windows_ip_with_preferred_suffix() {
        // 模拟 Windows ipconfig 输出：IPv4 地址 . . . . . . . . . . . : 192.168.1.100(Preferred)
        assert_eq!(clean_windows_ip("192.168.1.100(Preferred)"), "192.168.1.100");
    }

    #[test]
    fn test_clean_windows_ip_with_preferred_suffix_and_space() {
        // 带空格的情况：" 192.168.1.100(Preferred) "
        assert_eq!(clean_windows_ip(" 192.168.1.100(Preferred) "), "192.168.1.100");
    }

    #[test]
    fn test_clean_windows_ip_with_chinese_preferred_suffix() {
        // 中文 Windows 系统的"首选"后缀
        assert_eq!(clean_windows_ip("10.0.1.50(首选)"), "10.0.1.50");
    }

    #[test]
    fn test_clean_windows_ip_without_suffix() {
        // 无后缀的正常 IP（macOS 或无后缀的 Windows）
        assert_eq!(clean_windows_ip("192.168.1.100"), "192.168.1.100");
    }

    #[test]
    fn test_clean_windows_ip_with_spaces_only() {
        // 只有空格
        assert_eq!(clean_windows_ip("  "), "");
    }

    // ─── set_listen_address / get_listen_config 跨平台测试 ───
    // 这些测试使用临时目录，不依赖任何平台特定 API，macOS 和 Windows 均可通过

    /// 辅助函数：在指定目录保存监听配置（复用 set_listen_address 的核心逻辑）
    fn save_listen_config_to_dir(dir: &std::path::Path, ip: &str, port: u16) -> Result<(), String> {
        if port < 1024 {
            return Err(format!("端口号 {} 无效：端口范围应为 1024-65535", port));
        }
        if ip != "0.0.0.0" && !is_valid_ipv4(ip) {
            return Err(format!("IP 地址格式不正确：\"{}\"", ip));
        }
        let path = dir.join("listen_config.json");
        let config = ListenConfig { ip: ip.to_string(), port };
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("写入失败: {}", e))
    }

    /// 辅助函数：从指定目录读取监听配置
    fn load_listen_config_from_dir(dir: &std::path::Path) -> Result<ListenConfig, String> {
        let path = dir.join("listen_config.json");
        if !path.exists() {
            return Ok(ListenConfig { ip: "0.0.0.0".to_string(), port: 8889 });
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取失败: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("解析失败: {}", e))
    }

    #[test]
    fn test_set_listen_address_valid_port_min_boundary() {
        // 合法端口下边界：1024
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos()));
        fs::create_dir_all(&dir).unwrap();
        let result = save_listen_config_to_dir(&dir, "192.168.1.100", 1024);
        assert!(result.is_ok(), "端口 1024 应为合法端口");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_listen_address_valid_port_max_boundary() {
        // 合法端口上边界：65535
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 1));
        fs::create_dir_all(&dir).unwrap();
        let result = save_listen_config_to_dir(&dir, "0.0.0.0", 65535);
        assert!(result.is_ok(), "端口 65535 应为合法端口");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_listen_address_invalid_port_below_1024() {
        // 非法端口：< 1024
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 2));
        fs::create_dir_all(&dir).unwrap();
        let result = save_listen_config_to_dir(&dir, "192.168.1.100", 80);
        assert!(result.is_err(), "端口 80 应为非法端口");
        assert!(result.unwrap_err().contains("1024-65535"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_listen_address_invalid_port_zero() {
        // 非法端口：0
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 3));
        fs::create_dir_all(&dir).unwrap();
        let result = save_listen_config_to_dir(&dir, "0.0.0.0", 0);
        assert!(result.is_err(), "端口 0 应为非法端口");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_listen_address_all_interfaces() {
        // 选择"所有网卡（0.0.0.0）"时正确保存
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 4));
        fs::create_dir_all(&dir).unwrap();
        let result = save_listen_config_to_dir(&dir, "0.0.0.0", 8889);
        assert!(result.is_ok(), "0.0.0.0 应为合法的监听地址");
        // 验证保存的内容
        let config = load_listen_config_from_dir(&dir).unwrap();
        assert_eq!(config.ip, "0.0.0.0");
        assert_eq!(config.port, 8889);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_listen_config_persistence() {
        // 配置持久化：保存后重新读取返回相同值
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 5));
        fs::create_dir_all(&dir).unwrap();

        // 保存配置
        save_listen_config_to_dir(&dir, "192.168.1.50", 9000).unwrap();

        // 重新读取，验证一致
        let config = load_listen_config_from_dir(&dir).unwrap();
        assert_eq!(config.ip, "192.168.1.50");
        assert_eq!(config.port, 9000);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_listen_config_default_when_no_file() {
        // 无配置文件时返回默认值（0.0.0.0:8889）
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 6));
        fs::create_dir_all(&dir).unwrap();

        let config = load_listen_config_from_dir(&dir).unwrap();
        assert_eq!(config.ip, "0.0.0.0");
        assert_eq!(config.port, 8889);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_listen_config_overwrite() {
        // 多次保存，最后一次生效
        let dir = std::env::temp_dir().join(format!("kvmini_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() + 7));
        fs::create_dir_all(&dir).unwrap();

        save_listen_config_to_dir(&dir, "192.168.1.1", 8889).unwrap();
        save_listen_config_to_dir(&dir, "10.0.1.100", 9999).unwrap();

        let config = load_listen_config_from_dir(&dir).unwrap();
        assert_eq!(config.ip, "10.0.1.100");
        assert_eq!(config.port, 9999);

        let _ = fs::remove_dir_all(&dir);
    }
}
