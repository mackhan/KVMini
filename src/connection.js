/**
 * connection.js - 连接管理模块
 *
 * 封装所有连接相关的业务逻辑：
 * - IP/端口格式校验（纯函数，可测试）
 * - 手动 IP 连接
 * - 断开/重连操作
 * - 设备连接/历史重连
 */

// ─── 纯函数：格式校验 ────────────────────────────────────────────────────────

/**
 * 校验 IPv4 地址格式
 * @param {string} ip
 * @returns {boolean}
 */
function isValidIpv4(ip) {
    if (!ip || typeof ip !== 'string') return false;
    const parts = ip.split('.');
    if (parts.length !== 4) return false;
    return parts.every(p => {
        if (!/^\d+$/.test(p)) return false;
        const n = Number(p);
        return n >= 0 && n <= 255;
    });
}

/**
 * 校验端口范围（1024-65535）
 * @param {number} port
 * @returns {boolean}
 */
function isValidPort(port) {
    if (typeof port !== 'number' || !Number.isInteger(port)) return false;
    return port >= 1024 && port <= 65535;
}

// ─── 连接操作 ────────────────────────────────────────────────────────────────

/**
 * 处理手动 IP 连接
 * @param {object} elements - DOM 元素引用
 * @param {object} appState - 应用状态
 * @param {Function} invoke - Tauri invoke 函数
 */
async function handleManualConnect(elements, appState, invoke) {
    const ip = elements.manualIpInput.value.trim();
    const port = parseInt(elements.manualPortInput.value, 10) || 23456;
    const { t } = window.i18n;

    // 前端 IPv4 格式校验
    if (!ip) {
        elements.manualIpInput.classList.add('input-error');
        showManualConnectStatus(elements, 'error', t('errIpEmpty'));
        elements.manualIpInput.focus();
        return;
    }
    if (!isValidIpv4(ip)) {
        elements.manualIpInput.classList.add('input-error');
        showManualConnectStatus(elements, 'error', t('errIpFormat').replace('{ip}', ip));
        elements.manualIpInput.focus();
        return;
    }
    elements.manualIpInput.classList.remove('input-error');

    // 端口范围校验
    if (!isValidPort(port)) {
        showManualConnectStatus(elements, 'error', t('errPortRange'));
        elements.manualPortInput.focus();
        return;
    }

    // 开始连接
    elements.manualConnectBtn.disabled = true;
    showManualConnectStatus(elements, 'connecting', t('connectingTo').replace('{ip}', ip).replace('{port}', port));

    try {
        const result = await window.Platform.connectByIp(ip, port);
        if (result.success) {
            appState.connected = true;
            appState.connectionType = 'Network';
            appState.currentDevice = `manual:${ip}:${port}`;
            const platformInfo = result.remote_platform ? ` (${result.remote_platform})` : '';
            showManualConnectStatus(elements, 'success',
                t('connectedTo').replace('{ip}', ip).replace('{port}', port) + platformInfo);
        } else {
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
            showManualConnectStatus(elements, 'error', errorMsg);
        }
    } catch (e) {
        showManualConnectStatus(elements, 'error', t('connectErrGeneric').replace('{err}', e));
    } finally {
        elements.manualConnectBtn.disabled = false;
    }
}

/**
 * 处理断开连接
 * @param {object} elements - DOM 元素引用
 * @param {object} appState - 应用状态
 * @param {Function} invoke - Tauri invoke 函数
 */
async function handleDisconnect(elements, appState, invoke) {
    if (!invoke) { return; }
    try {
        await invoke('disconnect_peer').catch(() => {});
        appState.connected = false;
        appState.connectionType = null;
    } catch (error) {
        console.error('断开连接错误:', error);
    }
}

/**
 * 处理手动重新连接
 * @param {object} elements - DOM 元素引用
 * @param {Function} invoke - Tauri invoke 函数
 */
async function handleReconnect(elements, invoke) {
    if (!invoke) { return; }
    try {
        await invoke('reconnect_to_peer').catch(() => {});
    } catch (error) {
        console.error('重新连接失败:', error);
    }
}

/**
 * 连接到特定设备（自动发现）
 * BUG-020 修复：后端命令名是 connect_to_peer，参数为 peerIp + port。
 * deviceId 本身是自动发现设备唯一 id，但连接必须走 IP，所以此函数
 * 会从 appState.devices 找回 device.ip。
 * @param {string} deviceId
 * @param {object} appState - 应用状态（必须包含 devices 与 defaults.tcpPort）
 * @param {Function} invoke - Tauri invoke 函数
 * @returns {Promise<{success: boolean, error?: string}>}
 */
async function connectToDevice(deviceId, appState, invoke) {
    if (!invoke) { return { success: false, error: 'Tauri 环境未就绪' }; }
    try {
        const device = (appState?.devices || []).find(d => d.id === deviceId);
        const peerIp = device?.ip;
        if (!peerIp) {
            return { success: false, error: 'device ip missing' };
        }
        const port = appState?.defaults?.tcpPort || 23456;
        await invoke('connect_to_peer', { peerIp, port });
        appState.connected = true;
        appState.connectionType = device?.connectionType || 'Network';
        appState.currentDevice = deviceId;
        return { success: true };
    } catch (error) {
        return { success: false, error: String(error) };
    }
}

/**
 * 手动 IP 历史记录快速重连
 * @param {string} ip
 * @param {number} port
 * @param {object} elements - DOM 元素引用
 */
async function reconnectManualIp(ip, port, elements) {
    elements.manualIpInput.value = ip;
    elements.manualPortInput.value = port || 23456;
    // 触发手动连接（由调用方传入 appState 和 invoke）
}

// ─── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 显示手动连接状态提示
 */
function showManualConnectStatus(elements, type, message) {
    const el = elements.manualConnectStatus;
    if (!el) return;
    el.textContent = message;
    el.className = `manual-connect-status status-${type}`;
    el.classList.remove('hidden');
}

/**
 * 隐藏手动连接状态提示
 */
function hideManualConnectStatus(elements) {
    if (elements.manualConnectStatus) {
        elements.manualConnectStatus.classList.add('hidden');
    }
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.ConnectionModule = {
    // 纯函数（可测试）
    isValidIpv4,
    isValidPort,
    // 连接操作
    handleManualConnect,
    handleDisconnect,
    handleReconnect,
    connectToDevice,
    reconnectManualIp,
    // 辅助函数
    showManualConnectStatus,
    hideManualConnectStatus,
};
