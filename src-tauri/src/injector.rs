/// 平台事件注入模块
///
/// 定义统一的 EventInjector 接口，macOS 和 Windows 各自实现。
/// 由于 macOS 的 CGEvent 和 Windows 的 SendInput 需要系统级 API，
/// 在单元测试中使用 MockInjector 模拟注入行为。

use crate::protocol::KvmFrame;

/// 注入错误
#[derive(Debug, Clone, PartialEq)]
pub enum InjectorError {
    /// 权限不足（macOS 需要辅助功能权限）
    PermissionDenied,
    /// 平台不支持
    Unsupported,
    /// 系统调用失败
    SystemError(String),
}

impl std::fmt::Display for InjectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "权限不足，请在系统偏好设置中开启辅助功能权限"),
            Self::Unsupported => write!(f, "当前平台不支持此操作"),
            Self::SystemError(msg) => write!(f, "系统调用失败: {}", msg),
        }
    }
}

/// 统一事件注入接口
pub trait EventInjector: Send + Sync {
    /// 注入鼠标移动事件
    /// x, y 为归一化坐标（乘以 10000 的整数），需转换为屏幕像素坐标
    fn inject_mouse_move(&self, x: i32, y: i32) -> Result<(), InjectorError>;

    /// 注入鼠标按键事件
    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), InjectorError>;

    /// 注入鼠标滚轮事件
    fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<(), InjectorError>;

    /// 注入键盘事件
    fn inject_key_event(&self, keycode: u16, pressed: bool) -> Result<(), InjectorError>;

    /// 隐藏/显示本机鼠标光标
    fn set_cursor_visible(&self, visible: bool) -> Result<(), InjectorError>;

    /// 将鼠标移动到屏幕边缘指定位置（用于切换控制权时定位光标）
    /// edge: 0=左, 1=右, 2=上, 3=下；pos_ratio: 位置比例 [0, 10000]
    fn warp_cursor_to_edge(&self, edge: u8, pos_ratio: u16) -> Result<(), InjectorError>;
}

/// 从 KvmFrame 分发注入事件
pub fn inject_frame(injector: &dyn EventInjector, frame: &KvmFrame) -> Result<(), InjectorError> {
    match frame {
        KvmFrame::MouseMove { x, y } => injector.inject_mouse_move(*x, *y),
        KvmFrame::MouseButton { button, pressed } => {
            injector.inject_mouse_button(*button, *pressed)
        }
        KvmFrame::MouseScroll { dx, dy } => injector.inject_mouse_scroll(*dx, *dy),
        KvmFrame::KeyEvent { keycode, pressed } => injector.inject_key_event(*keycode, *pressed),
        KvmFrame::SwitchControl { edge, pos_ratio } => {
            injector.warp_cursor_to_edge(*edge as u8, *pos_ratio)
        }
        _ => Ok(()), // 其他帧类型不需要注入
    }
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

/// macOS 事件注入器（使用 CGEvent API）
///
/// 使用 core-graphics crate 调用 CGEvent API 注入键鼠事件。
/// 需要辅助功能（Accessibility）权限。
#[cfg(target_os = "macos")]
pub struct MacOSInjector {
    /// 屏幕宽度（像素，逻辑像素，非 Retina 物理像素）
    screen_width: i32,
    /// 屏幕高度（像素）
    screen_height: i32,
}

#[cfg(target_os = "macos")]
impl MacOSInjector {
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self { screen_width, screen_height }
    }

    /// 将归一化坐标（* 10000）转换为屏幕像素坐标
    fn normalize_to_screen(&self, x: i32, y: i32) -> (i32, i32) {
        let px = (x as i64 * self.screen_width as i64 / 10000) as i32;
        let py = (y as i64 * self.screen_height as i64 / 10000) as i32;
        (px.clamp(0, self.screen_width - 1), py.clamp(0, self.screen_height - 1))
    }
}

#[cfg(target_os = "macos")]
impl EventInjector for MacOSInjector {
    fn inject_mouse_move(&self, x: i32, y: i32) -> Result<(), InjectorError> {
        use core_graphics::geometry::CGPoint;

        let (px, py) = self.normalize_to_screen(x, y);
        let point = CGPoint { x: px as f64, y: py as f64 };

        // 使用 CGWarpMouseCursorPosition 移动光标（不产生事件，性能更好）
        unsafe {
            macos_warp_cursor(point);
        }
        Ok(())
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), InjectorError> {
        use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        use core_graphics::geometry::CGPoint;

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| InjectorError::SystemError("创建 CGEventSource 失败".to_string()))?;

