/**
 * history.js - 连接历史管理模块
 *
 * 封装连接历史的本地存储操作：
 * - 读取/保存/清空历史记录（localStorage）
 * - 去重、截断（最多 20 条）
 * - 渲染历史列表 UI
 */

const HISTORY_KEY = 'kvmini_connection_history';
const MAX_HISTORY = 20;

// ─── 本地存储操作 ─────────────────────────────────────────────────────────────

/**
 * 从 localStorage 读取历史记录
 * @returns {Array}
 */
function getHistoryFromStorage() {
    try {
        const raw = localStorage.getItem(HISTORY_KEY);
        return raw ? JSON.parse(raw) : [];
    } catch {
        return [];
    }
}

/**
 * 保存条目到历史记录（自动去重、截断）
 * @param {object} entry - 历史条目，必须包含 id 字段
 */
function saveToStorage(entry) {
    const history = getHistoryFromStorage();
    // 去重：移除相同 id 的旧条目
    const filtered = history.filter(h => h.id !== entry.id);
    // 最新条目排最前
    filtered.unshift({ ...entry, lastConnected: entry.lastConnected || Date.now() });
    // 截断到最大条数
    const trimmed = filtered.slice(0, MAX_HISTORY);
    localStorage.setItem(HISTORY_KEY, JSON.stringify(trimmed));
}

/**
 * 清空历史记录
 */
function clearStorage() {
    localStorage.removeItem(HISTORY_KEY);
}

// ─── UI 渲染 ─────────────────────────────────────────────────────────────────

/**
 * 渲染历史列表到指定容器
 * @param {Array} history - 历史记录数组
 * @param {HTMLElement} container - 列表容器元素
 * @param {Function} escapeHtml - HTML 转义函数
 */
function renderHistoryList(history, container, escapeHtml) {
    if (!container) return;
    const { t } = window.i18n;

    if (!history || history.length === 0) {
        container.innerHTML = `<div class="no-history">${t('noHistory')}</div>`;
        return;
    }

    container.innerHTML = history.map(item => {
        const ts = item.lastConnected;
        const date = ts ? new Date(typeof ts === 'number' ? ts : ts) : new Date();
        const dateStr = `${date.getMonth() + 1}/${date.getDate()} ${date.getHours()}:${String(date.getMinutes()).padStart(2, '0')}`;
        const isManual = item.entryType === 'manual';
        const icon = isManual ? '🔗' : '📡';
        const typeLabel = isManual ? t('manualIpLabel') : (item.connectionType || 'Network');
        const reconnectAttr = isManual
            ? `onclick="window._reconnectManualIp('${escapeHtml(item.ip)}', ${item.port || 23456})"`
            : `onclick="window._reconnectToDevice('${escapeHtml(item.id)}', '${escapeHtml(item.name)}')"`;
        return `
            <div class="history-item">
                <div class="history-info">
                    <div class="history-device-name">${icon} ${escapeHtml(item.name || item.ip)}</div>
                    <div class="history-meta">${escapeHtml(item.ip)} · ${escapeHtml(typeLabel)} · ${dateStr}</div>
                </div>
                <button class="reconnect-btn" ${reconnectAttr}>
                    ${t('btnReconnect')}
                </button>
            </div>
        `;
    }).join('');
}

/**
 * 加载并渲染历史记录（优先使用 Platform API，降级到 localStorage）
 * @param {HTMLElement} container - 列表容器元素
 * @param {Function} escapeHtml - HTML 转义函数
 */
async function loadConnectionHistory(container, escapeHtml) {
    let history = [];
    try {
        history = await window.Platform.getConnectionHistory();
        if (history && history.length > 0) {
            localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
        } else {
            history = getHistoryFromStorage();
        }
    } catch {
        history = getHistoryFromStorage();
    }
    renderHistoryList(history, container, escapeHtml);
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.HistoryModule = {
    // 存储操作
    getHistoryFromStorage,
    saveToStorage,
    clearStorage,
    // UI 渲染
    renderHistoryList,
    loadConnectionHistory,
};
