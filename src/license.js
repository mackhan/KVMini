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
 * 购买完整版（通过 tauri-commerce SDK IAP 内购）
 */
async function purchaseProduct() {
    const invoke = _getInvoke();
    if (!invoke) {
        console.warn('[License] 无 Tauri 环境，无法发起购买');
        return;
    }
    // 获取按钮并添加 loading 状态
    const btn = document.getElementById('license-purchase-btn');
    if (btn) { btn.disabled = true; btn.textContent = window.i18n ? window.i18n.t('purchaseProcessing') : '购买中...'; }
    try {
        console.log('[License] 发起 IAP 购买: com.kvmini.app.pro');
        await invoke('purchase', { productId: 'com.kvmini.app.pro' });
        console.log('[License] IAP 购买成功');
        // 购买成功后自动刷新授权状态
        const result = await checkLicenseStatus();
        updateLicenseUI(result.licensed);
        const msg = window.i18n ? window.i18n.t('purchaseSuccess') : '购买成功！';
        if (typeof showSuccess === 'function') {
            showSuccess(msg);
        } else {
            console.log('[License] ' + msg);
        }
    } catch (e) {
        console.error('[License] IAP 购买失败:', e);
        const detail = typeof e === 'string' ? e : (e?.message || '');
        const baseMsg = window.i18n ? window.i18n.t('purchaseFailed') : '购买失败，请重试';
        const msg = detail ? `${baseMsg}: ${detail}` : baseMsg;
        if (typeof showError === 'function') {
            showError(msg);
        } else {
            console.error('[License] ' + msg);
        }
    } finally {
        if (btn) { btn.disabled = false; btn.textContent = window.i18n ? window.i18n.t('purchaseBtn') : '购买完整版'; }
    }
}

/**
 * 恢复购买（通过 tauri-commerce SDK restore_purchase 命令）
 */
async function restorePurchase() {
    const invoke = _getInvoke();
    if (!invoke) {
        console.warn('[License] 无 Tauri 环境，无法恢复购买');
        return;
    }
    // 获取按钮并添加 loading 状态
    const btn = document.getElementById('license-restore-btn');
    if (btn) { btn.disabled = true; btn.textContent = window.i18n ? window.i18n.t('restoreProcessing') : '恢复中...'; }
    try {
        console.log('[License] 发起恢复购买');
        await invoke('restore_purchase');
        console.log('[License] 恢复购买成功');
        // 恢复成功后自动刷新授权状态
        const result = await checkLicenseStatus();
        updateLicenseUI(result.licensed);
        const msg = window.i18n ? window.i18n.t('restoreSuccess') : '恢复购买成功！';
        if (typeof showSuccess === 'function') {
            showSuccess(msg);
        } else {
            console.log('[License] ' + msg);
        }
    } catch (e) {
        console.error('[License] 恢复购买失败:', e);
        const detail = typeof e === 'string' ? e : (e?.message || '');
        const baseMsg = window.i18n ? window.i18n.t('restoreFailed') : '恢复购买失败';
        const msg = detail ? `${baseMsg}: ${detail}` : baseMsg;
        if (typeof showError === 'function') {
            showError(msg);
        } else {
            console.error('[License] ' + msg);
        }
    } finally {
        if (btn) { btn.disabled = false; btn.textContent = window.i18n ? window.i18n.t('restoreBtn') : '恢复购买'; }
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
    const mainContent = document.getElementById('view-main');
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

    // 绑定 IAP 购买按钮
    const purchaseBtn = document.getElementById('license-purchase-btn');
    if (purchaseBtn) {
        purchaseBtn.addEventListener('click', purchaseProduct);
        console.log('[License] 已绑定购买按钮: license-purchase-btn');
    }

    // 绑定恢复购买按钮
    const restoreBtn = document.getElementById('license-restore-btn');
    if (restoreBtn) {
        restoreBtn.addEventListener('click', restorePurchase);
        console.log('[License] 已绑定恢复购买按钮: license-restore-btn');
    }

    // 关于 Tab 中的购买/恢复按钮（settings-purchase-btn / settings-restore-btn）
    const settingsPurchaseBtn = document.getElementById('settings-purchase-btn');
    if (settingsPurchaseBtn) {
        settingsPurchaseBtn.addEventListener('click', purchaseProduct);
    }
    const settingsRestoreBtn = document.getElementById('settings-restore-btn');
    if (settingsRestoreBtn) {
        settingsRestoreBtn.addEventListener('click', restorePurchase);
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
    purchaseProduct,
    restorePurchase,
    getCurrentStatus,
    onStatusChange,
    updateLicenseUI,
    showSdkError,
    initLicense,
};
