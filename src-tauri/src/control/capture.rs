/// Windows 端键鼠事件采集子模块
///
/// 主控端模式下，采集 Windows 键鼠事件并通过连接发送给 Mac 端。
/// 与 edge.rs 集成，检测屏幕边缘触发控制权切换。

use crate::protocol::KvmFrame;

/// 采集控制器
///
/// 管理键鼠事件的采集状态（是否采集、是否发送给远端）。
pub struct CaptureController {
    /// 是否正在采集
    capturing: bool,
    /// 是否将事件发送给远端（主控端模式）
    forwarding: bool,
}

impl CaptureController {
    pub fn new() -> Self {
        Self { capturing: false, forwarding: false }
    }

    /// 开始采集
    pub fn start(&mut self) {
        self.capturing = true;
        println!("[采集] 开始采集键鼠事件");
    }

    /// 停止采集
    pub fn stop(&mut self) {
        self.capturing = false;
        self.forwarding = false;
        println!("[采集] 停止采集");
    }

    /// 开始转发（切换到控制远端）
    pub fn start_forwarding(&mut self) {
        self.forwarding = true;
        println!("[采集] 开始转发事件到远端");
    }

    /// 停止转发（切换回控制本机）
    pub fn stop_forwarding(&mut self) {
        self.forwarding = false;
        println!("[采集] 停止转发，恢复本机控制");
    }

    /// 是否正在采集
    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    /// 是否正在转发
    pub fn is_forwarding(&self) -> bool {
        self.forwarding
    }
}

impl Default for CaptureController {
    fn default() -> Self {
        Self::new()
    }
}
