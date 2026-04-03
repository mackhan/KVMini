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

// ─── 设置 API ───

/**
 * 获取应用设置
 * @returns {Promise<AppSettings>}
 */
async function getSettings() {
    return safeInvoke('get_settings', {}, {
        hotkey: '',
        preferConnectionType: 'auto',
        autoReconnect: true,
        autoStart: false,
        startMinimized: false,
    });
}

/**
 * 保存应用设置
 * @param {AppSettings} settings
 */
async function saveSettings(settings) {
    return safeInvoke('save_settings', { settings }, undefined);
}

// ─── 连接历史 API ───

/**
 * 获取连接历史记录
 * @returns {Promise<HistoryEntry[]>}
 */
async function getConnectionHistory() {
    return safeInvoke('get_connection_history', {}, []);
}

/**
 * 保存连接历史条目
 * @param {HistoryEntry} entry
 */
async function saveConnectionHistory(entry) {
    return safeInvoke('save_connection_history', { entry }, undefined);
}

/**
 * 清除连接历史记录
 */
async function clearConnectionHistory() {
    return safeInvoke('clear_connection_history', {}, undefined);
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

// ─── 全局快捷键 API ───

/**
 * 注册全局快捷键
 * @param {string} hotkey - 快捷键字符串，如 "Ctrl+Alt+K"
 */
async function registerGlobalHotkey(hotkey) {
    return safeInvoke('register_global_hotkey', { hotkey }, undefined);
}

// ─── 延迟检测 API ───

/**
 * 获取当前连接延迟（毫秒）
 * @returns {Promise<number|null>}
 */
async function getLatency() {
    return safeInvoke('get_latency', {}, null);
}

// ─── 遗留进程清理 API ───

/**
 * 检测并终止 kvnshare 遗留进程
 * @returns {Promise<boolean>} - true 表示找到并终止了遗留进程
 */
async function killLegacyProcess() {
    return safeInvoke('kill_legacy_process', {}, false);
}

// ─── 本机信息 API ───

/**
 * 获取本机局域网 IP 地址列表
 * @returns {Promise<string[]>}
 */
async function getLocalIp() {
    return safeInvoke('get_local_ip', {}, []);
}

// ─── 设备发现与连接 API ───

/**
 * 发现所有可用设备
 * @returns {Promise<DeviceInfo[]>}
 */
async function discoverDevices() {
    return safeInvoke('discover_devices', {}, []);
}

/**
 * 连接到指定设备
 * @param {string} deviceId
 * @returns {Promise<ConnectionResult>}
 */
async function connectToDevice(deviceId) {
    return safeInvoke('connect_to_device', { deviceId }, { success: false, error: 'Tauri 未就绪' });
}

/**
 * 断开连接
 */
async function disconnectDevices() {
    return safeInvoke('disconnect_devices', {}, undefined);
}

/**
 * 获取连接状态
 * @returns {Promise<string|null>}
 */
async function getConnectionStatus() {
    return safeInvoke('get_connection_status', {}, null);
}

// ─── 导出 ───
window.Platform = {
    init: initPlatform,
    // 设置
    getSettings,
    saveSettings,
    // 历史记录
    getConnectionHistory,
    saveConnectionHistory,
    clearConnectionHistory,
    // 通知
    showNotification,
    // 快捷键
    registerGlobalHotkey,
    // 延迟
    getLatency,
    // 遗留进程
    killLegacyProcess,
    // 本机信息
    getLocalIp,
    // 设备
    discoverDevices,
    connectToDevice,
    disconnectDevices,
    getConnectionStatus,
};
