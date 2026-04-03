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
        connectBtn: document.getElementById('connect-btn'),
        disconnectBtn: document.getElementById('disconnect-btn'),
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
    };

    setupEventListeners();
    updateUI();
    loadSettings();
    loadConnectionHistory();
    loadLocalIp();
}

// 设置事件监听器
function setupEventListeners() {
    elements.connectBtn.addEventListener('click', handleConnect);
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
    elements.hotkeyInput.addEventListener('focus', () => {
        elements.hotkeyInput.value = '请按下快捷键组合...';
    });
    elements.hotkeyInput.addEventListener('blur', () => {
        if (!elements.hotkeyInput.dataset.hotkey) {
            elements.hotkeyInput.value = '';
        } else {
            elements.hotkeyInput.value = elements.hotkeyInput.dataset.hotkey;
        }
    });
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
            elements.localIp.textContent = '无法获取';
        }
    } catch (e) {
        elements.localIp.textContent = '无法获取';
    }
}

// ─── 连接管理 ───

// 处理连接按钮点击
async function handleConnect() {
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
    try {
        setConnectionStatus('connecting');
        const result = await invoke('connect_devices');
        if (result.success) {
            appState.connected = true;
            appState.connectionType = result.connectionType;
            setConnectionStatus('connected');
            startLatencyCheck();
            showSuccess('设备连接成功');
        } else {
            setConnectionStatus('disconnected');
            showError('连接失败: ' + result.error);
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError('连接错误: ' + error);
    }
}

// 处理断开连接
async function handleDisconnect() {
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
    try {
        await invoke('disconnect_devices');
        appState.connected = false;
        appState.connectionType = null;
        setConnectionStatus('disconnected');
        stopLatencyCheck();
        showSuccess('已断开连接');
    } catch (error) {
        showError('断开连接错误: ' + error);
    }
}

// 连接到特定设备
async function connectToDevice(deviceId) {
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
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
            showSuccess(`已连接到 ${device?.name || deviceId}`);
        } else {
            setConnectionStatus('disconnected');
            showError('连接失败: ' + result.error);
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError('连接错误: ' + error);
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

    // 前端 IPv4 格式校验
    if (!ip) {
        elements.manualIpInput.classList.add('input-error');
        showManualConnectStatus('error', '请输入目标设备的 IP 地址');
        elements.manualIpInput.focus();
        return;
    }
    if (!isValidIpv4(ip)) {
        elements.manualIpInput.classList.add('input-error');
        showManualConnectStatus('error', `IP 格式不正确："${ip}"，请输入合法的 IPv4 地址（如 192.168.1.100）`);
        elements.manualIpInput.focus();
        return;
    }
    elements.manualIpInput.classList.remove('input-error');

    // 端口范围校验
    if (port < 1024 || port > 65535) {
        showManualConnectStatus('error', '端口范围应为 1024-65535');
        elements.manualPortInput.focus();
        return;
    }

    // 开始连接
    elements.manualConnectBtn.disabled = true;
    showManualConnectStatus('connecting', `⏳ 正在连接 ${ip}:${port}，最长等待 5 秒...`);

    try {
        const result = await window.Platform.connectByIp(ip, port);
        if (result.success) {
            appState.connected = true;
            appState.connectionType = 'Network';
            appState.currentDevice = `manual:${ip}:${port}`;
            setConnectionStatus('connected');
            startLatencyCheck();
            showManualConnectStatus('success', `✅ 已成功连接到 ${ip}:${port}`);
            showSuccess(`已连接到 ${ip}`);
            // 保存到历史记录（手动 IP 类型）
            saveManualIpToHistory(ip, port);
        } else {
            showManualConnectStatus('error', result.error_message || result.errorMessage || '连接失败');
            setConnectionStatus('disconnected');
        }
    } catch (e) {
        showManualConnectStatus('error', `连接出错：${e}`);
        setConnectionStatus('disconnected');
    } finally {
        elements.manualConnectBtn.disabled = false;
    }
}

// ─── 设备扫描 ───

// 处理扫描按钮
async function handleScan() {
    if (appState.scanning) return;
    if (!invoke) { showError('Tauri 环境未就绪'); return; }

    appState.scanning = true;
    setScanningState(true);

    try {
        const devices = await invoke('discover_devices');
        updateDevicesList(devices);
    } catch (error) {
        showError('扫描设备失败: ' + error);
        updateDevicesList([]);
    } finally {
        appState.scanning = false;
        setScanningState(false);
    }
}

// 设置扫描状态 UI
function setScanningState(scanning) {
    elements.scanBtn.classList.toggle('scanning', scanning);
    elements.scanBtn.disabled = scanning;
    elements.scanIcon.textContent = scanning ? '⏳' : '🔍';
    elements.scanText.textContent = scanning ? '扫描中...' : '扫描设备';
    elements.scanAnimation.classList.toggle('hidden', !scanning);
}

// 更新设备列表
function updateDevicesList(devices) {
    appState.devices = devices;

    if (devices.length === 0) {
        elements.devicesList.innerHTML = '<div class="no-devices">未发现设备，请确保目标设备在同一局域网内</div>';
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
                连接
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
    if (history.length === 0) {
        elements.historyList.innerHTML = '<div class="no-history">暂无连接历史</div>';
        return;
    }

    elements.historyList.innerHTML = history.map(item => {
        const date = new Date(item.lastConnected);
        const dateStr = `${date.getMonth() + 1}/${date.getDate()} ${date.getHours()}:${String(date.getMinutes()).padStart(2, '0')}`;
        // 区分手动 IP 和自动发现设备
        const isManual = item.entryType === 'manual';
        const icon = isManual ? '🔗' : '📡';
        const typeLabel = isManual ? '手动 IP' : (item.connectionType || 'Network');
        const reconnectAttr = isManual
            ? `onclick="reconnectManualIp('${escapeHtml(item.ip)}', ${item.port || 8889})"`
            : `onclick="reconnectToDevice('${escapeHtml(item.id)}', '${escapeHtml(item.name)}')"`;
        return `
            <div class="history-item">
                <div class="history-info">
                    <div class="history-device-name">${icon} ${escapeHtml(item.name)}</div>
                    <div class="history-meta">${escapeHtml(item.ip)} · ${escapeHtml(typeLabel)} · ${dateStr}</div>
                </div>
                <button class="reconnect-btn" ${reconnectAttr}>
                    重连
                </button>
            </div>
        `;
    }).join('');
}

// 快速重连（自动发现设备）
async function reconnectToDevice(deviceId, deviceName) {
    if (!invoke) { showError('Tauri 环境未就绪'); return; }
    try {
        setConnectionStatus('connecting');
        const result = await invoke('connect_to_device', { deviceId });
        if (result.success) {
            appState.connected = true;
            appState.connectionType = result.connectionType;
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');
            startLatencyCheck();
            showSuccess(`已重连到 ${deviceName}`);
        } else {
            setConnectionStatus('disconnected');
            showError('重连失败: ' + result.error);
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError('重连错误: ' + error);
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

// 将设置加载到 UI
function loadSettingsToUI() {
    elements.hotkeyInput.value = appState.settings.hotkey;
    elements.hotkeyInput.dataset.hotkey = appState.settings.hotkey;
    elements.preferConnectionType.value = appState.settings.preferConnectionType;
    elements.autoReconnect.checked = appState.settings.autoReconnect;
    elements.autoStart.checked = appState.settings.autoStart;
    elements.startMinimized.checked = appState.settings.startMinimized;
}

// 保存设置
async function saveSettings() {
    appState.settings = {
        hotkey: elements.hotkeyInput.dataset.hotkey || '',
        preferConnectionType: elements.preferConnectionType.value,
        autoReconnect: elements.autoReconnect.checked,
        autoStart: elements.autoStart.checked,
        startMinimized: elements.startMinimized.checked,
    };
    localStorage.setItem('kvm_settings', JSON.stringify(appState.settings));

    // 如果 Tauri 可用，同步到后端
    if (invoke) {
        try {
            await invoke('save_settings', { settings: appState.settings });
        } catch (e) {
            console.warn('后端设置保存失败（功能待实现）:', e);
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
    }

    closeSettings();
    showSuccess('设置已保存');
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
    const localIps = elements.localIp?.textContent;
    if (ip === '0.0.0.0' && localIps && localIps !== '获取中...' && localIps !== '无法获取') {
        elements.listenAddressDisplay.textContent = `${localIps.split(' / ')[0]}:${port}（或其他本机 IP）`;
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
    const statusMap = {
        connected: '已连接',
        connecting: '连接中...',
        disconnected: '断开',
    };
    const classMap = {
        connected: 'status-connected',
        connecting: 'status-connecting',
        disconnected: 'status-disconnected',
    };

    elements.connectionStatus.textContent = statusMap[status] || '断开';
    elements.connectionStatus.className = classMap[status] || 'status-disconnected';

    if (status === 'connected' && appState.connectionType) {
        elements.connectionType.textContent = appState.connectionType;
    } else if (status === 'disconnected') {
        elements.connectionType.textContent = '-';
    }

    elements.connectBtn.disabled = status !== 'disconnected';
    elements.disconnectBtn.disabled = status !== 'connected';
}

// 启动延迟检查（含警告色和断线重连）
let latencyTimer = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 3;
const RECONNECT_INTERVAL_MS = 2000;

function startLatencyCheck() {
    elements.latency.textContent = '检测中...';
    elements.latency.className = '';
    reconnectAttempts = 0;
    latencyTimer = setInterval(async () => {
        if (!invoke || !appState.connected) return;
        try {
            const latency = await invoke('get_latency');
            if (latency != null) {
                elements.latency.textContent = `${latency}ms`;
                // 延迟 > 50ms 显示黄色警告
                if (latency > 50) {
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
        showError(`连接已断开，自动重连 ${MAX_RECONNECT_ATTEMPTS} 次均失败，请手动重连`);
        window.Platform.showNotification('KVM Share', '连接已断开，请手动重连').catch(() => {});
        return;
    }

    reconnectAttempts++;
    elements.latency.textContent = `重连中(${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})...`;
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
                showSuccess(`已重连到 ${ip}`);
            }
        } else {
            // 自动发现设备重连
            result = await invoke('connect_to_device', { deviceId: appState.currentDevice });
            if (result.success) {
                reconnectAttempts = 0;
                elements.latency.style.color = '#10b981';
                showSuccess('连接已恢复');
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

    // 加载已保存的布局配置
    try {
        if (invoke) {
            const config = await invoke('get_layout_config');
            if (config && config.layout) {
                select.value = config.layout;
                updateLayoutPreview(config.layout, preview, hintEdge);
            }
            // 显示本机屏幕尺寸
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
                // 水平方向
                newLayout = dx < 0 ? 'LocalRightOfRemote' : 'LocalLeftOfRemote';
            } else {
                // 垂直方向
                newLayout = dy < 0 ? 'LocalBelowRemote' : 'LocalAboveRemote';
            }

            select.value = newLayout;
            updateLayoutPreview(newLayout, preview, hintEdge);
        });
    }
}

/**
 * 更新布局预览和提示文字
 */
function updateLayoutPreview(layout, preview, hintEdge) {
    // 移除所有布局类
    preview.classList.remove('layout-left', 'layout-right', 'layout-above', 'layout-below');

    // 添加对应布局类
    const cls = LAYOUT_PREVIEW_CLASS[layout];
    if (cls) preview.classList.add(cls);

    // 更新提示文字
    if (hintEdge) {
        hintEdge.textContent = LAYOUT_TRIGGER_EDGE[layout] || '右边缘';
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

/**
 * 检测辅助功能权限，未授权时显示引导横幅
 */
async function checkAccessibilityPermission() {
    const banner = document.getElementById('accessibility-banner');
    if (!banner || !invoke) return;

    try {
        const status = await invoke('check_accessibility');
        if (status === 'Denied') {
            banner.classList.remove('hidden');
        } else {
            banner.classList.add('hidden');
        }
    } catch (e) {
        // 检测失败，隐藏横幅（不影响主功能）
        banner.classList.add('hidden');
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

    try {
        await invoke('set_role', { newRole });
        currentRole = newRole;
        updateRoleUI(newRole);

        const roleName = newRole === 'Server' ? '被控端' : '主控端';
        showSuccess(`已切换为${roleName}模式`);

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

    if (role === 'Server') {
        roleBadge.textContent = '被控端';
        roleBadge.className = 'role-badge role-server';
    } else {
        roleBadge.textContent = '主控端';
        roleBadge.className = 'role-badge role-client';
    }
}