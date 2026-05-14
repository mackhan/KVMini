// 全局状态
let appState = {
    connected: false,
    connectionType: null,
    // v0.1.59 BUG-042: 连接按钮三态（disconnected / connecting / connected）
    connectionPhase: 'disconnected',
    devices: [],
    currentDevice: null,
    scanning: false,
    settings: {
        hotkey: '',
        preferConnectionType: 'auto',
        autoReconnect: true,
        autoStart: false,
        startMinimized: false,
        language: 'zh',
        theme: 'auto',
    },
    // 默认值（启动时从后端 Platform.getSettings 填充，兜底 23456）
    // 所有"需要端口兜底"的地方都必须读这里，禁止硬编码任何端口字面量
    defaults: {
        tcpPort: 23456,
    },
};

// invoke 函数引用（在启动时安全初始化）
let invoke = null;

/**
 * 安全获取 Tauri invoke 函数
 * 兼容 Tauri 2.x 的挂载方式
 */
function getTauriInvoke() {
    // 优先使用前置内联脚本保存的 invoke（最可靠，避免 WKWebView 脚本隔离问题）
    if (window.tauriInvokeReady) {
        return window.tauriInvokeReady;
    }
    // Tauri 2.x: withGlobalTauri=true 时挂载到 window.__TAURI__.core.invoke
    if (window.__TAURI__?.core?.invoke) {
        return window.__TAURI__.core.invoke;
    }
    // 兼容旧版本直接挂载到 window.__TAURI__.invoke
    if (window.__TAURI__?.invoke) {
        return window.__TAURI__.invoke;
    }
    // 未找到，返回 null（非 Tauri 环境，如浏览器预览）
    return null;
}

/**
 * 等待 Tauri invoke 就绪
 * 新架构：优先检查 window.tauriInvokeReady（前置内联脚本捕获），
 * 其次轮询 window.__TAURI__，超时后返回 null（降级模式）。
 * @param {number} timeout - 超时毫秒数，默认 5000ms
 * @param {number} interval - 轮询间隔毫秒数，默认 50ms
 * @returns {Promise<Function|null>} invoke 函数，或 null（超时降级）
 */
function waitForTauri(timeout = 5000, interval = 50) {
    return new Promise((resolve) => {
        // 优先：前置内联脚本已捕获 invoke
        const earlyInvoke = getTauriInvoke();
        if (earlyInvoke) {
            resolve(earlyInvoke);
            return;
        }

        const startTime = Date.now();
        const timer = setInterval(() => {
            const inv = getTauriInvoke();
            if (inv) {
                clearInterval(timer);
                resolve(inv);
                return;
            }
            if (Date.now() - startTime >= timeout) {
                clearInterval(timer);
                console.error(`[KVMini] waitForTauri 超时（${timeout}ms），Tauri 环境未就绪`);
                resolve(null); // 超时降级
            }
        }, interval);
    });
}

// DOM元素引用（在 DOMContentLoaded 后初始化）
let elements = {};

// 初始化应用
async function initApp() {
    console.log('[KVMini] initApp 开始');
    try {
        const logs = JSON.parse(localStorage.getItem('kvmini_diag') || '[]');
        logs.push(new Date().toISOString() + ' initApp 被调用');
        localStorage.setItem('kvmini_diag', JSON.stringify(logs));
    } catch (e) {}
    // 安全获取 invoke
    invoke = getTauriInvoke();
    // 诊断：记录 invoke 状态
    const invStatus = invoke ? 'YES' : 'NO';
    console.log('[KVMini] initApp: invoke=' + invStatus);
    if (invoke) {
        invoke('write_startup_log', { content: '[JS] initApp 被调用，invoke=' + invStatus }).catch(() => {});
    }

    // 初始化平台抽象层
    window.Platform.init(invoke);

    // 注入平台标识到 <html> 元素，用于 CSS 平台差异化样式
    try {
        const platform = await window.Platform.getPlatform();
        const platformId = (platform || '').toLowerCase().includes('windows') ? 'windows' : 'macos';
        document.documentElement.setAttribute('data-platform', platformId);
        console.log(`[KVMini] 平台标识: data-platform="${platformId}"`);
    } catch (e) {
        // 降级：通过 userAgent 判断
        const isWin = navigator.userAgent.includes('Windows');
        document.documentElement.setAttribute('data-platform', isWin ? 'windows' : 'macos');
        console.warn('[KVMini] getPlatform 失败，通过 UA 降级判断:', e);
    }

    // 初始化 DOM 元素引用
    elements = {
        localIp: document.getElementById('local-ip'),
        connectionStatus: document.getElementById('connection-status'),
        connectionType: document.getElementById('connection-type'),
        latency: document.getElementById('latency'),
        // v0.1.59 BUG-042: 单按钮三态（disconnected / connecting / connected）
        connectBtn: document.getElementById('connect-btn'),
        // 兼容字段：保留 disconnectBtn 别名（部分旧代码引用），实际指向同一按钮
        disconnectBtn: document.getElementById('connect-btn'),
        startSharingBtn: document.getElementById('start-sharing-btn'),
        permissionWarning: document.getElementById('permission-warning'),
        settingsBtn: document.getElementById('settings-btn'),
        devicesList: document.getElementById('devices-list'),
        scanBtn: document.getElementById('scan-btn'),
        scanIcon: document.getElementById('scan-icon'),
        scanText: document.getElementById('scan-text'),
        scanAnimation: document.getElementById('scan-animation'),
        historyList: document.getElementById('history-list'),
        clearHistoryBtn: document.getElementById('clear-history-btn'),
        viewMain: document.getElementById('view-main'),
        viewSettings: document.getElementById('view-settings'),
        errorToast: document.getElementById('error-toast'),
        errorMessage: document.getElementById('error-message'),
        successToast: document.getElementById('success-toast'),
        successMessage: document.getElementById('success-message'),
        hotkeyInput: document.getElementById('hotkey-input'),
        preferConnectionType: document.getElementById('prefer-connection-type'),
        autoReconnect: document.getElementById('auto-reconnect'),
        autoStart: document.getElementById('auto-start'),
        startMinimized: document.getElementById('start-minimized'),
        // 手动连接
        manualIpInput: document.getElementById('manual-ip-input'),
        manualPortInput: document.getElementById('manual-port-input'),
        manualConnectBtn: document.getElementById('manual-connect-btn'),
        manualConnectStatus: document.getElementById('manual-connect-status'),
        // 监听地址配置
        listenIpSelect: document.getElementById('listen-ip-select'),
        listenPortInput: document.getElementById('listen-port-input'),
        listenAddressDisplay: document.getElementById('listen-address-display'),
        // Windows 特有
        errorBanner: document.getElementById('error-banner'),
        errorMsg: document.getElementById('error-msg'),
        errorClose: document.getElementById('error-close'),
        hintPanel: document.getElementById('hint-panel'),
        // 控制状态
        controlState: document.getElementById('control-state'),
        // 已连接设备
        connectedDevice: document.getElementById('connected-device'),
        // 外观设置
        languageSelect: document.getElementById('language-select'),
        themeSelect: document.getElementById('theme-select'),
        // 剪贴板共享
        clipboardSync: document.getElementById('clipboard-sync'),
        // 键位映射
        keyMappingUnified: document.getElementById('key-mapping-unified'),
        // KVM 键鼠切换（plan v0.1.38 BUG-022）
        edgeTriggerToggle: document.getElementById('edge-trigger-toggle'),
        edgeThresholdInput: document.getElementById('edge-threshold-input'),
        // TLS 加密
        enableTls: document.getElementById('enable-tls'),
        localTlsFingerprint: document.getElementById('local-tls-fingerprint'),
        // 诊断
        debugLogToggle: document.getElementById('debug-log-toggle'),
        openLogBtn: document.getElementById('open-log-btn'),
        // 版本号
        appVersion: document.getElementById('app-version'),
        settingsVersion: document.getElementById('settings-version'),
        // 重连相关（v0.1.59 BUG-042: reconnectBtn 已合并到 connect-btn 单按钮三态）
        reconnectStatusRow: document.getElementById('reconnect-status-row'),
        reconnectStatusText: document.getElementById('reconnect-status-text'),
        reconnectToast: document.getElementById('reconnect-toast'),
        reconnectToastMessage: document.getElementById('reconnect-toast-message'),
        // 关于 Tab
        aboutTrialStatus: document.getElementById('about-trial-status'),
        aboutTrialStatusText: document.getElementById('about-trial-status-text'),
        aboutPurchaseActions: document.getElementById('about-purchase-actions'),
        aboutPurchaseBtn: document.getElementById('about-purchase-btn'),
        aboutRestoreBtn: document.getElementById('about-restore-btn'),
        aboutFeedbackBtn: document.getElementById('about-feedback-btn'),
        aboutShareBtn: document.getElementById('about-share-btn'),
        aboutPrivacyBtn: document.getElementById('about-privacy-btn'),
        aboutChangelogBtn: document.getElementById('about-changelog-btn'),
        aboutRateBtn: document.getElementById('about-rate-btn'),
        aboutCheckUpdateBtn: document.getElementById('about-check-update-btn'),
        aboutVersionText: document.getElementById('about-version-text'),
    };

    setupEventListeners();
    updateUI();
    loadSettings();
    await loadBackendDefaults();
    loadConnectionHistory();
    loadLocalIp();
    initAppearance();
    initVersion();
    prefillDefaultIp();

    // 注入平台 class 到 body，供 CSS 平台适配使用；同时控制 hint-panel 显示
    {
        const isWindows = navigator.userAgent.includes('Windows') ||
            (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32');
        document.body.classList.remove('platform-windows', 'platform-macos');
        document.body.classList.add(isWindows ? 'platform-windows' : 'platform-macos');
        if (elements.hintPanel) {
            elements.hintPanel.style.display = isWindows ? '' : 'none';
        }
        // v0.1.41: Windows 管理员权限检测
        if (isWindows && invoke) {
            checkWinAdminPermission();
        }
    }

    // 新架构：权限检测由 Rust 侧主动推送事件，前端只需注册事件监听器
    // 事件监听器和「前往授权」按钮绑定已在 accessibility.js 中完成

    // 延迟初始化显示器布局配置（需要 invoke 就绪后才能调用后端）
    setTimeout(initDisplayLayout, 500);

    // 自动启动 TCP 监听（等待对端连接）
    if (invoke) {
        invoke('start_listening').then((msg) => {
            console.log('[KVMini] 监听已启动:', msg);
        }).catch((e) => {
            console.warn('[KVMini] 启动监听失败（可能端口已占用）:', e);
        });
    }
}

/**
 * 根据当前平台预填充手动连接 IP 输入框的默认值
 * 仅在输入框为空时填入，不覆盖用户已输入的内容
 * 不再硬编码开发 IP，由用户手动输入目标设备 IP
 */
function prefillDefaultIp() {
    // 不再预填任何 IP，让用户根据实际网络环境手动输入
    // 之前硬编码的 30.19.x.x 是特定开发环境 IP，不通用
}

// 设置事件监听器
function setupEventListeners() {
    if (!elements.connectBtn) {
        console.error('[KVMini] 严重错误：connect-btn 元素不存在，事件绑定失败！');
        return;
    }
    // v0.1.59 BUG-042: 单按钮三态分发器
    // disconnected → 触发重连（沿用历史 IP）
    // connecting   → 禁用，无操作
    // connected    → 断开
    elements.connectBtn.addEventListener('click', handleConnectButtonClick);
    if (elements.startSharingBtn) {
        elements.startSharingBtn.addEventListener('click', handleToggleSharing);
    }
    elements.settingsBtn.addEventListener('click', openSettings);
    elements.scanBtn.addEventListener('click', handleScan);
    elements.clearHistoryBtn.addEventListener('click', handleClearHistory);
    elements.manualConnectBtn.addEventListener('click', handleManualConnect);

    elements.manualIpInput.addEventListener('keydown', (e) => {        if (e.key === 'Enter') handleManualConnect();
    });

    // 快捷键录入
    elements.hotkeyInput.addEventListener('keydown', handleHotkeyCapture);

    // ESC 键关闭设置页（有未保存修改时弹出确认对话框）
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            const settingsView = elements.viewSettings;
            if (settingsView && !settingsView.classList.contains('hidden')) {
                e.preventDefault();
                closeSettings();
            }
        }
    });

    // 语言选择器：实时切换（不保存）
    if (elements.languageSelect) {
        elements.languageSelect.addEventListener('change', () => {
            const lang = elements.languageSelect.value;
            window.i18n.setLocale(lang);
            applyI18n();
        });
    }

    // 主题选择器：实时切换（不保存）
    if (elements.themeSelect) {
        elements.themeSelect.addEventListener('change', () => {
            window.themeManager.setTheme(elements.themeSelect.value);
        });
    }

    // TLS 开关：实时更新指纹显示
    if (elements.enableTls) {
        elements.enableTls.addEventListener('change', () => {
            loadLocalTlsFingerprint();
        });
    }

    // 调试日志开关
    if (elements.debugLogToggle) {
        // 初始化：读取当前状态（通过 get_log_path 检测 debug_mode 文件是否存在）
        if (invoke) {
            invoke('set_debug_log', { enabled: false }).catch(() => {});
            // 检查是否已开启（通过后端查日志目录的 debug_mode 文件）
            invoke('get_log_path').then(logPath => {
                const debugFlagPath = logPath.replace('runtime.log', 'debug_mode');
                // 无法直接读文件系统，用 try 调 set_debug_log(false) 判断不准确
                // 改为：启动时调一次 check_debug_mode 命令（简化：默认 unchecked，用户手动勾选）
            }).catch(() => {});
        }

        elements.debugLogToggle.addEventListener('change', async () => {
            if (!invoke) return;
            const { t } = window.i18n;
            const enabled = elements.debugLogToggle.checked;
            try {
                const logPath = await invoke('set_debug_log', { enabled });
                if (enabled) {
                    showSuccess(t('msgDebugLogEnabled') + logPath);
                } else {
                    showSuccess(t('msgDebugLogDisabled'));
                }
            } catch (e) {
                console.warn('[Debug] set_debug_log 失败:', e);
                showError(String(e));
                elements.debugLogToggle.checked = !enabled; // 回滚
            }
        });
    }

    // 打开日志文件位置
    if (elements.openLogBtn) {
        elements.openLogBtn.addEventListener('click', async () => {
            if (!invoke) return;
            try {
                const logPath = await invoke('get_log_path');
                // 用 shell.open 打开文件所在目录
                if (window.__TAURI__?.shell?.open) {
                    const dir = logPath.replace(/[/\\][^/\\]+$/, '');
                    await window.__TAURI__.shell.open(dir);
                } else if (window.__TAURI__?.opener?.openPath) {
                    const dir = logPath.replace(/[/\\][^/\\]+$/, '');
                    await window.__TAURI__.opener.openPath(dir);
                } else {
                    showSuccess(logPath);
                }
            } catch (e) {
                console.warn('[Debug] get_log_path 失败:', e);
                showError(String(e));
            }
        });
    }

    elements.hotkeyInput.addEventListener('focus', () => {
        elements.hotkeyInput.value = window.i18n.t('hotkeyPrompt');
    });
    elements.hotkeyInput.addEventListener('blur', () => {
        if (!elements.hotkeyInput.dataset.hotkey) {
            elements.hotkeyInput.value = '';
        } else {
            elements.hotkeyInput.value = elements.hotkeyInput.dataset.hotkey;
        }
    });

    // 监听后端断线事件（心跳超时或对端主动断开）
    if (window.__TAURI__) {
        window.__TAURI__.event.listen('connection-lost', (event) => {
            const payload = event.payload || {};
            const message = payload.message || window.i18n.t('connectionLost');
            appState.connected = false;
            appState.connectionType = null;
            setConnectionStatus('disconnected');
            stopLatencyCheck();
            hidePermissionWarning();
            showError(`⚠️ ${message}`);
            // 隐藏手动连接成功状态
            hideManualConnectStatus();
        }).catch(e => console.warn('断线事件监听注册失败:', e));

        // 监听 TLS 指纹验证事件（TOFU 首次连接确认）
        window.__TAURI__.event.listen('tls-verify-fingerprint', async (event) => {
            const { address, fingerprint } = event.payload || {};
            if (!address || !fingerprint) return;
            const trusted = await showTlsConfirmDialog(address, fingerprint);
            if (trusted) {
                // 用户确认信任，保存到后端
                try {
                    await invoke('trust_peer_certificate', { address, fingerprint });
                } catch (e) {
                    console.warn('保存信任记录失败:', e);
                }
            }
        }).catch(e => console.warn('TLS 指纹验证事件监听注册失败:', e));

        // 监听全局快捷键触发事件，手动切换控制权（统一走 toggle_sharing）
        window.__TAURI__.event.listen('hotkey-triggered', async () => {
            try {
                await invoke('toggle_sharing');
                updateUI();
            } catch (e) {
                console.warn('快捷键触发控制权切换失败:', e);
            }
        }).catch(e => console.warn('快捷键事件监听注册失败:', e));

        // 监听控制状态变化事件（后端主动通知）
        // 后端 payload 形如 { from: "Idle", to: "LocalControlling" }
        window.__TAURI__.event.listen('control-state-changed', (event) => {
            const payload = event.payload || {};
            // 兼容旧 payload {state} 与新 payload {from,to}
            const state = payload.to || payload.state || 'Idle';
            updateControlStateUI(state);
            updateSharingButtonForState(state);
            const { t } = window.i18n;
            if (state === 'Armed') {
                showSuccess(t('sharingArmed'));
            } else if (state === 'LocalControlling') {
                showSuccess(t('sharingStarted'));
            } else if (state === 'RemoteControlling') {
                // v0.1.53: 被控端进入 RemoteControlling，显示提示
                showSuccess(t('controlStateRemote'));
            } else if (state === 'Idle') {
                showSuccess(t('sharingStopped'));
            }
        }).catch(e => console.warn('控制状态事件监听注册失败:', e));

        // v0.1.57 StateGuard: 监听完整状态快照（原子更新，消除事件顺序不确定性）
        // 后端 payload: { connected: bool, control_state: string, watchdog_active: bool, timestamp_ms: u64 }
        window.__TAURI__.event.listen('sharing-state-sync', (event) => {
            const snapshot = event.payload || {};
            console.log('[StateGuard] 状态快照:', JSON.stringify(snapshot));

            // 原子更新连接状态
            const wasConnected = appState.connected;
            appState.connected = snapshot.connected;

            // 更新控制状态 UI
            if (snapshot.control_state) {
                updateControlStateUI(snapshot.control_state);
                updateSharingButtonForState(snapshot.control_state);
            }

            // 连接状态变化时更新 UI
            if (wasConnected !== snapshot.connected) {
                setConnectionStatus(snapshot.connected ? 'connected' : 'disconnected');
                updateConnectionButtons();
            }
        }).catch(e => console.warn('sharing-state-sync 事件监听注册失败:', e));

        // BUG-035: 监听 capture 因权限失败 → 显示输入监控权限横幅（非全屏遮罩，允许用户授权后重试）
        window.__TAURI__.event.listen('sharing://capture-failed-permission', () => {
            const { t } = window.i18n;
            console.warn('[KVMini] capture 因输入监控权限失败，显示权限横幅');
            window.AccessibilityModule.showPermissionWarning(elements, 'input-monitoring');
            showError(t('captureFailedPermission'));
        }).catch(e => console.warn('capture-failed-permission 事件监听注册失败:', e));

        // v0.1.43 Bug #035: 监听远端物理输入抢占事件（Synergy/Logitech Flow 风格）
        // 当远端用户在被控端用物理键鼠"抢回"控制权时，后端 emit kvm://preempted，
        // 控制端弹 3 秒 toast 告知用户。被控端不弹。
        window.__TAURI__.event.listen('kvm://preempted', () => {
            const { t } = window.i18n;
            console.info('[KVMini] 远端用户物理输入抢占，控制端回 Armed');
            try {
                // 复用 success-toast（3 秒自动消失，与需求一致）
                showSuccess(t('preempt_resume_toast'));
            } catch (err) {
                console.warn('[KVMini] 显示抢占 toast 失败:', err);
            }
        }).catch(e => console.warn('kvm://preempted 事件监听注册失败:', e));

        // 监听后台更新检查发现新版本事件
        window.__TAURI__.event.listen('update-available', (event) => {
            const payload = event.payload || {};
            handleBackgroundUpdateAvailable(payload);
        }).catch(e => console.warn('更新事件监听注册失败:', e));

        // 监听更新下载进度事件
        window.__TAURI__.event.listen('update-progress', (event) => {
            const payload = event.payload || {};
            console.log('[Updater] 进度:', payload.status, payload.version);
        }).catch(e => console.warn('更新进度事件监听注册失败:', e));

        // 监听连接状态事件（后端主动通知）
        window.__TAURI__.event.listen('connection-status', (event) => {
            const payload = event.payload || {};
            const eventType = payload.event_type || '';
            const connected = payload.connected === true;
            const peerIp = payload.peer_ip || null;
            const message = payload.message || '';
            const attempt = payload.attempt || null;
            const localTls = !!payload.local_tls;
            const remoteTls = !!payload.remote_tls;

            if (eventType === 'listening') {
                // 监听已启动，等待对端连接
                console.log('[KVMini] 监听已启动:', message);

            } else if (eventType === 'connected') {
                // 连接成功
                console.log('[KVMini] connected 事件: localTls=', localTls, 'remoteTls=', remoteTls, 'tls_mismatch=', payload.tls_mismatch, 'payload=', JSON.stringify(payload));
                appState.connected = true;
                appState.connectionType = 'TCP';
                setConnectionStatus('connected');
                hideReconnectStatus();
                if (peerIp && elements.connectedDevice) {
                    elements.connectedDevice.textContent = peerIp;
                }
                showSuccess(message || window.i18n.t('msgConnectedToIp').replace('{ip}', peerIp || ''));
                updateConnectionButtons(true);
                // 更新智能状态面板（v0.1.38 组 F）
                updateSmartStatus({ connected: true, peer_ip: peerIp });
                // 连接成功后检查权限状态，更新权限横幅
                checkPermissionAfterConnect();
                // 更新 TLS 状态面板：只有当 payload 包含有效 TLS 信息时才更新
                // （被动连接时事件循环的 connected 不携带 TLS 信息，等 peer_info 事件）
                if (localTls || remoteTls || payload.tls_mismatch) {
                    updateTlsStatusDisplay(localTls, remoteTls);
                }

            } else if (eventType === 'reconnect_success') {
                // 重连成功
                appState.connected = true;
                appState.connectionType = 'TCP';
                setConnectionStatus('connected');
                hideReconnectStatus();
                if (peerIp && elements.connectedDevice) {
                    elements.connectedDevice.textContent = peerIp;
                }
                showSuccess(message || window.i18n.t('msgReconnectSuccessDefault'));
                updateConnectionButtons(true);
                // 更新智能状态面板
                updateSmartStatus({ connected: true, peer_ip: peerIp });
                // 更新 TLS 状态：同样只有包含有效信息时才更新
                if (localTls || remoteTls || payload.tls_mismatch) {
                    updateTlsStatusDisplay(localTls, remoteTls);
                }

            } else if (eventType === 'disconnected') {
                // 连接断开
                appState.connected = false;
                appState.connectionType = null;
                setConnectionStatus('disconnected');
                hideReconnectStatus();
                if (elements.connectedDevice) {
                    elements.connectedDevice.textContent = '-';
                }
                showReconnectToast(message || window.i18n.t('msgConnectionLostDefault'));
                updateConnectionButtons(false);
                // 清空智能状态面板（徽章/指纹）
                updateSmartStatus({ connected: false });
                // 重置 TLS 状态
                updateTlsStatusDisplay(false, false, true);

            } else if (eventType === 'reconnecting') {
                // 重连中（BUG-032 根因修复 v0.1.40）：
                // 不修改 appState.connected ——TCP socket 仍可能活着，仅心跳波动而已
                // 只有 'disconnected' 事件才是真正断开
                //
                // v0.1.59 BUG-042：reconnecting 不切按钮文案
                // 按钮保持当前 phase（已连接则显示"断开"），只更新状态徽章和 reconnectStatusRow
                // 不调用 setConnectionStatus('connecting')，避免按钮闪到"连接中…"
                if (elements.connectionStatus) {
                    const { t } = window.i18n;
                    elements.connectionStatus.textContent = t('statusConnecting');
                    elements.connectionStatus.className = 'status-connecting';
                }
                showReconnectStatus(message, attempt);
                // 不调 updateConnectionButtons，按钮 phase 维持原状

            } else if (eventType === 'tls_mismatch') {
                // TLS 配置不一致：一端开了一端没开
                const banner = document.getElementById('tls-mismatch-banner');
                const msgEl = document.getElementById('tls-mismatch-msg');
                if (banner && msgEl) {
                    msgEl.textContent = message || window.i18n.t('msgTlsMismatchDefault');
                    banner.classList.remove('hidden');
                }
                // 连接状态区域显示警告图标
                const indicator = document.getElementById('tls-status-indicator');
                if (indicator) {
                    indicator.classList.remove('hidden');
                    indicator.textContent = '⚠️ TLS';
                    indicator.className = 'tls-warning-indicator';
                }
                // 更新 TLS 状态面板
                updateTlsStatusDisplay(localTls, remoteTls);
                console.warn('[TLS] 不一致：', message);

            } else if (eventType === 'peer_info') {
                // 对端握手信息（被动连接时通过此事件获取 TLS 状态）
                console.log('[KVMini] peer_info 事件: localTls=', localTls, 'remoteTls=', remoteTls, 'tls_mismatch=', payload.tls_mismatch);
                updateTlsStatusDisplay(localTls, remoteTls);
                console.log('[KVMini] 对端信息:', message, 'TLS: local=', localTls, 'remote=', remoteTls);

                // 被动连接时，如果 TLS 不一致也显示横幅警告
                if (payload.tls_mismatch) {
                    const banner = document.getElementById('tls-mismatch-banner');
                    const msgEl = document.getElementById('tls-mismatch-msg');
                    if (banner && msgEl) {
                        // 根据本机/对端 TLS 状态生成文案
                        const { t } = window.i18n;
                        let mismatchMsg;
                        if (localTls && !remoteTls) {
                            mismatchMsg = t('msgTlsMismatchLocalOnly');
                        } else {
                            mismatchMsg = t('msgTlsMismatchRemoteOnly');
                        }
                        msgEl.textContent = mismatchMsg || message;
                        banner.classList.remove('hidden');
                    }
                    const indicator = document.getElementById('tls-status-indicator');
                    if (indicator) {
                        indicator.classList.remove('hidden');
                        indicator.textContent = '⚠️ TLS';
                        indicator.className = 'tls-warning-indicator';
                    }
                }

            } else if (eventType === 'error') {
                // 连接错误（不影响重连循环）
                if (!appState.connected) {
                    showError(`⚠️ ${message}`);
                }

            } else {
                // 兼容旧格式
                if (connected) {
                    appState.connected = true;
                    appState.connectionType = 'TCP';
                    setConnectionStatus('connected');
                    if (peerIp && elements.connectedDevice) {
                        elements.connectedDevice.textContent = peerIp;
                    }
                    if (message) showSuccess(message);
                    updateConnectionButtons(true);
                } else {
                    appState.connected = false;
                    appState.connectionType = null;
                    setConnectionStatus('disconnected');
                    if (elements.connectedDevice) {
                        elements.connectedDevice.textContent = '-';
                    }
                    updateConnectionButtons(false);
                }
            }
        }).catch(e => console.warn('连接状态事件监听注册失败:', e));

        // v0.1.60 BUG-046: 监听布局同步事件（被动方布局被自动镜像后触发）
        window.__TAURI__.event.listen('layout-synced', (event) => {
            const payload = event.payload || {};
            const newLayout = payload.new_layout || '';
            const wasChanged = payload.was_changed === true;
            console.log('[KVMini] layout-synced:', newLayout, 'wasChanged:', wasChanged);

            // 更新本地 layout 状态
            if (newLayout) {
                window._currentLayout = newLayout;
                // 刷新布局 UI（如果已初始化）
                if (window.LayoutModule && typeof window.LayoutModule.updateLayoutPreview === 'function') {
                    const preview = document.getElementById('display-layout-preview');
                    const hintEdge = document.getElementById('trigger-edge-text');
                    if (preview && hintEdge) {
                        window.LayoutModule.updateLayoutPreview(newLayout, preview, hintEdge);
                    }
                }
            }

            // 如果布局确实被改变，显示 Toast 通知用户
            if (wasChanged) {
                const { t } = window.i18n;
                showSuccess(t('layoutAutoSynced'));
            }
        }).catch(e => console.warn('布局同步事件监听注册失败:', e));
    }
}

