# KVMini 全量重构计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 对 KVMini 前后端代码进行全量重构，消除冗余、拆分职责、补充单元测试，提升代码质量和可维护性。

**Architecture:** 后端 Rust 拆分 main.rs 的 setup 闭包为独立函数，提取 connection 模块的纯函数；前端将 2009 行的 main.js 按职责拆分为 6 个模块（connection/ui/history/settings/layout/accessibility），每个模块配套单元测试。

**Tech Stack:** Rust, Tauri 2.x, TypeScript/Jest（前端测试）

---

## Task 1: 后端 - 拆分 main.rs setup 闭包

**Files:**
- Modify: `src-tauri/src/main.rs`

**Step 1: 写失败测试（验证函数存在性）**

在 `main.rs` 中，setup 闭包约 80 行，包含托盘构建、权限检查、初始状态推送。
目标：提取为 `build_tray()` 和 `emit_initial_status()` 两个独立函数。

由于 Rust 编译即测试，先确认当前编译通过：
```bash
cd src-tauri && cargo check 2>&1
```

**Step 2: 提取 `build_tray` 函数**

将 setup 中的托盘构建逻辑（MenuItem 创建、TrayIconBuilder）提取为：
```rust
fn build_tray(app: &mut tauri::App) -> tauri::Result<()> {
    // 托盘菜单和图标构建逻辑
    Ok(())
}
```

**Step 3: 提取 `emit_initial_status` 函数**

```rust
fn emit_initial_status(app: &tauri::AppHandle) {
    // 推送初始连接状态
}
```

**Step 4: 验证编译通过**
```bash
cd src-tauri && cargo check 2>&1
```

**Step 5: Commit**
```bash
git add src-tauri/src/main.rs
git commit -m "refactor: 拆分 main.rs setup 闭包为独立函数"
```

---

## Task 2: 后端 - connection/mod.rs 提取纯函数

**Files:**
- Modify: `src-tauri/src/connection/mod.rs`

**Step 1: 提取 `network_event_to_connection_event` 纯函数**

当前 `start_event_loop` 中的 match 分支内联在异步闭包里，无法单独测试。
提取为：
```rust
pub fn network_event_to_connection_event(
    event: &NetworkEvent,
    ip_opt: Option<&str>,
) -> Option<ConnectionEvent> {
    match event {
        NetworkEvent::ConnectionStateChanged(state) => {
            Some(ConnectionEvent::from_state(state, ip_opt))
        }
        NetworkEvent::Error(msg) => Some(ConnectionEvent::error(msg)),
        // ...
        _ => None,
    }
}
```

**Step 2: 将 `local_handshake_info` 移到 platform/mod.rs**

该函数逻辑属于平台层，不应放在 connection 模块：
```rust
// platform/mod.rs
pub fn local_handshake_info() -> HandshakeInfo { ... }
```

**Step 3: 验证编译**
```bash
cd src-tauri && cargo check 2>&1
```

**Step 4: Commit**
```bash
git add src-tauri/src/connection/mod.rs src-tauri/src/platform/mod.rs
git commit -m "refactor: 提取 network_event_to_connection_event 纯函数，移动 local_handshake_info"
```

---

## Task 3: 前端 - 创建 src/connection.js 模块（TDD）

**Files:**
- Create: `src/connection.js`
- Create: `__tests__/connection-module.test.ts`

**Step 1: 写失败测试**

```typescript
// __tests__/connection-module.test.ts
describe('connection.js 模块', () => {
  test('isValidIpv4 应接受合法 IP', () => {
    expect(window.ConnectionModule.isValidIpv4('192.168.1.1')).toBe(true);
  });
  test('isValidIpv4 应拒绝非法 IP', () => {
    expect(window.ConnectionModule.isValidIpv4('999.0.0.1')).toBe(false);
    expect(window.ConnectionModule.isValidIpv4('abc')).toBe(false);
    expect(window.ConnectionModule.isValidIpv4('')).toBe(false);
  });
  test('isValidPort 应接受 1024-65535 范围内的端口', () => {
    expect(window.ConnectionModule.isValidPort(23456)).toBe(true);
    expect(window.ConnectionModule.isValidPort(1024)).toBe(true);
    expect(window.ConnectionModule.isValidPort(65535)).toBe(true);
  });
  test('isValidPort 应拒绝范围外的端口', () => {
    expect(window.ConnectionModule.isValidPort(80)).toBe(false);
    expect(window.ConnectionModule.isValidPort(65536)).toBe(false);
  });
});
```

