#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

// updater
use tauri_plugin_updater::UpdaterExt;

// ── 模块声明 ──────────────────────────────────────────────────────────────
#[macro_use]
mod macros;
mod config;
mod connection;
mod network_scan;
mod platform;
mod sharing;

// ── 通用 Tauri 命令 ────────────────────────────────────────────────────────

/// 获取应用版本号（从 Cargo.toml 读取，两端保持一致）
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── 持久化日志初始化 ──────────────────────────────────────────────────────────

/// 同时写 stderr 和持久化日志文件的 Writer
///
/// 日志文件路径：
/// - macOS:   ~/Library/Logs/KVMini/runtime.log
/// - Windows: %LOCALAPPDATA%/KVMini/Logs/runtime.log
/// - Linux:   ~/.local/share/KVMini/logs/runtime.log
///
/// Rotate 策略：启动时若已有 runtime.log 且大小 > 2MB，重命名为 runtime.log.1，
///              仅保留最近 3 个历史文件。
struct DualWriter {
    file: Option<std::sync::Mutex<std::fs::File>>,
}

impl std::io::Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // 先写 stderr
        let _ = std::io::stderr().write_all(buf);
        // 再写文件（失败不影响 stderr）
        if let Some(ref m) = self.file {
            if let Ok(mut f) = m.lock() {
                let _ = f.write_all(buf);
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        if let Some(ref m) = self.file {
            if let Ok(mut f) = m.lock() {
                let _ = f.flush();
            }
        }
        Ok(())
    }
}

/// 返回日志目录路径（已确保目录存在）
fn resolve_log_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").ok()?;
        let dir = std::path::PathBuf::from(home).join("Library").join("Logs").join("KVMini");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }
    #[cfg(target_os = "windows")]
    {
        let local = std::env::var("LOCALAPPDATA").ok()?;
        let dir = std::path::PathBuf::from(local).join("KVMini").join("Logs");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let home = std::env::var("HOME").ok()?;
        let dir = std::path::PathBuf::from(home).join(".local").join("share").join("KVMini").join("logs");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }
}

/// 启动时做日志 rotate：若 runtime.log > 2MB，依次 rotate 到 .1/.2/.3，删除最老的
fn rotate_log_if_needed(log_path: &std::path::Path) {
    const MAX_SIZE: u64 = 2 * 1024 * 1024;
    const KEEP: usize = 3;
    let size = std::fs::metadata(log_path).ok().map(|m| m.len()).unwrap_or(0);
    if size < MAX_SIZE {
        return;
    }
    // 从最老的开始删除/重命名
    for i in (1..=KEEP).rev() {
        let cur = log_path.with_extension(format!("log.{}", i));
        if i == KEEP {
            let _ = std::fs::remove_file(&cur);
        } else {
            let next = log_path.with_extension(format!("log.{}", i + 1));
            let _ = std::fs::rename(&cur, &next);
        }
    }
    let _ = std::fs::rename(log_path, log_path.with_extension("log.1"));
}

fn init_persistent_logger() {
    use std::io::Write;

    let log_dir = resolve_log_dir();

    // v0.1.54: 调试模式——如果日志目录下存在 debug_mode 文件，提升日志级别到 debug
    // 用法：在日志目录下 touch debug_mode，重启应用即可看到 debug 日志；
    //       rm debug_mode 后重启恢复 info 级别
    let debug_mode = log_dir.as_ref()
        .map(|d| d.join("debug_mode").exists())
        .unwrap_or(false);
    let default_level = if debug_mode { "debug" } else { "info" };

    let file_handle = log_dir.as_ref().and_then(|dir| {
        let log_path = dir.join("runtime.log");
        rotate_log_if_needed(&log_path);
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok()
            .map(|f| (log_path, f))
    });

    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(default_level)
    );

    if let Some((log_path, file)) = file_handle {
        let dual = DualWriter {
            file: Some(std::sync::Mutex::new(file)),
        };
        builder
            .target(env_logger::Target::Pipe(Box::new(dual)))
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} [{}] {}: {}",
                    chrono_style_timestamp(),
                    record.level(),
                    record.target(),
                    record.args()
                )
            });
        builder.init();
        // v0.1.54: 启动头部打印完整诊断信息，方便查日志时快速定位版本/平台
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("KVMini v{} 启动", env!("CARGO_PKG_VERSION"));
        log::info!("  平台: {}", std::env::consts::OS);
        log::info!("  日志: {} (级别={}{})", log_path.display(), default_level,
            if debug_mode { ", 调试模式已开启" } else { "" });
        log::info!("  调试模式: {} (touch {}/debug_mode 可开启)", debug_mode,
            log_path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default());
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        // 日志目录创建失败时退化为只写 stderr（保持原行为）
        builder.init();
        log::warn!("[Logger] 无法创建持久化日志文件，仅写 stderr");
    }
}

/// 生成简易时间戳（避免引入 chrono 依赖）
/// 格式：YYYY-MM-DDTHH:MM:SS
fn chrono_style_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    // 简化：只记录 UTC 秒数对应的 YMD/HMS，足够排查用
    // 计算天数
    let days = secs / 86400;
    let sod = secs % 86400;
    let hh = sod / 3600;
    let mm = (sod % 3600) / 60;
    let ss = sod % 60;
    // 从 1970-01-01 开始算 y/m/d
    let (year, month, day) = epoch_days_to_ymd(days);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hh, mm, ss)
}

fn epoch_days_to_ymd(mut days: i64) -> (i64, u32, u32) {
    // 以 1970-01-01 为起点往后累加
    let mut year: i64 = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let year_days = if leap { 366 } else { 365 };
        if days < year_days { break; }
        days -= year_days;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let months: [i64; 12] = [31, if leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for (i, &dim) in months.iter().enumerate() {
        if days < dim { month = (i + 1) as u32; break; }
        days -= dim;
    }
    let day = (days + 1) as u32;
    (year, month, day)
}


/// 获取本机所有局域网 IP 列表
///
/// 排除法：排除 回环 / IPv6 link-local / APIPA(169.254.x) / 0.0.0.0，
/// 保留其他所有 IPv4 地址（兼容企业内网如 30.x.x.x 等非 RFC1918 网段）。
/// 返回结果按网卡类型排序：Wi-Fi/以太网优先，VPN/虚拟网卡靠后。
#[tauri::command]
fn get_local_ip() -> Vec<String> {
    let mut entries: Vec<(String, String)> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .filter(|iface| iface.ip().is_ipv4()) // 只保留 IPv4
        .filter(|iface| {
            let ip = iface.ip().to_string();
            // 排除 APIPA (169.254.x.x) 和 0.0.0.0
            !ip.starts_with("169.254.") && ip != "0.0.0.0"
        })
        .map(|iface| (iface.name.clone(), iface.ip().to_string()))
        .collect();

    // 按网卡优先级排序：Wi-Fi/以太网 > 其他 > VPN/虚拟网卡
    entries.sort_by_key(|(name, _)| {
        let n = name.to_lowercase();
        if n.contains("wlan") || n.contains("wifi") || n.contains("wi-fi")
            || n.starts_with("en") && !n.contains("utun") {
            0 // Wi-Fi / 以太网
        } else if n.contains("eth") {
            1 // 有线网卡
        } else if n.contains("vpn") || n.contains("utun") || n.contains("tun") || n.contains("tap") {
            9 // VPN / 虚拟隧道
        } else {
            5 // 其他
        }
    });

    // 详细日志：每个网卡名称 + IP + 排序优先级
    for (name, ip) in &entries {
        let n = name.to_lowercase();
        let priority = if n.contains("wlan") || n.contains("wifi") || n.contains("wi-fi")
            || n.starts_with("en") && !n.contains("utun") {
            0
        } else if n.contains("eth") {
            1
        } else if n.contains("vpn") || n.contains("utun") || n.contains("tun") || n.contains("tap") {
            9
        } else {
            5
        };
        log::info!("[Network] 网卡: {} -> IP: {} (优先级: {})", name, ip, priority);
    }

    let ips: Vec<String> = entries.into_iter().map(|(_, ip)| ip).collect();
    log::info!("[Network] get_local_ip 返回 {} 个地址: {:?}", ips.len(), ips);
    ips
}

/// 网卡信息结构体
#[derive(serde::Serialize, Clone)]
struct NetworkInterface {
    name: String,
    ip: String,
    is_up: bool,
    /// 网卡类型推断（wifi / ethernet / vpn / other）
    iface_type: String,
}

/// 获取本机所有活跃网卡（名称 + IP + 状态 + 类型）
///
/// 排除法：排除 回环 / IPv6 / APIPA(169.254.x) / 0.0.0.0，
/// 保留其他所有 IPv4 地址。
#[tauri::command]
fn get_network_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces: Vec<NetworkInterface> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .filter(|iface| iface.ip().is_ipv4())
        .filter_map(|iface| {
            let ip = iface.ip().to_string();
            // 排除 APIPA 和 0.0.0.0
            if ip.starts_with("169.254.") || ip == "0.0.0.0" {
                return None;
            }
            let n = iface.name.to_lowercase();
            let iface_type = if n.contains("wlan") || n.contains("wifi") || n.contains("wi-fi")
                || (n.starts_with("en") && !n.contains("utun"))
            {
                "wifi"
            } else if n.contains("eth") {
                "ethernet"
            } else if n.contains("vpn") || n.contains("utun") || n.contains("tun") || n.contains("tap") {
                "vpn"
            } else {
                "other"
            };
            Some(NetworkInterface {
                name: iface.name.clone(),
                ip,
                is_up: true, // if_addrs 只返回活跃接口
                iface_type: iface_type.to_string(),
            })
        })
        .collect();

    // 按网卡优先级排序
    interfaces.sort_by_key(|iface| match iface.iface_type.as_str() {
        "wifi" => 0,
        "ethernet" => 1,
        "other" => 5,
        "vpn" => 9,
        _ => 5,
    });

    log::info!("[Network] get_network_interfaces 返回 {} 个网卡", interfaces.len());
    interfaces
}