// ─── 本机 IP ───

// 加载并显示本机 IP
async function loadLocalIp() {
    if (!elements.localIp) return;
    // 先显示加载中文案
    elements.localIp.textContent = window.i18n.t('msgLocalIpLoading');
    try {
        const ips = await window.Platform.getLocalIp();
        if (ips && ips.length > 0) {
            elements.localIp.textContent = ips.join(' / ');
            // 成功加载后标记为动态内容，防止 applyI18n() 覆盖 IP 地址
            elements.localIp.setAttribute('data-i18n-dynamic', 'true');
        } else {
            elements.localIp.textContent = window.i18n.t('msgLocalIpFailed');
            elements.localIp.removeAttribute('data-i18n-dynamic');
        }
    } catch (e) {
        elements.localIp.textContent = window.i18n.t('msgLocalIpFailed');
        elements.localIp.removeAttribute('data-i18n-dynamic');
    }
}

// ─── 连接管理 ───

// 处理断开连接
async function handleDisconnect() {
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }
    const { t } = window.i18n;
    try {
        // v0.1.53: 统一调用 disconnect_peer（后端注册的真实断开命令）
        // 它会调用 ConnectionManager.disconnect() → 触发 ConnectionStateChanged(Disconnected) 事件
        // → on_disconnected() → SharingManager 状态机归零、解除键鼠屏蔽
        // 旧的 disconnect_from_mac/disconnect_tcp/disconnect_devices 命令不存在（.catch 静默吞掉了错误）
        await invoke('disconnect_peer').catch((e) => {
            console.warn('[KVMini] disconnect_peer 失败:', e);
        });
        appState.connected = false;
        appState.connectionType = null;
        setConnectionStatus('disconnected');
        hideReconnectStatus();
        stopLatencyCheck();
        hidePermissionWarning();
        showSuccess(t('msgDisconnectSuccess') || t('msgDisconnectManual'));
        updateConnectionButtons(false);
    } catch (error) {
        showError(window.i18n.t('errDisconnectPrefix') + error);
    }
}

// 处理手动重新连接
// BUG-033: 之前调用不存在的 reconnect_to_peer 命令（后端没有注册），
// 改为复用 connect_to_peer 逻辑，从 appState 或连接历史获取上次连接的 IP
async function handleReconnect() {
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }
    const { t } = window.i18n;
    try {
        // 获取上次连接的 IP：优先从连接详情面板、appState、或连接历史
        let peerIp = '';
        let port = appState.defaults?.tcpPort || 23456;

        // 1. 从当前 appState 获取（如果之前连接过，connectedDevice 有值）
        if (elements.connectedDevice && elements.connectedDevice.textContent && elements.connectedDevice.textContent !== '-') {
            peerIp = elements.connectedDevice.textContent.trim();
        }

        // 2. 回退：从连接历史获取最后一条记录
        if (!peerIp) {
            try {
                const hist = await window.Platform.getConnectionHistory();
                if (hist && hist.length > 0) {
                    peerIp = hist[0].ip;
                    port = hist[0].port || port;
                }
            } catch (e) {
                console.warn('[Reconnect] 读取连接历史失败:', e);
            }
        }

        if (!peerIp) {
            showError(t('errReconnectNoTarget') || t('errReconnectFailPrefix') + 'No previous connection found');
            return;
        }

        setConnectionStatus('connecting');
        showReconnectStatus(t('statusConnecting'));
        updateConnectionButtons(false);

        const result = await window.Platform.connectByIp(peerIp, port);
        if (result && (result.success || result.connected)) {
            // 连接成功会通过 connection-status 事件通知
            console.log('[Reconnect] 重连调用成功:', peerIp);
        } else {
            setConnectionStatus('disconnected');
            const errMsg = result?.errorMessage || result?.error_message || t('statusDisconnected');
            showError(t('errReconnectFailPrefix') + errMsg);
            updateConnectionButtons(false);
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError(t('errReconnectFailPrefix') + (error instanceof Error ? error.message : String(error)));
        updateConnectionButtons(false);
    }
}

