#![allow(dead_code)]
/// 多显示器支持模块
///
/// 枚举本机所有显示器信息，支持多屏布局配置与跨屏边缘切换。
/// - macOS：使用 CGGetActiveDisplayList 枚举活跃显示器
/// - Windows：使用 EnumDisplayMonitors 枚举显示器
///
/// 坐标归一化基于虚拟桌面总尺寸（所有显示器的包围盒），而非单个显示器。

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// 显示器信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// 显示器 ID（平台特定）
    pub id: u32,
    /// 显示器宽度（逻辑像素）
    pub width: i32,
    /// 显示器高度（逻辑像素）
    pub height: i32,
    /// DPI 缩放比例（1.0 = 100%）
    pub dpi_scale: f32,
    /// 相对于虚拟桌面左上角的 X 偏移（逻辑像素）
    pub offset_x: i32,
    /// 相对于虚拟桌面左上角的 Y 偏移（逻辑像素）
    pub offset_y: i32,
    /// 是否为主显示器
    pub is_primary: bool,
}

/// 虚拟桌面信息（所有显示器的包围盒）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualDesktop {
    /// 虚拟桌面总宽度
    pub total_width: i32,
    /// 虚拟桌面总高度
    pub total_height: i32,
    /// 虚拟桌面左上角 X（通常为 0，但多显示器时可能为负数）
    pub origin_x: i32,
    /// 虚拟桌面左上角 Y
    pub origin_y: i32,
    /// 所有显示器列表
    pub monitors: Vec<MonitorInfo>,
}

impl VirtualDesktop {
    /// 从显示器列表计算虚拟桌面包围盒
    pub fn from_monitors(monitors: Vec<MonitorInfo>) -> Self {
        if monitors.is_empty() {
            return Self {
                total_width: 1920,
                total_height: 1080,
                origin_x: 0,
                origin_y: 0,
                monitors,
            };
        }

        let min_x = monitors.iter().map(|m| m.offset_x).min().unwrap_or(0);
        let min_y = monitors.iter().map(|m| m.offset_y).min().unwrap_or(0);
        let max_x = monitors.iter().map(|m| m.offset_x + m.width).max().unwrap_or(1920);
        let max_y = monitors.iter().map(|m| m.offset_y + m.height).max().unwrap_or(1080);

        Self {
            total_width: max_x - min_x,
            total_height: max_y - min_y,
            origin_x: min_x,
            origin_y: min_y,
            monitors,
        }
    }

    /// 将屏幕绝对坐标归一化为 [0, 10000] 范围
    pub fn normalize(&self, x: i32, y: i32) -> (i32, i32) {
        let rel_x = x - self.origin_x;
        let rel_y = y - self.origin_y;
        let norm_x = (rel_x as i64 * 10000 / self.total_width as i64) as i32;
        let norm_y = (rel_y as i64 * 10000 / self.total_height as i64) as i32;
        (norm_x.clamp(0, 10000), norm_y.clamp(0, 10000))
    }

    /// 将归一化坐标 [0, 10000] 转换为屏幕绝对坐标
    pub fn denormalize(&self, norm_x: i32, norm_y: i32) -> (i32, i32) {
        let x = (norm_x as i64 * self.total_width as i64 / 10000) as i32 + self.origin_x;
        let y = (norm_y as i64 * self.total_height as i64 / 10000) as i32 + self.origin_y;
        (x, y)
    }

    /// 获取主显示器
    pub fn primary_monitor(&self) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.is_primary)
    }
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

/// 枚举 macOS 所有活跃显示器
#[cfg(target_os = "macos")]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    use core_graphics::display::{CGDisplay, CGGetActiveDisplayList};

    let mut display_ids = vec![0u32; 32];
    let mut count: u32 = 0;

    unsafe {
        let result = CGGetActiveDisplayList(
            display_ids.len() as u32,
            display_ids.as_mut_ptr(),
            &mut count,
        );
        if result != 0 {
            // 失败时返回主显示器
            return vec![get_macos_primary_monitor()];
        }
    }

    display_ids.truncate(count as usize);
    let main_id = CGDisplay::main().id;

    display_ids.iter().enumerate().map(|(idx, &id)| {
        let display = CGDisplay::new(id);
        let bounds = display.bounds();
        let mode = display.display_mode();
        let dpi_scale = mode.map(|m| {
            let pixel_w = m.pixel_width() as f32;
            let logical_w = bounds.size.width as f32;
            if logical_w > 0.0 { pixel_w / logical_w } else { 1.0 }
        }).unwrap_or(1.0);

        MonitorInfo {
            id: id,
            width: bounds.size.width as i32,
            height: bounds.size.height as i32,
            dpi_scale,
            offset_x: bounds.origin.x as i32,
            offset_y: bounds.origin.y as i32,
            is_primary: id == main_id,
        }
    }).collect()
}

