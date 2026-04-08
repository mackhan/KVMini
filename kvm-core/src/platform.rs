//! 平台适配 trait 定义
//!
//! 定义所有平台相关功能的抽象接口。
//! kvm-core 通过这些 trait 调用平台功能，由各端 crate 提供具体实现。
//! 本模块不包含任何平台特定代码。

use crate::error::KvmResult;
use serde::{Deserialize, Serialize};

// ── 屏幕相关数据结构 ────────────────────────────────────────────────────────

/// 显示器信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// 显示器 ID（平台特定）
    pub id: u32,
    /// 显示器宽度（逻辑像素）
    pub width: i32,
    /// 显示器高度（逻辑像素）
    pub height: i32,
    /// DPI 缩放比例（1.0 = 100%，2.0 = 200%）
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
    /// 虚拟桌面左上角 X（多显示器时可能为负数）
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
        let norm_x = if self.total_width > 0 {
            (rel_x as i64 * 10000 / self.total_width as i64) as i32
        } else {
            0
        };
        let norm_y = if self.total_height > 0 {
            (rel_y as i64 * 10000 / self.total_height as i64) as i32
        } else {
            0
        };
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

// ── 输入事件数据结构 ────────────────────────────────────────────────────────

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
    Side1 = 3,
    Side2 = 4,
}

impl MouseButton {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2 => Some(Self::Middle),
            3 => Some(Self::Side1),
            4 => Some(Self::Side2),
            _ => None,
        }
    }
}

/// 原始输入事件（平台适配层采集后传递给 kvm-core）
#[derive(Debug, Clone)]
pub enum RawInputEvent {
    /// 鼠标移动（屏幕绝对坐标，逻辑像素）
    MouseMove { x: i32, y: i32 },
    /// 鼠标按下
    MouseDown { button: MouseButton },
    /// 鼠标释放
    MouseUp { button: MouseButton },
    /// 键盘按下（平台原生键码）
    KeyDown { native_keycode: u16 },
    /// 键盘释放（平台原生键码）
    KeyUp { native_keycode: u16 },
    /// 滚轮（dx 水平，dy 垂直）
    Scroll { dx: i32, dy: i32 },
}

// ── 平台适配 trait ──────────────────────────────────────────────────────────

/// 屏幕枚举（获取所有显示器分辨率、位置和 DPI 缩放比例）
pub trait ScreenProvider: Send + Sync {
    /// 枚举本机所有已连接的显示器
    fn enumerate_monitors(&self) -> KvmResult<Vec<MonitorInfo>>;

    /// 获取虚拟桌面信息
    fn get_virtual_desktop(&self) -> KvmResult<VirtualDesktop> {
        let monitors = self.enumerate_monitors()?;
        Ok(VirtualDesktop::from_monitors(monitors))
    }

    /// 注册显示器变化回调（插拔显示器时触发）
    fn on_monitors_changed(&self, callback: Box<dyn Fn() + Send + Sync>) -> KvmResult<()>;
}

/// 键鼠事件采集（Hook 键盘和鼠标输入）
pub trait InputCapture: Send + Sync {
    /// 开始采集键鼠事件
    /// callback 在每次采集到事件时被调用
    fn start_capture(&self, callback: Box<dyn Fn(RawInputEvent) + Send + Sync>) -> KvmResult<()>;

    /// 停止采集
    fn stop_capture(&self) -> KvmResult<()>;

    /// 是否正在采集
    fn is_capturing(&self) -> bool;
}

/// 键鼠事件注入（模拟键盘和鼠标操作）
pub trait InputExecutor: Send + Sync {
    /// 模拟鼠标移动到指定位置（屏幕绝对坐标，逻辑像素）
    fn move_mouse(&self, x: i32, y: i32) -> KvmResult<()>;

    /// 模拟鼠标按下
    fn mouse_down(&self, button: MouseButton) -> KvmResult<()>;

    /// 模拟鼠标释放
    fn mouse_up(&self, button: MouseButton) -> KvmResult<()>;

    /// 模拟键盘按下（平台原生键码）
    fn key_down(&self, native_keycode: u16) -> KvmResult<()>;

    /// 模拟键盘释放（平台原生键码）
    fn key_up(&self, native_keycode: u16) -> KvmResult<()>;

    /// 模拟滚轮
    fn scroll(&self, dx: i32, dy: i32) -> KvmResult<()>;
}

/// 键鼠输入屏蔽（屏蔽/恢复本地键鼠）
pub trait InputBlocker: Send + Sync {
    /// 屏蔽本机键盘和鼠标的所有输入事件
    fn block_input(&self) -> KvmResult<()>;

    /// 恢复本机键鼠输入
    fn unblock_input(&self) -> KvmResult<()>;

    /// 是否正在屏蔽
    fn is_blocked(&self) -> bool;
}

