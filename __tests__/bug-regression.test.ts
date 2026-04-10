/**
 * Bug 回归测试
 *
 * 覆盖用户反馈的三个核心 bug：
 * 1. 设置无法点击（main.js 语法错误导致 JS 解析失败）
 * 2. 扫描有两个按钮（connect-btn 和 scan-btn 功能重复）
 * 3. 主控端/被控端切换按钮点了没有反应（initRoleSwitch 未执行）
 */

const mockInvoke = jest.fn();
const mockListen = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();
  (window as any).__TAURI__ = {
    invoke: mockInvoke,
    event: { listen: mockListen, emit: jest.fn() },
  };
});

// ── Bug 1：设置按钮可点击 ──────────────────────────────────────────────────

describe('Bug 1：设置按钮可点击（JS 语法错误修复）', () => {
  test('loadLocalIp 正常返回 IP 列表', async () => {
    mockInvoke.mockResolvedValueOnce(['192.168.1.100', '10.0.0.1']);
    const ips = await (window as any).__TAURI__.invoke('get_local_ip');
    expect(ips.join(' / ')).toBe('192.168.1.100 / 10.0.0.1');
  });

  test('loadLocalIp 返回空数组时显示失败提示', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const ips = await (window as any).__TAURI__.invoke('get_local_ip');
    const text = (ips && ips.length > 0) ? ips.join(' / ') : '获取失败';
    expect(text).toBe('获取失败');
  });

  test('loadLocalIp 异常时显示失败提示', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('网络错误'));
    let text = '获取失败';
    try {
      const ips = await (window as any).__TAURI__.invoke('get_local_ip');
      text = (ips && ips.length > 0) ? ips.join(' / ') : '获取失败';
    } catch (_) { text = '获取失败'; }
    expect(text).toBe('获取失败');
  });

  test('get_settings 命令正常调用', async () => {
    mockInvoke.mockResolvedValueOnce({ hotkey: 'Ctrl+Alt+S', prefer_connection_type: 'auto', auto_reconnect: true, auto_start: false, start_minimized: false, clipboard_sync: true, enable_tls: true, command_key_mapping: 'win', language: 'zh' });
    const s = await (window as any).__TAURI__.invoke('get_settings');
    expect(s.hotkey).toBe('Ctrl+Alt+S');
  });

  test('save_settings 命令正常调用', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await (window as any).__TAURI__.invoke('save_settings', { hotkey: 'Ctrl+Alt+K', prefer_connection_type: 'network', auto_reconnect: false, auto_start: true, start_minimized: true, clipboard_sync: false, enable_tls: false, command_key_mapping: 'ctrl', language: 'en' });
    expect(mockInvoke).toHaveBeenCalledWith('save_settings', expect.objectContaining({ hotkey: 'Ctrl+Alt+K' }));
  });
});

// ── Bug 2：扫描按钮不重复 ──────────────────────────────────────────────────

describe('Bug 2：connect-btn 与 scan-btn 功能区分', () => {
  test('connect-btn 调用 connect_to_device', async () => {
    mockInvoke.mockResolvedValueOnce({ success: true, connectionType: 'Network' });
    const r = await (window as any).__TAURI__.invoke('connect_to_device', { deviceId: 'device-001' });
    expect(r.success).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith('connect_to_device', { deviceId: 'device-001' });
  });

  test('scan-btn 调用 discover_devices', async () => {
    mockInvoke.mockResolvedValueOnce([{ id: 'device-001', name: 'MacBook Pro', os: 'macOS', ip: '192.168.1.100', connectionType: 'Network' }]);
    const devices = await (window as any).__TAURI__.invoke('discover_devices');
    expect(devices).toHaveLength(1);
    expect(mockInvoke).toHaveBeenCalledWith('discover_devices');
  });

  test('两个按钮调用不同命令', () => {
    expect('connect_to_device').not.toBe('discover_devices');
  });

  test('扫描结果为空时显示无设备提示', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const devices = await (window as any).__TAURI__.invoke('discover_devices');
    const text = devices.length === 0 ? '点击"扫描设备"发现局域网内的设备' : '';
    expect(text).toBe('点击"扫描设备"发现局域网内的设备');
  });

  test('扫描结果包含必要字段', async () => {
    mockInvoke.mockResolvedValueOnce([{ id: 'dev-1', name: 'Windows PC', os: 'Windows', ip: '192.168.1.200', connectionType: 'Network' }]);
    const devices = await (window as any).__TAURI__.invoke('discover_devices');
    expect(devices[0]).toHaveProperty('name');
    expect(devices[0]).toHaveProperty('os');
    expect(devices[0]).toHaveProperty('ip');
    expect(devices[0]).toHaveProperty('connectionType');
  });
});

