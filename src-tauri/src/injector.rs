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
/// 注意：实际的 CGEvent 调用需要辅助功能（Accessibility）权限。
/// 在生产代码中，这里会调用 core-graphics crate 的 CGEvent API。
/// 目前提供接口定义和测试桩，实际 CGEvent 调用在后续集成时补充。
#[cfg(target_os = "macos")]
pub struct MacOSInjector {
    /// 屏幕宽度（像素）
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
        let (px, py) = self.normalize_to_screen(x, y);
        // TODO: 调用 CGEvent::new_mouse_event(CGEventType::MouseMoved, CGPoint { x: px, y: py }, ...)
        // 目前记录日志，实际注入在集成 core-graphics crate 后实现
        let _ = (px, py);
        Ok(())
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), InjectorError> {
        // TODO: 调用 CGEvent::new_mouse_event(CGEventType::LeftMouseDown/Up, ...)
        let _ = (button, pressed);
        Ok(())
    }

    fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<(), InjectorError> {
        // TODO: 调用 CGEvent::new_scroll_event(...)
        let _ = (dx, dy);
        Ok(())
    }

    fn inject_key_event(&self, keycode: u16, pressed: bool) -> Result<(), InjectorError> {
        // TODO: 调用 CGEvent::new_keyboard_event(keycode, pressed)
        let _ = (keycode, pressed);
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) -> Result<(), InjectorError> {
        // TODO: 调用 CGDisplayShowCursor / CGDisplayHideCursor
        let _ = visible;
        Ok(())
    }

    fn warp_cursor_to_edge(&self, edge: u8, pos_ratio: u16) -> Result<(), InjectorError> {
        let (px, py) = edge_to_screen_pos(edge, pos_ratio, self.screen_width, self.screen_height);
        // TODO: 调用 CGWarpMouseCursorPosition(CGPoint { x: px, y: py })
        let _ = (px, py);
        Ok(())
    }
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// Windows 事件注入器（使用 SendInput API）
///
/// 注意：实际的 SendInput 调用需要 winapi crate。
/// 目前提供接口定义和测试桩，实际 SendInput 调用在后续集成时补充。
#[cfg(target_os = "windows")]
pub struct WindowsInjector {
    /// 屏幕宽度（像素，考虑 DPI 缩放后的逻辑像素）
    screen_width: i32,
    /// 屏幕高度（像素）
    screen_height: i32,
    /// DPI 缩放比例（默认 1.0）
    dpi_scale: f32,
}

#[cfg(target_os = "windows")]
impl WindowsInjector {
    pub fn new(screen_width: i32, screen_height: i32, dpi_scale: f32) -> Self {
        Self { screen_width, screen_height, dpi_scale }
    }

    /// 将归一化坐标（* 10000）转换为 SendInput 使用的绝对坐标（0-65535）
    /// Windows SendInput 使用 0-65535 的绝对坐标系
    fn normalize_to_absolute(&self, x: i32, y: i32) -> (i32, i32) {
        // SendInput 绝对坐标：0 = 左/上边缘，65535 = 右/下边缘
        let abs_x = (x as i64 * 65535 / 10000) as i32;
        let abs_y = (y as i64 * 65535 / 10000) as i32;
        (abs_x.clamp(0, 65535), abs_y.clamp(0, 65535))
    }
}

#[cfg(target_os = "windows")]
impl EventInjector for WindowsInjector {
    fn inject_mouse_move(&self, x: i32, y: i32) -> Result<(), InjectorError> {
        let (abs_x, abs_y) = self.normalize_to_absolute(x, y);
        // TODO: 调用 SendInput with MOUSEINPUT { dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, dx: abs_x, dy: abs_y }
        let _ = (abs_x, abs_y);
        Ok(())
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), InjectorError> {
        // TODO: 调用 SendInput with MOUSEINPUT { dwFlags: MOUSEEVENTF_LEFTDOWN/UP/RIGHTDOWN/UP }
        let _ = (button, pressed);
        Ok(())
    }

    fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<(), InjectorError> {
        // TODO: 调用 SendInput with MOUSEINPUT { dwFlags: MOUSEEVENTF_WHEEL, mouseData: dy }
        let _ = (dx, dy);
        Ok(())
    }

    fn inject_key_event(&self, keycode: u16, pressed: bool) -> Result<(), InjectorError> {
        // TODO: 调用 SendInput with KEYBDINPUT { wVk: keycode, dwFlags: KEYEVENTF_KEYUP if !pressed }
        let _ = (keycode, pressed);
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) -> Result<(), InjectorError> {
        // TODO: 调用 ShowCursor(visible)
        let _ = visible;
        Ok(())
    }

    fn warp_cursor_to_edge(&self, edge: u8, pos_ratio: u16) -> Result<(), InjectorError> {
        let (px, py) = edge_to_screen_pos(edge, pos_ratio, self.screen_width, self.screen_height);
        let abs_x = (px as i64 * 65535 / self.screen_width as i64) as i32;
        let abs_y = (py as i64 * 65535 / self.screen_height as i64) as i32;
        // TODO: 调用 SendInput with MOUSEINPUT { dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, dx: abs_x, dy: abs_y }
        let _ = (abs_x, abs_y);
        Ok(())
    }
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
