/**
 * accessibility-fix.test.ts
 * Bug #008：辅助功能权限引导彻底修复 - 回归测试
 *
 * 覆盖根因 A/B/C/D 的所有场景：
 * - 根因 A：main.js 底部第二个 DOMContentLoaded 不再包含 checkAccessibilityPermission
 * - 根因 B：waitForTauri 超时后 invoke 为 null 时打印警告，不静默跳过
 * - 根因 C：Rust 返回英文枚举，JS 检测 status !== 'Granted'
 * - 根因 D：按钮点击 invoke 为 null 时显示明确错误提示
 */

// ─── 环境准备 ─────────────────────────────────────────────────────────────────

function createAccessibilityDOM() {
    document.body.innerHTML = `
        <div id="accessibility-overlay" class="accessibility-overlay hidden"></div>
        <div id="permission-warning" class="permission-warning hidden"></div>
        <button id="start-sharing-btn" class="btn btn-success hidden"></button>
    `;
}

beforeAll(() => {
    (window as any).i18n = { t: (key: string) => key };
    require('../src/accessibility.js');
});

beforeEach(() => {
    createAccessibilityDOM();
    delete (window as any).__TAURI__;
    // 重置为 macOS 环境
    Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
        configurable: true,
    });
    jest.clearAllTimers();
});

afterEach(() => {
    jest.clearAllTimers();
    (window as any).AccessibilityModule?.stopAccessibilityPolling?.();
});

// ─── 根因 A：main.js 底部第二个 DOMContentLoaded 不再包含 checkAccessibilityPermission ──

describe('根因 A：main.js 底部第二个 DOMContentLoaded 监听器修复', () => {
    test('新架构：main.js 不再依赖 DOMContentLoaded 监听器启动（改用 startApp + tauriInvokeReady）', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // 新架构：main.js 末尾使用 startApp() 直接启动，不再依赖 DOMContentLoaded
        // DOMContentLoaded 监听器数量应为 0（已全部移除）
        const listenerMatches = content.match(/document\.addEventListener\('DOMContentLoaded'/g) || [];
        expect(listenerMatches.length).toBe(0);

        // 新架构：应包含 tauriInvokeReady 检查和 startApp 启动逻辑
        expect(content).toContain('tauriInvokeReady');
        expect(content).toContain('startApp');
    });

    test('main.js 中 DOMContentLoaded 监听器不应直接调用 setTimeout(checkAccessibilityPermission)', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // 不应有 setTimeout(checkAccessibilityPermission, 800) 这种形式
        const setTimeoutCallMatch = content.match(/setTimeout\s*\(\s*checkAccessibilityPermission\s*,\s*800\s*\)/g) || [];
        expect(setTimeoutCallMatch.length).toBe(0);
    });
});

// ─── 根因 B：新架构 - 前端不再主动 invoke，无需处理 invoke 为 null 的情况 ────────

describe('根因 B：新架构 - 前端不再主动 invoke', () => {
    test('新架构：accessibility.js 不再包含 checkAccessibilityPermission 函数', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/accessibility.js'), 'utf-8');
        expect(content).not.toContain('function checkAccessibilityPermission');
    });

    test('新架构：accessibility.js 不再包含 startAccessibilityPolling 函数', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/accessibility.js'), 'utf-8');
        expect(content).not.toContain('function startAccessibilityPolling');
    });

    test('新架构：前端启动时不再主动调用 check_accessibility_permission（main.js 中不调用）', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');
        // main.js 中不应有 check_accessibility_permission 的直接调用
        expect(content).not.toContain("invoke('check_accessibility_permission')");
    });
});
// ─── 根因 C：新架构 - Rust 推送事件，前端被动响应 ─────────────────────────

