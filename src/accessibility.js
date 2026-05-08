/**
 * accessibility.js - macOS 权限管理模块（事件驱动 · 双权限版）
 *
 * v0.1.36 起同时管理两种权限：
 *   1. Accessibility（辅助功能）     —— 注入鼠标/键盘事件
 *   2. Input Monitoring（输入监控）  —— 捕获 CGEventTap 原始事件
 *
 * 任一权限未授予 → 显示遮罩，文案按类型切换。
 * 两者都授予 → 隐藏遮罩。
 *
 * Tauri 事件：
 * - `accessibility://permission-denied`      / `accessibility://permission-granted`
 * - `input-monitoring://permission-denied`   / `input-monitoring://permission-granted`
 *
 * Tauri 命令：
 * - `check_accessibility_permission`       —— 返回 'Granted' | 'Denied' | 'N/A'
 * - `check_input_monitoring_permission`    —— 返回 'Granted' | 'Denied' | 'N/A'
 * - `open_accessibility_settings`          —— 打开系统偏好设置 · 辅助功能页
 * - `open_input_monitoring_settings`       —— 打开系统偏好设置 · 输入监控页
 */

// ─── 当前遮罩状态 ─────────────────────────────────────────────────────────────
// 用于决定"前往授权"按钮该打开哪个系统设置页面
let _currentOverlayKind = 'accessibility'; // 'accessibility' | 'input-monitoring'

/**
 * 根据权限种类刷新遮罩文案（标题 + 描述）
 * @param {'accessibility' | 'input-monitoring'} kind
 */
function applyOverlayText(kind) {
    _currentOverlayKind = kind;
    const titleEl = document.querySelector('.accessibility-overlay-title');
    const descEl = document.querySelector('.accessibility-overlay-desc');
    if (!titleEl || !descEl) return;
    const t = (window.i18n && window.i18n.t) ? window.i18n.t : (k) => k;
    if (kind === 'input-monitoring') {
        titleEl.textContent = t('overlayInputMonitoringTitle');
        descEl.textContent = t('overlayInputMonitoringDesc');
        titleEl.setAttribute('data-i18n', 'overlayInputMonitoringTitle');
        descEl.setAttribute('data-i18n', 'overlayInputMonitoringDesc');
    } else {
        titleEl.textContent = t('overlayTitle');
        descEl.textContent = t('overlayDesc');
        titleEl.setAttribute('data-i18n', 'overlayTitle');
        descEl.setAttribute('data-i18n', 'overlayDesc');
    }
}

function showOverlay(kind) {
    const overlay = document.getElementById('accessibility-overlay');
    if (!overlay) return;
    applyOverlayText(kind);
    overlay.classList.remove('hidden');
}

function hideOverlay() {
    const overlay = document.getElementById('accessibility-overlay');
    if (!overlay) return;
    overlay.classList.add('hidden');
}

// ─── 事件驱动：注册 Tauri 事件监听器 ─────────────────────────────────────────

/**
 * 初始化权限事件监听（辅助功能 + 输入监控）
 * 在 DOMContentLoaded 时调用，不依赖 invoke 是否就绪
 */