**Step 2: 运行测试确认失败**
```bash
npm test -- __tests__/connection-module.test.ts 2>&1
```

**Step 3: 创建 src/connection.js**

从 main.js 中提取连接相关函数：
- `isValidIpv4(ip)` - IP 格式校验
- `isValidPort(port)` - 端口范围校验
- `handleManualConnect()` - 手动 IP 连接
- `handleDisconnect()` - 断开连接
- `handleReconnect()` - 重新连接
- `connectToDevice(deviceId)` - 连接设备
- `reconnectToDevice(deviceId, name)` - 历史重连
- `reconnectManualIp(ip, port)` - 手动 IP 历史重连

**Step 4: 运行测试确认通过**
```bash
npm test -- __tests__/connection-module.test.ts 2>&1
```

**Step 5: Commit**
```bash
git add src/connection.js __tests__/connection-module.test.ts
git commit -m "refactor: 提取 connection.js 模块，补充单元测试"
```

---

## Task 4: 前端 - 创建 src/ui.js 模块（TDD）

**Files:**
- Create: `src/ui.js`
- Create: `__tests__/ui-module.test.ts`

**Step 1: 写失败测试**

```typescript
describe('ui.js 模块', () => {
  test('escapeHtml 应转义 HTML 特殊字符', () => {
    expect(window.UIModule.escapeHtml('<script>')).toBe('&lt;script&gt;');
    expect(window.UIModule.escapeHtml('"hello"')).toBe('&quot;hello&quot;');
    expect(window.UIModule.escapeHtml("it's")).toBe("it&#39;s");
    expect(window.UIModule.escapeHtml(null)).toBe('');
    expect(window.UIModule.escapeHtml(undefined)).toBe('');
  });
  test('setConnectionStatus 应更新 DOM 状态类', () => {
    document.body.innerHTML = '<span id="connection-status"></span>';
    window.UIModule.setConnectionStatus('connected', document.getElementById('connection-status'));
    expect(document.getElementById('connection-status').className).toContain('status-connected');
  });
});
```

**Step 2: 运行测试确认失败**
```bash
npm test -- __tests__/ui-module.test.ts 2>&1
```

**Step 3: 创建 src/ui.js**

从 main.js 中提取 UI 状态管理函数：
- `escapeHtml(str)` - HTML 转义
- `setConnectionStatus(status, el)` - 连接状态 UI
- `showError(message)` / `hideError()` - 错误提示
- `showSuccess(message)` - 成功提示
- `showReconnectStatus(message, attempt)` / `hideReconnectStatus()` - 重连状态
- `showReconnectToast(message)` - 重连 Toast
- `updateConnectionButtons(connected)` - 按钮状态
- `updateControlStateUI(state)` - 控制状态 UI

**Step 4: 运行测试确认通过**
```bash
npm test -- __tests__/ui-module.test.ts 2>&1
```

**Step 5: Commit**
```bash
git add src/ui.js __tests__/ui-module.test.ts
git commit -m "refactor: 提取 ui.js 模块，补充单元测试"
```

---

## Task 5: 前端 - 创建 src/history.js 模块（TDD）

**Files:**
- Create: `src/history.js`
- Create: `__tests__/history-module.test.ts`

**Step 1: 写失败测试**

```typescript
describe('history.js 模块', () => {
  beforeEach(() => localStorage.clear());

  test('getHistoryFromStorage 空时返回空数组', () => {
    expect(window.HistoryModule.getHistoryFromStorage()).toEqual([]);
  });
  test('saveManualIpToStorage 应保存并去重', () => {
    window.HistoryModule.saveToStorage({ id: 'manual:1.2.3.4:23456', ip: '1.2.3.4', port: 23456 });
    window.HistoryModule.saveToStorage({ id: 'manual:1.2.3.4:23456', ip: '1.2.3.4', port: 23456 });
    const history = window.HistoryModule.getHistoryFromStorage();
    expect(history.filter(h => h.id === 'manual:1.2.3.4:23456')).toHaveLength(1);
  });
  test('clearStorage 应清空历史', () => {
    window.HistoryModule.saveToStorage({ id: 'test', ip: '1.1.1.1' });
    window.HistoryModule.clearStorage();
    expect(window.HistoryModule.getHistoryFromStorage()).toEqual([]);
  });
});
```

**Step 2: 运行测试确认失败**
```bash
npm test -- __tests__/history-module.test.ts 2>&1
```

**Step 3: 创建 src/history.js**

