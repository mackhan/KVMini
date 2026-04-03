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
}
