/**
 * i18n 多语言模块
 * 支持中文（zh）和英文（en）
 * 语言偏好持久化到 localStorage（key: kvm_settings）
 */

const MESSAGES = {
  zh: {
    // 头部
    appTitle: 'KVMini',
    appSubtitle: '跨平台鼠标键盘共享软件',

    // 状态面板
    labelRole: '当前角色:',
    labelConnectionStatus: '连接状态:',
    labelControlState: '控制状态:',
    labelConnectedDevice: '已连接设备:',
    labelConnectionType: '连接方式:',
    labelLocalIp: '本机 IP:',
    labelLatency: '延迟:',
    roleServer: '被控端',
    roleClient: '主控端',
    statusConnected: '已连接',
    statusConnecting: '连接中...',
    statusDisconnected: '未连接',
    controlStateIdle: '各自独立',
    controlStateLocal: '正在控制对方',
    controlStateRemote: '对方正在控制本机',

    // 控制按钮
    btnScanDevices: '扫描设备',
    btnDisconnect: '断开连接',
    btnStartSharing: '▶ 开始共享',
    btnSettings: '⚙ 设置',
    btnSwitchRole: '切换',
    btnSwitch: '切换',

    // 设备面板
    titleAvailableDevices: '可用设备',
    availableDevices: '可用设备',
    btnScan: '扫描设备',
    scanningText: '扫描中...',
    scanningHint: '正在扫描局域网设备...',
    noDevices: '点击"扫描设备"发现局域网内的设备',
    noDevicesAfterScan: '未发现设备，请确保目标设备在同一局域网内',
    noDevicesFound: '未发现局域网设备，可尝试手动连接',
    scanFailed: '扫描失败，请重试',
    scanFoundDevices: '发现 {n} 台设备',
    btnConnect: '连接',

    // 手动连接
    titleManualConnect: '🔗 手动连接（跨网段）',
    manualConnectTitle: '🔗 手动连接（跨网段）',
    hintManualConnect: '适用于不同网段的设备',
    manualConnectHint: '适用于不同网段的设备',
    placeholderManualIp: '输入目标 IP，如 192.168.1.100',
    manualIpPlaceholder: '请输入目标设备 IP 地址',
    placeholderManualPort: '端口',
    btnManualConnect: '连接',
    btnScanConnect: '扫描设备',

    // 历史记录
    titleHistory: '连接历史',
    connectionHistory: '连接历史',
    btnClearHistory: '清除',
    noHistory: '暂无连接历史',
    btnReconnect: '重连',
    manualIpLabel: '手动 IP',

    // 提示面板
    hintMouseEdge: '💡 提示：连接成功后，将鼠标移到屏幕右边缘即可切换到 Mac 控制模式',
    hintHotkey: '💡 按 Ctrl+Alt+Tab 切换回 Windows 本地控制',

    // 权限警告
    permissionWarningText: '需要辅助功能权限才能开始键鼠共享，请先前往授权',

    // 辅助功能权限遮罩
    overlayTitle: '需要辅助功能权限',
    overlayDesc: 'KVMini 需要辅助功能（Accessibility）权限才能捕获和注入键盘鼠标事件。\n请在系统设置中授权后，应用将自动继续。',
    overlayStep1: '点击下方"前往授权"按钮',
    overlayStep2: '在系统设置中找到 KVMini，打开开关',
    overlayStep3: '返回应用，权限授予后自动继续',
    btnGoAuthorize: '前往授权 →',
    overlayPollingHint: '⏳ 等待授权中，授权后将自动继续...',

    // 设置面板
    titleSettings: '⚙ 设置',
    settingsTitle: '⚙ 设置',
    settingsHotkey: '全局快捷键',
    titleHotkey: '全局快捷键',
    labelToggleFocus: '切换控制焦点:',
    placeholderHotkey: '点击此处录入快捷键',
    hotkeyPlaceholder: '点击此处录入快捷键（默认：Ctrl+Alt+S）',
    btnClearHotkey: '清除',

    settingsConnection: '连接偏好',
    titleConnectionPrefs: '连接偏好',
    labelPreferConnection: '优先连接方式:',
    labelPreferConnectionType: '优先连接方式:',
    optionAuto: '自动选择',
    optionNetwork: '网络优先',
    optionUsb: 'USB优先',
    labelAutoReconnect: '自动重连:',

    settingsStartup: '启动选项',
    titleStartupOptions: '启动选项',
    labelAutoStart: '开机自启动:',
    labelStartMinimized: '启动时最小化到托盘:',

    settingsListen: '监听地址（让对端连接到本机）',
    titleListenAddress: '监听地址（让对端连接到本机）',
    labelListenInterface: '监听网卡:',
    optionAllInterfaces: '所有网卡（0.0.0.0，推荐）',
    labelListenPort: '监听端口:',
    labelListenAddress: '对端连接地址:',
    labelListenAddressDisplay: '对端连接地址:',

    settingsAppearance: '外观',
    settingsDisplay: '显示器布局',
    titleDisplayLayout: '显示器布局',
    hintDisplayLayout: '拖拽显示器方块，设置本机与远端设备的相对位置。鼠标超出对应边缘时自动切换控制权。',
    labelLocalDisplay: '本机',
    labelRemoteDisplay: '远端',
    labelLayoutPosition: '本机位于远端的:',
    optionLocalLeft: '左侧（鼠标→右边缘切换）',
    optionLocalRight: '右侧（鼠标→左边缘切换）',
    optionLocalAbove: '上方（鼠标→下边缘切换）',
    optionLocalBelow: '下方（鼠标→上边缘切换）',
    hintTriggerEdge: '当前：鼠标超出',
    hintTriggerEdgeSuffix: '时切换到远端',
    labelMonitorCount: '本机显示器数量:',
    labelMonitorList: '显示器列表',
    labelPrimaryMonitor: '主显示器',
    labelMonitorResolution: '分辨率',
    labelMonitorDpi: 'DPI 缩放',
    labelMonitorOffset: '偏移',
    btnRefreshMonitors: '刷新显示器',
    msgMonitorsRefreshed: '显示器信息已刷新',
    msgMonitorsChanged: '检测到显示器配置变化，已自动更新',
    hintMultiMonitor: '多显示器模式：鼠标到达任意显示器边缘时触发切换',

    // 语言与主题设置
    titleAppearance: '外观',
    labelLanguage: '语言:',
    optionZh: '中文',
    optionEn: 'English',
    labelTheme: '主题:',
    optionThemeDark: '深色',
    optionThemeLight: '浅色',
    optionThemeAuto: '跟随系统',

    // 版本号
    labelVersion: '版本',

    // 按钮
    btnSave: '保存设置',
    btnSaveSettings: '保存设置',
    btnCancel: '取消',

    // 触发边缘
    edgeRight: '右边缘',
    edgeLeft: '左边缘',
    edgeBottom: '下边缘',
    edgeTop: '上边缘',

    // 提示消息
    msgConnectSuccess: '设备连接成功',
    msgDisconnectSuccess: '已断开连接',
    msgSettingsSaved: '设置已保存',
    msgSwitchedToServer: '已切换为被控端模式',
    msgSwitchedToClient: '已切换为主控端模式',
    msgLocalIpLoading: '获取中...',
    msgLocalIpFailed: '无法获取',
    msgLatencyChecking: '检测中...',

    // 动态 UI 文本
    hotkeyPrompt: '请按下快捷键组合...',
    orOtherLocalIp: '或其他本机 IP',
    connectionLost: '连接已断开',
    reconnecting: '重连中({n}/{max})...',
    reconnectFailed: '连接已断开，自动重连 {max} 次均失败，请手动重连',
    reconnectSuccess: '连接已恢复',
    manualReconnectSuccess: '已重连到 {ip}',

    // 手动连接错误提示
    errIpEmpty: '请输入目标设备的 IP 地址',
    errIpFormat: 'IP 格式不正确："{ip}"，请输入合法的 IPv4 地址（如 192.168.1.100）',
    errPortRange: '端口范围应为 1024-65535',
    errHandshakeFailed: '❌ 连接失败：对端不是 KVMini 服务。请确认对端已启动 KVMini。',
    errVersionMismatch: '❌ 版本不兼容：请确保两端使用相同版本的 KVMini。',
    errTimeout: '⏱ 连接超时：请检查 {ip} 的防火墙设置，确保 KVMini 端口已开放。',
    errRefused: '🚫 连接被拒绝：请确认对端已启动 KVMini 并开放防火墙端口。',
    connectingTo: '⏳ 正在连接 {ip}:{port}，最长等待 5 秒...',
    connectedTo: '✅ 已成功连接到 {ip}:{port}',
    connectedToIp: '已连接到 {ip}',
    reconnectedToIp: '已重连到 {ip}',
    connectErrGeneric: '连接出错：{err}',

    // 功能说明文字
    listenIfaceHint: '选择本机用于接受远端连接的网络接口，通常选"所有网卡"即可',
    hotkeyHint: '按下此快捷键可在本机和远端之间切换键鼠控制权',

    // 布局触发边缘（动态文字）
    triggerEdgeHintPrefix: '当前：鼠标超出',
    triggerEdgeHintSuffix: '时切换到远端',

    // 剪贴板共享
    settingsClipboard: '剪贴板共享',
    labelClipboardSync: '跨设备剪贴板同步:',
    hintClipboardSync: '开启后，在一台设备上复制的文本可直接粘贴到另一台设备（仅支持纯文本，超过 1MB 自动截断）',

    // 安全与加密
    settingsSecurity: '安全与加密',
    labelEnableTls: 'TLS 1.3 加密传输:',
    hintEnableTls: '开启后所有键鼠数据和剪贴板内容均加密传输，关闭后显示安全警告',
    labelLocalFingerprint: '本机证书指纹:',
    btnCopyFingerprint: '复制',
    tlsFingerprintCopied: '指纹已复制到剪贴板',
    tlsFingerprintLoadFailed: '指纹加载失败',
    tlsFingerprintHint: '对端首次连接时需核对此指纹，确认一致后方可信任连接',
    tlsDisabled: '（TLS 已关闭）',
    copied: '已复制',

    // TLS 指纹确认对话框
    tlsConfirmTitle: '首次连接安全确认',
    tlsConfirmDesc: '检测到来自新设备的连接请求，请确认对端证书指纹是否与对端显示的一致：',
    tlsConfirmHint: '如果指纹与对端显示的不一致，请拒绝连接，可能存在中间人攻击风险。',
    btnTrustAndConnect: '信任并连接',
    btnRejectConnection: '拒绝连接',
    tlsDisabledWarning: '⚠️ TLS 加密已关闭，键鼠数据将以明文传输，存在安全风险',

    // 关于 Tab
    tabAbout: '关于',
    settingsLicense: '许可状态',
    settingsAbout: '关于',
    settingsVersion: '版本信息',
    trialChecking: '检测中...',
    trialActive: '试用中（剩余 {days} 天）',
    trialExpired: '试用已过期',
    trialPurchased: '✓ 已购买完整版',
    purchaseBtn: '购买完整版',
    restoreBtn: '恢复购买',
    feedbackTitle: '反馈与建议',
    shareTitle: '推荐给朋友',
    privacyTitle: '隐私政策',
    changelogTitle: '更新日志',
    rateApp: '给个赞吧',
    shareCopied: '分享链接已复制到剪贴板',
    changelogModalTitle: '更新日志',
    privacyModalTitle: '隐私政策',
  },

  en: {
    // Header
    appTitle: 'KVMini',
    appSubtitle: 'Cross-platform Mouse & Keyboard Sharing',

    // Status panel
    labelRole: 'Role:',
    labelConnectionStatus: 'Status:',
    labelControlState: 'Control:',
    labelConnectedDevice: 'Connected Device:',
    labelConnectionType: 'Connection:',
    labelLocalIp: 'Local IP:',
    labelLatency: 'Latency:',
    roleServer: 'Server',
    roleClient: 'Client',
    statusConnected: 'Connected',
    statusConnecting: 'Connecting...',
    statusDisconnected: 'Disconnected',
    controlStateIdle: 'Independent',
    controlStateLocal: 'Controlling Remote',
    controlStateRemote: 'Remote Controlling',

    // Control buttons
    btnScanDevices: 'Scan Devices',
    btnDisconnect: 'Disconnect',
    btnStartSharing: '▶ Start Sharing',
    btnSettings: '⚙ Settings',
    btnSwitchRole: 'Switch',
    btnSwitch: 'Switch',

    // Devices panel
    titleAvailableDevices: 'Available Devices',
    availableDevices: 'Available Devices',
    btnScan: 'Scan',
    scanningText: 'Scanning...',
    scanningHint: 'Scanning LAN devices...',
    noDevices: 'Click "Scan Devices" to find devices on LAN',
    noDevicesAfterScan: 'No devices found. Make sure the target device is on the same LAN.',
    noDevicesFound: 'No LAN devices found. Try manual connect.',
    scanFailed: 'Scan failed, please try again',
    scanFoundDevices: 'Found {n} device(s)',
    btnConnect: 'Connect',

    // Manual connect
    titleManualConnect: '🔗 Manual Connect (Cross-subnet)',
    manualConnectTitle: '🔗 Manual Connect (Cross-subnet)',
    hintManualConnect: 'For devices on different subnets',
    manualConnectHint: 'For devices on different subnets',
    placeholderManualIp: 'Enter target IP, e.g. 192.168.1.100',
    manualIpPlaceholder: 'Enter target device IP address',
    placeholderManualPort: 'Port',
    btnManualConnect: 'Connect',
    btnScanConnect: 'Scan Devices',

    // History
    titleHistory: 'Connection History',
    connectionHistory: 'Connection History',
    btnClearHistory: 'Clear',
    noHistory: 'No connection history',
    btnReconnect: 'Reconnect',
    manualIpLabel: 'Manual IP',

    // Hint panel
    hintMouseEdge: '💡 Tip: After connecting, move the mouse to the right edge to switch to Mac control',
    hintHotkey: '💡 Press Ctrl+Alt+Tab to switch back to Windows local control',

    // Permission warning
    permissionWarningText: 'Accessibility permission is required to start sharing. Please authorize first.',

    // Accessibility overlay
    overlayTitle: 'Accessibility Permission Required',
    overlayDesc: 'KVMini needs Accessibility permission to capture and inject keyboard/mouse events.\nPlease authorize in System Settings and the app will continue automatically.',
    overlayStep1: 'Click the "Authorize" button below',
    overlayStep2: 'Find KVMini in System Settings and enable it',
    overlayStep3: 'Return to the app, it will continue automatically after authorization',
    btnGoAuthorize: 'Authorize →',
    overlayPollingHint: '⏳ Waiting for authorization...',

    // Settings panel
    titleSettings: '⚙ Settings',
    settingsTitle: '⚙ Settings',
    settingsHotkey: 'Global Hotkey',
    titleHotkey: 'Global Hotkey',
    labelToggleFocus: 'Toggle Control Focus:',
    placeholderHotkey: 'Click here to record hotkey',
    hotkeyPlaceholder: 'Click here to record hotkey (default: Ctrl+Alt+S)',
    btnClearHotkey: 'Clear',

    settingsConnection: 'Connection Preferences',
    titleConnectionPrefs: 'Connection Preferences',
    labelPreferConnection: 'Preferred Connection:',
    labelPreferConnectionType: 'Preferred Connection:',
    optionAuto: 'Auto',
    optionNetwork: 'Network First',
    optionUsb: 'USB First',
    labelAutoReconnect: 'Auto Reconnect:',

    settingsStartup: 'Startup Options',
    titleStartupOptions: 'Startup Options',
    labelAutoStart: 'Launch at Login:',
    labelStartMinimized: 'Start Minimized to Tray:',

    settingsListen: 'Listen Address (for remote to connect)',
    titleListenAddress: 'Listen Address (for remote to connect)',
    labelListenInterface: 'Network Interface:',
    optionAllInterfaces: 'All Interfaces (0.0.0.0, recommended)',
    labelListenPort: 'Listen Port:',
    labelListenAddress: 'Remote Connect Address:',
    labelListenAddressDisplay: 'Remote Connect Address:',

    settingsAppearance: 'Appearance',
    settingsDisplay: 'Display Layout',
    titleDisplayLayout: 'Display Layout',
    hintDisplayLayout: 'Drag the display block to set the relative position. Mouse will switch control when crossing the corresponding edge.',
    labelLocalDisplay: 'Local',
    labelRemoteDisplay: 'Remote',
    labelLayoutPosition: 'Local is to the:',
    optionLocalLeft: 'Left (mouse → right edge to switch)',
    optionLocalRight: 'Right (mouse → left edge to switch)',
    optionLocalAbove: 'Above (mouse → bottom edge to switch)',
    optionLocalBelow: 'Below (mouse → top edge to switch)',
    hintTriggerEdge: 'Current: mouse crosses',
    hintTriggerEdgeSuffix: 'to switch to remote',
    labelMonitorCount: 'Local monitor count:',
    labelMonitorList: 'Monitor List',
    labelPrimaryMonitor: 'Primary',
    labelMonitorResolution: 'Resolution',
    labelMonitorDpi: 'DPI Scale',
    labelMonitorOffset: 'Offset',
    btnRefreshMonitors: 'Refresh Monitors',
    msgMonitorsRefreshed: 'Monitor info refreshed',
    msgMonitorsChanged: 'Display configuration changed, updated automatically',
    hintMultiMonitor: 'Multi-monitor: switching triggers when mouse reaches any display edge',

    // Language & Theme settings
    titleAppearance: 'Appearance',
    labelLanguage: 'Language:',
    optionZh: '中文',
    optionEn: 'English',
    labelTheme: 'Theme:',
    optionThemeDark: 'Dark',
    optionThemeLight: 'Light',
    optionThemeAuto: 'System',

    // Version
    labelVersion: 'Version',

    // Buttons
    btnSave: 'Save Settings',
    btnSaveSettings: 'Save Settings',
    btnCancel: 'Cancel',

    // Trigger edges
    edgeRight: 'right edge',
    edgeLeft: 'left edge',
    edgeBottom: 'bottom edge',
    edgeTop: 'top edge',

    // Messages
    msgConnectSuccess: 'Device connected successfully',
    msgDisconnectSuccess: 'Disconnected',
    msgSettingsSaved: 'Settings saved',
    msgSwitchedToServer: 'Switched to Server mode',
    msgSwitchedToClient: 'Switched to Client mode',
    msgLocalIpLoading: 'Loading...',
    msgLocalIpFailed: 'Unavailable',
    msgLatencyChecking: 'Checking...',

    // Dynamic UI text
    hotkeyPrompt: 'Press a hotkey combination...',
    orOtherLocalIp: 'or other local IP',
    connectionLost: 'Connection lost',
    reconnecting: 'Reconnecting({n}/{max})...',
    reconnectFailed: 'Connection lost. Auto-reconnect failed after {max} attempts. Please reconnect manually.',
    reconnectSuccess: 'Connection restored',
    manualReconnectSuccess: 'Reconnected to {ip}',

    // Manual connect error messages
    errIpEmpty: 'Please enter the target IP address',
    errIpFormat: 'Invalid IP format: "{ip}". Please enter a valid IPv4 address (e.g. 192.168.1.100)',
    errPortRange: 'Port must be in range 1024-65535',
    errHandshakeFailed: '❌ Connection failed: The remote is not a KVMini service. Make sure KVMini is running on the remote.',
    errVersionMismatch: '❌ Version mismatch: Please make sure both sides use the same version of KVMini.',
    errTimeout: '⏱ Connection timed out: Check the firewall settings on {ip} and make sure the KVMini port is open.',
    errRefused: '🚫 Connection refused: Make sure KVMini is running and the firewall port is open on the remote.',
    connectingTo: '⏳ Connecting to {ip}:{port}, waiting up to 5 seconds...',
    connectedTo: '✅ Successfully connected to {ip}:{port}',
    connectedToIp: 'Connected to {ip}',
    reconnectedToIp: 'Reconnected to {ip}',
    connectErrGeneric: 'Connection error: {err}',

    // Feature description hints
    listenIfaceHint: 'Select the network interface for accepting remote connections. "All Interfaces" is recommended.',
    hotkeyHint: 'Press this hotkey to toggle keyboard/mouse control between local and remote.',

    // Layout trigger edge (dynamic text)
    triggerEdgeHintPrefix: 'Current: mouse crosses',
    triggerEdgeHintSuffix: 'to switch to remote',

    // Clipboard sharing
    settingsClipboard: 'Clipboard Sharing',
    labelClipboardSync: 'Cross-device clipboard sync:',
    hintClipboardSync: 'When enabled, text copied on one device can be pasted on the other (plain text only, truncated at 1MB)',

    // Security & Encryption
    settingsSecurity: 'Security & Encryption',
    labelEnableTls: 'TLS 1.3 Encrypted Transport:',
    hintEnableTls: 'When enabled, all keyboard/mouse data and clipboard content is encrypted. Disabling shows a security warning.',
    labelLocalFingerprint: 'Local Certificate Fingerprint:',
    btnCopyFingerprint: 'Copy',
    tlsFingerprintCopied: 'Fingerprint copied to clipboard',
    tlsFingerprintLoadFailed: 'Failed to load fingerprint',
    tlsFingerprintHint: 'When the remote connects for the first time, verify this fingerprint matches what is shown on the remote before trusting the connection.',
    tlsDisabled: '(TLS Disabled)',
    copied: 'Copied',

    // TLS fingerprint confirmation dialog
    tlsConfirmTitle: 'First Connection Security Check',
    tlsConfirmDesc: 'A connection request from a new device was detected. Please verify the remote certificate fingerprint matches what is shown on the remote device:',
    tlsConfirmHint: 'If the fingerprint does not match, reject the connection — there may be a man-in-the-middle attack.',
    btnTrustAndConnect: 'Trust & Connect',
    btnRejectConnection: 'Reject',
    tlsDisabledWarning: '⚠️ TLS encryption is disabled. Keyboard/mouse data will be transmitted in plaintext.',

    // About Tab
    tabAbout: 'About',
    settingsLicense: 'License Status',
    settingsAbout: 'About',
    settingsVersion: 'Version Info',
    trialChecking: 'Checking...',
    trialActive: 'Trial ({days} days left)',
    trialExpired: 'Trial Expired',
    trialPurchased: '✓ Full Version Purchased',
    purchaseBtn: 'Buy Full Version',
    restoreBtn: 'Restore Purchase',
    feedbackTitle: 'Feedback',
    shareTitle: 'Recommend to Friends',
    privacyTitle: 'Privacy Policy',
    changelogTitle: 'Changelog',
    rateApp: 'Rate Us',
    shareCopied: 'Share link copied to clipboard',
    changelogModalTitle: 'Changelog',
    privacyModalTitle: 'Privacy Policy',
  },};

/** 当前语言，默认中文 */
let _locale = 'zh';

/**
 * 获取当前语言
 * @returns {'zh'|'en'}
 */
function getLocale() {
  return _locale;
}

/**
 * 设置当前语言（立即生效，不持久化）
 * @param {'zh'|'en'} lang
 */
function setLocale(lang) {
  if (lang === 'zh' || lang === 'en') {
    _locale = lang;
  }
}

/**
 * 翻译函数
 * @param {string} key
 * @returns {string}
 */
function t(key) {
  const msgs = MESSAGES[_locale] || MESSAGES['zh'];
  return msgs[key] !== undefined ? msgs[key] : (MESSAGES['zh'][key] || key);
}

/**
 * 从 localStorage 加载语言设置
 */
function loadLocaleFromStorage() {
  try {
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || 'null');
    if (saved && (saved.language === 'zh' || saved.language === 'en')) {
      _locale = saved.language;
    }
  } catch (_) {
    // 忽略解析错误
  }
}

// 导出（兼容浏览器全局和 Node.js 测试环境）
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { t, getLocale, setLocale, loadLocaleFromStorage, MESSAGES };
} else {
  window.i18n = { t, getLocale, setLocale, loadLocaleFromStorage, MESSAGES };
}
