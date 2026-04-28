#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

// ── 模块声明 ──────────────────────────────────────────────────────────────
#[macro_use]
mod macros;
mod config;
mod connection;
mod platform;

// ── 通用 Tauri 命令 ────────────────────────────────────────────────────────

/// 获取应用版本号（从 Cargo.toml 读取，两端保持一致）
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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

/// 生成自签名 TLS 证书，返回 DER 格式的证书字节
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), String> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "KVMini");
    dn.push(DnType::OrganizationName, "KVMini");
    params.distinguished_name = dn;

    let key_pair = KeyPair::generate().map_err(|e| format!("生成密钥对失败: {}", e))?;
    let cert = params.self_signed(&key_pair).map_err(|e| format!("生成证书失败: {}", e))?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    Ok((cert_der, key_der))
}

/// 计算证书 DER 字节的 SHA-256 指纹（格式：XX:XX:XX:...）
fn compute_cert_fingerprint(cert_der: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(cert_der);
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// 加载或生成本机 TLS 证书，返回 SHA-256 指纹
#[tauri::command]
fn get_local_tls_fingerprint() -> Result<String, String> {
    let cert_dir = get_cert_dir();
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");

    // 尝试加载已有证书
    if cert_path.exists() && key_path.exists() {
        match std::fs::read(&cert_path) {
            Ok(cert_der) => {
                log::info!("[TLS] 加载已有证书: {:?}", cert_path);
                return Ok(compute_cert_fingerprint(&cert_der));
            }
            Err(e) => {
                // 证书文件损坏，删除并重新生成
                log::warn!("[TLS] 证书文件损坏，重新生成: {}", e);
                let _ = std::fs::remove_file(&cert_path);
                let _ = std::fs::remove_file(&key_path);
            }
        }
    }

    // 生成新证书
    log::info!("[TLS] 生成新的自签名证书");
    std::fs::create_dir_all(&cert_dir)
        .map_err(|e| format!("创建证书目录失败: {}", e))?;

    let (cert_der, key_der) = generate_self_signed_cert()?;

    std::fs::write(&cert_path, &cert_der)
        .map_err(|e| format!("保存证书失败: {}", e))?;
    std::fs::write(&key_path, &key_der)
        .map_err(|e| format!("保存密钥失败: {}", e))?;

    log::info!("[TLS] 新证书已生成并保存: {:?}", cert_path);
    Ok(compute_cert_fingerprint(&cert_der))
}

/// 重新生成 TLS 证书（删除旧证书，生成新证书），返回新指纹
#[tauri::command]
fn regenerate_tls_cert() -> Result<String, String> {
    let cert_dir = get_cert_dir();
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");

    // 删除旧证书
    if cert_path.exists() { let _ = std::fs::remove_file(&cert_path); }
    if key_path.exists() { let _ = std::fs::remove_file(&key_path); }

    log::info!("[TLS] 重新生成证书");
    std::fs::create_dir_all(&cert_dir)
        .map_err(|e| format!("创建证书目录失败: {}", e))?;

    let (cert_der, key_der) = generate_self_signed_cert()?;

    std::fs::write(&cert_path, &cert_der)
        .map_err(|e| format!("保存证书失败: {}", e))?;
    std::fs::write(&key_path, &key_der)
        .map_err(|e| format!("保存密钥失败: {}", e))?;

    log::info!("[TLS] 证书已重新生成: {:?}", cert_path);
    Ok(compute_cert_fingerprint(&cert_der))
}

/// 扫描局域网内的 KVMini 设备
///
/// 策略：UDP 广播优先（5秒），无结果时自动降级为子网 TCP 扫描（/24 并发探测）。
/// 降级扫描对 VPN 环境和防火墙屏蔽 UDP 广播的场景有效。
#[tauri::command]
async fn discover_devices() -> Result<Vec<kvm_core::network::DiscoveredDevice>, String> {
    log::info!("[Discover] 开始局域网设备扫描（UDP 广播 255.255.255.255:{}）...",
        kvm_core::network::DEFAULT_UDP_PORT);

    // 第一阶段：UDP 广播
    let udp_result = kvm_core::network::discover_devices(kvm_core::network::DEFAULT_UDP_PORT).await;
    match &udp_result {
        Ok(devices) if !devices.is_empty() => {
            log::info!("[Discover] UDP 广播发现 {} 个设备", devices.len());
            return Ok(devices.clone());
        }
        Ok(_) => {
            log::warn!("[Discover] UDP 广播无结果，尝试子网 TCP 扫描降级...");
        }
        Err(e) => {
            log::warn!("[Discover] UDP 广播失败: {}，尝试子网 TCP 扫描降级...", e);
        }
    }

    // 第二阶段：子网 TCP 扫描（降级）
    // 获取本机非 loopback、非 VPN 的第一个 IPv4 地址
    let local_ip = get_local_lan_ip();
    match local_ip {
        Some(ip) => {
            log::info!("[Discover] 使用本机 IP {} 进行子网扫描", ip);
            match kvm_core::network::scan_subnet(&ip, kvm_core::network::DEFAULT_TCP_PORT).await {
                Ok(devices) => {
                    if devices.is_empty() {
                        log::warn!("[Discover] 子网扫描也未发现设备，请检查对端是否运行 KVMini");
                    } else {
                        log::info!("[Discover] 子网扫描发现 {} 个设备", devices.len());
                    }
                    Ok(devices)
                }
                Err(e) => {
                    log::error!("[Discover] 子网扫描失败: {}", e);
                    Ok(vec![]) // 返回空列表而非错误，让前端触发诊断
                }
            }
        }
        None => {
            log::error!("[Discover] 无法获取本机局域网 IP，跳过子网扫描");
            Ok(vec![])
        }
    }
}

/// 获取本机局域网 IP（排除 loopback、VPN utun* 接口，优先返回 192.168/10./172.16-31 网段）
fn get_local_lan_ip() -> Option<String> {
    // 通过连接外部地址来获取本机出口 IP（不实际发送数据）
    use std::net::UdpSocket as StdUdpSocket;
    // 尝试连接公共 DNS，获取路由出口 IP
    if let Ok(socket) = StdUdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip = addr.ip().to_string();
                if ip != "0.0.0.0" && !ip.starts_with("127.") {
                    log::info!("[Discover] 检测到本机出口 IP: {}", ip);
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
            format!("UDP 端口 {} 可用", udp_port)
        } else {
            format!("UDP 端口 {} 被占用或无法绑定", udp_port)
        },
    });

    // 2. 检查网络接口是否有活跃的 IP
    let interfaces = if_addrs::get_if_addrs().unwrap_or_default();
    let active_ipv4: Vec<_> = interfaces.iter()
        .filter(|iface| !iface.is_loopback() && iface.addr.ip().is_ipv4())
        .collect();
    let has_network = !active_ipv4.is_empty();
    results.push(DiagnosticItem {
        check: "network_interface".to_string(),
        ok: has_network,
        message: if has_network {
            format!("检测到 {} 个活跃网络接口", active_ipv4.len())
        } else {
            "未检测到活跃的 IPv4 网络接口".to_string()
        },
    });

    // 3. 检查是否存在 VPN 网卡（常见 VPN 接口名称）
    let vpn_keywords = ["tun", "tap", "utun", "ppp", "vpn", "wg", "tailscale", "wireguard"];
    let vpn_ifaces: Vec<_> = interfaces.iter()
        .filter(|iface| {
            let name = iface.name.to_lowercase();
            vpn_keywords.iter().any(|kw| name.contains(kw))
        })
        .map(|iface| iface.name.clone())
        .collect();
    let has_vpn = !vpn_ifaces.is_empty();
    results.push(DiagnosticItem {
        check: "vpn".to_string(),
        ok: !has_vpn, // VPN 存在时标记为 "不正常"（可能干扰）
        message: if has_vpn {
            format!("检测到 VPN 网卡 ({})，可能干扰 UDP 广播", vpn_ifaces.join(", "))
        } else {
            "未检测到 VPN 网卡".to_string()
        },
    });

    // 4. 平台特定：macOS 检查防火墙
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
                        "macOS 防火墙已开启，可能阻止 UDP 广播".to_string()
                    } else {
                        "macOS 防火墙未开启".to_string()
                    },
                });
            }
            Err(_) => {
                results.push(DiagnosticItem {
                    check: "firewall".to_string(),
                    ok: true,
                    message: "无法检测防火墙状态".to_string(),
                });
            }
        }
    }

    // 4. 平台特定：Windows 检查防火墙
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
                        "Windows 防火墙已开启，请确保 KVMini 已被允许通过".to_string()
                    } else {
                        "Windows 防火墙未开启".to_string()
                    },
                });
            }
            Err(_) => {
                results.push(DiagnosticItem {
                    check: "firewall".to_string(),
                    ok: true,
                    message: "无法检测防火墙状态".to_string(),
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
        return status.to_string();
    }
    #[cfg(not(target_os = "macos"))]
    "N/A".to_string()
}

/// 请求 macOS 辅助功能权限（弹出系统对话框）
#[tauri::command]
fn request_accessibility_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { platform::macos::permission::request_accessibility().map_err(|e| e.to_string())?; }
    Ok(())
}

