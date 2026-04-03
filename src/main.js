// 导入Tauri API
let invoke;

// 等待 Tauri API 就绪
function waitForTauri() {
    return new Promise((resolve) => {
        if (window.__TAURI__) {
            resolve(window.__TAURI__.tauri.invoke);
            return;
        }
        const timer = setInterval(() => {
            if (window.__TAURI__) {
                clearInterval(timer);
                resolve(window.__TAURI__.tauri.invoke);
            }
        }, 50);
    });
}

// 全局状态
let appState = {
    connected: false,
    connectionType: null,
    devices: [],
    currentDevice: null,
    latencyTimer: null,
    discoveryTimer: null,
};

// DOM元素引用
const elements = {
    connectionStatus: document.getElementById('connection-status'),
    connectionType: document.getElementById('connection-type'),
    latency: document.getElementById('latency'),
    localIp: document.getElementById('local-ip'),
    connectedDevice: document.getElementById('connected-device'),
    connectBtn: document.getElementById('connect-btn'),
    disconnectBtn: document.getElementById('disconnect-btn'),
    settingsBtn: document.getElementById('settings-btn'),
    devicesList: document.getElementById('devices-list'),
    errorBanner: document.getElementById('error-banner'),
    errorMsg: document.getElementById('error-msg'),
    errorClose: document.getElementById('error-close'),
};

// 初始化应用
async function initApp() {
    setupEventListeners();
    updateUI();
    loadLocalIp();
    startDeviceDiscovery();
}

// 获取并显示本机 IP
async function loadLocalIp() {
    try {
        const ip = await invoke('get_local_ip');
        elements.localIp.textContent = ip || '获取失败';
    } catch (error) {
        elements.localIp.textContent = '获取失败';
        console.error('获取本机IP失败:', error);
    }
}

// 设置事件监听器
function setupEventListeners() {
    elements.connectBtn.addEventListener('click', handleConnect);
    elements.disconnectBtn.addEventListener('click', handleDisconnect);
    elements.settingsBtn.addEventListener('click', handleSettings);
    if (elements.errorClose) {
        elements.errorClose.addEventListener('click', hideError);
    }
}

// 处理扫描按钮点击：主动触发一次设备扫描
async function handleConnect() {
    elements.connectBtn.disabled = true;
    elements.connectBtn.textContent = '扫描中...';
    try {
        await runDiscovery();
    } finally {
        elements.connectBtn.disabled = false;
        elements.connectBtn.textContent = '扫描设备';
    }
}

// 处理断开连接
async function handleDisconnect() {
    try {
        await invoke('disconnect_devices');
        // 停止输入捕获
        try { await invoke('stop_input_capture'); } catch (_) {}
        appState.connected = false;
        appState.connectionType = null;
        appState.currentDevice = null;
        setConnectionStatus('disconnected');
        stopLatencyCheck();
        updateConnectedDeviceInfo(null);
    } catch (error) {
        showError('断开连接错误: ' + error);
    }
}

// 处理设置
function handleSettings() {
    alert('设置功能开发中...');
}

// ============================================================
// 连接状态管理
// ============================================================

/**
 * 设置连接状态
 * @param {'disconnected'|'discovering'|'handshaking'|'connected'} status
 * @param {string} [detail] 附加说明文字
 */
function setConnectionStatus(status, detail) {
    const labels = {
        disconnected: '断开',
        discovering: '发现中...',
        handshaking: '握手中...',
        connected: '已连接',
    };
    const text = labels[status] || status;
    elements.connectionStatus.textContent = detail ? `${text} - ${detail}` : text;
    elements.connectionStatus.className = `status-${status === 'connected' ? 'connected' : status === 'disconnected' ? 'disconnected' : 'connecting'}`;

    elements.connectBtn.disabled = status === 'connected';
    elements.disconnectBtn.disabled = status !== 'connected';
}

// ============================================================
// 设备发现
// ============================================================

function startDeviceDiscovery() {
    runDiscovery();
    appState.discoveryTimer = setInterval(runDiscovery, 3000);
}

async function runDiscovery() {
    if (appState.connected) return;
    try {
        const devices = await invoke('discover_devices');
        updateDevicesList(devices);
    } catch (error) {
        console.error('设备发现错误:', error);
    }
}

// 更新设备列表（修复 undefined 字段）
function updateDevicesList(devices) {
    appState.devices = devices || [];

    if (!devices || devices.length === 0) {
        elements.devicesList.innerHTML = '<div class="no-devices">未发现设备，等待 KVM Agent 响应...</div>';
        return;
    }

    elements.devicesList.innerHTML = devices.map(device => {
        // 安全取值，避免 undefined 显示
        const name = device.name || device.hostname || '未知设备';
        const os = device.os || '未知系统';
        const ip = device.ip || '未知IP';
        const connType = formatConnectionType(device.connectionType);

        return `
        <div class="device-item" id="device-${device.id}">
            <div class="device-info">
                <div class="device-name">${escapeHtml(name)}</div>
                <div class="device-details">
                    <span class="device-os">${escapeHtml(os)}</span>
                    <span class="device-sep">|</span>
                    <span class="device-ip">${escapeHtml(ip)}</span>
                    <span class="device-sep">|</span>
                    <span class="device-type">${connType}</span>
                </div>
            </div>
            <button class="connect-device-btn" onclick="connectToDevice('${escapeHtml(device.id)}')">
                连接
            </button>
        </div>`;
    }).join('');
}

