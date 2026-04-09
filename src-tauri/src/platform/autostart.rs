//! 开机自启动（跨平台）
//!
//! Windows：注册表 HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! macOS：LaunchAgent plist 文件

// ── Windows 实现 ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
const REG_RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const APP_NAME: &str = "KVMini";

/// 注册开机自启动
pub fn enable_autostart() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("获取程序路径失败: {}", e))?;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(REG_RUN_KEY)
            .map_err(|e| format!("打开注册表失败: {}", e))?;
        key.set_value(APP_NAME, &exe_path.to_string_lossy().to_string())
            .map_err(|e| format!("写入注册表失败: {}", e))?;
        log::info!("[自启动] 已注册: {:?}", exe_path);
    }
    #[cfg(target_os = "macos")]
    {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("获取程序路径失败: {}", e))?;
        let plist_path = get_plist_path()?;
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.kvmini.agent</string>
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
            exe_path.to_string_lossy()
        );
        if let Some(parent) = plist_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建 LaunchAgents 目录失败: {}", e))?;
        }
        std::fs::write(&plist_path, plist_content)
            .map_err(|e| format!("写入 plist 失败: {}", e))?;
        log::info!("[自启动] 已注册: {:?}", plist_path);
    }
    Ok(())
}

/// 取消开机自启动
pub fn disable_autostart() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(REG_RUN_KEY, winreg::enums::KEY_WRITE) {
            let _ = key.delete_value(APP_NAME);
        }
        log::info!("[自启动] 已取消");
    }
    #[cfg(target_os = "macos")]
    {
        let plist_path = get_plist_path()?;
        if plist_path.exists() {
            std::fs::remove_file(&plist_path)
                .map_err(|e| format!("删除 plist 失败: {}", e))?;
            log::info!("[自启动] 已取消");
        }
    }
    Ok(())
}

/// 检查是否已注册自启动
pub fn is_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(REG_RUN_KEY) {
            return key.get_value::<String, _>(APP_NAME).is_ok();
        }
        false
    }
    #[cfg(target_os = "macos")]
    {
        get_plist_path().map(|p| p.exists()).unwrap_or(false)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { false }
}

#[cfg(target_os = "macos")]
fn get_plist_path() -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "无法获取 Home 目录".to_string())?;
    Ok(home.join("Library/LaunchAgents/com.kvmini.agent.plist"))
}
