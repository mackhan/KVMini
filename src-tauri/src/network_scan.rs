//! 多接口局域网扫描（VPN/防火墙鲁棒）
//!
//! 解决 Bug 3 (BUG-020)：在 macOS 防火墙开启 + VPN（utun*）环境下，
//! 原来的 `8.8.8.8 connect` 路径会拿到 VPN 出口 IP，子网扫描扫错网段；
//! `discover_devices` 单接口广播也会被路由到 VPN 出口。
//!
//! 本模块通过枚举所有物理 IPv4 接口，过滤 VPN/loopback，对每张物理网卡
//! 分别 `bind+广播+扫描`，结果按 IP 去重合并。

use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use kvm_core::network::{DiscoveredDevice, DISCOVERY_MAGIC};
use serde::Deserialize;
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// UDP 发现等待时间（与 kvm-core 内置一致，避免引入额外延迟）
const DISCOVER_TIMEOUT_SECS: u64 = 2;

/// 局域网网卡信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanIface {
    pub name: String,
    pub ipv4: Ipv4Addr,
    pub broadcast: Ipv4Addr,
}

/// 判断网卡名是否属于 VPN/虚拟隧道（按平台前缀过滤）
///
/// macOS：utun*/ppp*/tun*/tap*
/// Windows：名称含 PPP/TAP/WireGuard/OpenVPN/Tailscale
/// 跨平台共同关键字：vpn / wg / wireguard / tailscale
pub fn is_vpn_iface(name: &str) -> bool {
    let n = name.to_lowercase();
    // 跨平台通用 VPN 关键字
    if n.contains("vpn")
        || n.contains("wireguard")
        || n.contains("tailscale")
        || n.contains("openvpn")
    {
        return true;
    }
    // 平台特定前缀
    #[cfg(target_os = "macos")]
    {
        // macOS: utun0/utun1/.../ppp0/tun0/tap0
        if n.starts_with("utun")
            || n.starts_with("ppp")
            || n.starts_with("tun")
            || n.starts_with("tap")
        {
            return true;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if n.contains("ppp") || n.contains("tap") || n.contains("wg") {
            return true;
        }
    }
    false
}

/// 计算 IPv4 + netmask 的有向广播地址
pub fn compute_broadcast(ip: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Addr {
    let ip = u32::from(ip);
    let mask = u32::from(netmask);
    let bcast = ip | !mask;
    Ipv4Addr::from(bcast)
}

/// 枚举所有可用的物理 LAN IPv4 网卡（已过滤 loopback、VPN、APIPA、0.0.0.0）
pub fn list_lan_ipv4_ifaces() -> Vec<LanIface> {
    let raw = match if_addrs::get_if_addrs() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[Scan] if_addrs 枚举失败: {}", e);
            return vec![];
        }
    };

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for iface in raw {
        if iface.is_loopback() {
            continue;
        }
        let ip = match iface.ip() {
            std::net::IpAddr::V4(v) => v,
            _ => continue,
        };
        if ip.is_unspecified() || ip.octets()[0] == 0 {
            continue;
        }
        if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
            continue; // APIPA
        }
        if is_vpn_iface(&iface.name) {
            log::info!("[Scan] 跳过 VPN 网卡: {} ({})", iface.name, ip);
            continue;
        }

        // 取 netmask 计算广播地址
        let netmask = match &iface.addr {
            if_addrs::IfAddr::V4(v4) => v4.netmask,
            _ => continue,
        };
        let broadcast = compute_broadcast(ip, netmask);

        let key = (iface.name.clone(), ip);
        if !seen.insert(key) {
            continue;
        }

        out.push(LanIface {
            name: iface.name.clone(),
            ipv4: ip,
            broadcast,
        });
    }
    log::info!(
        "[Scan] 可用物理 LAN 网卡: {:?}",
        out.iter().map(|i| format!("{}={}->{}", i.name, i.ipv4, i.broadcast)).collect::<Vec<_>>()
    );
    out
}

