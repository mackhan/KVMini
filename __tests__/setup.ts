// 测试环境设置
import '@testing-library/jest-dom';

// 全局类型定义
declare global {
  interface Window {
    __TAURI__?: {
      invoke: jest.Mock;
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
  window.__TAURI__ = {
    invoke: jest.fn(),
    event: {
      listen: jest.fn(),
      emit: jest.fn(),
    },
  };
}

// 测试生命周期
beforeEach(() => {
  jest.clearAllMocks();
  console.log('Starting new test...');
});

afterEach(() => {
  console.log('Test completed');
});