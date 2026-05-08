//! 配置持久化模块
//!
//! 读写 ~/.config/kvmini/config.json（Windows: %APPDATA%\kvmini\config.json）
//! 首次启动时 peer_ip 为空字符串，不预填任何 IP。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用配置（序列化到 JSON 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 对端 IP（首次启动为空）
    #[serde(default)]
    pub peer_ip: String,

    /// 连接端口
    #[serde(default = "default_port")]
    pub port: u16,

    /// 切换快捷键（如 "Ctrl+Alt+S"）
    #[serde(default = "default_hotkey")]
    pub switch_hotkey: String,

    /// 是否启用边缘触发切换
    #[serde(default = "default_true")]
    pub edge_trigger: bool,

    /// 边缘触发灵敏度（像素）
    #[serde(default = "default_edge_threshold")]
    pub edge_threshold: i32,

    /// 边缘过冲量（像素），需推过边缘 N 像素才真正切换，0 = 贴边即切，默认 20
    #[serde(default = "default_edge_overshoot")]
    pub edge_overshoot_px: i32,

    /// 是否启用剪贴板共享
    #[serde(default = "default_true")]
    pub clipboard_sync: bool,

    /// 是否启用开机自启动
    #[serde(default)]
    pub autostart: bool,

    /// 是否启用 TLS 加密传输
    #[serde(default = "default_true")]
    pub enable_tls: bool,

    /// 是否启用自动重连
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,

    /// 自动重连最大次数（0 = 无限）
    #[serde(default = "default_reconnect_max")]
    pub reconnect_max_attempts: u32,

    /// 自动重连间隔（秒）
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval_secs: u64,
}

fn default_port() -> u16 { 23456 }
fn default_hotkey() -> String { "Ctrl+Alt+S".to_string() }
fn default_true() -> bool { true }
fn default_edge_threshold() -> i32 { 5 }
fn default_edge_overshoot() -> i32 { 5 }
fn default_reconnect_max() -> u32 { 0 }
fn default_reconnect_interval() -> u64 { 3 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            peer_ip: String::new(),
            port: default_port(),
            switch_hotkey: default_hotkey(),
            edge_trigger: true,
            edge_threshold: default_edge_threshold(),
            edge_overshoot_px: default_edge_overshoot(),
            clipboard_sync: true,
            autostart: false,
            enable_tls: true,
            auto_reconnect: true,
            reconnect_max_attempts: default_reconnect_max(),
            reconnect_interval_secs: default_reconnect_interval(),
        }
    }
}

/// 获取配置文件路径
pub fn config_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("kvmini").join("config.json")
}

/// 从文件加载配置（文件不存在时返回默认值）
pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        log::info!("[配置] 配置文件不存在，使用默认配置: {:?}", path);
        return AppConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            match serde_json::from_str::<AppConfig>(&content) {
                Ok(cfg) => {
                    log::info!("[配置] 已加载配置: {:?}", path);
                    cfg
                }
                Err(e) => {
                    log::warn!("[配置] 解析配置失败（{}），使用默认配置", e);
                    AppConfig::default()
                }
            }
        }
        Err(e) => {
            log::warn!("[配置] 读取配置文件失败（{}），使用默认配置", e);
            AppConfig::default()
        }
    }
}

/// 保存配置到文件
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    log::info!("[配置] 已保存配置: {:?}", path);
    Ok(())
}

/// 布局配置（显示器相对位置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// 布局方向：LocalLeftOfRemote / LocalRightOfRemote / LocalAboveRemote / LocalBelowRemote
    #[serde(default = "default_layout")]
    pub layout: String,
    /// 本机主显示器宽度（像素）
    #[serde(default)]
    pub local_screen_width: i32,
    /// 本机主显示器高度（像素）
    #[serde(default)]
    pub local_screen_height: i32,
}

fn default_layout() -> String { "LocalLeftOfRemote".to_string() }

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            layout: default_layout(),
            local_screen_width: 1920,
            local_screen_height: 1080,
        }
    }
}

/// 获取布局配置文件路径
pub fn layout_config_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("kvmini").join("layout.json")
}

/// 加载布局配置
pub fn load_layout_config() -> LayoutConfig {
    let path = layout_config_path();
    if !path.exists() { return LayoutConfig::default(); }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => LayoutConfig::default(),
    }
}

/// 保存布局配置
pub fn save_layout_config(config: &LayoutConfig) -> Result<(), String> {
    let path = layout_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化布局配置失败: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("写入布局配置失败: {}", e))?;
    Ok(())
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：获取当前设置
#[tauri::command]
pub fn get_settings() -> Result<AppConfig, String> {
    Ok(load_config())
}

/// Tauri 命令：保存设置
#[tauri::command]
pub fn save_settings(config: AppConfig) -> Result<(), String> {
    log::info!("[配置] save_settings 被调用: enable_tls={}, port={}", config.enable_tls, config.port);
    // 同步自启动状态
    if config.autostart {
        if let Err(e) = crate::platform::autostart::enable_autostart() {
            log::warn!("[配置] 注册自启动失败: {}", e);
        }
    } else {
        if let Err(e) = crate::platform::autostart::disable_autostart() {
            log::warn!("[配置] 取消自启动失败: {}", e);
        }
    }
    save_config(&config)
}

/// Tauri 命令：重置为默认设置
#[tauri::command]
pub fn reset_settings() -> Result<AppConfig, String> {
    let default_cfg = AppConfig::default();
    save_config(&default_cfg)?;
    Ok(default_cfg)
}
