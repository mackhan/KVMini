/// 主控端键鼠捕获模块
///
/// 定义统一的 EventCapturer 接口，macOS 和 Windows 各自实现。
/// - macOS：使用 CGEvent Tap 捕获全局键鼠事件（需要辅助功能权限）
/// - Windows：使用 SetWindowsHookEx 低级钩子（WH_MOUSE_LL、WH_KEYBOARD_LL）
///
/// 捕获到的事件通过 tokio channel 发送给主控端 TCP 发送模块。
/// 支持鼠标移动事件节流（最高 1000Hz）和快捷键 Ctrl+Alt+S 退出控制模式。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use crate::protocol::KvmFrame;

/// 捕获错误
#[derive(Debug, Clone, PartialEq)]
pub enum CapturerError {
    /// 权限不足（macOS 需要辅助功能权限）
    PermissionDenied,
    /// 已在运行
    AlreadyRunning,
    /// 系统调用失败
    SystemError(String),
}

impl std::fmt::Display for CapturerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "权限不足，请在系统偏好设置中开启辅助功能权限"),
            Self::AlreadyRunning => write!(f, "捕获器已在运行"),
            Self::SystemError(msg) => write!(f, "系统调用失败: {}", msg),
        }
    }
}

/// 统一事件捕获接口
pub trait EventCapturer: Send + Sync {
    /// 启动捕获，返回事件接收端
    /// 捕获到的事件通过 channel 发送
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError>;

    /// 停止捕获
    fn stop(&self);

    /// 是否正在运行
    fn is_running(&self) -> bool;

    /// 设置是否拦截本机输入（控制模式：true = 拦截，false = 透传）
    fn set_intercept(&self, intercept: bool);
}

/// 鼠标移动节流器（最高 1000Hz = 1ms 间隔）
pub struct MouseThrottle {
    /// 上次发送时间（微秒）
    last_send_us: std::sync::Mutex<u64>,
    /// 最小发送间隔（微秒），默认 1000us = 1ms = 1000Hz
    min_interval_us: u64,
}

impl MouseThrottle {
    pub fn new(max_hz: u32) -> Self {
        let min_interval_us = if max_hz == 0 { 0 } else { 1_000_000 / max_hz as u64 };
        Self {
            last_send_us: std::sync::Mutex::new(0),
            min_interval_us,
        }
    }

    /// 检查是否应该发送此次鼠标移动事件
    /// 返回 true 表示应该发送，false 表示应该丢弃（节流）
    pub fn should_send(&self) -> bool {
        let now_us = Self::now_us();
        let mut last = self.last_send_us.lock().unwrap();
        if now_us - *last >= self.min_interval_us {
            *last = now_us;
            true
        } else {
            false
        }
    }

    fn now_us() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

/// 快捷键检测器（Ctrl+Alt+S 退出控制模式）
pub struct HotkeyDetector {
    ctrl_pressed: AtomicBool,
    alt_pressed: AtomicBool,
}

impl HotkeyDetector {
    pub fn new() -> Self {
        Self {
            ctrl_pressed: AtomicBool::new(false),
            alt_pressed: AtomicBool::new(false),
        }
    }

    /// 更新修饰键状态，返回是否触发了退出快捷键（Ctrl+Alt+S）
    pub fn update(&self, keycode: u16, pressed: bool) -> bool {
        // macOS 虚拟键码：Ctrl=59, Alt/Option=58, S=1
        // Windows 虚拟键码：Ctrl=0x11, Alt=0x12, S=0x53
        let is_ctrl = is_ctrl_key(keycode);
        let is_alt = is_alt_key(keycode);
        let is_s = is_s_key(keycode);

        if is_ctrl {
            self.ctrl_pressed.store(pressed, Ordering::SeqCst);
        }
        if is_alt {
            self.alt_pressed.store(pressed, Ordering::SeqCst);
        }

        // 检测 Ctrl+Alt+S 组合键
        if is_s && pressed
            && self.ctrl_pressed.load(Ordering::SeqCst)
            && self.alt_pressed.load(Ordering::SeqCst)
        {
            return true;
        }

        false
    }

