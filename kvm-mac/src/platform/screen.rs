//! macOS 屏幕枚举实现（CoreGraphics）
//!
//! 使用 CGGetActiveDisplayList 枚举所有活跃显示器，
//! 获取分辨率、位置和 DPI 缩放信息。

use kvm_core::error::KvmResult;
use kvm_core::platform::{MonitorInfo, ScreenProvider, VirtualDesktop};
use std::sync::{Arc, Mutex};

pub struct MacScreenProvider {
    change_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl MacScreenProvider {
    pub fn new() -> Self {
        Self {
            change_callback: Arc::new(Mutex::new(None)),
        }
    }
}

impl ScreenProvider for MacScreenProvider {
    fn enumerate_monitors(&self) -> KvmResult<Vec<MonitorInfo>> {
        #[cfg(target_os = "macos")]
        {
            enumerate_monitors_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            // 非 macOS 平台返回默认单显示器
            Ok(vec![MonitorInfo {
                id: 0,
                width: 1920,
                height: 1080,
                dpi_scale: 1.0,
                offset_x: 0,
                offset_y: 0,
                is_primary: true,
            }])
        }
    }

    fn on_monitors_changed(&self, callback: Box<dyn Fn() + Send + Sync>) -> KvmResult<()> {
        *self.change_callback.lock().unwrap() = Some(callback);
        // TODO: 注册 CGDisplayRegisterReconfigurationCallback
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn enumerate_monitors_macos() -> KvmResult<Vec<MonitorInfo>> {
    use core_graphics::display::{CGDisplay, CGDisplayBounds};

    // 获取所有活跃显示器 ID
    let display_ids = CGDisplay::active_displays()
        .map_err(|e| kvm_core::error::KvmError::Platform(format!("CGGetActiveDisplayList 失败: {:?}", e)))?;

    if display_ids.is_empty() {
        log::warn!("[Mac] 未检测到活跃显示器，使用默认值");
        return Ok(vec![MonitorInfo {
            id: 0,
            width: 1920,
            height: 1080,
            dpi_scale: 2.0,
            offset_x: 0,
            offset_y: 0,
            is_primary: true,
        }]);
    }

    let main_display_id = CGDisplay::main().id;
    let mut monitors = Vec::with_capacity(display_ids.len());

    for display_id in &display_ids {
        let display = CGDisplay::new(*display_id);
        let bounds = display.bounds();

        // 逻辑分辨率（points）
        let width = bounds.size.width as i32;
        let height = bounds.size.height as i32;
        let offset_x = bounds.origin.x as i32;
        let offset_y = bounds.origin.y as i32;

        // DPI 缩放：物理像素 / 逻辑像素
        let pixel_width = display.pixels_wide() as f32;
        let dpi_scale = if width > 0 {
            pixel_width / width as f32
        } else {
            1.0
        };

        let is_primary = *display_id == main_display_id;

        log::debug!(
            "[Mac] 显示器 id={} {}x{} @({},{}) dpi={:.1} primary={}",
            display_id, width, height, offset_x, offset_y, dpi_scale, is_primary
        );

        monitors.push(MonitorInfo {
            id: *display_id,
            width,
            height,
            dpi_scale,
            offset_x,
            offset_y,
            is_primary,
        });
    }

    // 主显示器排在第一位
    monitors.sort_by_key(|m| if m.is_primary { 0 } else { 1 });

    Ok(monitors)
}
