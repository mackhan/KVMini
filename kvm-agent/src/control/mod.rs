/// kvm-agent 控制模块
///
/// 负责键鼠跨电脑操作的控制逻辑：
/// - `edge`：屏幕边缘检测子模块，支持上下左右四个方向，支持多显示器
/// - `hotkey`：快捷键控制子模块（Mac 端）
/// - `input`：输入事件采集与执行（Mac 端输入注入）

pub mod edge;
pub mod hotkey;
pub mod input;

pub use edge::{EdgeController, EdgeConfig, EdgeDirection, EdgeDetectResult};
pub use hotkey::{HotkeyController, HotkeyAction};
pub use input::InputController;
