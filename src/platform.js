/**
 * platform.js - 平台抽象层
 *
 * 封装所有平台差异调用，前端其他模块只调用此抽象层。
 * 对不支持的平台功能实现优雅降级（静默忽略或返回默认值）。
 *
 * 两端（macOS / Windows）暴露完全相同的接口，
 * 平台特定逻辑由 Rust 后端通过条件编译处理。
 */

// ─── invoke 引用（由 main.js 在 DOMContentLoaded 后注入） ───
let _invoke = null;

/**
 * 初始化平台抽象层
 * @param {Function} invokeFunc - Tauri invoke 函数
 */
function initPlatform(invokeFunc) {
    _invoke = invokeFunc;
}

/**
 * 安全调用 Tauri 命令，对非 Tauri 环境优雅降级
 * @param {string} cmd - 命令名称
 * @param {object} args - 命令参数
 * @param {*} fallback - 非 Tauri 环境的降级返回值
 */
async function safeInvoke(cmd, args = {}, fallback = null) {
    if (!_invoke) {
        console.warn(`[Platform] Tauri 未就绪，命令 "${cmd}" 降级处理`);
        return fallback;
    }
    try {
        return await _invoke(cmd, args);
    } catch (e) {
        console.error(`[Platform] 命令 "${cmd}" 执行失败:`, e);
        throw e;
    }
}

// ─── 版本信息 API ───

/**
 * 从后端读取应用版本号（两端保持一致，不使用硬编码值）
 * @returns {Promise<string>} 版本号字符串，如 "0.2.0"
 */
async function getAppVersion() {
    return safeInvoke('get_app_version', {}, '0.2.0');
}

/**
 * 获取当前平台名称
 * @returns {Promise<string>} "Windows" | "macOS"
 */
async function getPlatform() {
    return safeInvoke('get_platform', {}, 'Unknown');
}

// ─── 设置 API ───

/**
 * 获取应用设置
 * @returns {Promise<AppSettings>}
 */
async function getSettings() {
    return safeInvoke('get_settings', {}, {
        peer_ip: '',
        port: 23456,
        switch_hotkey: 'Ctrl+Alt+S',
        edge_trigger: true,
        edge_threshold: 5,
        clipboard_sync: true,
        autostart: false,
        auto_reconnect: true,
        reconnect_max_attempts: 0,
        reconnect_interval_secs: 3,
    });
}

/**
 * 保存应用设置
 * @param {AppSettings} config
 */
async function saveSettings(config) {
    return safeInvoke('save_settings', { config }, undefined);
}

/**
 * 重置为默认设置
 * @returns {Promise<AppSettings>}
 */
async function resetSettings() {
    return safeInvoke('reset_settings', {}, undefined);
}

// ─── 连接 API ───

/**
 * 启动监听模式（等待对端连接）
 * @returns {Promise<string>}
 */
async function startListening() {
    return safeInvoke('start_listening', {}, '');
}

/**
 * 主动连接对端
 * @param {string} peerIp - 对端 IP 地址
 * @returns {Promise<string>}
 */
async function connectToPeer(peerIp) {
    return safeInvoke('connect_to_peer', { peer_ip: peerIp }, '');
}

/**
 * 断开连接
 */
async function disconnectPeer() {
    return safeInvoke('disconnect_peer', {}, undefined);
}

/**
 * 获取连接状态
 * @returns {Promise<ConnectionStatus>}
 */
async function getConnectionStatus() {
    return safeInvoke('get_connection_status', {}, {
        connected: false,
        peer_ip: null,
        peer_platform: null,
        message: '未连接',
        state: 'disconnected',
    });
}

// ─── 本机信息 API ───

/**
 * 获取本机局域网 IP 地址列表
 * @returns {Promise<string[]>}
 */
async function getLocalIp() {
    return safeInvoke('get_local_ip', {}, []);
}

// ─── 自启动 API ───

/**
 * 获取自启动状态
 * @returns {Promise<boolean>}
 */
async function getAutostartStatus() {
    return safeInvoke('get_autostart_status', {}, false);
}

/**
 * 设置自启动
 * @param {boolean} enabled
 */
