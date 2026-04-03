/// 显示器布局配置持久化模块
///
/// 将 DisplayLayout 配置保存到本地文件，供边缘切换模块使用。
/// 配置文件路径：
/// - macOS: ~/Library/Application Support/com.kvmini.app/layout_config.json
/// - Windows: %APPDATA%\com.kvmini.app\layout_config.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::edge_switch::DisplayLayout;

/// 布局配置文件内容
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutConfig {
    /// 显示器布局（本机相对于远端的位置）
    pub layout: DisplayLayout,
    /// 本机屏幕宽度（像素）
    pub local_screen_width: i32,
    /// 本机屏幕高度（像素）
    pub local_screen_height: i32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            layout: DisplayLayout::LocalLeftOfRemote,
            local_screen_width: 1920,
            local_screen_height: 1080,
        }
    }
}

/// 获取配置文件路径
pub fn get_config_path() -> Result<PathBuf, String> {
    let base = dirs::config_dir()
        .ok_or_else(|| "无法获取配置目录".to_string())?;
    let dir = base.join("com.kvmini.app");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建配置目录失败: {}", e))?;
    Ok(dir.join("layout_config.json"))
}

/// 保存布局配置
pub fn save_layout_config(config: &LayoutConfig) -> Result<(), String> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

/// 读取布局配置（不存在时返回默认值）
pub fn load_layout_config() -> LayoutConfig {
    let path = match get_config_path() {
        Ok(p) => p,
        Err(_) => return LayoutConfig::default(),
    };

    let json = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return LayoutConfig::default(),
    };

    serde_json::from_str(&json).unwrap_or_default()
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// Tauri 命令：获取布局配置
#[tauri::command]
pub fn get_layout_config() -> Result<LayoutConfig, String> {
    Ok(load_layout_config())
}

