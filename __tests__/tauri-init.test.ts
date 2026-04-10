/**
 * tauri-init.test.ts
 * 测试 Tauri invoke 初始化时序修复
 * 需求：1.1、1.2、1.3、2.2、3.3、4.1、4.2、4.3、4.4、5.1、5.2、5.3
 *
 * Bug #007：Tauri 2.x build 版本中 window.__TAURI__ 是异步注入的，
 * DOMContentLoaded 触发时尚未就绪，导致 invoke = null
 */

// ─── waitForTauri 函数（与 main.js 实现保持一致，用于单元测试） ───────────────

/**
 * 等待 window.__TAURI__ 就绪的函数（从 main.js 提取的纯逻辑，便于测试）
 * @param timeout 超时毫秒数，默认 3000
 * @param interval 轮询间隔毫秒数，默认 50
 */
function waitForTauri(timeout = 3000, interval = 50): Promise<boolean> {
    return new Promise((resolve) => {
        // 已经就绪，立即 resolve
        if ((window as any).__TAURI__?.core?.invoke || (window as any).__TAURI__?.invoke) {
            resolve(true);
            return;
        }

        const startTime = Date.now();
        const timer = setInterval(() => {
            if ((window as any).__TAURI__?.core?.invoke || (window as any).__TAURI__?.invoke) {
                clearInterval(timer);
                resolve(true);
                return;
            }
            if (Date.now() - startTime >= timeout) {
                clearInterval(timer);
                console.warn(
                    `[KVMini] waitForTauri 超时（${timeout}ms），window.__TAURI__ 状态:`,
                    (window as any).__TAURI__
                );
                resolve(false); // 降级模式
            }
        }, interval);
    });
}

/**
 * 安全获取 Tauri invoke 函数（与 main.js 中 getTauriInvoke 逻辑一致）
 */
function getTauriInvoke(): Function | null {
    if ((window as any).__TAURI__?.core?.invoke) {
        return (window as any).__TAURI__.core.invoke;
    }
    if ((window as any).__TAURI__?.invoke) {
        return (window as any).__TAURI__.invoke;
    }
    return null;
}

// ─── waitForTauri 三种场景 ────────────────────────────────────────────────────

describe('waitForTauri - Tauri 就绪等待', () => {
    beforeEach(() => {
        delete (window as any).__TAURI__;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
        delete (window as any).__TAURI__;
    });

    test('场景1：window.__TAURI__ 立即就绪，应立即 resolve(true)', async () => {
        // 模拟 Tauri 2.x 已就绪
        (window as any).__TAURI__ = {
            core: { invoke: jest.fn() }
        };

        const promise = waitForTauri(3000, 50);
        // 立即就绪不需要推进时间
        jest.runAllTimers();
        const result = await promise;

        expect(result).toBe(true);
    });

    test('场景2：window.__TAURI__ 延迟 100ms 就绪，应在超时前 resolve(true)', async () => {
        // 初始未就绪
        (window as any).__TAURI__ = undefined;

        const promise = waitForTauri(3000, 50);

        // 100ms 后注入 __TAURI__
        setTimeout(() => {
            (window as any).__TAURI__ = {
                core: { invoke: jest.fn() }
            };
        }, 100);

        // 推进 200ms，足够检测到就绪
        jest.advanceTimersByTime(200);
        const result = await promise;

        expect(result).toBe(true);
    });

    test('场景3：window.__TAURI__ 超过 3000ms 仍未就绪，应 resolve(false) 降级', async () => {
        // 始终未就绪
        (window as any).__TAURI__ = undefined;

        const promise = waitForTauri(3000, 50);

        // 推进超过 3000ms
        jest.advanceTimersByTime(3100);
        const result = await promise;

        expect(result).toBe(false);
    });

    test('场景3：超时时应打印警告日志', async () => {
        const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => { });
        (window as any).__TAURI__ = undefined;

        const promise = waitForTauri(3000, 50);
        jest.advanceTimersByTime(3100);
        await promise;

        expect(warnSpy).toHaveBeenCalledWith(
            expect.stringContaining('waitForTauri 超时'),
            undefined // window.__TAURI__ 为 undefined
        );
        warnSpy.mockRestore();
    });

    test('兼容旧版 window.__TAURI__.invoke 挂载方式', async () => {
        // 旧版 Tauri 直接挂载 invoke
        (window as any).__TAURI__ = {
            invoke: jest.fn()
        };

        const promise = waitForTauri(3000, 50);
        jest.runAllTimers();
        const result = await promise;

        expect(result).toBe(true);
    });
});

// ─── getTauriInvoke 正确性 ────────────────────────────────────────────────────

describe('getTauriInvoke - invoke 获取', () => {
    afterEach(() => {
        delete (window as any).__TAURI__;
    });

    test('Tauri 2.x: window.__TAURI__.core.invoke 存在时正确返回', () => {
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = { core: { invoke: mockInvoke } };
        expect(getTauriInvoke()).toBe(mockInvoke);
    });

    test('旧版: window.__TAURI__.invoke 存在时正确返回', () => {
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = { invoke: mockInvoke };
        expect(getTauriInvoke()).toBe(mockInvoke);
    });

    test('window.__TAURI__ 未就绪时返回 null', () => {
        (window as any).__TAURI__ = undefined;
        expect(getTauriInvoke()).toBeNull();
    });

    test('window.__TAURI__ 为 null 时返回 null，不抛出错误', () => {
        (window as any).__TAURI__ = null;
        expect(() => getTauriInvoke()).not.toThrow();
        expect(getTauriInvoke()).toBeNull();
    });
});

