//! Mac 端配置持久化
//!
//! 配置文件路径：~/Library/Application Support/KVMini/config.json

use kvm_core::config::KvmConfig;
use std::path::PathBuf;

/// 获取 Mac 配置文件路径
pub fn get_config_path() -> Result<PathBuf, String> {
    let base = dirs::config_dir()
        .ok_or_else(|| "无法获取配置目录".to_string())?;
    let dir = base.join("KVMini");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建配置目录失败: {}", e))?;
    Ok(dir.join("config.json"))
}

/// 保存配置
pub fn save_config(config: &KvmConfig) -> Result<(), String> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    log::info!("[配置] 已保存到 {:?}", path);
    Ok(())
}

/// 加载配置
pub fn load_config() -> KvmConfig {
    let path = match get_config_path() {
        Ok(p) => p,
        Err(_) => return KvmConfig::default(),
    };

    let json = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return KvmConfig::default(),
    };

    match serde_json::from_str(&json) {
        Ok(config) => {
            log::info!("[配置] 已从 {:?} 加载", path);
            config
        }
        Err(e) => {
            log::warn!("[配置] 解析失败 ({}), 使用默认配置", e);
            KvmConfig::default()
        }
    }
}