function initAccessibilityEventListeners() {
    const tauri = window.__TAURI__;
    if (!tauri?.event?.listen) {
        console.warn('[Permission] Tauri event API 不可用，跳过事件监听注册');
        return;
    }

    // 兼容 Tauri 2.x (core.invoke) 和旧版 (invoke)
    const invoke = tauri?.core?.invoke || tauri?.invoke || null;

    // ─── 辅助功能权限事件 ───────────────────────────────────────────────────
    tauri.event.listen('accessibility://permission-denied', () => {
        if (navigator.userAgent.includes('Windows')) return;
        console.log('[Permission] accessibility://permission-denied → 显示辅助功能引导');
        showOverlay('accessibility');
    });

    tauri.event.listen('accessibility://permission-granted', () => {
        console.log('[Permission] accessibility://permission-granted');
        // 只有当前显示的是辅助功能遮罩时才隐藏；若是输入监控遮罩则保留
        if (_currentOverlayKind === 'accessibility') {
            hideOverlay();
        }
        // 同时隐藏连接后的权限横幅（如果当前显示的是辅助功能警告）
        const permWarn = document.getElementById('permission-warning');
        if (permWarn && !permWarn.classList.contains('hidden')) {
            const btn = permWarn.querySelector('.permission-warning-btn');
            const kind = btn ? btn.getAttribute('data-kind') : 'accessibility';
            if (kind === 'accessibility' || !kind) {
                permWarn.classList.add('hidden');
                // 恢复开始共享按钮
                const startBtn = document.getElementById('start-sharing-btn');
                if (startBtn) startBtn.disabled = false;
            }
        }
    });

    // ─── 输入监控权限事件（BUG-027 新增） ──────────────────────────────────
    tauri.event.listen('input-monitoring://permission-denied', () => {
        if (navigator.userAgent.includes('Windows')) return;
        console.log('[Permission] input-monitoring://permission-denied → 显示输入监控引导');
        // 如果辅助功能遮罩已经显示，不要覆盖——先处理完辅助功能再来
        const overlay = document.getElementById('accessibility-overlay');
        const isShown = overlay && !overlay.classList.contains('hidden');
        if (isShown && _currentOverlayKind === 'accessibility') {
            console.log('[Permission] 已有辅助功能遮罩显示，输入监控提示延后');
            return;
        }
        showOverlay('input-monitoring');
    });

    tauri.event.listen('input-monitoring://permission-granted', () => {
        console.log('[Permission] input-monitoring://permission-granted');
        if (_currentOverlayKind === 'input-monitoring') {
            hideOverlay();
        }
        // 同时隐藏连接后的权限横幅（如果当前显示的是输入监控警告）
        const permWarn = document.getElementById('permission-warning');
        if (permWarn && !permWarn.classList.contains('hidden')) {
            const btn = permWarn.querySelector('.permission-warning-btn');
            const kind = btn ? btn.getAttribute('data-kind') : '';
            if (kind === 'input-monitoring') {
                permWarn.classList.add('hidden');
                const startBtn = document.getElementById('start-sharing-btn');
                if (startBtn) startBtn.disabled = false;
            }
        }
    });

    // ─── "前往授权"按钮：按当前遮罩种类调用不同后端命令 ────────────────────
    const authBtn = document.getElementById('open-accessibility-btn');
    if (authBtn && invoke) {
        authBtn.addEventListener('click', async () => {
            try {
                if (_currentOverlayKind === 'input-monitoring') {
                    await invoke('open_input_monitoring_settings');
                } else {
                    await invoke('open_accessibility_settings');
                }
            } catch (e) {
                console.error('[Permission] 无法打开系统偏好设置:', e);
            }
        });
    }

    // ─── 主动查询初始权限状态（解决 Rust 事件发送早于前端注册的时序问题） ──
    if (invoke) {
        checkInitialPermissions(invoke);
    }

    console.log('[Permission] 权限事件监听器已注册（双权限版）');
}

/**
 * 启动时主动查询辅助功能 + 输入监控权限状态
 * 优先级：辅助功能未授权 > 输入监控未授权 > 全部通过
 * @param {Function} invoke
 */
async function checkInitialPermissions(invoke) {
    try {
        const ax = await invoke('check_accessibility_permission');
        // 'N/A' 表示非 macOS 平台，直接跳过
        if (ax === 'N/A') {
            hideOverlay();
            return;
        }
        // 只有明确拿到 'Denied' 才显示遮罩；undefined/null/其它值视为"未知"，
        // 保持现状等待 Rust 侧的事件通知（避免误触发）
        if (ax === 'Denied') {
            showOverlay('accessibility');
            return;
        }
        if (ax !== 'Granted') {
            // 未知结果（例如后端异常或 mock 未返回），不做遮罩切换
            return;
        }
        // 辅助功能 OK，再查输入监控
        try {
            const im = await invoke('check_input_monitoring_permission');
            if (im === 'Denied') {
                showOverlay('input-monitoring');
            } else if (im === 'Granted' || im === 'N/A') {
                hideOverlay();
            }
            // 其它值：保持现状，等待事件
        } catch (e) {
            // 后端如果还没实现 check_input_monitoring_permission，直接忽略
            console.warn('[Permission] check_input_monitoring_permission 不可用:', e);
        }
    } catch (e) {
        console.warn('[Permission] 查询初始权限状态失败:', e);
    }
}

// 立即执行：DOM 已就绪时直接调用，否则等待 DOMContentLoaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        initAccessibilityEventListeners();
    });
} else {
    initAccessibilityEventListeners();
}

// ─── 连接后权限检查（保留，供 main.js 调用） ──────────────────────────────────

function isWindowsPlatform() {
    if (navigator.userAgent.includes('Windows')) return true;
    if (window.__TAURI__ && window.__TAURI__?.os?.platform === 'win32') return true;
    return false;
}

/**
 * 连接成功后检查权限（辅助功能 + 输入监控都要有）
 */
