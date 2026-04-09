//! macOS 屏幕枚举实现（CoreGraphics）

use kvm_core::error::KvmResult;
use kvm_core::platform::{MonitorInfo, ScreenProvider};
use std::sync::{Arc, Mutex};

pub struct MacScreenProvider {
    change_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl MacScreenProvider {
    pub fn new() -> Self {
        Self { change_callback: Arc::new(Mutex::new(None)) }
    }
}

impl ScreenProvider for MacScreenProvider {
    fn enumerate_monitors(&self) -> KvmResult<Vec<MonitorInfo>> {
        #[cfg(target_os = "macos")]
        { enumerate_monitors_macos() }
        #[cfg(not(target_os = "macos"))]
        {
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

#[cfg(target_os = "macos")]
fn enumerate_monitors_macos() -> KvmResult<Vec<MonitorInfo>> {
    use core_graphics::display::CGDisplay;

    let display_ids = CGDisplay::active_displays()
        .map_err(|e| kvm_core::error::KvmError::PlatformError {
            api: "CGGetActiveDisplayList".to_string(),
            reason: format!("{:?}", e),
        })?;

    if display_ids.is_empty() {
        return Ok(vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 2.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }]);
    }

    let main_display_id = CGDisplay::main().id;
    let mut monitors = Vec::with_capacity(display_ids.len());

    for display_id in &display_ids {
        let display = CGDisplay::new(*display_id);
        let bounds = display.bounds();
        let width = bounds.size.width as i32;
        let height = bounds.size.height as i32;
        let offset_x = bounds.origin.x as i32;
        let offset_y = bounds.origin.y as i32;
        let pixel_width = display.pixels_wide() as f32;
        let dpi_scale = if width > 0 { pixel_width / width as f32 } else { 1.0 };
        let is_primary = *display_id == main_display_id;

        monitors.push(MonitorInfo {
            id: *display_id, width, height, dpi_scale, offset_x, offset_y, is_primary,
        });
    }

    monitors.sort_by_key(|m| if m.is_primary { 0 } else { 1 });
    Ok(monitors)
}
