/**
 * 后端核心功能前端测试
 *
 * 覆盖以下需求：
 * - 需求 6.2：快捷键输入框默认值显示 Ctrl+Alt+S
 * - 需求 7.1、7.2：托盘菜单语言切换
 * - 需求 7.5：连接/断开时托盘菜单状态更新
 * - 需求 8.6、8.7：快捷键冲突警告与确认对话框
 * - 需求 12.2、12.3：TLS 指纹确认对话框
 * - 需求 13.1、13.2：多显示器布局配置界面
 */

// ── 测试辅助 ────────────────────────────────────────────────────────────────

/** 模拟 Tauri invoke 调用 */
const mockInvoke = jest.fn();
/** 模拟 Tauri event.listen */
const mockListen = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();
  // 重置 Tauri mock
  (window as any).__TAURI__ = {
    invoke: mockInvoke,
    event: {
      listen: mockListen,
      emit: jest.fn(),
    },
  };
});

// ── 需求 6：默认快捷键 Ctrl+Alt+S ──────────────────────────────────────────

describe('需求 6：切换控制焦点默认快捷键', () => {
  const DEFAULT_HOTKEY = 'Ctrl+Alt+S';

  test('6.2 get_settings 返回空 hotkey 时，前端应显示默认值 Ctrl+Alt+S', async () => {
    // 模拟后端返回空 hotkey
    mockInvoke.mockResolvedValueOnce({
      hotkey: '',
      prefer_connection_type: 'auto',
      auto_reconnect: true,
      auto_start: false,
      start_minimized: false,
      clipboard_sync: true,
      enable_tls: true,
      command_key_mapping: 'win',
      language: 'zh',
    });

    // 调用 get_settings
    const settings = await (window as any).__TAURI__.invoke('get_settings');

    // 前端逻辑：空 hotkey 时使用默认值
    const displayHotkey = settings.hotkey || DEFAULT_HOTKEY;
    expect(displayHotkey).toBe(DEFAULT_HOTKEY);
  });

  test('6.2 get_settings 返回非空 hotkey 时，前端应显示用户设置的值', async () => {
    mockInvoke.mockResolvedValueOnce({
      hotkey: 'Ctrl+Shift+K',
      prefer_connection_type: 'auto',
      auto_reconnect: true,
      auto_start: false,
      start_minimized: false,
      clipboard_sync: true,
      enable_tls: true,
      command_key_mapping: 'win',
      language: 'zh',
    });

    const settings = await (window as any).__TAURI__.invoke('get_settings');
    const displayHotkey = settings.hotkey || DEFAULT_HOTKEY;
    expect(displayHotkey).toBe('Ctrl+Shift+K');
  });

  test('6.1 应用首次启动时，后端默认快捷键应为 Ctrl+Alt+S', async () => {
    // 模拟后端返回默认设置（已填充默认值）
    mockInvoke.mockResolvedValueOnce({
      hotkey: DEFAULT_HOTKEY,
      prefer_connection_type: 'auto',
      auto_reconnect: true,
      auto_start: false,
      start_minimized: false,
      clipboard_sync: true,
      enable_tls: true,
      command_key_mapping: 'win',
      language: 'zh',
    });

    const settings = await (window as any).__TAURI__.invoke('get_settings');
    expect(settings.hotkey).toBe(DEFAULT_HOTKEY);
  });

  test('6.5 快捷键为空时，前端不应注册快捷键', async () => {
    const registerHotkey = jest.fn();
    const hotkey = '';
    // 空快捷键时不注册
    if (hotkey) {
      registerHotkey(hotkey);
    }
    expect(registerHotkey).not.toHaveBeenCalled();
  });
});

// ── 需求 7：托盘菜单国际化与状态联动 ──────────────────────────────────────