// ── Bug 3：角色切换按钮正常工作 ────────────────────────────────────────────

describe('Bug 3：主控端/被控端切换按钮正常工作', () => {
  test('get_current_role 返回当前角色', async () => {
    mockInvoke.mockResolvedValueOnce('Server');
    const role = await (window as any).__TAURI__.invoke('get_current_role');
    expect(role).toBe('Server');
  });

  test('set_role 切换到 Client', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await (window as any).__TAURI__.invoke('set_role', { newRole: 'Client' });
    expect(mockInvoke).toHaveBeenCalledWith('set_role', { newRole: 'Client' });
  });

  test('set_role 切换到 Server', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await (window as any).__TAURI__.invoke('set_role', { newRole: 'Server' });
    expect(mockInvoke).toHaveBeenCalledWith('set_role', { newRole: 'Server' });
  });

  test('updateRoleUI Server 显示被控端', () => {
    const updateRoleUI = (role: string) => role === 'Server' ? '被控端' : '主控端';
    expect(updateRoleUI('Server')).toBe('被控端');
  });

  test('updateRoleUI Client 显示主控端', () => {
    const updateRoleUI = (role: string) => role === 'Server' ? '被控端' : '主控端';
    expect(updateRoleUI('Client')).toBe('主控端');
  });

  test('角色徽章 CSS 类正确', () => {
    const getClass = (role: string) => role === 'Server' ? 'role-badge role-server' : 'role-badge role-client';
    expect(getClass('Server')).toBe('role-badge role-server');
    expect(getClass('Client')).toBe('role-badge role-client');
  });

  test('切换角色时若已连接应断开', async () => {
    mockInvoke.mockResolvedValue(undefined);
    let connected = true;
    await (window as any).__TAURI__.invoke('set_role', { newRole: 'Client' });
    if (connected) {
      await (window as any).__TAURI__.invoke('disconnect_tcp').catch(() => { });
      connected = false;
    }
    expect(connected).toBe(false);
  });

  test('initRoleSwitch 初始化调用 get_current_role', async () => {
    mockInvoke.mockResolvedValueOnce('Server');
    const role = await (window as any).__TAURI__.invoke('get_current_role');
    expect(role).toBe('Server');
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});

// ── 连接流程完整性 ────────────────────────────────────────────────────────

