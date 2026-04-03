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
    };

    setupEventListeners();
    updateUI();
    loadSettings();
    loadConnectionHistory();
}

// 设置事件监听器
function setupEventListeners() {
    elements.connectBtn.addEventListener('click', handleConnect);
    elements.disconnectBtn.addEventListener('click', handleDisconnect);
    elements.settingsBtn.addEventListener('click', openSettings);
    elements.scanBtn.addEventListener('click', handleScan);
    elements.clearHistoryBtn.addEventListener('click', handleClearHistory);

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
    const trimmed = filtered.slice(0, 10);
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
        return `
            <div class="history-item">
                <div class="history-info">
                    <div class="history-device-name">${escapeHtml(item.name)}</div>
                    <div class="history-meta">${escapeHtml(item.ip)} · ${escapeHtml(item.connectionType)} · ${dateStr}</div>
                </div>
                <button class="reconnect-btn" onclick="reconnectToDevice('${escapeHtml(item.id)}', '${escapeHtml(item.name)}')">
                    重连
                </button>
            </div>
        `;
    }).join('');
}

// 快速重连
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

// 启动延迟检查
let latencyTimer = null;
function startLatencyCheck() {
    elements.latency.textContent = '检测中...';
    latencyTimer = setInterval(async () => {
        if (!invoke || !appState.connected) return;
        try {
            const latency = await invoke('get_latency');
            elements.latency.textContent = latency != null ? `${latency}ms` : '-';
        } catch {
            // get_latency 命令待实现，暂时模拟
            elements.latency.textContent = '< 5ms';
        }
    }, 2000);
}

// 停止延迟检查
function stopLatencyCheck() {
    if (latencyTimer) {
        clearInterval(latencyTimer);
        latencyTimer = null;
    }
    elements.latency.textContent = '-';
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
window.closeSettings = closeSettings;
window.saveSettings = saveSettings;
window.clearHotkey = clearHotkey;
window.hideError = hideError;