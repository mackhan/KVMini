//! TLS 证书模块 — 自签名证书生成、PEM I/O、SHA-256 指纹
//!
//! v0.1.38 组 C-1 产物。不依赖任何系统 API，纯逻辑 + rcgen/sha2。
//!
//! ## 用法
//!
//! ```ignore
//! use kvm_core::tls::SelfSignedCert;
//! let cert = SelfSignedCert::generate("kvmini").unwrap();
//! let fp   = cert.fingerprint_sha256_hex();          // 64 字符小写 hex
//! let pem  = cert.to_pem();                          // PEM 字符串对
//! let restored = SelfSignedCert::from_pem(&pem.cert_pem, &pem.key_pem).unwrap();
//! assert_eq!(cert.fingerprint_sha256_hex(), restored.fingerprint_sha256_hex());
//! ```

use crate::error::{KvmError, KvmResult};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use sha2::{Digest, Sha256};

/// 自签名 TLS 证书 + 私钥（DER 字节）
#[derive(Clone)]
pub struct SelfSignedCert {
    /// X.509 证书（DER 编码）
    pub cert_der: Vec<u8>,
    /// 私钥（PKCS#8 DER 编码）
    pub key_der: Vec<u8>,
}

/// PEM 形式的证书 + 私钥（便于持久化到 Keychain 等文本存储）
#[derive(Debug, Clone)]
pub struct CertPem {
    pub cert_pem: String,
    pub key_pem: String,
}

impl SelfSignedCert {
    /// 生成新的自签名证书
    ///
    /// - `common_name`：DN 的 CN 字段，也作为 SAN 的 DNS 名
    pub fn generate(common_name: &str) -> KvmResult<Self> {
        let mut params = CertificateParams::new(vec![common_name.to_string()])
            .map_err(|e| KvmError::Unknown { message: format!("cert params: {e}") })?;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, common_name);
        params.distinguished_name = dn;

        let key = KeyPair::generate()
            .map_err(|e| KvmError::Unknown { message: format!("keygen: {e}") })?;

        let cert = params.self_signed(&key)
            .map_err(|e| KvmError::Unknown { message: format!("sign: {e}") })?;

        Ok(Self {
            cert_der: cert.der().to_vec(),
            key_der: key.serialize_der(),
        })
    }

    /// 计算证书（DER）的 SHA-256 指纹，返回 64 字符小写十六进制
    pub fn fingerprint_sha256_hex(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.cert_der);
        hex::encode(hasher.finalize())
    }

    /// 计算证书（DER）的 SHA-256 指纹，返回 `AA:BB:CC:...` 冒号分隔形式（显示用）
    pub fn fingerprint_sha256_colon(&self) -> String {
        let hex_str = self.fingerprint_sha256_hex();
        hex_str
            .as_bytes()
            .chunks(2)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect::<Vec<_>>()
            .join(":")
            .to_uppercase()
    }

    /// 导出为 PEM 格式（"-----BEGIN CERTIFICATE-----" 开头）
    pub fn to_pem(&self) -> CertPem {
        CertPem {
            cert_pem: der_to_pem("CERTIFICATE", &self.cert_der),
            key_pem: der_to_pem("PRIVATE KEY", &self.key_der),
        }
    }

    /// 从 PEM 文本对还原
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> KvmResult<Self> {
        let cert_der = pem_to_der(cert_pem, "CERTIFICATE")?;
        let key_der = pem_to_der(key_pem, "PRIVATE KEY")?;
        Ok(Self { cert_der, key_der })
    }
}

// ─── PEM 编解码（本地最小实现，避免再引一个 crate） ────────────────────────

fn der_to_pem(label: &str, der: &[u8]) -> String {
    let b64 = base64_encode(der);
    let mut out = String::new();
    out.push_str(&format!("-----BEGIN {}-----\n", label));
    for line in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).unwrap_or(""));
        out.push('\n');
    }
    out.push_str(&format!("-----END {}-----\n", label));
    out
}

fn pem_to_der(pem: &str, label: &str) -> KvmResult<Vec<u8>> {
    let begin = format!("-----BEGIN {}-----", label);
    let end = format!("-----END {}-----", label);
    let start = pem.find(&begin).ok_or_else(|| KvmError::Unknown {
        message: format!("PEM 缺少 BEGIN {}", label),
    })?;
    let after_begin = start + begin.len();
    let end_pos = pem[after_begin..].find(&end).ok_or_else(|| KvmError::Unknown {
        message: format!("PEM 缺少 END {}", label),
    })?;
    let body: String = pem[after_begin..after_begin + end_pos]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64_decode(&body).map_err(|e| KvmError::Unknown {
        message: format!("PEM base64 解码失败：{e}"),
    })
}

// ─── 极简 base64（标准 alphabet，无 padding 校验） ──────────────────────────
// 仅用于 PEM I/O；不引入 base64 crate 以减少依赖。

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(B64_ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(B64_ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(B64_ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut lookup = [255u8; 256];
    for (i, &b) in B64_ALPHABET.iter().enumerate() {
        lookup[b as usize] = i as u8;
    }
    let bytes: Vec<u8> = input.bytes().filter(|b| *b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for b in bytes {
        let v = lookup[b as usize];
        if v == 255 {
            return Err(format!("非法 base64 字符: {}", b as char));
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

// ─── 测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed_cert_has_fingerprint() {
        let cert = SelfSignedCert::generate("kvmini-test").unwrap();
        let fp = cert.fingerprint_sha256_hex();
        assert_eq!(fp.len(), 64, "SHA-256 十六进制应 64 字符");
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_fingerprint_is_deterministic_for_same_cert() {
        let cert = SelfSignedCert::generate("kvmini-test").unwrap();
        let fp1 = cert.fingerprint_sha256_hex();
        let fp2 = cert.fingerprint_sha256_hex();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_different_certs_have_different_fingerprints() {
        let c1 = SelfSignedCert::generate("a").unwrap();
        let c2 = SelfSignedCert::generate("b").unwrap();
        assert_ne!(c1.fingerprint_sha256_hex(), c2.fingerprint_sha256_hex());
    }

    #[test]
    fn test_cert_pem_roundtrip() {
        let c1 = SelfSignedCert::generate("kvmini-roundtrip").unwrap();
        let pem = c1.to_pem();
        assert!(pem.cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(pem.key_pem.contains("-----BEGIN PRIVATE KEY-----"));
        let c2 = SelfSignedCert::from_pem(&pem.cert_pem, &pem.key_pem).unwrap();
        assert_eq!(c1.fingerprint_sha256_hex(), c2.fingerprint_sha256_hex());
        assert_eq!(c1.cert_der, c2.cert_der);
        assert_eq!(c1.key_der, c2.key_der);
    }

    #[test]
    fn test_fingerprint_colon_format() {
        let cert = SelfSignedCert::generate("k").unwrap();
        let colon = cert.fingerprint_sha256_colon();
        // 32 组两位 hex 用 ':' 分隔 → 长度 = 32 * 2 + 31 = 95
        assert_eq!(colon.len(), 95);
        assert_eq!(colon.matches(':').count(), 31);
        // 大写
        assert!(colon.chars().all(|c| c.is_ascii_hexdigit() || c == ':'));
        assert!(colon.chars().any(|c| c.is_ascii_uppercase()) || colon.chars().all(|c| !c.is_ascii_alphabetic()));
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"Hello, KVMini TLS!";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64_empty_and_one_byte() {
        assert_eq!(base64_encode(&[]), "");
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
        let one = b"A";
        let enc = base64_encode(one);
        assert_eq!(base64_decode(&enc).unwrap(), one);
    }
}