// v0.1.59 BUG-042: 连接按钮单按钮三态状态机
// 唯一真相源：appState.connected + appState.connectionPhase
// 不再依赖外部传入参数（心跳波动 reconnecting 事件不切按钮文案，只更新状态徽章）
//
// data-state 取值：
//   disconnected → 文案 "连接 / Connect"，可点击触发重连历史
//   connecting   → 文案 "连接中… / Connecting…"，禁用 + spinner
//   connected    → 文案 "断开 / Disconnect"，可点击触发断开
function updateConnectButton(phase) {
    const btn = elements.connectBtn;
    if (!btn) return;
    // 节流：相同状态连续多次调用直接跳过
    const currentState = btn.getAttribute('data-state');
    const targetState = phase || appState.connectionPhase || (appState.connected ? 'connected' : 'disconnected');
    if (currentState === targetState && btn.dataset.lastApplied === targetState) {
        return;
    }
    btn.dataset.lastApplied = targetState;
    btn.setAttribute('data-state', targetState);
    appState.connectionPhase = targetState;

    const { t } = window.i18n || { t: (k) => k };
    let label;
    let cls;
    let disabled = false;
    switch (targetState) {
        case 'connecting':
            label = t('btnConnecting') || 'Connecting…';
            cls = 'btn btn-secondary';
            disabled = true;
            break;
        case 'connected':
            label = t('btnDisconnect') || 'Disconnect';
            cls = 'btn btn-secondary';
            break;
        case 'disconnected':
        default:
            label = t('btnConnect') || 'Connect';
            cls = 'btn btn-primary';
            break;
    }
    btn.textContent = label;
    btn.className = cls;
    btn.disabled = disabled;
    // 移除 hidden（兼容旧逻辑），按钮始终可见
    btn.classList.remove('hidden');
    console.log('[ConnectBtn] state →', targetState, 'label=', label);
}

// 兼容封装：旧调用点全部走新 updateConnectButton
function updateConnectionButtons(_connectedIgnored) {
    const phase = appState.connected ? 'connected' : (appState.connectionPhase === 'connecting' ? 'connecting' : 'disconnected');
    updateConnectButton(phase);
}

// v0.1.59 BUG-042: 单按钮 click 分发器
async function handleConnectButtonClick() {
    const state = appState.connectionPhase || (elements.connectBtn?.getAttribute('data-state')) || 'disconnected';
    console.log('[ConnectBtn] click in state=', state);
    if (state === 'connecting') {
        // 连接中：忽略点击
        return;
    }
    if (state === 'connected') {
        // 已连接：触发断开
        return handleDisconnect();
    }
    // disconnected：尝试用历史 IP 重连
    return handleReconnect();
}

// 显示重连状态（状态面板中）
function showReconnectStatus(message, attempt) {
    if (elements.reconnectStatusRow) {
        elements.reconnectStatusRow.style.display = '';
    }
    if (elements.reconnectStatusText) {
        elements.reconnectStatusText.textContent = message || window.i18n.t('reconnectingText');
    }
    // 重连超过 10 次时显示 Toast 提示
    if (attempt && attempt === 11) {
        showReconnectToast(window.i18n.t('msgReconnectFailRetrying'));
    }
}

// 隐藏重连状态
function hideReconnectStatus() {
    if (elements.reconnectStatusRow) {
        elements.reconnectStatusRow.style.display = 'none';
    }
}

// 显示重连 Toast 提示（自动消失）
let reconnectToastTimer = null;
function showReconnectToast(message) {
    if (!elements.reconnectToast) return;
    // v0.1.60 BUG-045: Toast 互斥 — 弹橙色时关掉绿色和红色
    if (elements.successToast) elements.successToast.classList.add('hidden');
    if (successTimer) { clearTimeout(successTimer); successTimer = null; }
    if (elements.errorToast) elements.errorToast.classList.add('hidden');
    if (errorTimer) { clearTimeout(errorTimer); errorTimer = null; }

    if (elements.reconnectToastMessage) {
        elements.reconnectToastMessage.textContent = message;
    }
    elements.reconnectToast.classList.remove('hidden');
    if (reconnectToastTimer) clearTimeout(reconnectToastTimer);
    reconnectToastTimer = setTimeout(() => {
        if (elements.reconnectToast) elements.reconnectToast.classList.add('hidden');
    }, 4000);
}

// 连接到特定设备
async function connectToDevice(deviceId) {
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }
    const { t } = window.i18n;
    try {
        setConnectionStatus('connecting');
        // BUG-020 修复：后端命令名是 connect_to_peer，参数为 peerIp + port
        const device = appState.devices.find(d => d.id === deviceId);
        const peerIp = device?.ip;
        if (!peerIp) {
            setConnectionStatus('disconnected');
            showError(t('connectErrGeneric').replace('{err}', 'device ip missing'));
            return;
        }
        const port = appState.defaults?.tcpPort || 23456;
        const result = await window.Platform.connectByIp(peerIp, port);
        if (result.success) {
            appState.connected = true;
            appState.connectionType = device?.connectionType || 'Network';
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');
            startLatencyCheck();
            // 保存到历史记录
            if (device) saveToHistory(device);
            showSuccess(t('connectedToIp').replace('{ip}', device?.name || peerIp));
            await checkPermissionAfterConnect();
        } else {
            setConnectionStatus('disconnected');
            const errMsg = result.errorMessage || result.error_message || t('connectErrGeneric').replace('{err}', t('statusDisconnected'));
            showError(t('connectErrGeneric').replace('{err}', errMsg));
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError(t('connectErrGeneric').replace('{err}', error || t('statusDisconnected')));
    }
}

// ─── 手动 IP 连接 ───

/**
 * 校验 IPv4 地址格式（前端校验，与后端保持一致）
 */
function isValidIpv4(ip) {
    const parts = ip.split('.');
    if (parts.length !== 4) return false;
    return parts.every(p => {
        const n = Number(p);
        return /^\d+$/.test(p) && n >= 0 && n <= 255;
    });
}

/**
 * 显示手动连接状态提示
 */
function showManualConnectStatus(type, message) {
    const el = elements.manualConnectStatus;
    el.textContent = message;
    el.className = `manual-connect-status status-${type}`;
    el.classList.remove('hidden');
}

function hideManualConnectStatus() {
    elements.manualConnectStatus.classList.add('hidden');
}

/**
 * 处理手动 IP 连接
 */
async function handleManualConnect() {
    // 需求 3.1：invoke 为 null 时显示错误提示
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }

    const ip = elements.manualIpInput.value.trim();
    const port = parseInt(elements.manualPortInput.value, 10) || appState.defaults?.tcpPort || 23456;
    const { t } = window.i18n;

    // 前端 IPv4 格式校验
    if (!ip) {
        elements.manualIpInput.classList.add('input-error');
        showManualConnectStatus('error', t('errIpEmpty'));
        elements.manualIpInput.focus();
        return;
    }
    if (!isValidIpv4(ip)) {
        elements.manualIpInput.classList.add('input-error');
        showManualConnectStatus('error', t('errIpFormat').replace('{ip}', ip));
        elements.manualIpInput.focus();
        return;
    }
    elements.manualIpInput.classList.remove('input-error');

    // 端口范围校验
    if (port < 1024 || port > 65535) {
        showManualConnectStatus('error', t('errPortRange'));
        elements.manualPortInput.focus();
        return;
    }

    // 开始连接
    elements.manualConnectBtn.disabled = true;
    showManualConnectStatus('connecting', t('connectingTo').replace('{ip}', ip).replace('{port}', port));

    try {
        const result = await window.Platform.connectByIp(ip, port);
        if (result.success) {
            appState.connected = true;
            appState.connectionType = 'Network';
            appState.currentDevice = `manual:${ip}:${port}`;
            setConnectionStatus('connected');
            startLatencyCheck();
            const platformInfo = result.remote_platform ? ` (${result.remote_platform})` : '';
            showManualConnectStatus('success', t('connectedTo').replace('{ip}', ip).replace('{port}', port) + platformInfo);
            showSuccess(t('connectedToIp').replace('{ip}', ip));
            // 保存到历史记录（手动 IP 类型）
            saveManualIpToHistory(ip, port);
            // 启动心跳监控（连接成功后）
            try {
                await invoke('start_heartbeat_monitor');
            } catch (e) {
                console.warn('心跳监控启动失败（不影响连接）:', e);
            }
            await checkPermissionAfterConnect();
        } else {
            // 根据错误类型显示不同提示
            let errorMsg = result.error_message || result.errorMessage || t('errIpEmpty');
            if (result.error_type === 'handshake_failed') {
                errorMsg = t('errHandshakeFailed');
            } else if (result.error_type === 'version_mismatch') {
                errorMsg = t('errVersionMismatch');
            } else if (result.error_type === 'timeout') {
                errorMsg = t('errTimeout').replace('{ip}', ip);
            } else if (result.error_type === 'refused') {
                errorMsg = t('errRefused');
            }
            showManualConnectStatus('error', errorMsg);
            setConnectionStatus('disconnected');
        }
    } catch (e) {
        showManualConnectStatus('error', t('connectErrGeneric').replace('{err}', e));
        setConnectionStatus('disconnected');
    } finally {
        elements.manualConnectBtn.disabled = false;
    }
}

// ─── 设备扫描 ───

// 处理扫描按鈕
async function handleScan() {
    if (appState.scanning) return;
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }

    appState.scanning = true;
    setScanningState(true);

    // 提示用户两阶段扫描（UDP广播 + 子网TCP扫描）可能需要几秒
    const { t } = window.i18n;
    if (elements.scanBtn) {
        elements.scanBtn.title = t('scanningSubnetTip');
    }

    try {
        const devices = await invoke('discover_devices');
        if (devices.length === 0) {
            // 扫描成功但未发现设备，执行网络诊断
            updateDevicesList([]);
            try {
                const diagnostics = await invoke('diagnose_network');
                showNetworkDiagnostics(diagnostics);
            } catch (diagErr) {
                console.warn('[Scan] 网络诊断失败:', diagErr);
                // 诊断失败时显示静态提示
                showSuccess(t('noDevicesAfterScan'));
            }
        } else {
            updateDevicesList(devices);
            showSuccess(window.i18n.t('scanFoundDevices').replace('{n}', String(devices.length)));
        }
    } catch (error) {
        console.error('[Scan] discover_devices 调用失败:', error);
        showError(window.i18n.t('scanFailed'));
        updateDevicesList([]);
    } finally {
        appState.scanning = false;
        setScanningState(false);
        if (elements.scanBtn) elements.scanBtn.title = '';
    }
}

/**
 * 格式化并展示网络诊断结果
 * @param {Array<{check: string, ok: boolean, message: string}>} diagnostics
 */
function showNetworkDiagnostics(diagnostics) {
    const { t } = window.i18n;
    if (!diagnostics || diagnostics.length === 0) {
        showSuccess(t('noDevicesAfterScan'));
        return;
    }

    // 统计问题数量
    const issues = diagnostics.filter(d => !d.ok);
    const lines = [];

    // 标题
    lines.push(t('diagTitle'));
    lines.push('');

    // 逐项展示
    for (const item of diagnostics) {
        const icon = item.ok ? '✅' : '⚠️';
        lines.push(`${icon} ${item.message}`);
    }

    // 如果有问题，给出建议
    if (issues.length > 0) {
        lines.push('');
        lines.push(t('diagSuggestion'));
    } else {
        lines.push('');
        lines.push(t('diagAllPass'));
    }

    // 使用 devices-list 区域展示诊断结果
    if (elements.devicesList) {
        const html = `<div class="network-diagnostic">
            ${lines.map(line => line ? `<div class="diag-line">${escapeHtml(line)}</div>` : '<div class="diag-spacer"></div>').join('')}
        </div>`;
        elements.devicesList.innerHTML = html;
    }
}
// 设置扫描状态 UI
function setScanningState(scanning) {
    const { t } = window.i18n;
    elements.scanBtn.classList.toggle('scanning', scanning);
    elements.scanBtn.disabled = scanning;
    elements.scanIcon.textContent = scanning ? '⏳' : '🔍';
    elements.scanText.textContent = scanning ? t('scanningText') : t('btnScan');
    elements.scanAnimation.classList.toggle('hidden', !scanning);
}

// 更新设备列表
function updateDevicesList(devices) {
    appState.devices = devices;
    const { t } = window.i18n;

    if (devices.length === 0) {
        elements.devicesList.innerHTML = `<div class="no-devices">${t('noDevicesFound')}</div>`;
        return;
    }

    elements.devicesList.innerHTML = devices.map(device => `
        <div class="device-item">
            <div class="device-info">
                <div class="device-name">${escapeHtml(device.name)}</div>
                <div class="device-details">
                    ${escapeHtml(device.os)} | ${escapeHtml(device.ip)} | ${escapeHtml(device.connectionType)}
                </div>
            </div>
            <button class="connect-device-btn" onclick="connectToDevice('${escapeHtml(device.id)}')">
                ${t('btnConnect')}
            </button>
        </div>
    `).join('');
}

// ─── 连接历史 ───

// 保存到历史记录（优先使用 Rust 后端，降级到 localStorage）
async function saveToHistory(device) {
    const entry = {
        id: device.id,
        name: device.name,
        os: device.os,
        ip: device.ip,
        connectionType: device.connectionType,
        entryType: 'auto',  // 自动发现
        lastConnected: new Date().toISOString(),
    };

    // 尝试使用 Rust 后端持久化
    try {
        await window.Platform.saveConnectionHistory(entry);
    } catch (e) {
        console.warn('后端历史保存失败，降级到 localStorage:', e);
    }

    // 同时更新 localStorage（作为缓存）
    const history = getHistoryFromStorage();
    const filtered = history.filter(h => h.id !== device.id);
    filtered.unshift(entry);
    const trimmed = filtered.slice(0, 20);
    localStorage.setItem('kvm_connection_history', JSON.stringify(trimmed));
    renderHistoryList(trimmed);
}

/**
 * 保存手动 IP 连接到历史记录
 */
async function saveManualIpToHistory(ip, port) {
    const id = `manual:${ip}:${port}`;
    const entry = {
        id,
        name: ip,
        os: '',
        ip,
        connectionType: 'Network',
        entryType: 'manual',  // 手动 IP
        port,
        lastConnected: new Date().toISOString(),
    };

    try {
        await window.Platform.saveConnectionHistory(entry);
    } catch (e) {
        console.warn('后端历史保存失败，降级到 localStorage:', e);
    }

    const history = getHistoryFromStorage();
    const filtered = history.filter(h => h.id !== id);
    filtered.unshift(entry);
    const trimmed = filtered.slice(0, 20);
    localStorage.setItem('kvm_connection_history', JSON.stringify(trimmed));
    renderHistoryList(trimmed);
}

// 从本地存储读取历史
function getHistoryFromStorage() {
    try {
        return JSON.parse(localStorage.getItem('kvm_connection_history') || '[]');
    } catch {
        return [];
    }
}

// 加载并渲染历史记录（优先使用 Rust 后端，降级到 localStorage）
async function loadConnectionHistory() {
    let history = [];
    try {
        history = await window.Platform.getConnectionHistory();
        if (history && history.length > 0) {
            // 同步到 localStorage
            localStorage.setItem('kvm_connection_history', JSON.stringify(history));
        } else {
            history = getHistoryFromStorage();
        }
    } catch (e) {
        history = getHistoryFromStorage();
    }
    renderHistoryList(history);
}

