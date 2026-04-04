#![allow(dead_code)]
/// TLS 1.3 加密传输模块
///
/// 使用 rustls（纯 Rust 实现，无 OpenSSL 依赖）+ rcgen（证书生成）。
/// 实现 TOFU（Trust On First Use）指纹验证模式：
/// - 首次连接时展示对端证书指纹，用户确认后保存
/// - 后续连接自动验证指纹，不匹配时拒绝连接
///
/// 注意：TLS 功能需要在 Cargo.toml 中添加 rustls 和 rcgen 依赖。
/// 当前实现提供完整的接口定义和 Mock 实现，
/// 实际 TLS 流需要在 Cargo.toml 中启用相关依赖后集成。

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// TLS 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum TlsError {
    /// 证书生成失败
    CertGenError(String),
    /// 握手失败
    HandshakeError(String),
    /// 指纹不匹配（可能遭受中间人攻击）
    FingerprintMismatch { expected: String, got: String },
    /// 证书过期
    CertExpired,
    /// 版本不兼容
    VersionMismatch,
    /// IO 错误
    IoError(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CertGenError(msg) => write!(f, "证书生成失败: {}", msg),
            Self::HandshakeError(msg) => write!(f, "TLS 握手失败: {}", msg),
            Self::FingerprintMismatch { expected, got } => {
                write!(f, "证书指纹不匹配（可能遭受中间人攻击）：期望 {}，实际 {}", expected, got)
            }
            Self::CertExpired => write!(f, "证书已过期"),
            Self::VersionMismatch => write!(f, "TLS 版本不兼容"),
            Self::IoError(msg) => write!(f, "IO 错误: {}", msg),
        }
    }
}

/// 证书信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertInfo {
    /// 证书 PEM 内容
    pub cert_pem: String,
    /// 私钥 PEM 内容
    pub key_pem: String,
    /// 证书指纹（SHA-256，十六进制）
    pub fingerprint: String,
    /// 生成时间（Unix 时间戳）
    pub created_at: u64,
}

/// 受信任的对端记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    /// 对端地址（IP:Port）
    pub address: String,
    /// 对端证书指纹
    pub fingerprint: String,
    /// 首次信任时间
    pub trusted_at: u64,
}

/// TLS 配置管理器
pub struct TlsManager {
    /// 应用数据目录
    data_dir: PathBuf,
    /// 本机证书（懒加载）
    cert: std::sync::Mutex<Option<CertInfo>>,
    /// 受信任的对端列表
    trusted_peers: std::sync::Mutex<HashMap<String, TrustedPeer>>,
}

