/// src-tauri Windows 端控制模块
///
/// 负责键鼠跨电脑操作的控制逻辑：
/// - `edge`：屏幕边缘检测子模块，支持上下左右四个方向，支持多显示器
/// - `hotkey`：全局快捷键控制（Windows 端，通过 Tauri GlobalShortcutManager）
/// - `capture`：键鼠事件采集（Windows 端主控端使用）

pub mod edge;
pub mod hotkey;
pub mod capture;

pub use edge::{WinEdgeController, EdgeConfig, EdgeDirection, EdgeDetectResult};
pub use hotkey::WinHotkeyManager;
pub use capture::CaptureController;