/// Tauri 命令：保存布局配置
#[tauri::command]
pub fn set_layout_config(config: LayoutConfig) -> Result<(), String> {
    save_layout_config(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// 辅助：创建临时目录并返回配置文件路径
    fn temp_config_path(dir: &TempDir) -> PathBuf {
        dir.path().join("layout_config.json")
    }

    /// 辅助：直接写入配置文件（绕过 get_config_path）
    fn write_config(path: &PathBuf, config: &LayoutConfig) {
        let json = serde_json::to_string_pretty(config).unwrap();
        fs::write(path, json).unwrap();
    }

    /// 辅助：直接读取配置文件
    fn read_config(path: &PathBuf) -> LayoutConfig {
        let json = fs::read_to_string(path).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    // ── LayoutConfig 序列化/反序列化测试 ────────────────────────────────

    #[test]
    fn test_layout_config_default() {
        let config = LayoutConfig::default();
        assert_eq!(config.layout, DisplayLayout::LocalLeftOfRemote);
        assert_eq!(config.local_screen_width, 1920);
        assert_eq!(config.local_screen_height, 1080);
    }

    #[test]
    fn test_layout_config_serialize_deserialize() {
        let config = LayoutConfig {
            layout: DisplayLayout::LocalRightOfRemote,
            local_screen_width: 2560,
            local_screen_height: 1440,
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: LayoutConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, restored);
    }

    #[test]
    fn test_layout_config_all_layouts() {
        let layouts = [
            DisplayLayout::LocalLeftOfRemote,
            DisplayLayout::LocalRightOfRemote,
            DisplayLayout::LocalAboveRemote,
            DisplayLayout::LocalBelowRemote,
        ];

        for layout in &layouts {
            let config = LayoutConfig {
                layout: *layout,
                local_screen_width: 1920,
                local_screen_height: 1080,
            };
            let json = serde_json::to_string(&config).unwrap();
            let restored: LayoutConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(config, restored);
        }
    }

    #[test]
    fn test_layout_config_save_and_load() {
        let dir = TempDir::new().unwrap();
        let path = temp_config_path(&dir);

        let config = LayoutConfig {
            layout: DisplayLayout::LocalAboveRemote,
            local_screen_width: 1440,
            local_screen_height: 900,
        };

        write_config(&path, &config);
        let loaded = read_config(&path);
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_layout_config_missing_file_returns_default() {
        // 不存在的文件路径，应返回默认值
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");

        // 模拟 load_layout_config 的行为
        let loaded: LayoutConfig = if path.exists() {
            let json = std::fs::read_to_string(&path).unwrap();
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            LayoutConfig::default()
        };

        assert_eq!(loaded, LayoutConfig::default());
    }

    #[test]
    fn test_layout_config_invalid_json_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = temp_config_path(&dir);

        // 写入无效 JSON
        fs::write(&path, "invalid json content").unwrap();

        let loaded: LayoutConfig = {
            let json = std::fs::read_to_string(&path).unwrap();
            serde_json::from_str(&json).unwrap_or_default()
        };

        assert_eq!(loaded, LayoutConfig::default());
    }

    #[test]
    fn test_layout_config_overwrite() {
        let dir = TempDir::new().unwrap();
        let path = temp_config_path(&dir);

        // 第一次写入
        let config1 = LayoutConfig {
            layout: DisplayLayout::LocalLeftOfRemote,
            local_screen_width: 1920,
            local_screen_height: 1080,
        };
        write_config(&path, &config1);

        // 第二次覆盖写入
        let config2 = LayoutConfig {
            layout: DisplayLayout::LocalBelowRemote,
            local_screen_width: 2560,
            local_screen_height: 1440,
        };
        write_config(&path, &config2);

        let loaded = read_config(&path);
        assert_eq!(loaded, config2);
    }

    // ── 边缘映射逻辑测试（布局 → 触发边缘 → 远端入口边缘）──────────────

    #[test]
    fn test_edge_mapping_local_left_of_remote() {
        let config = LayoutConfig {
            layout: DisplayLayout::LocalLeftOfRemote,
            ..Default::default()
        };
        // 本机在左 → 右边缘触发 → 远端左边缘入口
        assert_eq!(config.layout.trigger_edge(), crate::protocol::ScreenEdge::Right);
        assert_eq!(config.layout.remote_entry_edge(), crate::protocol::ScreenEdge::Left);
    }

    #[test]
    fn test_edge_mapping_local_right_of_remote() {
        let config = LayoutConfig {
            layout: DisplayLayout::LocalRightOfRemote,
            ..Default::default()
        };
        assert_eq!(config.layout.trigger_edge(), crate::protocol::ScreenEdge::Left);
        assert_eq!(config.layout.remote_entry_edge(), crate::protocol::ScreenEdge::Right);
    }

    #[test]
    fn test_edge_mapping_local_above_remote() {
        let config = LayoutConfig {
            layout: DisplayLayout::LocalAboveRemote,
            ..Default::default()
        };
        assert_eq!(config.layout.trigger_edge(), crate::protocol::ScreenEdge::Bottom);
        assert_eq!(config.layout.remote_entry_edge(), crate::protocol::ScreenEdge::Top);
    }

    #[test]
    fn test_edge_mapping_local_below_remote() {
        let config = LayoutConfig {
            layout: DisplayLayout::LocalBelowRemote,
            ..Default::default()
        };
        assert_eq!(config.layout.trigger_edge(), crate::protocol::ScreenEdge::Top);
        assert_eq!(config.layout.remote_entry_edge(), crate::protocol::ScreenEdge::Bottom);
    }

    // ── macOS 和 Windows 路径兼容性测试 ─────────────────────────────────

    #[test]
    fn test_config_json_format_is_cross_platform() {
        // 验证 JSON 格式不包含平台特定内容
        let config = LayoutConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("LocalLeftOfRemote"));
        assert!(!json.contains("\\"));  // 不包含 Windows 路径分隔符
    }
}