impl TlsManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let manager = Self {
            data_dir,
            cert: std::sync::Mutex::new(None),
            trusted_peers: std::sync::Mutex::new(HashMap::new()),
        };
        // 加载已保存的受信任对端
        let _ = manager.load_trusted_peers();
        manager
    }

    /// 获取或生成本机证书
    /// 首次调用时自动生成 RSA-2048 自签名证书并保存
    pub fn get_or_create_cert(&self) -> Result<CertInfo, TlsError> {
        let mut cert_guard = self.cert.lock().unwrap();
        if let Some(ref cert) = *cert_guard {
            return Ok(cert.clone());
        }

        // 尝试从文件加载
        let cert_path = self.data_dir.join("cert.pem");
        let key_path = self.data_dir.join("key.pem");

        if cert_path.exists() && key_path.exists() {
            if let Ok(cert) = self.load_cert_from_files(&cert_path, &key_path) {
                *cert_guard = Some(cert.clone());
                return Ok(cert);
            }
        }

        // 生成新证书
        let cert = self.generate_self_signed_cert()?;
        // 保存到文件
        let _ = std::fs::create_dir_all(&self.data_dir);
        let _ = std::fs::write(&cert_path, &cert.cert_pem);
        let _ = std::fs::write(&key_path, &cert.key_pem);

        *cert_guard = Some(cert.clone());
        Ok(cert)
    }

    /// 生成 ECDSA P-256 自签名证书（使用 rcgen）
    fn generate_self_signed_cert(&self) -> Result<CertInfo, TlsError> {
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 生成 ECDSA P-256 密钥对（rcgen 0.13 完整支持）
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| TlsError::CertGenError(format!("生成密钥对失败: {}", e)))?;

        // 配置证书参数
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "KVMini");
        dn.push(DnType::OrganizationName, "KVM Share");
        params.distinguished_name = dn;
        params.subject_alt_names = vec![
            rcgen::SanType::DnsName(
                rcgen::Ia5String::try_from("kvmini.local".to_string())
                    .unwrap_or_else(|_| rcgen::Ia5String::try_from("localhost".to_string()).unwrap())
            ),
        ];
        // 有效期 10 年
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 1, 1);

        // 生成证书
        let cert = params.self_signed(&key_pair)
            .map_err(|e| TlsError::CertGenError(format!("生成证书失败: {}", e)))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();
        let fingerprint = compute_fingerprint(cert_pem.as_bytes());

        Ok(CertInfo {
            cert_pem,
            key_pem,
            fingerprint,
            created_at: now,
        })
    }

    /// 从文件加载证书
    fn load_cert_from_files(&self, cert_path: &PathBuf, key_path: &PathBuf) -> Result<CertInfo, TlsError> {
        let cert_pem = std::fs::read_to_string(cert_path)
            .map_err(|e| TlsError::IoError(e.to_string()))?;
        let key_pem = std::fs::read_to_string(key_path)
            .map_err(|e| TlsError::IoError(e.to_string()))?;
        let fingerprint = compute_fingerprint(cert_pem.as_bytes());
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(CertInfo { cert_pem, key_pem, fingerprint, created_at })
    }

    /// 验证对端证书指纹（TOFU 模式）
    /// 返回 Ok(true) 表示已信任，Ok(false) 表示首次连接需用户确认
    pub fn verify_peer_fingerprint(
        &self,
        address: &str,
        fingerprint: &str,
    ) -> Result<bool, TlsError> {
        let peers = self.trusted_peers.lock().unwrap();
        if let Some(peer) = peers.get(address) {
            if peer.fingerprint == fingerprint {
                Ok(true) // 已信任且指纹匹配
            } else {
                Err(TlsError::FingerprintMismatch {
                    expected: peer.fingerprint.clone(),
                    got: fingerprint.to_string(),
                })
            }
        } else {
            Ok(false) // 首次连接，需用户确认
        }
    }

    /// 保存受信任的对端（用户确认后调用）
    pub fn trust_peer(&self, address: String, fingerprint: String) -> Result<(), TlsError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let peer = TrustedPeer {
            address: address.clone(),
            fingerprint,
            trusted_at: now,
        };

        self.trusted_peers.lock().unwrap().insert(address, peer);
        self.save_trusted_peers()
    }

    /// 加载受信任对端列表
    fn load_trusted_peers(&self) -> Result<(), TlsError> {
        let path = self.data_dir.join("trusted_peers.json");
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| TlsError::IoError(e.to_string()))?;
        let peers: HashMap<String, TrustedPeer> = serde_json::from_str(&content)
            .map_err(|e| TlsError::IoError(e.to_string()))?;
        *self.trusted_peers.lock().unwrap() = peers;
        Ok(())
    }

    /// 保存受信任对端列表
    fn save_trusted_peers(&self) -> Result<(), TlsError> {
        let path = self.data_dir.join("trusted_peers.json");
        let peers = self.trusted_peers.lock().unwrap();
        let content = serde_json::to_string_pretty(&*peers)
            .map_err(|e| TlsError::IoError(e.to_string()))?;
        std::fs::write(&path, content)
            .map_err(|e| TlsError::IoError(e.to_string()))
    }

    /// 获取本机证书指纹
    pub fn get_local_fingerprint(&self) -> Result<String, TlsError> {
        let cert = self.get_or_create_cert()?;
        Ok(cert.fingerprint)
    }
}