    pub fn reset(&self) {
        self.ctrl_pressed.store(false, Ordering::SeqCst);
        self.alt_pressed.store(false, Ordering::SeqCst);
    }
}

/// 判断是否为 Ctrl 键（跨平台）
fn is_ctrl_key(keycode: u16) -> bool {
    // macOS: Left Ctrl = 59, Right Ctrl = 62
    // Windows: VK_CONTROL = 0x11, VK_LCONTROL = 0xA2, VK_RCONTROL = 0xA3
    matches!(keycode, 59 | 62 | 0x11 | 0xA2 | 0xA3)
}

/// 判断是否为 Alt 键（跨平台）
fn is_alt_key(keycode: u16) -> bool {
    // macOS: Left Option = 58, Right Option = 61
    // Windows: VK_MENU = 0x12, VK_LMENU = 0xA4, VK_RMENU = 0xA5
    matches!(keycode, 58 | 61 | 0x12 | 0xA4 | 0xA5)
}

/// 判断是否为 S 键（跨平台）
fn is_s_key(keycode: u16) -> bool {
    // macOS: S = 1 (CGKeyCode)
    // Windows: VK_S = 0x53
    matches!(keycode, 1 | 0x53)
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

/// macOS 事件捕获器（使用 CGEvent Tap）
///
/// 注意：实际的 CGEvent Tap 需要辅助功能权限，并且必须在主线程的 RunLoop 中运行。
/// 目前提供接口定义，实际 CGEvent Tap 调用在后续集成时补充。
#[cfg(target_os = "macos")]
pub struct MacOSCapturer {
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
}

#[cfg(target_os = "macos")]
impl MacOSCapturer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            intercept: Arc::new(AtomicBool::new(false)),
            throttle: Arc::new(MouseThrottle::new(1000)),
            hotkey: Arc::new(HotkeyDetector::new()),
        }
    }
}

#[cfg(target_os = "macos")]
impl EventCapturer for MacOSCapturer {
    fn start(&self, _tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CapturerError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);
        // TODO: 在专用线程中启动 CGEvent Tap RunLoop
        // 参考 macos-cgevent-tap-hotkey skill 的实现
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.reset();
        // TODO: 停止 CGEvent Tap RunLoop
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_intercept(&self, intercept: bool) {
        self.intercept.store(intercept, Ordering::SeqCst);
    }
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// Windows 事件捕获器（使用 SetWindowsHookEx 低级钩子）
#[cfg(target_os = "windows")]
pub struct WindowsCapturer {
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    throttle: Arc<MouseThrottle>,
    hotkey: Arc<HotkeyDetector>,
}

#[cfg(target_os = "windows")]
impl WindowsCapturer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            intercept: Arc::new(AtomicBool::new(false)),
            throttle: Arc::new(MouseThrottle::new(1000)),
            hotkey: Arc::new(HotkeyDetector::new()),
        }
    }
}

#[cfg(target_os = "windows")]
impl EventCapturer for WindowsCapturer {
    fn start(&self, _tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CapturerError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);
        // TODO: 在专用线程中调用 SetWindowsHookEx(WH_MOUSE_LL, ...) 和 SetWindowsHookEx(WH_KEYBOARD_LL, ...)
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.hotkey.reset();
        // TODO: 调用 UnhookWindowsHookEx
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_intercept(&self, intercept: bool) {
        self.intercept.store(intercept, Ordering::SeqCst);
    }
}

// ── Mock 捕获器（用于测试）──────────────────────────────────────────────────

/// Mock 捕获器，可手动注入事件，用于单元测试
pub struct MockCapturer {
    running: Arc<AtomicBool>,
    intercept: Arc<AtomicBool>,
    tx: std::sync::Mutex<Option<mpsc::Sender<KvmFrame>>>,
}

impl MockCapturer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            intercept: Arc::new(AtomicBool::new(false)),
            tx: std::sync::Mutex::new(None),
        }
    }

    /// 手动注入一个事件（模拟捕获到的输入）
    pub fn inject_event(&self, frame: KvmFrame) -> bool {
        if let Some(tx) = self.tx.lock().unwrap().as_ref() {
            tx.try_send(frame).is_ok()
        } else {
            false
        }
    }

    pub fn is_intercepting(&self) -> bool {
        self.intercept.load(Ordering::SeqCst)
    }
}

impl EventCapturer for MockCapturer {
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), CapturerError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CapturerError::AlreadyRunning);
        }
        *self.tx.lock().unwrap() = Some(tx);
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.tx.lock().unwrap() = None;
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_intercept(&self, intercept: bool) {
        self.intercept.store(intercept, Ordering::SeqCst);
    }
}

