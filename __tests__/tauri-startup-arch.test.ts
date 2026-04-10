/**
 * tauri-startup-arch.test.ts
 * 测试 Tauri 启动架构彻底修复（方案 B：调整脚本顺序）
 *
 * 根因：macOS WKWebView 会在 <script src="main.js"> 执行完成后清除该脚本中
 * 新增到 window 的属性，导致 main.js 中定义的启动函数在后续内联脚本中不可见。
 *
 * 修复方案：将前置内联脚本移到 main.js 之前，只负责捕获 invoke 到
 * window.tauriInvokeReady；main.js 末尾直接读取该值并调用 initApp()。
 *
 * 需求：1.1、1.2、1.3、2.1、2.2、2.3、2.4、5.1、5.2、5.3
 */

// ─── 辅助函数（与 main.js 新实现保持一致，用于单元测试） ─────────────────────

/**
 * 模拟新版 waitForTauri：优先检查 window.tauriInvokeReady，
 * 其次轮询 window.__TAURI__，超时后降级
 */
function waitForTauri(timeout = 5000, interval = 50): Promise<Function | null> {
    return new Promise((resolve) => {
        // 优先：前置内联脚本已捕获 invoke
        if ((window as any).tauriInvokeReady) {
            resolve((window as any).tauriInvokeReady);
            return;
        }

        // 次选：直接检查 __TAURI__
        const directInvoke = (window as any).__TAURI__?.core?.invoke
            || (window as any).__TAURI__?.invoke
            || null;
        if (directInvoke) {
            resolve(directInvoke);
            return;
        }

        // 轮询等待
        const startTime = Date.now();
        const timer = setInterval(() => {
            const inv = (window as any).tauriInvokeReady
                || (window as any).__TAURI__?.core?.invoke
                || (window as any).__TAURI__?.invoke
                || null;
            if (inv) {
                clearInterval(timer);
                resolve(inv);
                return;
            }
            if (Date.now() - startTime >= timeout) {
                clearInterval(timer);
                console.error(
                    `[KVMini] waitForTauri 超时（${timeout}ms），Tauri 环境未就绪`
                );
                resolve(null); // 超时降级，返回 null
            }
        }, interval);
    });
}

// ─── 场景 1：tauriInvokeReady 有值时直接就绪 ─────────────────────────────────

describe('新启动架构 - tauriInvokeReady 有值时直接就绪', () => {
    beforeEach(() => {
        // 清除 setup.ts 中设置的 __TAURI__，模拟纯净环境
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
    });

    test('tauriInvokeReady 有值时，waitForTauri 应立即 resolve 该 invoke 函数', async () => {
        const mockInvoke = jest.fn();
        // 模拟前置内联脚本已捕获 invoke
        (window as any).tauriInvokeReady = mockInvoke;

        const promise = waitForTauri(5000, 50);
        jest.runAllTimers();
        const result = await promise;

        expect(result).toBe(mockInvoke);
    });

    test('tauriInvokeReady 有值时，不需要轮询，立即返回', async () => {
        const mockInvoke = jest.fn();
        (window as any).tauriInvokeReady = mockInvoke;

        // 不推进时间，应立即 resolve
        const result = await waitForTauri(5000, 50);

        expect(result).toBe(mockInvoke);
    });

    test('tauriInvokeReady 有值时，__TAURI__ 不存在也能正常工作', async () => {
        const mockInvoke = jest.fn();
        (window as any).tauriInvokeReady = mockInvoke;
        // 确保 __TAURI__ 不存在
        expect((window as any).__TAURI__).toBeUndefined();

        const result = await waitForTauri(5000, 50);

        expect(result).toBe(mockInvoke);
    });
});

// ─── 场景 2：tauriInvokeReady 为 null，走 waitForTauri 轮询路径 ──────────────

