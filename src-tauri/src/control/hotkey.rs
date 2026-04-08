/// Windows 端全局快捷键管理子模块
///
/// 通过 Tauri GlobalShortcutManager 注册全局快捷键。
/// 支持角色切换、断开连接等操作。

use serde::{Serialize, Deserialize};
use tauri::Manager;

/// 快捷键动作
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HotkeyAction {
    /// 切换主控/被控角色
    ToggleRole,
    /// 断开当前连接
    Disconnect,
    /// 自定义动作
    Custom(String),
}

/// 快捷键配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// 快捷键字符串（Tauri 格式，如 "Ctrl+Alt+K"）
    pub shortcut: String,
    /// 触发的动作
    pub action: HotkeyAction,
    /// 是否启用
    pub enabled: bool,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Alt+K".to_string(),
            action: HotkeyAction::ToggleRole,
            enabled: true,
        }
    }
}

/// Windows 端快捷键管理器
pub struct WinHotkeyManager {
    configs: Vec<HotkeyConfig>,
}

impl WinHotkeyManager {
    pub fn new() -> Self {
        Self { configs: Vec::new() }
    }

    /// 注册默认快捷键
    pub fn with_defaults() -> Self {
        let mut mgr = Self::new();
        mgr.configs.push(HotkeyConfig::default());
        mgr
    }

    /// 获取所有快捷键配置
    pub fn configs(&self) -> &[HotkeyConfig] {
        &self.configs
    }

    /// 添加快捷键配置
    pub fn add(&mut self, config: HotkeyConfig) {
        self.configs.push(config);
    }

    /// 注册到 Tauri GlobalShortcutManager
    pub fn register_all(
        &self,
        shortcut_mgr: &mut impl tauri::GlobalShortcutManager,
        app_handle: tauri::AppHandle,
    ) {
        for config in &self.configs {
            if !config.enabled { continue; }
            let handle = app_handle.clone();
            let action = config.action.clone();
            let shortcut = config.shortcut.clone();

            match shortcut_mgr.register(&shortcut, move || {
                // 触发 hotkey-triggered 事件，前端处理角色切换
                let _ = handle.emit_all("hotkey-triggered", serde_json::json!({
                    "action": format!("{:?}", action)
                }));
            }) {
                Ok(_) => println!("[快捷键] 已注册: {}", shortcut),
                Err(e) => eprintln!("[快捷键] 注册失败: {} ({})", shortcut, e),
            }
        }
    }
}

impl Default for WinHotkeyManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：获取快捷键配置
#[tauri::command]
pub fn get_hotkey_config() -> HotkeyConfig {
    HotkeyConfig::default()
}

/// Tauri 命令：检查快捷键是否冲突
#[tauri::command]
pub fn check_hotkey_available(shortcut: String) -> bool {
    // 简单检查：非空且格式合理
    !shortcut.is_empty() && shortcut.contains('+')
}