// ── TLS 证书相关 ──────────────────────────────────────────────────────────────

/// 获取 TLS 证书存储路径（跨平台）
fn get_cert_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("KVMini")
        .join("certs")
}

/// 加载或生成本机 TLS 证书，返回 SHA-256 指纹（AA:BB:CC... 形式）
///
/// v0.1.38 组 C-5：统一走 `kvm_core::tls::SelfSignedCert`，不再在前端层重复实现 rcgen/sha2。
#[tauri::command]
fn get_local_tls_fingerprint() -> Result<String, String> {
    let cert = load_or_generate_local_cert()?;
    Ok(cert.fingerprint_sha256_colon())
}

/// 重新生成 TLS 证书（删除旧证书，生成新证书），返回新指纹
#[tauri::command]
fn regenerate_tls_cert() -> Result<String, String> {
    let cert_dir = get_cert_dir();
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");
    if cert_path.exists() { let _ = std::fs::remove_file(&cert_path); }
    if key_path.exists() { let _ = std::fs::remove_file(&key_path); }
    log::info!("[TLS] 重新生成证书");
    let cert = load_or_generate_local_cert()?;
    Ok(cert.fingerprint_sha256_colon())
}

/// 加载或生成 `kvm_core::tls::SelfSignedCert`，并持久化到磁盘
fn load_or_generate_local_cert() -> Result<kvm_core::tls::SelfSignedCert, String> {
    use kvm_core::tls::SelfSignedCert;

    let cert_dir = get_cert_dir();
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");

    // 尝试加载已有证书
    if cert_path.exists() && key_path.exists() {
        match (std::fs::read(&cert_path), std::fs::read(&key_path)) {
            (Ok(cert_der), Ok(key_der)) => {
                log::info!("[TLS] 加载已有证书: {:?}", cert_path);
                return Ok(SelfSignedCert { cert_der, key_der });
            }
            (cert_res, key_res) => {
                log::warn!(
                    "[TLS] 证书文件读取失败（cert={:?} key={:?}），将重新生成",
                    cert_res.as_ref().err(), key_res.as_ref().err()
                );
                let _ = std::fs::remove_file(&cert_path);
                let _ = std::fs::remove_file(&key_path);
            }
        }
    }

    log::info!("[TLS] 生成新的自签名证书");
    std::fs::create_dir_all(&cert_dir)
        .map_err(|e| format!("创建证书目录失败: {}", e))?;
    let cert = SelfSignedCert::generate("KVMini")
        .map_err(|e| format!("生成证书失败: {}", e))?;
    std::fs::write(&cert_path, &cert.cert_der)
        .map_err(|e| format!("保存证书失败: {}", e))?;
    std::fs::write(&key_path, &cert.key_der)
        .map_err(|e| format!("保存密钥失败: {}", e))?;
    log::info!("[TLS] 新证书已生成并保存: {:?}", cert_path);
    Ok(cert)
}

/// 扫描局域网内的 KVMini 设备
///
/// 策略：多接口 UDP 广播优先（按物理网卡 bind+定向广播，规避 VPN 干扰），
/// 无结果时降级为多接口子网 TCP 扫描（每张物理网卡的 /24 并发探测）。
#[tauri::command]
async fn discover_devices() -> Result<Vec<kvm_core::network::DiscoveredDevice>, String> {
    log::info!(
        "[Discover] 开始多接口局域网设备扫描（UDP 广播 :{}）...",
        kvm_core::network::DEFAULT_UDP_PORT
    );

    // 第一阶段：多接口 UDP 定向广播（v2，规避 VPN）
    let udp_devices =
        network_scan::discover_devices_multi(kvm_core::network::DEFAULT_UDP_PORT).await;
    if !udp_devices.is_empty() {
        log::info!("[Discover] 多接口 UDP 广播发现 {} 个设备", udp_devices.len());
        return Ok(udp_devices);
    }
    log::warn!("[Discover] 多接口 UDP 广播无结果，回退到 kvm-core 单 socket 广播...");

    // 第二阶段：兼容路径，跑一次老的 0.0.0.0 广播（极端情况下 VPN 没干扰，老路径仍可工作）
    if let Ok(devices) =
        kvm_core::network::discover_devices(kvm_core::network::DEFAULT_UDP_PORT).await
    {
        if !devices.is_empty() {
            log::info!("[Discover] 单 socket 广播兜底发现 {} 个设备", devices.len());
            return Ok(devices);
        }
    }
    log::warn!("[Discover] UDP 广播全部失败，降级到多接口子网 TCP 扫描");

    // 第三阶段：多接口子网 TCP 扫描（降级）
    let scan_devices =
        network_scan::scan_subnet_multi(kvm_core::network::DEFAULT_TCP_PORT).await;
    if scan_devices.is_empty() {
        log::warn!("[Discover] 多接口子网扫描也未发现设备，请检查对端是否运行 KVMini / 防火墙 / VPN");
    } else {
        log::info!("[Discover] 多接口子网扫描发现 {} 个设备", scan_devices.len());
    }
    Ok(scan_devices)
}

/// 获取本机局域网 IP（优先使用 network_scan 过滤掉 VPN 的物理网卡，兜底走 8.8.8.8 connect）
#[allow(dead_code)] // 保留作为后续 NIC/监听 IP 选择的工具函数
fn get_local_lan_ip() -> Option<String> {
    // 优先：枚举物理网卡（已排除 utun*/ppp*/tun*/tap*/loopback/APIPA），取第一张
    let ifaces = network_scan::list_lan_ipv4_ifaces();
    if let Some(first) = ifaces.first() {
        let ip = first.ipv4.to_string();
        log::info!("[Discover] 使用物理网卡 {} 的 IP: {}", first.name, ip);
        return Some(ip);
    }

    // 兜底：通过 connect 拿出口 IP（极端情况下 if_addrs 失败时使用，但可能命中 VPN）
    use std::net::UdpSocket as StdUdpSocket;
    if let Ok(socket) = StdUdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip = addr.ip().to_string();
                if ip != "0.0.0.0" && !ip.starts_with("127.") {
                    log::warn!("[Discover] 物理网卡枚举失败，回退使用出口 IP: {}", ip);
                    return Some(ip);
                }
            }
        }
    }
    None
}

// ── 网络诊断 ─────────────────────────────────────────────────────────────

/// 单条诊断结果
#[derive(serde::Serialize, Clone)]
struct DiagnosticItem {
    /// 检查项名称（如 "udp_port", "vpn", "firewall"）
    check: String,
    /// 是否通过
    ok: bool,
    /// 诊断描述
    message: String,
}

