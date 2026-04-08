//! # kvm-core — KVMini 公共核心库
//!
//! 纯逻辑 + 网络，零平台依赖。
//! 不包含任何 `#[cfg(target_os)]` 条件编译，不调用任何系统 API。
//!
//! ## 模块概览
//!
//! - [`protocol`] — 事件帧定义、序列化、键码映射
//! - [`state`] — 控制权状态机
//! - [`network`] — TCP 连接、握手、心跳、重连、UDP 发现
//! - [`edge_logic`] — 边缘触发判定（纯坐标计算）
//! - [`cursor_mapping`] — 光标跨屏坐标映射
//! - [`clipboard`] — 剪贴板共享协议逻辑
//! - [`auth`] — 密码验证、TLS 配置
//! - [`config`] — 配置数据结构
//! - [`error`] — 统一错误类型
//! - [`log`] — 日志接口定义
//! - [`platform`] — 平台适配 trait 定义

pub mod protocol;
pub mod state;
pub mod network;
pub mod edge_logic;
pub mod cursor_mapping;
pub mod clipboard;
pub mod auth;
pub mod config;
pub mod error;
pub mod logging;
pub mod platform;
