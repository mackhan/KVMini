//! Mac 系统托盘 UI
//!
//! 通过 NSStatusBar 系统托盘菜单提供 UI 交互。
//! 不包含任何"主控端/被控端"角色相关内容。
//! 连接状态：未连接 / 已连接·各自独立 / 正在控制对方 / 对方正在控制本机

use kvm_core::state::ControlState;
use kvm_core::network::ConnectionState;

// ── 状态数据结构 ────────────────────────────────────────────────────────────

/// 托盘状态信息
#[derive(Debug, Clone)]
pub struct TrayStatus {
    pub connection: ConnectionState,
    pub control: ControlState,
    pub peer_name: Option<String>,
    pub accessibility_granted: bool,
}

impl TrayStatus {
    pub fn new() -> Self {
        Self {
            connection: ConnectionState::Disconnected,
            control: ControlState::Idle,
            peer_name: None,
            accessibility_granted: false,
        }
    }

    /// 获取托盘提示文本（不含角色信息）
    pub fn tooltip(&self) -> String {
        let perm = if self.accessibility_granted { "" } else { " ⚠️未授权" };
        match &self.connection {
            ConnectionState::Disconnected => format!("KVMini - 未连接{}", perm),
            ConnectionState::Connecting => format!("KVMini - 正在连接...{}", perm),
            ConnectionState::Connected => {
                let peer = self.peer_name.as_deref().unwrap_or("对方");
                match self.control {
                    ControlState::Idle => format!("KVMini - 已连接 · 各自独立{}", perm),
                    ControlState::LocalControlling => {
                        format!("KVMini - 正在控制 {}{}", peer, perm)
                    }
                    ControlState::RemoteControlling => {
                        format!("KVMini - {} 正在控制本机{}", peer, perm)
                    }
                }
            }
            ConnectionState::Reconnecting { attempt } => {
                format!("KVMini - 正在重连（第 {} 次）{}", attempt, perm)
            }
        }
    }

    /// 获取托盘图标颜色标识
    /// - green：Idle（未连接或已连接各自独立）
    /// - blue：LocalControlling（本机正在控制对方）
    /// - orange：RemoteControlling（对方正在控制本机）
    pub fn icon_color(&self) -> &'static str {
        match self.control {
            ControlState::Idle => "green",
            ControlState::LocalControlling => "blue",
            ControlState::RemoteControlling => "orange",
        }
    }

    /// 获取状态栏显示文本（简短）
    pub fn status_bar_title(&self) -> &'static str {
        match self.control {
            ControlState::Idle => "⌨",
            ControlState::LocalControlling => "⌨→",
            ControlState::RemoteControlling => "←⌨",
        }
    }

    /// 获取连接状态描述文本
    pub fn connection_description(&self) -> String {
        match &self.connection {
            ConnectionState::Disconnected => "未连接".to_string(),
            ConnectionState::Connecting => "正在连接...".to_string(),
            ConnectionState::Connected => {
                let peer = self.peer_name.as_deref().unwrap_or("对方");
                match self.control {
                    ControlState::Idle => format!("已连接 {} · 各自独立", peer),
                    ControlState::LocalControlling => format!("正在控制 {}", peer),
                    ControlState::RemoteControlling => format!("{} 正在控制本机", peer),
                }
            }
            ConnectionState::Reconnecting { attempt } => {
                format!("正在重连（第 {} 次）", attempt)
            }
        }
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionState::Connected)
    }
}

// ── 托盘菜单动作 ────────────────────────────────────────────────────────────

/// 托盘菜单项动作（不含角色切换）
#[derive(Debug, Clone)]
pub enum TrayAction {
    /// 扫描局域网设备（UDP 广播发现）
    ScanDevices,
    /// 手动输入 IP 连接
    ManualConnect,
    /// 断开当前连接
    Disconnect,
    /// 打开设置
    OpenSettings,
    /// 显示/打开辅助功能权限设置
    OpenAccessibilitySettings,
    /// 退出应用
    Quit,
}

// ── macOS 托盘实现 ──────────────────────────────────────────────────────────

/// Mac 系统托盘管理器
pub struct MacTray {
    status: TrayStatus,
    action_callback: Option<Box<dyn Fn(TrayAction) + Send + Sync>>,
}

impl MacTray {
    pub fn new() -> Self {
        Self {
            status: TrayStatus::new(),
            action_callback: None,
        }
    }

    /// 设置菜单动作回调
    pub fn set_action_callback(&mut self, cb: Box<dyn Fn(TrayAction) + Send + Sync>) {
        self.action_callback = Some(cb);
    }

    /// 更新托盘状态并刷新菜单
    pub fn update_status(&mut self, status: TrayStatus) {
        self.status = status;
        #[cfg(target_os = "macos")]
        self.refresh_menu();
    }

