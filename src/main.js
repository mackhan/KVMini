// 全局状态
let appState = {
    connected: false,
    connectionType: null,
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
    }
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
function initApp() {
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

    // 初始化 DOM 元素引用
    elements = {
        localIp: document.getElementById('local-ip'),
        connectionStatus: document.getElementById('connection-status'),
        connectionType: document.getElementById('connection-type'),
        latency: document.getElementById('latency'),
        disconnectBtn: document.getElementById('disconnect-btn'),
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
        settingsModal: document.getElementById('settings-modal'),
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
        // TLS 加密
        enableTls: document.getElementById('enable-tls'),
        localTlsFingerprint: document.getElementById('local-tls-fingerprint'),
        // 版本号
        appVersion: document.getElementById('app-version'),
        settingsVersion: document.getElementById('settings-version'),
        // 重连相关
        reconnectBtn: document.getElementById('reconnect-btn'),
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
        aboutVersionText: document.getElementById('about-version-text'),
    };

    setupEventListeners();
    updateUI();
    loadSettings();
    loadConnectionHistory();
    loadLocalIp();
    initAppearance();
    initVersion();
    prefillDefaultIp();

    // Windows 特有：hint-panel 仅在 Windows 下显示
    if (elements.hintPanel) {
        const isWindows = navigator.userAgent.includes('Windows') ||
            (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32');
        elements.hintPanel.style.display = isWindows ? '' : 'none';
    }

    // 新架构：权限检测由 Rust 侧主动推送事件，前端只需注册事件监听器
    // 事件监听器和「前往授权」按钮绑定已在 accessibility.js 中完成

    // 延迟初始化显示器布局配置（需要 invoke 就绪后才能调用后端）
    setTimeout(initDisplayLayout, 500);
}

/**
 * 根据当前平台预填充手动连接 IP 输入框的默认值
 * - Windows 端：预填 Mac 端 IP（30.19.140.39）
 * - Mac 端：预填 Windows 端 IP（30.19.24.43）
 * 仅在输入框为空时填入，不覆盖用户已输入的内容
 */
function prefillDefaultIp() {
    if (!elements.manualIpInput) return;
    // 已有内容则不覆盖
    if (elements.manualIpInput.value.trim()) return;

    const isWindows = navigator.userAgent.includes('Windows') ||
        (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32');

    // Windows 端连接 Mac，Mac 端连接 Windows
    const defaultIp = isWindows ? '30.19.140.39' : '30.19.24.43';
    elements.manualIpInput.value = defaultIp;
}

// 设置事件监听器
function setupEventListeners() {
    if (!elements.disconnectBtn) {
        console.error('[KVMini] 严重错误：disconnect-btn 元素不存在，事件绑定失败！');
        return;
    }
    elements.disconnectBtn.addEventListener('click', handleDisconnect);
    if (elements.reconnectBtn) {
        elements.reconnectBtn.addEventListener('click', handleReconnect);
    }
    elements.settingsBtn.addEventListener('click', openSettings);
    elements.scanBtn.addEventListener('click', handleScan);
    elements.clearHistoryBtn.addEventListener('click', handleClearHistory);
    elements.manualConnectBtn.addEventListener('click', handleManualConnect);

    console.log('[KVMini] 按鈕事件监听器绑定状态:', bindStatus);
    elements.manualIpInput.addEventListener('keydown', (e) => {        if (e.key === 'Enter') handleManualConnect();
    });

    // 快捷键录入
    elements.hotkeyInput.addEventListener('keydown', handleHotkeyCapture);

    // ESC 键关闭设置页（有未保存修改时弹出确认对话框）
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            const modal = elements.settingsModal;
            if (modal && !modal.classList.contains('hidden')) {
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

        // 监听全局快捷键触发事件，手动切换控制权
        window.__TAURI__.event.listen('hotkey-triggered', async () => {
            try {
                await invoke('toggle_control');
                updateUI();
            } catch (e) {
                console.warn('快捷键触发控制权切换失败:', e);
            }
        }).catch(e => console.warn('快捷键事件监听注册失败:', e));

        // 监听控制状态变化事件（后端主动通知）
        window.__TAURI__.event.listen('control-state-changed', (event) => {
            const payload = event.payload || {};
            const state = payload.state || 'Idle';
            updateControlStateUI(state);
        }).catch(e => console.warn('控制状态事件监听注册失败:', e));

        // 监听连接状态事件（后端主动通知）
        window.__TAURI__.event.listen('connection-status', (event) => {
            const payload = event.payload || {};
            const eventType = payload.event_type || '';
            const connected = payload.connected === true;
            const peerIp = payload.peer_ip || null;
            const message = payload.message || '';
            const attempt = payload.attempt || null;

            if (eventType === 'connected') {
                // 连接成功
                appState.connected = true;
                appState.connectionType = 'TCP';
                setConnectionStatus('connected');
                hideReconnectStatus();
                if (peerIp && elements.connectedDevice) {
                    elements.connectedDevice.textContent = peerIp;
                }
                showSuccess(message || `已连接到 ${peerIp}`);
                updateConnectionButtons(true);
                // 显示连接详情面板
                updateConnectionDetailPanel({ connected: true, peer_ip: peerIp });

            } else if (eventType === 'reconnect_success') {
                // 重连成功
                appState.connected = true;
                appState.connectionType = 'TCP';
                setConnectionStatus('connected');
                hideReconnectStatus();
                if (peerIp && elements.connectedDevice) {
                    elements.connectedDevice.textContent = peerIp;
                }
                showSuccess(message || '重连成功');
                updateConnectionButtons(true);
                // 显示连接详情面板
                updateConnectionDetailPanel({ connected: true, peer_ip: peerIp });

            } else if (eventType === 'disconnected') {
                // 连接断开
                appState.connected = false;
                appState.connectionType = null;
                setConnectionStatus('disconnected');
                hideReconnectStatus();
                if (elements.connectedDevice) {
                    elements.connectedDevice.textContent = '-';
                }
                showReconnectToast(message || '连接已断开');
                updateConnectionButtons(false);
                // 隐藏连接详情面板
                updateConnectionDetailPanel({ connected: false });

            } else if (eventType === 'reconnecting') {
                // 重连中
                appState.connected = false;
                setConnectionStatus('connecting');
                showReconnectStatus(message, attempt);
                updateConnectionButtons(false);

            } else if (eventType === 'tls_mismatch') {
                // TLS 配置不一致：一端开了一端没开
                const banner = document.getElementById('tls-mismatch-banner');
                const msgEl = document.getElementById('tls-mismatch-msg');
                if (banner && msgEl) {
                    msgEl.textContent = message || 'TLS 配置不一致，请检查双端设置';
                    banner.classList.remove('hidden');
                }
                // 连接状态区域显示警告图标
                const indicator = document.getElementById('tls-status-indicator');
                if (indicator) {
                    indicator.classList.remove('hidden');
                    indicator.textContent = '⚠️ TLS';
                    indicator.className = 'tls-warning-indicator';
                }
                log.warn('[TLS] 不一致：', message);

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
    }
}

// ─── 本机 IP ───

// 加载并显示本机 IP
async function loadLocalIp() {
    if (!elements.localIp) return;
    try {
        const ips = await window.Platform.getLocalIp();
        if (ips && ips.length > 0) {
            elements.localIp.textContent = ips.join(' / ');
        } else {
            elements.localIp.textContent = window.i18n.t('msgLocalIpFailed');
        }
    } catch (e) {
        elements.localIp.textContent = window.i18n.t('msgLocalIpFailed');
    }
}

// ─── 连接管理 ───

// 处理断开连接
async function handleDisconnect() {
    if (!invoke) { showError('Tauri 环境未就绪，请重启应用'); return; }
    const { t } = window.i18n;
    try {
        // 断开 TCP 连接（包括握手建立的连接），停止自动重连
        await invoke('disconnect_from_mac').catch(() => {});
        await invoke('disconnect_tcp').catch(() => {});
        await invoke('disconnect_devices').catch(() => {});
        // 停止输入捕获（Windows 特有）
        try { await invoke('stop_input_capture'); } catch (_) {}
        appState.connected = false;
        appState.connectionType = null;
        setConnectionStatus('disconnected');
        hideReconnectStatus();
        stopLatencyCheck();
        hidePermissionWarning();
        showSuccess(t('msgDisconnectSuccess') || '已手动断开连接');
        updateConnectionButtons(false);
    } catch (error) {
        showError('断开连接错误: ' + error);
    }
}

// 处理手动重新连接
async function handleReconnect() {
    if (!invoke) { showError('Tauri 环境未就绪，请重启应用'); return; }
    try {
        setConnectionStatus('connecting');
        showReconnectStatus('正在连接...');
        updateConnectionButtons(false);
        await invoke('reconnect_to_peer').catch(() => {});
    } catch (error) {
        showError('重新连接失败: ' + error);
    }
}

// 更新断开/重连按钮显示状态
function updateConnectionButtons(connected) {
    if (elements.disconnectBtn) {
        elements.disconnectBtn.disabled = !connected;
        elements.disconnectBtn.classList.toggle('hidden', !connected);
    }
    if (elements.reconnectBtn) {
        elements.reconnectBtn.classList.toggle('hidden', connected);
    }
}

// 显示重连状态（状态面板中）
function showReconnectStatus(message, attempt) {
    if (elements.reconnectStatusRow) {
        elements.reconnectStatusRow.style.display = '';
    }
    if (elements.reconnectStatusText) {
        elements.reconnectStatusText.textContent = message || '正在重连...';
    }
    // 重连超过 10 次时显示 Toast 提示
    if (attempt && attempt === 11) {
        showReconnectToast('重连失败，仍在后台尝试...');
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
    if (!invoke) { showError('Tauri 环境未就绪，请重启应用'); return; }
    const { t } = window.i18n;
    try {
        setConnectionStatus('connecting');
        const result = await invoke('connect_to_device', { deviceId });
        if (result.success) {
            appState.connected = true;
            appState.connectionType = result.connectionType;
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');
            startLatencyCheck();
            // 保存到历史记录
            const device = appState.devices.find(d => d.id === deviceId);
            if (device) saveToHistory(device);
            showSuccess(t('connectedToIp').replace('{ip}', device?.name || deviceId));
            await checkPermissionAfterConnect();
        } else {
            setConnectionStatus('disconnected');
            // 修复：result.error 可能为 undefined，使用兜底文案
            const errMsg = result.error || result.errorMessage || t('connectErrGeneric').replace('{err}', t('statusDisconnected'));
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
    if (!invoke) { showError('Tauri 环境未就绪，请重启应用'); return; }

    const ip = elements.manualIpInput.value.trim();
    const port = parseInt(elements.manualPortInput.value, 10) || 8889;
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
    if (!invoke) { showError('Tauri 环境未就绪，请重启应用'); return; }

    appState.scanning = true;
    setScanningState(true);

    try {
        const devices = await invoke('discover_devices');
        if (devices.length === 0) {
            // 扫描成功但未发现其他设备
            updateDevicesList([]);
            showSuccess(window.i18n.t('noDevicesFound'));
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
            ? `onclick="reconnectManualIp('${escapeHtml(item.ip)}', ${item.port || 8889})"`
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
    if (!invoke) { showError('Tauri 环境未就绪，请重启应用'); return; }
    const { t } = window.i18n;
    try {
        setConnectionStatus('connecting');
        const result = await invoke('connect_to_device', { deviceId });
        if (result.success) {
            appState.connected = true;
            appState.connectionType = result.connectionType;
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');
            startLatencyCheck();
            showSuccess(t('reconnectedToIp').replace('{ip}', deviceName));
        } else {
            setConnectionStatus('disconnected');
            // 修复：result.error 可能为 undefined，使用兜底文案
            const errMsg = result.error || result.errorMessage || t('connectErrGeneric').replace('{err}', t('statusDisconnected'));
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
    elements.manualPortInput.value = port || 8889;
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
    loadSettingsToUI();
    loadListenConfig();
    loadLocalTlsFingerprint();
    elements.settingsModal.classList.remove('hidden');
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

// 实际关闭设置页（内部函数）
function _doCloseSettings() {
    if (window.SettingsTabsModule) {
        window.SettingsTabsModule.clearAllDirty();
    }
    elements.settingsModal.classList.add('hidden');
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
        log.warn('[TLS] 证书指纹加载失败:', e);
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
        showToast(t('certRegenSuccess') || '证书已重新生成');
    } catch (e) {
        showToast(t('certRegenFailed') || '证书重新生成失败');
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

        // 2. 保存设置（enable_tls = false）
        const newConfig = { ...appState.settings, enable_tls: false };
        await invoke('save_settings', { config: newConfig });
        appState.settings = newConfig;

        // 3. 隐藏 banner 和警告图标
        const banner = document.getElementById('tls-mismatch-banner');
        if (banner) banner.classList.add('hidden');
        const indicator = document.getElementById('tls-status-indicator');
        if (indicator) indicator.classList.add('hidden');

        // 4. 触发重连
        log.info('[TLS] 已关闭本机 TLS，正在重新连接...');
        await invoke('connect_to_peer', { ip: appState.settings.peer_ip });
    } catch (e) {
        log.error('[TLS] 关闭 TLS 重试失败:', e);
        showError('关闭 TLS 重试失败：' + (e instanceof Error ? e.message : String(e)));
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
        if (addrEl) addrEl.textContent = `对端地址：${address}`;

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
    // TLS 加密
    if (elements.enableTls) {
        elements.enableTls.checked = appState.settings.enableTls !== false;
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
            await invoke('save_settings', { config: appState.settings });
        } catch (e) {
            // 显示后端错误（如开机自启动注册失败）
            showError(`设置保存失败: ${e}`);
            return;
        }

        // 保存监听地址配置
        if (elements.listenIpSelect && elements.listenPortInput) {
            const listenIp = elements.listenIpSelect.value || '0.0.0.0';
            const listenPort = parseInt(elements.listenPortInput.value, 10) || 8889;
            if (listenPort >= 1024 && listenPort <= 65535) {
                try {
                    await window.Platform.setListenAddress(listenIp, listenPort);
                } catch (e) {
                    console.warn('保存监听配置失败:', e);
                }
            } else {
                showError('监听端口范围应为 1024-65535');
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
            showError(`快捷键注册失败: ${e}`);
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
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        el.textContent = t(key);
    });

    // 更新 placeholder
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        const key = el.getAttribute('data-i18n-placeholder');
        el.placeholder = t(key);
    });

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

    // 更新权限警告文本
    const permWarnText = document.querySelector('.permission-warning-text');
    if (permWarnText) permWarnText.textContent = t('permissionWarningText');

    // 更新语言选择器选项文本（确保选中项文本与当前语言一致）
    if (elements.languageSelect) {
        const zhOption = elements.languageSelect.querySelector('option[value="zh"]');
        const enOption = elements.languageSelect.querySelector('option[value="en"]');
        if (zhOption) zhOption.textContent = t('optionZh');
        if (enOption) enOption.textContent = t('optionEn');
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
        const defaultOption = '<option value="0.0.0.0">所有网卡（0.0.0.0，推荐）</option>';
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
        list.innerHTML = '<div class="monitor-list-empty">未检测到活跃网卡</div>';
        return;
    }

    list.innerHTML = interfaces.map(iface => {
        // 根据网卡名称推断类型
        const name = iface.name.toLowerCase();
        let icon = '🌐';
        let type = 'other';
        if (name.includes('wlan') || name.includes('wifi') || name.includes('wi-fi') ||
            (name.startsWith('en') && !name.includes('eth'))) {
            icon = '📶'; type = 'wifi';
        } else if (name.includes('eth') || name.includes('en0') || name.includes('en1')) {
            icon = '🔌'; type = 'ethernet';
        } else if (name.includes('vpn') || name.includes('tun') || name.includes('tap') || name.includes('utun')) {
            icon = '🔒'; type = 'vpn';
        }
        const statusClass = iface.is_up ? 'iface-active' : 'iface-disconnected';
        return `<div class="nic-item nic-type-${type} ${statusClass}"
                     data-iface="${escapeHtml(iface.name)}" data-ip="${escapeHtml(iface.ip)}"
                     title="${escapeHtml(iface.name)} | ${escapeHtml(iface.ip)}">
            <span class="nic-icon">${icon}</span>
            <span class="nic-name">${escapeHtml(iface.name)}</span>
            <span class="nic-ip">${escapeHtml(iface.ip)}</span>
            <span class="nic-status ${iface.is_up ? 'status-up' : 'status-down'}">${iface.is_up ? '活跃' : '断开'}</span>
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
        if (warningText) warningText.textContent = `选定网卡 ${ifaceName} 已断开，请切换到其他网卡`;
        warning.classList.remove('hidden');
    } else {
        warning.classList.add('hidden');
    }
}

function updateListenAddressFromInputs() {
    const ip = elements.listenIpSelect?.value || '0.0.0.0';
    const port = parseInt(elements.listenPortInput?.value, 10) || 8889;
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
        elements.listenAddressDisplay.textContent = `本机IP:${port}`;
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

    elements.disconnectBtn.disabled = status !== 'connected';
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
                    elements.latency.title = '网络延迟过高，使用体验可能受到严重影响';
                } else if (latency > 50) {
                    elements.latency.style.color = '#d97706';
                    elements.latency.title = '网络延迟较高，可能影响使用体验';
                } else {
                    elements.latency.style.color = '#10b981';
                    elements.latency.title = '';
                }
                // 同步更新连接详情面板的延迟显示
                updateDetailLatency(latency);
                // 连接正常，重置重连计数
                reconnectAttempts = 0;
            } else {
                elements.latency.textContent = '-';
                elements.latency.style.color = '';
            }
        } catch (e) {
            // get_latency 失败可能意味着连接中断
            if (appState.connected) {
                handleConnectionLost();
            }
        }
    }, 2000);
}

// ─── 连接详情面板 ─────────────────────────────────────────────────────────────────

/**
 * 更新连接详情面板
 * @param {object} data - 连接信息
 */
function updateConnectionDetailPanel(data) {
    const panel = document.getElementById('connection-detail-panel');
    if (!panel) return;

    if (!data || !data.connected) {
        panel.classList.add('hidden');
        return;
    }

    panel.classList.remove('hidden');

    // 本机 IP
    const localIpEl = document.getElementById('detail-local-ip');
    if (localIpEl) {
        const localIp = appState.settings?.local_ip || data.local_ip || '-';
        const port = appState.settings?.port || 8765;
        localIpEl.textContent = localIp !== '-' ? `${localIp}:${port}` : '-';
    }

    // 对端 IP
    const peerIpEl = document.getElementById('detail-peer-ip');
    if (peerIpEl) {
        peerIpEl.textContent = data.peer_ip || appState.settings?.peer_ip || '-';
    }

    // TLS 状态
    const tlsEl = document.getElementById('detail-tls-status');
    if (tlsEl) {
        const tlsEnabled = appState.settings?.enable_tls !== false;
        tlsEl.textContent = tlsEnabled ? '🔒 TLS 加密' : '⚠️ 未加密';
        tlsEl.className = `detail-value ${tlsEnabled ? 'tls-on' : 'tls-off tls-warning'}`;
    }

    // 证书指纹（展开区域）
    if (data.cert_fingerprint) {
        const fpEl = document.getElementById('detail-cert-fingerprint');
        if (fpEl) fpEl.textContent = data.cert_fingerprint;
    }

    log.info('[Detail] 连接详情面板已更新');
}

/**
 * 更新详情面板的延迟显示
 * @param {number} ms - 延迟毫秒数
 */
function updateDetailLatency(ms) {
    const el = document.getElementById('detail-latency');
    if (!el) return;
    el.textContent = `${ms}ms`;
    // latency-critical(>300ms 红色) / latency-warning(>100ms 黄色) / latency-normal(正常绿色)
    el.className = `detail-value ${ms > 300 ? 'latency-critical' : ms > 100 ? 'latency-warning' : 'latency-normal'}`;
}

/**
 * 切换连接详情面板的展开/折叠状态
 */
function toggleConnectionDetail() {
    const expanded = document.getElementById('detail-expanded');
    const icon = document.getElementById('detail-toggle-icon');
    if (!expanded) return;
    const isHidden = expanded.classList.toggle('hidden');
    if (icon) icon.textContent = isHidden ? '▼' : '▲';
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
        window.Platform.showNotification('KVMini', '连接已断开，请手动重连').catch(() => {});
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
            const port = parseInt(parts[2], 10) || 8889;
            result = await window.Platform.connectByIp(ip, port);
            if (result.success) {
                reconnectAttempts = 0;
                elements.latency.style.color = '#10b981';
                showSuccess(window.i18n.t('manualReconnectSuccess').replace('{ip}', ip));
            }
        } else {
            // 自动发现设备重连
            result = await invoke('connect_to_device', { deviceId: appState.currentDevice });
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
    if (state === 'LocalControlling') {
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
        initApp();
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
                    errorMessage.textContent = 'Tauri 环境初始化失败，请重启应用';
                    errorToast.classList.remove('hidden');
                }
                return;
            }
            diagLog('waitForTauri 成功，invoke 就绪，调用 initApp');
            initApp();
        });
    }
})();
// 暴露全局函数（供 HTML 内联调用）
window.connectToDevice = connectToDevice;
window.reconnectToDevice = reconnectToDevice;
window.reconnectManualIp = reconnectManualIp;
window.closeSettings = closeSettings;
window.saveSettings = saveSettings;
window.clearHotkey = clearHotkey;
window.hideError = hideError;

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
                 title="远端设备（未连接）">
                <span class="monitor-block-label">远端</span>
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
        const tooltip = `${m.width}×${m.height} | DPI: ${dpiPct}% | 偏移: (${m.offset_x || 0}, ${m.offset_y || 0})`;

        // 方块内容：尺寸足够时显示分辨率文字
        const showLabel = w >= 50 && h >= 36;
        const showSize = w >= 55 && h >= 48;

        return `
            <div class="monitor-block ${cssClass}"
                 style="left:${left}px;top:${top}px;width:${w}px;height:${h}px;"
                 title="${tooltip}">
                ${showLabel ? `<span class="monitor-block-label">${isLocal ? (isPrimary ? '本机' : '副屏') : '远端'}</span>` : ''}
                ${isPrimary ? `<span class="monitor-primary-tag">主</span>` : ''}
                ${showSize ? `<span class="monitor-block-size">${m.width}×${m.height}</span>` : ''}
            </div>
        `;
    }).join('');
}

/**
 * 初始化显示器布局配置 UI
 */
async function initDisplayLayout() {
    const select = document.getElementById('display-layout-select');
    const preview = document.getElementById('display-layout-preview');
    const hintEdge = document.getElementById('trigger-edge-text');
    const localGroup = document.getElementById('local-monitor-group');

    if (!select || !preview) return;

    // 加载真实显示器信息（同时触发布局渲染）
    await refreshMonitors();

    // 加载已保存的布局配置
    try {
        if (invoke) {
            const config = await invoke('get_layout_config');
            if (config && config.layout) {
                select.value = config.layout;
                updateLayoutPreview(config.layout, preview, hintEdge);
            }
        }
    } catch (e) {
        // 加载失败使用默认值
        updateLayoutPreview('LocalLeftOfRemote', preview, hintEdge);
    }

    // 监听下拉框变化
    select.addEventListener('change', () => {
        updateLayoutPreview(select.value, preview, hintEdge);
    });

    // 使用 LayoutModule 的拖拽交互（支持重叠检测和自动保存）
    if (window.LayoutModule && window.LayoutModule.initLayoutDrag) {
        window.LayoutModule.initLayoutDrag((newLayout) => {
            select.value = newLayout;
            updateLayoutPreview(newLayout, preview, hintEdge);
            // 拖拽结束自动保存布局配置
            saveLayoutConfig();
        });
    } else {
        // 降级：旧版拖拽逻辑
        if (localGroup) {
            localGroup.addEventListener('dragstart', (e) => {
                e.dataTransfer.setData('text/plain', 'local-monitor-group');
                preview.classList.add('drag-over');
            });
            localGroup.addEventListener('dragend', () => {
                preview.classList.remove('drag-over');
            });
        }
        if (preview) {
            preview.addEventListener('dragover', (e) => { e.preventDefault(); });
            preview.addEventListener('drop', (e) => {
                e.preventDefault();
                preview.classList.remove('drag-over');
                const rect = preview.getBoundingClientRect();
                const relX = e.clientX - rect.left;
                const relY = e.clientY - rect.top;
                const centerX = rect.width / 2;
                const centerY = rect.height / 2;
                const dx = relX - centerX;
                const dy = relY - centerY;
                let newLayout;
                if (Math.abs(dx) > Math.abs(dy)) {
                    // dx < 0 → 鼠标在中心左侧 → 本机在远端左侧 → LocalLeftOfRemote
                    newLayout = dx < 0 ? 'LocalLeftOfRemote' : 'LocalRightOfRemote';
                } else {
                    // dy < 0 → 鼠标在中心上方 → 本机在远端上方 → LocalAboveRemote
                    newLayout = dy < 0 ? 'LocalAboveRemote' : 'LocalBelowRemote';
                }
                select.value = newLayout;
                updateLayoutPreview(newLayout, preview, hintEdge);
            });
        }
    }

    // 监听后端 monitors-changed 事件（显示器配置变化时自动刷新）
    if (!window.__monitorsListenerRegistered && window.__TAURI__) {
        window.__monitorsListenerRegistered = true;
        window.__TAURI__.event.listen('monitors-changed', () => {
            refreshMonitors();
            const { t } = window.i18n;
            showToast(t('msgMonitorsChanged'));
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
        const select = document.getElementById('display-layout-select');
        const currentLayout = select ? select.value : 'LocalLeftOfRemote';
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
        const select = document.getElementById('display-layout-select');
        const currentLayout = select ? select.value : 'LocalLeftOfRemote';
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
    const select = document.getElementById('display-layout-select');
    if (!select || !invoke) return;

    // 使用主显示器的真实分辨率
    const localMonitors = window._localMonitors;
    const primary = localMonitors && localMonitors.find(m => m.is_primary) || (localMonitors && localMonitors[0]);
    const screenW = primary ? primary.width : window.screen.width;
    const screenH = primary ? primary.height : window.screen.height;

    const config = {
        layout: select.value,
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
    // 断开连接时隐藏开始共享按钮
    if (elements.startSharingBtn && !appState.connected) {
        elements.startSharingBtn.classList.add('hidden');
    }
}

// ─── 关于 Tab ────────────────────────────────────────────────────────────────

/** 关于 Tab 是否已初始化（避免重复绑定事件） */
let _aboutTabInited = false;

/**
 * 初始化关于 Tab（每次打开设置时调用，但事件只绑定一次）
 */
async function initAboutTab() {
    // 更新版本号
    const meta = document.querySelector('meta[name="app-version"]');
    if (meta && elements.aboutVersionText) {
        elements.aboutVersionText.textContent = `KVMini v${meta.getAttribute('content')}`;
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
            showError('反馈功能暂时不可用');
        }
    });

    // 分享按钮
    elements.aboutShareBtn?.addEventListener('click', async () => {
        const shareText = 'KVMini - 跨平台键鼠共享工具，让多台电脑共享一套键鼠！https://kvmini.app';
        try {
            await navigator.clipboard.writeText(shareText);
            showSuccess(window.i18n.t('shareCopied'));
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

    // 评分按钮
    elements.aboutRateBtn?.addEventListener('click', async () => {
        try {
            // 使用 SDK request_review，传入 app_id（App Store 版）
            await invoke('request_review', { appId: '' });
            console.log('[About] 评分引导已触发');
        } catch (e) {
            console.warn('[About] 评分失败:', e);
        }
    });

    // 购买按钮（App Store 版）
    elements.aboutPurchaseBtn?.addEventListener('click', async () => {
        try {
            await invoke('purchase', { productId: 'com.kvmini.app.pro' });
        } catch (e) {
            console.warn('[About] 购买失败:', e);
            showError('购买功能暂时不可用');
        }
    });

    // 恢复购买按钮（App Store 版）
    elements.aboutRestoreBtn?.addEventListener('click', async () => {
        try {
            await invoke('restore_purchase');
            await refreshAboutTrialStatus();
        } catch (e) {
            console.warn('[About] 恢复购买失败:', e);
            showError('恢复购买功能暂时不可用');
        }
    });
}

/**
 * 刷新关于 Tab 的许可状态显示
 */
async function refreshAboutTrialStatus() {
    if (!elements.aboutTrialStatus || !elements.aboutTrialStatusText) return;

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
        content = '隐私政策内容加载失败，请稍后重试。';
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
            if (Array.isArray(entries) && entries.length > 0) {
                html = entries.map(entry => `
                    <div class="changelog-entry">
                        <div class="changelog-version">v${entry.version}</div>
                        <div class="changelog-date">${entry.date || ''}</div>
                        <ul class="changelog-changes">
                            ${(entry.changes || []).map(c => `<li>${c}</li>`).join('')}
                        </ul>
                    </div>
                `).join('');
            } else {
                html = '<p>暂无更新日志</p>';
            }
        }
    } catch (e) {
        console.warn('[About] 获取更新日志失败:', e);
        html = '<p>更新日志加载失败，请稍后重试。</p>';
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
                    <button class="btn btn-outline" onclick="closeAboutModal()">关闭</button>
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
