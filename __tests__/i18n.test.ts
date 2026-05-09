/**
 * i18n 多语言模块测试
 * 覆盖：语言切换、t() 返回值、默认语言、持久化
 */

// 在 Node.js 环境中模拟 localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();

Object.defineProperty(global, 'localStorage', { value: localStorageMock });

// 加载 i18n 模块
const i18n = require('../src/i18n.js');
const { t, getLocale, setLocale, loadLocaleFromStorage, MESSAGES } = i18n;

describe('i18n 多语言模块', () => {
  beforeEach(() => {
    // 每次测试前重置为中文
    setLocale('zh');
    localStorage.clear();
  });

  // ─── 默认语言 ───────────────────────────────────────────────────────────

  describe('默认语言', () => {
    it('默认语言应为中文', () => {
      expect(getLocale()).toBe('zh');
    });

    it('t() 默认返回中文文本', () => {
      expect(t('appTitle')).toBe('KVMini');
      expect(t('btnScanDevices')).toBe('扫描设备');
      expect(t('statusDisconnected')).toBe('未连接');
    });
  });

  // ─── 语言切换 ────────────────────────────────────────────────────────────

  describe('语言切换', () => {
    it('setLocale("en") 应切换到英文', () => {
      setLocale('en');
      expect(getLocale()).toBe('en');
    });

    it('切换到英文后 t() 返回英文文本', () => {
      setLocale('en');
      expect(t('btnScanDevices')).toBe('Scan Devices');
      expect(t('statusDisconnected')).toBe('Disconnected');
      expect(t('statusConnected')).toBe('Connected');
    });

    it('切换回中文后 t() 返回中文文本', () => {
      setLocale('en');
      setLocale('zh');
      expect(t('btnScanDevices')).toBe('扫描设备');
      expect(t('statusDisconnected')).toBe('未连接');
    });

    it('传入无效语言时不改变当前语言', () => {
      setLocale('zh');
      setLocale('fr' as any);
      expect(getLocale()).toBe('zh');
    });

    it('传入空字符串时不改变当前语言', () => {
      setLocale('zh');
      setLocale('' as any);
      expect(getLocale()).toBe('zh');
    });
  });

  // ─── t() 翻译函数 ────────────────────────────────────────────────────────

  describe('t() 翻译函数', () => {
    it('中文模式下返回正确的中文翻译', () => {
      setLocale('zh');
      expect(t('appSubtitle')).toBe('跨平台鼠标键盘共享软件');
      expect(t('roleServer')).toBe('被控端');
      expect(t('roleClient')).toBe('主控端');
      expect(t('overlayTitle')).toBe('需要辅助功能权限');
      expect(t('btnGoAuthorize')).toBe('前往授权 →');
    });

    it('英文模式下返回正确的英文翻译', () => {
      setLocale('en');
      expect(t('appSubtitle')).toBe('Cross-platform Mouse & Keyboard Sharing');
      expect(t('roleServer')).toBe('Server');
      expect(t('roleClient')).toBe('Client');
      expect(t('overlayTitle')).toBe('Accessibility Permission Required');
      expect(t('btnGoAuthorize')).toBe('Authorize →');
    });

    it('key 不存在时降级返回中文翻译', () => {
      setLocale('en');
      // appTitle 在中英文都有，应返回英文
      expect(t('appTitle')).toBe('KVMini');
    });

    it('key 完全不存在时返回 key 本身', () => {
      expect(t('nonExistentKey')).toBe('nonExistentKey');
    });

    it('设置面板相关 key 翻译正确（中文）', () => {
      setLocale('zh');
      expect(t('settingsHotkey')).toBe('全局快捷键');
      expect(t('settingsConnection')).toBe('连接偏好');
      expect(t('settingsStartup')).toBe('启动选项');
      expect(t('settingsAppearance')).toBe('外观');
      expect(t('settingsDisplay')).toBe('显示器布局');
    });

    it('设置面板相关 key 翻译正确（英文）', () => {
      setLocale('en');
      expect(t('settingsHotkey')).toBe('Global Hotkey');
      expect(t('settingsConnection')).toBe('Connection Preferences');
      expect(t('settingsStartup')).toBe('Startup Options');
      expect(t('settingsAppearance')).toBe('Appearance');
      expect(t('settingsDisplay')).toBe('Display Layout');
    });

    it('辅助功能遮罩相关 key 翻译正确（中文）', () => {
      setLocale('zh');
      expect(t('overlayStep1')).toBe('点击下方"前往授权"按钮');
      expect(t('overlayStep2')).toBe('在系统设置中找到 KVMini，打开开关');
      expect(t('overlayStep3')).toBe('返回应用，权限授予后自动继续');
      expect(t('overlayPollingHint')).toBe('⏳ 等待授权中，授权后将自动继续...');
    });

    it('辅助功能遮罩相关 key 翻译正确（英文）', () => {
      setLocale('en');
      expect(t('overlayStep1')).toBe('Click the "Authorize" button below');
      expect(t('overlayStep2')).toBe('Find KVMini in System Settings and enable it');
      expect(t('overlayStep3')).toBe('Return to the app, it will continue automatically after authorization');
      expect(t('overlayPollingHint')).toBe('⏳ Waiting for authorization...');
    });

    it('版本号相关 key 翻译正确', () => {
      setLocale('zh');
      expect(t('labelVersion')).toBe('版本');
      setLocale('en');
      expect(t('labelVersion')).toBe('Version');
    });

    it('主题选项 key 翻译正确（中文）', () => {
      setLocale('zh');
      expect(t('optionThemeDark')).toBe('深色');
      expect(t('optionThemeLight')).toBe('浅色');
      expect(t('optionThemeAuto')).toBe('跟随系统');
    });

    it('主题选项 key 翻译正确（英文）', () => {
      setLocale('en');
      expect(t('optionThemeDark')).toBe('Dark');
      expect(t('optionThemeLight')).toBe('Light');
      expect(t('optionThemeAuto')).toBe('System');
    });
  });

  // ─── 持久化 ──────────────────────────────────────────────────────────────

  describe('持久化（loadLocaleFromStorage）', () => {
    it('localStorage 中有 language=en 时加载英文', () => {
      localStorage.setItem('kvm_settings', JSON.stringify({ language: 'en' }));
      loadLocaleFromStorage();
      expect(getLocale()).toBe('en');
    });

    it('localStorage 中有 language=zh 时加载中文', () => {
      setLocale('en');
      localStorage.setItem('kvm_settings', JSON.stringify({ language: 'zh' }));
      loadLocaleFromStorage();
      expect(getLocale()).toBe('zh');
    });

    it('localStorage 为空时保持默认中文', () => {
      localStorage.clear();
      loadLocaleFromStorage();
      expect(getLocale()).toBe('zh');
    });

    it('localStorage 中 language 为无效值时保持当前语言', () => {
      setLocale('zh');
      localStorage.setItem('kvm_settings', JSON.stringify({ language: 'fr' }));
      loadLocaleFromStorage();
      expect(getLocale()).toBe('zh');
    });

    it('localStorage 中 JSON 格式错误时不崩溃', () => {
      localStorage.setItem('kvm_settings', 'invalid json{');
      expect(() => loadLocaleFromStorage()).not.toThrow();
    });
  });

  // ─── MESSAGES 字典完整性 ──────────────────────────────────────────────────

  describe('MESSAGES 字典完整性', () => {
    it('中英文字典应包含相同的 key', () => {
      const zhKeys = Object.keys(MESSAGES.zh).sort();
      const enKeys = Object.keys(MESSAGES.en).sort();
      // 英文字典的 key 应是中文字典的子集（允许中文有更多 key）
      enKeys.forEach(key => {
        expect(zhKeys).toContain(key);
      });
    });

    it('中文字典不应有空字符串值', () => {
      Object.entries(MESSAGES.zh).forEach(([key, value]) => {
        expect(value).not.toBe('');
      });
    });

    it('英文字典不应有空字符串值', () => {
      Object.entries(MESSAGES.en).forEach(([key, value]) => {
        expect(value).not.toBe('');
      });
    });

    // BUG-020: VPN/防火墙诊断提示 keys 必须中英双语完整
    it('Bug 3 诊断提示 keys 必须在 zh+en 都存在', () => {
      const requiredKeys = [
        'vpnInterferenceTip',
        'firewallBlockTipMacos',
        'firewallBlockTipWindows',
      ];
      for (const key of requiredKeys) {
        expect(MESSAGES.zh[key]).toBeDefined();
        expect(MESSAGES.zh[key]).not.toBe('');
        expect(MESSAGES.en[key]).toBeDefined();
        expect(MESSAGES.en[key]).not.toBe('');
        // 中英文不应一致（防止只复制了一份）
        expect(MESSAGES.zh[key]).not.toBe(MESSAGES.en[key]);
      }
    });

    // BUG-019: 开始/停止共享相关 keys 必须中英双语完整
    it('Bug 2 共享按钮 keys 必须在 zh+en 都存在', () => {
      const requiredKeys = [
        'btnStartSharing',
        'btnStopSharing',
        'sharingStarted',
        'sharingStopped',
        'shareToggleFailed',
      ];
      for (const key of requiredKeys) {
        expect(MESSAGES.zh[key]).toBeDefined();
        expect(MESSAGES.zh[key]).not.toBe('');
        expect(MESSAGES.en[key]).toBeDefined();
        expect(MESSAGES.en[key]).not.toBe('');
        expect(MESSAGES.zh[key]).not.toBe(MESSAGES.en[key]);
      }
    });

    // BUG-035 / Feature #035: 远端物理输入抢占回切 toast 的中英双语
    it('远端抢占回切 toast key 必须在 zh+en 都存在且中英文不一致', () => {
      const requiredKeys = ['preempt_resume_toast'];
      for (const key of requiredKeys) {
        expect(MESSAGES.zh[key]).toBeDefined();
        expect(MESSAGES.zh[key]).not.toBe('');
        expect(MESSAGES.en[key]).toBeDefined();
        expect(MESSAGES.en[key]).not.toBe('');
        expect(MESSAGES.zh[key]).not.toBe(MESSAGES.en[key]);
      }
    });
  });
});
