/// 输入控制子模块
///
/// 封装 Mac 端的输入事件执行（CGEvent API）和输入采集逻辑。
/// 提供统一的 InputController 接口，隐藏平台差异。

use crate::protocol::KvmFrame;

/// 输入控制器
///
/// 封装输入事件的执行（被控端）和采集（主控端）。
pub struct InputController;

impl InputController {
    /// 执行来自主控端的输入帧（被控端使用）
    pub fn execute(frame: &KvmFrame) {
        crate::input_executor::execute_frame(frame);
    }

    /// 检查辅助功能权限（macOS 必须）
    pub fn check_permission() -> bool {
        crate::input_executor::check_accessibility_permission()
    }

    /// 打印权限引导信息
    pub fn print_permission_guide() {
        crate::input_executor::print_accessibility_guide();
    }
}
