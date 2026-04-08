//! Mac 开机自启动（LaunchAgent）

use std::path::PathBuf;

const PLIST_LABEL: &str = "com.kvmini.agent";

/// 获取 LaunchAgent plist 文件路径
fn get_plist_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "无法获取 Home 目录".to_string())?;
    Ok(home.join("Library/LaunchAgents").join(format!("{}.plist", PLIST_LABEL)))
}

/// 注册开机自启动
pub fn enable_autostart() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("获取程序路径失败: {}", e))?;

    let plist_path = get_plist_path()?;

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>"#,
        PLIST_LABEL,
        exe_path.to_string_lossy()
    );

    // 确保目录存在
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 LaunchAgents 目录失败: {}", e))?;
    }

    std::fs::write(&plist_path, plist_content)
        .map_err(|e| format!("写入 plist 失败: {}", e))?;

    log::info!("[自启动] 已注册: {:?}", plist_path);
    Ok(())
}

/// 取消开机自启动
pub fn disable_autostart() -> Result<(), String> {
    let plist_path = get_plist_path()?;
    if plist_path.exists() {
        std::fs::remove_file(&plist_path)
            .map_err(|e| format!("删除 plist 失败: {}", e))?;
        log::info!("[自启动] 已取消");
    }
    Ok(())
}

/// 检查是否已注册自启动
pub fn is_autostart_enabled() -> bool {
    get_plist_path().map(|p| p.exists()).unwrap_or(false)
}
