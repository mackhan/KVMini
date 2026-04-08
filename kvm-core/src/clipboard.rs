//! 剪贴板共享协议逻辑
//!
//! 控制权切换时自动同步剪贴板内容。
//! 详细实现将在任务 6 中完成。

/// 剪贴板内容最大大小（10MB）
pub const MAX_CLIPBOARD_SIZE: usize = 10 * 1024 * 1024;

/// 剪贴板同步管理器
pub struct ClipboardSync {
    /// 是否启用剪贴板共享
    enabled: bool,
    /// 上次同步的内容哈希（避免重复同步）
    last_hash: Option<u64>,
}

impl ClipboardSync {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last_hash: None,
        }
    }

    /// 设置是否启用
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 检查内容是否应该同步（大小限制 + 去重）
    pub fn should_sync(&mut self, text: &str) -> bool {
        if !self.enabled {
            return false;
        }

        if text.len() > MAX_CLIPBOARD_SIZE {
            log::warn!("剪贴板内容超过 {}MB，跳过同步", MAX_CLIPBOARD_SIZE / 1024 / 1024);
            return false;
        }

        let hash = simple_hash(text);
        if self.last_hash == Some(hash) {
            return false;
        }

        self.last_hash = Some(hash);
        true
    }

    /// 记录已接收的内容（避免回环同步）
    pub fn mark_received(&mut self, text: &str) {
        self.last_hash = Some(simple_hash(text));
    }
}

/// 简单哈希函数（用于去重，非安全用途）
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_sync_enabled() {
        let mut sync = ClipboardSync::new(true);
        assert!(sync.should_sync("hello"));
    }

    #[test]
    fn test_clipboard_sync_disabled() {
        let mut sync = ClipboardSync::new(false);
        assert!(!sync.should_sync("hello"));
    }

    #[test]
    fn test_clipboard_sync_dedup() {
        let mut sync = ClipboardSync::new(true);
        assert!(sync.should_sync("hello"));
        assert!(!sync.should_sync("hello")); // 重复内容
        assert!(sync.should_sync("world")); // 新内容
    }

    #[test]
    fn test_clipboard_sync_size_limit() {
        let mut sync = ClipboardSync::new(true);
        let large = "x".repeat(MAX_CLIPBOARD_SIZE + 1);
        assert!(!sync.should_sync(&large));
    }

    #[test]
    fn test_mark_received() {
        let mut sync = ClipboardSync::new(true);
        sync.mark_received("hello");
        assert!(!sync.should_sync("hello")); // 已标记，不再同步
    }
}