/// 网络诊断（扫描无结果时前端调用）
#[tauri::command]
async fn diagnose_network() -> Vec<DiagnosticItem> {
    let mut results = Vec::new();
    let udp_port = kvm_core::network::DEFAULT_UDP_PORT;
    let is_en = tauri_commerce::i18n::get_locale().starts_with("en");

    // 1. 检查 UDP 端口是否可绑定
    let port_ok = match tokio::net::UdpSocket::bind(format!("0.0.0.0:{}", udp_port)).await {
        Ok(_socket) => {
            log::info!("[Diag] UDP 端口 {} 绑定成功", udp_port);
            true
        }
        Err(e) => {
            log::warn!("[Diag] UDP 端口 {} 绑定失败: {}", udp_port, e);
            false
        }
    };
    results.push(DiagnosticItem {
        check: "udp_port".to_string(),
        ok: port_ok,
        message: if port_ok {
            if is_en {
                format!("UDP port {} is available", udp_port)
            } else {
                format!("UDP 端口 {} 可用", udp_port)
            }
        } else if is_en {
            format!("UDP port {} is in use or cannot bind", udp_port)
        } else {
            format!("UDP 端口 {} 被占用或无法绑定", udp_port)
        },
    });

    // 2. 检查物理 LAN 网卡
    let lan_ifaces = network_scan::list_lan_ipv4_ifaces();
    let has_lan = !lan_ifaces.is_empty();
    results.push(DiagnosticItem {
        check: "network_interface".to_string(),
        ok: has_lan,
        message: if has_lan {
            let names: Vec<String> = lan_ifaces
                .iter()
                .map(|i| format!("{}({})", i.name, i.ipv4))
                .collect();
            if is_en {
                format!("Found {} physical LAN interface(s): {}", names.len(), names.join(", "))
            } else {
                format!("检测到 {} 个物理局域网接口: {}", names.len(), names.join(", "))
            }
        } else if is_en {
            "No active physical LAN interface found (only loopback / VPN)".to_string()
        } else {
            "未检测到物理局域网接口（仅 loopback / VPN）".to_string()
        },
    });

    // 3. 检查是否存在 VPN 网卡（提供具体的可操作提示）
    let vpn_ifaces = network_scan::detect_vpn_ifaces();
    let has_vpn = !vpn_ifaces.is_empty();
    results.push(DiagnosticItem {
        check: "vpn".to_string(),
        ok: !has_vpn,
        message: if has_vpn {
            if is_en {
                format!(
                    "VPN interface(s) detected ({}). VPN may hijack the default route and break LAN broadcast. Try temporarily disconnecting the VPN, then rescan.",
                    vpn_ifaces.join(", ")
                )
            } else {
                format!(
                    "检测到 VPN 网卡 ({})，VPN 可能劫持默认路由导致局域网广播失败。建议临时断开 VPN 后重新扫描。",
                    vpn_ifaces.join(", ")
                )
            }
        } else if is_en {
            "No VPN interface detected".to_string()
        } else {
            "未检测到 VPN 网卡".to_string()
        },
    });

    // 4. 平台特定：macOS 检查防火墙（含可执行的修复命令）
    #[cfg(target_os = "macos")]
    {
        let fw_status = std::process::Command::new("/usr/libexec/ApplicationFirewall/socketfilterfw")
            .arg("--getglobalstate")
            .output();
        match fw_status {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let fw_enabled = stdout.contains("enabled");
                results.push(DiagnosticItem {
                    check: "firewall".to_string(),
                    ok: !fw_enabled,
                    message: if fw_enabled {
                        if is_en {
                            "macOS firewall is ON. Run: sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/KVMini.app && sudo /usr/libexec/ApplicationFirewall/socketfilterfw --unblock /Applications/KVMini.app".to_string()
                        } else {
                            "macOS 防火墙已开启。请执行: sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/KVMini.app && sudo /usr/libexec/ApplicationFirewall/socketfilterfw --unblock /Applications/KVMini.app".to_string()
                        }
                    } else if is_en {
                        "macOS firewall is off".to_string()
                    } else {
                        "macOS 防火墙未开启".to_string()
                    },
                });
            }
            Err(_) => {
                results.push(DiagnosticItem {
                    check: "firewall".to_string(),
                    ok: true,
                    message: if is_en {
                        "Cannot detect firewall state".to_string()
                    } else {
                        "无法检测防火墙状态".to_string()
                    },
                });
            }
        }
    }

    // 4. 平台特定：Windows 检查防火墙（含可执行的修复命令）
    #[cfg(target_os = "windows")]
    {
        let fw_status = std::process::Command::new("netsh")
            .args(["advfirewall", "show", "currentprofile", "state"])
            .output();
        match fw_status {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let fw_on = stdout.contains("ON") || stdout.contains("启用");
                results.push(DiagnosticItem {
                    check: "firewall".to_string(),
                    ok: !fw_on,
                    message: if fw_on {
                        if is_en {
                            "Windows firewall is ON. Allow KVMini in 'Windows Defender Firewall > Allowed apps', or run: netsh advfirewall firewall add rule name=KVMini dir=in action=allow program=\"%LOCALAPPDATA%\\KVMini\\KVMini.exe\" enable=yes".to_string()
                        } else {
                            "Windows 防火墙已开启。请在「Windows Defender 防火墙 > 允许的应用」中放行 KVMini，或执行: netsh advfirewall firewall add rule name=KVMini dir=in action=allow program=\"%LOCALAPPDATA%\\KVMini\\KVMini.exe\" enable=yes".to_string()
                        }
                    } else if is_en {
                        "Windows firewall is off".to_string()
                    } else {
                        "Windows 防火墙未开启".to_string()
                    },
                });
            }
            Err(_) => {
                results.push(DiagnosticItem {
                    check: "firewall".to_string(),
                    ok: true,
                    message: if is_en {
                        "Cannot detect firewall state".to_string()
                    } else {
                        "无法检测防火墙状态".to_string()
                    },
                });
            }
        }
    }

    log::info!("[Diag] 网络诊断完成: {} 项检查", results.len());
    results
}

// ── 显示器信息 ─────────────────────────────────────────────────────────────

/// 显示器信息（序列化给前端）
#[derive(serde::Serialize, Clone)]
struct MonitorInfo {
    id: u32,
    width: i32,
    height: i32,
    offset_x: i32,
    offset_y: i32,
    dpi_scale: f32,
    is_primary: bool,
}

/// 获取本机所有显示器信息
#[tauri::command]
fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    use platform::screen::get_monitors_impl;
    let monitors = get_monitors_impl()
        .map_err(|e| e.to_string())?;
    Ok(monitors.into_iter().map(|m| MonitorInfo {
        id: m.id,
        width: m.width,
        height: m.height,
        offset_x: m.offset_x,
        offset_y: m.offset_y,
        dpi_scale: m.dpi_scale,
        is_primary: m.is_primary,
    }).collect())
}

/// 获取布局配置
#[tauri::command]
fn get_layout_config() -> Result<config::LayoutConfig, String> {
    Ok(config::load_layout_config())
}

/// 保存布局配置
#[tauri::command]
fn set_layout_config(config: config::LayoutConfig) -> Result<(), String> {
    config::save_layout_config(&config)
}

/// 获取当前平台名称
#[tauri::command]
fn get_platform() -> String {
    platform::platform_name().to_string()
}

/// 检查 macOS 辅助功能权限
#[tauri::command]
fn check_accessibility_permission() -> String {
    #[cfg(target_os = "macos")]
    {
        let status = platform::macos::permission::check_accessibility();
        log::info!("[Mac] check_accessibility_permission 被调用，结果: {:?}", status);
        return status.to_string();
    }
    #[cfg(not(target_os = "macos"))]
    "N/A".to_string()
}

/// 请求 macOS 辅助功能权限（弹出系统对话框）
/// 如果请求后仍为 Denied，启动轮询直到授权
#[tauri::command]
fn request_accessibility_permission(app_handle: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        platform::macos::permission::request_accessibility().map_err(|e| e.to_string())?;
        // 请求后若仍未授权，启动轮询（覆盖启动时轮询已结束的情况）
        let status = platform::macos::permission::check_accessibility();
        if status != platform::macos::permission::AccessibilityStatus::Granted {
            start_accessibility_polling(app_handle);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app_handle;
    Ok(())
}

/// 打开 macOS 辅助功能设置页面
#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { platform::macos::permission::open_accessibility_settings().map_err(|e| e.to_string())?; }
    Ok(())
}

/// 检查 macOS 输入监控权限（BUG-027）
/// 返回 "Granted" / "Denied" / "N/A"（非 macOS）
#[tauri::command]
fn check_input_monitoring_permission() -> String {
    #[cfg(target_os = "macos")]
    {
        let status = platform::macos::permission::check_input_monitoring();
        return status.to_string();
    }
    #[cfg(not(target_os = "macos"))]
    "N/A".to_string()
}

/// 打开 macOS 输入监控设置页面（BUG-027）
#[tauri::command]
fn open_input_monitoring_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { platform::macos::permission::open_input_monitoring_settings().map_err(|e| e.to_string())?; }
    Ok(())
}

/// 获取自启动状态
#[tauri::command]
fn get_autostart_status() -> bool {
    platform::autostart::is_autostart_enabled()
}

/// 设置自启动
#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        platform::autostart::enable_autostart()
    } else {
        platform::autostart::disable_autostart()
    }
}

/// 写入启动诊断日志到 ~/Library/Logs/KVMini/startup.log（macOS 专属）
/// 非 macOS 平台直接返回 Ok
#[tauri::command]
fn write_startup_log(_content: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let content = _content;
    #[cfg(target_os = "macos")]
    {
        use std::fs;
        use std::path::PathBuf;

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_dir = PathBuf::from(&home).join("Library").join("Logs").join("KVMini");

        // 自动创建目录
        fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;

        let log_path = log_dir.join("startup.log");

        // 追加写入
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("打开日志文件失败: {}", e))?;

        file.write_all(content.as_bytes())
            .map_err(|e| format!("写入日志失败: {}", e))?;

        log::info!("[Diagnostics] 诊断日志已写入: {:?}", log_path);
    }
    Ok(())
}

/// 获取当前运行时日志文件路径（供前端"打开日志"按钮使用）
#[tauri::command]
fn get_log_path() -> Result<String, String> {
    resolve_log_dir()
        .map(|d| d.join("runtime.log").to_string_lossy().into_owned())
        .ok_or_else(|| "无法确定日志目录".to_string())
}

/// v0.1.54: 一键开启/关闭调试模式
///
/// 在日志目录下创建/删除 `debug_mode` 文件。
/// 创建后**重启应用**即切换到 debug 级别日志，可诊断拖拽/滚轮/状态切换等问题。
/// 返回操作结果和日志文件路径，方便前端展示提示。
#[tauri::command]
fn set_debug_log(enabled: bool) -> Result<String, String> {
    let dir = resolve_log_dir().ok_or_else(|| "无法确定日志目录".to_string())?;
    let flag = dir.join("debug_mode");
    if enabled {
        std::fs::write(&flag, b"").map_err(|e| format!("创建 debug_mode 失败: {}", e))?;
        let log_path = dir.join("runtime.log").to_string_lossy().into_owned();
        log::info!("[Logger] 调试模式已开启（重启后生效），日志文件: {}", log_path);
        Ok(log_path)
    } else {
        if flag.exists() {
            std::fs::remove_file(&flag).map_err(|e| format!("删除 debug_mode 失败: {}", e))?;
        }
        log::info!("[Logger] 调试模式已关闭（重启后生效）");
        Ok(dir.join("runtime.log").to_string_lossy().into_owned())
    }
}

// ── 自动更新 ──────────────────────────────────────────────────────────────

/// 更新信息（序列化给前端）
#[derive(serde::Serialize, Clone)]
struct UpdateInfo {
    version: String,
    body: Option<String>,
    date: Option<String>,
}

/// 检查是否有可用更新
#[tauri::command]
async fn check_for_update(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    log::info!("[Updater] 开始检查更新...");
    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    log::info!("[Updater] 发现新版本: {}", update.version);
                    Ok(Some(UpdateInfo {
                        version: update.version.clone(),
                        body: update.body.clone(),
                        date: update.date.map(|d| d.to_string()),
                    }))
                }
                Ok(None) => {
                    log::info!("[Updater] 已是最新版本");
                    Ok(None)
                }
                Err(e) => {
                    log::warn!("[Updater] 检查更新失败: {}", e);
                    Err(format!("检查更新失败: {}", e))
                }
            }
        }
        Err(e) => {
            log::warn!("[Updater] Updater 初始化失败: {}", e);
            Err(format!("Updater 不可用: {}", e))
        }
    }
}