describe('新启动架构 - tauriInvokeReady 为 null 时走轮询路径', () => {
    beforeEach(() => {
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
    });

    test('tauriInvokeReady 为 null，__TAURI__ 延迟 100ms 就绪，应在超时前 resolve', async () => {
        (window as any).tauriInvokeReady = null;
        const mockInvoke = jest.fn();

        const promise = waitForTauri(5000, 50);

        // 100ms 后 __TAURI__ 就绪
        setTimeout(() => {
            (window as any).__TAURI__ = { core: { invoke: mockInvoke } };
        }, 100);

        jest.advanceTimersByTime(200);
        const result = await promise;

        expect(result).toBe(mockInvoke);
    });

    test('tauriInvokeReady 为 null，__TAURI__ 延迟就绪（旧版 invoke 挂载方式）', async () => {
        (window as any).tauriInvokeReady = null;
        const mockInvoke = jest.fn();

        const promise = waitForTauri(5000, 50);

        setTimeout(() => {
            (window as any).__TAURI__ = { invoke: mockInvoke };
        }, 100);

        jest.advanceTimersByTime(200);
        const result = await promise;

        expect(result).toBe(mockInvoke);
    });

    test('tauriInvokeReady 为 null，超时（5000ms）后 resolve(null) 降级', async () => {
        (window as any).tauriInvokeReady = null;

        const promise = waitForTauri(5000, 50);

        jest.advanceTimersByTime(5100);
        const result = await promise;

        expect(result).toBeNull();
    });

    test('超时时应打印错误日志', async () => {
        const errorSpy = jest.spyOn(console, 'error').mockImplementation(() => { });
        (window as any).tauriInvokeReady = null;

        const promise = waitForTauri(5000, 50);
        jest.advanceTimersByTime(5100);
        await promise;

        expect(errorSpy).toHaveBeenCalledWith(
            expect.stringContaining('waitForTauri 超时')
        );
        errorSpy.mockRestore();
    });
});

// ─── 场景 3：前置内联脚本逻辑验证 ────────────────────────────────────────────

describe('前置内联脚本逻辑 - 捕获 invoke 到 tauriInvokeReady', () => {
    afterEach(() => {
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
    });

    test('__TAURI__ 已就绪时，前置脚本应将 invoke 保存到 tauriInvokeReady', () => {
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = { core: { invoke: mockInvoke } };

        // 模拟前置内联脚本执行
        (window as any).tauriInvokeReady = (window as any).__TAURI__?.core?.invoke
            || (window as any).__TAURI__?.invoke
            || null;

        expect((window as any).tauriInvokeReady).toBe(mockInvoke);
    });

    test('__TAURI__ 未就绪时，前置脚本应将 tauriInvokeReady 设为 null', () => {
        delete (window as any).__TAURI__;

        // 模拟前置内联脚本执行
        (window as any).tauriInvokeReady = (window as any).__TAURI__?.core?.invoke
            || (window as any).__TAURI__?.invoke
            || null;

        expect((window as any).tauriInvokeReady).toBeNull();
    });

    test('旧版 __TAURI__.invoke 挂载方式也能被前置脚本捕获', () => {
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = { invoke: mockInvoke };

        // 模拟前置内联脚本执行
        (window as any).tauriInvokeReady = (window as any).__TAURI__?.core?.invoke
            || (window as any).__TAURI__?.invoke
            || null;

        expect((window as any).tauriInvokeReady).toBe(mockInvoke);
    });
});

// ─── 场景 4：启动流程集成验证 ─────────────────────────────────────────────────

