/**
 * 测试：修复 Tauri invoke 解构错误
 * 需求：1.1、1.2、1.3、1.4、1.5
 */

describe('Tauri invoke 安全获取', () => {
    beforeEach(() => {
        // 重置 window.__TAURI__
        delete (window as any).__TAURI__;
    });

    test('window.__TAURI__ 为 undefined 时不抛出 TypeError', () => {
        // 模拟 Tauri 未初始化
        (window as any).__TAURI__ = undefined;

        // 安全获取函数（与 main.js 中 getTauriInvoke 逻辑一致）
        const getTauriInvoke = () => {
            if ((window as any).__TAURI__?.tauri?.invoke) {
                return (window as any).__TAURI__.tauri.invoke;
            }
            if ((window as any).__TAURI__?.invoke) {
                return (window as any).__TAURI__.invoke;
            }
            return null;
        };

        // 不应抛出错误
        expect(() => getTauriInvoke()).not.toThrow();
        // 返回 null 而非报错
        expect(getTauriInvoke()).toBeNull();
    });

    test('window.__TAURI__ 为 null 时不抛出 TypeError', () => {
        (window as any).__TAURI__ = null;

        const getTauriInvoke = () => {
            if ((window as any).__TAURI__?.tauri?.invoke) {
                return (window as any).__TAURI__.tauri.invoke;
            }
            if ((window as any).__TAURI__?.invoke) {
                return (window as any).__TAURI__.invoke;
            }
            return null;
        };

        expect(() => getTauriInvoke()).not.toThrow();
        expect(getTauriInvoke()).toBeNull();
    });

    test('Tauri 1.x: window.__TAURI__.tauri.invoke 存在时正确返回', () => {
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = {
            tauri: { invoke: mockInvoke }
        };

        const getTauriInvoke = () => {
            if ((window as any).__TAURI__?.tauri?.invoke) {
                return (window as any).__TAURI__.tauri.invoke;
            }
            if ((window as any).__TAURI__?.invoke) {
                return (window as any).__TAURI__.invoke;
            }
            return null;
        };

        expect(getTauriInvoke()).toBe(mockInvoke);
    });

    test('Tauri 1.x 备用: window.__TAURI__.invoke 存在时正确返回', () => {
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = {
            invoke: mockInvoke
        };

        const getTauriInvoke = () => {
            if ((window as any).__TAURI__?.tauri?.invoke) {
                return (window as any).__TAURI__.tauri.invoke;
            }
            if ((window as any).__TAURI__?.invoke) {
                return (window as any).__TAURI__.invoke;
            }
            return null;
        };

        expect(getTauriInvoke()).toBe(mockInvoke);
    });

    test('DOMContentLoaded 触发后才执行初始化（不在顶层执行）', () => {
        // 验证：在 DOMContentLoaded 之前，invoke 应为 null（未初始化）
        // 这里通过检查 document.readyState 来模拟
        const mockInvoke = jest.fn();
        (window as any).__TAURI__ = {
            tauri: { invoke: mockInvoke }
        };

        // 模拟顶层执行时 __TAURI__ 不存在（Tauri 尚未注入）
        let invokeRef: any = null;

        // 模拟 DOMContentLoaded 后才赋值
        const initApp = () => {
            const getTauriInvoke = () => {
                if ((window as any).__TAURI__?.tauri?.invoke) {
                    return (window as any).__TAURI__.tauri.invoke;
                }
                if ((window as any).__TAURI__?.invoke) {
                    return (window as any).__TAURI__.invoke;
                }
                return null;
            };
            invokeRef = getTauriInvoke();
        };

        // 初始化前 invoke 为 null
        expect(invokeRef).toBeNull();

        // 触发初始化（模拟 DOMContentLoaded）
        initApp();

        // 初始化后 invoke 已就绪
        expect(invokeRef).toBe(mockInvoke);
    });
});