从 main.js 中提取历史记录管理函数：
- `getHistoryFromStorage()` - 读取本地存储
- `saveToStorage(entry)` - 保存到本地存储（去重）
- `clearStorage()` - 清空本地存储
- `renderHistoryList(history, container)` - 渲染历史列表
- `loadConnectionHistory()` - 加载并渲染历史

**Step 4: 运行测试确认通过**
```bash
npm test -- __tests__/history-module.test.ts 2>&1
```

**Step 5: Commit**
```bash
git add src/history.js __tests__/history-module.test.ts
git commit -m "refactor: 提取 history.js 模块，补充单元测试"
```

---

## Task 6: 前端 - 创建 src/settings.js 模块（TDD）

**Files:**
- Create: `src/settings.js`
- Create: `__tests__/settings-module.test.ts`

**Step 1: 写失败测试**

```typescript
describe('settings.js 模块', () => {
  test('loadSettingsFromStorage 空时返回默认值', () => {
    localStorage.clear();
    const settings = window.SettingsModule.loadSettingsFromStorage();
    expect(settings.hotkey).toBeDefined();
    expect(settings.autoReconnect).toBe(true);
  });
  test('saveSettingsToStorage 应持久化设置', () => {
    const settings = { hotkey: 'Ctrl+Alt+K', autoReconnect: false };
    window.SettingsModule.saveSettingsToStorage(settings);
    const loaded = window.SettingsModule.loadSettingsFromStorage();
    expect(loaded.hotkey).toBe('Ctrl+Alt+K');
  });
});
```

**Step 2: 运行测试确认失败**
```bash
npm test -- __tests__/settings-module.test.ts 2>&1
```

**Step 3: 创建 src/settings.js**

从 main.js 中提取设置管理函数：
- `loadSettingsFromStorage()` - 从 localStorage 读取
- `saveSettingsToStorage(settings)` - 保存到 localStorage
- `loadSettingsToUI(settings, elements)` - 填充 UI
- `collectSettingsFromUI(elements)` - 从 UI 收集
- `openSettings()` / `closeSettings()` - 面板开关
- `saveSettings()` - 保存并同步后端

**Step 4: 运行测试确认通过**
```bash
npm test -- __tests__/settings-module.test.ts 2>&1
```

**Step 5: Commit**
```bash
git add src/settings.js __tests__/settings-module.test.ts
git commit -m "refactor: 提取 settings.js 模块，补充单元测试"
```

---

## Task 7: 前端 - 创建 src/layout.js 模块（TDD）

**Files:**
- Create: `src/layout.js`
- Create: `__tests__/layout-module.test.ts`

**Step 1: 写失败测试**

```typescript
describe('layout.js 模块', () => {
  test('calcBoundingBox 单显示器应返回正确包围盒', () => {
    const monitors = [{ width: 1920, height: 1080, offset_x: 0, offset_y: 0 }];
    const bbox = window.LayoutModule.calcBoundingBox(monitors);
    expect(bbox.totalW).toBe(1920);
    expect(bbox.totalH).toBe(1080);
    expect(bbox.minX).toBe(0);
    expect(bbox.minY).toBe(0);
  });
  test('calcBoundingBox 双显示器横向排列应返回合并包围盒', () => {
    const monitors = [
      { width: 1920, height: 1080, offset_x: 0, offset_y: 0 },
      { width: 2560, height: 1440, offset_x: 1920, offset_y: 0 },
    ];
    const bbox = window.LayoutModule.calcBoundingBox(monitors);
    expect(bbox.totalW).toBe(4480);
    expect(bbox.totalH).toBe(1440);
  });
  test('calcBoundingBox 空数组应返回默认值', () => {
    const bbox = window.LayoutModule.calcBoundingBox([]);
    expect(bbox.totalW).toBe(1920);
    expect(bbox.totalH).toBe(1080);
  });
  test('LAYOUT_TRIGGER_EDGE 应包含所有四个方向', () => {
    const edges = window.LayoutModule.LAYOUT_TRIGGER_EDGE;
    expect(edges['LocalLeftOfRemote']).toBeDefined();
    expect(edges['LocalRightOfRemote']).toBeDefined();
    expect(edges['LocalAboveRemote']).toBeDefined();
    expect(edges['LocalBelowRemote']).toBeDefined();
  });
});
```

**Step 2: 运行测试确认失败**
```bash
npm test -- __tests__/layout-module.test.ts 2>&1
```

**Step 3: 创建 src/layout.js**

