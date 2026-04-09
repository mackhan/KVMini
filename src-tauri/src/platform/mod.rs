//! 平台适配层
//!
//! 重新导出 kvm-core 定义的所有平台 trait，
//! 并通过条件编译选择当前平台的具体实现。
//! 所有平台特定代码只在此目录下，通过 #[cfg(target_os)] 隔离。

// 平台层实现已完成，但尚未接入主流程，暂时允许 dead_code
#![allow(dead_code)]

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
