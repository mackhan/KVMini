/**
 * license.js - KVMini 授权管理模块
 *
 * 架构设计：
 * - 通过 tauri-commerce SDK 的 get_trial_status 命令获取试用/购买状态
 * - 支持 Active（试用中）/ Expired（已过期）/ Purchased（已购买）三种状态
 * - SDK 初始化在 Rust 侧完成，前端只需调用命令
 */

'use strict';

// ─── 常量 ─────────────────────────────────────────────────────────────────────

const LICENSE_STATUS = {
    LICENSED: 'licensed',
    UNLICENSED: 'unlicensed',
    TRIAL: 'trial',
    EXPIRED: 'expired',
    SDK_ERROR: 'sdk_error',
};

// ─── 内部状态 ─────────────────────────────────────────────────────────────────

let _currentStatus = LICENSE_STATUS.UNLICENSED;
let _statusChangeCallbacks = [];

/**
 * 获取 Tauri invoke 函数（兼容 Tauri 2.x core.invoke 和 Tauri 1.x invoke）
 * @returns {Function|null}
 */
function _getInvoke() {
    return window.__TAURI__?.core?.invoke
        || window.__TAURI__?.invoke
        || null;
}

// ─── 核心函数 ─────────────────────────────────────────────────────────────────

/**
 * 检查授权状态（通过 tauri-commerce SDK get_trial_status 命令）
 * @returns {Promise<{licensed: boolean, status: string, message?: string}>}
 */
async function checkLicenseStatus() {
    const invoke = _getInvoke();
    try {
        if (invoke) {
            // 通过 tauri-commerce SDK 获取试用/购买状态
            const trialStatus = await invoke('get_trial_status');
            // trialStatus 是 Rust TrialStatus 枚举，序列化后格式：
            // { "Active": 5 } / "Expired" / "Purchased"
            let licensed = false;
            let status = LICENSE_STATUS.UNLICENSED;

            if (trialStatus === 'Purchased' || trialStatus?.Purchased !== undefined) {
                licensed = true;
                status = LICENSE_STATUS.LICENSED;
            } else if (trialStatus === 'Active' || trialStatus?.Active !== undefined) {
                licensed = true;
                status = LICENSE_STATUS.TRIAL;
                const daysLeft = trialStatus?.Active ?? trialStatus;
                console.log(`[License] 试用中，剩余 ${daysLeft} 天`);
            } else {
                // Expired
                licensed = false;
                status = LICENSE_STATUS.EXPIRED;
            }

            _currentStatus = status;
            _notifyStatusChange(licensed);
            return { licensed, status };
        }

        // 无 Tauri 环境：默认未授权
        _currentStatus = LICENSE_STATUS.UNLICENSED;
        _notifyStatusChange(false);
        return { licensed: false, status: LICENSE_STATUS.UNLICENSED };
    } catch (e) {
        console.warn('[License] SDK 调用失败:', e);
        _currentStatus = LICENSE_STATUS.SDK_ERROR;
        // SDK 失败时降级：允许使用（不阻塞主界面）
        return { licensed: true, status: LICENSE_STATUS.SDK_ERROR, message: e?.message || String(e) };
    }
}

/**
 * 打开购买页面（App Store 版）
 */
async function openPurchasePage() {
    const purchaseUrl = 'https://kvmini.app/purchase';
    try {
        if (window.__TAURI__ && window.__TAURI__.shell) {
            await window.__TAURI__.shell.open(purchaseUrl);
        } else {
            window.open(purchaseUrl, '_blank');
        }
    } catch (e) {
        console.warn('[License] 打开购买页面失败:', e);
    }
}

/**
 * 获取当前授权状态
 * @returns {string}
 */
function getCurrentStatus() {
    return _currentStatus;
}

/**
 * 注册授权状态变化回调
 * @param {Function} callback - (licensed: boolean) => void
 */
function onStatusChange(callback) {
    _statusChangeCallbacks.push(callback);
}

/**
 * 通知所有状态变化回调
 * @param {boolean} licensed
 */
function _notifyStatusChange(licensed) {
    _statusChangeCallbacks.forEach(cb => {
        try { cb(licensed); } catch (e) { console.warn('[License] 回调执行失败:', e); }
    });
}

// ─── UI 控制 ──────────────────────────────────────────────────────────────────

/**
 * 根据授权状态更新 UI
 * @param {boolean} licensed
 */
function updateLicenseUI(licensed) {
    const licenseGate = document.getElementById('license-gate');
    const mainContent = document.getElementById('main-content');
    const statusBadge = document.getElementById('license-status-badge');

    if (licensed) {
        if (licenseGate) licenseGate.classList.add('hidden');
        if (mainContent) mainContent.classList.remove('hidden');
        if (statusBadge) statusBadge.classList.remove('hidden');
    } else {
        if (licenseGate) licenseGate.classList.remove('hidden');
        if (mainContent) mainContent.classList.add('hidden');
        if (statusBadge) statusBadge.classList.add('hidden');
    }
}

/**
 * 显示 SDK 初始化失败提示（不阻塞主界面）
 * @param {string} message
 */
function showSdkError(message) {
    const sdkError = document.getElementById('license-sdk-error');
    const sdkErrorMsg = document.getElementById('license-sdk-error-msg');
    if (sdkError) {
        if (sdkErrorMsg) sdkErrorMsg.textContent = message || '授权服务暂时不可用';
        sdkError.classList.remove('hidden');
    }
}

/**
 * 初始化授权模块（应用启动时调用）
 */
async function initLicense() {
    // 注册状态变化回调，自动更新 UI
    onStatusChange(updateLicenseUI);

    // 绑定购买按钮
    const buyBtn = document.getElementById('license-buy-btn');
    if (buyBtn) {
        buyBtn.addEventListener('click', openPurchasePage);
    }

    // 检查授权状态（通过 SDK）
    const result = await checkLicenseStatus();

    if (result.status === LICENSE_STATUS.SDK_ERROR) {
        // SDK 失败：显示错误提示，但不阻塞主界面（降级允许使用）
        showSdkError(result.message);
        updateLicenseUI(true);
    } else {
        updateLicenseUI(result.licensed);
    }
}

// ─── 导出 ─────────────────────────────────────────────────────────────────────

window.LicenseModule = {
    LICENSE_STATUS,
    checkLicenseStatus,
    openPurchasePage,
    getCurrentStatus,
    onStatusChange,
    updateLicenseUI,
    showSdkError,
    initLicense,
};
