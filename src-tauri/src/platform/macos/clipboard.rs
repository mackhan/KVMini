//! macOS 剪贴板读写实现（NSPasteboard）

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
        { get_pasteboard_text() }
        #[cfg(not(target_os = "macos"))]
        { Ok(None) }
    }

    fn set_text(&self, text: &str) -> KvmResult<()> {
        #[cfg(target_os = "macos")]
        { set_pasteboard_text(text) }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }

    fn on_change(&self, callback: Box<dyn Fn(String) + Send + Sync>) -> KvmResult<()> {
        *self.callback.lock().unwrap() = Some(callback);
        self.watching.store(true, Ordering::SeqCst);
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
                        last_change_count.store(current, Ordering::SeqCst);
                    } else if current != last {
                        last_change_count.store(current, Ordering::SeqCst);
                        if let Ok(Some(text)) = get_pasteboard_text() {
                            if let Ok(guard) = cb.lock() {
                                if let Some(f) = guard.as_ref() { f(text); }
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(500));
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

#[cfg(target_os = "macos")]
fn get_pasteboard_text() -> KvmResult<Option<String>> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CStr;
    unsafe {
        let pb: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() { return Ok(None); }
        let ns_string_type: *mut Object = msg_send![
            class!(NSString), stringWithUTF8String: b"public.utf8-plain-text\0".as_ptr()
        ];
        let text_obj: *mut Object = msg_send![pb, stringForType: ns_string_type];
        if text_obj.is_null() { return Ok(None); }
        let utf8_ptr: *const i8 = msg_send![text_obj, UTF8String];
        if utf8_ptr.is_null() { return Ok(None); }
        let text = CStr::from_ptr(utf8_ptr).to_string_lossy().into_owned();
        Ok(Some(text))
    }
}

#[cfg(target_os = "macos")]
fn set_pasteboard_text(text: &str) -> KvmResult<()> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;
    let c_text = CString::new(text)
        .map_err(|e| platform_err!("CString::new", format!("文本含 NUL 字节: {}", e)))?;
    unsafe {
        let pb: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() {
            return Err(platform_err!("NSPasteboard", "获取 NSPasteboard 失败"));
        }
        let _: i64 = msg_send![pb, clearContents];
        let ns_text: *mut Object = msg_send![
            class!(NSString), stringWithUTF8String: c_text.as_ptr()
        ];
        let ns_string_type: *mut Object = msg_send![
            class!(NSString), stringWithUTF8String: b"public.utf8-plain-text\0".as_ptr()
        ];
        let success: bool = msg_send![pb, setString: ns_text forType: ns_string_type];
        if !success {
            return Err(platform_err!("NSPasteboard::setString", "写入剪贴板失败"));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn get_pasteboard_change_count() -> i64 {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let pb: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() { return 0; }
        msg_send![pb, changeCount]
    }
}