function formatConnectionType(type) {
    if (!type) return '网络';
    if (typeof type === 'string') {
        if (type === 'Network' || type === 'network') return '网络';
        if (type === 'Usb' || type === 'usb') return 'USB';
    }
    return '网络';
}

function escapeHtml(str) {
    if (!str) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

// ============================================================
// 连接到特定设备（含进度显示）
// ============================================================

async function connectToDevice(deviceId) {
    hideError();
    try {
        // 步骤1：发现
        setConnectionStatus('discovering', '正在定位设备...');
        await sleep(200); // 给用户视觉反馈

        // 步骤2：握手
        setConnectionStatus('handshaking', '正在握手...');

        const result = await invoke('connect_to_device', { deviceId });

        if (result.success) {
            // 步骤3：已连接
            appState.connected = true;
            appState.connectionType = result.connectionType;
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');

            // 安装输入捕获钩子
            try {
                await invoke('start_input_capture');
                console.log('[InputCapture] 全局输入钩子已安装');
            } catch (e) {
                console.warn('[InputCapture] 安装钩子失败:', e);
            }

            // 获取已连接设备信息
            try {
                const info = await invoke('get_connected_device_info');
                updateConnectedDeviceInfo(info);
            } catch (_) {}

            startLatencyCheck();
        } else {
            setConnectionStatus('disconnected');
            const errMsg = result.error || '连接失败';
            showError(formatConnectionError(errMsg));
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError(formatConnectionError(String(error)));
    }
}

function formatConnectionError(raw) {
    if (!raw) return '未知错误';
    if (raw.includes('未运行') || raw.includes('KVM Agent')) {
        return '目标设备未运行 KVM Agent 程序，请先在 Mac 上启动 kvm-agent';
    }
    if (raw.includes('不可达') || raw.includes('TimedOut') || raw.includes('timeout')) {
        return '网络不可达或连接超时，请检查两台设备是否在同一局域网';
    }
    if (raw.includes('ConnectionRefused') || raw.includes('refused')) {
        return '目标设备拒绝连接，请确认 kvm-agent 正在运行';
    }
    if (raw.includes('重连次数耗尽')) {
        return '连接失败（已重试3次），请检查网络或重启 kvm-agent';
    }
    return raw;
}

// ============================================================
// 已连接设备信息显示
// ============================================================

function updateConnectedDeviceInfo(info) {
    if (!elements.connectedDevice) return;
    if (!info) {
        elements.connectedDevice.textContent = '-';
        elements.connectionType.textContent = '-';
        return;
    }
    const name = info.hostname || info.ip || '已连接设备';
    const os = info.os || '';
    elements.connectedDevice.textContent = os ? `${name} (${os})` : name;
    elements.connectionType.textContent = '网络';
}

// ============================================================
// 延迟检测
// ============================================================

function startLatencyCheck() {
    stopLatencyCheck();
    elements.latency.textContent = '检测中...';
    appState.latencyTimer = setInterval(async () => {
        if (!appState.connected) {
            stopLatencyCheck();
            return;
        }
        try {
            const latency = await invoke('get_connection_latency');
            if (latency !== null && latency !== undefined) {
                elements.latency.textContent = `${latency} ms`;
            } else {
                elements.latency.textContent = '- ms';
            }
        } catch (_) {
            elements.latency.textContent = '- ms';
        }
    }, 2000);
}

function stopLatencyCheck() {
    if (appState.latencyTimer) {
        clearInterval(appState.latencyTimer);
        appState.latencyTimer = null;
    }
    elements.latency.textContent = '-';
}

// ============================================================
// 错误提示
// ============================================================

function showError(message) {
    console.error('[Error]', message);
    if (elements.errorBanner && elements.errorMsg) {
        elements.errorMsg.textContent = message;
        elements.errorBanner.style.display = 'flex';
    } else {
        // 降级到 alert
        alert('错误: ' + message);
    }
}

function hideError() {
    if (elements.errorBanner) {
        elements.errorBanner.style.display = 'none';
    }
}

// ============================================================
// 工具函数
// ============================================================

function updateUI() {
    setConnectionStatus('disconnected');
}

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// ============================================================
// 启动
// ============================================================

document.addEventListener('DOMContentLoaded', async () => {
    invoke = await waitForTauri();
    initApp();
});

// 暴露全局函数用于HTML调用
window.connectToDevice = connectToDevice;