async function checkPermissionAfterConnect(elements, invoke) {
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.classList.remove('hidden');
    }

    if (isWindowsPlatform() || !invoke) {
        hidePermissionWarning(elements);
        return;
    }

    console.log('[Permission] checkPermissionAfterConnect: 开始权限检查');
    try {
        let ax = await invoke('check_accessibility_permission');
        console.log('[Permission] 辅助功能权限首次检查结果:', ax);
        if (ax !== 'Granted') {
            // 延迟 1.5 秒重试一次（macOS TCC 缓存刷新需要时间）
            console.log('[Permission] 辅助功能未授权，1.5秒后重试...');
            await new Promise(r => setTimeout(r, 1500));
            ax = await invoke('check_accessibility_permission');
            console.log('[Permission] 辅助功能权限重试结果:', ax);
        }
        if (ax !== 'Granted') {
            console.warn('[Permission] 辅助功能权限最终 Denied，显示横幅');
            showPermissionWarning(elements, 'accessibility');
            // 请求后端重新启动权限轮询（防止旧轮询已停止）
            invoke('request_accessibility_permission').catch(() => {});
            return;
        }
        console.log('[Permission] 辅助功能已授权 ✓，检查输入监控...');
        try {
            let im = await invoke('check_input_monitoring_permission');
            console.log('[Permission] 输入监控权限首次检查结果:', im);
            if (im !== 'Granted' && im !== 'N/A') {
                // 延迟 1.5 秒重试一次
                console.log('[Permission] 输入监控未授权，1.5秒后重试...');
                await new Promise(r => setTimeout(r, 1500));
                im = await invoke('check_input_monitoring_permission');
                console.log('[Permission] 输入监控权限重试结果:', im);
            }
            if (im !== 'Granted' && im !== 'N/A') {
                console.warn('[Permission] 输入监控权限最终 Denied，显示横幅');
                showPermissionWarning(elements, 'input-monitoring');
                return;
            }
            console.log('[Permission] 输入监控已授权 ✓');
        } catch (e) {
            console.warn('[Permission] check_input_monitoring_permission 不可用:', e);
            // 后端未实现时降级为只校验辅助功能
        }
        console.log('[Permission] 所有权限检查通过，隐藏横幅');
        hidePermissionWarning(elements);
    } catch (e) {
        console.warn('[Permission] 权限检查异常:', e);
        hidePermissionWarning(elements);
    }
}

function showPermissionWarning(elements, kind = 'accessibility') {
    const t = (window.i18n && window.i18n.t) ? window.i18n.t : (k) => k;
    if (elements?.permissionWarning) {
        elements.permissionWarning.classList.remove('hidden');
        const textEl = elements.permissionWarning.querySelector('.permission-warning-text');
        const btnEl = elements.permissionWarning.querySelector('.permission-warning-btn') ||
                      document.getElementById('permission-warning-btn');
        if (textEl) {
            const textKey = kind === 'input-monitoring'
                ? 'permissionWarningInputMonitoringText'
                : 'permissionWarningText';
            textEl.textContent = t(textKey);
            textEl.setAttribute('data-i18n', textKey);
        }
        if (btnEl) {
            const btnKey = kind === 'input-monitoring'
                ? 'permissionWarningBtnInputMonitoring'
                : 'permissionWarningBtnAccessibility';
            btnEl.textContent = t(btnKey);
            btnEl.setAttribute('data-i18n', btnKey);
            btnEl.setAttribute('data-kind', kind);
            btnEl.classList.remove('hidden');
            // 幂等绑定：用 onclick 避免多次累积
            const tauri = window.__TAURI__;
            const invoke = tauri?.core?.invoke || tauri?.invoke || null;
            btnEl.onclick = async () => {
                if (!invoke) return;
                const cmd = btnEl.getAttribute('data-kind') === 'input-monitoring'
                    ? 'open_input_monitoring_settings'
                    : 'open_accessibility_settings';
                try { await invoke(cmd); } catch (e) {
                    console.error('[Permission] 打开系统设置失败:', e);
                }
            };
        }
    }
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.disabled = true;
    }
}

function hidePermissionWarning(elements) {
    if (elements?.permissionWarning) {
        elements.permissionWarning.classList.add('hidden');
        const btnEl = elements.permissionWarning.querySelector('.permission-warning-btn');
        if (btnEl) btnEl.classList.add('hidden');
    }
    if (elements?.startSharingBtn) {
        elements.startSharingBtn.disabled = false;
    }
}

async function openAccessibilitySettings(invoke) {
    if (!invoke) return;
    try {
        await invoke('open_accessibility_settings');
    } catch (e) {
        console.error('[Permission] 无法打开系统偏好设置:', e);
    }
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.AccessibilityModule = {
    isWindowsPlatform,
    initAccessibilityEventListeners,
    checkPermissionAfterConnect,
    showPermissionWarning,
    hidePermissionWarning,
    openAccessibilitySettings,
    // 供测试/main.js 使用
    applyOverlayText,
    showOverlay,
    hideOverlay,
};