/// 下载并安装更新
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    log::info!("[Updater] 开始下载并安装更新...");
    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    log::info!("[Updater] 下载更新 v{}...", update.version);
                    let _ = app.emit("update-progress", serde_json::json!({
                        "status": "downloading",
                        "version": update.version,
                    }));
                    update.download_and_install(
                        |chunk_length, content_length| {
                            log::debug!("[Updater] 下载进度: {} / {:?}", chunk_length, content_length);
                        },
                        || {
                            log::info!("[Updater] 下载完成，准备安装");
                        },
                    ).await.map_err(|e| {
                        log::error!("[Updater] 下载安装失败: {}", e);
                        format!("下载安装失败: {}", e)
                    })?;
                    log::info!("[Updater] 更新安装完成，即将重启...");
                    let _ = app.emit("update-progress", serde_json::json!({
                        "status": "installed",
                    }));
                    Ok(())
                }
                Ok(None) => {
                    Err("没有可用更新".to_string())
                }
                Err(e) => {
                    Err(format!("检查更新失败: {}", e))
                }
            }
        }
        Err(e) => {
            Err(format!("Updater 不可用: {}", e))
        }
    }
}

/// 启动后台定期检查更新（启动时检查一次，之后每 4 小时检查一次）
fn start_background_update_check(app: tauri::AppHandle) {
    use std::thread;
    use std::time::Duration;

    thread::spawn(move || {
        // 启动后延迟 10 秒再检查（等待 UI 完全加载）
        thread::sleep(Duration::from_secs(10));

        loop {
            let app_clone = app.clone();
            // 在异步运行时中执行检查
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(async {
                    match app_clone.updater() {
                        Ok(updater) => {
                            match updater.check().await {
                                Ok(Some(update)) => {
                                    log::info!("[Updater] 后台检查：发现新版本 v{}", update.version);
                                    let _ = app_clone.emit("update-available", serde_json::json!({
                                        "version": update.version,
                                        "body": update.body,
                                        "date": update.date.map(|d| d.to_string()),
                                    }));
                                }
                                Ok(None) => {
                                    log::debug!("[Updater] 后台检查：已是最新版本");
                                }
                                Err(e) => {
                                    log::debug!("[Updater] 后台检查失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::debug!("[Updater] Updater 不可用: {}", e);
                        }
                    }
                });
            }

            // 每 4 小时检查一次
            thread::sleep(Duration::from_secs(4 * 60 * 60));
        }
    });
}

// ── tauri-commerce SDK 命令 ──────────────────────────────────────────────────

/// 获取试用/购买状态
#[tauri::command]
fn get_trial_status() -> tauri_commerce::trial::TrialStatus {
    tauri_commerce::trial::get_trial_status()
}

/// 获取当前语言
#[tauri::command]
fn get_locale() -> String {
    tauri_commerce::i18n::get_locale()
}

/// 设置语言
#[tauri::command]
fn set_locale(locale: String) -> Result<(), String> {
    tauri_commerce::i18n::set_locale(&locale);
    log::info!("[i18n] 语言已切换为: {}", locale);
    Ok(())
}

/// v0.1.41: 检查 Windows 管理员权限（Mac 端始终返回 true）
#[tauri::command]
fn check_admin_permission() -> serde_json::Value {
    #[cfg(target_os = "windows")]
    {
        let provider = crate::platform::windows::permission::WinPermissionProvider::new();
        use kvm_core::platform::PermissionProvider;
        let status = provider.permission_status();
        serde_json::json!({
            "elevated": status.elevated,
            "admin_required": status.admin_required
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        serde_json::json!({
            "elevated": true,
            "admin_required": false
        })
    }
}

/// v0.1.41: 请求提权（Windows 以管理员身份重启）
#[tauri::command]
fn request_admin_elevation() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let provider = crate::platform::windows::permission::WinPermissionProvider::new();
        use kvm_core::platform::PermissionProvider;
        provider.request_elevation().map_err(|e| format!("{}", e))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

/// 获取更新日志（KVMini 实际版本历史）
#[tauri::command]
fn get_changelog() -> Vec<tauri_commerce::changelog::ChangelogEntry> {
    vec![
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.44".to_string(),
            date: "2026-05-09".to_string(),
            changes: vec![
                "修复 Win 被控端断开连接后键鼠锁死（Bug #036）：主控端点「停止共享并退出」后，Win 端 Hook 层 should_swallow() 仍在吞事件 → 键鼠完全无响应".to_string(),
                "broadcast channel Lagged 防护：事件循环 RecvError::Lagged 改为 warn + continue（不再 break 退出），确保 Disconnected 事件不会因缓冲区溢出被丢失".to_string(),
                "on_disconnected 纵深防御：正常事件处理链路后增加显式兜底——强制 control_state=Idle + 检查 blocker 是否仍 blocked + 强制 recouple_physical_mouse + 清理 remote_cursor".to_string(),
                "WinInputBlocker 新增 force_unblock() 方法：极端场景（进程退出前 / 状态异常）强制调用 BlockInput(FALSE) 确保键鼠恢复".to_string(),
            ],
            changes_en: vec![
                "Fix Win target keyboard/mouse lockup on disconnect (Bug #036): after controller clicks 'Stop Sharing & Exit', Win hook layer should_swallow() keeps swallowing events → keyboard and mouse completely unresponsive".to_string(),
                "Broadcast channel Lagged protection: event loop RecvError::Lagged now logs warning + continues (no longer breaks), ensuring Disconnected events are never lost due to buffer overflow".to_string(),
                "on_disconnected defense-in-depth: after normal event processing, explicitly force control_state=Idle + check/force unblock blocker + recouple physical mouse + clear remote cursor".to_string(),
                "WinInputBlocker adds force_unblock() method: for extreme scenarios (pre-exit cleanup / abnormal state) forcibly calls BlockInput(FALSE) to guarantee input recovery".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.43".to_string(),
            date: "2026-05-09".to_string(),
            changes: vec![
                "修复远端虚拟光标不显示（Bug #034）：被控端进入 RemoteControlling 时主动调 CGAssociateMouseAndMouseCursorPosition(0) 解耦物理光标 + 显式 unhide，让本机撞边后远端屏幕能看到虚拟光标随 MouseMoveDelta 实时移动".to_string(),
                "新增远端物理输入抢占回切（Feature #035，Synergy / Logitech Flow 风格）：被控端在 RemoteControlling 下检测到物理鼠标累计位移 ≥ 40px 或任意键盘按下，立即发 SwitchBackControl{reason=Preempt} 让控制端秒退 Armed 态，被控端自动切回 Idle 并解除 BlockInput".to_string(),
                "控制端抢占回切时弹 3 秒 toast「远端用户恢复了本机控制 / Remote user resumed control」，被控端静默，符合主流 KM 共享产品体验".to_string(),
                "协议扩展：KvmFrame::SwitchBackControl 新增 reason 字段（1 字节 payload），向后兼容 v0.1.42 及以下版本（空 payload 自动解析为 EdgeReturn）".to_string(),
                "合成事件过滤：Mac 端给所有注入事件打 kCGEventSourceUserData magic tag（0x4B56_4D49 \"KVMI\"），Win 端依赖 LLMHF_INJECTED / LLKHF_INJECTED flag，确保被控端吞掉自己注入的事件时不会误触发抢占".to_string(),
                "新增核心模块 kvm-core/preempt.rs：无锁 PreemptDetector（AtomicI64 累加 dx²+dy² + AtomicBool triggered），18 单元测试护航阈值 / 勾股 / reset / 状态机隔离".to_string(),
                "Mac + Win 两端同步实现、行为一致、代码隔离；整体新增/扩展 248 Rust 测试 + 33 i18n 测试全绿".to_string(),
            ],
            changes_en: vec![
                "Fix remote virtual cursor invisible (Bug #034): when target enters RemoteControlling, proactively call CGAssociateMouseAndMouseCursorPosition(0) to decouple physical cursor and explicitly unhide it, so after the controller hits the edge the virtual cursor becomes visible on remote and follows MouseMoveDelta in real time".to_string(),
                "Add remote physical input preemption (Feature #035, Synergy / Logitech Flow style): while RemoteControlling, target detects local physical mouse cumulative movement ≥ 40px or any key press, immediately sends SwitchBackControl{reason=Preempt} to flip controller back to Armed, and target auto-returns to Idle + releases BlockInput".to_string(),
                "Controller shows a 3-second toast 'Remote user resumed control' on preempt; target stays silent — matches mainstream KM sharing UX".to_string(),
                "Protocol extension: KvmFrame::SwitchBackControl adds a reason field (1-byte payload). Backward compatible with v0.1.42 and older (empty payload decodes as EdgeReturn)".to_string(),
                "Synthetic event filtering: Mac tags every injected event with kCGEventSourceUserData magic number (0x4B56_4D49 'KVMI'); Win relies on LLMHF_INJECTED / LLKHF_INJECTED flags — so the target never preempts itself when its own injected events bubble through the capture layer".to_string(),
                "New core module kvm-core/preempt.rs: lock-free PreemptDetector (AtomicI64 accumulates dx²+dy² + AtomicBool triggered), covered by 18 unit tests on threshold / pythagorean / reset / state isolation".to_string(),
                "Mac + Win parity: identical behavior, isolated platform code; 248 Rust tests + 33 i18n tests all green".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.42".to_string(),
            date: "2026-05-08".to_string(),
            changes: vec![
                "Mac 端控制响应彻底修复：v0.1.41 的 FocusKeeper 抢前台焦点（NSApp.activateIgnoringOtherApps）导致键盘失效/点击丢失/误触发拖拽/本机闪烁。v0.1.42 改为业界主流方案 — 仅周期性调 CGAssociateMouseAndMouseCursorPosition(0) keep-alive，不再创建窗口、不再抢焦点".to_string(),
                "修复隐藏 bug：cursor.warp_cursor 在 grab 状态下不再误调 CGAssociate(1)，避免远端绝对定位时悄悄解除光标 grab".to_string(),
                "前端 LocalControlling 状态下持续显示连接延迟（RTT），UI 提示更清晰".to_string(),
                "Mac 手势屏蔽保留：LeftDragged/RightDragged/OtherDragged + kCGEventGesture/Pinch/Rotate/Swipe 在控制远端时全部 swallow".to_string(),
                "Win 端继续完善 RawInput 双源、多屏 VIRTUALDESK、XButton、滚轮 i32 cast、Hook 5s 健康检查、UAC 权限横幅、WM_DISPLAYCHANGE 监听（详见 0.1.41 条目）".to_string(),
            ],
            changes_en: vec![
                "Mac control responsiveness fully fixed: v0.1.41's FocusKeeper used NSApp.activateIgnoringOtherApps every 200ms to grab foreground, which broke keyboard input, swallowed clicks, misfired drags and flashed the local desktop. v0.1.42 follows the industry-standard approach (Synergy/Barrier/Logitech Flow) — only periodically calls CGAssociateMouseAndMouseCursorPosition(0) keep-alive, no longer creates a window, no longer steals focus".to_string(),
                "Hidden bug fix: cursor.warp_cursor no longer calls CGAssociate(1) while grabbed, preventing silent grab release on remote absolute positioning".to_string(),
                "Frontend now keeps showing latency (RTT) during LocalControlling for clearer UX".to_string(),
                "Mac gesture blocking retained: LeftDragged/RightDragged/OtherDragged + kCGEventGesture/Pinch/Rotate/Swipe all swallowed when controlling remote".to_string(),
                "Win side continues parity work: RawInput dual-source, multi-monitor VIRTUALDESK, XButton, signed i32 wheel cast, hook 5s health check, UAC banner, WM_DISPLAYCHANGE listener (see 0.1.41 notes)".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.41".to_string(),
            date: "2026-05-08".to_string(),
            changes: vec![
                "Mac 端修复「失焦后键鼠屏蔽失效」：新增 FocusKeeper 模块，进入控制模式时创建透明窗口 + 200ms 自动激活循环，确保 CGAssociate 永不被系统重置".to_string(),
                "Mac 端手势屏蔽：LocalControlling 时吞掉所有高级手势（三指/四指滑动、Pinch、Rotate），避免触发 Mission Control / Launchpad".to_string(),
                "Win 端 RawInput 硬件 delta：解决 ClipCursor 钳住光标后 LL Hook dx/dy 恒为 0 的致命 bug，远端鼠标终于能动了".to_string(),
                "Win 端 XButton（侧键）支持：补全 Side1/Side2 按钮的采集和注入".to_string(),
                "Win 端滚轮修复：正确使用 i32 有符号右移表达负向滚动".to_string(),
                "Win 端 UAC 权限检测：新增权限横幅，提示用户以管理员身份运行以获得完整 Hook 功能".to_string(),
                "Win 端 Hook 5s 健康检查：应对 LowLevelHooksTimeout 静默移除问题".to_string(),
                "跨平台 FocusKeeper + PermissionProvider trait 抽象，Win 端空实现/Mac 端完整实现，互不影响".to_string(),
            ],
            changes_en: vec![
                "Mac: Fix 'input blocking lost on focus change' — new FocusKeeper module creates a 1×1 transparent window + 200ms auto-activate loop to keep CGAssociate from being reset".to_string(),
                "Mac: Gesture blocking in LocalControlling — swallows trackpad gestures (swipe/pinch/rotate) to prevent Mission Control / Launchpad triggers".to_string(),
                "Win: RawInput hardware delta — solves critical bug where ClipCursor clamping caused LL Hook dx/dy = 0, remote mouse frozen".to_string(),
                "Win: XButton (side button) support — Side1/Side2 capture and injection added".to_string(),
                "Win: Scroll fix — correct signed i32 shift for negative scroll direction".to_string(),
                "Win: UAC permission detection — new banner prompts user to run as admin for full Hook functionality".to_string(),
                "Win: Hook 5s health check — guards against silent LowLevelHooksTimeout removal".to_string(),
                "Cross-platform FocusKeeper + PermissionProvider trait abstraction, Windows no-op / Mac full implementation, safely isolated".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.40".to_string(),
            date: "2026-05-07".to_string(),
            changes: vec![
                "BUG-033 致命修复：本机进入「控制远端」后远端鼠标不动——根因是 grab() 调用 CGAssociateMouseAndMouseCursorPosition(0) 后 event.location() 冻结；改为读取 CGEvent 硬件 delta 字段（kCGMouseEventDeltaX/DeltaY），参考 Synergy/Barrier 竞品方案".to_string(),
                "协议破坏性升级：KvmFrame::MouseMove{x,y} → MouseMoveDelta{dx,dy}，双端必须同时升级到 v0.1.40".to_string(),
                "新增远端虚拟光标 RemoteCursorState：累加 delta + 屏幕 clamp + 反向边缘检测，贴反向边缘 N 次自动回切".to_string(),
                "核心改造：新增 Armed 中间态（4 状态机），「开始共享」变为激活边缘检测，鼠标推到屏幕边缘才真正切换控制权".to_string(),
                "边缘过冲量防误触：需推过边缘 N 像素才触发，默认 20px，用户可配置 0~200".to_string(),
                "Y 坐标镜像映射：切换时保留鼠标 Y 位置比例，光标按比例落到远端对应位置".to_string(),
                "双通道回切：Ctrl+Alt+S 快捷键 + 远端反向边缘检测（新增 SwitchBackControl 协议帧）".to_string(),
                "BUG-032 修复：连接状态下 reconnecting 事件不再导致按钮误显示「重连」（改为以 appState.connected 单一真源判断）".to_string(),
            ],
            changes_en: vec![
                "Critical BUG-033 fix: Remote mouse frozen in 'Control Remote' mode - root cause: event.location() freezes after grab() disassociates cursor; switched to CGEvent hardware delta (kCGMouseEventDeltaX/DeltaY), following Synergy/Barrier design".to_string(),
                "Breaking protocol upgrade: KvmFrame::MouseMove{x,y} -> MouseMoveDelta{dx,dy}. Both sides must upgrade to v0.1.40".to_string(),
                "New RemoteCursorState: accumulates delta + clamps + reverse edge detection; auto-sends SwitchBackControl after N consecutive hits".to_string(),
                "Core overhaul: Armed intermediate state (4-state machine). 'Start Sharing' activates edge detection; real transfer happens on edge hit".to_string(),
                "Edge overshoot: N pixels past edge (default 20, configurable 0~200)".to_string(),
                "Y-coordinate mirror mapping preserves ratio across sides".to_string(),
                "Dual-channel return: Ctrl+Alt+S hotkey + remote reverse edge detection (new SwitchBackControl frame)".to_string(),
                "Fix BUG-032: reconnecting events no longer flip disconnect button when still connected".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.39".to_string(),
            date: "2026-05-06".to_string(),
            changes: vec![
                "BUG-031 修复：macOS 已授予「辅助功能」但未授予「输入监控」时，横幅依然提示「需要辅助功能权限」导致用户误判——横幅改为按缺失权限类型显示差异化文案".to_string(),
                "权限横幅新增「前往设置」按钮，按缺失权限一键跳转到对应的系统偏好设置页（辅助功能 / 输入监控）".to_string(),
                "输入监控权限授权后在横幅文案中明确提示必须完全退出 KVMini（Cmd+Q）再重新打开，权限才会生效（CGPreflightListenEventAccess 进程内缓存限制）".to_string(),
                "i18n 新增 4 条 key 并中英双语同步：permissionWarningText / permissionWarningInputMonitoringText / permissionWarningBtnAccessibility / permissionWarningBtnInputMonitoring".to_string(),
            ],
            changes_en: vec![
                "Fix BUG-031: on macOS with Accessibility granted but Input Monitoring denied, the banner still said \"Accessibility permission required\", misleading users — banner now renders distinct text per missing permission kind".to_string(),
                "Permission banner now has an action button that jumps directly to the correct System Settings pane (Accessibility or Input Monitoring) based on the missing permission".to_string(),
                "Input Monitoring banner explicitly tells users they must fully quit KVMini (Cmd+Q) and relaunch for the newly granted permission to take effect (process-cached CGPreflightListenEventAccess limitation)".to_string(),
                "i18n: added 4 new keys in sync across zh/en: permissionWarningText / permissionWarningInputMonitoringText / permissionWarningBtnAccessibility / permissionWarningBtnInputMonitoring".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.38".to_string(),
            date: "2026-05-06".to_string(),
            changes: vec![
                "BUG-020 修复：扫描到设备后点击「连接」无响应——前端错误调用不存在的 connect_to_device 命令，已统一改为 connect_to_peer，手动连接、历史记录、重连三路径同步修复".to_string(),
                "BUG-021 修复：端口默认值混乱——清除 main.js/index.html/history.js 所有硬编码 8889，启动时从后端 get_settings 读取 tcp_port 填充 appState.defaults.tcpPort；tcp_port==0 回退到 23456 真源值；保存监听端口后连接对话框默认值立即反映新值".to_string(),
                "BUG-022 修复：共享开启后两台电脑同时响应键鼠——macOS CGEventTap 由 ListenOnly 改为 Default，按 AtomicU8 控制状态决定吞/放事件；Windows Low-Level Hook 对称实现；SharingManager 接入 EdgeDetector，鼠标触达布局边缘自动切换控制权；BlockInput/UnblockInput 分支真实调用 InputBlocker".to_string(),
                "设置页通用分页新增「键鼠切换」分区：边缘触发开关 + 灵敏度（像素）输入框，保存后立即生效；i18n 补齐 settings_kvm_section_title / settings_edge_trigger_enabled / settings_edge_threshold 等 key，中英双语同步".to_string(),
                "save_settings 前端 payload 显式构造 snake_case 字段（edge_trigger / edge_threshold / switch_hotkey 等），使后端 AppConfig 正确反序列化，彻底告别历史 camelCase 被整体忽略的问题".to_string(),
                "构建侧：轮换 updater minisign 公钥（新 key ID 6274F20D6F01A7A1），build.sh 为无密码签名增加 TAURI_SIGNING_PRIVATE_KEY_PASSWORD 空值兜底，解决 CI/非 TTY 环境 prompt 卡住".to_string(),
            ],
            changes_en: vec![
                "Fix BUG-020: clicking Connect on a discovered device did nothing — the frontend was calling a non-existent connect_to_device command. Unified to connect_to_peer; manual connect, history reconnect and discover-list reconnect paths all fixed".to_string(),
                "Fix BUG-021: port default chaos — removed all hard-coded 8889 from main.js / index.html / history.js; at startup the frontend reads tcp_port from get_settings into appState.defaults.tcpPort; tcp_port==0 falls back to 23456 (kvm-core source of truth); after saving listen port the connect dialog default reflects the new value immediately".to_string(),
                "Fix BUG-022: both machines reacting to mouse/keyboard at the same time — switched macOS CGEventTap from ListenOnly to Default and swallow/pass events based on an AtomicU8 control state; symmetric Low-Level Hook implementation on Windows; SharingManager now drives EdgeDetector so the cursor hitting the configured layout edge auto-switches control; BlockInput/UnblockInput branches now actually call InputBlocker".to_string(),
                "Settings: added a KVM Switching section under General — auto-edge-switch toggle + edge sensitivity (pixels) input, applied immediately on save; i18n adds settings_kvm_section_title / settings_edge_trigger_enabled / settings_edge_threshold etc., zh/en in sync".to_string(),
                "Frontend save_settings now explicitly builds a snake_case payload (edge_trigger / edge_threshold / switch_hotkey …) so the Rust AppConfig deserializes correctly — the historical camelCase-gets-silently-ignored bug is finally gone".to_string(),
                "Build: rotated updater minisign public key (new key id 6274F20D6F01A7A1); build.sh now exports an empty TAURI_SIGNING_PRIVATE_KEY_PASSWORD as a fallback for passwordless keys so CI / non-TTY environments no longer hang on a prompt".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.37".to_string(),
            date: "2026-05-01".to_string(),
            changes: vec![
                "前端暂时屏蔽试用到期与购买相关界面，统一视为已购买，后端 Commerce/IAP 初始化和 Keychain 记录完全保留，已购用户数据不会丢失".to_string(),
                "关于页「许可状态」固定显示为『已购买』，购买/恢复按钮隐藏；将来通过单一开关即可恢复完整商业化 UI".to_string(),
            ],
            changes_en: vec![
                "Temporarily hide trial-expired and purchase UI on the frontend; all users are treated as licensed. Backend Commerce/IAP init and Keychain records are fully preserved — existing purchases will NOT be lost".to_string(),
                "About page now always shows 'Purchased', purchase/restore buttons are hidden. A single flag restores the full commerce UI when needed".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.36".to_string(),
            date: "2026-05-01".to_string(),
            changes: vec![
                "后端启动持久化日志：日志同时写入 stderr 和 ~/Library/Logs/KVMini/runtime.log（Windows: %LOCALAPPDATA%/KVMini/Logs/runtime.log），方便用户自查和反馈".to_string(),
                "macOS 权限自检完整化：同时检查辅助功能 + 输入监控（Input Monitoring）两项权限，任一缺失均在启动时弹窗提醒，带「打开设置」按钮，修复以前只提醒一个权限的问题（BUG-027）".to_string(),
                "macOS 主动调用 CGRequestListenEventAccess 将 KVMini 注册到 TCC 输入监控列表，否则用户可能在系统设置里根本看不到应用开关（BUG-021）".to_string(),
            ],
            changes_en: vec![
                "Persistent backend logs: written to both stderr and ~/Library/Logs/KVMini/runtime.log (Windows: %LOCALAPPDATA%/KVMini/Logs/runtime.log) for easier diagnostics".to_string(),
                "Complete macOS permission self-check: validates both Accessibility and Input Monitoring permissions at startup; shows a bilingual dialog with 'Open Settings' button if either is missing (BUG-027)".to_string(),
                "Proactively call CGRequestListenEventAccess so KVMini is registered in the TCC Input Monitoring list — otherwise the toggle won't show up in System Settings (BUG-021)".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.35".to_string(),
            date: "2026-05-01".to_string(),
            changes: vec![
                "修复连接成功后误报「TLS 不一致」（双端均开启 TLS 时不再弹警告）".to_string(),
                "接入键鼠共享主流程：「开始共享」按钮真正可用，点击后双端键鼠互相转发，再次点击恢复".to_string(),
                "扫描鲁棒性：自动过滤 VPN/虚拟网卡，多接口定向广播 + 多网段并发扫描，无结果时给出可操作的诊断提示（中英双语）".to_string(),
                "macOS CGEventTap 在专用线程跑 CFRunLoop，避免与 Tauri 主线程冲突".to_string(),
            ],
            changes_en: vec![
                "Fix false 'TLS mismatch' warning when both sides have TLS enabled".to_string(),
                "Wire up keyboard/mouse sharing: 'Start Sharing' button is now functional — click to take over remote, click again to release".to_string(),
                "Scan robustness: filter VPN/virtual interfaces, per-interface targeted UDP broadcast + multi-subnet TCP scan, with actionable bilingual diagnostics on empty result".to_string(),
                "macOS CGEventTap now runs CFRunLoop in a dedicated thread to avoid conflict with Tauri main thread".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.27".to_string(),
            date: "2026-04-29".to_string(),
            changes: vec![
                "防多开：第二次启动时聚焦已有窗口，解决菜单栏出现多个图标问题".to_string(),
                "版本号统一：以 Cargo.toml 为唯一真相，前端从后端读取，消除多处版本号不一致".to_string(),
                "Developer ID 签名集成：Launcher 和 build.sh 均自动使用 Developer ID 签名".to_string(),
            ],
            changes_en: vec![
                "Single instance: second launch focuses existing window, fixes multiple tray icons".to_string(),
                "Version unification: Cargo.toml as single source of truth, frontend reads from backend".to_string(),
                "Developer ID signing integrated: Launcher and build.sh auto-sign with Developer ID".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.25".to_string(),
            date: "2026-04-28".to_string(),
            changes: vec![
                "修复心跳保活：收到 Heartbeat 自动回复 HeartbeatAck，解决连接约 15 秒自动断开问题".to_string(),
                "扫描升级：UDP 广播无结果时自动降级为子网 TCP 扫描，解决 VPN/防火墙环境下扫描失败".to_string(),
            ],
            changes_en: vec![
                "Fix heartbeat keepalive: auto-reply HeartbeatAck on Heartbeat, fixes connection dropping after ~15s".to_string(),
                "Scan upgrade: fallback to subnet TCP scan when UDP broadcast finds nothing, fixes VPN/firewall environments".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.24".to_string(),
            date: "2026-04-25".to_string(),
            changes: vec![
                "修复设置按钮点击无响应（initApp 非 async 导致事件绑定失败）".to_string(),
                "修复每次重装需重新授权辅助功能权限（集成稳定代码签名到构建流程）".to_string(),
                "构建脚本自动签名，确保 TCC 权限跨版本持久化".to_string(),
            ],
            changes_en: vec![
                "Fixed settings button not responding (initApp not async caused event binding failure)".to_string(),
                "Fixed accessibility permission lost after each reinstall (integrated stable code signing into build flow)".to_string(),
                "Build script auto-signs binaries to persist TCC permissions across versions".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.23".to_string(),
            date: "2026-04-24".to_string(),
            changes: vec![
                "更新 tauri-commerce SDK 到 v0.1.7".to_string(),
                "新增显示器布局后端命令（get_monitors / get_layout_config / set_layout_config）".to_string(),
                "修复显示器列表无法正确识别多显示器问题".to_string(),
                "多语言全覆盖：分享、反馈、隐私政策、更新日志文案均支持中英文".to_string(),
                "设置按钮新增文字标签，提升可见性".to_string(),
            ],
            changes_en: vec![
                "Updated tauri-commerce SDK to v0.1.7".to_string(),
                "Added monitor layout backend commands (get_monitors / get_layout_config / set_layout_config)".to_string(),
                "Fixed multi-monitor detection issue".to_string(),
                "Full i18n coverage: share, feedback, privacy policy, changelog all support zh/en".to_string(),
                "Settings button now shows text label for better visibility".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.22".to_string(),
            date: "2026-04-23".to_string(),
            changes: vec![
                "更新 tauri-commerce SDK 到 v0.1.6（开机自启动沙盒兼容）".to_string(),
                "新增 build.sh --reset-purchase 开发测试工具".to_string(),
                "优化 TLS 证书指纹显示".to_string(),
            ],
            changes_en: vec![
                "Updated tauri-commerce SDK to v0.1.6 (sandbox-compatible autostart)".to_string(),
                "Added build.sh --reset-purchase dev tool".to_string(),
                "Improved TLS certificate fingerprint display".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.20".to_string(),
            date: "2026-04-12".to_string(),
            changes: vec![
                "重构 macOS 辅助功能权限模块为事件驱动架构".to_string(),
                "新增连接详情面板（本机 IP / 对端 IP / TLS / 延迟）".to_string(),
                "修复 TLS 不一致检测和横幅提示".to_string(),
                "新增全量设置页测试覆盖".to_string(),
            ],
            changes_en: vec![
                "Refactored macOS accessibility permission module to event-driven architecture".to_string(),
                "Added connection detail panel (local IP / peer IP / TLS / latency)".to_string(),
                "Fixed TLS mismatch detection and banner notification".to_string(),
                "Added comprehensive settings page test coverage".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.15".to_string(),
            date: "2026-04-05".to_string(),
            changes: vec![
                "新增跨网段手动 IP 连接功能".to_string(),
                "新增监听网卡选择（支持指定具体网卡）".to_string(),
                "连接历史区分手动 IP 和自动发现两种类型".to_string(),
                "TLS 1.3 加密传输支持".to_string(),
            ],
            changes_en: vec![
                "Added cross-subnet manual IP connection".to_string(),
                "Added network interface selection for listening".to_string(),
                "Connection history now distinguishes manual IP and auto-discovered types".to_string(),
                "TLS 1.3 encrypted transport support".to_string(),
            ],
        },
        tauri_commerce::changelog::ChangelogEntry {
            version: "0.1.0".to_string(),
            date: "2026-03-31".to_string(),
            changes: vec![
                "初始版本发布".to_string(),
                "支持 macOS 和 Windows 跨平台键鼠共享".to_string(),
                "局域网自动设备发现".to_string(),
                "多显示器布局配置".to_string(),
                "深色/浅色主题支持".to_string(),
            ],
            changes_en: vec![
                "Initial release".to_string(),
                "Cross-platform keyboard/mouse sharing between macOS and Windows".to_string(),
                "LAN auto device discovery".to_string(),
                "Multi-monitor layout configuration".to_string(),
                "Dark/light theme support".to_string(),
            ],
        },
    ]
}

/// 打开反馈邮件
#[tauri::command]
fn send_feedback() -> Result<(), String> {
    let locale = tauri_commerce::i18n::get_locale();
    tauri_commerce::feedback::open_feedback_email(&locale)
}

/// 触发评分引导
#[tauri::command]
fn request_review() -> Result<(), String> {
    // 获取 App Store ID（上架后替换为真实 ID）
    let app_id = option_env!("KVMINI_APP_STORE_ID").unwrap_or("");
    if app_id.is_empty() {
        log::info!("[Review] App Store ID 未配置，跳过评分引导");
        return Ok(());
    }
    tauri_commerce::review::open_app_store_rating(app_id)
}

/// 获取隐私政策（中文）
#[tauri::command]
fn get_privacy_policy_zh() -> String {
    tauri_commerce::privacy::get_privacy_policy_zh()
}

/// 获取隐私政策（英文）
#[tauri::command]
fn get_privacy_policy_en() -> String {
    tauri_commerce::privacy::get_privacy_policy_en()
}

/// 发起内购（App Store 版）
#[tauri::command]
fn purchase(product_id: String) -> Result<(), String> {
    #[cfg(feature = "iap")]
    {
        use tauri_commerce::iap::PurchaseResult;
        match tauri_commerce::iap::purchase(&product_id) {
            PurchaseResult::Success | PurchaseResult::Pending => Ok(()),
            PurchaseResult::Cancelled => Err("用户取消了购买".to_string()),
            PurchaseResult::Failed(msg) => Err(msg),
            PurchaseResult::DevMode(msg) => {
                log::info!("[IAP] 开发模式: {}", msg);
                Ok(())
            }
        }
    }
    #[cfg(not(feature = "iap"))]
    {
        let _ = product_id;
        Err("当前版本不支持内购，请通过 App Store 购买".to_string())
    }
}

/// 恢复购买（App Store 版）
#[tauri::command]
fn restore_purchase() -> Result<(), String> {
    #[cfg(feature = "iap")]
    {
        use tauri_commerce::iap::RestoreResult;
        match tauri_commerce::iap::restore() {
            RestoreResult::Restored => Ok(()),
            RestoreResult::NoPurchaseFound => Err("未找到购买记录".to_string()),
            RestoreResult::Failed(msg) => Err(msg),
            RestoreResult::DevMode(msg) => {
                log::info!("[IAP] 开发模式恢复: {}", msg);
                Ok(())
            }
        }
    }
    #[cfg(not(feature = "iap"))]
    {
        Err("当前版本不支持内购恢复".to_string())
    }
}

// ── 全局快捷键管理 ────────────────────────────────────────────────────────────

/// 全局快捷键状态（通过 tauri::State 管理）
pub struct HotkeyState {
    /// 当前注册的快捷键字符串（空字符串表示未注册）
    current_hotkey: std::sync::Mutex<String>,
    /// 平台快捷键管理器
    #[cfg(target_os = "macos")]
    manager: platform::macos::hotkey::MacHotkeyManager,
    #[cfg(target_os = "windows")]
    manager: platform::windows::hotkey::WinHotkeyManager,
}

impl HotkeyState {
    pub fn new() -> Self {
        Self {
            current_hotkey: std::sync::Mutex::new(String::new()),
            #[cfg(target_os = "macos")]
            manager: platform::macos::hotkey::MacHotkeyManager::new(),
            #[cfg(target_os = "windows")]
            manager: platform::windows::hotkey::WinHotkeyManager::new(),
        }
    }
}

/// 注册全局快捷键
/// - hotkey: 快捷键字符串，如 "Ctrl+Alt+S"；空字符串表示注销当前快捷键
#[tauri::command]
fn register_global_hotkey(
    hotkey: String,
    state: tauri::State<HotkeyState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use platform::HotkeyManager;

    let mut current = state.current_hotkey.lock().unwrap();

    // 先注销旧的快捷键
    if !current.is_empty() {
        if let Err(e) = state.manager.unregister_hotkey(&current) {
            log::warn!("[Hotkey] 注销旧快捷键 {} 失败: {}", current, e);
        }
        log::info!("[Hotkey] 已注销旧快捷键: {}", current);
    }

    // 如果新快捷键为空，只注销不注册
    if hotkey.trim().is_empty() {
        *current = String::new();
        log::info!("[Hotkey] 快捷键已清空");
        return Ok(());
    }

    // 注册新快捷键，触发时向前端发送 hotkey-triggered 事件
    let app_clone = app.clone();
    let hotkey_clone = hotkey.clone();
    let result = state.manager.register_hotkey(
        &hotkey,
        Box::new(move || {
            log::info!("[Hotkey] 快捷键触发: {}", hotkey_clone);
            let _ = app_clone.emit("hotkey-triggered", ());
        }),
    );

    match result {
        Ok(_) => {
            *current = hotkey.clone();
            log::info!("[Hotkey] 全局快捷键已注册: {}", hotkey);
            Ok(())
        }
        Err(e) => {
            log::error!("[Hotkey] 注册快捷键 {} 失败: {}", hotkey, e);
            Err(format!("{}", e))
        }
    }
}

/// 注销当前全局快捷键
#[tauri::command]
fn unregister_global_hotkey(state: tauri::State<HotkeyState>) -> Result<(), String> {
    use platform::HotkeyManager;

    let mut current = state.current_hotkey.lock().unwrap();
    if current.is_empty() {
        return Ok(());
    }

    if let Err(e) = state.manager.unregister_hotkey(&current) {
        log::warn!("[Hotkey] 注销快捷键 {} 失败: {}", current, e);
        return Err(format!("{}", e));
    }

    log::info!("[Hotkey] 全局快捷键已注销: {}", current);
    *current = String::new();
    Ok(())
}

// ── 托盘构建 ──────────────────────────────────────────────────────────────

/// 构建系统托盘菜单和图标
fn build_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show       = MenuItem::with_id(app, "show",       "显示窗口", true, None::<&str>)?;
    let connect    = MenuItem::with_id(app, "connect",    "连接设备", true, None::<&str>)?;
    let disconnect = MenuItem::with_id(app, "disconnect", "断开连接", true, None::<&str>)?;
    let separator  = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit       = MenuItem::with_id(app, "quit",       "退出",     true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &show,
        &separator,
        &connect,
        &disconnect,
        &separator2,
        &quit,
    ])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "connect"    => { let _ = app.emit("tray-connect", ()); }
            "disconnect" => { let _ = app.emit("tray-disconnect", ()); }
            "quit"       => { std::process::exit(0); }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// 推送初始连接状态到前端
fn emit_initial_status(app: &tauri::AppHandle) {
    let _ = app.emit("connection-status", connection::ConnectionEvent {
        event_type: "disconnected".to_string(),
        connected: false,
        peer_ip: None,
        message: "未连接".to_string(),
        attempt: None,
        guide: None,
        tls_mismatch: false,
        local_tls: false,
        remote_tls: false,
    });
}

/// 检查 macOS 权限（辅助功能 + 输入监控），并通知前端
/// BUG-027：原来只检查辅助功能，用户授权后以为一切就绪，但「开始共享」仍失败。
/// 现在同时检查两个权限，任一未授予都发对应事件。
fn check_macos_permission(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use platform::macos::permission::{
            check_accessibility, check_input_monitoring, request_input_monitoring,
            AccessibilityStatus,
        };

        // ─── 1. 辅助功能权限 ─────────────────────────────────────────────
        let ax = check_accessibility();
        log::info!("[Mac] 辅助功能权限状态: {:?}", ax);

        if ax == AccessibilityStatus::Granted {
            let _ = app.emit("accessibility://permission-granted", ());
        } else {
            log::warn!("[Mac] 辅助功能权限未授权，通知前端引导用户");
            let _ = app.emit("accessibility://permission-denied", ());
            start_accessibility_polling(app.clone());
        }

        // ─── 2. 输入监控权限（BUG-021 / BUG-027） ────────────────────────
        // 主动调用 CGRequestListenEventAccess 把 KVMini 注册到 TCC 数据库的
        // "输入监控"列表，否则用户在系统设置里根本看不到 KVMini 的开关。
        // 注意：这里的返回值受进程内缓存影响，授权后需重启才会变 true。
        let im_after_request = request_input_monitoring();
        let im = check_input_monitoring();
        log::info!(
            "[Mac] 输入监控权限状态: preflight={:?}, request={:?}",
            im,
            im_after_request
        );

        if im == AccessibilityStatus::Granted || im_after_request == AccessibilityStatus::Granted {
            let _ = app.emit("input-monitoring://permission-granted", ());
        } else {
            log::warn!("[Mac] 输入监控权限未授权，通知前端引导用户");
            let _ = app.emit("input-monitoring://permission-denied", ());
            start_input_monitoring_polling(app.clone());
        }
    }
    // 非 macOS 平台无需权限检查
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// 启动后台辅助功能权限轮询（每 2 秒检测一次，授权后发送 granted 事件并停止）
#[cfg(target_os = "macos")]
fn start_accessibility_polling(app: tauri::AppHandle) {
    use platform::macos::permission::{check_accessibility, AccessibilityStatus};
    use std::thread;
    use std::time::Duration;

    thread::spawn(move || {
        log::info!("[Mac] 启动辅助功能权限轮询（每 2 秒）");
        loop {
            thread::sleep(Duration::from_secs(2));
            match check_accessibility() {
                AccessibilityStatus::Granted => {
                    log::info!("[Mac] 轮询检测到辅助功能权限已授权");
                    let _ = app.emit("accessibility://permission-granted", ());
                    break;
                }
                _ => {}
            }
        }
        log::info!("[Mac] 辅助功能权限轮询已停止");
    });
}

/// 启动后台输入监控权限轮询
/// 注意：CGPreflightListenEventAccess 进程内缓存不刷新，此轮询主要用于
/// 在用户"打开设置"之后，Rust 侧无法感知授权 —— 实际生效靠重启 App。
/// 这里保留轮询是为了兜底（某些 macOS 版本/情况下缓存可能刷新）。
#[cfg(target_os = "macos")]
fn start_input_monitoring_polling(app: tauri::AppHandle) {
    use platform::macos::permission::{check_input_monitoring, AccessibilityStatus};
    use std::thread;
    use std::time::Duration;

    thread::spawn(move || {
        log::info!("[Mac] 启动输入监控权限轮询（每 3 秒，最多 5 分钟）");
        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        while std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_secs(3));
            if check_input_monitoring() == AccessibilityStatus::Granted {
                log::info!("[Mac] 轮询检测到输入监控权限已授权");
                let _ = app.emit("input-monitoring://permission-granted", ());
                return;
            }
        }
        log::warn!("[Mac] 输入监控权限轮询超时（进程内缓存未刷新，需重启 App）");
    });
}

// ── 主函数 ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // 初始化日志（持久化 + stderr 双路输出）
    // BUG-027 之前：env_logger 只写 stderr，用户反馈 bug 时无法提供日志
    // 现在：同时写 stderr 和 ~/Library/Logs/KVMini/runtime.log
    //       （Windows: %LOCALAPPDATA%\KVMini\Logs\runtime.log）
    init_persistent_logger();

    // 初始化 tauri-commerce SDK
    let commerce_config = tauri_commerce::CommerceConfig::builder()
        .app_name("KVMini")
        .app_version(env!("CARGO_PKG_VERSION"))
        .bundle_id("com.kvmini.app")
        .product_id("com.kvmini.app.pro")
        .feedback_email("support@kvmini.app")
        .privacy_url("https://mackhan.github.io/KVMini/privacy-policy")
        .app_description("跨平台鼠标键盘共享软件，支持 macOS 和 Windows 之间的设备控制和文件传输")
        .add_permission("辅助功能权限", "用于监听全局快捷键和模拟鼠标键盘输入")
        .add_permission("输入监控权限", "用于实现跨设备鼠标键盘共享")
        .github_repo("mackhan/KVMini")
        .trial_days(7)
        .review_trigger_count(10)
        .build()
        .expect("Commerce SDK 配置构建失败");
    if let Err(e) = tauri_commerce::init(commerce_config) {
        log::error!("[Commerce SDK] 初始化失败: {}", e);
    } else {
        log::info!("[Commerce SDK] SDK 初始化成功");
    }
    // 初始化 i18n 模块
    tauri_commerce::i18n::init_i18n();
    // 初始化 IAP 模块：从 Keychain 恢复购买状态缓存到内存
    // SDK 文档要求在 init() 之后调用（见 README.md 步骤 2）
    #[cfg(feature = "iap")]
    tauri_commerce::iap::init_iap();

    // 调试：捕获 panic 消息
    #[cfg(debug_assertions)]
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            eprintln!("[PANIC HOOK] panic info: {}", info);
            if let Some(location) = info.location() {
                eprintln!("[PANIC HOOK] location: {}:{}:{}", location.file(), location.line(), location.column());
            }
            if let Some(msg) = info.payload().downcast_ref::<&str>() {
                eprintln!("[PANIC HOOK] message: {}", msg);
            } else if let Some(msg) = info.payload().downcast_ref::<String>() {
                eprintln!("[PANIC HOOK] message: {}", msg);
            }
            default_hook(info);
        }));
    }

    tauri::Builder::default()
        // 防多开：第二个实例启动时聚焦已有窗口
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        // 注册 Tauri 2.x 插件
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // 注册所有 Tauri 命令
        .invoke_handler(tauri::generate_handler![
            // 通用
            get_app_version,
            get_local_ip,
            get_network_interfaces,
            get_local_tls_fingerprint,
            regenerate_tls_cert,
            get_platform,
            discover_devices,
            diagnose_network,
            // 显示器与布局
            get_monitors,
            get_layout_config,
            set_layout_config,
            // 连接管理
            connection::start_listening,
            connection::connect_to_peer,
            connection::disconnect_peer,
            connection::get_connection_status,
            connection::get_latency,
            // 配置
            config::get_settings,
            config::save_settings,
            config::reset_settings,
            // 自启动
            get_autostart_status,
            set_autostart,
            // macOS 权限
            check_accessibility_permission,
            request_accessibility_permission,
            open_accessibility_settings,
            check_input_monitoring_permission,
            open_input_monitoring_settings,
            // 诊断日志
            write_startup_log,
            get_log_path,
            set_debug_log,
            // tauri-commerce SDK
            get_trial_status,
            get_locale,
            set_locale,
            get_changelog,
            send_feedback,
            request_review,
            get_privacy_policy_zh,
            get_privacy_policy_en,
            purchase,
            restore_purchase,
            // 全局快捷键
            register_global_hotkey,
            unregister_global_hotkey,
            // 自动更新
            check_for_update,
            install_update,
            // 键鼠共享（Bug 2 / BUG-019）
            sharing::toggle_sharing,
            sharing::get_control_state,
            // v0.1.41: Windows 权限检测
            check_admin_permission,
            request_admin_elevation,
        ])
        // 托管连接管理器状态
        .manage(connection::KvmConnectionManager::new())
        // 托管键鼠共享管理器（Bug 2 / BUG-019）
        .manage(std::sync::Arc::new(sharing::SharingManager::new()))
        // 托管全局快捷键状态
        .manage(HotkeyState::new())
        .setup(|app| {
            // ── BUG-039 修复：启动时强制清除可能残留的 BlockInput 状态 ──────
            // 上次进程被强杀时 Drop 未执行，Win32 BlockInput(1) 不随进程退出还原
            #[cfg(target_os = "windows")]
            platform::windows::input_block::startup_cleanup();
            // Mac：清除可能残留的 CGAssociate(0) 解绑状态（强杀后鼠标↔光标不同步）
            #[cfg(target_os = "macos")]
            platform::macos::cursor::startup_cleanup();

            // 注册 updater 插件（必须在 setup 中注册，desktop only）
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

            // 构建系统托盘
            build_tray(app)?;

            let app_handle = app.handle().clone();

            // 启动连接事件监听循环
            {
                let mgr = app_handle.state::<connection::KvmConnectionManager>();
                mgr.start_event_loop(app_handle.clone());
            }

            // 推送初始连接状态
            emit_initial_status(&app_handle);

            // macOS：检查辅助功能权限，未授权时引导用户
            check_macos_permission(&app_handle);

            // 启动时自动注册已保存的全局快捷键
            {
                let config = config::load_config();
                let saved_hotkey = config.switch_hotkey.clone();
                if !saved_hotkey.is_empty() {
                    let hotkey_state = app_handle.state::<HotkeyState>();
                    let app_for_hotkey = app_handle.clone();
                    match register_global_hotkey(saved_hotkey.clone(), hotkey_state, app_for_hotkey) {
                        Ok(_) => log::info!("[Hotkey] 启动时自动注册快捷键: {}", saved_hotkey),
                        Err(e) => log::warn!("[Hotkey] 启动时注册快捷键失败: {}", e),
                    }
                }
            }

            log::info!("KVMini v{} 已启动（{}）",
                env!("CARGO_PKG_VERSION"),
                platform::platform_name()
            );

            // 启动后台自动更新检查（启动后 10 秒首次检查，之后每 4 小时一次）
            start_background_update_check(app.handle().clone());

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .on_window_event(|_window, event| {
            // BUG-039: 窗口销毁时强制清除残留状态，防止正常退出后下次启动时键鼠被锁
            if let tauri::WindowEvent::Destroyed = event {
                #[cfg(target_os = "windows")]
                platform::windows::input_block::startup_cleanup();
                #[cfg(target_os = "macos")]
                platform::macos::cursor::startup_cleanup();
            }
        })
        .run(tauri::generate_context!())
        .expect("KVMini 启动失败");
}
