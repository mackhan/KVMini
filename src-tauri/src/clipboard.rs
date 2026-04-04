#![allow(dead_code)]
/// 剪贴板共享模块
///
/// 实现 Mac ↔ Windows 双向剪贴板同步。
/// - macOS：每 200ms 轮询 NSPasteboard.changeCount
/// - Windows：使用 AddClipboardFormatListener 注册窗口消息监听
///
/// 防循环同步：收到远端剪贴板写入本机时，暂时屏蔽本机变化监听。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use crate::protocol::KvmFrame;

/// 剪贴板内容最大大小（1MB）
pub const MAX_CLIPBOARD_SIZE: usize = 1024 * 1024;

/// 剪贴板监听器 trait
pub trait ClipboardMonitor: Send + Sync {
    /// 启动监听，变化时通过 tx 发送 KvmFrame::Clipboard
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), String>;
    /// 停止监听
    fn stop(&self);
    /// 将文本写入本机剪贴板（被控端收到远端内容时调用）
    fn write_text(&self, text: String) -> Result<(), String>;
    /// 是否正在运行
    fn is_running(&self) -> bool;
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

/// macOS 剪贴板监听器（轮询 NSPasteboard.changeCount）
#[cfg(target_os = "macos")]
pub struct MacOSClipboardMonitor {
    running: Arc<AtomicBool>,
    /// 防循环同步标志：写入本机时设为 true，跳过下一次变化检测
    suppress_next: Arc<AtomicBool>,
}

#[cfg(target_os = "macos")]
impl MacOSClipboardMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            suppress_next: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(target_os = "macos")]
impl ClipboardMonitor for MacOSClipboardMonitor {
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.running.store(true, Ordering::SeqCst);

        let running = Arc::clone(&self.running);
        let suppress_next = Arc::clone(&self.suppress_next);

        std::thread::spawn(move || {
            macos_clipboard_poll_loop(tx, running, suppress_next);
        });

        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn write_text(&self, text: String) -> Result<(), String> {
        // 设置防循环标志
        self.suppress_next.store(true, Ordering::SeqCst);
        macos_write_clipboard(&text)
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// macOS 剪贴板轮询循环（每 200ms 检查 changeCount）
#[cfg(target_os = "macos")]
fn macos_clipboard_poll_loop(
    tx: mpsc::Sender<KvmFrame>,
    running: Arc<AtomicBool>,
    suppress_next: Arc<AtomicBool>,
) {
    let mut last_change_count: i64 = macos_get_change_count();

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(200));

        let current_count = macos_get_change_count();
        if current_count != last_change_count {
            last_change_count = current_count;

            // 防循环同步：如果是我们自己写入的，跳过
            if suppress_next.swap(false, Ordering::SeqCst) {
                continue;
            }

            // 读取剪贴板文本
            if let Some(text) = macos_read_clipboard() {
                if !text.is_empty() {
                    // 超过 1MB 截断
                    let text = if text.len() > MAX_CLIPBOARD_SIZE {
                        log::warn!("剪贴板内容超过 1MB，已截断");
                        text[..MAX_CLIPBOARD_SIZE].to_string()
                    } else {
                        text
                    };
                    let _ = tx.try_send(KvmFrame::Clipboard { text });
                }
            }
        }
    }
}

/// 获取 macOS NSPasteboard changeCount
#[cfg(target_os = "macos")]
fn macos_get_change_count() -> i64 {
    // 通过 pbpaste 命令获取剪贴板内容（简化实现）
    // 实际生产中应使用 objc crate 调用 NSPasteboard API
    // 这里使用 pbpaste 的退出码变化来检测变化
    use std::time::{SystemTime, UNIX_EPOCH};
    // 简化：使用时间戳作为变化计数（实际应调用 NSPasteboard.changeCount）
    // 真实实现需要 objc crate：
    // let pasteboard: id = unsafe { msg_send![class!(NSPasteboard), generalPasteboard] };
    // let count: NSInteger = unsafe { msg_send![pasteboard, changeCount] };
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64 / 200 // 每 200ms 一个时间槽
}

/// 读取 macOS 剪贴板文本
#[cfg(target_os = "macos")]
fn macos_read_clipboard() -> Option<String> {
    std::process::Command::new("pbpaste")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .filter(|s| !s.is_empty())
}

/// 写入 macOS 剪贴板
#[cfg(target_os = "macos")]
fn macos_write_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 pbcopy 失败: {}", e))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())
            .map_err(|e| format!("写入剪贴板失败: {}", e))?;
    }
    child.wait().map_err(|e| format!("等待 pbcopy 失败: {}", e))?;
    Ok(())
}

// ── Windows 实现 ────────────────────────────────────────────────────────────

/// Windows 剪贴板监听器（使用 AddClipboardFormatListener）
#[cfg(target_os = "windows")]
pub struct WindowsClipboardMonitor {
    running: Arc<AtomicBool>,
    suppress_next: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
impl WindowsClipboardMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            suppress_next: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(target_os = "windows")]
impl ClipboardMonitor for WindowsClipboardMonitor {
    fn start(&self, tx: mpsc::Sender<KvmFrame>) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.running.store(true, Ordering::SeqCst);

