/**
 * accessibility.js - macOS 辅助功能权限管理模块
 *
 * 封装辅助功能权限检测和引导逻辑：
 * - 平台检测（纯函数，可测试）
 * - 权限状态检测
 * - 轮询检测（授权后自动隐藏遮罩）
 * - 连接后权限检查
 */

// ─── 纯函数：平台检测 ─────────────────────────────────────────────────────────

/**
 * 检测当前是否为 Windows 平台
 * @returns {boolean}
 */
function isWindowsPlatform() {
    if (navigator.userAgent.includes('Windows')) return true;
    if (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32') return true;
    return false;
}

// ─── 权限检测 ─────────────────────────────────────────────────────────────────

/** 权限轮询定时器 ID */
let _accessibilityPollTimer = null;

/**
 * 检测辅助功能权限，未授权时显示全屏遮罩引导
 * Windows 平台不需要此权限，直接跳过
 * @param {Function} invoke - Tauri invoke 函数
 */
async function checkAccessibilityPermission(invoke) {
    const overlay = document.getElementById('accessibility-overlay');
    if (!overlay || !invoke) return;

    // Windows 平台不需要辅助功能权限
    if (isWindowsPlatform()) {
        overlay.classList.add('hidden');
        return;
    }

    try {
        const status = await invoke('check_accessibility_permission');
        if (status === 'Denied' || status === '未授权') {
            overlay.classList.remove('hidden');
            startAccessibilityPolling(invoke);
        } else {
            overlay.classList.add('hidden');
            stopAccessibilityPolling();
        }
    } catch {
        overlay.classList.add('hidden');
    }
}

/**
 * 启动权限轮询（每 2 秒检测一次）
 * @param {Function} invoke - Tauri invoke 函数
 */
function startAccessibilityPolling(invoke) {
    if (_accessibilityPollTimer) return;
    _accessibilityPollTimer = setInterval(async () => {
        try {
            const status = await invoke('check_accessibility_permission');
            if (status !== 'Denied' && status !== '未授权') {
                const overlay = document.getElementById('accessibility-overlay');
                if (overlay) overlay.classList.add('hidden');
                stopAccessibilityPolling();
            }
        } catch {
            // 忽略轮询错误
        }
    }, 2000);
}

/**
 * 停止权限轮询
 */
function stopAccessibilityPolling() {
    if (_accessibilityPollTimer) {
        clearInterval(_accessibilityPollTimer);
        _accessibilityPollTimer = null;
    }
}

/**
 * 连接成功后检查辅助功能权限
 * - macOS 且权限 Denied：显示提示，禁用开始共享按钮
 * - macOS 且权限 Granted / Windows：隐藏提示，启用开始共享按钮
 * @param {object} elements - DOM 元素引用
 * @param {Function} invoke - Tauri invoke 函数
 */
async function checkPermissionAfterConnect(elements, invoke) {
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.classList.remove('hidden');
    }

    if (isWindowsPlatform() || !invoke) {
        hidePermissionWarning(elements);
        return;
    }

    try {
        const status = await invoke('check_accessibility_permission');
        if (status === 'Denied' || status === '未授权') {
            showPermissionWarning(elements);
        } else {
            hidePermissionWarning(elements);
        }
    } catch {
        hidePermissionWarning(elements);
    }
}

/**
 * 显示连接后权限不足提示，禁用开始共享按钮
 * @param {object} elements - DOM 元素引用
 */
function showPermissionWarning(elements) {
    if (elements?.permissionWarning) {
        elements.permissionWarning.classList.remove('hidden');
    }
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.disabled = true;
    }
}

/**
 * 隐藏连接后权限不足提示，恢复开始共享按钮
 * @param {object} elements - DOM 元素引用
 */
function hidePermissionWarning(elements) {
    if (elements?.permissionWarning) {
        elements.permissionWarning.classList.add('hidden');
    }
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.disabled = false;
    }
}

/**
 * 打开辅助功能设置页面
 * @param {Function} invoke - Tauri invoke 函数
 */
async function openAccessibilitySettings(invoke) {
    if (!invoke) return;
    try {
        await invoke('open_accessibility_settings');
        setTimeout(() => checkAccessibilityPermission(invoke), 3000);
    } catch (e) {
        console.error('无法打开系统偏好设置:', e);
    }
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.AccessibilityModule = {
    // 纯函数
    isWindowsPlatform,
    // 权限检测
    checkAccessibilityPermission,
    startAccessibilityPolling,
    stopAccessibilityPolling,
    checkPermissionAfterConnect,
    showPermissionWarning,
    hidePermissionWarning,
    openAccessibilitySettings,
};
