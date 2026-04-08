//! Windows 开机自启动（注册表）

/// 注册表路径
const REG_RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
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
    Ok(())
}

/// 取消开机自启动
pub fn disable_autostart() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(REG_RUN_KEY, KEY_WRITE) {
            let _ = key.delete_value(APP_NAME);
        }
        log::info!("[自启动] 已取消");
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
    }
    false
}
