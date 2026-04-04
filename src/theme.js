/**
 * theme 主题切换模块
 * 支持深色（dark）、浅色（light）、跟随系统（auto）
 * 主题偏好持久化到 localStorage（key: kvm_settings）
 */

/** 当前主题设置，默认跟随系统 */
let _theme = 'auto';

/** 系统主题媒体查询监听器引用 */
let _mediaQueryListener = null;

/**
 * 获取当前主题设置
 * @returns {'dark'|'light'|'auto'}
 */
function getTheme() {
  return _theme;
}

/**
 * 应用主题到 <html> 元素
 * @param {'dark'|'light'|'auto'} theme
 */
function applyTheme(theme) {
  const html = document.documentElement;
  if (theme === 'dark') {
    html.setAttribute('data-theme', 'dark');
  } else if (theme === 'light') {
    html.setAttribute('data-theme', 'light');
  } else {
    // auto：跟随系统
    const prefersDark = window.matchMedia &&
      window.matchMedia('(prefers-color-scheme: dark)').matches;
    html.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
  }
}

/**
 * 设置主题（立即生效，不持久化）
 * @param {'dark'|'light'|'auto'} theme
 */
function setTheme(theme) {
  if (theme !== 'dark' && theme !== 'light' && theme !== 'auto') return;

  // 移除旧的系统主题监听器
  _removeSystemThemeListener();

  _theme = theme;
  applyTheme(theme);

  // 如果是 auto，监听系统主题变化
  if (theme === 'auto' && window.matchMedia) {
    _addSystemThemeListener();
  }
}

/**
 * 添加系统主题变化监听器
 */
function _addSystemThemeListener() {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  _mediaQueryListener = (e) => {
    if (_theme === 'auto') {
      document.documentElement.setAttribute('data-theme', e.matches ? 'dark' : 'light');
    }
  };
  if (mq.addEventListener) {
    mq.addEventListener('change', _mediaQueryListener);
  } else if (mq.addListener) {
    // 兼容旧版浏览器
    mq.addListener(_mediaQueryListener);
  }
}

/**
 * 移除系统主题变化监听器
 */
function _removeSystemThemeListener() {
  if (_mediaQueryListener && window.matchMedia) {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    if (mq.removeEventListener) {
      mq.removeEventListener('change', _mediaQueryListener);
    } else if (mq.removeListener) {
      mq.removeListener(_mediaQueryListener);
    }
    _mediaQueryListener = null;
  }
}

/**
 * 从 localStorage 加载主题设置并立即应用（避免闪烁，应在 <head> 中尽早调用）
 */
function loadThemeFromStorage() {
  try {
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || 'null');
    const theme = (saved && saved.theme) ? saved.theme : 'auto';
    setTheme(theme);
  } catch (_) {
    setTheme('auto');
  }
}

// 导出（兼容浏览器全局和 Node.js 测试环境）
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    getTheme,
    setTheme,
    applyTheme,
    loadThemeFromStorage,
    _addSystemThemeListener,
    _removeSystemThemeListener,
  };
} else {
  window.themeManager = {
    getTheme,
    setTheme,
    applyTheme,
    loadThemeFromStorage,
  };
}