        // 获取当前鼠标位置
        let location = unsafe { macos_get_cursor_position() };

        let (event_type, cg_button) = match (button, pressed) {
            (0, true) => (CGEventType::LeftMouseDown, CGMouseButton::Left),
            (0, false) => (CGEventType::LeftMouseUp, CGMouseButton::Left),
            (1, true) => (CGEventType::RightMouseDown, CGMouseButton::Right),
            (1, false) => (CGEventType::RightMouseUp, CGMouseButton::Right),
            (_, true) => (CGEventType::OtherMouseDown, CGMouseButton::Center),
            (_, false) => (CGEventType::OtherMouseUp, CGMouseButton::Center),
        };

        let event = CGEvent::new_mouse_event(source, event_type, location, cg_button)
            .map_err(|_| InjectorError::SystemError("创建鼠标事件失败".to_string()))?;
        event.post(core_graphics::event::CGEventTapLocation::HID);
        Ok(())
    }

    fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<(), InjectorError> {
        // 使用 C FFI 直接创建滚轮事件，避免 core-graphics crate API 不兼容
        unsafe {
            macos_post_scroll_event(dx, dy);
        }
        Ok(())
    }

    fn inject_key_event(&self, keycode: u16, pressed: bool) -> Result<(), InjectorError> {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| InjectorError::SystemError("创建 CGEventSource 失败".to_string()))?;

        let event = CGEvent::new_keyboard_event(source, keycode, pressed)
            .map_err(|_| InjectorError::SystemError("创建键盘事件失败".to_string()))?;
        event.post(core_graphics::event::CGEventTapLocation::HID);
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) -> Result<(), InjectorError> {
        unsafe {
            if visible {
                macos_show_cursor();
            } else {
                macos_hide_cursor();
            }
        }
        Ok(())
    }

    fn warp_cursor_to_edge(&self, edge: u8, pos_ratio: u16) -> Result<(), InjectorError> {
        use core_graphics::geometry::CGPoint;
        let (px, py) = edge_to_screen_pos(edge, pos_ratio, self.screen_width, self.screen_height);
        let point = CGPoint { x: px as f64, y: py as f64 };
        unsafe { macos_warp_cursor(point); }
        Ok(())
    }
}

// macOS FFI：CGWarpMouseCursorPosition
#[cfg(target_os = "macos")]
extern "C" {
    fn CGWarpMouseCursorPosition(new_cursor_position: core_graphics::geometry::CGPoint);
    fn CGDisplayShowCursor(display: u32) -> i32;
    fn CGDisplayHideCursor(display: u32) -> i32;
}

/// 使用 C FFI 发送滚轮事件
#[cfg(target_os = "macos")]
unsafe fn macos_post_scroll_event(dx: i32, dy: i32) {
    extern "C" {
        fn CGEventCreateScrollWheelEvent(
            source: *mut std::ffi::c_void,
            units: u32,
            wheel_count: u32,
            wheel1: i32,
            ...
        ) -> *mut std::ffi::c_void;
        fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
        fn CFRelease(cf: *mut std::ffi::c_void);
    }
    // kCGScrollEventUnitPixel = 0
    let event = CGEventCreateScrollWheelEvent(
        std::ptr::null_mut(),
        0, // kCGScrollEventUnitPixel
        2, // wheel_count
        dy,
        dx,
    );
    if !event.is_null() {
        CGEventPost(0, event); // kCGHIDEventTap = 0
        CFRelease(event);
    }
}

/// 移动光标到指定位置
#[cfg(target_os = "macos")]
unsafe fn macos_warp_cursor(point: core_graphics::geometry::CGPoint) {
    CGWarpMouseCursorPosition(point);
}

/// 显示光标
#[cfg(target_os = "macos")]
unsafe fn macos_show_cursor() {
    use core_graphics::display::CGDisplay;
    CGDisplayShowCursor(CGDisplay::main().id);
}

/// 隐藏光标
#[cfg(target_os = "macos")]
unsafe fn macos_hide_cursor() {
    use core_graphics::display::CGDisplay;
    CGDisplayHideCursor(CGDisplay::main().id);
}