/// 全局快捷键注册与监听
pub trait HotkeyManager: Send + Sync {
    /// 注册全局快捷键
    /// hotkey_id: 快捷键标识
    /// callback: 快捷键触发时的回调
    fn register_hotkey(
        &self,
        hotkey_id: &str,
        callback: Box<dyn Fn() + Send + Sync>,
    ) -> KvmResult<()>;

    /// 取消注册快捷键
    fn unregister_hotkey(&self, hotkey_id: &str) -> KvmResult<()>;

    /// 取消所有快捷键
    fn unregister_all(&self) -> KvmResult<()>;
}

/// 剪贴板读写
pub trait ClipboardProvider: Send + Sync {
    /// 读取剪贴板纯文本内容
    fn get_text(&self) -> KvmResult<Option<String>>;

    /// 写入剪贴板纯文本内容
    fn set_text(&self, text: &str) -> KvmResult<()>;

    /// 注册剪贴板变化回调
    fn on_change(&self, callback: Box<dyn Fn(String) + Send + Sync>) -> KvmResult<()>;

    /// 停止监听剪贴板变化
    fn stop_watching(&self) -> KvmResult<()>;
}

/// 光标控制（显示/隐藏/设置位置）
pub trait CursorController: Send + Sync {
    /// 隐藏光标
    fn hide_cursor(&self) -> KvmResult<()>;

    /// 显示光标
    fn show_cursor(&self) -> KvmResult<()>;

    /// 设置光标位置（屏幕绝对坐标，逻辑像素）
    fn set_position(&self, x: i32, y: i32) -> KvmResult<()>;

    /// 获取当前光标位置
    fn get_position(&self) -> KvmResult<(i32, i32)>;
}

// ── 平台适配层聚合 ──────────────────────────────────────────────────────────

/// 平台适配层聚合接口
///
/// 各端 crate 实现此 trait，将所有平台功能聚合在一起。
pub trait PlatformAdapter: Send + Sync {
    fn screen_provider(&self) -> &dyn ScreenProvider;
    fn input_capture(&self) -> &dyn InputCapture;
    fn input_executor(&self) -> &dyn InputExecutor;
    fn input_blocker(&self) -> &dyn InputBlocker;
    fn hotkey_manager(&self) -> &dyn HotkeyManager;
    fn clipboard_provider(&self) -> &dyn ClipboardProvider;
    fn cursor_controller(&self) -> &dyn CursorController;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_desktop_single_monitor() {
        let monitors = vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 1920);
        assert_eq!(vd.total_height, 1080);
        assert_eq!(vd.origin_x, 0);
        assert_eq!(vd.origin_y, 0);
    }

    #[test]
    fn test_virtual_desktop_two_monitors_side_by_side() {
        let monitors = vec![
            MonitorInfo { id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
                offset_x: 0, offset_y: 0, is_primary: true },
            MonitorInfo { id: 1, width: 2560, height: 1440, dpi_scale: 1.5,
                offset_x: 1920, offset_y: 0, is_primary: false },
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 4480);
        assert_eq!(vd.total_height, 1440);
    }

    #[test]
    fn test_virtual_desktop_negative_offset() {
        let monitors = vec![
            MonitorInfo { id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
                offset_x: 0, offset_y: 0, is_primary: true },
            MonitorInfo { id: 1, width: 1920, height: 1080, dpi_scale: 1.0,
                offset_x: -1920, offset_y: 0, is_primary: false },
        ];
        let vd = VirtualDesktop::from_monitors(monitors);
        assert_eq!(vd.total_width, 3840);
        assert_eq!(vd.origin_x, -1920);
    }

    #[test]
    fn test_virtual_desktop_empty() {
        let vd = VirtualDesktop::from_monitors(vec![]);
        assert_eq!(vd.total_width, 1920);
        assert_eq!(vd.total_height, 1080);
    }

    #[test]
    fn test_normalize_center() {
        let monitors = vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (nx, ny) = vd.normalize(960, 540);
        assert_eq!(nx, 5000);
        assert_eq!(ny, 5000);
    }

    #[test]
    fn test_denormalize_center() {
        let monitors = vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (x, y) = vd.denormalize(5000, 5000);
        assert_eq!(x, 960);
        assert_eq!(y, 540);
    }

    #[test]
    fn test_normalize_denormalize_roundtrip() {
        let monitors = vec![MonitorInfo {
            id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
            offset_x: 0, offset_y: 0, is_primary: true,
        }];
        let vd = VirtualDesktop::from_monitors(monitors);
        let (nx, ny) = vd.normalize(800, 600);
        let (x, y) = vd.denormalize(nx, ny);
        assert!((x - 800).abs() <= 1);
        assert!((y - 600).abs() <= 1);
    }

    #[test]
    fn test_mouse_button_from_u8() {
        assert_eq!(MouseButton::from_u8(0), Some(MouseButton::Left));
        assert_eq!(MouseButton::from_u8(1), Some(MouseButton::Right));
        assert_eq!(MouseButton::from_u8(5), None);
    }
}