// 渲染历史列表
function renderHistoryList(history) {
    const { t } = window.i18n;
    if (history.length === 0) {
        elements.historyList.innerHTML = `<div class="no-history">${t('noHistory')}</div>`;
        return;
    }

    elements.historyList.innerHTML = history.map(item => {
        const date = new Date(item.lastConnected);
        const dateStr = `${date.getMonth() + 1}/${date.getDate()} ${date.getHours()}:${String(date.getMinutes()).padStart(2, '0')}`;
        // 区分手动 IP 和自动发现设备
        const isManual = item.entryType === 'manual';
        const icon = isManual ? '🔗' : '📡';
        const typeLabel = isManual ? t('manualIpLabel') : (item.connectionType || 'Network');
        const reconnectAttr = isManual
            ? `onclick="reconnectManualIp('${escapeHtml(item.ip)}', ${item.port || appState.defaults?.tcpPort || 23456})"`
            : `onclick="reconnectToDevice('${escapeHtml(item.id)}', '${escapeHtml(item.name)}')"` ;
        return `
            <div class="history-item">
                <div class="history-info">
                    <div class="history-device-name">${icon} ${escapeHtml(item.name)}</div>
                    <div class="history-meta">${escapeHtml(item.ip)} · ${escapeHtml(typeLabel)} · ${dateStr}</div>
                </div>
                <button class="reconnect-btn" ${reconnectAttr}>
                    ${t('btnReconnect')}
                </button>
            </div>
        `;
    }).join('');
}
// 快速重连（自动发现设备）
async function reconnectToDevice(deviceId, deviceName) {
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }
    const { t } = window.i18n;
    try {
        setConnectionStatus('connecting');
        // BUG-020 修复：统一走 connect_to_peer；优先从历史记录找 IP
        const hist = await window.Platform.getConnectionHistory();
        const record = (hist || []).find(h => h.id === deviceId);
        const peerIp = record?.ip;
        if (!peerIp) {
            setConnectionStatus('disconnected');
            showError(t('connectErrGeneric').replace('{err}', 'device ip missing'));
            return;
        }
        const port = record?.port || appState.defaults?.tcpPort || 23456;
        const result = await window.Platform.connectByIp(peerIp, port);
        if (result.success) {
            appState.connected = true;
            appState.connectionType = record?.connectionType || 'Network';
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');
            startLatencyCheck();
            showSuccess(t('reconnectedToIp').replace('{ip}', deviceName));
        } else {
            setConnectionStatus('disconnected');
            const errMsg = result.errorMessage || result.error_message || t('connectErrGeneric').replace('{err}', t('statusDisconnected'));
            showError(t('connectErrGeneric').replace('{err}', errMsg));
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError(t('connectErrGeneric').replace('{err}', error || t('statusDisconnected')));
    }
}

/**
 * 手动 IP 历史记录快速重连
 */
async function reconnectManualIp(ip, port) {
    // 填充手动连接输入框并触发连接
    elements.manualIpInput.value = ip;
    elements.manualPortInput.value = port || appState.defaults?.tcpPort || 23456;
    await handleManualConnect();
}

// 清除历史记录
async function handleClearHistory() {
    try {
        await window.Platform.clearConnectionHistory();
    } catch (e) {
        console.warn('后端历史清除失败:', e);
    }
    localStorage.removeItem('kvm_connection_history');
    renderHistoryList([]);
}

// ─── 设置面板 ───

// 打开设置
function openSettings() {
    console.log('[KVMini] openSettings: 切换到设置视图');
    loadSettingsToUI();
    loadListenConfig();
    loadLocalTlsFingerprint();
    // 视图切换：隐藏主界面，显示设置页
    elements.viewMain.classList.add('hidden');
    elements.viewSettings.classList.remove('hidden');
    // 初始化分页模块（首次打开时）
    if (window.SettingsTabsModule) {
        window.SettingsTabsModule.initSettingsTabs();
    }
    // 打开设置时自动刷新显示器列表
    refreshMonitors();
    // 初始化关于 Tab
    initAboutTab();
}

// 关闭设置
function closeSettings() {
    // 有未保存修改时弹出自定义确认对话框
    if (window.SettingsTabsModule && window.SettingsTabsModule.hasDirty()) {
        const dialog = document.getElementById('settings-discard-dialog');
        if (dialog) {
            dialog.classList.remove('hidden');
            return; // 不直接关闭，等待用户确认
        }
    }
    _doCloseSettings();
}

// 实际关闭设置页（内部函数）：隐藏设置视图，显示主界面
function _doCloseSettings() {
    console.log('[KVMini] _doCloseSettings: 切换回主界面');
    if (window.SettingsTabsModule) {
        window.SettingsTabsModule.clearAllDirty();
    }
    // 视图切换：隐藏设置页，显示主界面
    elements.viewSettings.classList.add('hidden');
    elements.viewMain.classList.remove('hidden');
}

// 取消确认对话框：用户选择"放弃修改"
function onSettingsDiscardConfirm() {
    const dialog = document.getElementById('settings-discard-dialog');
    if (dialog) dialog.classList.add('hidden');
    // 恢复原始设置到 UI
    loadSettingsToUI();
    _doCloseSettings();
}

// 取消确认对话框：用户选择"继续编辑"
function onSettingsDiscardCancel() {
    const dialog = document.getElementById('settings-discard-dialog');
    if (dialog) dialog.classList.add('hidden');
    // 保持设置页打开，不做任何修改
}

// ─── TLS 指纹相关 ────────────────────────────────────────────────────────────

/** 加载本机 TLS 证书指纹并显示在设置页 */
async function loadLocalTlsFingerprint() {
    if (!elements.localTlsFingerprint || !invoke) return;
    const { t } = window.i18n;
    const fingerprintRow = document.getElementById('tls-fingerprint-row');
    const fingerprintHint = document.getElementById('tls-fingerprint-hint');
    const certErrorRow = document.getElementById('cert-error-row');
    const certErrorMsg = document.getElementById('cert-error-msg');
    const copyBtn = fingerprintRow ? fingerprintRow.querySelector('.btn-small') : null;

    // 检查 TLS 是否开启
    const tlsEnabled = elements.enableTls ? elements.enableTls.checked : true;

    if (!tlsEnabled) {
        // TLS 关闭时：显示灰色提示
        elements.localTlsFingerprint.textContent = t('tlsDisabled');
        elements.localTlsFingerprint.style.color = '#9ca3af';
        elements.localTlsFingerprint.title = '';
        if (copyBtn) copyBtn.disabled = true;
        if (fingerprintHint) fingerprintHint.style.color = '#9ca3af';
        if (certErrorRow) certErrorRow.classList.add('hidden');
        return;
    }

    // TLS 开启时：正常加载指纹
    if (copyBtn) copyBtn.disabled = false;
    if (fingerprintHint) fingerprintHint.style.color = '';
    elements.localTlsFingerprint.style.color = '';
    try {
        const fp = await invoke('get_local_tls_fingerprint');
        // 只显示前 47 个字符，完整指纹放 title
        elements.localTlsFingerprint.textContent = fp ? fp.substring(0, 47) + '...' : '-';
        elements.localTlsFingerprint.title = fp || '';
        // 加载成功：隐藏错误行
        if (certErrorRow) certErrorRow.classList.add('hidden');
    } catch (e) {
        elements.localTlsFingerprint.textContent = t('tlsFingerprintLoadFailed');
        // 加载失败：显示错误行和重新生成按钮
        if (certErrorRow) {
            if (certErrorMsg) certErrorMsg.textContent = e instanceof Error ? e.message : String(e);
            certErrorRow.classList.remove('hidden');
        }
        console.warn('[TLS] 证书指纹加载失败:', e);
    }
}

/** 重新生成 TLS 证书 */
async function regenerateCert() {
    if (!invoke) return;
    const { t } = window.i18n;
    const certErrorRow = document.getElementById('cert-error-row');
    try {
        const fp = await invoke('regenerate_tls_cert');
        if (elements.localTlsFingerprint) {
            elements.localTlsFingerprint.textContent = fp ? fp.substring(0, 47) + '...' : '-';
            elements.localTlsFingerprint.title = fp || '';
        }
        if (certErrorRow) certErrorRow.classList.add('hidden');
        showToast(t('certRegenSuccess'));
    } catch (e) {
        showToast(t('certRegenFailed'));
    }
}

/** 复制本机 TLS 指纹到剪贴板 */
async function copyTlsFingerprint() {
    if (!invoke) return;
    try {
        const fp = await invoke('get_local_tls_fingerprint');
        if (fp) {
            await navigator.clipboard.writeText(fp);
            showSuccess(window.i18n.t('tlsFingerprintCopied'));
        }
    } catch (e) {
        showError(window.i18n.t('tlsFingerprintLoadFailed'));
    }
}

/**
 * TLS 不一致时：关闭本机 TLS 并重试连接
 * 由 tls-mismatch-banner 中的"关闭本机 TLS 重试"按钮调用
 */
async function tlsDisableAndRetry() {
    if (!invoke) return;
    try {
        // 1. 关闭本机 TLS 开关
        if (elements.enableTls) elements.enableTls.checked = false;

        // 2. 保存设置（enable_tls = false）— 同步 camelCase 和 snake_case
        appState.settings.enableTls = false;
        appState.settings.enable_tls = false;
        const backendPayload = {
            peer_ip: appState.settings.peer_ip || appState.settings.peerIp || '',
            port: appState.settings.port || appState.defaults?.tcpPort || 23456,
            switch_hotkey: appState.settings.hotkey || appState.settings.switch_hotkey || 'Ctrl+Alt+S',
            edge_trigger: appState.settings.edgeTrigger !== false,
            edge_threshold: parseInt(appState.settings.edgeThreshold || appState.settings.edge_threshold, 10) || 5,
            clipboard_sync: appState.settings.clipboardSync !== false,
            autostart: appState.settings.autoStart === true,
            enable_tls: false,
            auto_reconnect: appState.settings.autoReconnect !== false,
        };
        await invoke('save_settings', { config: backendPayload });
        localStorage.setItem('kvm_settings', JSON.stringify(appState.settings));

        // 3. 隐藏 banner 和警告图标
        const banner = document.getElementById('tls-mismatch-banner');
        if (banner) banner.classList.add('hidden');
        const indicator = document.getElementById('tls-status-indicator');
        if (indicator) indicator.classList.add('hidden');

        // 4. 触发重连
        log.info('[TLS] 已关闭本机 TLS，正在重新连接...');
        await invoke('connect_to_peer', { peerIp: appState.settings.peer_ip });
    } catch (e) {
        log.error('[TLS] 关闭 TLS 重试失败:', e);
        showError(window.i18n.t('errDisableTlsRetryPrefix') + (e instanceof Error ? e.message : String(e)));
    }
}

/**
 * 显示 TLS 指纹确认对话框（首次连接时调用）
 * @param {string} address - 对端地址 IP:Port
 * @param {string} fingerprint - 对端证书指纹
 * @returns {Promise<boolean>} 用户是否信任
 */
function showTlsConfirmDialog(address, fingerprint) {
    return new Promise((resolve) => {
        const modal = document.getElementById('tls-confirm-modal');
        const fpEl = document.getElementById('tls-confirm-fingerprint');
        const addrEl = document.getElementById('tls-confirm-address');
        const trustBtn = document.getElementById('tls-trust-btn');
        const rejectBtn = document.getElementById('tls-reject-btn');
        if (!modal || !fpEl) { resolve(false); return; }

        // 只显示前 47 个字符，完整指纹放 title
        fpEl.textContent = fingerprint ? fingerprint.substring(0, 47) + '...' : fingerprint;
        fpEl.title = fingerprint || '';
        if (addrEl) addrEl.textContent = window.i18n.t('peerAddressPrefix') + address;

        modal.classList.remove('hidden');

        const cleanup = () => modal.classList.add('hidden');

        trustBtn.onclick = async () => {
            cleanup();
            if (invoke) {
                try {
                    await invoke('trust_peer_certificate', { address, fingerprint });
                } catch (e) {
                    console.warn('保存信任记录失败:', e);
                }
            }
            resolve(true);
        };
        rejectBtn.onclick = () => { cleanup(); resolve(false); };
    });
}

// 将设置加载到 UI
const DEFAULT_HOTKEY = 'Ctrl+Alt+S';
function loadSettingsToUI() {
    // 修复：hotkey 为空时显示默认值 Ctrl+Alt+S
    const hotkey = appState.settings.hotkey || DEFAULT_HOTKEY;
    elements.hotkeyInput.value = hotkey;
    elements.hotkeyInput.dataset.hotkey = appState.settings.hotkey || '';
    elements.preferConnectionType.value = appState.settings.preferConnectionType;
    elements.autoReconnect.checked = appState.settings.autoReconnect;
    elements.autoStart.checked = appState.settings.autoStart;
    elements.startMinimized.checked = appState.settings.startMinimized;
    // 剪贴板共享
    if (elements.clipboardSync) {
        elements.clipboardSync.checked = appState.settings.clipboardSync !== false;
    }
    // 键位映射统一模式（v0.1.58）
    if (elements.keyMappingUnified) {
        elements.keyMappingUnified.checked = appState.settings.keyMappingUnified !== false;
    }
    // 键鼠切换（plan v0.1.38 BUG-022）：兼容两种键名
    //  - appState.settings.edgeTrigger / edgeThreshold（前端 camelCase）
    //  - appState.settings.edge_trigger / edge_threshold（后端 snake_case）
    if (elements.edgeTriggerToggle) {
        const et = (appState.settings.edgeTrigger !== undefined)
            ? appState.settings.edgeTrigger
            : (appState.settings.edge_trigger !== undefined ? appState.settings.edge_trigger : true);
        elements.edgeTriggerToggle.checked = et !== false;
    }
    if (elements.edgeThresholdInput) {
        const threshold = (appState.settings.edgeThreshold !== undefined)
            ? appState.settings.edgeThreshold
            : (appState.settings.edge_threshold !== undefined ? appState.settings.edge_threshold : 5);
        const clamped = Math.max(1, Math.min(50, parseInt(threshold, 10) || 5));
        elements.edgeThresholdInput.value = String(clamped);
    }
    // TLS 加密（兼容 camelCase/snake_case，后端配置为准）
    if (elements.enableTls) {
        const tlsVal = (appState.settings.enableTls !== undefined)
            ? appState.settings.enableTls
            : (appState.settings.enable_tls !== undefined ? appState.settings.enable_tls : true);
        elements.enableTls.checked = tlsVal !== false;
    }
    // 外观设置
    if (elements.languageSelect) {
        elements.languageSelect.value = appState.settings.language || 'zh';
    }
    if (elements.themeSelect) {
        elements.themeSelect.value = appState.settings.theme || 'auto';
    }
    // 加载本机 TLS 指纹
    loadLocalTlsFingerprint();
}