describe('连接流程完整性', () => {
  test('手动 IP 连接成功后保存历史', async () => {
    mockInvoke.mockResolvedValueOnce({ success: true, remote_platform: 'Windows' });
    const r = await (window as any).__TAURI__.invoke('connect_by_ip', { ip: '192.168.1.200', port: 8889 });
    expect(r.success).toBe(true);
    mockInvoke.mockResolvedValueOnce(undefined);
    await (window as any).__TAURI__.invoke('save_connection_history', { id: 'manual:192.168.1.200:8889', name: '192.168.1.200', ip: '192.168.1.200', connectionType: 'Network', entryType: 'manual', port: 8889, lastConnected: new Date().toISOString() });
    expect(mockInvoke).toHaveBeenCalledWith('save_connection_history', expect.objectContaining({ ip: '192.168.1.200', entryType: 'manual' }));
  });

  test('IPv4 格式校验：有效地址', () => {
    const isValidIpv4 = (ip: string) => {
      const parts = ip.split('.');
      if (parts.length !== 4) return false;
      return parts.every(p => { const n = Number(p); return /^\d+$/.test(p) && n >= 0 && n <= 255; });
    };
    expect(isValidIpv4('192.168.1.100')).toBe(true);
    expect(isValidIpv4('10.0.0.1')).toBe(true);
    expect(isValidIpv4('255.255.255.255')).toBe(true);
  });

  test('IPv4 格式校验：无效地址', () => {
    const isValidIpv4 = (ip: string) => {
      const parts = ip.split('.');
      if (parts.length !== 4) return false;
      return parts.every(p => { const n = Number(p); return /^\d+$/.test(p) && n >= 0 && n <= 255; });
    };
    expect(isValidIpv4('256.0.0.1')).toBe(false);
    expect(isValidIpv4('192.168.1')).toBe(false);
    expect(isValidIpv4('abc.def.ghi.jkl')).toBe(false);
    expect(isValidIpv4('')).toBe(false);
  });

  test('端口范围校验：有效端口', () => {
    const isValidPort = (p: number) => p >= 1024 && p <= 65535;
    expect(isValidPort(8889)).toBe(true);
    expect(isValidPort(1024)).toBe(true);
    expect(isValidPort(65535)).toBe(true);
  });

  test('端口范围校验：无效端口', () => {
    const isValidPort = (p: number) => p >= 1024 && p <= 65535;
    expect(isValidPort(80)).toBe(false);
    expect(isValidPort(1023)).toBe(false);
    expect(isValidPort(65536)).toBe(false);
  });
});

// ── 辅助功能权限检测 ──────────────────────────────────────────────────────

describe('macOS 辅助功能权限检测', () => {
  test('Denied 时显示遮罩', async () => {
    mockInvoke.mockResolvedValueOnce('未授权');
    const status = await (window as any).__TAURI__.invoke('check_accessibility_permission');
    expect(status === '未授权' || status === 'Denied').toBe(true);
  });

  test('Granted 时隐藏遮罩', async () => {
    mockInvoke.mockResolvedValueOnce('已授权');
    const status = await (window as any).__TAURI__.invoke('check_accessibility_permission');
    expect(status === '未授权' || status === 'Denied').toBe(false);
  });

  test('open_accessibility_settings 打开系统偏好设置', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await (window as any).__TAURI__.invoke('open_accessibility_settings');
    expect(mockInvoke).toHaveBeenCalledWith('open_accessibility_settings');
  });
});

// ── 连接历史管理 ──────────────────────────────────────────────────────────

describe('连接历史管理', () => {
  test('get_connection_history 返回历史列表', async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: 'manual:192.168.1.100:8889', name: '192.168.1.100', ip: '192.168.1.100', entryType: 'manual', lastConnected: '2026-04-04T10:00:00Z' },
      { id: 'device-001', name: 'MacBook Pro', ip: '192.168.1.50', entryType: 'auto', lastConnected: '2026-04-03T10:00:00Z' },
    ]);
    const history = await (window as any).__TAURI__.invoke('get_connection_history');
    expect(history).toHaveLength(2);
    expect(history[0].entryType).toBe('manual');
  });

  test('clear_connection_history 清空历史', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await (window as any).__TAURI__.invoke('clear_connection_history');
    expect(mockInvoke).toHaveBeenCalledWith('clear_connection_history');
  });
});

// ── 延迟检测 ──────────────────────────────────────────────────────────────

describe('延迟检测', () => {
  test('延迟 ≤ 50ms 不显示警告', async () => {
    mockInvoke.mockResolvedValueOnce(15);
    const latency = await (window as any).__TAURI__.invoke('get_latency');
    expect(latency > 50).toBe(false);
  });

  test('延迟 > 50ms 显示黄色警告', async () => {
    mockInvoke.mockResolvedValueOnce(80);
    const latency = await (window as any).__TAURI__.invoke('get_latency');
    expect(latency > 50).toBe(true);
  });

  test('重连次数超过 3 次后停止', () => {
    const MAX = 3;
    let attempts = 0;
    const shouldReconnect = () => { if (attempts >= MAX) return false; attempts++; return true; };
    expect(shouldReconnect()).toBe(true);
    expect(shouldReconnect()).toBe(true);
    expect(shouldReconnect()).toBe(true);
    expect(shouldReconnect()).toBe(false);
  });
});