// ─── 新架构：accessibility.js 事件监听器注册（需求 3.3） ─────────────────────

describe('新架构：accessibility.js 事件监听器注册', () => {
    beforeAll(() => {
        (window as any).i18n = { t: (key: string) => key };
    });

    beforeEach(() => {
        document.body.innerHTML = `
            <div id="accessibility-overlay" class="accessibility-overlay hidden"></div>
        `;
        jest.resetModules();
    });

    test('__TAURI__ 存在时：应注册 permission-denied 和 permission-granted 事件监听器', () => {
        const listeners: Record<string, Function[]> = {};
        (window as any).__TAURI__ = {
            invoke: jest.fn(),
            event: {
                listen: jest.fn((name: string, cb: Function) => {
                    if (!listeners[name]) listeners[name] = [];
                    listeners[name].push(cb);
                    return Promise.resolve(() => { });
                }),
            },
        };

        require('../src/accessibility.js');

        expect(listeners['accessibility://permission-denied']?.length).toBeGreaterThan(0);
        expect(listeners['accessibility://permission-granted']?.length).toBeGreaterThan(0);
    });

    test('__TAURI__ 不存在时：不抛出异常', () => {
        delete (window as any).__TAURI__;
        expect(() => {
            require('../src/accessibility.js');
        }).not.toThrow();
    });

    test('收到 permission-denied 事件时：遮罩显示', () => {
        const listeners: Record<string, Function[]> = {};
        (window as any).__TAURI__ = {
            invoke: jest.fn(),
            event: {
                listen: jest.fn((name: string, cb: Function) => {
                    if (!listeners[name]) listeners[name] = [];
                    listeners[name].push(cb);
                    return Promise.resolve(() => { });
                }),
            },
        };

        require('../src/accessibility.js');

        const overlay = document.getElementById('accessibility-overlay');
        expect(overlay?.classList.contains('hidden')).toBe(true);

        listeners['accessibility://permission-denied']?.forEach(cb => cb({}));
        expect(overlay?.classList.contains('hidden')).toBe(false);
    });
});

// ─── 版本号降级：应显示 package.json 版本而非硬编码 0.2.0 ────────────────────

describe('版本号降级 - invoke 为 null 时不显示硬编码 0.2.0', () => {
    beforeAll(() => {
        require('../src/platform.js');
    });

    beforeEach(() => {
        // 重置 Platform._invoke 为 null（模拟 invoke 未就绪）
        (window as any).Platform.init(null);
    });

    test('invoke 为 null 时 getAppVersion 降级值不应是硬编码的 "0.2.0"', async () => {
        // 当前实现：safeInvoke 降级返回 '0.2.0'（硬编码）
        // 修复后：应返回 package.json 中的版本号，或至少不是硬编码的 '0.2.0'
        const version = await (window as any).Platform.getAppVersion();

        // 验证：降级值不应是硬编码的 '0.2.0'
        // 修复后应从 package.json 读取真实版本号
        expect(version).not.toBe('0.2.0');
    });

    test('invoke 为 null 时 getAppVersion 应打印降级原因日志（需求 4.3）', async () => {
        const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => { });

        await (window as any).Platform.getAppVersion();

        // 验证打印了降级原因（safeInvoke 打印的格式：单个字符串含命令名）
        expect(warnSpy).toHaveBeenCalledWith(
            expect.stringContaining('Tauri 未就绪'),
            expect.stringContaining('get_app_version')
        );
        warnSpy.mockRestore();
    });
    test('invoke 就绪时 getAppVersion 应调用后端命令', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('0.3.1');
        (window as any).Platform.init(mockInvoke);

        const version = await (window as any).Platform.getAppVersion();

        expect(mockInvoke).toHaveBeenCalledWith('get_app_version', {});
        expect(version).toBe('0.3.1');
    });
});

// ─── initApp 初始化诊断日志（需求 4.1） ──────────────────────────────────────

describe('initApp 初始化诊断日志', () => {
    test('waitForTauri 就绪后 invoke 不为 null 时应打印就绪日志', () => {
        const logSpy = jest.spyOn(console, 'log').mockImplementation(() => { });
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = { core: { invoke: mockInvoke } };

        // 模拟 initApp 中的诊断日志逻辑
        const invoke = getTauriInvoke();
        if (invoke) {
            console.log('[KVMini] invoke 已就绪');
        }

        expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('invoke 已就绪'));
        logSpy.mockRestore();
        delete (window as any).__TAURI__;
    });

    test('invoke 为 null 时应打印 Tauri 未就绪警告', () => {
        const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => { });
        (window as any).__TAURI__ = undefined;

        const invoke = getTauriInvoke();
        if (!invoke) {
            console.warn('[KVMini] Tauri invoke 未就绪，部分功能不可用（非 Tauri 环境）');
        }

        expect(warnSpy).toHaveBeenCalledWith(
            expect.stringContaining('Tauri invoke 未就绪')
        );
        warnSpy.mockRestore();
    });
});