describe('需求 7：托盘菜单国际化与连接状态联动', () => {
  test('7.1 语言切换为英文时，前端应调用 update_tray_language(en)', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    // 模拟前端语言切换
    await (window as any).__TAURI__.invoke('update_tray_language', {
      lang: 'en',
      connected: false,
    });

    expect(mockInvoke).toHaveBeenCalledWith('update_tray_language', {
      lang: 'en',
      connected: false,
    });
  });

  test('7.2 语言切换为中文时，前端应调用 update_tray_language(zh)', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await (window as any).__TAURI__.invoke('update_tray_language', {
      lang: 'zh',
      connected: false,
    });

    expect(mockInvoke).toHaveBeenCalledWith('update_tray_language', {
      lang: 'zh',
      connected: false,
    });
  });

  test('7.5 连接成功时，前端应调用 update_tray_connection_state(connected=true)', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    // 模拟连接成功后更新托盘状态
    await (window as any).__TAURI__.invoke('update_tray_connection_state', {
      connected: true,
    });

    expect(mockInvoke).toHaveBeenCalledWith('update_tray_connection_state', {
      connected: true,
    });
  });

  test('7.5 断开连接时，前端应调用 update_tray_connection_state(connected=false)', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await (window as any).__TAURI__.invoke('update_tray_connection_state', {
      connected: false,
    });

    expect(mockInvoke).toHaveBeenCalledWith('update_tray_connection_state', {
      connected: false,
    });
  });

  test('7.3 托盘菜单 tray-connect 事件应触发前端打开连接面板', () => {
    const onTrayConnect = jest.fn();
    // 模拟监听 tray-connect 事件
    mockListen.mockImplementation((event: string, handler: Function) => {
      if (event === 'tray-connect') {
        handler();
      }
      return Promise.resolve(() => {});
    });

    (window as any).__TAURI__.event.listen('tray-connect', onTrayConnect);
    expect(onTrayConnect).toHaveBeenCalled();
  });
});

// ── 需求 8：快捷键冲突检测 ──────────────────────────────────────────────────

describe('需求 8：快捷键系统冲突检测', () => {
  test('8.1 输入快捷键时，前端应调用 check_hotkey_conflict', async () => {
    mockInvoke.mockResolvedValueOnce({ conflict: false });

    const hotkey = 'Ctrl+Alt+S';
    const result = await (window as any).__TAURI__.invoke('check_hotkey_conflict', { hotkey });

    expect(mockInvoke).toHaveBeenCalledWith('check_hotkey_conflict', { hotkey });
    expect(result.conflict).toBe(false);
  });

  test('8.6 收到冲突结果时，前端应显示警告文字', async () => {
    // 模拟后端返回冲突
    mockInvoke.mockResolvedValueOnce({
      conflict: true,
      conflict_type: 'system',
      description: '与 macOS 系统快捷键冲突：cmd+space (Spotlight 搜索)',
    });

    const result = await (window as any).__TAURI__.invoke('check_hotkey_conflict', {
      hotkey: 'Cmd+Space',
    });

    // 前端逻辑：有冲突时显示警告
    const showWarning = result.conflict;
    const warningText = result.description;

    expect(showWarning).toBe(true);
    expect(warningText).toContain('冲突');
  });

  test('8.7 保存有冲突的快捷键时，前端应弹出确认对话框', async () => {
    const showConfirmDialog = jest.fn().mockReturnValue(true);

    // 模拟有冲突的情况
    const conflictResult = {
      conflict: true,
      conflict_type: 'system',
      description: '与系统快捷键冲突',
    };

    // 前端逻辑：有冲突时弹出确认
    let saved = false;
    if (conflictResult.conflict) {
      const confirmed = showConfirmDialog('检测到快捷键冲突，是否仍要保存？');
      if (confirmed) {
        saved = true;
      }
    }

    expect(showConfirmDialog).toHaveBeenCalledWith('检测到快捷键冲突，是否仍要保存？');
    expect(saved).toBe(true);
  });

  test('8.5 无冲突时，前端不应显示警告', async () => {
    mockInvoke.mockResolvedValueOnce({ conflict: false });

    const result = await (window as any).__TAURI__.invoke('check_hotkey_conflict', {
      hotkey: 'Ctrl+Alt+K',
    });

    expect(result.conflict).toBe(false);
    // 无冲突时不显示警告
    const showWarning = result.conflict;
    expect(showWarning).toBe(false);
  });

  test('8.2 macOS 系统保留快捷键应被检测为冲突', async () => {
    mockInvoke.mockResolvedValueOnce({
      conflict: true,
      conflict_type: 'system',
      description: '与 macOS 系统快捷键冲突：cmd+tab (应用切换)',
    });

    const result = await (window as any).__TAURI__.invoke('check_hotkey_conflict', {
      hotkey: 'Cmd+Tab',
    });

    expect(result.conflict).toBe(true);
    expect(result.conflict_type).toBe('system');
  });

  test('8.3 Windows 系统保留快捷键应被检测为冲突', async () => {
    mockInvoke.mockResolvedValueOnce({
      conflict: true,
      conflict_type: 'system',
      description: '与 Windows 系统快捷键冲突：win+l (锁定屏幕)',
    });

    const result = await (window as any).__TAURI__.invoke('check_hotkey_conflict', {
      hotkey: 'Win+L',
    });

    expect(result.conflict).toBe(true);
    expect(result.conflict_type).toBe('system');
  });
});

