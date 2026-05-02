//! 平台适配层
//!
//! 重新导出 kvm-core 定义的所有平台 trait，
//! 并通过条件编译选择当前平台的具体实现。
//! 所有平台特定代码只在此目录下，通过 #[cfg(target_os)] 隔离。

// 平台层实现已接入 sharing 主流程；保留 #[allow(dead_code)] 在仅部分平台启用的具体函数上

// 重新导出 kvm-core 的平台 trait（避免外部模块直接依赖 kvm-core）
#[allow(unused_imports)]
pub use kvm_core::platform::{
    PlatformAdapter,
    ScreenProvider,
    InputCapture,
    InputExecutor,
    InputBlocker,
    HotkeyManager,
    ClipboardProvider,
    CursorController,
    MonitorInfo,
    VirtualDesktop,
    RawInputEvent,
    MouseButton,
};

// ── 平台特定实现 ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// ── 开机自启动（平台特定） ──────────────────────────────────────────────────

pub mod autostart;

// ── 屏幕枚举（跨平台） ──────────────────────────────────────────────────────

pub mod screen {
    use kvm_core::error::KvmResult;
    use kvm_core::platform::{MonitorInfo, ScreenProvider};

    /// 获取当前平台的显示器列表（跨平台入口）
    pub fn get_monitors_impl() -> KvmResult<Vec<MonitorInfo>> {
        #[cfg(target_os = "macos")]
        {
            let provider = super::macos::screen::MacScreenProvider::new();
            provider.enumerate_monitors()
        }
        #[cfg(target_os = "windows")]
        {
            let provider = super::windows::screen::WinScreenProvider::new();
            provider.enumerate_monitors()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Ok(vec![MonitorInfo {
                id: 0, width: 1920, height: 1080, dpi_scale: 1.0,
                offset_x: 0, offset_y: 0, is_primary: true,
            }])
        }
    }
}

// ── 获取当前平台适配器 ──────────────────────────────────────────────────────

/// 创建当前平台的适配器实例
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn create_platform_adapter() -> windows::WinPlatformAdapter {
    windows::WinPlatformAdapter::new()
}

/// 创建当前平台的适配器实例
#[cfg(target_os = "macos")]
pub fn create_platform_adapter() -> macos::MacPlatformAdapter {
    macos::MacPlatformAdapter::new()
}

/// 获取当前平台名称
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    { "Windows" }
    #[cfg(target_os = "macos")]
    { "macOS" }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { "Unknown" }
}

/// 获取本机平台握手信息（用于连接时的握手协议）
/// 自动从配置文件读取 TLS 状态，携带到握手信息中供对端比较
pub fn local_handshake_info() -> kvm_core::network::HandshakeInfo {
    #[cfg(target_os = "windows")]
    let platform = kvm_core::protocol::Platform::Windows;
    #[cfg(target_os = "macos")]
    let platform = kvm_core::protocol::Platform::MacOS;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let platform = kvm_core::protocol::Platform::Unknown;

    // 从配置中读取 TLS 状态，默认开启
    let tls_enabled = crate::config::load_config().enable_tls;

    kvm_core::network::HandshakeInfo::new_with_tls(
        platform,
        kvm_core::platform::VirtualDesktop::from_monitors(vec![]),
        tls_enabled,
    )
}
