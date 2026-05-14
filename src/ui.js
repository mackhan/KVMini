/**
 * ui.js - UI 状态管理模块
 *
 * 封装所有 UI 状态更新逻辑：
 * - HTML 转义（防 XSS）
 * - 连接状态显示
 * - 控制状态显示
 * - Toast 提示（错误/成功/重连）
 * - 按钮状态管理
 */

// ─── 纯函数：工具 ────────────────────────────────────────────────────────────

/**
 * HTML 转义，防止 XSS
 * @param {*} str
 * @returns {string}
 */
function escapeHtml(str) {
    if (str == null) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

// ─── 连接状态 UI ─────────────────────────────────────────────────────────────

/**
 * 设置连接状态显示
 * @param {string} status - 'connected' | 'connecting' | 'disconnected'
 * @param {HTMLElement} el - 状态显示元素
 */
function setConnectionStatus(status, el) {
    if (!el) return;
    const { t } = window.i18n;
    const statusMap = {
        connected: t('statusConnected'),
        connecting: t('statusConnecting'),
        disconnected: t('statusDisconnected'),
    };
    const classMap = {
        connected: 'status-connected',
        connecting: 'status-connecting',
        disconnected: 'status-disconnected',
    };
    el.textContent = statusMap[status] || t('statusDisconnected');
    el.className = classMap[status] || 'status-disconnected';
}

// ─── 控制状态 UI ─────────────────────────────────────────────────────────────

/**
 * 更新控制状态 UI
 * @param {string} state - 'LocalControlling' | 'RemoteControlling' | 'Idle'
 * @param {HTMLElement} el - 控制状态元素
 */
function updateControlStateUI(state, el) {
    if (!el) return;
    const { t } = window.i18n;
    if (state === 'LocalControlling') {
        el.textContent = t('controlStateLocal');
        el.className = 'control-state-local';
    } else if (state === 'RemoteControlling') {
        el.textContent = t('controlStateRemote');
        el.className = 'control-state-remote';
    } else {
        el.textContent = t('controlStateIdle');
        el.className = 'control-state-idle';
    }
}

// ─── Toast 提示 ──────────────────────────────────────────────────────────────

let _errorTimer = null;
let _successTimer = null;
let _reconnectToastTimer = null;

/**
 * 显示错误提示（5 秒后自动消失）
 * @param {string} message
 * @param {object} elements - DOM 元素引用
 */
function showError(message, elements) {
    console.error('Error:', message);
    if (!elements?.errorMessage || !elements?.errorToast) return;
    elements.errorMessage.textContent = message;
    elements.errorToast.classList.remove('hidden');
    if (_errorTimer) clearTimeout(_errorTimer);
    _errorTimer = setTimeout(() => hideError(elements), 5000);
}

/**
 * 隐藏错误提示
 * @param {object} elements - DOM 元素引用
 */
function hideError(elements) {
    if (elements?.errorToast) elements.errorToast.classList.add('hidden');
}

/**
 * 显示成功提示（3 秒后自动消失）
 * @param {string} message
 * @param {object} elements - DOM 元素引用
 */
function showSuccess(message, elements) {
    if (!elements?.successMessage || !elements?.successToast) return;
    elements.successMessage.textContent = message;
    elements.successToast.classList.remove('hidden');
    if (_successTimer) clearTimeout(_successTimer);
    _successTimer = setTimeout(() => {
        if (elements.successToast) elements.successToast.classList.add('hidden');
    }, 3000);
}

/**
 * 显示重连状态（状态面板中）
 * @param {string} message
 * @param {number|null} attempt
 * @param {object} elements - DOM 元素引用
 */
function showReconnectStatus(message, attempt, elements) {
    if (elements?.reconnectStatusRow) {
        elements.reconnectStatusRow.style.display = '';
    }
    if (elements?.reconnectStatusText) {
        elements.reconnectStatusText.textContent = message || '正在重连...';
    }
    if (attempt && attempt === 11) {
        showReconnectToast('重连失败，仍在后台尝试...', elements);
    }
}

/**
 * 隐藏重连状态
 * @param {object} elements - DOM 元素引用
 */
function hideReconnectStatus(elements) {
    if (elements?.reconnectStatusRow) {
        elements.reconnectStatusRow.style.display = 'none';
    }
}

/**
 * 显示重连 Toast 提示（4 秒后自动消失）
 * @param {string} message
 * @param {object} elements - DOM 元素引用
 */
function showReconnectToast(message, elements) {
    if (!elements?.reconnectToast) return;
    if (elements.reconnectToastMessage) {
        elements.reconnectToastMessage.textContent = message;
    }
    elements.reconnectToast.classList.remove('hidden');
    if (_reconnectToastTimer) clearTimeout(_reconnectToastTimer);
    _reconnectToastTimer = setTimeout(() => {
        if (elements.reconnectToast) elements.reconnectToast.classList.add('hidden');
    }, 4000);
}

// ─── 按钮状态 ────────────────────────────────────────────────────────────────

/**
 * v0.1.59 BUG-042: 单按钮三态（dead code，保留签名兼容性）
 * 主路径在 main.js updateConnectButton(phase)
 * @param {boolean} connected
 * @param {object} elements - DOM 元素引用
 */
function updateConnectionButtons(connected, elements) {
    const btn = elements?.connectBtn || elements?.disconnectBtn;
    if (btn) {
        btn.setAttribute('data-state', connected ? 'connected' : 'disconnected');
    }
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.UIModule = {
    // 纯函数
    escapeHtml,
    // 连接状态
    setConnectionStatus,
    // 控制状态
    updateControlStateUI,
    // Toast 提示
    showError,
    hideError,
    showSuccess,
    showReconnectStatus,
    hideReconnectStatus,
    showReconnectToast,
    // 按钮状态
    updateConnectionButtons,
};