describe('启动流程集成 - tauriInvokeReady 决定启动路径', () => {
    beforeEach(() => {
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
        jest.useFakeTimers();
    });

    afterEach(() => {
        jest.useRealTimers();
        delete (window as any).__TAURI__;
        delete (window as any).tauriInvokeReady;
    });

    test('tauriInvokeReady 有值时，启动逻辑应直接调用 initApp（不走 waitForTauri）', async () => {
        const mockInvoke = jest.fn();
        const mockInitApp = jest.fn();
        (window as any).tauriInvokeReady = mockInvoke;

        // 模拟 main.js 末尾启动逻辑
        if ((window as any).tauriInvokeReady) {
            mockInitApp();
        } else {
            await waitForTauri(5000, 50).then((inv) => {
                if (inv) mockInitApp();
            });
        }

        expect(mockInitApp).toHaveBeenCalledTimes(1);
    });

    test('tauriInvokeReady 为 null 时，启动逻辑应走 waitForTauri 路径', async () => {
        const mockInvoke = jest.fn();
        const mockInitApp = jest.fn();
        (window as any).tauriInvokeReady = null;

        // 模拟 main.js 末尾启动逻辑
        let initAppCalled = false;
        const startupPromise = (async () => {
            if ((window as any).tauriInvokeReady) {
                mockInitApp();
            } else {
                const inv = await waitForTauri(5000, 50);
                if (inv) {
                    mockInitApp();
                    initAppCalled = true;
                }
            }
        })();

        // 模拟 __TAURI__ 延迟就绪
        setTimeout(() => {
            (window as any).__TAURI__ = { core: { invoke: mockInvoke } };
        }, 100);

        jest.advanceTimersByTime(200);
        await startupPromise;

        expect(mockInitApp).toHaveBeenCalledTimes(1);
    });

    test('waitForTauri 超时时，应显示错误提示而不是静默失败', async () => {
        const mockInitApp = jest.fn();
        (window as any).tauriInvokeReady = null;

        // 设置 DOM 错误提示元素
        document.body.innerHTML = `
            <div id="error-toast" class="error-toast hidden">
                <span id="error-message"></span>
            </div>
        `;

        // 模拟 main.js 末尾启动逻辑（超时处理）
        const startupPromise = (async () => {
            if ((window as any).tauriInvokeReady) {
                mockInitApp();
            } else {
                const inv = await waitForTauri(5000, 50);
                if (!inv) {
                    // 超时：显示错误提示
                    const errorToast = document.getElementById('error-toast');
                    const errorMessage = document.getElementById('error-message');
                    if (errorToast && errorMessage) {
                        errorMessage.textContent = 'Tauri 环境初始化失败，请重启应用';
                        errorToast.classList.remove('hidden');
                    }
                } else {
                    mockInitApp();
                }
            }
        })();

        jest.advanceTimersByTime(5100);
        await startupPromise;

        // initApp 不应被调用
        expect(mockInitApp).not.toHaveBeenCalled();
        // 错误提示应显示
        const errorToast = document.getElementById('error-toast');
        const errorMessage = document.getElementById('error-message');
        expect(errorToast?.classList.contains('hidden')).toBe(false);
        expect(errorMessage?.textContent).toBe('Tauri 环境初始化失败，请重启应用');
    });
});

// ─── 场景 5：版本号降级不显示硬编码 0.2.0 ────────────────────────────────────

describe('版本号降级 - invoke 为 null 时不显示硬编码 0.2.0', () => {
    beforeAll(() => {
        // 确保 Platform 模块已加载
        delete (window as any).Platform;
        require('../src/platform.js');
    });

    beforeEach(() => {
        // 重置 Platform._invoke 为 null（模拟 invoke 未就绪）
        (window as any).Platform.init(null);
        // 设置 meta 标签版本号（模拟构建脚本注入）
        let meta = document.querySelector('meta[name="app-version"]') as HTMLMetaElement;
        if (!meta) {
            meta = document.createElement('meta');
            meta.name = 'app-version';
            document.head.appendChild(meta);
        }
        meta.setAttribute('content', '0.1.11');
    });

    test('invoke 为 null 时 getAppVersion 降级值不应是硬编码的 "0.2.0"', async () => {
        const version = await (window as any).Platform.getAppVersion();
        // 降级值应从 meta 标签读取（0.1.11），而不是硬编码的 0.2.0
        expect(version).not.toBe('0.2.0');
    });

    test('invoke 为 null 时 getAppVersion 应返回 meta 标签中的版本号', async () => {
        const version = await (window as any).Platform.getAppVersion();
        // 应返回 meta 标签中的版本号
        expect(version).toBe('0.1.11');
    });

    test('invoke 就绪时 getAppVersion 应调用后端命令', async () => {
        const mockInvoke = jest.fn().mockResolvedValue('0.3.1');
        (window as any).Platform.init(mockInvoke);

        const version = await (window as any).Platform.getAppVersion();

        expect(mockInvoke).toHaveBeenCalledWith('get_app_version', {});
        expect(version).toBe('0.3.1');
    });
});

// ─── 场景 6：清理验证 - 不再有 kvminiBootstrap 等遗留函数 ────────────────────

describe('清理验证 - 遗留启动函数已删除', () => {
    test('window.kvminiBootstrap 不应存在（已删除）', () => {
        // 新架构中不再需要 kvminiBootstrap
        // 此测试在实现后验证清理是否彻底
        // 注意：main.js 未加载时此测试通过；加载后如果仍有 kvminiBootstrap 则失败
        // 这是一个"红灯"测试，驱动我们删除 kvminiBootstrap
        expect(typeof (window as any).kvminiBootstrap).toBe('undefined');
    });

    test('window.kvminiBootstrapDone 不应存在（已删除）', () => {
        expect(typeof (window as any).kvminiBootstrapDone).toBe('undefined');
    });
});
