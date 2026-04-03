use std::net::UdpSocket;
use crate::protocol::{DISCOVER_MAGIC, DiscoverResponse};

/// 获取本机主机名
pub fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "kvm-mac-agent".to_string())
}

/// 获取 macOS 版本字符串，如 "macOS 14.2.1"
pub fn get_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!("macOS {}", output)
    }
    #[cfg(not(target_os = "macos"))]
    {
        "macOS (unknown)".to_string()
    }
}

/// 启动 UDP 广播监听，阻塞运行
/// 收到 KVM_DISCOVER 后回复 KVM_RESPONSE JSON
pub fn run_udp_listener(hostname: String, os_version: String) -> Result<(), String> {
    let socket = UdpSocket::bind("0.0.0.0:8888")
        .map_err(|e| format!("UDP 绑定端口 8888 失败: {}", e))?;

    println!("[UDP] 监听端口 8888，等待发现请求...");

    let mut buf = [0u8; 256];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                let data = &buf[..size];
                if data == DISCOVER_MAGIC {
                    println!("[UDP] 收到来自 {} 的发现请求", src);
                    let resp = DiscoverResponse::new(hostname.clone(), os_version.clone());
                    match serde_json::to_string(&resp) {
                        Ok(json) => {
                            if let Err(e) = socket.send_to(json.as_bytes(), src) {
                                eprintln!("[UDP] 回复失败: {}", e);
                            } else {
                                println!("[UDP] 已回复 KVM_RESPONSE 给 {}", src);
                            }
                        }
                        Err(e) => eprintln!("[UDP] 序列化响应失败: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("[UDP] 接收错误: {}", e);
            }
        }
    }
}
