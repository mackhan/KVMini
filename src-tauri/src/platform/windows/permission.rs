//! Windows 权限检测（UAC Elevation）
//!
//! 检测当前进程是否以管理员权限运行。
//! 若未提权，可请求提权（以 runas verb 重启当前进程后退出）。

use kvm_core::error::KvmResult;
use kvm_core::platform::{FocusKeeper, PermissionProvider, PermissionStatus};

pub struct WinPermissionProvider;

impl WinPermissionProvider {
    pub fn new() -> Self {
        Self
    }
}

impl PermissionProvider for WinPermissionProvider {
    fn is_elevated(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            is_elevated_impl()
        }
        #[cfg(not(target_os = "windows"))]
        {
            true
        }
    }

    fn request_elevation(&self) -> KvmResult<()> {
        #[cfg(target_os = "windows")]
        {
            request_elevation_impl()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(())
        }
    }

    fn permission_status(&self) -> PermissionStatus {
        let elevated = self.is_elevated();
        PermissionStatus {
            elevated,
            accessibility: true, // Win 不需要辅助功能权限
            input_monitoring: true, // Win 不需要输入监控权限
            admin_required: !elevated,
        }
    }
}

#[cfg(target_os = "windows")]
fn is_elevated_impl() -> bool {
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY, HANDLE};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            log::warn!("[Win/Permission] OpenProcessToken 失败");
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size: u32 = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );

        winapi::um::handleapi::CloseHandle(token);

        if result == 0 {
            log::warn!("[Win/Permission] GetTokenInformation 失败");
            return false;
        }

        elevation.TokenIsElevated != 0
    }
}

#[cfg(target_os = "windows")]
fn request_elevation_impl() -> KvmResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::shellapi::ShellExecuteW;

    let exe_path = std::env::current_exe().map_err(|e| {
        kvm_core::error::KvmError::PlatformError {
            api: "current_exe".to_string(),
            reason: format!("获取当前进程路径失败: {}", e),
        }
    })?;

    let exe_wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "runas\0".encode_utf16().collect();

    unsafe {
        let result = ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            exe_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            winapi::um::winuser::SW_SHOWNORMAL,
        );

        if (result as usize) <= 32 {
            return Err(kvm_core::error::KvmError::PlatformError {
                api: "ShellExecuteW".to_string(),
                reason: format!("UAC 提权启动失败: result={}", result as usize),
            });
        }
    }

    // 成功启动新进程后退出当前进程
    log::info!("[Win/Permission] UAC 提权进程已启动，当前进程即将退出");
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_status_structure() {
        let provider = WinPermissionProvider::new();
        let status = provider.permission_status();
        // accessibility 和 input_monitoring 在 Win 上始终 true
        assert!(status.accessibility);
        assert!(status.input_monitoring);
        // admin_required 是 elevated 的反值
        assert_eq!(status.admin_required, !status.elevated);
    }
}