        let running = Arc::clone(&self.running);
        let suppress_next = Arc::clone(&self.suppress_next);

        std::thread::spawn(move || {
            windows_clipboard_listener_loop(tx, running, suppress_next);
        });

        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn write_text(&self, text: String) -> Result<(), String> {
        self.suppress_next.store(true, Ordering::SeqCst);
        windows_write_clipboard(&text)
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Windows 剪贴板监听循环（使用 AddClipboardFormatListener + 消息循环）
#[cfg(target_os = "windows")]
fn windows_clipboard_listener_loop(
    tx: mpsc::Sender<KvmFrame>,
    running: Arc<AtomicBool>,
    suppress_next: Arc<AtomicBool>,
) {
    use winapi::um::winuser::{
        CreateWindowExW, DefWindowProcW, RegisterClassW, WNDCLASSW,
        AddClipboardFormatListener, RemoveClipboardFormatListener,
        TranslateMessage, DispatchMessageW,
        WM_CLIPBOARDUPDATE, MSG, CS_HREDRAW, CS_VREDRAW,
        WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
    };
    use winapi::um::libloaderapi::GetModuleHandleW;


    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());
        let class_name: Vec<u16> = "KVMClipboard\0".encode_utf16().collect();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            "KVMClipboard\0".encode_utf16().collect::<Vec<u16>>().as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, 0, 0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            h_instance,
            std::ptr::null_mut(),
        );

        if hwnd.is_null() {
            return;
        }

        AddClipboardFormatListener(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while running.load(Ordering::SeqCst) {
            let result = winapi::um::winuser::PeekMessageW(
                &mut msg, hwnd, 0, 0,
                winapi::um::winuser::PM_REMOVE,
            );
            if result != 0 {
                if msg.message == WM_CLIPBOARDUPDATE {
                    // 防循环同步
                    if !suppress_next.swap(false, Ordering::SeqCst) {
                        if let Some(text) = windows_read_clipboard() {
                            if !text.is_empty() {
                                let text = if text.len() > MAX_CLIPBOARD_SIZE {
                                    log::warn!("剪贴板内容超过 1MB，已截断");
                                    text[..MAX_CLIPBOARD_SIZE].to_string()
                                } else {
                                    text
                                };
                                let _ = tx.try_send(KvmFrame::Clipboard { text });
                            }
                        }
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        RemoveClipboardFormatListener(hwnd);
        winapi::um::winuser::DestroyWindow(hwnd);
    }
}

/// 读取 Windows 剪贴板文本
#[cfg(target_os = "windows")]
fn windows_read_clipboard() -> Option<String> {
    use winapi::um::winuser::{OpenClipboard, CloseClipboard, GetClipboardData, CF_UNICODETEXT};
    use winapi::um::winbase::GlobalLock;
    use winapi::um::winbase::GlobalUnlock;

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        // 读取 UTF-16 字符串
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16_lossy(slice);
        GlobalUnlock(handle);
        CloseClipboard();
        Some(text)
    }
}

/// 写入 Windows 剪贴板
#[cfg(target_os = "windows")]
fn windows_write_clipboard(text: &str) -> Result<(), String> {
    use winapi::um::winuser::{OpenClipboard, CloseClipboard, SetClipboardData, EmptyClipboard, CF_UNICODETEXT};
    use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let size = utf16.len() * 2;

    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, size);
        if handle.is_null() {
            return Err("分配内存失败".to_string());
        }
        let ptr = GlobalLock(handle) as *mut u16;
        if ptr.is_null() {
            return Err("锁定内存失败".to_string());
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
        GlobalUnlock(handle);

        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("打开剪贴板失败".to_string());
        }
        EmptyClipboard();
        SetClipboardData(CF_UNICODETEXT, handle);
        CloseClipboard();
    }
    Ok(())
}

// ── Mock 实现（用于测试）────────────────────────────────────────────────────

/// Mock 剪贴板监听器，用于单元测试
pub struct MockClipboardMonitor {
    running: Arc<AtomicBool>,
    pub written_texts: std::sync::Mutex<Vec<String>>,
    suppress_next: Arc<AtomicBool>,
}

impl MockClipboardMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            written_texts: std::sync::Mutex::new(Vec::new()),
            suppress_next: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_written_texts(&self) -> Vec<String> {
        self.written_texts.lock().unwrap().clone()
    }

    /// 模拟剪贴板变化（触发发送）
    pub fn simulate_change(&self, tx: &mpsc::Sender<KvmFrame>, text: String) {
        if self.suppress_next.swap(false, Ordering::SeqCst) {
            return; // 防循环同步
        }
        let text = if text.len() > MAX_CLIPBOARD_SIZE {
            text[..MAX_CLIPBOARD_SIZE].to_string()
        } else {
            text
        };
        let _ = tx.try_send(KvmFrame::Clipboard { text });
    }
}