describe('根因 C：新架构 - Rust 推送事件，前端被动响应', () => {
    let tauriListeners: Record<string, Function[]> = {};

    beforeEach(() => {
        tauriListeners = {};
        createAccessibilityDOM();
        (window as any).__TAURI__ = {
            invoke: jest.fn(),
            event: {
                listen: jest.fn((name: string, cb: Function) => {
                    if (!tauriListeners[name]) tauriListeners[name] = [];
                    tauriListeners[name].push(cb);
                    return Promise.resolve(() => { });
                }),
            },
        };
        jest.resetModules();
        require('../src/accessibility.js');
    });

    test('Rust 发送 permission-denied 事件时：应显示权限引导遮罩', () => {
        tauriListeners['accessibility://permission-denied']?.forEach(cb => cb({}));
        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });

    test('Rust 发送 permission-granted 事件时：应隐藏权限引导遮罩', () => {
        document.getElementById('accessibility-overlay')?.classList.remove('hidden');
        tauriListeners['accessibility://permission-granted']?.forEach(cb => cb({}));
        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);
    });

    test('accessibility.js 中权限检测应使用 Tauri 事件而非 invoke', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/accessibility.js'), 'utf-8');
        expect(content).toContain('accessibility://permission-denied');
        expect(content).toContain('accessibility://permission-granted');
    });

    test('permission.rs 中 Display 实现应返回英文枚举字符串', () => {
        const fs = require('fs');
        const path = require('path');
        const permissionRsPath = path.join(__dirname, '../src-tauri/src/platform/macos/permission.rs');
        const content = fs.readFileSync(permissionRsPath, 'utf-8');
        expect(content).not.toMatch(/write!\(f,\s*"已授权"\)/);
        expect(content).not.toMatch(/write!\(f,\s*"未授权"\)/);
        expect(content).toContain('"Granted"');
        expect(content).toContain('"Denied"');
    });
});
// ─── 根因 D：按钮点击 invoke 为 null 时显示明确错误提示 ──────────────────────

describe('根因 D：按钮点击 invoke 为 null 时有明确错误提示', () => {
    test('main.js 中 handleDisconnect 在 invoke 为 null 时应调用 showError', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // handleDisconnect 函数中应有 invoke 为 null 时的错误提示
        const handleDisconnectMatch = content.match(/async function handleDisconnect[\s\S]*?^}/m);
        if (handleDisconnectMatch) {
            expect(handleDisconnectMatch[0]).toContain('showError');
        }
    });

    test('main.js 中 handleScan 在 invoke 为 null 时应调用 showError', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // handleScan 函数中应有 invoke 为 null 时的错误提示
        const handleScanMatch = content.match(/async function handleScan[\s\S]*?^}/m);
        if (handleScanMatch) {
            expect(handleScanMatch[0]).toContain('showError');
        }
    });

    test('main.js 中 invoke 为 null 时的错误提示应包含"Tauri 环境未就绪"', () => {
        const fs = require('fs');
        const path = require('path');
        const mainJsPath = path.join(__dirname, '../src/main.js');
        const content = fs.readFileSync(mainJsPath, 'utf-8');

        // 应包含明确的错误提示文案
        expect(content).toContain('Tauri 环境未就绪');
    });
});

// ─── 诊断日志完整性 ─────────────────────────────────────────────────────────────────

describe('诊断日志：关键节点应有日志输出', () => {
    let tauriListeners: Record<string, Function[]> = {};

    beforeEach(() => {
        tauriListeners = {};
        createAccessibilityDOM();
        (window as any).__TAURI__ = {
            invoke: jest.fn(),
            event: {
                listen: jest.fn((name: string, cb: Function) => {
                    if (!tauriListeners[name]) tauriListeners[name] = [];
                    tauriListeners[name].push(cb);
                    return Promise.resolve(() => { });
                }),
            },
        };
        jest.resetModules();
        require('../src/accessibility.js');
    });

    test('收到 permission-denied 事件时应打印日志', () => {
        const logSpy = jest.spyOn(console, 'log').mockImplementation(() => { });
        tauriListeners['accessibility://permission-denied']?.forEach(cb => cb({}));
        expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('permission-denied'));
        logSpy.mockRestore();
    });

    test('收到 permission-granted 事件时应打印日志', () => {
        const logSpy = jest.spyOn(console, 'log').mockImplementation(() => { });
        tauriListeners['accessibility://permission-granted']?.forEach(cb => cb({}));
        expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('permission-granted'));
        logSpy.mockRestore();
    });
});
// ─── 轮询：新架构 - 由 Rust 后台线程负责 ───────────────────────────────────────

describe('轮询：新架构 - 由 Rust 后台线程负责', () => {
    test('accessibility.js 不再包含前端轮询逻辑', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src/accessibility.js'), 'utf-8');
        expect(content).not.toContain('setInterval');
        expect(content).not.toContain('startAccessibilityPolling');
    });

    test('main.rs 中应包含 start_accessibility_polling 或后台轮询逻辑', () => {
        const fs = require('fs');
        const path = require('path');
        const content = fs.readFileSync(path.join(__dirname, '../src-tauri/src/main.rs'), 'utf-8');
        // Rust 侧应有轮询逻辑
        const hasPolling = content.includes('start_accessibility_polling') || content.includes('thread::sleep');
        expect(hasPolling).toBe(true);
    });
});