/// 获取 macOS 主显示器信息（备用）
#[cfg(target_os = "macos")]
fn get_macos_primary_monitor() -> MonitorInfo {
    use core_graphics::display::CGDisplay;
    let display = CGDisplay::main();
    let bounds = display.bounds();
    MonitorInfo {
        id: display.id,
        width: bounds.size.width as i32,
        height: bounds.size.height as i32,
        dpi_scale: 1.0,
        offset_x: 0,
        offset_y: 0,
        is_primary: true,
    }
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// 枚举 Windows 所有显示器
#[cfg(target_os = "windows")]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
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
            let is_primary = (info.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1

            // 获取 DPI 缩放（Windows 8.1+）
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
        1 // 继续枚举
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
        // 备用：返回主显示器
        monitors.push(get_windows_primary_monitor());
    }

    monitors
}

/// 获取 Windows 显示器 DPI 缩放比例
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
    get_dpi(hmonitor, 0, &mut dpi_x, &mut dpi_y); // MDT_EFFECTIVE_DPI = 0
    dpi_x as f32 / 96.0
}

/// 获取 Windows 主显示器信息（备用）
#[cfg(target_os = "windows")]
fn get_windows_primary_monitor() -> MonitorInfo {
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

// ── 跨平台接口 ──────────────────────────────────────────────────────────────

/// 获取当前平台的虚拟桌面信息
pub fn get_virtual_desktop() -> VirtualDesktop {
    let monitors = enumerate_monitors_platform();
    VirtualDesktop::from_monitors(monitors)
}

/// 枚举当前平台的显示器（跨平台入口）
pub fn enumerate_monitors_platform() -> Vec<MonitorInfo> {
    #[cfg(target_os = "macos")]
    return enumerate_monitors();

    #[cfg(target_os = "windows")]
    return enumerate_monitors();

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return vec![MonitorInfo {
        id: 0,
        width: 1920,
        height: 1080,
        dpi_scale: 1.0,
        offset_x: 0,
        offset_y: 0,
        is_primary: true,
    }];
}

/// Tauri 命令：获取所有显示器信息（需求 13.3）
#[tauri::command]
pub fn get_monitors() -> Vec<MonitorInfo> {
    enumerate_monitors_platform()
}

/// Tauri 命令：获取虚拟桌面信息
#[tauri::command]
pub fn get_virtual_desktop_info() -> VirtualDesktop {
    get_virtual_desktop()
}

/// 启动显示器变化监听（在 setup 中调用）
/// 当显示器配置变化时，通过 Tauri 事件 `monitors-changed` 通知前端
pub fn start_monitor_watcher(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_monitors = enumerate_monitors_platform();

        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let current_monitors = enumerate_monitors_platform();

            // 比较显示器数量或分辨率是否变化
            let changed = current_monitors.len() != last_monitors.len()
                || current_monitors.iter().zip(last_monitors.iter()).any(|(a, b)| {
                    a.width != b.width || a.height != b.height
                        || a.offset_x != b.offset_x || a.offset_y != b.offset_y
                });

            if changed {
                last_monitors = current_monitors;
                let _ = app_handle.emit_all("monitors-changed", ());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_monitor(id: u32, w: i32, h: i32, ox: i32, oy: i32, primary: bool) -> MonitorInfo {
        MonitorInfo {
            id,
            width: w,
            height: h,
            dpi_scale: 1.0,
            offset_x: ox,
            offset_y: oy,
            is_primary: primary,
        }
    }

    // ── VirtualDesktop 包围盒计算测试 ────────────────────────────────────

    #[test]
    fn test_virtual_desktop_single_monitor() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 1920);
        assert_eq!(vd.total_height, 1080);
        assert_eq!(vd.origin_x, 0);
        assert_eq!(vd.origin_y, 0);
    }

    #[test]
    fn test_virtual_desktop_two_monitors_side_by_side() {
        // 两个显示器并排：左 1920x1080，右 2560x1440（偏移 1920）
        let monitors = vec![
            make_monitor(0, 1920, 1080, 0, 0, true),
            make_monitor(1, 2560, 1440, 1920, 0, false),
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 1920 + 2560); // 4480
        assert_eq!(vd.total_height, 1440); // max(1080, 1440)
        assert_eq!(vd.origin_x, 0);
    }

    #[test]
    fn test_virtual_desktop_two_monitors_stacked() {
        // 两个显示器上下排列
        let monitors = vec![
            make_monitor(0, 1920, 1080, 0, 0, true),
            make_monitor(1, 1920, 1080, 0, 1080, false),
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 1920);
        assert_eq!(vd.total_height, 2160);
    }

    #[test]
    fn test_virtual_desktop_with_negative_offset() {
        // 左侧显示器在负坐标（macOS 多显示器常见）
        let monitors = vec![
            make_monitor(0, 1920, 1080, 0, 0, true),
            make_monitor(1, 1920, 1080, -1920, 0, false),
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 3840);
        assert_eq!(vd.origin_x, -1920);
    }

    #[test]
    fn test_virtual_desktop_empty_monitors() {
        let vd = VirtualDesktop::from_monitors(vec![]);
        assert_eq!(vd.total_width, 1920); // 默认值
        assert_eq!(vd.total_height, 1080);
    }

    // ── 坐标归一化测试 ───────────────────────────────────────────────────

    #[test]
    fn test_normalize_center_of_single_monitor() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (nx, ny) = vd.normalize(960, 540);
        assert_eq!(nx, 5000); // 960/1920 * 10000 = 5000
        assert_eq!(ny, 5000); // 540/1080 * 10000 = 5000
    }

    #[test]
    fn test_normalize_top_left() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (nx, ny) = vd.normalize(0, 0);
        assert_eq!(nx, 0);
        assert_eq!(ny, 0);
    }

    #[test]
    fn test_normalize_bottom_right() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (nx, ny) = vd.normalize(1920, 1080);
        assert_eq!(nx, 10000);
        assert_eq!(ny, 10000);
    }

    #[test]
    fn test_normalize_two_monitors() {
        // 两个 1920x1080 并排，总宽 3840
        let monitors = vec![
            make_monitor(0, 1920, 1080, 0, 0, true),
            make_monitor(1, 1920, 1080, 1920, 0, false),
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        // 右侧显示器中心：(1920+960, 540) = (2880, 540)
        let (nx, ny) = vd.normalize(2880, 540);
        assert_eq!(nx, 7500); // 2880/3840 * 10000 = 7500
        assert_eq!(ny, 5000);
    }

    // ── 反归一化测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_denormalize_center() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (x, y) = vd.denormalize(5000, 5000);
        assert_eq!(x, 960);
        assert_eq!(y, 540);
    }

    #[test]
    fn test_normalize_denormalize_roundtrip() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        let original = (800, 600);
        let (nx, ny) = vd.normalize(original.0, original.1);
        let (x, y) = vd.denormalize(nx, ny);
        // 允许 1 像素误差（整数除法）
        assert!((x - original.0).abs() <= 1, "x 误差: {} vs {}", x, original.0);
        assert!((y - original.1).abs() <= 1, "y 误差: {} vs {}", y, original.1);
    }

    // ── primary_monitor 测试 ─────────────────────────────────────────────

    #[test]
    fn test_primary_monitor() {
        let monitors = vec![
            make_monitor(0, 1920, 1080, 0, 0, true),
            make_monitor(1, 2560, 1440, 1920, 0, false),
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        let primary = vd.primary_monitor().unwrap();
        assert!(primary.is_primary);
        assert_eq!(primary.id, 0);
    }

    #[test]
    fn test_primary_monitor_none_when_empty() {
        let vd = VirtualDesktop::from_monitors(vec![]);
        assert!(vd.primary_monitor().is_none());
    }

    // ── MonitorInfo 序列化测试 ───────────────────────────────────────────

    #[test]
    fn test_monitor_info_serialize() {
        let monitor = make_monitor(0, 1920, 1080, 0, 0, true);
        let json = serde_json::to_string(&monitor).unwrap();
        let restored: MonitorInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(monitor, restored);
    }

    #[test]
    fn test_virtual_desktop_serialize() {
        let monitors = vec![make_monitor(0, 1920, 1080, 0, 0, true)];
        let vd = VirtualDesktop::from_monitors(monitors);
        let json = serde_json::to_string(&vd).unwrap();
        let restored: VirtualDesktop = serde_json::from_str(&json).unwrap();
        assert_eq!(vd.total_width, restored.total_width);
        assert_eq!(vd.total_height, restored.total_height);
    }

    // ── 平台特定测试 ─────────────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "需要图形环境（CGGetActiveDisplayList），在无头 CI 中跳过"]
    fn test_enumerate_monitors_macos_returns_at_least_one() {
        let monitors = enumerate_monitors_platform();
        assert!(!monitors.is_empty(), "macOS 应至少有一个显示器");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_enumerate_monitors_windows_returns_at_least_one() {
        let monitors = enumerate_monitors();
        assert!(!monitors.is_empty(), "Windows 应至少有一个显示器");
    }
}