async function setAutostart(enabled) {
    return safeInvoke('set_autostart', { enabled }, undefined);
}

// ─── macOS 辅助功能权限 API ───

/**
 * 检查 macOS 辅助功能权限状态
 * @returns {Promise<string>} "已授权" | "未授权" | "N/A"
 */
async function checkAccessibilityPermission() {
    return safeInvoke('check_accessibility_permission', {}, 'N/A');
}

/**
 * 请求 macOS 辅助功能权限（弹出系统对话框）
 */
async function requestAccessibilityPermission() {
    return safeInvoke('request_accessibility_permission', {}, undefined);
}

/**
 * 打开 macOS 辅助功能设置页面
 */
async function openAccessibilitySettings() {
    return safeInvoke('open_accessibility_settings', {}, undefined);
}

// ─── 系统通知 API ───

/**
 * 显示系统通知（两端统一接口）
 * @param {string} title - 通知标题
 * @param {string} body - 通知内容
 */
async function showNotification(title, body) {
    return safeInvoke('show_tray_notification', { title, body }, undefined);
}

// ─── 兼容接口（映射旧接口到新命令，避免main.js大规模改动） ───

/**
 * 通过 IP 直连（映射到 connect_to_peer）
 * @param {string} ip
 * @param {number} port
 */
async function connectByIp(ip, port) {
    try {
        await safeInvoke('connect_to_peer', { peer_ip: ip }, '');
        return { success: true, errorType: null, errorMessage: null };
    } catch (e) {
        return { success: false, errorType: 'connection_failed', errorMessage: String(e) };
    }
}

/**
 * 获取连接历史（本地存储实现，后端无此接口）
 * @returns {Promise<Array>}
 */
async function getConnectionHistory() {
    try {
        const raw = localStorage.getItem('kvmini_connection_history');
        return raw ? JSON.parse(raw) : [];
    } catch { return []; }
}

/**
 * 保存连接历史条目（本地存储实现）
 * @param {object} entry
 */
async function saveConnectionHistory(entry) {
    try {
        const history = await getConnectionHistory();
        const exists = history.findIndex(h => h.ip === entry.ip);
        if (exists >= 0) history.splice(exists, 1);
        history.unshift({ ...entry, lastConnected: Date.now() });
        if (history.length > 20) history.splice(20);
        localStorage.setItem('kvmini_connection_history', JSON.stringify(history));
    } catch {}
}

/**
 * 清除连接历史（本地存储实现）
 */
async function clearConnectionHistory() {
    localStorage.removeItem('kvmini_connection_history');
}

/**
 * 获取网络接口列表（映射到 get_local_ip）
 * @returns {Promise<Array<{name: string, ip: string}>>}
 */
async function getNetworkInterfaces() {
    const ips = await getLocalIp();
    return ips.map((ip, i) => ({ name: `网卡 ${i + 1}`, ip }));
}

/**
 * 设置监听地址（新架构中端口在配置中统一管理，此处为兼容存根）
 */
async function setListenAddress(ip, port) {
    // 新架构中端口通过 save_settings 统一管理
    return undefined;
}

/**
 * 获取监听配置（从设置中读取）
 * @returns {Promise<{ip: string, port: number}>}
 */
async function getListenConfig() {
    const settings = await getSettings();
    return { ip: '0.0.0.0', port: settings.port || 23456 };
}

// ─── 导出 ───
window.Platform = {
    init: initPlatform,
    // 版本信息
    getAppVersion,
    getPlatform,
    // 设置
    getSettings,
    saveSettings,
    resetSettings,
    // 连接
    startListening,
    connectToPeer,
    disconnectPeer,
    getConnectionStatus,
    // 本机信息
    getLocalIp,
    // 自启动
    getAutostartStatus,
    setAutostart,
    // macOS 权限
    checkAccessibilityPermission,
    requestAccessibilityPermission,
    openAccessibilitySettings,
    // 通知
    showNotification,
    // 兼容接口
    connectByIp,
    getConnectionHistory,
    saveConnectionHistory,
    clearConnectionHistory,
    getNetworkInterfaces,
    setListenAddress,
    getListenConfig,
};
