// 测试环境设置
import '@testing-library/jest-dom';

// 全局类型定义
declare global {
  interface Window {
    __TAURI__?: {
      invoke: jest.Mock;
      core?: {
        invoke: jest.Mock;
      };
      event: {
        listen: jest.Mock;
        emit: jest.Mock;
      };
    };
  }
}

// 全局测试配置
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

// 模拟Tauri API
if (typeof window !== 'undefined') {
  // v0.1.40: invoke 默认返回 resolved Promise，避免 main.js 顶层的 diagLog/initVersion
  // 等对 invoke 返回值调用 .catch/.then 时报 undefined
  const invokeMock: jest.Mock = jest.fn().mockResolvedValue(undefined);
  window.__TAURI__ = {
    invoke: invokeMock,
    // 新版 Tauri 2.x 走 __TAURI__.core.invoke，main.js 做了兼容
    core: { invoke: invokeMock } as any,
    event: {
      listen: jest.fn().mockResolvedValue(() => {}),
      emit: jest.fn().mockResolvedValue(undefined),
    },
  } as any;
}

// 测试生命周期
beforeEach(() => {
  jest.clearAllMocks();
  console.log('Starting new test...');
});

afterEach(() => {
  console.log('Test completed');
});