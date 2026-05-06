/**
 * settings-tabs.js - 设置页分页管理模块
 *
 * 功能：
 * - 分页导航切换（通用 / 显示器布局 / 网络 / 安全）
 * - 未保存状态标记（dirty）
 * - 关闭时检测未保存修改
 * - 小屏幕（< 1200px）折叠导航为下拉菜单
 */

// ─── 分页配置 ─────────────────────────────────────────────────────────────────

const SETTINGS_TABS = ['general', 'display', 'network', 'security', 'about'];

// ─── 状态 ─────────────────────────────────────────────────────────────────────

let _currentTab = 'general';
const _dirtyTabs = new Set();

// ─── 核心 API ─────────────────────────────────────────────────────────────────

/**
 * 切换到指定分页
 * @param {string} tabId - 分页 ID（general / display / network / security）
 */
function switchTab(tabId) {
    if (!SETTINGS_TABS.includes(tabId)) return;

    // 更新内容区域
    document.querySelectorAll('.settings-tab-content').forEach(el => {
        el.classList.remove('active');
    });
    const content = document.getElementById(`tab-${tabId}`);
    if (content) content.classList.add('active');

    // 更新导航按钮
    document.querySelectorAll('.settings-tab-btn').forEach(btn => {
        btn.classList.remove('active');
    });
    const btn = document.querySelector(`[data-tab="${tabId}"]`);
    if (btn) btn.classList.add('active');

    _currentTab = tabId;

    // 同步下拉菜单（小屏幕）
    const select = document.getElementById('settings-tabs-select');
    if (select) select.value = tabId;
}

/**
 * 标记指定分页为"未保存"
 * @param {string} tabId
 */
function markDirty(tabId) {
    _dirtyTabs.add(tabId);
    const btn = document.querySelector(`[data-tab="${tabId}"]`);
    if (btn) {
        btn.setAttribute('data-dirty', 'true');
        btn.classList.add('tab-dirty');
        // B1: 红点 tooltip（双语，来自 i18n）
        const tip = (window.i18n && typeof window.i18n.t === 'function')
            ? window.i18n.t('tabDirtyTooltip')
            : '此分类下有未保存的修改';
        btn.title = tip;
    }
    // B2: 任一 tab 变脏 → 顶部保存按钮加脉冲动画
    _syncSaveButtonDirtyClass();
}

/**
 * 清除指定分页的"未保存"标记
 * @param {string} tabId
 */
function clearDirty(tabId) {
    _dirtyTabs.delete(tabId);
    const btn = document.querySelector(`[data-tab="${tabId}"]`);
    if (btn) {
        btn.removeAttribute('data-dirty');
        btn.classList.remove('tab-dirty');
        btn.removeAttribute('title');
    }
    _syncSaveButtonDirtyClass();
}

/**
 * 清除所有分页的"未保存"标记
 */
function clearAllDirty() {
    SETTINGS_TABS.forEach(id => clearDirty(id));
    // 显式再同步一次，确保 save 按钮动画被移除
    _syncSaveButtonDirtyClass();
}

/**
 * 根据 _dirtyTabs 状态，给顶部保存按钮加/去 .btn-save-dirty
 */
function _syncSaveButtonDirtyClass() {
    const saveBtn = document.getElementById('settings-save-btn');
    if (!saveBtn) return;
    if (_dirtyTabs.size > 0) {
        saveBtn.classList.add('btn-save-dirty');
    } else {
        saveBtn.classList.remove('btn-save-dirty');
    }
}

/**
 * 是否有未保存的修改
 * @returns {boolean}
 */
function hasDirty() {
    return _dirtyTabs.size > 0;
}

/**
 * 获取当前激活的分页 ID
 * @returns {string}
 */
function getCurrentTab() {
    return _currentTab;
}

// ─── 初始化 ───────────────────────────────────────────────────────────────────

/**
 * 初始化分页模块：绑定导航点击事件，处理小屏幕折叠
 */
function initSettingsTabs() {
    // 绑定分页按钮点击
    document.querySelectorAll('.settings-tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.getAttribute('data-tab');
            if (tabId) switchTab(tabId);
        });
    });

    // 绑定小屏幕下拉菜单
    const select = document.getElementById('settings-tabs-select');
    if (select) {
        select.addEventListener('change', () => {
            switchTab(select.value);
        });
    }

    // 监听设置项变化，自动标记 dirty
    _bindDirtyListeners();

    // 初始激活第一个分页
    switchTab('general');
}

/**
 * 监听设置项变化，自动标记对应分页为 dirty
 */
function _bindDirtyListeners() {
    const tabContentMap = {
        'tab-general': 'general',
        'tab-display': 'display',
        'tab-network': 'network',
        'tab-security': 'security',
        'tab-about': 'about',
    };

    Object.entries(tabContentMap).forEach(([contentId, tabId]) => {
        const content = document.getElementById(contentId);
        if (!content) return;

        // 监听 input / change 事件
        content.addEventListener('input', () => markDirty(tabId));
        content.addEventListener('change', () => markDirty(tabId));
    });
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.SettingsTabsModule = {
    SETTINGS_TABS,
    switchTab,
    markDirty,
    clearDirty,
    clearAllDirty,
    hasDirty,
    getCurrentTab,
    initSettingsTabs,
};