从 main.js 中提取显示器布局函数：
- `calcBoundingBox(monitors)` - 计算包围盒（纯函数）
- `renderMonitorGroup(groupEl, monitors, bbox, scale, isLocal)` - 渲染显示器组
- `renderMonitorLayout(localMonitors, remoteMonitors, layout)` - 渲染布局预览
- `updateLayoutPreview(layout, preview, hintEdge)` - 更新预览
- `LAYOUT_TRIGGER_EDGE` / `LAYOUT_PREVIEW_CLASS` / `LAYOUT_DIVIDER_ARROW` - 常量

**Step 4: 运行测试确认通过**
```bash
npm test -- __tests__/layout-module.test.ts 2>&1
```

**Step 5: Commit**
```bash
git add src/layout.js __tests__/layout-module.test.ts
git commit -m "refactor: 提取 layout.js 模块，补充单元测试"
```

---

## Task 8: 前端 - 创建 src/accessibility.js 模块（TDD）

**Files:**
- Create: `src/accessibility.js`
- Create: `__tests__/accessibility-module.test.ts`

**Step 1: 写失败测试**

```typescript
describe('accessibility.js 模块', () => {
  test('isWindowsPlatform 在 Windows userAgent 下应返回 true', () => {
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
      configurable: true,
    });
    expect(window.AccessibilityModule.isWindowsPlatform()).toBe(true);
  });
  test('isWindowsPlatform 在 macOS userAgent 下应返回 false', () => {
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
      configurable: true,
    });
    expect(window.AccessibilityModule.isWindowsPlatform()).toBe(false);
  });
});
```

**Step 2: 运行测试确认失败**
```bash
npm test -- __tests__/accessibility-module.test.ts 2>&1
```

**Step 3: 创建 src/accessibility.js**

从 main.js 中提取辅助功能权限相关函数：
- `isWindowsPlatform()` - 平台检测（纯函数）
- `checkAccessibilityPermission()` - 检测权限
- `startAccessibilityPolling()` / `stopAccessibilityPolling()` - 轮询
- `checkPermissionAfterConnect()` - 连接后检测
- `showPermissionWarning()` / `hidePermissionWarning()` - 提示显示
- `openAccessibilitySettings()` - 打开系统设置

**Step 4: 运行测试确认通过**
```bash
npm test -- __tests__/accessibility-module.test.ts 2>&1
```

**Step 5: Commit**
```bash
git add src/accessibility.js __tests__/accessibility-module.test.ts
git commit -m "refactor: 提取 accessibility.js 模块，补充单元测试"
```

---

## Task 9: 前端 - 精简 main.js（组装层）

**Files:**
- Modify: `src/main.js`
- Modify: `src/index.html`（添加新模块的 script 标签）

**Step 1: 从 main.js 删除已提取的函数**

删除已移入各模块的函数，main.js 只保留：
- `appState` 全局状态
- `initApp()` - 初始化入口
- `setupEventListeners()` - 事件绑定（调用各模块）
- `loadLocalIp()` - 本机 IP 加载
- `initAppearance()` / `applyI18n()` - 外观初始化
- `initVersion()` - 版本号初始化
- `handleScan()` / `setScanningState()` / `updateDevicesList()` - 扫描功能
- `handleHotkeyCapture()` - 快捷键录入
- `startLatencyCheck()` / `stopLatencyCheck()` - 延迟检查
- `loadListenConfig()` - 监听配置

目标：main.js 从 2009 行压缩到 ~600 行

**Step 2: 更新 index.html 引入新模块**

```html
<script src="connection.js"></script>
<script src="ui.js"></script>
<script src="history.js"></script>
<script src="settings.js"></script>
<script src="layout.js"></script>
<script src="accessibility.js"></script>
<script src="main.js"></script>
```

**Step 3: 运行全量测试确认无回归**
```bash
npm test 2>&1
```

**Step 4: Commit**
```bash
git add src/main.js src/index.html
git commit -m "refactor: 精简 main.js 为组装层，引入各功能模块"
```

---

## Task 10: 回归验证与版本更新

**Step 1: 运行全量测试**
```bash
npm test 2>&1
```
预期：所有测试通过，测试数量 >= 238

**Step 2: 编译验证**
```bash
cd src-tauri && cargo check 2>&1
```

**Step 3: 版本号更新**
```bash
npm run version:update
```

**Step 4: 最终 Commit**
```bash
git add -A
git commit -m "[版本号] refactor: 全量重构前后端代码，补充单元测试"
```