// 保存设置
async function saveSettings() {
    const enableTls = elements.enableTls ? elements.enableTls.checked : true;
    // TLS 关闭时显示安全警告
    if (!enableTls) {
        showError(window.i18n.t('tlsDisabledWarning'));
    }
    appState.settings = {
        hotkey: elements.hotkeyInput.dataset.hotkey || '',
        preferConnectionType: elements.preferConnectionType.value,
        autoReconnect: elements.autoReconnect.checked,
        autoStart: elements.autoStart.checked,
        startMinimized: elements.startMinimized.checked,
        clipboardSync: elements.clipboardSync ? elements.clipboardSync.checked : true,
        keyMappingUnified: elements.keyMappingUnified ? elements.keyMappingUnified.checked : true,
        // 键鼠切换（plan v0.1.38 BUG-022）：camelCase 供前端消费
        edgeTrigger: elements.edgeTriggerToggle ? elements.edgeTriggerToggle.checked : true,
        edgeThreshold: elements.edgeThresholdInput
            ? Math.max(1, Math.min(50, parseInt(elements.edgeThresholdInput.value, 10) || 5))
            : 5,
        enableTls,
        language: elements.languageSelect ? elements.languageSelect.value : (appState.settings.language || 'zh'),
        theme: elements.themeSelect ? elements.themeSelect.value : (appState.settings.theme || 'auto'),
    };
    localStorage.setItem('kvm_settings', JSON.stringify(appState.settings));

    // 立即应用主题（保存后界面立即生效）
    if (window.themeManager) {
        window.themeManager.setTheme(appState.settings.theme || 'auto');
    }

    // 通过 tauri-commerce SDK 切换语言（Rust 后端持久化 + 前端同步）
    if (invoke) {
        try {
            await invoke('set_locale', { locale: appState.settings.language || 'zh' });
            console.log(`[i18n] 语言已切换为: ${appState.settings.language}`);
        } catch (e) {
            console.warn('[i18n] SDK set_locale 调用失败，降级到本地切换:', e);
        }
    }
    // 同步前端 i18n 状态并重新渲染
    if (window.i18n) {
        window.i18n.setLocale(appState.settings.language || 'zh');
        applyI18n();
    }

    // 如果 Tauri 可用，同步到后端
    if (invoke) {
        try {
            // 后端 AppConfig 使用 snake_case 字段（kvm-core::config::KvmConfig 同名），
            // 前端 appState.settings 是 camelCase，显式构造 snake_case payload 以便正确反序列化。
            // 未纳入映射的字段 serde 默认值兜底（见 src-tauri/src/config.rs）。
            const backendPayload = {
                peer_ip: appState.settings.peerIp || '',
                port: appState.defaults?.tcpPort || 23456,
                switch_hotkey: appState.settings.hotkey || 'Ctrl+Alt+S',
                edge_trigger: appState.settings.edgeTrigger !== false,
                edge_threshold: Math.max(1, Math.min(50,
                    parseInt(appState.settings.edgeThreshold, 10) || 5)),
                clipboard_sync: appState.settings.clipboardSync !== false,
                key_mapping_unified: appState.settings.keyMappingUnified !== false,
                autostart: appState.settings.autoStart === true,
                enable_tls: appState.settings.enableTls !== false,
                auto_reconnect: appState.settings.autoReconnect !== false,
            };
            await invoke('save_settings', { config: backendPayload });
            // v0.1.58: 同步运行时剪贴板开关和键位映射设置（无需重启）
            try {
                await invoke('apply_sharing_settings', {
                    clipboardSync: appState.settings.clipboardSync !== false,
                    keyMappingUnified: appState.settings.keyMappingUnified !== false,
                });
            } catch (e) {
                console.warn('[Settings] apply_sharing_settings 失败（可能未连接）:', e);
            }
        } catch (e) {
            // 显示后端错误（如开机自启动注册失败）
            showError(window.i18n.t('errSettingsSavePrefix') + e);
            return;
        }

        // 保存监听地址配置
        if (elements.listenIpSelect && elements.listenPortInput) {
            const listenIp = elements.listenIpSelect.value || '0.0.0.0';
            const listenPort = parseInt(elements.listenPortInput.value, 10) || appState.defaults?.tcpPort || 23456;
            if (listenPort >= 1024 && listenPort <= 65535) {
                try {
                    await window.Platform.setListenAddress(listenIp, listenPort);
                    // plan v0.1.38：保存后同步更新 appState.defaults.tcpPort，
                    // 让连接对话框、手动连接端口框默认值立即反映新值。
                    appState.defaults = { ...(appState.defaults || {}), tcpPort: listenPort };
                    if (elements.manualPortInput && !elements.manualPortInput.value) {
                        elements.manualPortInput.value = listenPort;
                    }
                } catch (e) {
                    console.warn('保存监听配置失败:', e);
                }
            } else {
                showError(window.i18n.t('errPortRange'));
                return;
            }
        }

        // 保存显示器布局配置
        await saveLayoutConfig();

        // 注册全局快捷键（先注销旧的，再注册新的）
        const newHotkey = appState.settings.hotkey || '';
        try {
            await invoke('register_global_hotkey', { hotkey: newHotkey });
            if (newHotkey) {
                log.debug && log.debug(`全局快捷键已注册: ${newHotkey}`);
            }
        } catch (e) {
            // 快捷键冲突或注册失败，显示提示但不阻止保存
            showError(window.i18n.t('errHotkeyRegisterPrefix') + e);
        }
    }

    // 保存设置
    // 清除未保存标记
    if (window.SettingsTabsModule) {
        window.SettingsTabsModule.clearAllDirty();
    }
    closeSettings();
    showSuccess(window.i18n.t('msgSettingsSaved'));
}

// 加载设置
function loadSettings() {
    try {
        const saved = JSON.parse(localStorage.getItem('kvm_settings') || 'null');
        if (saved) {
            appState.settings = { ...appState.settings, ...saved };
        }
    } catch {
        // 忽略解析错误
    }
}

/**
 * BUG-021 修复：启动时从后端 get_settings 拉取默认端口，填入 appState.defaults.tcpPort。
 * 同步把默认端口回填到 manual-port-input / listen-port-input 两个输入框，
 * 避免用户看到空输入框后误以为端口丢失。
 * 兼容旧配置：tcp_port == 0 或缺失时回退到 23456（kvm-core 真源值）。
 */
async function loadBackendDefaults() {
    let port = 23456;
    try {
        const settings = await window.Platform.getSettings();
        if (settings && Number.isInteger(settings.port) && settings.port >= 1024 && settings.port <= 65535) {
            port = settings.port;
        } else {
            console.warn('[KVMini] 后端返回端口无效，使用默认 23456:', settings?.port);
        }
        // ▶ BUG-FIX: 从后端同步 enable_tls 到前端，防止 localStorage 与后端配置不一致
        // 后端配置文件是唯一真相源（snake_case），同步到前端 appState（camelCase + snake_case）
        if (settings && typeof settings.enable_tls === 'boolean') {
            const backendTls = settings.enable_tls;
            appState.settings.enableTls = backendTls;
            appState.settings.enable_tls = backendTls;
            console.log('[KVMini] 从后端同步 TLS 状态: enable_tls =', backendTls);
            // 同步更新 UI 开关（如果已渲染）
            if (elements.enableTls) {
                elements.enableTls.checked = backendTls;
            }
            // 同步 localStorage
            localStorage.setItem('kvm_settings', JSON.stringify(appState.settings));
        }
    } catch (e) {
        console.warn('[KVMini] get_settings 失败，使用默认端口 23456:', e);
    }
    appState.defaults = { ...(appState.defaults || {}), tcpPort: port };
    // 回填 UI 输入框（仅在空时填，避免覆盖用户正在输入的内容）
    if (elements.manualPortInput && !elements.manualPortInput.value) {
        elements.manualPortInput.value = port;
    }
    if (elements.listenPortInput && !elements.listenPortInput.value) {
        elements.listenPortInput.value = port;
    }
    console.log('[KVMini] appState.defaults.tcpPort =', port);
    console.log('[KVMini] loadBackendDefaults 完成: appState.settings.enableTls =', appState.settings.enableTls, 'appState.settings.enable_tls =', appState.settings.enable_tls);
}

// ─── 外观（语言 + 主题）初始化 ───────────────────────────────────────────

/**
 * 初始化外观：从已保存设置中读取语言和主题并立即应用
 * 语言优先从 tauri-commerce SDK get_locale 读取（Rust 后端持久化），
 * 降级到 appState.settings.language（localStorage）
 * 在 loadSettings() 之后调用
 */
async function initAppearance() {
    let lang = appState.settings.language || 'zh';
    const theme = appState.settings.theme || 'auto';

    // 通过 SDK get_locale 读取已保存语言（Rust 后端持久化）
    if (invoke) {
        try {
            const sdkLocale = await invoke('get_locale');
            if (sdkLocale && (sdkLocale === 'zh' || sdkLocale === 'en')) {
                lang = sdkLocale;
                // 同步到 appState
                appState.settings.language = lang;
                console.log(`[i18n] SDK get_locale 返回: ${lang}`);
            }
        } catch (e) {
            console.warn('[i18n] SDK get_locale 调用失败，使用本地设置:', e);
        }
    }

    // 应用语言
    window.i18n.setLocale(lang);
    applyI18n();

    // 应用主题（防止闪烁，已在 <head> 内联脚本中提前设置，这里同步模块状态）
    window.themeManager.setTheme(theme);
}

/**
 * 将 data-i18n 属性的元素文本全部替换为当前语言文本
 * 同时更新动态生成的文本（状态、按钮等）
 */
function applyI18n() {
    const { t } = window.i18n;

    // 批量替换带 data-i18n 属性的元素
    // 跳过 data-i18n-dynamic="true" 的元素（其文案由 JS 动态设置，如 IP 地址）
    document.querySelectorAll('[data-i18n]').forEach(el => {
        if (el.getAttribute('data-i18n-dynamic') === 'true') return;
        const key = el.getAttribute('data-i18n');
        el.textContent = t(key);
    });

    // 更新 placeholder
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        const key = el.getAttribute('data-i18n-placeholder');
        el.placeholder = t(key);
    });

    // 更新页面标题
    document.title = t('pageTitle');

    // 更新动态状态文本（已渲染到 DOM 的）
    if (elements.connectionStatus) {
        const cls = elements.connectionStatus.className;
        if (cls.includes('status-connected')) {
            elements.connectionStatus.textContent = t('statusConnected');
        } else if (cls.includes('status-connecting')) {
            elements.connectionStatus.textContent = t('statusConnecting');
        } else {
            elements.connectionStatus.textContent = t('statusDisconnected');
        }
    }

    // 更新控制状态文本
    if (elements.controlState) {
        const cls = elements.controlState.className;
        if (cls.includes('control-state-local')) {
            elements.controlState.textContent = t('controlStateLocal');
        } else if (cls.includes('control-state-remote')) {
            elements.controlState.textContent = t('controlStateRemote');
        } else {
            elements.controlState.textContent = t('controlStateIdle');
        }
    }

    // 更新设备列表空状态文本
    const noDevicesEl = document.querySelector('.no-devices');
    if (noDevicesEl) noDevicesEl.textContent = t('noDevices');

    // 更新历史列表空状态文本
    const noHistoryEl = document.querySelector('.no-history');
    if (noHistoryEl) noHistoryEl.textContent = t('noHistory');

    // 更新权限警告文本（BUG-031：按 data-i18n 动态 key 刷新，兼容 accessibility/input-monitoring 两种 kind）
    const permWarnText = document.querySelector('.permission-warning-text');
    if (permWarnText) {
        const key = permWarnText.getAttribute('data-i18n') || 'permissionWarningText';
        permWarnText.textContent = t(key);
    }
    const permWarnBtn = document.getElementById('permission-warning-btn');
    if (permWarnBtn) {
        const btnKey = permWarnBtn.getAttribute('data-i18n') || 'permissionWarningBtnAccessibility';
        permWarnBtn.textContent = t(btnKey);
    }

    // 更新语言选择器选项文本（确保选中项文本与当前语言一致）
    if (elements.languageSelect) {
        const zhOption = elements.languageSelect.querySelector('option[value="zh"]');
        const enOption = elements.languageSelect.querySelector('option[value="en"]');
        if (zhOption) zhOption.textContent = t('optionZh');
        if (enOption) enOption.textContent = t('optionEn');
    }

    // 重新渲染显示器布局预览（更新方块内的标签文字）
    if (window.LayoutModule && window._localMonitors) {
        const layout = _getCurrentLayout();
        const preview = document.getElementById('display-layout-preview');
        const hintEdge = document.getElementById('trigger-edge-text');
        window.LayoutModule.renderMonitorLayout(window._localMonitors, window._remoteMonitors || null, layout);
        if (hintEdge) {
            window.LayoutModule.updateLayoutPreview(layout, preview, hintEdge);
        }
    }

    // 更新辅助功能遮罩文本
    updateOverlayI18n();
}

/**
 * 更新辅助功能权限遮罩内的文本
 */
function updateOverlayI18n() {
    const { t } = window.i18n;
    const titleEl = document.querySelector('.accessibility-overlay-title');
    if (titleEl) titleEl.textContent = t('overlayTitle');

    const descEl = document.querySelector('.accessibility-overlay-desc');
    if (descEl) {
        descEl.innerHTML = t('overlayDesc')
            .replace('辅助功能（Accessibility）权限', '<strong>辅助功能（Accessibility）权限</strong>')
            .replace('Accessibility permission', '<strong>Accessibility permission</strong>')
            .replace(/\n/g, '<br>');
    }

    const steps = document.querySelectorAll('.accessibility-step span:last-child');
    const stepKeys = ['overlayStep1', 'overlayStep2', 'overlayStep3'];
    steps.forEach((el, i) => {
        if (stepKeys[i]) {
            const text = t(stepKeys[i]);
            // step2 需要加粗 KVMini
            if (i === 1) {
                el.innerHTML = text.replace('KVMini', '<strong>KVMini</strong>');
            } else {
                el.textContent = text;
            }
        }
    });

    const authBtn = document.getElementById('open-accessibility-btn');
    if (authBtn) authBtn.textContent = t('btnGoAuthorize');

    const pollingHint = document.getElementById('accessibility-polling-hint');
    if (pollingHint) pollingHint.textContent = t('overlayPollingHint');
}

// ─── 版本号 ───────────────────────────────────────────────────────────────

/**
 * 初始化版本号显示
 * 通过 Platform.getAppVersion() 从后端读取，两端保持一致
 * 降级时从 HTML meta 标签读取（由构建脚本注入 package.json 版本号）
 */
async function initVersion() {
    let version = 'vunknown';

    try {
        const v = await window.Platform.getAppVersion();
        if (v && v !== 'unknown') {
            version = `v${v}`;
        } else {
            console.warn('[KVMini] initVersion 降级：getAppVersion 返回空值，使用 meta 标签版本');
        }
    } catch (e) {
        console.warn('[KVMini] initVersion 降级：getAppVersion 调用失败:', e);
    }

    if (elements.appVersion) elements.appVersion.textContent = version;
    if (elements.settingsVersion) elements.settingsVersion.textContent = version;
}

// ─── 监听地址配置 ───

/**
 * 加载网卡列表和当前监听配置到设置面板
 */
async function loadListenConfig() {
    if (!elements.listenIpSelect) return;

    // 加载网卡列表
    let interfaces = [];
    try {
        // 优先使用新的 get_network_interfaces 命令（返回名称+IP+状态）
        if (invoke) {
            try {
                interfaces = await invoke('get_network_interfaces');
            } catch {
                // 降级：使用旧的 getNetworkInterfaces
                const ips = await window.Platform.getNetworkInterfaces();
                interfaces = ips.map(ip => ({ name: ip, ip, is_up: true }));
            }
        }

        // 保留默认的"所有网卡"选项，追加具体网卡
        const defaultOption = `<option value="0.0.0.0">${escapeHtml(window.i18n.t('optionAllInterfaces'))}</option>`;
        const ifaceOptions = interfaces.map(iface =>
            `<option value="${escapeHtml(iface.ip)}">${escapeHtml(iface.name)} (${escapeHtml(iface.ip)})</option>`
        ).join('');
        elements.listenIpSelect.innerHTML = defaultOption + ifaceOptions;

        // 渲染网卡详情列表（需求 2.1）
        _renderNetworkIfaceList(interfaces);
    } catch (e) {
        console.warn('获取网卡列表失败:', e);
    }

    // 加载当前监听配置
    try {
        const config = await window.Platform.getListenConfig();
        if (config) {
            elements.listenIpSelect.value = config.ip;
            elements.listenPortInput.value = config.port;
            updateListenAddressDisplay(config.ip, config.port);

            // 检查选定网卡是否仍然活跃（需求 2.4）
            if (config.ip !== '0.0.0.0') {
                const isActive = interfaces.some(iface => iface.ip === config.ip && iface.is_up);
                _showNetworkIfaceWarning(!isActive, config.ip);
            }
        }
    } catch (e) {
        console.warn('获取监听配置失败:', e);
    }

    // 监听变化，实时更新显示
    elements.listenIpSelect.addEventListener('change', updateListenAddressFromInputs);
    elements.listenPortInput.addEventListener('input', updateListenAddressFromInputs);
}

/**
 * 渲染网卡详情列表到 #nic-list（含类型图标）
 * @param {Array} interfaces - 网卡列表 [{name, ip, is_up}]
 */
function _renderNetworkIfaceList(interfaces) {
    const list = document.getElementById('nic-list');
    if (!list) return;

    if (!interfaces || interfaces.length === 0) {
        list.innerHTML = `<div class="monitor-list-empty">${window.i18n.t('nicNoActive')}</div>`;
        return;
    }

    const { t } = window.i18n;
    list.innerHTML = interfaces.map(iface => {
        // 使用后端返回的 iface_type 字段（wifi/ethernet/vpn/other）
        const type = iface.iface_type || 'other';
        let icon = '🌐';
        if (type === 'wifi') icon = '📶';
        else if (type === 'ethernet') icon = '🔌';
        else if (type === 'vpn') icon = '🔒';
        const statusClass = iface.is_up ? 'iface-active' : 'iface-disconnected';
        return `<div class="nic-item nic-type-${type} ${statusClass}"
                     data-iface="${escapeHtml(iface.name)}" data-ip="${escapeHtml(iface.ip)}"
                     title="${escapeHtml(iface.name)} | ${escapeHtml(iface.ip)}">
            <span class="nic-icon">${icon}</span>
            <span class="nic-name">${escapeHtml(iface.name)}</span>
            <span class="nic-ip">${escapeHtml(iface.ip)}</span>
            <span class="nic-status ${iface.is_up ? 'status-up' : 'status-down'}">${iface.is_up ? t('nicStatusUp') : t('nicStatusDown')}</span>
        </div>`;
    }).join('');
}

/**
 * 显示/隐藏网卡断开警告（需求 2.4）
 * @param {boolean} show
 * @param {string} ifaceName
 */