/// 获取当前鼠标位置
#[cfg(target_os = "macos")]
unsafe fn macos_get_cursor_position() -> core_graphics::geometry::CGPoint {
    // 简化实现：返回屏幕中心
    use core_graphics::display::CGDisplay;
    let bounds = CGDisplay::main().bounds();
    core_graphics::geometry::CGPoint {
        x: bounds.size.width / 2.0,
        y: bounds.size.height / 2.0,
    }
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// Windows 事件注入器（使用 SendInput API）
///
/// 实现要点：
/// 1. 使用 SetProcessDpiAwarenessContext 确保 DPI 感知
/// 2. 使用 GetSystemMetrics(SM_CXVIRTUALSCREEN) 获取虚拟屏幕尺寸
/// 3. MOUSEEVENTF_ABSOLUTE 坐标范围 0-65535，需正确映射
#[cfg(target_os = "windows")]
pub struct WindowsInjector {
    /// 屏幕宽度（虚拟屏幕，多显示器时为总宽度）
    screen_width: i32,
    /// 屏幕高度（虚拟屏幕）
    screen_height: i32,
    /// DPI 缩放比例（默认 1.0）
    dpi_scale: f32,
}

#[cfg(target_os = "windows")]
impl WindowsInjector {
    pub fn new(screen_width: i32, screen_height: i32, dpi_scale: f32) -> Self {
        // 初始化时设置 DPI 感知
        unsafe { windows_set_dpi_awareness(); }
        Self { screen_width, screen_height, dpi_scale }
    }

    /// 将归一化坐标（* 10000）转换为 SendInput 使用的绝对坐标（0-65535）
    /// 使用虚拟屏幕尺寸（多显示器时为总宽高），确保 DPI 感知
    fn normalize_to_absolute(&self, x: i32, y: i32) -> (i32, i32) {
        // 使用实时虚拟屏幕尺寸（支持动态分辨率变化）
        let (vw, vh) = unsafe { windows_get_virtual_screen_size_ffi() };
        let screen_w = if vw > 0 { vw } else { self.screen_width };
        let screen_h = if vh > 0 { vh } else { self.screen_height };

        // MOUSEEVENTF_ABSOLUTE 坐标：0 = 左/上边缘，65535 = 右/下边缘
        // 公式：abs = norm * 65535 / screen_size
        let abs_x = (x as i64 * 65535 / screen_w as i64) as i32;
        let abs_y = (y as i64 * 65535 / screen_h as i64) as i32;
        (abs_x.clamp(0, 65535), abs_y.clamp(0, 65535))
    }
}

#[cfg(target_os = "windows")]
impl EventInjector for WindowsInjector {
    fn inject_mouse_move(&self, x: i32, y: i32) -> Result<(), InjectorError> {
        use winapi::um::winuser::{SendInput, INPUT, INPUT_MOUSE, MOUSEINPUT, MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_VIRTUALDESK};

        let (abs_x, abs_y) = self.normalize_to_absolute(x, y);

        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: unsafe { std::mem::zeroed() },
        };
        unsafe {
            *input.u.mi_mut() = MOUSEINPUT {
                dx: abs_x,
                dy: abs_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), InjectorError> {
        use winapi::um::winuser::{
            SendInput, INPUT, INPUT_MOUSE, MOUSEINPUT,
            MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
            MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
            MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
        };

        let flags = match (button, pressed) {
            (0, true) => MOUSEEVENTF_LEFTDOWN,
            (0, false) => MOUSEEVENTF_LEFTUP,
            (1, true) => MOUSEEVENTF_RIGHTDOWN,
            (1, false) => MOUSEEVENTF_RIGHTUP,
            (2, true) => MOUSEEVENTF_MIDDLEDOWN,
            (2, false) => MOUSEEVENTF_MIDDLEUP,
            _ => return Ok(()),
        };

        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: unsafe { std::mem::zeroed() },
        };
        unsafe {
            *input.u.mi_mut() = MOUSEINPUT {
                dx: 0, dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<(), InjectorError> {
        use winapi::um::winuser::{SendInput, INPUT, INPUT_MOUSE, MOUSEINPUT, MOUSEEVENTF_WHEEL, MOUSEEVENTF_HWHEEL};

        // 垂直滚轮
        if dy != 0 {
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: unsafe { std::mem::zeroed() },
            };
            unsafe {
                *input.u.mi_mut() = MOUSEINPUT {
                    dx: 0, dy: 0,
                    mouseData: dy as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }

        // 水平滚轮
        if dx != 0 {
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: unsafe { std::mem::zeroed() },
            };
            unsafe {
                *input.u.mi_mut() = MOUSEINPUT {
                    dx: 0, dy: 0,
                    mouseData: dx as u32,
                    dwFlags: MOUSEEVENTF_HWHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
        }
        Ok(())
    }

    fn inject_key_event(&self, keycode: u16, pressed: bool) -> Result<(), InjectorError> {
        use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};

        let flags = if pressed { 0 } else { KEYEVENTF_KEYUP };

        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: unsafe { std::mem::zeroed() },
        };
        unsafe {
            *input.u.ki_mut() = KEYBDINPUT {
                wVk: keycode,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) -> Result<(), InjectorError> {
        use winapi::um::winuser::ShowCursor;
        unsafe { ShowCursor(if visible { 1 } else { 0 }); }
        Ok(())
    }

    fn warp_cursor_to_edge(&self, edge: u8, pos_ratio: u16) -> Result<(), InjectorError> {
        use winapi::um::winuser::SetCursorPos;
        let (px, py) = edge_to_screen_pos(edge, pos_ratio, self.screen_width, self.screen_height);
        unsafe { SetCursorPos(px, py); }
        Ok(())
    }
}

/// 设置 Windows 进程 DPI 感知（Per-Monitor V2）
#[cfg(target_os = "windows")]
unsafe fn windows_set_dpi_awareness() {
    // SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
    // 使用动态加载避免在旧版 Windows 上崩溃
    use winapi::um::libloaderapi::{LoadLibraryW, GetProcAddress};

    let lib_name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    let lib = LoadLibraryW(lib_name.as_ptr());
    if lib.is_null() { return; }

    let func_name = b"SetProcessDpiAwarenessContext\0";
    let func = GetProcAddress(lib, func_name.as_ptr() as *const i8);
    if func.is_null() { return; }

    type SetDpiAwarenessFn = unsafe extern "system" fn(isize) -> bool;
    let set_dpi: SetDpiAwarenessFn = std::mem::transmute(func);
    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4
    let _ = set_dpi(-4isize);
}

/// 获取 Windows 虚拟屏幕尺寸（FFI 版本，用于注入器）
#[cfg(target_os = "windows")]
unsafe fn windows_get_virtual_screen_size_ffi() -> (i32, i32) {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
    let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    (w, h)
}

// ── 跨平台辅助函数 ──────────────────────────────────────────────────────────

/// 将边缘 + 位置比例转换为屏幕像素坐标
/// edge: 1=左, 2=右, 3=上, 4=下（对应 ScreenEdge 枚举值）
pub fn edge_to_screen_pos(edge: u8, pos_ratio: u16, width: i32, height: i32) -> (i32, i32) {
    let ratio = pos_ratio as i64;
    match edge {
        1 => (0, (ratio * height as i64 / 10000) as i32),           // 左边缘
        2 => (width - 1, (ratio * height as i64 / 10000) as i32),   // 右边缘
        3 => ((ratio * width as i64 / 10000) as i32, 0),            // 上边缘
        4 => ((ratio * width as i64 / 10000) as i32, height - 1),   // 下边缘
        _ => (0, 0),
    }
}

/// 创建当前平台的注入器（工厂函数）
pub fn create_platform_injector(
    screen_width: i32,
    screen_height: i32,
) -> Box<dyn EventInjector> {
    #[cfg(target_os = "macos")]
    return Box::new(MacOSInjector::new(screen_width, screen_height));

    #[cfg(target_os = "windows")]
    return Box::new(WindowsInjector::new(screen_width, screen_height, 1.0));

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Box::new(MockInjector::new());
}

// ── Mock 注入器（用于测试和不支持的平台）──────────────────────────────────

/// Mock 注入器，记录所有注入的事件，用于单元测试
pub struct MockInjector {
    pub events: std::sync::Mutex<Vec<MockEvent>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MockEvent {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: u8, pressed: bool },
    MouseScroll { dx: i32, dy: i32 },
    KeyEvent { keycode: u16, pressed: bool },
    CursorVisible(bool),
    WarpToEdge { edge: u8, pos_ratio: u16 },
}

impl MockInjector {
    pub fn new() -> Self {
        Self { events: std::sync::Mutex::new(Vec::new()) }
    }

    pub fn get_events(&self) -> Vec<MockEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl EventInjector for MockInjector {
    fn inject_mouse_move(&self, x: i32, y: i32) -> Result<(), InjectorError> {
        self.events.lock().unwrap().push(MockEvent::MouseMove { x, y });
        Ok(())
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), InjectorError> {
        self.events.lock().unwrap().push(MockEvent::MouseButton { button, pressed });
        Ok(())
    }

    fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<(), InjectorError> {
        self.events.lock().unwrap().push(MockEvent::MouseScroll { dx, dy });
        Ok(())
    }

    fn inject_key_event(&self, keycode: u16, pressed: bool) -> Result<(), InjectorError> {
        self.events.lock().unwrap().push(MockEvent::KeyEvent { keycode, pressed });
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) -> Result<(), InjectorError> {
        self.events.lock().unwrap().push(MockEvent::CursorVisible(visible));
        Ok(())
    }

    fn warp_cursor_to_edge(&self, edge: u8, pos_ratio: u16) -> Result<(), InjectorError> {
        self.events.lock().unwrap().push(MockEvent::WarpToEdge { edge, pos_ratio });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{KvmFrame, ScreenEdge};

    // ── MockInjector 基础测试 ────────────────────────────────────────────

    #[test]
    fn test_mock_inject_mouse_move() {
        let injector = MockInjector::new();
        injector.inject_mouse_move(5000, 3000).unwrap();
        assert_eq!(injector.get_events(), vec![MockEvent::MouseMove { x: 5000, y: 3000 }]);
    }

    #[test]
    fn test_mock_inject_mouse_button() {
        let injector = MockInjector::new();
        injector.inject_mouse_button(0, true).unwrap();  // 左键按下
        injector.inject_mouse_button(0, false).unwrap(); // 左键释放
        assert_eq!(injector.get_events(), vec![
            MockEvent::MouseButton { button: 0, pressed: true },
            MockEvent::MouseButton { button: 0, pressed: false },
        ]);
    }

    #[test]
    fn test_mock_inject_mouse_scroll() {
        let injector = MockInjector::new();
        injector.inject_mouse_scroll(0, -120).unwrap();
        assert_eq!(injector.get_events(), vec![MockEvent::MouseScroll { dx: 0, dy: -120 }]);
    }

    #[test]
    fn test_mock_inject_key_event() {
        let injector = MockInjector::new();
        injector.inject_key_event(0x41, true).unwrap();  // 'A' 按下
        injector.inject_key_event(0x41, false).unwrap(); // 'A' 释放
        assert_eq!(injector.get_events(), vec![
            MockEvent::KeyEvent { keycode: 0x41, pressed: true },
            MockEvent::KeyEvent { keycode: 0x41, pressed: false },
        ]);
    }

    #[test]
    fn test_mock_cursor_visible() {
        let injector = MockInjector::new();
        injector.set_cursor_visible(false).unwrap();
        injector.set_cursor_visible(true).unwrap();
        assert_eq!(injector.get_events(), vec![
            MockEvent::CursorVisible(false),
            MockEvent::CursorVisible(true),
        ]);
    }

    #[test]
    fn test_mock_warp_to_edge() {
        let injector = MockInjector::new();
        injector.warp_cursor_to_edge(2, 5000).unwrap(); // 右边缘，中间位置
        assert_eq!(injector.get_events(), vec![
            MockEvent::WarpToEdge { edge: 2, pos_ratio: 5000 },
        ]);
    }

    // ── inject_frame 分发测试 ────────────────────────────────────────────

    #[test]
    fn test_inject_frame_mouse_move() {
        let injector = MockInjector::new();
        let frame = KvmFrame::MouseMove { x: 1000, y: 2000 };
        inject_frame(&injector, &frame).unwrap();
        assert_eq!(injector.get_events(), vec![MockEvent::MouseMove { x: 1000, y: 2000 }]);
    }

    #[test]
    fn test_inject_frame_key_event() {
        let injector = MockInjector::new();
        let frame = KvmFrame::KeyEvent { keycode: 0x1B, pressed: true }; // ESC
        inject_frame(&injector, &frame).unwrap();
        assert_eq!(injector.get_events(), vec![MockEvent::KeyEvent { keycode: 0x1B, pressed: true }]);
    }

    #[test]
    fn test_inject_frame_switch_control() {
        let injector = MockInjector::new();
        let frame = KvmFrame::SwitchControl { edge: ScreenEdge::Right, pos_ratio: 7500 };
        inject_frame(&injector, &frame).unwrap();
        assert_eq!(injector.get_events(), vec![
            MockEvent::WarpToEdge { edge: ScreenEdge::Right as u8, pos_ratio: 7500 },
        ]);
    }

    #[test]
    fn test_inject_frame_heartbeat_ignored() {
        let injector = MockInjector::new();
        inject_frame(&injector, &KvmFrame::Heartbeat).unwrap();
        assert!(injector.get_events().is_empty());
    }

    // ── edge_to_screen_pos 测试（macOS 和 Windows 坐标系均适用）──────────

    #[test]
    fn test_edge_to_screen_pos_left() {
        // 左边缘，50% 位置，1920x1080 屏幕
        let (x, y) = edge_to_screen_pos(1, 5000, 1920, 1080);
        assert_eq!(x, 0);
        assert_eq!(y, 540); // 1080 * 5000 / 10000 = 540
    }

    #[test]
    fn test_edge_to_screen_pos_right() {
        // 右边缘，25% 位置，1920x1080 屏幕
        let (x, y) = edge_to_screen_pos(2, 2500, 1920, 1080);
        assert_eq!(x, 1919); // width - 1
        assert_eq!(y, 270);  // 1080 * 2500 / 10000 = 270
    }

    #[test]
    fn test_edge_to_screen_pos_top() {
        // 上边缘，75% 位置，1920x1080 屏幕
        let (x, y) = edge_to_screen_pos(3, 7500, 1920, 1080);
        assert_eq!(x, 1440); // 1920 * 7500 / 10000 = 1440
        assert_eq!(y, 0);
    }

    #[test]
    fn test_edge_to_screen_pos_bottom() {
        // 下边缘，100% 位置，2560x1440 屏幕（4K）
        let (x, y) = edge_to_screen_pos(4, 10000, 2560, 1440);
        assert_eq!(x, 2560); // 2560 * 10000 / 10000 = 2560
        assert_eq!(y, 1439); // height - 1
    }

    // ── macOS 坐标归一化测试 ─────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_normalize_to_screen() {
        let injector = MacOSInjector::new(1920, 1080);
        let (px, py) = injector.normalize_to_screen(5000, 5000);
        assert_eq!(px, 960);  // 1920 * 5000 / 10000 = 960
        assert_eq!(py, 540);  // 1080 * 5000 / 10000 = 540
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_normalize_boundary() {
        let injector = MacOSInjector::new(1920, 1080);
        // 超出范围应被 clamp
        let (px, py) = injector.normalize_to_screen(10001, -1);
        assert_eq!(px, 1919); // clamp 到 width - 1
        assert_eq!(py, 0);    // clamp 到 0
    }

    // ── Windows 坐标归一化测试 ───────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_normalize_to_absolute() {
        let injector = WindowsInjector::new(1920, 1080, 1.0);
        let (abs_x, abs_y) = injector.normalize_to_absolute(5000, 5000);
        assert_eq!(abs_x, 32767); // 65535 * 5000 / 10000 = 32767
        assert_eq!(abs_y, 32767);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_normalize_boundary() {
        let injector = WindowsInjector::new(1920, 1080, 1.0);
        let (abs_x, abs_y) = injector.normalize_to_absolute(10000, 10000);
        assert_eq!(abs_x, 65535);
        assert_eq!(abs_y, 65535);
    }

    // ── 跨平台 inject_frame 完整流程测试 ────────────────────────────────

    #[test]
    fn test_inject_multiple_frames_sequence() {
        let injector = MockInjector::new();

        // 模拟一个完整的操作序列
        inject_frame(&injector, &KvmFrame::MouseMove { x: 5000, y: 5000 }).unwrap();
        inject_frame(&injector, &KvmFrame::MouseButton { button: 0, pressed: true }).unwrap();
        inject_frame(&injector, &KvmFrame::MouseButton { button: 0, pressed: false }).unwrap();
        inject_frame(&injector, &KvmFrame::KeyEvent { keycode: 0x41, pressed: true }).unwrap();
        inject_frame(&injector, &KvmFrame::KeyEvent { keycode: 0x41, pressed: false }).unwrap();

        let events = injector.get_events();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0], MockEvent::MouseMove { x: 5000, y: 5000 });
        assert_eq!(events[1], MockEvent::MouseButton { button: 0, pressed: true });
        assert_eq!(events[2], MockEvent::MouseButton { button: 0, pressed: false });
        assert_eq!(events[3], MockEvent::KeyEvent { keycode: 0x41, pressed: true });
        assert_eq!(events[4], MockEvent::KeyEvent { keycode: 0x41, pressed: false });
    }
}
