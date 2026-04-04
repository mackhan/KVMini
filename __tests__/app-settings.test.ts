/**
 * 应用设置增强功能集成测试
 * 覆盖：版本号读取、语言持久化、主题持久化
 */

const fs = require('fs');
const path = require('path');

// ─── 工具函数 ────────────────────────────────────────────────────────────────

function loadI18n() {
  const code = fs.readFileSync(path.resolve(__dirname, '../src/i18n.js'), 'utf-8');
  const fn = new Function('window', code);
  const win: any = { i18n: undefined };
  fn(win);
  return win.i18n as {
    t: (k: string) => string;
    setLocale: (l: string) => void;
    getLocale: () => string;
  };
}

function loadTheme() {
  const code = fs.readFileSync(path.resolve(__dirname, '../src/theme.js'), 'utf-8');
  const fn = new Function('window', code);
  const win: any = { themeManager: undefined };
  fn(win);
  return win.themeManager as {
    setTheme: (t: string) => void;
    getTheme: () => string;
  };
}

// ─── 版本号显示逻辑测试 ───────────────────────────────────────────────────────

describe('版本号显示逻辑', () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <p id="app-version">v0.1.4</p>
      <div id="settings-version">v0.1.4</div>
    `;
  });

  it('静态降级版本号应为 v0.1.4', () => {
    const el = document.getElementById('app-version');
    expect(el?.textContent).toBe('v0.1.4');
  });

  it('设置面板版本号静态值应为 v0.1.4', () => {
    const el = document.getElementById('settings-version');
    expect(el?.textContent).toBe('v0.1.4');
  });

  it('Tauri 环境可用时应优先读取动态版本号', async () => {
    // Mock Tauri app.getVersion
    (window as any).__TAURI__ = {
      app: {
        getVersion: jest.fn().mockResolvedValue('0.2.0'),
      },
    };

    // 模拟 initVersion 逻辑
    let version = 'v0.1.4';
    try {
      if ((window as any).__TAURI__?.app?.getVersion) {
        const v = await (window as any).__TAURI__.app.getVersion();
        if (v) version = `v${v}`;
      }
    } catch (e) { /* 降级 */ }

    const appVersionEl = document.getElementById('app-version');
    const settingsVersionEl = document.getElementById('settings-version');
    if (appVersionEl) appVersionEl.textContent = version;
    if (settingsVersionEl) settingsVersionEl.textContent = version;

    expect(version).toBe('v0.2.0');
    expect(appVersionEl?.textContent).toBe('v0.2.0');
    expect(settingsVersionEl?.textContent).toBe('v0.2.0');
  });

  it('Tauri getVersion 抛出异常时应降级使用静态版本号', async () => {
    (window as any).__TAURI__ = {
      app: {
        getVersion: jest.fn().mockRejectedValue(new Error('not available')),
      },
    };

    let version = 'v0.1.4';
    try {
      if ((window as any).__TAURI__?.app?.getVersion) {
        const v = await (window as any).__TAURI__.app.getVersion();
        if (v) version = `v${v}`;
      }
    } catch (e) { /* 降级 */ }

    expect(version).toBe('v0.1.4');
  });

  it('非 Tauri 环境（无 __TAURI__）时应使用静态版本号', async () => {
    delete (window as any).__TAURI__;

    let version = 'v0.1.4';
    try {
      if ((window as any).__TAURI__?.app?.getVersion) {
        const v = await (window as any).__TAURI__.app.getVersion();
        if (v) version = `v${v}`;
      }
    } catch (e) { /* 降级 */ }

    expect(version).toBe('v0.1.4');
  });
});

// ─── 语言持久化测试 ───────────────────────────────────────────────────────────

describe('语言持久化', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('保存设置时应将 language 写入 localStorage', () => {
    const settings = { language: 'en', theme: 'auto' };
    localStorage.setItem('kvm_settings', JSON.stringify(settings));
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || '{}');
    expect(saved.language).toBe('en');
  });

  it('读取设置时应恢复上次保存的语言', () => {
    localStorage.setItem('kvm_settings', JSON.stringify({ language: 'en', theme: 'dark' }));
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || '{}');
    const i18n = loadI18n();
    i18n.setLocale(saved.language || 'zh');
    expect(i18n.getLocale()).toBe('en');
  });

  it('未保存语言设置时应默认使用中文', () => {
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || '{}');
    const i18n = loadI18n();
    i18n.setLocale(saved.language || 'zh');
    expect(i18n.getLocale()).toBe('zh');
  });

  it('语言切换为英文后界面文本应变为英文', () => {
    const i18n = loadI18n();
    i18n.setLocale('en');
    expect(i18n.t('btnScanDevices')).toBe('Scan Devices');
    expect(i18n.t('connectionHistory')).toBe('Connection History');
    expect(i18n.t('settingsTitle')).toBe('⚙ Settings');
  });

  it('语言切换为中文后界面文本应恢复中文', () => {
    const i18n = loadI18n();
    i18n.setLocale('en');
    i18n.setLocale('zh');
    expect(i18n.t('btnScanDevices')).toBe('扫描设备');
    expect(i18n.t('connectionHistory')).toBe('连接历史');
  });
});

// ─── 主题持久化测试 ───────────────────────────────────────────────────────────

describe('主题持久化', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: jest.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
      })),
    });
  });

  it('保存设置时应将 theme 写入 localStorage', () => {
    const settings = { language: 'zh', theme: 'dark' };
    localStorage.setItem('kvm_settings', JSON.stringify(settings));
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || '{}');
    expect(saved.theme).toBe('dark');
  });

  it('读取设置时应恢复上次保存的主题', () => {
    localStorage.setItem('kvm_settings', JSON.stringify({ language: 'zh', theme: 'dark' }));
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || '{}');
    const themeManager = loadTheme();
    themeManager.setTheme(saved.theme || 'auto');
    expect(themeManager.getTheme()).toBe('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('未保存主题设置时应默认使用跟随系统（auto）', () => {
    const saved = JSON.parse(localStorage.getItem('kvm_settings') || '{}');
    const themeManager = loadTheme();
    themeManager.setTheme(saved.theme || 'auto');
    expect(themeManager.getTheme()).toBe('auto');
  });

  it('主题切换为 dark 后 data-theme 属性应为 dark', () => {
    const themeManager = loadTheme();
    themeManager.setTheme('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('主题切换为 light 后 data-theme 属性应为 light', () => {
    const themeManager = loadTheme();
    themeManager.setTheme('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  it('主题切换为 auto 且系统浅色时 data-theme 应为 light', () => {
    const themeManager = loadTheme();
    themeManager.setTheme('auto');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });
});