function _showNetworkIfaceWarning(show, ifaceName) {
    const warning = document.getElementById('network-iface-warning');
    const warningText = document.getElementById('network-iface-warning-text');
    if (!warning) return;

    if (show) {
        if (warningText) warningText.textContent = window.i18n.t('nicDisconnectedWarning').replace('{name}', ifaceName);
        warning.classList.remove('hidden');
    } else {
        warning.classList.add('hidden');
    }
}

function updateListenAddressFromInputs() {
    const ip = elements.listenIpSelect?.value || '0.0.0.0';
    const port = parseInt(elements.listenPortInput?.value, 10) || appState.defaults?.tcpPort || 23456;
    updateListenAddressDisplay(ip, port);
}

function updateListenAddressDisplay(ip, port) {
    if (!elements.listenAddressDisplay) return;
    const { t } = window.i18n;
    const localIps = elements.localIp?.textContent;
    const loadingText = t('msgLocalIpLoading');
    const failedText = t('msgLocalIpFailed');
    if (ip === '0.0.0.0' && localIps && localIps !== loadingText && localIps !== failedText) {
        elements.listenAddressDisplay.textContent = `${localIps.split(' / ')[0]}:${port}（${t('orOtherLocalIp')}）`;
    } else if (ip !== '0.0.0.0') {
        elements.listenAddressDisplay.textContent = `${ip}:${port}`;
    } else {
        elements.listenAddressDisplay.textContent = `${window.i18n.t('labelLocalIpShort')}${port}`;
    }
}

// 清除快捷键
function clearHotkey() {
    elements.hotkeyInput.value = '';
    elements.hotkeyInput.dataset.hotkey = '';
}

// 快捷键录入
function handleHotkeyCapture(e) {
    e.preventDefault();
    const parts = [];
    if (e.metaKey) parts.push('Cmd');
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');

    const key = e.key;
    if (!['Meta', 'Control', 'Alt', 'Shift'].includes(key)) {
        parts.push(key.toUpperCase());
    }

    if (parts.length > 1) {
        const hotkey = parts.join('+');
        elements.hotkeyInput.value = hotkey;
        elements.hotkeyInput.dataset.hotkey = hotkey;
    }
}

// ─── 状态管理 ───

// 设置连接状态
function setConnectionStatus(status) {
    const { t } = window.i18n;
    const statusMap = {
        connected: t('statusConnected'),
        connecting: t('statusConnecting'),
        disconnected: t('statusDisconnected'),
    };
    const classMap = {
        connected: 'status-connected',
        connecting: 'status-connecting',
        disconnected: 'status-disconnected',
    };

    elements.connectionStatus.textContent = statusMap[status] || t('statusDisconnected');
    elements.connectionStatus.className = classMap[status] || 'status-disconnected';

    if (status === 'connected' && appState.connectionType) {
        elements.connectionType.textContent = appState.connectionType;
    } else if (status === 'disconnected') {
        elements.connectionType.textContent = '-';
    }

    // v0.1.59 BUG-042: 单按钮三态由 connectionPhase 驱动
    appState.connectionPhase = status;
    updateConnectButton(status);
}

// 启动延迟检查（含警告色和断线重连）
let latencyTimer = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 3;
const RECONNECT_INTERVAL_MS = 2000;

function startLatencyCheck() {
    elements.latency.textContent = window.i18n.t('msgLatencyChecking');
    elements.latency.className = '';
    reconnectAttempts = 0;
    latencyTimer = setInterval(async () => {
        if (!invoke || !appState.connected) return;
        try {
            const latency = await invoke('get_latency');
            if (latency != null) {
                elements.latency.textContent = `${latency}ms`;
                // 延迟颜色：>100ms 红色，>50ms 黄色，否则绿色
                if (latency > 100) {
                    elements.latency.style.color = '#ef4444';
                    elements.latency.title = window.i18n.t('latencyTipCritical');
                } else if (latency > 50) {
                    elements.latency.style.color = '#d97706';
                    elements.latency.title = window.i18n.t('latencyTipHigh');
                } else {
                    elements.latency.style.color = '#10b981';
                    elements.latency.title = '';
                }
                // v0.1.38 组 F：主面板 elements.latency 已作为延迟单一数据源，
                // 原 updateDetailLatency 已随 #connection-detail-panel 删除。
                // 连接正常，重置重连计数
                reconnectAttempts = 0;
            } else {
                // null = 尚未完成首次测量，正常显示 '-'，不触发重连
                elements.latency.textContent = '-';
                elements.latency.style.color = '';
            }
        } catch (e) {
            // get_latency 调用本身失败（命令不存在等）—— 只显示 '-'，不触发重连
            // 真正的连接断开由后端 connection-status 事件推送
            console.warn('[latency] get_latency 调用失败:', e);
            elements.latency.textContent = '-';
            elements.latency.style.color = '';
        }
    }, 2000);
}

// ─── 智能状态面板（v0.1.38 组 F：合并原 status-panel + connection-detail-panel） ─

/**
 * 统一入口：更新智能状态面板
 *
 * 数据源单一，避免主面板与详情面板字段重复导致的不一致（BUG-024）。
 *
 * @param {object} [data]
 * @param {boolean} [data.connected]          是否已连接
 * @param {string}  [data.peer_ip]            对端 IP
 * @param {string}  [data.local_ip]           本机 IP
 * @param {number}  [data.latency_ms]         延迟（毫秒）
 * @param {boolean} [data.tls_enabled]        是否启用 TLS（未传时读 appState.settings.enable_tls）
 * @param {string}  [data.cert_fingerprint]   对端证书指纹（连接成功且 TLS 开启时）
 */
function updateSmartStatus(data) {
    const payload = data || {};
    const connected = !!payload.connected;

    // ── 1. 加密徽章（🔒 / ⚠️），放在连接状态行后面
    const badge = document.getElementById('encryption-badge');
    if (badge) {
        const { t } = window.i18n || { t: (k) => k };
        const tlsEnabled = payload.tls_enabled !== undefined
            ? !!payload.tls_enabled
            : (appState.settings?.enable_tls !== false);
        if (connected) {
            badge.textContent = tlsEnabled ? '🔒' : '⚠️';
            badge.title = tlsEnabled ? t('tlsBadgeOn') : t('tlsBadgeOff');
            badge.classList.remove('hidden');
            badge.classList.toggle('encryption-badge--warn', !tlsEnabled);
        } else {
            badge.textContent = '';
            badge.title = '';
            badge.classList.add('hidden');
            badge.classList.remove('encryption-badge--warn');
        }
    }

    // ── 2. 证书指纹折叠区
    const fpDetails = document.getElementById('peer-fingerprint-details');
    const fpEl = document.getElementById('detail-cert-fingerprint');
    if (fpDetails && fpEl) {
        const tlsEnabled = payload.tls_enabled !== undefined
            ? !!payload.tls_enabled
            : (appState.settings?.enable_tls !== false);
        if (connected && tlsEnabled && payload.cert_fingerprint) {
            fpEl.textContent = payload.cert_fingerprint;
            fpDetails.hidden = false;
        } else {
        }
    }
}

/**
 * 更新 TLS 状态面板显示
 * @param {boolean} localTls - 本机是否开启 TLS
 * @param {boolean} remoteTls - 对端是否开启 TLS
 * @param {boolean} [disconnected=false] - 是否已断开连接（重置为 '-'）
 */
function updateTlsStatusDisplay(localTls, remoteTls, disconnected = false) {
    const el = document.getElementById('tls-status');
    if (!el) return;
    const { t } = window.i18n;

    if (disconnected) {
        el.textContent = t('tlsStatusDisconnected');
        el.className = '';
        return;
    }

    if (localTls && remoteTls) {
        el.textContent = t('tlsStatusBothOn');
        el.className = 'tls-status-ok';
    } else if (localTls && !remoteTls) {
        el.textContent = t('tlsStatusLocalOnly');
        el.className = 'tls-status-warn';
    } else if (!localTls && remoteTls) {
        el.textContent = t('tlsStatusRemoteOnly');
        el.className = 'tls-status-warn';
    } else {
        el.textContent = t('tlsStatusBothOff');
        el.className = 'tls-status-warn';
    }
}

/**
 * 更新延迟显示（主状态面板单一真相）
 * @param {number} ms
 */
function updateLatencyDisplay(ms) {
    if (!elements.latency) return;
    elements.latency.textContent = `${ms}ms`;
    // latency-critical(>300ms 红色) / latency-warning(>100ms 黄色) / latency-normal(绿)
    elements.latency.className = ms > 300
        ? 'latency-critical'
        : ms > 100 ? 'latency-warning' : 'latency-normal';
}

/**
 * 处理连接中断，自动重连（最多 3 次，间隔 2 秒）
 */
async function handleConnectionLost() {
    if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
        // 重连失败，通知用户
        stopLatencyCheck();
        appState.connected = false;
        setConnectionStatus('disconnected');
        showError(window.i18n.t('reconnectFailed').replace('{max}', MAX_RECONNECT_ATTEMPTS));
        window.Platform.showNotification('KVMini', window.i18n.t('notifyDisconnectedManualRetry')).catch(() => {});
        return;
    }

    reconnectAttempts++;
    elements.latency.textContent = window.i18n.t('reconnecting').replace('{n}', reconnectAttempts).replace('{max}', MAX_RECONNECT_ATTEMPTS);
    elements.latency.style.color = '#f59e0b';

    await new Promise(resolve => setTimeout(resolve, RECONNECT_INTERVAL_MS));

    if (!appState.connected || !appState.currentDevice) return;

    try {
        let result;
        if (appState.currentDevice.startsWith('manual:')) {
            // 手动 IP 重连
            const parts = appState.currentDevice.split(':');
            const ip = parts[1];
            const port = parseInt(parts[2], 10) || appState.defaults?.tcpPort || 23456;
            result = await window.Platform.connectByIp(ip, port);
            if (result.success) {
                reconnectAttempts = 0;
                elements.latency.style.color = '#10b981';
                showSuccess(window.i18n.t('manualReconnectSuccess').replace('{ip}', ip));
            }
        } else {
            // 自动发现设备重连（BUG-020：改用 connect_to_peer）
            const device = appState.devices.find(d => d.id === appState.currentDevice);
            const hist = device ? null : await window.Platform.getConnectionHistory();
            const record = device || (hist || []).find(h => h.id === appState.currentDevice);
            const peerIp = record?.ip;
            if (!peerIp) {
                console.warn('[KVMini] 重连失败：无法找到设备 IP');
                return;
            }
            const port = record?.port || appState.defaults?.tcpPort || 23456;
            result = await window.Platform.connectByIp(peerIp, port);
            if (result.success) {
                reconnectAttempts = 0;
                elements.latency.style.color = '#10b981';
                showSuccess(window.i18n.t('reconnectSuccess'));
            }
        }
    } catch (e) {
        // 重连失败，继续等待下次检查触发
        console.warn(`重连尝试 ${reconnectAttempts} 失败:`, e);
    }
}

// 停止延迟检查
function stopLatencyCheck() {
    if (latencyTimer) {
        clearInterval(latencyTimer);
        latencyTimer = null;
    }
    elements.latency.textContent = '-';
    elements.latency.style.color = '';
    elements.latency.title = '';
    reconnectAttempts = 0;
}

// ─── 通知提示 ───

let errorTimer = null;
// 显示错误提示（替代 alert）
function showError(message) {
    console.error('Error:', message);
    // v0.1.60 BUG-045: Toast 互斥 — 弹红色时关掉绿色和橙色
    if (elements.successToast) elements.successToast.classList.add('hidden');
    if (successTimer) { clearTimeout(successTimer); successTimer = null; }
    if (elements.reconnectToast) elements.reconnectToast.classList.add('hidden');
    if (reconnectToastTimer) { clearTimeout(reconnectToastTimer); reconnectToastTimer = null; }

    elements.errorMessage.textContent = message;
    elements.errorToast.classList.remove('hidden');
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(hideError, 5000);
}

// 隐藏错误提示
function hideError() {
    elements.errorToast.classList.add('hidden');
}

let successTimer = null;
// 显示成功提示
function showSuccess(message) {
    // v0.1.60 BUG-045: Toast 互斥 — 弹绿色时关掉橙色和红色
    if (elements.reconnectToast) elements.reconnectToast.classList.add('hidden');
    if (reconnectToastTimer) { clearTimeout(reconnectToastTimer); reconnectToastTimer = null; }
    if (elements.errorToast) elements.errorToast.classList.add('hidden');
    if (errorTimer) { clearTimeout(errorTimer); errorTimer = null; }
    // 清除重连进度行
    hideReconnectStatus();

    elements.successMessage.textContent = message;
    elements.successToast.classList.remove('hidden');
    if (successTimer) clearTimeout(successTimer);
    successTimer = setTimeout(() => {
        elements.successToast.classList.add('hidden');
    }, 3000);
}

// 更新控制状态 UI
function updateControlStateUI(state) {
    const el = elements.controlState;
    if (!el) return;
    const { t } = window.i18n;
    if (state === 'Armed') {
        el.textContent = t('controlStateArmed');
        el.className = 'control-state-armed';
    } else if (state === 'LocalControlling') {
        el.textContent = t('controlStateLocal');
        el.className = 'control-state-local';
    } else if (state === 'RemoteControlling') {
        el.textContent = t('controlStateRemote');
        el.className = 'control-state-remote';
    } else {
        el.textContent = t('controlStateIdle');
        el.className = 'control-state-idle';
    }
}

// 根据控制状态切换「开始/停止共享」按钮的 i18n key 与文字
function updateSharingButtonForState(state) {
    const btn = elements.startSharingBtn;
    if (!btn) return;
    const { t } = window.i18n;
    if (state === 'Armed') {
        btn.setAttribute('data-i18n', 'btnArmedWaiting');
        btn.textContent = t('btnArmedWaiting');
        btn.disabled = false;
    } else if (state === 'LocalControlling') {
        btn.setAttribute('data-i18n', 'btnStopSharing');
        btn.textContent = t('btnStopSharing');
        btn.disabled = false;
    } else if (state === 'RemoteControlling') {
        // v0.1.53: 被远端控制时，按钮显示"被远端控制中"，禁用（不允许手动切换）
        btn.setAttribute('data-i18n', 'btnRemoteControlled');
        btn.textContent = t('btnRemoteControlled');
        btn.disabled = true;
    } else {
        btn.setAttribute('data-i18n', 'btnStartSharing');
        btn.textContent = t('btnStartSharing');
        btn.disabled = false;
    }
}

// 处理「开始/停止共享」按钮点击：直接 toggle，由后端状态机决定方向
async function handleToggleSharing() {
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }
    const { t } = window.i18n;
    try {
        console.log('[KVMini] toggle sharing 请求');
        const newState = await invoke('toggle_sharing');
        console.log('[KVMini] toggle sharing → ', newState);
        // 后端会通过 control-state-changed 事件主动通知，这里不强制刷新
    } catch (e) {
        console.warn('[KVMini] toggle_sharing 失败:', e);
        showError(t('shareToggleFailed'));
    }
}

// 更新UI状态
function updateUI() {
    setConnectionStatus('disconnected');
    updateControlStateUI('Idle');
    // 初始化按钮状态：未连接时显示"重新连接"按钮
    updateConnectionButtons(false);
}

// ─── 工具函数 ───