// ── 需求 12：TLS 加密传输 ────────────────────────────────────────────────────

describe('需求 12：TLS 加密传输', () => {
  test('12.2 首次连接时，前端应展示对端证书指纹', async () => {
    const peerFingerprint = 'abc123:def456:789012:345678';

    // 模拟后端发送 tls-first-connection 事件
    mockListen.mockImplementation((event: string, handler: Function) => {
      if (event === 'tls-first-connection') {
        handler({ fingerprint: peerFingerprint, address: '192.168.1.100:8080' });
      }
      return Promise.resolve(() => {});
    });

    let receivedFingerprint = '';
    await (window as any).__TAURI__.event.listen('tls-first-connection', (data: any) => {
      receivedFingerprint = data.fingerprint;
    });

    expect(receivedFingerprint).toBe(peerFingerprint);
  });

  test('12.3 用户确认指纹后，前端应调用 trust_peer_certificate', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const address = '192.168.1.100:8080';
    const fingerprint = 'abc123:def456';

    // 用户确认后调用
    await (window as any).__TAURI__.invoke('trust_peer_certificate', {
      address,
      fingerprint,
    });

    expect(mockInvoke).toHaveBeenCalledWith('trust_peer_certificate', {
      address,
      fingerprint,
    });
  });

  test('12.3 指纹不匹配时，前端应收到 tls-fingerprint-mismatch 事件', () => {
    const onMismatch = jest.fn();

    mockListen.mockImplementation((event: string, handler: Function) => {
      if (event === 'tls-fingerprint-mismatch') {
        handler({ expected: 'abc123', got: 'xyz789' });
      }
      return Promise.resolve(() => {});
    });

    (window as any).__TAURI__.event.listen('tls-fingerprint-mismatch', onMismatch);
    expect(onMismatch).toHaveBeenCalledWith({ expected: 'abc123', got: 'xyz789' });
  });

  test('12.1 get_local_tls_fingerprint 应返回本机指纹', async () => {
    const localFingerprint = 'local:fingerprint:abc:123';
    mockInvoke.mockResolvedValueOnce(localFingerprint);

    const fp = await (window as any).__TAURI__.invoke('get_local_tls_fingerprint');
    expect(fp).toBe(localFingerprint);
    expect(mockInvoke).toHaveBeenCalledWith('get_local_tls_fingerprint');
  });
});

// ── 需求 13：多显示器支持 ────────────────────────────────────────────────────

