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

// invoke 函数引用（在 DOMContentLoaded 后安全初始化）
let invoke = null;

/**
 * 安全获取 Tauri invoke 函数
 * 兼容 Tauri 1.x 的多种挂载方式
 */
function getTauriInvoke() {
    // Tauri 1.x: withGlobalTauri=true 时挂载到 window.__TAURI__.tauri.invoke
    if (window.__TAURI__?.tauri?.invoke) {
        return window.__TAURI__.tauri.invoke;
    }
    // Tauri 1.x 部分版本直接挂载到 window.__TAURI__.invoke
    if (window.__TAURI__?.invoke) {
        return window.__TAURI__.invoke;
    }
    // 未找到，返回 null（非 Tauri 环境，如浏览器预览）
    return null;
}

// DOM元素引用（在 DOMContentLoaded 后初始化）
let elements = {};

// 初始化应用
function initApp() {
    // 安全获取 invoke
    invoke = getTauriInvoke();
    if (!invoke) {
        console.warn('Tauri invoke 未就绪，部分功能不可用（非 Tauri 环境）');
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
        // 角色徽章
        currentRole: document.getElementById('current-role'),
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
    };

    setupEventListeners();
    updateUI();
    loadSettings();
    loadConnectionHistory();
    loadLocalIp();
    initAppearance();
    initVersion();

    // Windows 特有：hint-panel 仅在 Windows 下显示
    if (elements.hintPanel) {
        const isWindows = navigator.userAgent.includes('Windows') ||
            (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32');
        elements.hintPanel.style.display = isWindows ? '' : 'none';
    }
}

// 设置事件监听器
function setupEventListeners() {
    elements.disconnectBtn.addEventListener('click', handleDisconnect);
    elements.settingsBtn.addEventListener('click', openSettings);
    elements.scanBtn.addEventListener('click', handleScan);
    elements.clearHistoryBtn.addEventListener('click', handleClearHistory);
    elements.manualConnectBtn.addEventListener('click', handleManualConnect);

    // 手动 IP 输入框回车触发连接
    elements.manualIpInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') handleManualConnect();
    });

    // 快捷键录入
    elements.hotkeyInput.addEventListener('keydown', handleHotkeyCapture);

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

        // 监听全局快捷键触发事件，切换主控/被控角色
        window.__TAURI__.event.listen('hotkey-triggered', async () => {
            try {
                // 切换角色：主控端 ↔ 被控端
                const currentRole = await invoke('get_current_role');
                const newRole = currentRole === 'Client' ? 'Server' : 'Client';
                await invoke('set_role', { role: newRole });
                const { t } = window.i18n;
                const msg = newRole === 'Client' ? t('msgSwitchedToClient') : t('msgSwitchedToServer');
                showSuccess(msg);
                updateUI();
            } catch (e) {
                console.warn('快捷键触发角色切换失败:', e);
            }
        }).catch(e => console.warn('快捷键事件监听注册失败:', e));
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
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
    const { t } = window.i18n;
    try {
        // 断开 TCP 连接（包括握手建立的连接）
        await invoke('disconnect_tcp').catch(() => {});
        await invoke('disconnect_devices').catch(() => {});
        // 停止输入捕获（Windows 特有）
        try { await invoke('stop_input_capture'); } catch (_) {}
        appState.connected = false;
        appState.connectionType = null;
        setConnectionStatus('disconnected');
        stopLatencyCheck();
        hidePermissionWarning();
        showSuccess(t('msgDisconnectSuccess'));
    } catch (error) {
        showError('断开连接错误: ' + error);
    }
}