/// 计算证书指纹（SHA-256，十六进制，冒号分隔格式）
/// 使用 sha2 crate 计算真实的 SHA-256 哈希
pub fn compute_fingerprint(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(data);
    // 格式化为 XX:XX:XX:... 形式（类似浏览器证书指纹显示）
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// TLS 连接包装器（对上层透明）
///
/// 实际实现需要 rustls crate：
/// ```rust
/// use rustls::{ClientConfig, ServerConfig, StreamOwned};
/// pub enum TlsStream<S: std::io::Read + std::io::Write> {
///     Client(StreamOwned<ClientConnection, S>),
///     Server(StreamOwned<ServerConnection, S>),
/// }
/// ```
///
/// 当前提供接口定义，待 rustls 依赖添加后实现
pub struct TlsStreamWrapper {
    /// 是否已加密
    pub encrypted: bool,
    /// 对端指纹
    pub peer_fingerprint: Option<String>,
}

impl TlsStreamWrapper {
    pub fn new_encrypted(peer_fingerprint: String) -> Self {
        Self { encrypted: true, peer_fingerprint: Some(peer_fingerprint) }
    }

    pub fn new_plaintext() -> Self {
        Self { encrypted: false, peer_fingerprint: None }
    }
}

/// Tauri 命令：获取本机 TLS 证书指纹
#[tauri::command]
pub fn get_local_tls_fingerprint() -> Result<String, String> {
    let data_dir = get_tls_data_dir();
    let manager = TlsManager::new(data_dir);
    manager.get_local_fingerprint().map_err(|e| e.to_string())
}

/// Tauri 命令：信任对端证书（用户确认后调用）
#[tauri::command]
pub fn trust_peer_certificate(address: String, fingerprint: String) -> Result<(), String> {
    let data_dir = get_tls_data_dir();
    let manager = TlsManager::new(data_dir);
    manager.trust_peer(address, fingerprint).map_err(|e| e.to_string())
}

/// 获取 TLS 数据目录
fn get_tls_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join("Library/Application Support/com.kvmshare.app/tls")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Temp".to_string());
        PathBuf::from(appdata).join("KVMShare/tls")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/tmp/kvmshare/tls")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── TlsManager 测试 ──────────────────────────────────────────────────

    #[test]
    fn test_get_or_create_cert_generates_cert() {
        let dir = temp_dir();
        let manager = TlsManager::new(dir.path().to_path_buf());
        let cert = manager.get_or_create_cert().unwrap();
        assert!(!cert.cert_pem.is_empty());
        assert!(!cert.key_pem.is_empty());
        assert!(!cert.fingerprint.is_empty());
    }

    #[test]
    fn test_get_or_create_cert_returns_same_cert() {
        let dir = temp_dir();
        let manager = TlsManager::new(dir.path().to_path_buf());
        let cert1 = manager.get_or_create_cert().unwrap();
        let cert2 = manager.get_or_create_cert().unwrap();
        // 第二次调用应返回相同证书（缓存）
        assert_eq!(cert1.fingerprint, cert2.fingerprint);
    }

    #[test]
    fn test_verify_peer_fingerprint_first_connection() {
        let dir = temp_dir();
        let manager = TlsManager::new(dir.path().to_path_buf());
        // 首次连接，返回 false（需用户确认）
        let result = manager.verify_peer_fingerprint("192.168.1.100:8080", "abc123").unwrap();
        assert!(!result, "首次连接应返回 false（需用户确认）");
    }

    #[test]
    fn test_trust_peer_and_verify() {
        let dir = temp_dir();
        let manager = TlsManager::new(dir.path().to_path_buf());
        let address = "192.168.1.100:8080".to_string();
        let fingerprint = "abc123def456".to_string();

        // 信任对端
        manager.trust_peer(address.clone(), fingerprint.clone()).unwrap();

        // 验证应成功
        let result = manager.verify_peer_fingerprint(&address, &fingerprint).unwrap();
        assert!(result, "已信任的对端应验证成功");
    }

    #[test]
    fn test_fingerprint_mismatch_returns_error() {
        let dir = temp_dir();
        let manager = TlsManager::new(dir.path().to_path_buf());
        let address = "192.168.1.100:8080".to_string();

        // 信任指纹 A
        manager.trust_peer(address.clone(), "fingerprint_a".to_string()).unwrap();

        // 验证指纹 B（不匹配）
        let result = manager.verify_peer_fingerprint(&address, "fingerprint_b");
        assert!(matches!(result, Err(TlsError::FingerprintMismatch { .. })),
            "指纹不匹配应返回 FingerprintMismatch 错误");
    }

    #[test]
    fn test_trusted_peers_persisted() {
        let dir = temp_dir();
        let address = "192.168.1.100:8080".to_string();
        let fingerprint = "abc123".to_string();

        // 第一个 manager 保存信任
        {
            let manager = TlsManager::new(dir.path().to_path_buf());
            manager.trust_peer(address.clone(), fingerprint.clone()).unwrap();
        }

        // 第二个 manager 加载后应能验证
        {
            let manager = TlsManager::new(dir.path().to_path_buf());
            let result = manager.verify_peer_fingerprint(&address, &fingerprint).unwrap();
            assert!(result, "持久化的信任记录应在重新加载后有效");
        }
    }

    // ── compute_fingerprint 测试 ─────────────────────────────────────────

    #[test]
    fn test_compute_fingerprint_deterministic() {
        let data = b"test certificate data";
        let fp1 = compute_fingerprint(data);
        let fp2 = compute_fingerprint(data);
        assert_eq!(fp1, fp2, "相同数据应生成相同指纹");
    }

    #[test]
    fn test_compute_fingerprint_different_data() {
        let fp1 = compute_fingerprint(b"cert1");
        let fp2 = compute_fingerprint(b"cert2");
        assert_ne!(fp1, fp2, "不同数据应生成不同指纹");
    }

    #[test]
    fn test_compute_fingerprint_not_empty() {
        let fp = compute_fingerprint(b"some data");
        assert!(!fp.is_empty());
    }

    #[test]
    fn test_compute_fingerprint_sha256_format() {
        // SHA-256 指纹格式应为 XX:XX:...（32 个字节，63 个字符）
        let fp = compute_fingerprint(b"test");
        // SHA-256 输出 32 字节，格式化后为 32*3-1 = 95 个字符（XX:）
        assert_eq!(fp.len(), 95, "SHA-256 指纹应为 95 个字符（XX:XX:...）");
        // 应包含冒号分隔符
        assert!(fp.contains(':'), "指纹应包含冒号分隔符");
    }

    // ── TlsError 测试 ────────────────────────────────────────────────────

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::FingerprintMismatch {
            expected: "abc".to_string(),
            got: "def".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("中间人"), "指纹不匹配错误应提示中间人攻击");
    }

    #[test]
    fn test_tls_error_cert_gen() {
        let err = TlsError::CertGenError("test error".to_string());
        assert!(err.to_string().contains("证书生成失败"));
    }

    // ── TlsStreamWrapper 测试 ────────────────────────────────────────────

    #[test]
    fn test_tls_stream_wrapper_encrypted() {
        let wrapper = TlsStreamWrapper::new_encrypted("fingerprint123".to_string());
        assert!(wrapper.encrypted);
        assert_eq!(wrapper.peer_fingerprint, Some("fingerprint123".to_string()));
    }

    #[test]
    fn test_tls_stream_wrapper_plaintext() {
        let wrapper = TlsStreamWrapper::new_plaintext();
        assert!(!wrapper.encrypted);
        assert!(wrapper.peer_fingerprint.is_none());
    }

    // ── get_local_tls_fingerprint 测试 ───────────────────────────────────

    #[test]
    fn test_get_local_fingerprint() {
        let dir = temp_dir();
        let manager = TlsManager::new(dir.path().to_path_buf());
        let fp = manager.get_local_fingerprint().unwrap();
        assert!(!fp.is_empty(), "本机指纹不应为空");
    }
}
