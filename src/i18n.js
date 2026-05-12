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
    labelTlsStatus: 'TLS 状态:',
    tlsStatusBothOn: '🔒 双端加密',
    tlsStatusLocalOnly: '⚠️ 仅本机开启',
    tlsStatusRemoteOnly: '⚠️ 仅对端开启',
    tlsStatusBothOff: '⚠️ 未加密',
    tlsStatusDisconnected: '-',
    roleServer: '被控端',
    roleClient: '主控端',
    statusConnected: '已连接',
    statusConnecting: '连接中...',
    statusDisconnected: '未连接',
    controlStateIdle: '各自独立',
    controlStateArmed: '已激活·待边缘切换',
    controlStateLocal: '正在控制对方',
    controlStateRemote: '对方正在控制本机',

    // 控制状态徽标（plan v0.1.38 BUG-022：与 controlState* 对齐的 snake_case 别名）
    status_idle: '各自独立',
    status_armed: '已激活·待边缘切换',
    status_local_controlling: '正在控制对方',
    status_remote_controlling: '对方正在控制本机',

    // 控制按钮
    btnScanDevices: '扫描设备',
    btnDisconnect: '断开连接',
    btnStartSharing: '▶ 开始共享',
    btnStopSharing: '■ 停止共享',
    btnArmedWaiting: '⏳ 待切换',
    btnRemoteControlled: '🖱 被远端控制中',
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

    // 设置页分页（dirty tooltip）
    tabDirtyTooltip: '此分类下有未保存的修改',

    // 提示面板
    hintMouseEdge: '💡 提示：连接成功后，将鼠标移到屏幕右边缘即可切换到 Mac 控制模式',
    hintHotkey: '💡 按 Ctrl+Alt+Tab 切换回 Windows 本地控制',

    // 权限警告（BUG-031 按权限类型分开）
    permissionWarningText: '需要「辅助功能」权限才能开始键鼠共享。如果已授权仍提示，请在系统设置中先用"-"删除 KVMini 再用"+"重新添加',
    permissionWarningInputMonitoringText: '需要「输入监控」权限才能开始键鼠共享。授权后请完全退出 KVMini（Cmd+Q）再重新打开，权限才会生效',
    permissionWarningInputMonitoringInfoText: '输入监控权限可能未授予（检测结果可能不准确）。可直接点击"开始共享"尝试，如失败请前往设置授权',
    permissionWarningBtnAccessibility: '前往辅助功能设置',
    permissionWarningBtnInputMonitoring: '前往输入监控设置',

    // 辅助功能权限遮罩
    overlayTitle: '需要辅助功能权限',
    overlayDesc: 'KVMini 需要辅助功能（Accessibility）权限才能捕获和注入键盘鼠标事件。\n请在系统设置中授权后，应用将自动继续。',
    // 输入监控权限遮罩（BUG-027：macOS 除了辅助功能还需要输入监控权限）
    overlayInputMonitoringTitle: '需要输入监控权限',
    overlayInputMonitoringDesc: 'KVMini 还需要输入监控（Input Monitoring）权限才能接收键盘和鼠标事件。\n请在系统设置中授权后，应用将自动继续。',
    overlayStep1: '点击下方"前往授权"按钮',
    overlayStep2: '在系统设置中找到 KVMini，打开开关',
    overlayStep3: '返回应用，权限授予后自动继续',
    btnGoAuthorize: '前往授权 →',
    overlayPollingHint: '⏳ 等待授权中，授权后将自动继续...',

    // 设置面板
    titleSettings: '设置',
    settingsTitle: '⚙ 设置',
    settingsHotkey: '全局快捷键',
    titleHotkey: '全局快捷键',
    labelToggleFocus: '切换控制焦点:',
    placeholderHotkey: '点击此处录入快捷键',
    hotkeyPlaceholder: '点击此处录入快捷键（默认：Ctrl+Alt+S）',
    btnClearHotkey: '清除',

    // 键鼠切换分区（plan v0.1.38 BUG-022）
    settings_kvm_section_title: '键鼠切换',
    settings_edge_trigger_enabled: '鼠标触达边缘时自动切换:',
    settings_edge_threshold: '边缘触发灵敏度（像素）:',
    hintEdgeTrigger: '关闭后仅能通过全局快捷键切换。建议保持开启以获得无缝跨屏体验。',

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

    // 键位映射
    settingsKeyMapping: '键位映射',
    labelKeyMappingUnified: '统一 Cmd/Ctrl 快捷键:',
    hintKeyMappingUnified: '开启后，Mac 的 Command 键与 Windows 的 Ctrl 键互相对应（如 Mac 端 Cmd+C 在 Windows 被控端执行为 Ctrl+C）',


    // 安全与加密
    settingsSecurity: '安全与加密',
    labelEnableTls: 'TLS 1.3 加密传输:',
    hintEnableTls: '开启后所有键鼠数据和剪贴板内容均加密传输，关闭后显示安全警告',
    labelLocalFingerprint: '本机证书指纹:',
    btnCopyFingerprint: '复制',
    tlsFingerprintCopied: '指纹已复制到剪贴板',
    tlsFingerprintLoadFailed: '指纹加载失败',
    tlsFingerprintHint: '对端首次连接时需核对此指纹，确认一致后方可信任连接',

    // 诊断
    settingsDiagnostics: '诊断',
    labelDebugLog: '调试日志模式:',
    hintDebugLog: '开启后重启应用，日志包含详细调试信息（拖拽/滚轮/状态切换），排查完毕请关闭',
    btnOpenLog: '📄 打开日志文件位置',
    msgDebugLogEnabled: '调试模式已开启，重启应用后生效。日志路径：',
    msgDebugLogDisabled: '调试模式已关闭，重启应用后生效',
    tlsDisabled: '（TLS 已关闭）',
    copied: '已复制',

    // TLS 指纹确认对话框
    tlsConfirmTitle: '首次连接安全确认',
    tlsConfirmDesc: '检测到来自新设备的连接请求，请确认对端证书指纹是否与对端显示的一致：',
    tlsConfirmHint: '如果指纹与对端显示的不一致，请拒绝连接，可能存在中间人攻击风险。',
    btnTrustAndConnect: '信任并连接',
    btnRejectConnection: '拒绝连接',
    tlsDisabledWarning: '⚠️ TLS 加密已关闭，键鼠数据将以明文传输，存在安全风险',

    // 页面标题
    pageTitle: 'KVMini - 跨平台键鼠共享',

    // 连接详情面板
    detailCertFingerprint: '证书指纹',

    // 重连进度
    labelReconnectProgress: '重连进度:',
    reconnectingText: '正在重连...',

    // 授权界面（过期引导 + IAP 购买）
    licenseGateTitle: '试用已到期',
    licenseGateDescNew: '感谢试用 KVMini！购买完整版以继续使用全部功能。',
    licenseGateHint: '已购买？点击"恢复购买"即可恢复。',
    purchaseSuccess: '购买成功！',
    purchaseFailed: '购买失败，请重试',
    restoreFailed: '恢复购买功能暂时不可用',
    purchaseProcessing: '购买中...',
    restoreProcessing: '恢复中...',
    restoreSuccess: '购买已恢复！',
    btnBack: '返回',

    // 授权状态
    licenseActivated: '✓ 已激活',
    licenseSdkError: '授权服务暂时不可用',

    // 取消确认对话框
    discardDialogTitle: '⚠️ 未保存的修改',
    discardDialogMsg: '您有未保存的修改，是否放弃？',
    btnDiscardChanges: '放弃修改',
    btnContinueEditing: '继续编辑',

    // 通用按钮
    btnClose: '关闭',
    btnRegenerateCert: '重新生成证书',
    btnTlsDisableRetry: '关闭本机 TLS 重试',

    // 显示器布局标签（layout.js 使用）
    labelLocalMonitor: '本机',
    labelSecondaryMonitor: '副屏',
    labelRemoteMonitor: '远端',
    labelPrimaryTag: '主',

    // 网卡状态
    nicStatusUp: '活跃',
    nicStatusDown: '断开',
    nicNoActive: '未检测到活跃网卡',
    nicDisconnectedWarning: '选定网卡 {name} 已断开，请切换到其他网卡',

    // 网络诊断
    diagTitle: '🔍 网络诊断结果',
    diagSuggestion: '💡 建议：检查上述标记 ⚠️ 的项目，确保对端已运行 KVMini',
    diagAllPass: '✅ 网络配置正常，请确认对端已启动 KVMini 并在同一子网',
    vpnInterferenceTip: '⚠️ 检测到 VPN 网卡（{names}），可能劫持默认路由导致局域网广播失败。建议临时断开 VPN 后重新扫描。',
    firewallBlockTipMacos: '🛡️ macOS 防火墙可能拦截 KVMini。可执行：sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/KVMini.app && sudo /usr/libexec/ApplicationFirewall/socketfilterfw --unblock /Applications/KVMini.app',
    firewallBlockTipWindows: '🛡️ Windows 防火墙可能拦截 KVMini。请在「Windows Defender 防火墙 > 允许的应用」中放行 KVMini。',
    sharingArmed: '已激活边缘检测，鼠标推到屏幕边缘即可切换控制',
    sharingStarted: '共享已启动，正在控制对端',
    sharingStopped: '共享已停止',
    shareToggleFailed: '切换共享状态失败，请检查辅助功能权限或连接状态',
    captureFailedPermission: '⚠️ 启动共享失败：缺少输入监控权限。请在系统设置中授权后重试"开始共享"。',
    // v0.1.43 Bug #035: 远端用户物理输入抢占回切 toast（Synergy/Logitech Flow 风格）
    preempt_resume_toast: '远端用户恢复了本机控制',

    // 分页导航
    tabGeneral: '通用',
    tabDisplay: '显示器布局',
    tabNetwork: '网络',
    tabSecurity: '安全',

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
    privacyLoadFailed: '隐私政策内容加载失败，请稍后重试。',
    changelogEmpty: '暂无更新日志',
    changelogLoadFailed: '更新日志加载失败，请稍后重试。',
    feedbackFailed: '反馈功能暂时不可用',

    // 自动更新
    checkUpdate: '检查更新',
    checkingUpdate: '检查中...',
    updateAvailable: '发现新版本 v{version}',
    updateAvailableTitle: '🎉 发现新版本',
    updateAvailableDesc: '新版本 v{version} 已发布，是否立即更新？',
    updateDownloading: '正在下载更新...',
    updateInstalling: '正在安装更新...',
    updateReady: '更新已就绪，重启后生效',
    updateFailed: '更新失败，请稍后重试',
    updateFailedDetail: '更新失败: {err}',
    noUpdateAvailable: '已是最新版本 🎉',
    updateNow: '立即更新',
    updateLater: '稍后再说',
    updateRestart: '重启应用',

    // BUG-021/plan v0.1.38：i18n 残留清扫新增 key
    errTauriNotReady: 'Tauri 环境未就绪，请重启应用',
    errTauriInitFailed: 'Tauri 环境初始化失败，请重启应用',
    msgDisconnectManual: '已手动断开连接',
    errDisconnectPrefix: '断开连接错误: ',
    errReconnectFailPrefix: '重新连接失败: ',
    errReconnectNoTarget: '无法重连：未找到上次连接的设备，请手动连接',
    msgReconnectFailRetrying: '重连失败，仍在后台尝试...',
    msgTlsMismatchDefault: 'TLS 配置不一致，请检查双端设置',
    msgTlsMismatchLocalOnly: '本机已开启 TLS 加密，但对端未开启，连接可能失败。请关闭本机 TLS 或通知对端开启。',
    msgTlsMismatchRemoteOnly: '对端已开启 TLS 加密，但本机未开启，连接可能失败。请开启本机 TLS 或通知对端关闭。',
    msgConnectionLostDefault: '连接已断开',
    msgReconnectSuccessDefault: '重连成功',
    errSettingsSavePrefix: '设置保存失败: ',
    errHotkeyRegisterPrefix: '快捷键注册失败: ',
    errDisableTlsRetryPrefix: '关闭 TLS 重试失败：',
    peerAddressPrefix: '对端地址：',
    labelLocalIpShort: '本机IP:',
    latencyTipHigh: '网络延迟较高，可能影响使用体验',
    latencyTipCritical: '网络延迟过高，使用体验可能受到严重影响',
    tlsBadgeOn: '🔒 TLS 加密',
    tlsBadgeOff: '⚠️ 未加密',
    notifyDisconnectedManualRetry: '连接已断开，请手动重连',
    scanningSubnetTip: '正在扫描...（UDP广播 + 子网扫描）',
    msgConnectedToIp: '已连接到 {ip}',
    monitorRemoteDisconnected: '远端设备（未连接）',
    monitorTooltipTemplate: '{w}×{h} | DPI: {dpi}% | 偏移: ({x}, {y})',
    certRegenSuccess: '证书已重新生成',
    certRegenFailed: '证书重新生成失败',
    shareText: 'KVMini - 跨平台键鼠共享工具，让多台电脑共享一套键鼠！https://kvmini.app',
    nicDescriptionHint: '您的设备有多个网络接口（如 Wi-Fi、有线网、VPN 等），选择<strong>所有网卡</strong>可接受来自任意网络的连接；选择具体网卡可限制只接受该网络的连接。',
    nicDisconnectedInlineText: '选定网卡已断开，请切换到其他网卡',
    tlsFingerprintLoading: '加载中...',
    certLoadFailed: '证书加载失败',
    // v0.1.41: Windows 管理员权限
    winAdminRequired: '需要管理员权限',
    winAdminBannerText: '当前以普通用户运行，键鼠 Hook 可能受限。建议以管理员身份重新启动。',
    winAdminBtnElevate: '以管理员身份重启',
    // v0.1.41: FocusKeeper 状态
    focusKeeperActive: '焦点保持已启动',
    focusKeeperStopped: '焦点保持已停止',
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
    labelTlsStatus: 'TLS Status:',
    tlsStatusBothOn: '🔒 Both Encrypted',
    tlsStatusLocalOnly: '⚠️ Local Only',
    tlsStatusRemoteOnly: '⚠️ Remote Only',
    tlsStatusBothOff: '⚠️ Unencrypted',
    tlsStatusDisconnected: '-',
    roleServer: 'Server',
    roleClient: 'Client',
    statusConnected: 'Connected',
    statusConnecting: 'Connecting...',
    statusDisconnected: 'Disconnected',
    controlStateIdle: 'Independent',
    controlStateArmed: 'Armed · Waiting for edge',
    controlStateLocal: 'Controlling Remote',
    controlStateRemote: 'Remote Controlling',

    // Control state badge (plan v0.1.38 BUG-022: snake_case aliases aligned with controlState*)
    status_idle: 'Independent',
    status_armed: 'Armed · Waiting for edge',
    status_local_controlling: 'Controlling Remote',
    status_remote_controlling: 'Remote Controlling',

    // Control buttons
    btnScanDevices: 'Scan Devices',
    btnDisconnect: 'Disconnect',
    btnStartSharing: '▶ Start Sharing',
    btnStopSharing: '■ Stop Sharing',
    btnArmedWaiting: '⏳ Waiting',
    btnRemoteControlled: '🖱 Being Controlled',
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

    // Settings tabs (dirty tooltip)
    tabDirtyTooltip: 'Unsaved changes in this section',

    // Hint panel
    hintMouseEdge: '💡 Tip: After connecting, move the mouse to the right edge to switch to Mac control',
    hintHotkey: '💡 Press Ctrl+Alt+Tab to switch back to Windows local control',

    // Permission warning (BUG-031: distinguish permission kinds)
    permissionWarningText: 'Accessibility permission is required. If already granted but still showing, remove KVMini with "-" in System Settings then re-add with "+".',
    permissionWarningInputMonitoringText: 'Input Monitoring permission is required to start sharing. After authorizing, please fully quit KVMini (Cmd+Q) and relaunch for the permission to take effect.',
    permissionWarningInputMonitoringInfoText: 'Input Monitoring permission may not be granted (detection might be inaccurate). You can try clicking "Start Sharing" — if it fails, please authorize in Settings.',
    permissionWarningBtnAccessibility: 'Open Accessibility Settings',
    permissionWarningBtnInputMonitoring: 'Open Input Monitoring Settings',

    // Accessibility overlay
    overlayTitle: 'Accessibility Permission Required',
    overlayDesc: 'KVMini needs Accessibility permission to capture and inject keyboard/mouse events.\nPlease authorize in System Settings and the app will continue automatically.',
    // Input Monitoring overlay (BUG-027: macOS needs BOTH Accessibility and Input Monitoring)
    overlayInputMonitoringTitle: 'Input Monitoring Permission Required',
    overlayInputMonitoringDesc: 'KVMini also needs Input Monitoring permission to receive keyboard and mouse events.\nPlease authorize in System Settings and the app will continue automatically.',
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

    // KVM switch section (plan v0.1.38 BUG-022)
    settings_kvm_section_title: 'KVM Switching',
    settings_edge_trigger_enabled: 'Auto-switch when cursor hits edge:',
    settings_edge_threshold: 'Edge trigger sensitivity (pixels):',
    hintEdgeTrigger: 'When disabled, only the global hotkey can switch control. Keep enabled for seamless cross-screen experience.',

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

    // Key mapping
    settingsKeyMapping: 'Key Mapping',
    labelKeyMappingUnified: 'Unify Cmd/Ctrl shortcuts:',
    hintKeyMappingUnified: 'When enabled, Mac Command key maps to Windows Ctrl key (e.g. Cmd+C on Mac performs Ctrl+C on Windows)',


    // Security & Encryption
    settingsSecurity: 'Security & Encryption',
    labelEnableTls: 'TLS 1.3 Encrypted Transport:',
    hintEnableTls: 'When enabled, all keyboard/mouse data and clipboard content is encrypted. Disabling shows a security warning.',
    labelLocalFingerprint: 'Local Certificate Fingerprint:',
    btnCopyFingerprint: 'Copy',
    tlsFingerprintCopied: 'Fingerprint copied to clipboard',
    tlsFingerprintLoadFailed: 'Failed to load fingerprint',
    tlsFingerprintHint: 'When the remote connects for the first time, verify this fingerprint matches what is shown on the remote before trusting the connection.',

    // Diagnostics
    settingsDiagnostics: 'Diagnostics',
    labelDebugLog: 'Debug Log Mode:',
    hintDebugLog: 'When enabled, restart the app to get detailed debug logs (drag/scroll/state). Disable after troubleshooting.',
    btnOpenLog: '📄 Open Log File Location',
    msgDebugLogEnabled: 'Debug mode enabled. Restart app to take effect. Log path: ',
    msgDebugLogDisabled: 'Debug mode disabled. Restart app to take effect.',
    tlsDisabled: '(TLS Disabled)',
    copied: 'Copied',

    // TLS fingerprint confirmation dialog
    tlsConfirmTitle: 'First Connection Security Check',
    tlsConfirmDesc: 'A connection request from a new device was detected. Please verify the remote certificate fingerprint matches what is shown on the remote device:',
    tlsConfirmHint: 'If the fingerprint does not match, reject the connection — there may be a man-in-the-middle attack.',
    btnTrustAndConnect: 'Trust & Connect',
    btnRejectConnection: 'Reject',
    tlsDisabledWarning: '⚠️ TLS encryption is disabled. Keyboard/mouse data will be transmitted in plaintext.',

    // Page title
    pageTitle: 'KVMini - Cross-platform KVM Sharing',

    // Connection detail panel
    detailCertFingerprint: 'Certificate Fingerprint',

    // Reconnect progress
    labelReconnectProgress: 'Reconnect:',
    reconnectingText: 'Reconnecting...',

    // License gate (expiry + IAP purchase)
    licenseGateTitle: 'Trial Expired',
    licenseGateDescNew: 'Thank you for trying KVMini! Purchase the full version to continue using all features.',
    licenseGateHint: 'Already purchased? Tap "Restore Purchase" to restore.',
    purchaseSuccess: 'Purchase successful!',
    purchaseFailed: 'Purchase failed, please try again',
    restoreFailed: 'Restore purchase is temporarily unavailable',
    purchaseProcessing: 'Purchasing...',
    restoreProcessing: 'Restoring...',
    restoreSuccess: 'Purchase restored!',
    btnBack: 'Back',

    // License status
    licenseActivated: '✓ Activated',
    licenseSdkError: 'License service temporarily unavailable',

    // Discard dialog
    discardDialogTitle: '⚠️ Unsaved Changes',
    discardDialogMsg: 'You have unsaved changes. Discard them?',
    btnDiscardChanges: 'Discard Changes',
    btnContinueEditing: 'Continue Editing',

    // Common buttons
    btnClose: 'Close',
    btnRegenerateCert: 'Regenerate Certificate',
    btnTlsDisableRetry: 'Disable Local TLS & Retry',

    // Monitor layout labels (used by layout.js)
    labelLocalMonitor: 'Local',
    labelSecondaryMonitor: 'Secondary',
    labelRemoteMonitor: 'Remote',
    labelPrimaryTag: 'Primary',

    // Network interface status
    nicStatusUp: 'Active',
    nicStatusDown: 'Down',
    nicNoActive: 'No active network interfaces detected',
    nicDisconnectedWarning: 'Selected interface {name} is disconnected. Please switch to another.',

    // Network Diagnostics
    diagTitle: '🔍 Network Diagnostic Results',
    diagSuggestion: '💡 Suggestion: Check the ⚠️ items above and make sure the remote is running KVMini',
    diagAllPass: '✅ Network configuration looks good. Please confirm the remote has KVMini running on the same subnet.',
    vpnInterferenceTip: '⚠️ VPN interface(s) detected ({names}). VPN may hijack the default route and break LAN broadcast. Try temporarily disconnecting the VPN, then rescan.',
    firewallBlockTipMacos: '🛡️ macOS firewall may be blocking KVMini. Run: sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/KVMini.app && sudo /usr/libexec/ApplicationFirewall/socketfilterfw --unblock /Applications/KVMini.app',
    firewallBlockTipWindows: '🛡️ Windows firewall may be blocking KVMini. Allow KVMini in "Windows Defender Firewall > Allowed apps".',
    sharingArmed: 'Edge detection activated. Push mouse to screen edge to switch.',
    sharingStarted: 'Sharing started, you are now controlling the remote machine',
    sharingStopped: 'Sharing stopped',
    shareToggleFailed: 'Failed to toggle sharing. Check accessibility permission or connection status.',
    captureFailedPermission: '⚠️ Failed to start sharing: Input Monitoring permission required. Please authorize in System Settings and try "Start Sharing" again.',
    // v0.1.43 Bug #035: Remote physical input preemption toast (Synergy / Logitech Flow style)
    preempt_resume_toast: 'Remote user resumed control',

    // Tab navigation
    tabGeneral: 'General',
    tabDisplay: 'Display Layout',
    tabNetwork: 'Network',
    tabSecurity: 'Security',

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
    privacyLoadFailed: 'Failed to load privacy policy. Please try again later.',
    changelogEmpty: 'No changelog available',
    changelogLoadFailed: 'Failed to load changelog. Please try again later.',
    feedbackFailed: 'Feedback is temporarily unavailable',

    // Auto Update
    checkUpdate: 'Check for Updates',
    checkingUpdate: 'Checking...',
    updateAvailable: 'New version v{version} available',
    updateAvailableTitle: '🎉 Update Available',
    updateAvailableDesc: 'Version v{version} is available. Update now?',
    updateDownloading: 'Downloading update...',
    updateInstalling: 'Installing update...',
    updateReady: 'Update ready, restart to apply',
    updateFailed: 'Update failed, please try again later',
    updateFailedDetail: 'Update failed: {err}',
    noUpdateAvailable: 'You are up to date 🎉',
    updateNow: 'Update Now',
    updateLater: 'Later',
    updateRestart: 'Restart App',

    // BUG-021/plan v0.1.38: i18n residual cleanup
    errTauriNotReady: 'Tauri runtime is not ready, please restart the app',
    errTauriInitFailed: 'Tauri runtime initialization failed, please restart the app',
    msgDisconnectManual: 'Disconnected manually',
    errDisconnectPrefix: 'Disconnect error: ',
    errReconnectFailPrefix: 'Reconnect failed: ',
    errReconnectNoTarget: 'Cannot reconnect: no previous device found, please connect manually',
    msgReconnectFailRetrying: 'Reconnect failed, still retrying in background...',
    msgTlsMismatchDefault: 'TLS configuration mismatch, please check both sides',
    msgTlsMismatchLocalOnly: 'Local TLS is enabled but remote is not. Connection may fail. Please disable local TLS or ask the remote to enable it.',
    msgTlsMismatchRemoteOnly: 'Remote TLS is enabled but local is not. Connection may fail. Please enable local TLS or ask the remote to disable it.',
    msgConnectionLostDefault: 'Connection lost',
    msgReconnectSuccessDefault: 'Reconnected',
    errSettingsSavePrefix: 'Failed to save settings: ',
    errHotkeyRegisterPrefix: 'Hotkey registration failed: ',
    errDisableTlsRetryPrefix: 'Failed to disable TLS and retry: ',
    peerAddressPrefix: 'Peer address: ',
    labelLocalIpShort: 'Local IP:',
    latencyTipHigh: 'Network latency is high, may affect user experience',
    latencyTipCritical: 'Network latency is very high, experience may be severely impacted',
    tlsBadgeOn: '🔒 TLS compatibility mode (data not yet encrypted; real TLS encryption coming in v0.1.39)',
    tlsBadgeOff: '⚠️ Unencrypted (neither side negotiated TLS)',
    notifyDisconnectedManualRetry: 'Connection lost, please reconnect manually',
    scanningSubnetTip: 'Scanning... (UDP broadcast + subnet scan)',
    msgConnectedToIp: 'Connected to {ip}',
    monitorRemoteDisconnected: 'Remote device (not connected)',
    monitorTooltipTemplate: '{w}×{h} | DPI: {dpi}% | Offset: ({x}, {y})',
    certRegenSuccess: 'Certificate regenerated',
    certRegenFailed: 'Failed to regenerate certificate',
    shareText: 'KVMini - Cross-platform keyboard & mouse sharing tool, share one set of keyboard & mouse across multiple computers! https://kvmini.app',
    nicDescriptionHint: 'Your device has multiple network interfaces (Wi-Fi, Ethernet, VPN, etc.). Select <strong>All Interfaces</strong> to accept connections from any network, or pick a specific interface to restrict connections to that network only.',
    nicDisconnectedInlineText: 'Selected network interface is disconnected. Please switch to another.',
    tlsFingerprintLoading: 'Loading...',
    certLoadFailed: 'Failed to load certificate',
    // v0.1.41: Windows admin permission
    winAdminRequired: 'Administrator Required',
    winAdminBannerText: 'Running as standard user. Keyboard/mouse hooks may be limited. Please restart as administrator.',
    winAdminBtnElevate: 'Restart as Admin',
    // v0.1.41: FocusKeeper status
    focusKeeperActive: 'Focus keeper active',
    focusKeeperStopped: 'Focus keeper stopped',
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