// 连接到特定设备
async function connectToDevice(deviceId) {
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
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
    if (!invoke) { showError('Tauri 环境未就绪'); return; }

    appState.scanning = true;
    setScanningState(true);

    try {
        const devices = await invoke('discover_devices');
        if (devices.length === 0) {
            // 扫描成功但未发现其他设备
            updateDevicesList([]);
        } else {
            updateDevicesList(devices);
        }
    } catch (error) {
        showError(window.i18n.t('noDevicesAfterScan'));
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
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
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
    elements.settingsModal.classList.remove('hidden');
}

// 关闭设置
function closeSettings() {
    elements.settingsModal.classList.add('hidden');
}

// ─── TLS 指纹相关 ────────────────────────────────────────────────────────────

/** 加载本机 TLS 证书指纹并显示在设置页 */
async function loadLocalTlsFingerprint() {
    if (!elements.localTlsFingerprint || !invoke) return;
    const { t } = window.i18n;
    const fingerprintRow = document.getElementById('tls-fingerprint-row');
    const fingerprintHint = document.getElementById('tls-fingerprint-hint');
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
    } catch (e) {
        elements.localTlsFingerprint.textContent = t('tlsFingerprintLoadFailed');
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

    // 如果 Tauri 可用，同步到后端
    if (invoke) {
        try {
            await invoke('save_settings', { settings: appState.settings });
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
 * 在 loadSettings() 之后调用
 */
function initAppearance() {
    const lang = appState.settings.language || 'zh';
    const theme = appState.settings.theme || 'auto';

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

    // 更新角色徽章
    if (elements.currentRole) {
        const isClient = elements.currentRole.classList.contains('role-client');
        elements.currentRole.textContent = isClient ? t('roleClient') : t('roleServer');
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
            // step2 需要加粗 KVM Share
            if (i === 1) {
                el.innerHTML = text.replace('KVM Share', '<strong>KVM Share</strong>');
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
 * 优先从 Tauri API 读取，降级使用 HTML 静态值
 */
async function initVersion() {
    let version = 'v0.1.4'; // 静态降级值

    try {
        if (window.__TAURI__?.app?.getVersion) {
            const v = await window.__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
        }
    } catch (e) {
        // 降级使用静态值
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
    try {
        const interfaces = await window.Platform.getNetworkInterfaces();
        // 保留默认的"所有网卡"选项，追加具体网卡
        const defaultOption = '<option value="0.0.0.0">所有网卡（0.0.0.0，推荐）</option>';
        const ifaceOptions = interfaces.map(iface =>
            `<option value="${escapeHtml(iface.ip)}">${escapeHtml(iface.name)} (${escapeHtml(iface.ip)})</option>`
        ).join('');
        elements.listenIpSelect.innerHTML = defaultOption + ifaceOptions;
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
        }
    } catch (e) {
        console.warn('获取监听配置失败:', e);
    }

    // 监听变化，实时更新显示
    elements.listenIpSelect.addEventListener('change', updateListenAddressFromInputs);
    elements.listenPortInput.addEventListener('input', updateListenAddressFromInputs);
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
        window.Platform.showNotification('KVM Share', '连接已断开，请手动重连').catch(() => {});
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

// 更新UI状态
function updateUI() {
    setConnectionStatus('disconnected');
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

// 启动应用
document.addEventListener('DOMContentLoaded', initApp);

// 暴露全局函数用于 HTML 内联调用
window.connectToDevice = connectToDevice;
window.reconnectToDevice = reconnectToDevice;
window.reconnectManualIp = reconnectManualIp;
window.closeSettings = closeSettings;
window.saveSettings = saveSettings;
window.clearHotkey = clearHotkey;
window.hideError = hideError;

// ─── 显示器布局配置 ───────────────────────────────────────────────────────

/**
 * 布局方向到触发边缘的映射
 */
const LAYOUT_TRIGGER_EDGE = {
    'LocalLeftOfRemote':  '右边缘',
    'LocalRightOfRemote': '左边缘',
    'LocalAboveRemote':   '下边缘',
    'LocalBelowRemote':   '上边缘',
};

/**
 * 布局方向到预览 CSS 类的映射
 */
const LAYOUT_PREVIEW_CLASS = {
    'LocalLeftOfRemote':  'layout-left',
    'LocalRightOfRemote': 'layout-right',
    'LocalAboveRemote':   'layout-above',
    'LocalBelowRemote':   'layout-below',
};

/**
 * 初始化显示器布局配置 UI
 */
async function initDisplayLayout() {
    const select = document.getElementById('display-layout-select');
    const preview = document.getElementById('display-layout-preview');
    const hintEdge = document.getElementById('trigger-edge-text');
    const localDisplay = document.getElementById('local-display');
    const localSizeEl = document.getElementById('local-display-size');

    if (!select || !preview) return;

    // 加载真实显示器信息
    await refreshMonitors();

    // 加载已保存的布局配置
    try {
        if (invoke) {
            const config = await invoke('get_layout_config');
            if (config && config.layout) {
                select.value = config.layout;
                updateLayoutPreview(config.layout, preview, hintEdge);
            }
            // 显示本机屏幕尺寸（优先使用真实显示器数据）
            if (config && config.local_screen_width && localSizeEl) {
                localSizeEl.textContent = `${config.local_screen_width}×${config.local_screen_height}`;
            }
        }
    } catch (e) {
        // 加载失败使用默认值
    }

    // 监听下拉框变化
    select.addEventListener('change', () => {
        updateLayoutPreview(select.value, preview, hintEdge);
    });

    // 拖拽支持（本机显示器方块）
    if (localDisplay) {
        localDisplay.addEventListener('dragstart', (e) => {
            e.dataTransfer.setData('text/plain', 'local-display');
            preview.classList.add('drag-over');
        });

        localDisplay.addEventListener('dragend', () => {
            preview.classList.remove('drag-over');
        });
    }

    // 预览区域接受拖拽
    if (preview) {
        preview.addEventListener('dragover', (e) => {
            e.preventDefault();
        });

        preview.addEventListener('drop', (e) => {
            e.preventDefault();
            preview.classList.remove('drag-over');

            // 根据拖拽落点位置决定布局方向
            const rect = preview.getBoundingClientRect();
            const relX = e.clientX - rect.left;
            const relY = e.clientY - rect.top;
            const centerX = rect.width / 2;
            const centerY = rect.height / 2;

            let newLayout;
            const dx = relX - centerX;
            const dy = relY - centerY;

            if (Math.abs(dx) > Math.abs(dy)) {
                newLayout = dx < 0 ? 'LocalRightOfRemote' : 'LocalLeftOfRemote';
            } else {
                newLayout = dy < 0 ? 'LocalBelowRemote' : 'LocalAboveRemote';
            }

            select.value = newLayout;
            updateLayoutPreview(newLayout, preview, hintEdge);
        });
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
 * 刷新显示器列表（调用后端 get_monitors）
 */
async function refreshMonitors() {
    const monitorList = document.getElementById('monitor-list');
    const countBadge = document.getElementById('monitor-count-badge');
    const localSizeEl = document.getElementById('local-display-size');
    const { t } = window.i18n;

    if (!monitorList) return;

    try {
        if (!invoke) return;
        const monitors = await invoke('get_monitors');

        // 更新显示器数量徽章
        if (countBadge) countBadge.textContent = monitors.length;

        // 更新本机显示器预览尺寸（使用主显示器）
        const primary = monitors.find(m => m.is_primary) || monitors[0];
        if (primary && localSizeEl) {
            localSizeEl.textContent = `${primary.width}×${primary.height}`;
        }

        // 渲染显示器列表
        if (monitors.length === 0) {
            monitorList.innerHTML = `<div class="monitor-list-empty">-</div>`;
            return;
        }

        monitorList.innerHTML = monitors.map((m, idx) => `
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

    } catch (e) {
        // 降级：使用 window.screen
        if (countBadge) countBadge.textContent = '1';
        if (localSizeEl) localSizeEl.textContent = `${window.screen.width}×${window.screen.height}`;
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
    }
}

/**
 * 更新布局预览和提示文字
 */
function updateLayoutPreview(layout, preview, hintEdge) {
    const { t } = window.i18n;
    // 移除所有布局类
    preview.classList.remove('layout-left', 'layout-right', 'layout-above', 'layout-below');

    // 添加对应布局类
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
}

/**
 * 保存布局配置（在 saveSettings 中调用）
 */
async function saveLayoutConfig() {
    const select = document.getElementById('display-layout-select');
    if (!select || !invoke) return;

    const config = {
        layout: select.value,
        local_screen_width: window.screen.width,
        local_screen_height: window.screen.height,
    };

    try {
        await invoke('set_layout_config', { config });
    } catch (e) {
        console.error('保存布局配置失败:', e);
    }
}

// 在 DOMContentLoaded 后初始化布局配置
document.addEventListener('DOMContentLoaded', () => {
    // 延迟初始化，等待 invoke 可用
    setTimeout(initDisplayLayout, 500);
    setTimeout(initRoleSwitch, 600);
    setTimeout(checkAccessibilityPermission, 800);
});

// ─── macOS 辅助功能权限检测 ───────────────────────────────────────────────

/** 权限轮询定时器 ID */
let accessibilityPollTimer = null;

/**
 * 检测辅助功能权限，未授权时显示全屏遮罩引导
 * Windows 平台不需要此权限，直接跳过
 */
async function checkAccessibilityPermission() {
    const overlay = document.getElementById('accessibility-overlay');
    if (!overlay || !invoke) return;

    // Windows 平台不需要辅助功能权限，隐藏遮罩并返回
    const isWindows = navigator.userAgent.includes('Windows') ||
        (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32');
    if (isWindows) {
        overlay.classList.add('hidden');
        return;
    }

    try {
        const status = await invoke('check_accessibility');
        if (status === 'Denied') {
            overlay.classList.remove('hidden');
            // 启动轮询：每 2 秒检测一次，授权后自动隐藏
            startAccessibilityPolling();
        } else {
            overlay.classList.add('hidden');
            stopAccessibilityPolling();
        }
    } catch (e) {
        // 检测失败，隐藏遮罩（不影响主功能）
        overlay.classList.add('hidden');
    }
}

/**
 * 启动权限轮询（每 2 秒检测一次）
 */
function startAccessibilityPolling() {
    if (accessibilityPollTimer) return; // 已在轮询中
    accessibilityPollTimer = setInterval(async () => {
        try {
            const status = await invoke('check_accessibility');
            if (status !== 'Denied') {
                // 权限已授权，隐藏全屏遮罩并停止轮询
                const overlay = document.getElementById('accessibility-overlay');
                if (overlay) overlay.classList.add('hidden');
                stopAccessibilityPolling();
                // 权限授予后，若已连接则隐藏权限提示并恢复开始共享按钮
                if (appState.connected) {
                    hidePermissionWarning();
                }
            }
        } catch (e) {
            // 忽略轮询错误
        }
    }, 2000);
}

/**
 * 任务3：连接成功后检查辅助功能权限
 * - macOS 且权限 Denied：显示提示，禁用开始共享按钮
 * - macOS 且权限 Granted / Windows：隐藏提示，启用开始共享按钮
 */
async function checkPermissionAfterConnect() {
    // 显示开始共享按钮
    if (elements.startSharingBtn) {
        elements.startSharingBtn.classList.remove('hidden');
    }

    // Windows 平台不需要权限检查
    const isWindows = navigator.userAgent.includes('Windows') ||
        (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32');
    if (isWindows || !invoke) {
        hidePermissionWarning();
        return;
    }

    try {
        const status = await invoke('check_accessibility');
        if (status === 'Denied') {
            showPermissionWarning();
        } else {
            hidePermissionWarning();
        }
    } catch (e) {
        // 检测失败，不影响主功能
        hidePermissionWarning();
    }
}

/**
 * 显示连接后权限不足提示，禁用开始共享按钮
 */
function showPermissionWarning() {
    if (elements.permissionWarning) {
        elements.permissionWarning.classList.remove('hidden');
    }
    if (elements.startSharingBtn) {
        elements.startSharingBtn.disabled = true;
    }
}

/**
 * 隐藏连接后权限不足提示，恢复开始共享按钮
 */
function hidePermissionWarning() {
    if (elements.permissionWarning) {
        elements.permissionWarning.classList.add('hidden');
    }
    if (elements.startSharingBtn) {
        elements.startSharingBtn.disabled = false;
        // 断开连接时也隐藏开始共享按钮
        if (!appState.connected) {
            elements.startSharingBtn.classList.add('hidden');
        }
    }
}

/**
 * 停止权限轮询
 */
function stopAccessibilityPolling() {
    if (accessibilityPollTimer) {
        clearInterval(accessibilityPollTimer);
        accessibilityPollTimer = null;
    }
}

/**
 * 打开辅助功能设置页面
 */
async function openAccessibilitySettings() {
    if (!invoke) return;

    try {
        await invoke('open_accessibility_prefs');
        // 3 秒后重新检测权限（用户可能已授权）
        setTimeout(checkAccessibilityPermission, 3000);
    } catch (e) {
        showError(`无法打开系统偏好设置: ${e}`);
    }
}

// 暴露全局函数
window.openAccessibilitySettings = openAccessibilitySettings;

// ─── 角色切换 ─────────────────────────────────────────────────────────────

/**
 * 当前角色状态
 */
let currentRole = 'Server'; // 'Server' | 'Client'

/**
 * 初始化角色切换 UI
 */
async function initRoleSwitch() {
    const switchBtn = document.getElementById('switch-role-btn');
    if (!switchBtn) return;

    // 加载当前角色
    try {
        if (invoke) {
            const role = await invoke('get_current_role');
            currentRole = role;
            updateRoleUI(role);
        }
    } catch (e) {
        // 使用默认值
    }

    // 监听切换按钮
    switchBtn.addEventListener('click', async () => {
        const newRole = currentRole === 'Server' ? 'Client' : 'Server';
        await switchRole(newRole);
    });
}

/**
 * 切换角色
 */
async function switchRole(newRole) {
    if (!invoke) return;
    const { t } = window.i18n;

    try {
        await invoke('set_role', { newRole });
        currentRole = newRole;
        updateRoleUI(newRole);

        const successMsg = newRole === 'Server' ? t('msgSwitchedToServer') : t('msgSwitchedToClient');
        showSuccess(successMsg);

        // 断开当前连接
        if (appState.connected) {
            await disconnectDevices();
        }
    } catch (e) {
        showError(`角色切换失败: ${e}`);
    }
}

/**
 * 更新角色 UI 显示
 */
function updateRoleUI(role) {
    const roleBadge = document.getElementById('current-role');
    if (!roleBadge) return;
    const { t } = window.i18n;

    if (role === 'Server') {
        roleBadge.textContent = t('roleServer');
        roleBadge.className = 'role-badge role-server';
    } else {
        roleBadge.textContent = t('roleClient');
        roleBadge.className = 'role-badge role-client';
    }
}