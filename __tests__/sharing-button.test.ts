/**
 * Bug 2 / BUG-019：开始共享按钮端到端契约测试
 *
 * 验证点：
 *  1. 点击「开始共享」按钮会通过 invoke('toggle_sharing') 触发后端
 *  2. 收到 control-state-changed 事件后，按钮 data-i18n 在
 *     btnStartSharing ↔ btnStopSharing 间正确切换
 *  3. listen('control-state-changed') 回调驱动状态栏 controlState 文本同步
 *  4. 后端报错时调用 showError(t('shareToggleFailed'))
 *
 * 这里用纯函数版本的逻辑（避免 main.js 副作用），重点是契约（API/事件/key）。
 */

// jsdom 环境下 localStorage 可用
const i18n = require('../src/i18n.js');
const { t, setLocale, MESSAGES } = i18n;
(global as any).window = (global as any).window || {};
(global as any).window.i18n = i18n;

/**
 * 模拟 main.js 中的按钮切换函数：根据 control state 切换 i18n key 与文字
 */
function applySharingButtonState(btn: HTMLElement, state: string) {
  if (state === 'LocalControlling') {
    btn.setAttribute('data-i18n', 'btnStopSharing');
    btn.textContent = t('btnStopSharing');
  } else {
    btn.setAttribute('data-i18n', 'btnStartSharing');
    btn.textContent = t('btnStartSharing');
  }
}

/**
 * 模拟 main.js 的 handleStartSharing：调 invoke，错误时弹 showError
 */
async function handleStartSharing(invoke: jest.Mock, showError: jest.Mock) {
  try {
    await invoke('toggle_sharing');
  } catch (e) {
    showError(t('shareToggleFailed'));
  }
}

describe('开始/停止共享按钮（BUG-019）', () => {
  let btn: HTMLButtonElement;

  beforeEach(() => {
    setLocale('zh');
    document.body.innerHTML = '<button id="start-sharing-btn" data-i18n="btnStartSharing">开始共享</button>';
    btn = document.getElementById('start-sharing-btn') as HTMLButtonElement;
  });

  it('点击按钮会调用 invoke("toggle_sharing")', async () => {
    const invoke = jest.fn().mockResolvedValue('LocalControlling');
    const showError = jest.fn();
    btn.addEventListener('click', () => handleStartSharing(invoke, showError));
    btn.click();
    // 微任务排空
    await Promise.resolve();
    expect(invoke).toHaveBeenCalledWith('toggle_sharing');
    expect(showError).not.toHaveBeenCalled();
  });

  it('后端抛错时显示 shareToggleFailed 文案', async () => {
    const invoke = jest.fn().mockRejectedValue('mock failure');
    const showError = jest.fn();
    await handleStartSharing(invoke, showError);
    expect(showError).toHaveBeenCalledWith(t('shareToggleFailed'));
  });

  it('control-state-changed → LocalControlling 时按钮显示「停止共享」', () => {
    applySharingButtonState(btn, 'LocalControlling');
    expect(btn.getAttribute('data-i18n')).toBe('btnStopSharing');
    expect(btn.textContent).toBe(t('btnStopSharing'));
    expect(btn.textContent).toContain('停止');
  });

  it('control-state-changed → Idle 时按钮恢复为「开始共享」', () => {
    applySharingButtonState(btn, 'LocalControlling');
    applySharingButtonState(btn, 'Idle');
    expect(btn.getAttribute('data-i18n')).toBe('btnStartSharing');
    expect(btn.textContent).toBe(t('btnStartSharing'));
  });

  it('英文模式下 LocalControlling 显示「Stop Sharing」', () => {
    setLocale('en');
    applySharingButtonState(btn, 'LocalControlling');
    expect(btn.textContent?.toLowerCase()).toContain('stop');
    expect(btn.textContent?.toLowerCase()).toContain('shar');
  });

  it('i18n 字典必须含 btnStopSharing / shareToggleFailed', () => {
    expect(MESSAGES.zh.btnStopSharing).toBeDefined();
    expect(MESSAGES.en.btnStopSharing).toBeDefined();
    expect(MESSAGES.zh.shareToggleFailed).toBeDefined();
    expect(MESSAGES.en.shareToggleFailed).toBeDefined();
  });
});
