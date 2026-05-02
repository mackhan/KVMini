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

/// 真 SHA-256 哈希（使用 `sha2` crate）
///
/// v0.1.38 组 C-4 起改为真 SHA-256；此前是 DJB2 变体的"伪哈希"，
/// 导致与 TLS 指纹模型对不上号，且理论上存在碰撞风险。
fn sha256_hash(input: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
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

    #[test]
    fn test_hash_matches_real_sha256_vector() {
        // 真 SHA-256("hello") 的已知向量
        let h = sha256_hash("hello");
        assert_eq!(
            hex::encode(&h),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_hash_output_is_32_bytes() {
        assert_eq!(sha256_hash("").len(), 32);
        assert_eq!(sha256_hash("a").len(), 32);
        assert_eq!(sha256_hash("long input string ....").len(), 32);
    }
}
