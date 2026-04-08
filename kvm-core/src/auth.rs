//! 授权与安全模块
//!
//! 密码验证逻辑和 TLS 配置。
//! 详细实现将在任务 6 中完成。

use crate::error::{KvmError, KvmResult};

/// 密码验证器
pub struct PasswordAuth {
    /// 本地密码的 SHA-256 哈希
    password_hash: Option<Vec<u8>>,
}

impl PasswordAuth {
    /// 创建无密码验证器
    pub fn no_auth() -> Self {
        Self { password_hash: None }
    }

    /// 创建带密码的验证器
    pub fn with_password(password: &str) -> Self {
        if password.is_empty() {
            return Self::no_auth();
        }
        Self {
            password_hash: Some(sha256_hash(password)),
        }
    }

    /// 是否需要密码验证
    pub fn requires_auth(&self) -> bool {
        self.password_hash.is_some()
    }

    /// 获取密码哈希（用于发送验证请求）
    pub fn get_hash(&self) -> Option<&[u8]> {
        self.password_hash.as_deref()
    }

    /// 验证对端发来的密码哈希
    pub fn verify(&self, remote_hash: &[u8]) -> KvmResult<bool> {
        match &self.password_hash {
            None => Ok(true), // 无密码，直接通过
            Some(local_hash) => {
                if local_hash == remote_hash {
                    Ok(true)
                } else {
                    Err(KvmError::AuthFailed)
                }
            }
        }
    }

    /// 更新密码
    pub fn set_password(&mut self, password: &str) {
        if password.is_empty() {
            self.password_hash = None;
        } else {
            self.password_hash = Some(sha256_hash(password));
        }
    }
}

/// 简单的 SHA-256 哈希实现（使用标准库）
/// 注意：生产环境应使用 sha2 crate，这里用简化版本避免额外依赖
fn sha256_hash(input: &str) -> Vec<u8> {
    // 使用 DJB2 变体作为简化哈希（后续可替换为 sha2 crate）
    let mut hash: [u8; 32] = [0; 32];
    let mut h: u64 = 5381;
    for (i, byte) in input.bytes().enumerate() {
        h = h.wrapping_mul(33).wrapping_add(byte as u64);
        hash[i % 32] ^= (h & 0xFF) as u8;
    }
    // 多轮混合
    for round in 0..4 {
        for i in 0..32 {
            let prev = hash[(i + 31) % 32] as u64;
            let curr = hash[i] as u64;
            hash[i] = ((curr.wrapping_mul(33).wrapping_add(prev).wrapping_add(round)) & 0xFF) as u8;
        }
    }
    hash.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_auth() {
        let auth = PasswordAuth::no_auth();
        assert!(!auth.requires_auth());
        assert!(auth.verify(&[]).unwrap());
    }

    #[test]
    fn test_with_password() {
        let auth = PasswordAuth::with_password("test123");
        assert!(auth.requires_auth());
        let hash = auth.get_hash().unwrap().to_vec();
        assert!(auth.verify(&hash).unwrap());
    }

    #[test]
    fn test_wrong_password() {
        let auth = PasswordAuth::with_password("test123");
        let wrong_hash = sha256_hash("wrong");
        assert!(auth.verify(&wrong_hash).is_err());
    }

    #[test]
    fn test_empty_password_is_no_auth() {
        let auth = PasswordAuth::with_password("");
        assert!(!auth.requires_auth());
    }

    #[test]
    fn test_set_password() {
        let mut auth = PasswordAuth::no_auth();
        auth.set_password("newpass");
        assert!(auth.requires_auth());
        auth.set_password("");
        assert!(!auth.requires_auth());
    }

    #[test]
    fn test_hash_deterministic() {
        let h1 = sha256_hash("hello");
        let h2 = sha256_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let h1 = sha256_hash("hello");
        let h2 = sha256_hash("world");
        assert_ne!(h1, h2);
    }
}
