/**
 * settings.js - 设置管理模块
 *
 * 封装应用设置的存储和 UI 操作：
 * - 默认设置常量
 * - localStorage 读写
 * - 设置面板 UI 操作
 */

// ─── 默认设置 ─────────────────────────────────────────────────────────────────

const DEFAULT_SETTINGS = {
    hotkey: 'Ctrl+Alt+S',
    preferConnectionType: 'auto',
    autoReconnect: true,
    autoStart: false,
    startMinimized: false,
    clipboardSync: true,
    keyMappingUnified: true,
    enableTls: true,
    language: 'zh',
    theme: 'auto',
};

// ─── 本地存储操作 ─────────────────────────────────────────────────────────────

/**
 * 从 localStorage 加载设置（合并默认值）
 * @returns {object}
 */
function loadSettingsFromStorage() {
    try {
        const saved = JSON.parse(localStorage.getItem('kvm_settings') || 'null');
        if (saved) {
            return { ...DEFAULT_SETTINGS, ...saved };
        }
    } catch {
        // 解析失败，使用默认值
    }
    return { ...DEFAULT_SETTINGS };
}

/**
 * 保存设置到 localStorage
 * @param {object} settings
 */
function saveSettingsToStorage(settings) {
    localStorage.setItem('kvm_settings', JSON.stringify(settings));
}

// ─── UI 操作 ─────────────────────────────────────────────────────────────────

/**
 * 将设置填充到 UI 元素
 * @param {object} settings - 设置对象
 * @param {object} elements - DOM 元素引用
 */
function loadSettingsToUI(settings, elements) {
    if (!elements) return;
    const hotkey = settings.hotkey || DEFAULT_SETTINGS.hotkey;
    if (elements.hotkeyInput) {
        elements.hotkeyInput.value = hotkey;
        elements.hotkeyInput.dataset.hotkey = settings.hotkey || '';
    }
    if (elements.preferConnectionType) elements.preferConnectionType.value = settings.preferConnectionType;
    if (elements.autoReconnect) elements.autoReconnect.checked = settings.autoReconnect;
    if (elements.autoStart) elements.autoStart.checked = settings.autoStart;
    if (elements.startMinimized) elements.startMinimized.checked = settings.startMinimized;
    if (elements.clipboardSync) elements.clipboardSync.checked = settings.clipboardSync !== false;
    if (elements.keyMappingUnified) elements.keyMappingUnified.checked = settings.keyMappingUnified !== false;
    if (elements.enableTls) elements.enableTls.checked = settings.enableTls !== false;
    if (elements.languageSelect) elements.languageSelect.value = settings.language || 'zh';
    if (elements.themeSelect) elements.themeSelect.value = settings.theme || 'auto';
}

/**
 * 从 UI 元素收集设置
 * @param {object} elements - DOM 元素引用
 * @param {object} currentSettings - 当前设置（用于保留未在 UI 中的字段）
 * @returns {object}
 */
function collectSettingsFromUI(elements, currentSettings) {
    return {
        ...currentSettings,
        hotkey: elements.hotkeyInput?.dataset.hotkey || '',
        preferConnectionType: elements.preferConnectionType?.value || 'auto',
        autoReconnect: elements.autoReconnect?.checked ?? true,
        autoStart: elements.autoStart?.checked ?? false,
        startMinimized: elements.startMinimized?.checked ?? false,
        clipboardSync: elements.clipboardSync ? elements.clipboardSync.checked : true,
        keyMappingUnified: elements.keyMappingUnified ? elements.keyMappingUnified.checked : true,
        enableTls: elements.enableTls ? elements.enableTls.checked : true,
        language: elements.languageSelect?.value || (currentSettings?.language || 'zh'),
        theme: elements.themeSelect?.value || (currentSettings?.theme || 'auto'),
    };
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

window.SettingsModule = {
    DEFAULT_SETTINGS,
    loadSettingsFromStorage,
    saveSettingsToStorage,
    loadSettingsToUI,
    collectSettingsFromUI,
};