/// 用于解析对端 UDP 应答的最小结构（与 kvm-core 内 DiscoveryResponse 保持字段一致）
#[derive(Deserialize)]
struct UdpDiscoveryResponse {
    magic: String,
    hostname: String,
    os: String,
    tcp_port: u16,
}

/// 多接口 UDP 广播发现：对每张物理 LAN 网卡 bind+定向广播+255.255.255.255 兜底
///
/// 与 `kvm_core::network::discover_devices` 不同：
/// - 后者只 `bind 0.0.0.0:0` → 在多接口/VPN 环境下广播会被路由到 VPN 出口；
/// - 本函数显式 `bind <iface_ip>:0` + `send_to <iface_broadcast>:port`，确保广播
///   走每张物理网卡，规避 VPN 干扰。
pub async fn discover_devices_multi(udp_port: u16) -> Vec<DiscoveredDevice> {
    let ifaces = list_lan_ipv4_ifaces();
    if ifaces.is_empty() {
        log::warn!("[Scan] 没有可用的物理 LAN 网卡，跳过多接口广播");
        return Vec::new();
    }

    // 启动一组 task，每张卡一个 socket
    let mut handles = Vec::with_capacity(ifaces.len());
    for iface in ifaces {
        handles.push(tokio::spawn(discover_on_iface(iface, udp_port)));
    }

    // 合并结果，按 ip 去重（同一台机器可能从多张卡收到响应）
    let mut dedup: HashMap<String, DiscoveredDevice> = HashMap::new();
    for h in handles {
        if let Ok(list) = h.await {
            for d in list {
                dedup.entry(d.ip.clone()).or_insert(d);
            }
        }
    }

    let result: Vec<_> = dedup.into_values().collect();
    log::info!("[Scan] 多接口广播共发现 {} 个设备", result.len());
    result
}

async fn discover_on_iface(iface: LanIface, udp_port: u16) -> Vec<DiscoveredDevice> {
    let bind_addr = format!("{}:0", iface.ipv4);
    let socket = match UdpSocket::bind(&bind_addr).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[Scan] 接口 {} bind {} 失败: {}", iface.name, bind_addr, e);
            return Vec::new();
        }
    };
    if let Err(e) = socket.set_broadcast(true) {
        log::warn!("[Scan] 接口 {} set_broadcast 失败: {}", iface.name, e);
        return Vec::new();
    }

    // 定向广播（按网段广播） + 全局兜底 255.255.255.255
    let directed = format!("{}:{}", iface.broadcast, udp_port);
    let limited = format!("255.255.255.255:{}", udp_port);
    if let Err(e) = socket.send_to(DISCOVERY_MAGIC, &directed).await {
        log::warn!("[Scan] 接口 {} 定向广播 {} 失败: {}", iface.name, directed, e);
    } else {
        log::info!("[Scan] 接口 {} 已发出定向广播 {}", iface.name, directed);
    }
    if let Err(e) = socket.send_to(DISCOVERY_MAGIC, &limited).await {
        log::debug!("[Scan] 接口 {} 全局广播 {} 失败: {}", iface.name, limited, e);
    }

    // 收集 DISCOVER_TIMEOUT_SECS 秒内的响应
    let deadline = Instant::now() + Duration::from_secs(DISCOVER_TIMEOUT_SECS);
    let mut buf = vec![0u8; 2048];
    let mut devices = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((size, src))) => {
                let text = match std::str::from_utf8(&buf[..size]) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let resp: UdpDiscoveryResponse = match serde_json::from_str(text) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if resp.magic != "KVM_RESPONSE" {
                    continue;
                }
                devices.push(DiscoveredDevice {
                    name: resp.hostname,
                    os: resp.os,
                    ip: src.ip().to_string(),
                    tcp_port: resp.tcp_port,
                });
            }
            _ => break,
        }
    }
    log::info!("[Scan] 接口 {} 收到 {} 个响应", iface.name, devices.len());
    devices
}