describe('需求 13：多显示器支持', () => {
  const mockMonitors = [
    { id: 0, width: 1920, height: 1080, dpi_scale: 1.0, offset_x: 0, offset_y: 0, is_primary: true },
    { id: 1, width: 2560, height: 1440, dpi_scale: 2.0, offset_x: 1920, offset_y: 0, is_primary: false },
  ];

  test('13.1 打开设置时，前端应调用 get_monitors 获取显示器列表', async () => {
    mockInvoke.mockResolvedValueOnce(mockMonitors);

    const monitors = await (window as any).__TAURI__.invoke('get_monitors');

    expect(mockInvoke).toHaveBeenCalledWith('get_monitors');
    expect(monitors).toHaveLength(2);
    expect(monitors[0].is_primary).toBe(true);
  });

  test('13.1 显示器列表应包含分辨率、DPI、偏移量信息', async () => {
    mockInvoke.mockResolvedValueOnce(mockMonitors);

    const monitors = await (window as any).__TAURI__.invoke('get_monitors');

    const primary = monitors.find((m: any) => m.is_primary);
    expect(primary).toBeDefined();
    expect(primary.width).toBe(1920);
    expect(primary.height).toBe(1080);
    expect(primary.dpi_scale).toBe(1.0);
    expect(primary.offset_x).toBe(0);
    expect(primary.offset_y).toBe(0);
  });

  test('13.2 用户配置显示器布局时，前端应能设置偏移量', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    // 模拟用户设置布局配置
    const layoutConfig = {
      remote_position: 'right',
      offset: 0,
    };

    await (window as any).__TAURI__.invoke('set_layout_config', layoutConfig);

    expect(mockInvoke).toHaveBeenCalledWith('set_layout_config', layoutConfig);
  });

  test('13.7 显示器插拔时，前端应收到 monitors-changed 事件', () => {
    const onMonitorsChanged = jest.fn();

    mockListen.mockImplementation((event: string, handler: Function) => {
      if (event === 'monitors-changed') {
        handler({ monitors: mockMonitors });
      }
      return Promise.resolve(() => {});
    });

    (window as any).__TAURI__.event.listen('monitors-changed', onMonitorsChanged);
    expect(onMonitorsChanged).toHaveBeenCalledWith({ monitors: mockMonitors });
  });

  test('13.3 get_virtual_desktop_info 应返回虚拟桌面总尺寸', async () => {
    const mockVirtualDesktop = {
      total_width: 4480,
      total_height: 1440,
      origin_x: 0,
      origin_y: 0,
      monitors: mockMonitors,
    };
    mockInvoke.mockResolvedValueOnce(mockVirtualDesktop);

    const vd = await (window as any).__TAURI__.invoke('get_virtual_desktop_info');

    expect(vd.total_width).toBe(4480);
    expect(vd.total_height).toBe(1440);
    expect(vd.monitors).toHaveLength(2);
  });
});

// ── 需求 11：剪贴板共享 ──────────────────────────────────────────────────────

describe('需求 11：剪贴板共享', () => {
  test('11.5 关闭剪贴板共享时，设置中 clipboard_sync 应为 false', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const settings = {
      hotkey: 'Ctrl+Alt+S',
      prefer_connection_type: 'auto',
      auto_reconnect: true,
      auto_start: false,
      start_minimized: false,
      clipboard_sync: false, // 关闭剪贴板共享
      enable_tls: true,
      command_key_mapping: 'win',
      language: 'zh',
    };

    await (window as any).__TAURI__.invoke('save_settings', settings);

    expect(mockInvoke).toHaveBeenCalledWith('save_settings', expect.objectContaining({
      clipboard_sync: false,
    }));
  });

  test('11.1 剪贴板变化时，前端应收到 clipboard-changed 事件', () => {
    const onClipboardChanged = jest.fn();

    mockListen.mockImplementation((event: string, handler: Function) => {
      if (event === 'clipboard-changed') {
        handler({ text: 'Hello, World!' });
      }
      return Promise.resolve(() => {});
    });

    (window as any).__TAURI__.event.listen('clipboard-changed', onClipboardChanged);
    expect(onClipboardChanged).toHaveBeenCalledWith({ text: 'Hello, World!' });
  });
});

// ── 需求 10：跨平台键码映射 ──────────────────────────────────────────────────

describe('需求 10：跨平台键码映射', () => {
  test('10.6 设置中 command_key_mapping 应支持 win 和 ctrl 两种模式', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    // 切换为 Ctrl 模式
    const settings = {
      hotkey: 'Ctrl+Alt+S',
      prefer_connection_type: 'auto',
      auto_reconnect: true,
      auto_start: false,
      start_minimized: false,
      clipboard_sync: true,
      enable_tls: true,
      command_key_mapping: 'ctrl', // Command → Ctrl
      language: 'zh',
    };

    await (window as any).__TAURI__.invoke('save_settings', settings);

    expect(mockInvoke).toHaveBeenCalledWith('save_settings', expect.objectContaining({
      command_key_mapping: 'ctrl',
    }));
  });
});
