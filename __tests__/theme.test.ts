/**
 * theme 主题切换模块测试
 * 覆盖：主题切换、data-theme 属性设置、跟随系统逻辑、持久化
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

// 模拟 matchMedia
const mockMatchMedia = (prefersDark: boolean) => {
  const listeners: Array<(e: { matches: boolean }) => void> = [];
  const mq = {
    matches: prefersDark,
    addEventListener: jest.fn((event: string, cb: (e: { matches: boolean }) => void) => {
      listeners.push(cb);
    }),
    removeEventListener: jest.fn((event: string, cb: (e: { matches: boolean }) => void) => {
      const idx = listeners.indexOf(cb);
      if (idx >= 0) listeners.splice(idx, 1);
    }),
    addListener: jest.fn(),
    removeListener: jest.fn(),
    // 触发变化（用于测试）
    _trigger: (matches: boolean) => {
      mq.matches = matches;
      listeners.forEach(cb => cb({ matches }));
    },
  };
  return mq;
};

let currentMq = mockMatchMedia(false);

Object.defineProperty(global, 'matchMedia', {
  writable: true,
  value: jest.fn(() => currentMq),
});

// 加载 theme 模块
const themeModule = require('../src/theme.js');
const { getTheme, setTheme, applyTheme, loadThemeFromStorage } = themeModule;

describe('theme 主题切换模块', () => {
  beforeEach(() => {
    // 重置 html 属性
    document.documentElement.removeAttribute('data-theme');
    // 重置为浅色系统主题
    currentMq = mockMatchMedia(false);
    (global as any).matchMedia = jest.fn(() => currentMq);
    // 重置主题为 auto
    setTheme('auto');
    localStorage.clear();
  });

  // ─── 主题切换 ────────────────────────────────────────────────────────────

  describe('主题切换', () => {
    it('setTheme("dark") 应设置 data-theme="dark"', () => {
      setTheme('dark');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      expect(getTheme()).toBe('dark');
    });

    it('setTheme("light") 应设置 data-theme="light"', () => {
      setTheme('light');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
      expect(getTheme()).toBe('light');
    });

    it('setTheme("auto") 在浅色系统下应设置 data-theme="light"', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      setTheme('auto');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
      expect(getTheme()).toBe('auto');
    });

    it('setTheme("auto") 在深色系统下应设置 data-theme="dark"', () => {
      currentMq = mockMatchMedia(true);
      (global as any).matchMedia = jest.fn(() => currentMq);
      setTheme('auto');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      expect(getTheme()).toBe('auto');
    });

    it('传入无效主题值时不改变当前主题', () => {
      setTheme('dark');
      setTheme('invalid' as any);
      expect(getTheme()).toBe('dark');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('从深色切换到浅色时 data-theme 正确更新', () => {
      setTheme('dark');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      setTheme('light');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });
  });

  // ─── applyTheme 函数 ──────────────────────────────────────────────────────

  describe('applyTheme 函数', () => {
    it('applyTheme("dark") 设置 data-theme="dark"', () => {
      applyTheme('dark');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('applyTheme("light") 设置 data-theme="light"', () => {
      applyTheme('light');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });

    it('applyTheme("auto") 在浅色系统下设置 data-theme="light"', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      applyTheme('auto');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });

    it('applyTheme("auto") 在深色系统下设置 data-theme="dark"', () => {
      currentMq = mockMatchMedia(true);
      (global as any).matchMedia = jest.fn(() => currentMq);
      applyTheme('auto');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });
  });

  // ─── 跟随系统逻辑 ────────────────────────────────────────────────────────

  describe('跟随系统逻辑', () => {
    it('auto 模式下系统主题变化时 data-theme 自动更新', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      setTheme('auto');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');

      // 模拟系统切换到深色
      currentMq._trigger(true);
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('切换到 dark 后不再响应系统主题变化', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      setTheme('auto');

      // 切换到固定深色
      setTheme('dark');

      // 模拟系统切换到浅色（不应影响 data-theme）
      currentMq._trigger(false);
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('auto 模式下 addEventListener 被调用', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      setTheme('auto');
      expect(currentMq.addEventListener).toHaveBeenCalledWith('change', expect.any(Function));
    });

    it('从 auto 切换到 dark 时 removeEventListener 被调用', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      setTheme('auto');
      setTheme('dark');
      expect(currentMq.removeEventListener).toHaveBeenCalled();
    });
  });

  // ─── 持久化（loadThemeFromStorage）──────────────────────────────────────

  describe('持久化（loadThemeFromStorage）', () => {
    it('localStorage 中有 theme=dark 时加载深色主题', () => {
      localStorage.setItem('kvm_settings', JSON.stringify({ theme: 'dark' }));
      loadThemeFromStorage();
      expect(getTheme()).toBe('dark');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('localStorage 中有 theme=light 时加载浅色主题', () => {
      localStorage.setItem('kvm_settings', JSON.stringify({ theme: 'light' }));
      loadThemeFromStorage();
      expect(getTheme()).toBe('light');
      expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    });

    it('localStorage 中有 theme=auto 时加载跟随系统', () => {
      currentMq = mockMatchMedia(false);
      (global as any).matchMedia = jest.fn(() => currentMq);
      localStorage.setItem('kvm_settings', JSON.stringify({ theme: 'auto' }));
      loadThemeFromStorage();
      expect(getTheme()).toBe('auto');
    });

    it('localStorage 为空时默认使用 auto 主题', () => {
      localStorage.clear();
      loadThemeFromStorage();
      expect(getTheme()).toBe('auto');
    });

    it('localStorage 中 JSON 格式错误时不崩溃，使用 auto', () => {
      localStorage.setItem('kvm_settings', 'invalid json{');
      expect(() => loadThemeFromStorage()).not.toThrow();
      expect(getTheme()).toBe('auto');
    });

    it('localStorage 中 theme 为无效值时使用 auto', () => {
      localStorage.setItem('kvm_settings', JSON.stringify({ theme: 'invalid' }));
      loadThemeFromStorage();
      // invalid 值会被 setTheme 忽略，但 loadThemeFromStorage 传入 'invalid' 后 setTheme 不改变
      // 实际上 loadThemeFromStorage 直接调用 setTheme，setTheme 会忽略无效值
      // 所以主题保持为上一次的值（auto）
      expect(getTheme()).toBe('auto');
    });
  });
});