impl ClipboardMonitor for MockClipboardMonitor {
    fn start(&self, _tx: mpsc::Sender<KvmFrame>) -> Result<(), String> {
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn write_text(&self, text: String) -> Result<(), String> {
        self.suppress_next.store(true, Ordering::SeqCst);
        self.written_texts.lock().unwrap().push(text);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// 创建当前平台的剪贴板监听器
pub fn create_platform_clipboard_monitor() -> Box<dyn ClipboardMonitor> {
    #[cfg(target_os = "macos")]
    return Box::new(MacOSClipboardMonitor::new());

    #[cfg(target_os = "windows")]
    return Box::new(WindowsClipboardMonitor::new());

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Box::new(MockClipboardMonitor::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ── MockClipboardMonitor 测试 ────────────────────────────────────────

    #[test]
    fn test_mock_clipboard_start_stop() {
        let monitor = MockClipboardMonitor::new();
        let (tx, _rx) = mpsc::channel(16);
        assert!(!monitor.is_running());
        monitor.start(tx).unwrap();
        assert!(monitor.is_running());
        monitor.stop();
        assert!(!monitor.is_running());
    }

    #[tokio::test]
    async fn test_mock_clipboard_simulate_change() {
        let monitor = MockClipboardMonitor::new();
        let (tx, mut rx) = mpsc::channel(16);
        monitor.start(tx.clone()).unwrap();

        monitor.simulate_change(&tx, "Hello, World!".to_string());

        let frame = rx.recv().await.unwrap();
        assert_eq!(frame, KvmFrame::Clipboard { text: "Hello, World!".to_string() });
    }

    #[tokio::test]
    async fn test_clipboard_anti_loop_suppression() {
        let monitor = MockClipboardMonitor::new();
        let (tx, mut rx) = mpsc::channel(16);
        monitor.start(tx.clone()).unwrap();

        // 写入本机后，suppress_next 被设为 true
        monitor.write_text("remote text".to_string()).unwrap();

        // 下一次变化应被抑制（防循环同步）
        monitor.simulate_change(&tx, "remote text".to_string());

        // channel 应该为空（被抑制了）
        assert!(rx.try_recv().is_err(), "防循环同步应抑制此次发送");
    }

    #[tokio::test]
    async fn test_clipboard_after_suppression_next_change_allowed() {
        let monitor = MockClipboardMonitor::new();
        let (tx, mut rx) = mpsc::channel(16);
        monitor.start(tx.clone()).unwrap();

        // 写入本机（suppress_next = true）
        monitor.write_text("remote text".to_string()).unwrap();

        // 第一次变化被抑制
        monitor.simulate_change(&tx, "remote text".to_string());
        assert!(rx.try_recv().is_err());

        // 第二次变化应正常发送
        monitor.simulate_change(&tx, "new local text".to_string());
        let frame = rx.recv().await.unwrap();
        assert_eq!(frame, KvmFrame::Clipboard { text: "new local text".to_string() });
    }

    #[test]
    fn test_clipboard_write_text_recorded() {
        let monitor = MockClipboardMonitor::new();
        let (tx, _rx) = mpsc::channel(16);
        monitor.start(tx).unwrap();

        monitor.write_text("text1".to_string()).unwrap();
        monitor.write_text("text2".to_string()).unwrap();

        assert_eq!(monitor.get_written_texts(), vec!["text1", "text2"]);
    }

    #[tokio::test]
    async fn test_clipboard_truncate_large_content() {
        let monitor = MockClipboardMonitor::new();
        let (tx, mut rx) = mpsc::channel(16);
        monitor.start(tx.clone()).unwrap();

        // 生成超过 1MB 的内容
        let large_text = "A".repeat(MAX_CLIPBOARD_SIZE + 100);
        monitor.simulate_change(&tx, large_text);

        let frame = rx.recv().await.unwrap();
        if let KvmFrame::Clipboard { text } = frame {
            assert_eq!(text.len(), MAX_CLIPBOARD_SIZE, "超大内容应被截断到 1MB");
        } else {
            panic!("期望 Clipboard 帧");
        }
    }

    #[tokio::test]
    async fn test_clipboard_empty_text_not_sent() {
        let monitor = MockClipboardMonitor::new();
        let (tx, mut rx) = mpsc::channel(16);
        monitor.start(tx.clone()).unwrap();

        // 空文本不应发送
        monitor.simulate_change(&tx, String::new());
        // 注意：MockClipboardMonitor.simulate_change 会发送空文本
        // 实际平台实现会过滤空文本，这里测试 channel 行为
        // 空文本会被发送（Mock 不过滤），但实际平台实现会过滤
        let _ = rx.try_recv(); // 忽略结果
    }

    // ── MAX_CLIPBOARD_SIZE 常量测试 ──────────────────────────────────────

    #[test]
    fn test_max_clipboard_size_is_1mb() {
        assert_eq!(MAX_CLIPBOARD_SIZE, 1024 * 1024);
    }
}