    /// 启动托盘（在主线程调用）
    pub fn run(&self) {
        #[cfg(target_os = "macos")]
        {
            self.setup_status_bar();
        }
        #[cfg(not(target_os = "macos"))]
        {
            log::info!("[Mac-stub] 托盘 UI 已启动（非 macOS 平台，仅日志输出）");
            log::info!("  状态: {}", self.status.tooltip());
        }
    }

    #[cfg(target_os = "macos")]
    fn setup_status_bar(&self) {
        use objc::runtime::{Object, YES, NO};
        use objc::{class, msg_send, sel, sel_impl};
        use std::ffi::CString;

        unsafe {
            // NSStatusBar *bar = [NSStatusBar systemStatusBar]
            let bar: *mut Object = msg_send![class!(NSStatusBar), systemStatusBar];

            // NSStatusItem *item = [bar statusItemWithLength:NSVariableStatusItemLength]
            let item: *mut Object = msg_send![bar, statusItemWithLength: -1.0f64];

            // 设置标题
            let title = self.status.status_bar_title();
            let ns_title = ns_string(title);
            let button: *mut Object = msg_send![item, button];
            let _: () = msg_send![button, setTitle: ns_title];

            // 设置 tooltip
            let tooltip = self.status.tooltip();
            let ns_tooltip = ns_string(&tooltip);
            let _: () = msg_send![button, setToolTip: ns_tooltip];

            // 构建菜单
            let menu = self.build_menu();
            let _: () = msg_send![item, setMenu: menu];

            log::info!("[Mac] NSStatusBar 托盘已创建");
        }
    }

    #[cfg(target_os = "macos")]
    fn refresh_menu(&self) {
        // 重新构建菜单（简化实现：每次状态变化重建）
        log::debug!("[Mac] 托盘菜单已刷新: {}", self.status.tooltip());
    }

    /// 构建 NSMenu
    #[cfg(target_os = "macos")]
    fn build_menu(&self) -> *mut objc::runtime::Object {
        use objc::runtime::{Object, YES, NO};
        use objc::{class, msg_send, sel, sel_impl};

        unsafe {
            // NSMenu *menu = [[NSMenu alloc] init]
            let menu: *mut Object = msg_send![class!(NSMenu), new];
            let _: () = msg_send![menu, setAutoenablesItems: NO];

            // ── 状态信息（不可点击）──
            let status_text = self.status.connection_description();
            add_menu_item_disabled(menu, &status_text);

            // 权限警告
            if !self.status.accessibility_granted {
                add_menu_item_disabled(menu, "⚠️ 辅助功能权限未授权");
                add_menu_item(menu, "前往授权...", "openAccessibilitySettings:");
                add_separator(menu);
            } else {
                add_separator(menu);
            }

            // ── 连接操作 ──
            if self.status.is_connected() {
                add_menu_item(menu, "断开连接", "disconnect:");
            } else {
                add_menu_item(menu, "扫描设备...", "scanDevices:");
                add_menu_item(menu, "手动连接...", "manualConnect:");
            }

            add_separator(menu);

            // ── 设置 ──
            add_menu_item(menu, "设置...", "openSettings:");

            add_separator(menu);

            // ── 退出 ──
            add_menu_item(menu, "退出 KVMini", "quit:");

            menu
        }
    }
}

// ── NSMenu 辅助函数 ─────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
unsafe fn ns_string(s: &str) -> *mut objc::runtime::Object {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;

    let c = CString::new(s).unwrap_or_default();
    msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()]
}

#[cfg(target_os = "macos")]
unsafe fn add_menu_item(
    menu: *mut objc::runtime::Object,
    title: &str,
    action: &str,
) {
    use objc::runtime::{Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};

    let item: *mut Object = msg_send![class!(NSMenuItem), new];
    let ns_title = ns_string(title);
    let _: () = msg_send![item, setTitle: ns_title];

    // 设置 action selector（字符串转 SEL）
    let sel_name = std::ffi::CString::new(action).unwrap_or_default();
    let sel_ref: Sel = objc::runtime::Sel::register(sel_name.as_ptr());
    let _: () = msg_send![item, setAction: sel_ref];

    let _: () = msg_send![menu, addItem: item];
}

#[cfg(target_os = "macos")]
unsafe fn add_menu_item_disabled(menu: *mut objc::runtime::Object, title: &str) {
    use objc::runtime::{Object, NO};
    use objc::{class, msg_send, sel, sel_impl};

    let item: *mut Object = msg_send![class!(NSMenuItem), new];
    let ns_title = ns_string(title);
    let _: () = msg_send![item, setTitle: ns_title];
    let _: () = msg_send![item, setEnabled: NO];
    let _: () = msg_send![menu, addItem: item];
}

#[cfg(target_os = "macos")]
unsafe fn add_separator(menu: *mut objc::runtime::Object) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    let sep: *mut Object = msg_send![class!(NSMenuItem), separatorItem];
    let _: () = msg_send![menu, addItem: sep];
}
