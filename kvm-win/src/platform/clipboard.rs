//! Windows 剪贴板读写实现
//!
//! 使用 Win32 Clipboard API 读写剪贴板。

use kvm_core::error::{KvmError, KvmResult};
use kvm_core::platform::ClipboardProvider;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Windows 剪贴板提供者
pub struct WinClipboardProvider {
    watching: AtomicBool,
    callback: Arc<Mutex<Option<Box<dyn Fn(String) + Send + Sync>>>>,
}

impl WinClipboardProvider {
    pub fn new() -> Self {
        Self {
            watching: AtomicBool::new(false),
            callback: Arc::new(Mutex::new(None)),
        }
    }
}

impl ClipboardProvider for WinClipboardProvider {
    fn get_text(&self) -> KvmResult<Option<String>> {
        #[cfg(target_os = "windows")]
        {
            return win32_get_clipboard_text();
        }
        #[cfg(not(target_os = "windows"))]
        Ok(None)
    }

    fn set_text(&self, text: &str) -> KvmResult<()> {
        log::debug!("[Win] 设置剪贴板: {} 字节", text.len());
        #[cfg(target_os = "windows")]
        {
            return win32_set_clipboard_text(text);
        }
        #[cfg(not(target_os = "windows"))]
        Ok(())
    }

    fn on_change(&self, callback: Box<dyn Fn(String) + Send + Sync>) -> KvmResult<()> {
        *self.callback.lock().unwrap() = Some(callback);
        self.watching.store(true, Ordering::SeqCst);

        let cb = Arc::clone(&self.callback);
        let watching = Arc::new(AtomicBool::new(true));
        let watching_clone = Arc::clone(&watching);

        // 启动剪贴板监听线程（轮询方式）
        std::thread::spawn(move || {
            let mut last_text: Option<String> = None;
            while watching_clone.load(Ordering::SeqCst) {
                #[cfg(target_os = "windows")]
                {
                    if let Ok(Some(text)) = win32_get_clipboard_text() {
                        let changed = match &last_text {
                            None => true,
                            Some(prev) => prev != &text,
                        };
                        if changed {
                            last_text = Some(text.clone());
                            if let Ok(guard) = cb.lock() {
                                if let Some(f) = guard.as_ref() {
                                    f(text);
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });

        Ok(())
    }

    fn stop_watching(&self) -> KvmResult<()> {
        self.watching.store(false, Ordering::SeqCst);
        *self.callback.lock().unwrap() = None;
        Ok(())
    }
}

/// 从 Win32 剪贴板读取文本
#[cfg(target_os = "windows")]
fn win32_get_clipboard_text() -> KvmResult<Option<String>> {
    use winapi::um::winuser::{OpenClipboard, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, CF_UNICODETEXT};
    use winapi::um::winbase::GlobalLock;
    use winapi::um::winbase::GlobalUnlock;

    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return Ok(None);
        }

        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Ok(None);
        }

        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            CloseClipboard();
            return Ok(None);
        }

        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return Ok(None);
        }

        // 计算字符串长度
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16_lossy(slice);

        GlobalUnlock(handle);
        CloseClipboard();

        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }
}

/// 向 Win32 剪贴板写入文本
#[cfg(target_os = "windows")]
fn win32_set_clipboard_text(text: &str) -> KvmResult<()> {
    use winapi::um::winuser::{OpenClipboard, CloseClipboard, SetClipboardData, EmptyClipboard, CF_UNICODETEXT};
    use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let size = wide.len() * 2;

    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, size);
        if handle.is_null() {
            return Err(KvmError::PlatformError {
                api: "GlobalAlloc".to_string(),
                reason: "内存分配失败".to_string(),
            });
        }

        let ptr = GlobalLock(handle) as *mut u16;
        if ptr.is_null() {
            return Err(KvmError::PlatformError {
                api: "GlobalLock".to_string(),
                reason: "内存锁定失败".to_string(),
            });
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        GlobalUnlock(handle);

        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err(KvmError::PlatformError {
                api: "OpenClipboard".to_string(),
                reason: "打开剪贴板失败".to_string(),
            });
        }

        EmptyClipboard();
        SetClipboardData(CF_UNICODETEXT, handle);
        CloseClipboard();
    }

    Ok(())
}
