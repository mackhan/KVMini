/**
 * accessibility.js - macOS 辅助功能权限管理模块（事件驱动版）
 *
 * 新架构：
 * - Rust 侧启动时主动检测权限，通过 Tauri 事件通知前端
 * - 前端只监听事件，被动响应，不主动发起 invoke 调用
 * - 遮罩默认隐藏，收到 permission-denied 事件才显示
 * - 收到 permission-granted 事件后隐藏遮罩
 * - 轮询由 Rust 后台线程负责，授权后 Rust 发送 granted 事件
 *
 * Tauri 事件：
 * - `accessibility://permission-denied`：权限未授权，显示引导遮罩
 * - `accessibility://permission-granted`：权限已授权，隐藏遮罩
 */

// ─── 事件驱动：注册 Tauri 事件监听器 ─────────────────────────────────────────

/**
 * 初始化辅助功能权限事件监听
 * 在 DOMContentLoaded 时调用，不依赖 invoke 是否就绪
 * 同时绑定"前往授权"按钮的点击事件
 */
function initAccessibilityEventListeners() {
    const tauri = window.__TAURI__;
    if (!tauri?.event?.listen) {
        console.warn('[Accessibility] Tauri event API 不可用，跳过事件监听注册');
        return;
    }

    // 兼容 Tauri 2.x (core.invoke) 和旧版 (invoke)
    const invoke = tauri?.core?.invoke || tauri?.invoke || null;

    // 监听权限未授权事件 → 显示引导遮罩
    tauri.event.listen('accessibility://permission-denied', () => {
        const overlay = document.getElementById('accessibility-overlay');
        if (!overlay) {
            console.warn('[Accessibility] 遮罩 DOM 元素不存在');
            return;
        }
        console.log('[Accessibility] 收到 permission-denied 事件，显示引导遮罩');
        overlay.classList.remove('hidden');
    });

    // 监听权限已授权事件 → 隐藏遮罩
    tauri.event.listen('accessibility://permission-granted', () => {
        const overlay = document.getElementById('accessibility-overlay');
        if (!overlay) {
            console.warn('[Accessibility] 遮罩 DOM 元素不存在');
            return;
        }
        console.log('[Accessibility] 收到 permission-granted 事件，隐藏遮罩');
        overlay.classList.add('hidden');
    });

    // 绑定"前往授权"按钮点击事件
    const authBtn = document.getElementById('open-accessibility-btn');
    if (authBtn && invoke) {
        authBtn.addEventListener('click', async () => {
            try {
                await invoke('open_accessibility_settings');
            } catch (e) {
                console.error('[Accessibility] 无法打开系统偏好设置:', e);
            }
        });
    }

    console.log('[Accessibility] 权限事件监听器已注册');
}

// 立即执行：DOM 已就绪时直接调用，否则等待 DOMContentLoaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        initAccessibilityEventListeners();
    });
} else {
    // DOM 已就绪（jsdom 测试环境或脚本在 body 末尾加载时）
    initAccessibilityEventListeners();
}

// ─── 连接后权限检查（保留，供 main.js 调用） ──────────────────────────────────

/**
 * 检测当前是否为 Windows 平台
 * @returns {boolean}
 */
function isWindowsPlatform() {
    if (navigator.userAgent.includes('Windows')) return true;
    if (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32') return true;
    return false;
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
        if (status !== 'Granted') {
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
    } catch (e) {
        console.error('[Accessibility] 无法打开系统偏好设置:', e);
    }
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.AccessibilityModule = {
    // 平台检测
    isWindowsPlatform,
    // 事件监听初始化
    initAccessibilityEventListeners,
    // 连接后权限检查
    checkPermissionAfterConnect,
    showPermissionWarning,
    hidePermissionWarning,
    openAccessibilitySettings,
};
