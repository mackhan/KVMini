/// 快捷键控制子模块（Mac 端）
///
/// 监听全局快捷键，触发连接控制操作。
/// macOS 使用 CGEventTap 实现全局快捷键监听。

/// 快捷键动作
#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyAction {
    /// 断开当前连接
    Disconnect,
    /// 自定义动作（字符串标识）
    Custom(String),
}

/// 快捷键配置
#[derive(Debug, Clone)]
pub struct HotkeyConfig {
    /// 快捷键描述（如 "Ctrl+Alt+D"）
    pub shortcut: String,
    /// 触发的动作
    pub action: HotkeyAction,
}

/// 快捷键控制器
pub struct HotkeyController {
    configs: Vec<HotkeyConfig>,
    callback: Box<dyn Fn(HotkeyAction) + Send + Sync>,
}

impl HotkeyController {
    /// 创建快捷键控制器
    pub fn new(callback: impl Fn(HotkeyAction) + Send + Sync + 'static) -> Self {
        Self {
            configs: Vec::new(),
            callback: Box::new(callback),
        }
    }

    /// 注册快捷键
    pub fn register(&mut self, config: HotkeyConfig) {
        println!("[快捷键] 注册: {} → {:?}", config.shortcut, config.action);
        self.configs.push(config);
    }

    /// 注册默认快捷键（Ctrl+Alt+D 断开连接）
    pub fn register_defaults(&mut self) {
        self.register(HotkeyConfig {
            shortcut: "Ctrl+Alt+D".to_string(),
            action: HotkeyAction::Disconnect,
        });
    }

    /// 模拟触发快捷键（用于测试或托盘菜单触发）
    pub fn trigger(&self, action: HotkeyAction) {
        (self.callback)(action);
    }

    /// 获取已注册的快捷键列表
    pub fn registered(&self) -> &[HotkeyConfig] {
        &self.configs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_hotkey_register_and_trigger() {
        let triggered = Arc::new(Mutex::new(None::<HotkeyAction>));
        let triggered_clone = Arc::clone(&triggered);

        let mut ctrl = HotkeyController::new(move |action| {
            *triggered_clone.lock().unwrap() = Some(action);
        });

        ctrl.register_defaults();
        assert_eq!(ctrl.registered().len(), 1);

        ctrl.trigger(HotkeyAction::Disconnect);
        assert_eq!(*triggered.lock().unwrap(), Some(HotkeyAction::Disconnect));
    }
}
