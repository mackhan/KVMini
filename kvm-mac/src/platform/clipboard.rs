//! macOS 剪贴板读写实现（NSPasteboard）
//!
//! 使用 NSPasteboard generalPasteboard 读写纯文本内容。
//! 变化监听通过轮询 changeCount 实现（macOS 无原生推送通知）。

use kvm_core::error::KvmResult;
use kvm_core::platform::ClipboardProvider;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicI64, Ordering}};
use std::thread;
use std::time::Duration;

pub struct MacClipboardProvider {
    watching: Arc<AtomicBool>,
    callback: Arc<Mutex<Option<Box<dyn Fn(String) + Send + Sync>>>>,
}

impl MacClipboardProvider {
    pub fn new() -> Self {
        Self {
            watching: Arc::new(AtomicBool::new(false)),
            callback: Arc::new(Mutex::new(None)),
        }
    }
}

impl ClipboardProvider for MacClipboardProvider {
    fn get_text(&self) -> KvmResult<Option<String>> {
        #[cfg(target_os = "macos")]
        {
            get_pasteboard_text()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(None)
        }
    }

    fn set_text(&self, text: &str) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        {
            set_pasteboard_text(text)
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::debug!("[Mac-stub] set_text: {} bytes", text.len());
            Ok(())
        }
    }

    fn on_change(&self, callback: Box<dyn Fn(String) + Send + Sync>) -> KvmResult<()> {
        *self.callback.lock().unwrap() = Some(callback);
        self.watching.store(true, Ordering::SeqCst);

        // 启动轮询线程
        let watching = Arc::clone(&self.watching);
        let cb = Arc::clone(&self.callback);

        thread::spawn(move || {
            let last_change_count = Arc::new(AtomicI64::new(-1));

            while watching.load(Ordering::SeqCst) {
                #[cfg(target_os = "macos")]
                {
                    let current = get_pasteboard_change_count();
                    let last = last_change_count.load(Ordering::SeqCst);

                    if last == -1 {
                        // 初始化：记录当前 changeCount，不触发回调
                        last_change_count.store(current, Ordering::SeqCst);
                    } else if current != last {
                        last_change_count.store(current, Ordering::SeqCst);
                        // 剪贴板内容已变化，读取新内容
                        if let Ok(Some(text)) = get_pasteboard_text() {
                            if let Ok(guard) = cb.lock() {
                                if let Some(f) = guard.as_ref() {
                                    f(text);
                                }
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(500));
            }

            log::debug!("[Mac] 剪贴板监听线程已退出");
        });

        log::info!("[Mac] 剪贴板变化监听已启动（轮询间隔 500ms）");
        Ok(())
    }

    fn stop_watching(&self) -> KvmResult<()> {
        self.watching.store(false, Ordering::SeqCst);
        *self.callback.lock().unwrap() = None;
        log::info!("[Mac] 剪贴板变化监听已停止");
        Ok(())
    }
}

// ── macOS 实现 ──────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn get_pasteboard_text() -> KvmResult<Option<String>> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CStr;

    unsafe {
        // NSPasteboard *pb = [NSPasteboard generalPasteboard];
        let pb: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() {
            return Ok(None);
        }

        // NSString *type = NSPasteboardTypeString
        let ns_string_type: *mut Object = msg_send![
            class!(NSString),
            stringWithUTF8String: b"public.utf8-plain-text\0".as_ptr()
        ];

        // NSString *text = [pb stringForType:NSPasteboardTypeString]
        let text_obj: *mut Object = msg_send![pb, stringForType: ns_string_type];
        if text_obj.is_null() {
            return Ok(None);
        }

        // const char *utf8 = [text UTF8String]
        let utf8_ptr: *const i8 = msg_send![text_obj, UTF8String];
        if utf8_ptr.is_null() {
            return Ok(None);
        }

        let text = CStr::from_ptr(utf8_ptr)
            .to_string_lossy()
            .into_owned();

        Ok(Some(text))
    }
}

#[cfg(target_os = "macos")]
fn set_pasteboard_text(text: &str) -> KvmResult<()> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;

    let c_text = CString::new(text)
        .map_err(|e| kvm_core::error::KvmError::Platform(format!("文本含 NUL 字节: {}", e)))?;

    unsafe {
        let pb: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() {
            return Err(kvm_core::error::KvmError::Platform("获取 NSPasteboard 失败".into()));
        }

        // [pb clearContents]
        let _: i64 = msg_send![pb, clearContents];

        // NSString *ns_text = [NSString stringWithUTF8String:text]
        let ns_text: *mut Object = msg_send![
            class!(NSString),
            stringWithUTF8String: c_text.as_ptr()
        ];

        // [pb setString:ns_text forType:NSPasteboardTypeString]
        let ns_string_type: *mut Object = msg_send![
            class!(NSString),
            stringWithUTF8String: b"public.utf8-plain-text\0".as_ptr()
        ];

        let success: bool = msg_send![pb, setString: ns_text forType: ns_string_type];
        if !success {
            return Err(kvm_core::error::KvmError::Platform("写入剪贴板失败".into()));
        }
    }

    Ok(())
}

/// 获取 NSPasteboard changeCount（每次内容变化时递增）
#[cfg(target_os = "macos")]
fn get_pasteboard_change_count() -> i64 {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let pb: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() {
            return 0;
        }
        msg_send![pb, changeCount]
    }
}
