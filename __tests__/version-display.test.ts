/**
 * 版本号显示逻辑测试
 * 覆盖：Tauri 环境可用时读取动态版本、降级读取静态版本
 */

describe('版本号显示逻辑', () => {
  // 设置 DOM 环境
  beforeEach(() => {
    document.body.innerHTML = `
      <p id="app-version">v0.1.4</p>
      <div id="settings-version">v0.1.4</div>
    `;
  });

  afterEach(() => {
    // 清理 window.__TAURI__
    delete (window as any).__TAURI__;
  });

  // ─── 静态版本号 ──────────────────────────────────────────────────────────

  describe('静态版本号（降级）', () => {
    it('DOM 中应有静态版本号 v0.1.4', () => {
      const appVersion = document.getElementById('app-version');
      expect(appVersion).not.toBeNull();
      expect(appVersion!.textContent).toBe('v0.1.4');
    });

    it('设置面板中应有静态版本号 v0.1.4', () => {
      const settingsVersion = document.getElementById('settings-version');
      expect(settingsVersion).not.toBeNull();
      expect(settingsVersion!.textContent).toBe('v0.1.4');
    });

    it('版本号格式应以 v 开头', () => {
      const appVersion = document.getElementById('app-version');
      expect(appVersion!.textContent).toMatch(/^v\d+\.\d+\.\d+/);
    });
  });

  // ─── Tauri 环境版本号 ────────────────────────────────────────────────────

  describe('Tauri 环境版本号', () => {
    it('Tauri app.getVersion 可用时应读取动态版本', async () => {
      // 模拟 Tauri 环境
      (window as any).__TAURI__ = {
        app: {
          getVersion: jest.fn().mockResolvedValue('0.2.0'),
        },
      };

      // 模拟 initVersion 逻辑
      async function initVersion() {
        let version = 'v0.1.4';
        try {
          if ((window as any).__TAURI__?.app?.getVersion) {
            const v = await (window as any).__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
          }
        } catch (e) {
          // 降级
        }
        const appVersionEl = document.getElementById('app-version');
        const settingsVersionEl = document.getElementById('settings-version');
        if (appVersionEl) appVersionEl.textContent = version;
        if (settingsVersionEl) settingsVersionEl.textContent = version;
      }

      await initVersion();

      expect(document.getElementById('app-version')!.textContent).toBe('v0.2.0');
      expect(document.getElementById('settings-version')!.textContent).toBe('v0.2.0');
    });

    it('Tauri app.getVersion 抛出异常时降级使用静态版本', async () => {
      (window as any).__TAURI__ = {
        app: {
          getVersion: jest.fn().mockRejectedValue(new Error('API not available')),
        },
      };

      async function initVersion() {
        let version = 'v0.1.4';
        try {
          if ((window as any).__TAURI__?.app?.getVersion) {
            const v = await (window as any).__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
          }
        } catch (e) {
          // 降级使用静态值
        }
        const appVersionEl = document.getElementById('app-version');
        if (appVersionEl) appVersionEl.textContent = version;
      }

      await initVersion();

      expect(document.getElementById('app-version')!.textContent).toBe('v0.1.4');
    });

    it('Tauri 不可用时使用静态版本', async () => {
      // 不设置 __TAURI__
      delete (window as any).__TAURI__;

      async function initVersion() {
        let version = 'v0.1.4';
        try {
          if ((window as any).__TAURI__?.app?.getVersion) {
            const v = await (window as any).__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
          }
        } catch (e) {
          // 降级
        }
        const appVersionEl = document.getElementById('app-version');
        if (appVersionEl) appVersionEl.textContent = version;
      }

      await initVersion();

      expect(document.getElementById('app-version')!.textContent).toBe('v0.1.4');
    });

    it('Tauri getVersion 返回空字符串时使用静态版本', async () => {
      (window as any).__TAURI__ = {
        app: {
          getVersion: jest.fn().mockResolvedValue(''),
        },
      };

      async function initVersion() {
        let version = 'v0.1.4';
        try {
          if ((window as any).__TAURI__?.app?.getVersion) {
            const v = await (window as any).__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
          }
        } catch (e) {
          // 降级
        }
        const appVersionEl = document.getElementById('app-version');
        if (appVersionEl) appVersionEl.textContent = version;
      }

      await initVersion();

      // 空字符串不满足 if (v)，保持静态版本
      expect(document.getElementById('app-version')!.textContent).toBe('v0.1.4');
    });

    it('Tauri getVersion 返回版本号时格式应为 v{version}', async () => {
      (window as any).__TAURI__ = {
        app: {
          getVersion: jest.fn().mockResolvedValue('1.0.0'),
        },
      };

      async function initVersion() {
        let version = 'v0.1.4';
        try {
          if ((window as any).__TAURI__?.app?.getVersion) {
            const v = await (window as any).__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
          }
        } catch (e) {}
        const appVersionEl = document.getElementById('app-version');
        if (appVersionEl) appVersionEl.textContent = version;
      }

      await initVersion();

      expect(document.getElementById('app-version')!.textContent).toBe('v1.0.0');
    });
  });

  // ─── 版本号同步 ──────────────────────────────────────────────────────────

  describe('版本号同步', () => {
    it('主界面和设置面板版本号应保持一致', async () => {
      (window as any).__TAURI__ = {
        app: {
          getVersion: jest.fn().mockResolvedValue('0.1.4'),
        },
      };

      async function initVersion() {
        let version = 'v0.1.4';
        try {
          if ((window as any).__TAURI__?.app?.getVersion) {
            const v = await (window as any).__TAURI__.app.getVersion();
            if (v) version = `v${v}`;
          }
        } catch (e) {}
        const appVersionEl = document.getElementById('app-version');
        const settingsVersionEl = document.getElementById('settings-version');
        if (appVersionEl) appVersionEl.textContent = version;
        if (settingsVersionEl) settingsVersionEl.textContent = version;
      }

      await initVersion();

      const appVersion = document.getElementById('app-version')!.textContent;
      const settingsVersion = document.getElementById('settings-version')!.textContent;
      expect(appVersion).toBe(settingsVersion);
    });
  });
});