// HTML 转义，防止 XSS
function escapeHtml(str) {
    if (str == null) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

// ─── 启动入口 ────────────────────────────────────────────────────────────────
// 新架构：前置内联脚本（在 main.js 之前执行）已将 invoke 捕获到 window.tauriInvokeReady。
// main.js 末尾直接读取该值：有值则立即启动，无值则走 waitForTauri 轮询降级路径。
// 这彻底解决了 macOS WKWebView 外部脚本隔离导致的启动失败问题。

(function startApp() {
    // 诊断日志：写到文件（通过 Tauri 命令）和 console
    const diagLog = (msg) => {
        console.log('[KVMini] ' + msg);
        // 尝试用 write_startup_log 写文件（需要 invoke 就绪）
        const inv = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke;
        if (inv) {
            inv('write_startup_log', { content: '[JS] ' + msg }).catch(() => {});
        }
    };

    const tauriReady = !!(window.tauriInvokeReady);
    const tauriExists = !!(window.__TAURI__);
    diagLog('startApp 执行，tauriInvokeReady=' + (tauriReady ? 'YES' : 'NO') + ', __TAURI__=' + (tauriExists ? 'YES' : 'NO'));

    if (window.tauriInvokeReady) {
        // 前置脚本已捕获 invoke，直接启动
        diagLog('tauriInvokeReady 就绪，直接启动');
        initApp().catch(e => console.error('[KVMini] initApp 异常:', e));
    } else {
        // 降级：等待 Tauri 就绪（非 Tauri 环境或 invoke 延迟注入）
        diagLog('tauriInvokeReady 未就绪，进入 waitForTauri 轮询');
        waitForTauri(5000).then((inv) => {
            if (!inv) {
                diagLog('waitForTauri 超时，initApp 未执行！');
                // 超时：显示错误提示
                const errorToast = document.getElementById('error-toast');
                const errorMessage = document.getElementById('error-message');
                if (errorToast && errorMessage) {
                    errorMessage.textContent = window.i18n.t('errTauriInitFailed');
                    errorToast.classList.remove('hidden');
                }
                return;
            }
            diagLog('waitForTauri 成功，invoke 就绪，调用 initApp');
            initApp().catch(e => console.error('[KVMini] initApp 异常:', e));
        });
    }
})();
// 暴露全局函数（供 HTML 内联调用）
window.connectToDevice = connectToDevice;
window.reconnectToDevice = reconnectToDevice;
window.reconnectManualIp = reconnectManualIp;
window.closeSettings = closeSettings;
window.openSettings = openSettings;
window.saveSettings = saveSettings;
window.clearHotkey = clearHotkey;
window.hideError = hideError;
window.onSettingsDiscardConfirm = onSettingsDiscardConfirm;
window.onSettingsDiscardCancel = onSettingsDiscardCancel;

// ─── 显示器布局配置 ───────────────────────────────────────────────────────

/**
 * 核心函数：渲染多显示器布局预览
 * @param {Array} localMonitors  - 本机显示器数组（来自 get_monitors）
 * @param {Array|null} remoteMonitors - 远端显示器数组（未连接时为 null）
 * @param {string} layout - 布局方向枚举值
 */
function renderMonitorLayout(localMonitors, remoteMonitors, layout) {
    const localGroup = document.getElementById('local-monitor-group');
    const remoteGroup = document.getElementById('remote-monitor-group');
    const divider = document.getElementById('layout-divider');
    if (!localGroup || !remoteGroup) return;

    // 更新分隔线箭头
    if (divider) {
        divider.textContent = LAYOUT_DIVIDER_ARROW[layout] || '→';
    }

    // ── 计算本机显示器包围盒 ──
    const localBBox = calcBoundingBox(localMonitors);

    // ── 计算远端显示器包围盒（占位时用默认尺寸） ──
    const hasRemote = remoteMonitors && remoteMonitors.length > 0;
    const remoteBBoxRaw = hasRemote
        ? calcBoundingBox(remoteMonitors)
        : { minX: 0, minY: 0, totalW: 1920, totalH: 1080 };

    // ── 确定预览容器可用尺寸 ──
    const preview = document.getElementById('display-layout-preview');
    const isVertical = layout === 'LocalAboveRemote' || layout === 'LocalBelowRemote';
    const containerW = preview ? preview.clientWidth - 16 : 400;
    const containerH = preview ? preview.clientHeight - 24 : 140;
    const dividerSize = 28; // 分隔线占用的像素

    // ── 计算统一缩放比例 ──
    // 两组显示器并排，需要在容器内同时放下
    const totalPhysicalW = isVertical
        ? Math.max(localBBox.totalW, remoteBBoxRaw.totalW)
        : localBBox.totalW + remoteBBoxRaw.totalW;
    const totalPhysicalH = isVertical
        ? localBBox.totalH + remoteBBoxRaw.totalH
        : Math.max(localBBox.totalH, remoteBBoxRaw.totalH);

    const availW = isVertical ? containerW : containerW - dividerSize;
    const availH = isVertical ? containerH - dividerSize : containerH;

    const scaleX = availW / totalPhysicalW;
    const scaleY = availH / totalPhysicalH;
    const scale = Math.min(scaleX, scaleY, 1); // 不放大，只缩小

    // ── 渲染本机显示器组 ──
    renderMonitorGroup(localGroup, localMonitors, localBBox, scale, true);

    // ── 渲染远端显示器组 ──
    if (hasRemote) {
        renderMonitorGroup(remoteGroup, remoteMonitors, remoteBBoxRaw, scale, false);
    } else {
        // 远端占位方块
        const placeholderW = Math.max(Math.round(remoteBBoxRaw.totalW * scale), 60);
        const placeholderH = Math.max(Math.round(remoteBBoxRaw.totalH * scale), 50);
        remoteGroup.style.width = placeholderW + 'px';
        remoteGroup.style.height = placeholderH + 'px';
        remoteGroup.innerHTML = `
            <div class="monitor-block remote-placeholder"
                 style="left:0;top:0;width:${placeholderW}px;height:${placeholderH}px;"
                 title="${escapeHtml(window.i18n.t('monitorRemoteDisconnected'))}">
                <span class="monitor-block-label">${escapeHtml(window.i18n.t('labelRemoteMonitor'))}</span>
            </div>
        `;
    }
}

/**
 * 计算一组显示器的包围盒
 * @param {Array} monitors
 * @returns {{ minX, minY, totalW, totalH }}
 */
function calcBoundingBox(monitors) {
    if (!monitors || monitors.length === 0) {
        return { minX: 0, minY: 0, totalW: 1920, totalH: 1080 };
    }
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const m of monitors) {
        const ox = m.offset_x || 0;
        const oy = m.offset_y || 0;
        minX = Math.min(minX, ox);
        minY = Math.min(minY, oy);
        maxX = Math.max(maxX, ox + m.width);
        maxY = Math.max(maxY, oy + m.height);
    }
    return { minX, minY, totalW: maxX - minX, totalH: maxY - minY };
}

/**
 * 渲染一组显示器方块到指定容器
 * @param {HTMLElement} groupEl - 容器元素
 * @param {Array} monitors - 显示器数组
 * @param {{ minX, minY, totalW, totalH }} bbox - 包围盒
 * @param {number} scale - 缩放比例
 * @param {boolean} isLocal - 是否为本机显示器
 */
function renderMonitorGroup(groupEl, monitors, bbox, scale, isLocal) {
    const MIN_SIZE = 40;
    const groupW = Math.max(Math.round(bbox.totalW * scale), MIN_SIZE);
    const groupH = Math.max(Math.round(bbox.totalH * scale), MIN_SIZE);

    groupEl.style.width = groupW + 'px';
    groupEl.style.height = groupH + 'px';

    groupEl.innerHTML = monitors.map((m) => {
        const ox = (m.offset_x || 0) - bbox.minX;
        const oy = (m.offset_y || 0) - bbox.minY;
        const left = Math.round(ox * scale);
        const top = Math.round(oy * scale);
        const w = Math.max(Math.round(m.width * scale), MIN_SIZE);
        const h = Math.max(Math.round(m.height * scale), MIN_SIZE);

        const isPrimary = isLocal && m.is_primary;
        const cssClass = isLocal
            ? (isPrimary ? 'local-primary' : 'local-secondary')
            : 'remote-connected';

        const dpiPct = m.dpi_scale ? Math.round(m.dpi_scale * 100) : 100;
        const tooltip = window.i18n.t('monitorTooltipTemplate')
            .replace('{w}', String(m.width))
            .replace('{h}', String(m.height))
            .replace('{dpi}', String(dpiPct))
            .replace('{x}', String(m.offset_x || 0))
            .replace('{y}', String(m.offset_y || 0));

        // 方块内容：尺寸足够时显示分辨率文字
        const showLabel = w >= 50 && h >= 36;
        const showSize = w >= 55 && h >= 48;

        const localLabel = isPrimary
            ? window.i18n.t('labelLocalMonitor')
            : window.i18n.t('labelSecondaryMonitor');
        const remoteLabel = window.i18n.t('labelRemoteMonitor');
        const primaryTag = window.i18n.t('labelPrimaryTag');

        return `
            <div class="monitor-block ${cssClass}"
                 style="left:${left}px;top:${top}px;width:${w}px;height:${h}px;"
                 title="${escapeHtml(tooltip)}">
                ${showLabel ? `<span class="monitor-block-label">${escapeHtml(isLocal ? localLabel : remoteLabel)}</span>` : ''}
                ${isPrimary ? `<span class="monitor-primary-tag">${escapeHtml(primaryTag)}</span>` : ''}
                ${showSize ? `<span class="monitor-block-size">${m.width}×${m.height}</span>` : ''}
            </div>
        `;
    }).join('');
}

/**
 * 获取当前布局方向（从全局状态读取）
 * @returns {string}
 */
function _getCurrentLayout() {
    return window._currentLayout || 'LocalLeftOfRemote';
}

/**
 * 初始化显示器布局配置 UI
 * 使用 mousedown/mousemove/mouseup 拖拽交互（无下拉菜单）
 */
async function initDisplayLayout() {
    const preview = document.getElementById('display-layout-preview');
    const hintEdge = document.getElementById('trigger-edge-text');

    if (!preview) return;

    // 加载真实显示器信息（同时触发布局渲染）
    await refreshMonitors();

    // 加载已保存的布局配置
    let currentLayout = 'LocalLeftOfRemote';
    try {
        if (invoke) {
            const config = await invoke('get_layout_config');
            if (config && config.layout) {
                currentLayout = config.layout;
            }
        }
    } catch (e) {
        // 加载失败使用默认值
    }
    window._currentLayout = currentLayout;
    updateLayoutPreview(currentLayout, preview, hintEdge);

    // 使用 LayoutModule 的拖拽交互（mousedown/mousemove/mouseup）
    if (window.LayoutModule && window.LayoutModule.initLayoutDrag) {
        window.LayoutModule.initLayoutDrag((newLayout) => {
            window._currentLayout = newLayout;
            updateLayoutPreview(newLayout, preview, hintEdge);
            // 拖拽结束自动保存布局配置
            saveLayoutConfig();
        });
    }

    // 监听后端 monitors-changed 事件（显示器配置变化时自动刷新）
    if (!window.__monitorsListenerRegistered && window.__TAURI__) {
        window.__monitorsListenerRegistered = true;
        window.__TAURI__.event.listen('monitors-changed', () => {
            refreshMonitors();
            const { t } = window.i18n;
            showSuccess(t('msgMonitorsChanged'));
        });
    }
}

/**
 * 刷新显示器列表（调用后端 get_monitors），并触发布局预览重新渲染
 */
async function refreshMonitors() {
    const monitorList = document.getElementById('monitor-list');
    const countBadge = document.getElementById('monitor-count-badge');
    const { t } = window.i18n;

    if (!monitorList) return;

    try {
        if (!invoke) return;
        const monitors = await invoke('get_monitors');

        // 存储到全局变量，供 renderMonitorLayout 使用
        window._localMonitors = monitors;

        // 更新显示器数量徽章
        if (countBadge) countBadge.textContent = monitors.length;

        // 渲染显示器列表
        if (monitors.length === 0) {
            monitorList.innerHTML = `<div class="monitor-list-empty">-</div>`;
        } else {
            monitorList.innerHTML = monitors.map((m) => `
                <div class="monitor-item ${m.is_primary ? 'monitor-primary' : ''}">
                    <div class="monitor-item-icon">🖥</div>
                    <div class="monitor-item-info">
                        <div class="monitor-item-name">
                            ${t('labelMonitorResolution')}: ${m.width}×${m.height}
                            ${m.is_primary ? `<span class="monitor-primary-badge">${t('labelPrimaryMonitor')}</span>` : ''}
                        </div>
                        <div class="monitor-item-detail">
                            ${t('labelMonitorDpi')}: ${(m.dpi_scale * 100).toFixed(0)}%
                            &nbsp;|&nbsp;
                            ${t('labelMonitorOffset')}: (${m.offset_x}, ${m.offset_y})
                        </div>
                    </div>
                </div>
            `).join('');

            // 多显示器时显示提示
            if (monitors.length > 1) {
                monitorList.innerHTML += `
                    <div class="monitor-multi-hint">
                        <span>💡</span> ${t('hintMultiMonitor')}
                    </div>
                `;
            }
        }

        // 触发布局预览重新渲染
        const currentLayout = window._currentLayout || 'LocalLeftOfRemote';
        renderMonitorLayout(monitors, window._remoteMonitors || null, currentLayout);

    } catch (e) {
        // 降级：使用 window.screen
        if (countBadge) countBadge.textContent = '1';
        const fallbackMonitors = [{
            width: window.screen.width,
            height: window.screen.height,
            offset_x: 0,
            offset_y: 0,
            dpi_scale: window.devicePixelRatio || 1,
            is_primary: true,
        }];
        window._localMonitors = fallbackMonitors;
        if (monitorList) monitorList.innerHTML = `
            <div class="monitor-item monitor-primary">
                <div class="monitor-item-icon">🖥</div>
                <div class="monitor-item-info">
                    <div class="monitor-item-name">
                        ${t('labelMonitorResolution')}: ${window.screen.width}×${window.screen.height}
                        <span class="monitor-primary-badge">${t('labelPrimaryMonitor')}</span>
                    </div>
                    <div class="monitor-item-detail">
                        ${t('labelMonitorDpi')}: ${Math.round(window.devicePixelRatio * 100)}%
                    </div>
                </div>
            </div>
        `;
        // 降级渲染
        const currentLayout = window._currentLayout || 'LocalLeftOfRemote';
        renderMonitorLayout(fallbackMonitors, null, currentLayout);
    }
}

/**
 * 更新布局预览：切换方向 CSS 类、更新提示文字、更新分隔线箭头、重新渲染方块
 */
function updateLayoutPreview(layout, preview, hintEdge) {
    const { t } = window.i18n;

    // 切换布局方向 CSS 类
    preview.classList.remove('layout-left', 'layout-right', 'layout-above', 'layout-below');
    const cls = LAYOUT_PREVIEW_CLASS[layout];
    if (cls) preview.classList.add(cls);

    // 更新提示文字（使用 i18n 翻译键）
    if (hintEdge) {
        const edgeKeyMap = {
            'LocalLeftOfRemote':  'edgeRight',
            'LocalRightOfRemote': 'edgeLeft',
            'LocalAboveRemote':   'edgeBottom',
            'LocalBelowRemote':   'edgeTop',
        };
        const edgeKey = edgeKeyMap[layout] || 'edgeRight';
        hintEdge.textContent = t(edgeKey);
    }

    // 重新渲染多显示器方块
    const localMonitors = window._localMonitors || null;
    const remoteMonitors = window._remoteMonitors || null;
    if (localMonitors) {
        renderMonitorLayout(localMonitors, remoteMonitors, layout);
    }
}

/**
 * 保存布局配置（在 saveSettings 中调用）
 */
async function saveLayoutConfig() {
    if (!invoke) return;

    const layout = window._currentLayout || 'LocalLeftOfRemote';

    // 使用主显示器的真实分辨率
    const localMonitors = window._localMonitors;
    const primary = localMonitors && localMonitors.find(m => m.is_primary) || (localMonitors && localMonitors[0]);
    const screenW = primary ? primary.width : window.screen.width;
    const screenH = primary ? primary.height : window.screen.height;

    const config = {
        layout: layout,
        local_screen_width: screenW,
        local_screen_height: screenH,
    };

    try {
        await invoke('set_layout_config', { config });
    } catch (e) {
        console.error('保存布局配置失败:', e);
    }
}

// 注意：布局配置初始化已移入 initApp() 中，在 waitForTauri 完成后调用
// 不再使用独立的 DOMContentLoaded 监听器，避免 invoke 未就绪时的时序问题

// ─── macOS 辅助功能权限检测（委托给 AccessibilityModule）────────────────────

/**
 * 连接成功后检查辅助功能权限
 */
async function checkPermissionAfterConnect() {
    await window.AccessibilityModule.checkPermissionAfterConnect(elements, invoke);
}

/**
 * 显示连接后权限不足提示，禁用开始共享按钮
 */
function showPermissionWarning() {
    window.AccessibilityModule.showPermissionWarning(elements);
}

/**
 * 隐藏连接后权限不足提示，恢复开始共享按钮
 */
function hidePermissionWarning() {
    window.AccessibilityModule.hidePermissionWarning(elements);
    // 断开连接时隐藏开始共享按钮，并把按钮文字复位为「开始共享」
    if (elements.startSharingBtn && !appState.connected) {
        elements.startSharingBtn.classList.add('hidden');
        updateSharingButtonForState('Idle');
    }
}

/**
 * v0.1.41: Windows 管理员权限检测
 * 若未以管理员身份运行，在页面顶部显示提示横幅
 */