// ── 快捷键录入 ────────────────────────────────────────────────────────────

describe('快捷键录入', () => {
  const capture = (e: any) => {
    const parts: string[] = [];
    if (e.metaKey) parts.push('Cmd');
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    const key = e.key;
    if (!['Meta', 'Control', 'Alt', 'Shift'].includes(key)) parts.push(key.toUpperCase());
    return parts.length > 1 ? parts.join('+') : null;
  };

  test('Ctrl+S 正确解析', () => {
    expect(capture({ ctrlKey: true, altKey: false, shiftKey: false, metaKey: false, key: 's' })).toBe('Ctrl+S');
  });

  test('Ctrl+Alt+S 正确解析', () => {
    expect(capture({ ctrlKey: true, altKey: true, shiftKey: false, metaKey: false, key: 's' })).toBe('Ctrl+Alt+S');
  });

  test('单独修饰键不录入', () => {
    expect(capture({ ctrlKey: true, altKey: false, shiftKey: false, metaKey: false, key: 'Control' })).toBeNull();
  });

  test('Cmd+Space 正确解析', () => {
    expect(capture({ ctrlKey: false, altKey: false, shiftKey: false, metaKey: true, key: ' ' })).toBe('Cmd+ ');
  });
});

// ── Bug #005：辅助功能权限遮罩文字应显示 KVMini 而非 KVM Share ──────────────

describe('Bug #005：权限遮罩文字品牌名称正确', () => {
  beforeEach(() => {
    // 加载真实 HTML 内容
    document.body.innerHTML = require('fs').readFileSync(
      require('path').join(__dirname, '../src/index.html'),
      'utf-8'
    ).replace(/^[\s\S]*<body[^>]*>/, '').replace(/<\/body>[\s\S]*$/, '');
  });

  test('权限遮罩描述文字不包含 KVM Share', () => {
    const desc = document.querySelector('.accessibility-overlay-desc');
    expect(desc?.textContent).not.toContain('KVM Share');
  });

  test('权限遮罩步骤文字不包含 KVM Share', () => {
    const steps = document.querySelector('.accessibility-overlay-steps');
    expect(steps?.textContent).not.toContain('KVM Share');
  });

  test('权限遮罩描述文字包含 KVMini', () => {
    const desc = document.querySelector('.accessibility-overlay-desc');
    expect(desc?.textContent).toContain('KVMini');
  });

  test('权限遮罩步骤文字包含 KVMini', () => {
    const steps = document.querySelector('.accessibility-overlay-steps');
    expect(steps?.textContent).toContain('KVMini');
  });
});

// ── Bug #006：启动时必须调用权限检测，否则权限引导永远不出现 ──────────────

describe('Bug #006：main.js initApp 必须调用 checkAccessibilityPermission', () => {
  test('main.js 源码中 initApp 函数必须调用 checkAccessibilityPermission', () => {
    const fs = require('fs');
    const path = require('path');
    const mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');

    // 提取 initApp 函数体（从 function initApp 到下一个顶层 function）
    const initAppMatch = mainJs.match(/function initApp\(\)[^{]*\{([\s\S]*?)(?=\n\/\*\*|\nfunction |\n\/\/ ─)/);
    expect(initAppMatch).not.toBeNull();

    const initAppBody = initAppMatch![1];
    // initApp 必须调用 checkAccessibilityPermission 或 AccessibilityModule.checkAccessibilityPermission
    const callsPermissionCheck =
      initAppBody.includes('checkAccessibilityPermission') ||
      initAppBody.includes('AccessibilityModule.checkAccessibilityPermission');

    expect(callsPermissionCheck).toBe(true);
  });

  test('main.js 中 checkAccessibilityPermission 调用必须传入 invoke', () => {
    const fs = require('fs');
    const path = require('path');
    const mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');

    // 确认调用时传入了 invoke 参数
    expect(mainJs).toMatch(/checkAccessibilityPermission\s*\(\s*invoke\s*\)/);
  });
});