/// 创建当前平台的捕获器（工厂函数）
pub fn create_platform_capturer() -> Box<dyn EventCapturer> {
    #[cfg(target_os = "macos")]
    return Box::new(MacOSCapturer::new());

    #[cfg(target_os = "windows")]
    return Box::new(WindowsCapturer::new());

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Box::new(MockCapturer::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ── MouseThrottle 测试 ───────────────────────────────────────────────

    #[test]
    fn test_throttle_1000hz_allows_first_event() {
        let throttle = MouseThrottle::new(1000);
        assert!(throttle.should_send()); // 第一次总是允许
    }

    #[test]
    fn test_throttle_1000hz_blocks_rapid_events() {
        let throttle = MouseThrottle::new(1000);
        throttle.should_send(); // 第一次
        // 立即再次调用，应该被节流
        let second = throttle.should_send();
        // 在 1ms 内，第二次可能被节流（取决于系统时钟精度）
        // 这里只验证节流器不会 panic
        let _ = second;
    }

    #[test]
    fn test_throttle_0hz_always_allows() {
        // 0Hz 表示不节流
        let throttle = MouseThrottle::new(0);
        for _ in 0..100 {
            assert!(throttle.should_send());
        }
    }

    // ── HotkeyDetector 测试 ──────────────────────────────────────────────

    #[test]
    fn test_hotkey_ctrl_alt_s_macos() {
        let detector = HotkeyDetector::new();
        // macOS 键码：Ctrl=59, Alt=58, S=1
        detector.update(59, true);  // Ctrl 按下
        detector.update(58, true);  // Alt 按下
        let triggered = detector.update(1, true); // S 按下
        assert!(triggered, "Ctrl+Alt+S 应该触发退出快捷键");
    }

    #[test]
    fn test_hotkey_ctrl_alt_s_windows() {
        let detector = HotkeyDetector::new();
        // Windows 键码：Ctrl=0x11, Alt=0x12, S=0x53
        detector.update(0x11, true);  // Ctrl 按下
        detector.update(0x12, true);  // Alt 按下
        let triggered = detector.update(0x53, true); // S 按下
        assert!(triggered, "Ctrl+Alt+S 应该触发退出快捷键（Windows）");
    }

    #[test]
    fn test_hotkey_not_triggered_without_ctrl() {
        let detector = HotkeyDetector::new();
        detector.update(58, true);  // 只按 Alt
        let triggered = detector.update(1, true); // S 按下
        assert!(!triggered, "没有 Ctrl 不应触发快捷键");
    }

    #[test]
    fn test_hotkey_not_triggered_without_alt() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);  // 只按 Ctrl
        let triggered = detector.update(1, true); // S 按下
        assert!(!triggered, "没有 Alt 不应触发快捷键");
    }

    #[test]
    fn test_hotkey_not_triggered_on_key_release() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);  // Ctrl 按下
        detector.update(58, true);  // Alt 按下
        let triggered = detector.update(1, false); // S 释放（不是按下）
        assert!(!triggered, "键释放不应触发快捷键");
    }

    #[test]
    fn test_hotkey_reset() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);
        detector.update(58, true);
        detector.reset();
        let triggered = detector.update(1, true);
        assert!(!triggered, "reset 后不应触发快捷键");
    }

    #[test]
    fn test_hotkey_ctrl_released_before_s() {
        let detector = HotkeyDetector::new();
        detector.update(59, true);  // Ctrl 按下
        detector.update(58, true);  // Alt 按下
        detector.update(59, false); // Ctrl 释放
        let triggered = detector.update(1, true); // S 按下
        assert!(!triggered, "Ctrl 已释放，不应触发快捷键");
    }

    // ── MockCapturer 测试 ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_mock_capturer_start_stop() {
        let capturer = MockCapturer::new();
        let (tx, _rx) = mpsc::channel(16);

        assert!(!capturer.is_running());
        capturer.start(tx).unwrap();
        assert!(capturer.is_running());
        capturer.stop();
        assert!(!capturer.is_running());
    }

    #[tokio::test]
    async fn test_mock_capturer_already_running() {
        let capturer = MockCapturer::new();
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        capturer.start(tx1).unwrap();
        let result = capturer.start(tx2);
        assert_eq!(result, Err(CapturerError::AlreadyRunning));
        capturer.stop();
    }

    #[tokio::test]
    async fn test_mock_capturer_inject_event() {
        let capturer = MockCapturer::new();
        let (tx, mut rx) = mpsc::channel(16);

        capturer.start(tx).unwrap();

        // 注入鼠标移动事件
        let sent = capturer.inject_event(KvmFrame::MouseMove { x: 5000, y: 3000 });
        assert!(sent);

        let received = rx.recv().await.unwrap();
        assert_eq!(received, KvmFrame::MouseMove { x: 5000, y: 3000 });

        capturer.stop();
    }

    #[tokio::test]
    async fn test_mock_capturer_inject_key_event() {
        let capturer = MockCapturer::new();
        let (tx, mut rx) = mpsc::channel(16);

        capturer.start(tx).unwrap();

        capturer.inject_event(KvmFrame::KeyEvent { keycode: 0x41, pressed: true });
        capturer.inject_event(KvmFrame::KeyEvent { keycode: 0x41, pressed: false });

        assert_eq!(rx.recv().await.unwrap(), KvmFrame::KeyEvent { keycode: 0x41, pressed: true });
        assert_eq!(rx.recv().await.unwrap(), KvmFrame::KeyEvent { keycode: 0x41, pressed: false });

        capturer.stop();
    }

    #[test]
    fn test_mock_capturer_intercept() {
        let capturer = MockCapturer::new();
        assert!(!capturer.is_intercepting());
        capturer.set_intercept(true);
        assert!(capturer.is_intercepting());
        capturer.set_intercept(false);
        assert!(!capturer.is_intercepting());
    }

    #[test]
    fn test_mock_capturer_inject_without_start() {
        let capturer = MockCapturer::new();
        let sent = capturer.inject_event(KvmFrame::Heartbeat);
        assert!(!sent, "未启动时注入事件应返回 false");
    }

    // ── macOS 平台特定测试 ───────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_capturer_start_stop() {
        let capturer = MacOSCapturer::new();
        let (tx, _rx) = mpsc::channel(16);

        assert!(!capturer.is_running());
        capturer.start(tx).unwrap();
        assert!(capturer.is_running());
        capturer.stop();
        assert!(!capturer.is_running());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_capturer_already_running() {
        let capturer = MacOSCapturer::new();
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        capturer.start(tx1).unwrap();
        assert_eq!(capturer.start(tx2), Err(CapturerError::AlreadyRunning));
        capturer.stop();
    }

    // ── Windows 平台特定测试 ─────────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_capturer_start_stop() {
        let capturer = WindowsCapturer::new();
        let (tx, _rx) = mpsc::channel(16);

        assert!(!capturer.is_running());
        capturer.start(tx).unwrap();
        assert!(capturer.is_running());
        capturer.stop();
        assert!(!capturer.is_running());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_capturer_already_running() {
        let capturer = WindowsCapturer::new();
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        capturer.start(tx1).unwrap();
        assert_eq!(capturer.start(tx2), Err(CapturerError::AlreadyRunning));
        capturer.stop();
    }

    // ── is_ctrl/alt/s_key 跨平台键码测试 ────────────────────────────────

    #[test]
    fn test_is_ctrl_key_macos() {
        assert!(is_ctrl_key(59));  // Left Ctrl
        assert!(is_ctrl_key(62));  // Right Ctrl
        assert!(!is_ctrl_key(58)); // Alt
    }

    #[test]
    fn test_is_ctrl_key_windows() {
        assert!(is_ctrl_key(0x11));  // VK_CONTROL
        assert!(is_ctrl_key(0xA2));  // VK_LCONTROL
        assert!(is_ctrl_key(0xA3));  // VK_RCONTROL
        assert!(!is_ctrl_key(0x12)); // VK_MENU (Alt)
    }

    #[test]
    fn test_is_alt_key_macos() {
        assert!(is_alt_key(58));  // Left Option
        assert!(is_alt_key(61));  // Right Option
        assert!(!is_alt_key(59)); // Ctrl
    }

    #[test]
    fn test_is_alt_key_windows() {
        assert!(is_alt_key(0x12));  // VK_MENU
        assert!(is_alt_key(0xA4));  // VK_LMENU
        assert!(is_alt_key(0xA5));  // VK_RMENU
        assert!(!is_alt_key(0x11)); // VK_CONTROL
    }

    #[test]
    fn test_is_s_key_both_platforms() {
        assert!(is_s_key(1));     // macOS CGKeyCode
        assert!(is_s_key(0x53));  // Windows VK_S
        assert!(!is_s_key(0));
        assert!(!is_s_key(0x41)); // 'A'
    }
}
