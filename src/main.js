// 导入Tauri API
const { invoke } = window.__TAURI__.tauri;

// 全局状态
let appState = {
    connected: false,
    connectionType: null,
    devices: [],
    currentDevice: null
};

// DOM元素引用
const elements = {
    connectionStatus: document.getElementById('connection-status'),
    connectionType: document.getElementById('connection-type'),
    latency: document.getElementById('latency'),
    connectBtn: document.getElementById('connect-btn'),
    disconnectBtn: document.getElementById('disconnect-btn'),
    settingsBtn: document.getElementById('settings-btn'),
    devicesList: document.getElementById('devices-list')
};

// 初始化应用
function initApp() {
    setupEventListeners();
    updateUI();
    
    // 启动设备发现
    startDeviceDiscovery();
}

// 设置事件监听器
function setupEventListeners() {
    elements.connectBtn.addEventListener('click', handleConnect);
    elements.disconnectBtn.addEventListener('click', handleDisconnect);
    elements.settingsBtn.addEventListener('click', handleSettings);
}

// 处理连接按钮点击
async function handleConnect() {
    try {
        setConnectionStatus('connecting');
        
        // 调用Rust后端连接逻辑
        const result = await invoke('connect_devices');
        
        if (result.success) {
            appState.connected = true;
            appState.connectionType = result.connectionType;
            setConnectionStatus('connected');
            startLatencyCheck();
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
    try {
        await invoke('disconnect_devices');
        appState.connected = false;
        appState.connectionType = null;
        setConnectionStatus('disconnected');
        stopLatencyCheck();
    } catch (error) {
        showError('断开连接错误: ' + error);
    }
}

// 处理设置
function handleSettings() {
    // 打开设置对话框
    alert('设置功能开发中...');
}

// 设置连接状态
function setConnectionStatus(status) {
    elements.connectionStatus.textContent = 
        status === 'connected' ? '已连接' :
        status === 'connecting' ? '连接中...' : '断开';
    
    elements.connectionStatus.className = 
        status === 'connected' ? 'status-connected' :
        status === 'connecting' ? 'status-connecting' : 'status-disconnected';
    
    // 更新按钮状态
    elements.connectBtn.disabled = status !== 'disconnected';
    elements.disconnectBtn.disabled = status !== 'connected';
}

// 启动设备发现
async function startDeviceDiscovery() {
    try {
        // 定期发现设备
        setInterval(async () => {
            if (!appState.connected) {
                const devices = await invoke('discover_devices');
                updateDevicesList(devices);
            }
        }, 5000); // 每5秒发现一次
    } catch (error) {
        console.error('设备发现错误:', error);
    }
}

// 更新设备列表
function updateDevicesList(devices) {
    appState.devices = devices;
    
    if (devices.length === 0) {
        elements.devicesList.innerHTML = '<div class="no-devices">未发现设备</div>';
        return;
    }
    
    elements.devicesList.innerHTML = devices.map(device => `
        <div class="device-item">
            <div class="device-info">
                <div class="device-name">${device.name}</div>
                <div class="device-details">
                    ${device.os} | ${device.ip} | ${device.connectionType}
                </div>
            </div>
            <button class="connect-device-btn" onclick="connectToDevice('${device.id}')">
                连接
            </button>
        </div>
    `).join('');
}

// 连接到特定设备
async function connectToDevice(deviceId) {
    try {
        setConnectionStatus('connecting');
        const result = await invoke('connect_to_device', { deviceId });
        
        if (result.success) {
            appState.connected = true;
            appState.connectionType = result.connectionType;
            appState.currentDevice = deviceId;
            setConnectionStatus('connected');
            startLatencyCheck();
        } else {
            setConnectionStatus('disconnected');
            showError('连接失败: ' + result.error);
        }
    } catch (error) {
        setConnectionStatus('disconnected');
        showError('连接错误: ' + error);
    }
}

// 启动延迟检查
function startLatencyCheck() {
    // 实现延迟检测逻辑
    elements.latency.textContent = '检测中...';
    
    // 模拟延迟检测
    setTimeout(() => {
        elements.latency.textContent = '5ms';
    }, 1000);
}

// 停止延迟检查
function stopLatencyCheck() {
    elements.latency.textContent = '-';
}

// 显示错误信息
function showError(message) {
    // 实现错误提示UI
    console.error('Error:', message);
    alert('错误: ' + message);
}

// 更新UI状态
function updateUI() {
    setConnectionStatus('disconnected');
}

// 启动应用
document.addEventListener('DOMContentLoaded', initApp);

// 暴露全局函数用于HTML调用
window.connectToDevice = connectToDevice;