async function checkWinAdminPermission() {
    if (!invoke) return;
    try {
        const result = await invoke('check_admin_permission');
        if (result && result.admin_required) {
            showWinAdminBanner();
        }
    } catch (e) {
        console.warn('[KVMini] check_admin_permission failed:', e);
    }
}

/**
 * 显示 Windows 管理员权限横幅
 */
function showWinAdminBanner() {
    const { t } = window.i18n;
    // 复用现有的 permission-warning 元素，或创建新的
    let banner = document.getElementById('win-admin-banner');
    if (!banner) {
        banner = document.createElement('div');
        banner.id = 'win-admin-banner';
        banner.className = 'permission-warning';
        banner.innerHTML = `
            <span class="permission-warning-text" data-i18n="winAdminBannerText">${t('winAdminBannerText')}</span>
            <button id="win-admin-elevate-btn" class="permission-warning-btn" data-i18n="winAdminBtnElevate">${t('winAdminBtnElevate')}</button>
        `;
        // 插入到 main content 顶部
        const mainContent = document.querySelector('.main-content') || document.body;
        mainContent.insertBefore(banner, mainContent.firstChild);

        // 绑定按钮事件
        const btn = document.getElementById('win-admin-elevate-btn');
        if (btn) {
            btn.addEventListener('click', async () => {
                try {
                    await invoke('request_admin_elevation');
                } catch (e) {
                    console.error('[KVMini] request_admin_elevation failed:', e);
                }
            });
        }
    }
    banner.classList.remove('hidden');
}

// ─── 自动更新 ─────────────────────────────────────────────────────────────────

/** 缓存的更新信息（供 install 时使用） */
let _pendingUpdate = null;

/**
 * 手动点击"检查更新"按钮时调用
 */
async function handleCheckUpdate() {
    if (!invoke) { showError(window.i18n.t('errTauriNotReady')); return; }
    const { t } = window.i18n;
    const btn = elements.aboutCheckUpdateBtn;
    if (!btn || btn.disabled) return;

    // 更新按钮状态
    btn.disabled = true;
    const iconEl = btn.querySelector('.about-action-icon');
    const textEl = btn.querySelector('[data-i18n="checkUpdate"]');
    const originalIcon = iconEl ? iconEl.textContent : '🔄';
    if (iconEl) iconEl.textContent = '⏳';
    if (textEl) textEl.textContent = t('checkingUpdate');

    try {
        const info = await invoke('check_for_update');
        if (info) {
            // 有新版本
            _pendingUpdate = info;
            showUpdateModal(info);
        } else {
            // 已是最新版本
            showSuccess(t('noUpdateAvailable'));
        }
    } catch (e) {
        console.warn('[Updater] 检查更新失败:', e);
        showError(t('updateFailedDetail').replace('{err}', String(e)));
    } finally {
        btn.disabled = false;
        if (iconEl) iconEl.textContent = originalIcon;
        if (textEl) textEl.textContent = t('checkUpdate');
    }
}

/**
 * 显示"发现新版本"模态框
 * @param {{ version: string, body?: string, date?: string }} info
 */
function showUpdateModal(info) {
    const { t } = window.i18n;
    const version = info.version || '';
    const body = info.body || '';

    const title = t('updateAvailableTitle');
    const desc = t('updateAvailableDesc').replace('{version}', version);
    const bodyHtml = body ? `<div class="update-release-notes">${escapeHtml(body).replace(/\n/g, '<br>')}</div>` : '';

    const html = `
        <div class="about-modal-body">
            <p>${escapeHtml(desc)}</p>
            ${bodyHtml}
            <div class="update-modal-actions" style="display:flex;gap:12px;margin-top:16px;justify-content:flex-end;">
                <button class="btn btn-outline" id="update-later-btn">${t('updateLater')}</button>
                <button class="btn btn-primary" id="update-now-btn">${t('updateNow')}</button>
            </div>
        </div>
    `;

    showAboutModal(title, html);

    // 绑定按钮事件
    setTimeout(() => {
        const nowBtn = document.getElementById('update-now-btn');
        const laterBtn = document.getElementById('update-later-btn');
        if (nowBtn) {
            nowBtn.addEventListener('click', async () => {
                nowBtn.disabled = true;
                nowBtn.textContent = t('updateDownloading');
                if (laterBtn) laterBtn.style.display = 'none';
                await handleInstallUpdate(nowBtn);
            });
        }
        if (laterBtn) {
            laterBtn.addEventListener('click', () => {
                closeAboutModal();
            });
        }
    }, 0);
}

/**
 * 执行更新下载和安装
 * @param {HTMLElement} btn - 更新按钮（用于显示状态）
 */
async function handleInstallUpdate(btn) {
    if (!invoke) return;
    const { t } = window.i18n;

    try {
        if (btn) btn.textContent = t('updateInstalling');
        await invoke('install_update');

        // 安装成功，提示用户重启
        if (btn) btn.textContent = t('updateRestart');
        if (btn) btn.disabled = false;
        if (btn) {
            btn.onclick = async () => {
                try {
                    // 使用 tauri-plugin-process 的 relaunch
                    if (window.__TAURI__?.core?.invoke) {
                        await window.__TAURI__.core.invoke('plugin:process|restart');
                    }
                } catch (e) {
                    console.warn('[Updater] 重启失败:', e);
                    showError(t('updateFailed'));
                }
            };
        }

        showSuccess(t('updateReady'));
    } catch (e) {
        console.error('[Updater] 安装更新失败:', e);
        if (btn) {
            btn.textContent = t('updateNow');
            btn.disabled = false;
        }
        showError(t('updateFailedDetail').replace('{err}', String(e)));
    }
}

/**
 * 后台更新事件处理：Rust 端 start_background_update_check 发现新版本时触发
 * @param {{ version: string, body?: string, date?: string }} payload
 */
function handleBackgroundUpdateAvailable(payload) {
    if (!payload || !payload.version) return;
    _pendingUpdate = payload;
    // 显示更新模态框（如果用户在设置页则直接弹出，否则发 toast 提示）
    const settingsVisible = elements.viewSettings && !elements.viewSettings.classList.contains('hidden');
    if (settingsVisible) {
        showUpdateModal(payload);
    } else {
        const { t } = window.i18n;
        showSuccess(t('updateAvailable').replace('{version}', payload.version));
    }
}

// ─── 关于 Tab ────────────────────────────────────────────────────────────────

/** 关于 Tab 是否已初始化（避免重复绑定事件） */
let _aboutTabInited = false;

/**
 * 初始化关于 Tab（每次打开设置时调用，但事件只绑定一次）
 */
async function initAboutTab() {
    // 更新版本号（从后端读取，保证与 Cargo.toml 一致）
    try {
        const v = await window.Platform.getAppVersion();
        if (v && elements.aboutVersionText) {
            elements.aboutVersionText.textContent = `KVMini v${v}`;
        }
    } catch (e) {
        // 降级：从 meta 标签读取
        const meta = document.querySelector('meta[name="app-version"]');
        if (meta && elements.aboutVersionText) {
            elements.aboutVersionText.textContent = `KVMini v${meta.getAttribute('content')}`;
        }
    }

    // 刷新许可状态
    await refreshAboutTrialStatus();

    // 事件只绑定一次
    if (_aboutTabInited) return;
    _aboutTabInited = true;

    // 反馈按钮
    elements.aboutFeedbackBtn?.addEventListener('click', async () => {
        try {
            await invoke('send_feedback');
            console.log('[About] 反馈邮件已打开');
        } catch (e) {
            console.warn('[About] 反馈失败:', e);
            showError(window.i18n.t('feedbackFailed'));
        }
    });

    // 分享按钮
    elements.aboutShareBtn?.addEventListener('click', async () => {
        const { t } = window.i18n;
        const shareText = t('shareText');
        try {
            await navigator.clipboard.writeText(shareText);
            showSuccess(t('shareCopied'));
        } catch (e) {
            console.warn('[About] 分享复制失败:', e);
        }
    });

    // 隐私政策按钮
    elements.aboutPrivacyBtn?.addEventListener('click', () => {
        showPrivacyModal();
    });

    // 更新日志按钮
    elements.aboutChangelogBtn?.addEventListener('click', () => {
        showChangelogModal();
    });

    // 检查更新按钮
    elements.aboutCheckUpdateBtn?.addEventListener('click', handleCheckUpdate);

    // 评分按钮
    elements.aboutRateBtn?.addEventListener('click', async () => {
        try {
            // 使用 SDK request_review（App Store ID 由 Rust 侧通过 KVMINI_APP_STORE_ID 环境变量读取）
            await invoke('request_review');
            console.log('[About] 评分引导已触发');
        } catch (e) {
            console.warn('[About] 评分失败:', e);
        }
    });

    // 购买按钮（App Store 版）
    elements.aboutPurchaseBtn?.addEventListener('click', async () => {
        const btn = elements.aboutPurchaseBtn;
        if (btn.disabled) return;
        btn.disabled = true;
        const originalText = btn.textContent;
        btn.textContent = window.i18n.t('purchaseProcessing');
        try {
            await invoke('purchase', { productId: 'com.kvmini.app.pro' });
            showSuccess(window.i18n.t('purchaseSuccess'));
            await refreshAboutTrialStatus();
        } catch (e) {
            console.warn('[About] 购买失败:', e);
            const detail = typeof e === 'string' ? e : (e?.message || '');
            const msg = detail
                ? `${window.i18n.t('purchaseFailed')}: ${detail}`
                : window.i18n.t('purchaseFailed');
            showError(msg);
        } finally {
            btn.disabled = false;
            btn.textContent = originalText;
        }
    });

    // 恢复购买按钮（App Store 版）
    elements.aboutRestoreBtn?.addEventListener('click', async () => {
        const btn = elements.aboutRestoreBtn;
        if (btn.disabled) return;
        btn.disabled = true;
        const originalText = btn.textContent;
        btn.textContent = window.i18n.t('restoreProcessing');
        try {
            await invoke('restore_purchase');
            showSuccess(window.i18n.t('restoreSuccess'));
            await refreshAboutTrialStatus();
        } catch (e) {
            console.warn('[About] 恢复购买失败:', e);
            const detail = typeof e === 'string' ? e : (e?.message || '');
            const msg = detail
                ? `${window.i18n.t('restoreFailed')}: ${detail}`
                : window.i18n.t('restoreFailed');
            showError(msg);
        } finally {
            btn.disabled = false;
            btn.textContent = originalText;
        }
    });
}

/**
 * 刷新关于 Tab 的许可状态显示
 * v0.1.37 起：商业化 UI 已屏蔽，统一显示为「已购买」，隐藏购买/恢复按钮
 * 将来恢复：把 LICENSE_BYPASS 置 false 即可
 */
async function refreshAboutTrialStatus() {
    if (!elements.aboutTrialStatus || !elements.aboutTrialStatusText) return;

    // 屏蔽模式：统一显示已购买，不调用 SDK，不影响后端 Keychain
    const bypass = (typeof window !== 'undefined' && window.__LICENSE_GATE_BYPASS__ !== undefined)
        ? window.__LICENSE_GATE_BYPASS__ === true
        : true;
    if (bypass) {
        const el = elements.aboutTrialStatus;
        const text = elements.aboutTrialStatusText;
        const actions = elements.aboutPurchaseActions;
        el.className = 'about-trial-status purchased';
        text.textContent = window.i18n.t('trialPurchased');
        if (actions) actions.style.display = 'none';
        return;
    }

    try {
        if (invoke) {
            const trialStatus = await invoke('get_trial_status');
            const el = elements.aboutTrialStatus;
            const text = elements.aboutTrialStatusText;
            const actions = elements.aboutPurchaseActions;

            el.className = 'about-trial-status';

            if (trialStatus === 'Purchased' || trialStatus?.Purchased !== undefined) {
                el.classList.add('purchased');
                text.textContent = window.i18n.t('trialPurchased');
                if (actions) actions.style.display = 'none';
            } else if (trialStatus === 'Active' || trialStatus?.Active !== undefined) {
                const daysLeft = typeof trialStatus?.Active === 'number' ? trialStatus.Active : '';
                el.classList.add('active');
                text.textContent = window.i18n.t('trialActive').replace('{days}', String(daysLeft));
                if (actions) actions.style.display = 'block';
            } else {
                el.classList.add('expired');
                text.textContent = window.i18n.t('trialExpired');
                if (actions) actions.style.display = 'block';
            }
        }
    } catch (e) {
        console.warn('[About] 获取许可状态失败:', e);
        if (elements.aboutTrialStatusText) {
            elements.aboutTrialStatusText.textContent = window.i18n.t('trialChecking');
        }
    }
}

/**
 * 显示隐私政策模态框
 */
async function showPrivacyModal() {
    let content = '';
    try {
        // 通过 SDK 获取隐私政策（根据当前语言）
        const locale = window.i18n.getLocale();
        if (invoke) {
            if (locale === 'en') {
                content = await invoke('get_privacy_policy_en');
            } else {
                content = await invoke('get_privacy_policy_zh');
            }
        }
    } catch (e) {
        console.warn('[About] 获取隐私政策失败:', e);
        content = window.i18n.t('privacyLoadFailed');
    }
    showAboutModal(window.i18n.t('privacyModalTitle'), `<div class="about-modal-body">${content.replace(/\n/g, '<br>')}</div>`);
}

/**
 * 显示更新日志模态框
 */
async function showChangelogModal() {
    let html = '';
    try {
        if (invoke) {
            const entries = await invoke('get_changelog');
            const locale = window.i18n.getLocale();
            if (Array.isArray(entries) && entries.length > 0) {
                html = entries.map(entry => {
                    // 根据语言选择 changes 或 changes_en
                    const changes = (locale === 'en' && entry.changes_en && entry.changes_en.length > 0)
                        ? entry.changes_en
                        : (entry.changes || []);
                    return `
                    <div class="changelog-entry">
                        <div class="changelog-version">v${entry.version}</div>
                        <div class="changelog-date">${entry.date || ''}</div>
                        <ul class="changelog-changes">
                            ${changes.map(c => `<li>${c}</li>`).join('')}
                        </ul>
                    </div>
                `}).join('');
            } else {
                html = `<p>${window.i18n.t('changelogEmpty')}</p>`;
            }
        }
    } catch (e) {
        console.warn('[About] 获取更新日志失败:', e);
        html = `<p>${window.i18n.t('changelogLoadFailed')}</p>`;
    }
    showAboutModal(window.i18n.t('changelogModalTitle'), `<div class="about-modal-body">${html}</div>`);
}

/**
 * 通用关于模态框（复用现有 modal 样式）
 */
function showAboutModal(title, bodyHtml) {
    // 复用现有 settings-discard-dialog 的 modal 结构，动态创建
    let modal = document.getElementById('about-content-modal');
    if (!modal) {
        modal = document.createElement('div');
        modal.id = 'about-content-modal';
        modal.className = 'modal';
        modal.innerHTML = `
            <div class="modal-overlay" onclick="closeAboutModal()"></div>
            <div class="modal-content" style="max-width:520px;">
                <div class="modal-header">
                    <h2 id="about-modal-title"></h2>
                    <button class="modal-close" onclick="closeAboutModal()">✕</button>
                </div>
                <div id="about-modal-body"></div>
                <div class="modal-footer">
                    <button class="btn btn-outline" onclick="closeAboutModal()" data-i18n="btnClose">${window.i18n.t('btnClose')}</button>
                </div>
            </div>
        `;
        document.body.appendChild(modal);
    }
    document.getElementById('about-modal-title').textContent = title;
    document.getElementById('about-modal-body').innerHTML = bodyHtml;
    modal.classList.remove('hidden');
}

function closeAboutModal() {
    const modal = document.getElementById('about-content-modal');
    if (modal) modal.classList.add('hidden');
}

window.closeAboutModal = closeAboutModal;

// main.js 加载完成后立即启动应用
// 初始化授权模块（异步，不阻塞主界面）
if (window.LicenseModule) {
    window.LicenseModule.initLicense().catch(e => {
        console.warn('[License] 授权模块初始化失败:', e);
    });
}

// ── Node/Jest 测试环境导出（浏览器下 module 不存在，此块被忽略） ────────────
// v0.1.40: 为 bug-032 / sharing-armed 单元测试暴露纯 UI 函数
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        updateConnectionButtons,
        updateControlStateUI,
        updateSharingButtonForState,
        // 测试可通过这两个 setter 注入 mock 的 appState / elements
        __setAppState: (s) => { appState = s; },
        __setElements: (e) => { elements = e; },
    };
}
