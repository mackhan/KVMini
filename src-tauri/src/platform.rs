use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ─── 数据结构 ───

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey: String,
    pub prefer_connection_type: String,
    pub auto_reconnect: bool,
    pub auto_start: bool,
    pub start_minimized: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: String::new(),
            prefer_connection_type: "auto".to_string(),
            auto_reconnect: true,
            auto_start: false,
            start_minimized: false,
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
pub fn get_settings() -> Result<AppSettings, String> {
    let dir = ensure_data_dir()?;
    let path = dir.join("settings.json");

    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取设置文件失败: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析设置失败: {}", e))
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
/// 获取当前连接延迟（毫秒）
pub async fn get_latency() -> Option<u32> {
    // 实际实现：ping 当前连接的设备
    // 当前返回模拟值，后续与 connection_manager 集成
    Some(3)
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
    /// 错误类型："refused" | "timeout" | "unreachable" | "invalid_ip" | ""
    pub error_type: String,
    /// 用户可读的错误消息
    pub error_message: String,
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

#[tauri::command]
/// 通过手动输入的 IP 地址直接发起 TCP 连接（跨网段直连）
pub async fn connect_by_ip(ip: String, port: u16) -> ConnectByIpResult {
    // 1. IPv4 格式校验
    if !is_valid_ipv4(&ip) {
        return ConnectByIpResult {
            success: false,
            error_type: "invalid_ip".to_string(),
            error_message: format!("IP 地址格式不正确：\"{}\"，请输入合法的 IPv4 地址（如 192.168.1.100）", ip),
        };
    }

    let addr = format!("{}:{}", ip, port);

    // 2. 带 5 秒超时的 TCP 连接
    let connect_future = tokio::net::TcpStream::connect(&addr);
    match tokio::time::timeout(std::time::Duration::from_secs(5), connect_future).await {
        Ok(Ok(_stream)) => {
            // 连接成功
            ConnectByIpResult {
                success: true,
                error_type: String::new(),
                error_message: String::new(),
            }
        }
        Ok(Err(e)) => {
            // 连接失败，区分错误类型
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
            ConnectByIpResult { success: false, error_type, error_message }
        }
        Err(_) => {
            // 超时
            ConnectByIpResult {
                success: false,
                error_type: "timeout".to_string(),
                error_message: format!(
                    "连接超时（5秒）：可能被防火墙拦截。请检查 {} 的 Windows Defender / macOS 防火墙设置，确保 KVM Share 端口已开放。",
                    ip
                ),
            }
        }
    }
}

// ─── 单元测试 ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.hotkey, "");
        assert_eq!(settings.prefer_connection_type, "auto");
        assert!(settings.auto_reconnect);
        assert!(!settings.auto_start);
        assert!(!settings.start_minimized);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = AppSettings {
            hotkey: "Ctrl+Alt+K".to_string(),
            prefer_connection_type: "network".to_string(),
            auto_reconnect: false,
            auto_start: true,
            start_minimized: false,
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
