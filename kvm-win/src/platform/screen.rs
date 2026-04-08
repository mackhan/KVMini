//! Windows 屏幕枚举实现
//!
//! 使用 Win32 EnumDisplayMonitors 枚举显示器。

use kvm_core::error::{KvmError, KvmResult};
use kvm_core::platform::{MonitorInfo, ScreenProvider, VirtualDesktop};
use std::sync::{Arc, Mutex};

/// Windows 屏幕提供者
pub struct WinScreenProvider {
    change_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl WinScreenProvider {
    pub fn new() -> Self {
        Self {
            change_callback: Arc::new(Mutex::new(None)),
        }
    }
}

impl ScreenProvider for WinScreenProvider {
    fn enumerate_monitors(&self) -> KvmResult<Vec<MonitorInfo>> {
        #[cfg(target_os = "windows")]
        {
            enumerate_monitors_win32()
        }
        #[cfg(not(target_os = "windows"))]
        {
            // 非 Windows 平台返回默认值
            Ok(vec![MonitorInfo {
                id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
                offset_x: 0, offset_y: 0, is_primary: true,
            }])
        }
    }

    fn on_monitors_changed(&self, callback: Box<dyn Fn() + Send + Sync>) -> KvmResult<()> {
        *self.change_callback.lock().unwrap() = Some(callback);
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn enumerate_monitors_win32() -> KvmResult<Vec<MonitorInfo>> {
    use winapi::um::winuser::{EnumDisplayMonitors, GetMonitorInfoW, MONITORINFOEXW};
    use winapi::shared::windef::{HDC, HMONITOR, LPRECT};
    use winapi::shared::minwindef::{BOOL, LPARAM};

    let mut monitors: Vec<MonitorInfo> = Vec::new();
    let monitors_ptr = &mut monitors as *mut Vec<MonitorInfo>;

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _lprect: LPRECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam as *mut Vec<MonitorInfo>);

        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmonitor, &mut info as *mut _ as *mut _) != 0 {
            let rc = info.rcMonitor;
            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;
            let is_primary = (info.dwFlags & 1) != 0;
            let dpi_scale = get_monitor_dpi_scale(hmonitor);

            monitors.push(MonitorInfo {
                id: monitors.len() as u32,
                width,
                height,
                dpi_scale,
                offset_x: rc.left,
                offset_y: rc.top,
                is_primary,
            });
        }
        1
    }

    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(monitor_enum_proc),
            monitors_ptr as LPARAM,
        );
    }

    if monitors.is_empty() {
        monitors.push(get_primary_monitor_fallback());
    }

    Ok(monitors)
}

#[cfg(target_os = "windows")]
unsafe fn get_monitor_dpi_scale(hmonitor: winapi::shared::windef::HMONITOR) -> f32 {
    use winapi::um::libloaderapi::{LoadLibraryW, GetProcAddress};

    let lib_name: Vec<u16> = "shcore.dll\0".encode_utf16().collect();
    let lib = LoadLibraryW(lib_name.as_ptr());
    if lib.is_null() { return 1.0; }

    let func_name = b"GetDpiForMonitor\0";
    let func = GetProcAddress(lib, func_name.as_ptr() as *const i8);
    if func.is_null() { return 1.0; }

    type GetDpiForMonitorFn = unsafe extern "system" fn(
        winapi::shared::windef::HMONITOR, u32, *mut u32, *mut u32
    ) -> i32;
    let get_dpi: GetDpiForMonitorFn = std::mem::transmute(func);

    let mut dpi_x: u32 = 96;
    let mut dpi_y: u32 = 96;
    get_dpi(hmonitor, 0, &mut dpi_x, &mut dpi_y);
    dpi_x as f32 / 96.0
}

#[cfg(target_os = "windows")]
fn get_primary_monitor_fallback() -> MonitorInfo {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        MonitorInfo {
            id: 0,
            width: GetSystemMetrics(SM_CXSCREEN),
            height: GetSystemMetrics(SM_CYSCREEN),
            dpi_scale: 1.0,
            offset_x: 0,
            offset_y: 0,
            is_primary: true,
        }
    }
}
