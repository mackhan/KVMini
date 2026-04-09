// lib.rs：暴露模块供集成测试使用
// 注意：main.rs 中的 #[macro_use] mod macros 需要在此重新声明

#[macro_use]
pub mod macros;
pub mod config;
pub mod connection;
pub mod platform;