/// 打开 macOS 辅助功能设置页面
#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { platform::macos::permission::open_accessibility_settings().map_err(|e| e.to_string())?; }
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
fn write_startup_log(content: String) -> Result<(), String> {
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

/// 获取更新日志（KVMini 实际版本历史）
#[tauri::command]
fn get_changelog() -> Vec<tauri_commerce::changelog::ChangelogEntry> {
    vec![
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

/// 检查 macOS 辅助功能权限，并通知前端，权限不足时启动后台轮询
fn check_macos_permission(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use platform::macos::permission::{check_accessibility, AccessibilityStatus};
        let status = check_accessibility();
        log::info!("[Mac] 辅助功能权限状态: {:?}", status);

        if status == AccessibilityStatus::Granted {
            // 已授权：通知前端隐藏遮罩
            let _ = app.emit("accessibility://permission-granted", ());
            log::info!("[Mac] 辅助功能权限已授权，通知前端");
        } else {
            // 未授权：通知前端显示引导遮罩，并启动后台轮询
            log::warn!("[Mac] 辅助功能权限未授权，通知前端引导用户");
            let _ = app.emit("accessibility://permission-denied", ());
            start_accessibility_polling(app.clone());
        }
    }
    // 非 macOS 平台无需权限检查
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// 启动后台权限轮询（每 2 秒检测一次，授权后发送 granted 事件并停止）
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
                    log::info!("[Mac] 轮询检测到权限已授权，通知前端");
                    let _ = app.emit("accessibility://permission-granted", ());
                    break;
                }
                _ => {
                    // 继续轮询
                }
            }
        }
        log::info!("[Mac] 辅助功能权限轮询已停止");
    });
}

// ── 主函数 ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 初始化 tauri-commerce SDK
    let commerce_config = tauri_commerce::CommerceConfig::builder()
        .app_name("KVMini")
        .app_version(env!("CARGO_PKG_VERSION"))
        .bundle_id("com.kvmini.app")
        .product_id("com.kvmini.app.pro")
        .feedback_email("support@kvmini.app")
        .privacy_url("https://kvmini.app/privacy")
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
        // 注册 Tauri 2.x 插件
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            // 诊断日志
            write_startup_log,
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
        ])
        // 托管连接管理器状态
        .manage(connection::KvmConnectionManager::new())
        // 托管全局快捷键状态
        .manage(HotkeyState::new())
        .setup(|app| {
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

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("KVMini 启动失败");
}