/// 多接口子网 TCP 扫描：对每张物理 LAN 网卡的 /24 网段并发扫描，结果合并去重
pub async fn scan_subnet_multi(tcp_port: u16) -> Vec<DiscoveredDevice> {
    let ifaces = list_lan_ipv4_ifaces();
    if ifaces.is_empty() {
        log::warn!("[Scan] 没有可用 LAN 网卡，跳过 subnet 扫描");
        return Vec::new();
    }

    let mut handles = Vec::with_capacity(ifaces.len());
    for iface in ifaces {
        let local_ip = iface.ipv4.to_string();
        handles.push(tokio::spawn(async move {
            log::info!("[Scan] 子网扫描 (接口={}, ip={})", iface.name, local_ip);
            kvm_core::network::scan_subnet(&local_ip, tcp_port)
                .await
                .unwrap_or_else(|e| {
                    log::warn!("[Scan] 接口 {} 子网扫描失败: {}", iface.name, e);
                    Vec::new()
                })
        }));
    }

    let mut dedup: HashMap<String, DiscoveredDevice> = HashMap::new();
    for h in handles {
        if let Ok(list) = h.await {
            for d in list {
                dedup.entry(d.ip.clone()).or_insert(d);
            }
        }
    }
    let result: Vec<_> = dedup.into_values().collect();
    log::info!("[Scan] 多接口子网扫描共发现 {} 个设备", result.len());
    result
}

/// 是否检测到 VPN 网卡（用于诊断提示）
pub fn detect_vpn_ifaces() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback() && i.ip().is_ipv4())
        .filter(|i| is_vpn_iface(&i.name))
        .map(|i| i.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_broadcast_class_c() {
        let ip: Ipv4Addr = "192.168.1.50".parse().unwrap();
        let mask: Ipv4Addr = "255.255.255.0".parse().unwrap();
        assert_eq!(compute_broadcast(ip, mask), "192.168.1.255".parse::<Ipv4Addr>().unwrap());
    }

    #[test]
    fn test_compute_broadcast_class_b() {
        let ip: Ipv4Addr = "172.16.5.10".parse().unwrap();
        let mask: Ipv4Addr = "255.255.0.0".parse().unwrap();
        assert_eq!(compute_broadcast(ip, mask), "172.16.255.255".parse::<Ipv4Addr>().unwrap());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_is_vpn_iface_macos() {
        assert!(is_vpn_iface("utun0"));
        assert!(is_vpn_iface("utun6"));
        assert!(is_vpn_iface("ppp0"));
        assert!(is_vpn_iface("tap0"));
        assert!(is_vpn_iface("tun5"));
        assert!(is_vpn_iface("WireGuard-1"));
        assert!(is_vpn_iface("Tailscale"));
        assert!(!is_vpn_iface("en0"));
        assert!(!is_vpn_iface("en1"));
        assert!(!is_vpn_iface("eth0"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_is_vpn_iface_windows() {
        assert!(is_vpn_iface("OpenVPN TAP"));
        assert!(is_vpn_iface("WireGuard Tunnel"));
        assert!(is_vpn_iface("Tailscale"));
        assert!(is_vpn_iface("PPP Adapter"));
        assert!(!is_vpn_iface("Ethernet"));
        assert!(!is_vpn_iface("Wi-Fi"));
    }

    #[test]
    fn test_is_vpn_iface_cross_platform_keywords() {
        assert!(is_vpn_iface("Corporate-VPN"));
        assert!(is_vpn_iface("wireguard0"));
        assert!(is_vpn_iface("Tailscale-utun"));
    }

    #[test]
    fn test_list_lan_ipv4_ifaces_returns_data() {
        // 实测：在 CI/本地环境下应至少有 1 张物理网卡（除非完全断网）
        let ifaces = list_lan_ipv4_ifaces();
        // 不强制断言 ≥ 1（CI 沙箱可能无网络），但确保不 panic 且过滤掉了 VPN
        for i in &ifaces {
            assert!(!is_vpn_iface(&i.name), "结果不应包含 VPN 网卡: {}", i.name);
            assert!(!i.ipv4.is_loopback(), "结果不应包含 loopback");
        }
    }
}
