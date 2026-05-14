# KVMini Bug 清单

## Bug 记录格式

| 字段 | 说明 |
|------|------|
| 现象 | 用户看到的问题表现 |
| 根因 | 为什么会出现这个问题 |
| 修改方式 | 怎么改的 |
| 修改次数 | 该 bug 被修改了几次 |
| 出现次数 | 该 bug 出现了几次 |

---

## Bug #001：macOS 26 (Tahoe) 启动即崩溃

| 字段 | 内容 |
|------|------|
| **现象** | 应用在 macOS 26 beta 上启动后立即崩溃，无法显示主界面。日志只有大量 `[objc2-patch] is_main_thread: pthread_main_np() = 1`，然后 abort |
| **根因** | 第1/2次：`objc2-0.6.4` 的 `MainThreadMarker::new()` 使用 `pthread_main_np() == 1` 检测主线程。macOS 26 beta 在 `applicationDidFinishLaunching` 回调中，`pthread_main_np()` 返回 **`-1`**（线程初始化未完成），导致 `.unwrap()` panic。第3次：修复时引入了 `msg_send![NSThread, isMainThread]` 作为 fallback，但 `msg_send!` 宏内部会调用 `MainThreadMarker::new()` 进行线程安全检查，导致**无限递归 → 栈溢出 → abort** |
| **修改方式** | 1. 将 `objc2-0.6.4` 复制到 `patches/objc2-0.6.4/`；2. 修改 `patches/objc2-0.6.4/src/main_thread_marker.rs`：`pthread_main_np() == 1` 走快路径，否则用 **`objc_msgSend` 直接调用** `[NSThread isMainThread]`（绕过 `msg_send!` 的主线程检查，避免无限递归）；3. 在根 `Cargo.toml` 添加 `[patch.crates-io] objc2 = { path = "patches/objc2-0.6.4" }` |
| **修改次数** | 3 |
| **出现次数** | 3 |
| **关键教训** | 在 `is_main_thread()` 函数内部，**绝对不能使用 `msg_send!` 宏**，因为 `msg_send!` 内部会调用 `MainThreadMarker::new()` 导致无限递归。必须用 `objc_msgSend` 直接调用 |
| **测试用例** | `src-tauri/tests/main_thread_detection_test.rs` |

---

## Bug #002：Tauri 2.x 迁移 - emit_all 方法不存在

| 字段 | 内容 |
|------|------|
| **现象** | 升级 Tauri 2.x 后编译报错：`no method named 'emit_all' found for struct AppHandle<R>` |
| **根因** | Tauri 2.x 将 `emit_all` 重命名为 `emit`，且 `emit` 方法从 `Manager` trait 移到了 `Emitter` trait |
| **修改方式** | 1. 将所有 `emit_all(...)` 替换为 `emit(...)`；2. 在 `main.rs` 和 `connection/mod.rs` 中添加 `use tauri::Emitter;` |
| **修改次数** | 1 |
| **出现次数** | 1 |

---

## Bug #003：Tauri 2.x 迁移 - tauri.conf.json 配置字段错误

| 字段 | 内容 |
|------|------|
| **现象** | 升级 Tauri 2.x 后构建报错：`unknown field 'withGlobalTauri'`、`unknown field 'identifier'` 等 |
| **根因** | Tauri 2.x 配置文件结构变更：`withGlobalTauri` 从 `build` 移到 `app`；`identifier` 和 `productName` 从 `package`/`bundle` 移到顶层 |
| **修改方式** | 重写 `tauri.conf.json`，将 `identifier`、`productName` 提升到顶层，`withGlobalTauri` 移到 `app` 块 |
| **修改次数** | 1 |
| **出现次数** | 1 |

---

## Bug #004：Tauri 2.x 迁移 - capabilities 权限名称错误

| 字段 | 内容 |
|------|------|
| **现象** | 构建报错：`Permission shell:allow-open not found` |
| **根因** | Tauri 2.x 中 shell、notification、global-shortcut 是独立插件，需要在 Cargo.toml 中单独安装并在 main.rs 中注册 |
| **修改方式** | 在 `src-tauri/Cargo.toml` 中添加 `tauri-plugin-shell`、`tauri-plugin-notification`、`tauri-plugin-global-shortcut`；在 `main.rs` 中调用 `.plugin(...)` 注册 |
| **修改次数** | 1 |
| **出现次数** | 1 |

---

## Bug #005：权限遮罩文字品牌名称错误（KVM Share → KVMini）

| 字段 | 内容 |
|------|------|
| **现象** | build 版本启动后，辅助功能权限遮罩显示，遮罩内文字仍为 "KVM Share"，用户在系统设置中找不到该应用，无法完成授权，遮罩一直显示，所有按钮被遮挡无法点击 |
| **根因** | 重构时将应用名从 KVM Share 改为 KVMini，但 `src/index.html` 中权限遮罩的描述文字和步骤文字未同步更新，仍残留 "KVM Share" |
| **修改方式** | 修改 `src/index.html` 第 47 行描述文字和第 55 行步骤文字，将 "KVM Share" 替换为 "KVMini" |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/bug-regression.test.ts` - "Bug #005：权限遮罩文字品牌名称正确" |

---

## Bug #006：启动时权限引导从未触发

| 字段 | 内容 |
|------|------|
| **现象** | build 版本启动后，没有任何权限引导遮罩出现，所有按钮可以点击但功能不正常（因为没有辅助功能权限） |
| **根因** | `initApp()` 函数里从未调用 `window.AccessibilityModule.checkAccessibilityPermission(invoke)`。`accessibility.js` 定义了完整的权限检测逻辑，但 `main.js` 的 `initApp` 初始化流程中漏掉了这个调用，导致权限遮罩永远不会显示 |
| **修改方式** | 在 `src/main.js` 的 `initApp()` 函数末尾添加 `window.AccessibilityModule.checkAccessibilityPermission(invoke)` 调用；同时在 `setupEventListeners` 加防御性 null 检查和诊断日志 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/bug-regression.test.ts` - "Bug #006：main.js initApp 必须调用 checkAccessibilityPermission" |

---

## Bug #007：Tauri invoke 初始化时序问题（版本号显示 v0.2.0 + 权限引导从不出现）

| 字段 | 内容 |
|------|------|
| **现象** | build 版本启动后：1) 界面版本号始终显示 `v0.2.0`（硬编码降级值）；2) 辅助功能权限引导遮罩从不出现，所有按钮可点击但功能异常 |
| **根因** | Tauri 2.x build 版本中，`withGlobalTauri: true` 的 `window.__TAURI__` 是通过注入脚本**异步**挂载的。`DOMContentLoaded` 触发时 `window.__TAURI__` 尚未就绪，导致 `getTauriInvoke()` 返回 `null`。后续：`getAppVersion()` 的 `safeInvoke` 因 `_invoke = null` 走降级路径返回硬编码 `'0.2.0'`；`checkAccessibilityPermission(invoke)` 第一行 `if (!overlay \|\| !invoke) return` 因 invoke 为 null 直接静默返回。与 Bug #006 为同一问题的深层根因（#006 修复了"未调用"，本 bug 修复"调用时 invoke 为 null"） |
| **修改方式** | 1. 在 `src/main.js` 新增 `waitForTauri(timeout=3000)` 函数，轮询 `window.__TAURI__` 就绪状态，就绪后 resolve，超时后以降级模式 resolve 并打印警告；2. 将 `DOMContentLoaded` 回调改为 `waitForTauri().then(() => initApp())`；3. 在 `checkAccessibilityPermission` 中将静默 return 改为打印警告日志；4. 修复 `getAppVersion()` 和 `initVersion()` 的降级值从硬编码 `'0.2.0'` 改为从 HTML meta 标签动态读取 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/tauri-init.test.js` - waitForTauri 立即就绪/延迟就绪/超时降级；invoke 为 null 时打印警告；版本号降级显示 package.json 版本 |

---

## Bug #008：辅助功能权限引导反复不出现 + 按钮点击无响应（第4次复发）

| 字段 | 内容 |
|------|------|
| **现象** | 1) build 版本启动后，辅助功能权限引导遮罩从不出现；2) 按键悬浮会变色但点击没有任何反应；3) 问题已反复出现 4+ 次，每次修复只解决表面症状 |
| **根因 A** | `main.js` 底部存在**第二个** `DOMContentLoaded` 监听器（约第1900行），在 `waitForTauri()` 完成之前就触发，800ms 后调用 `checkAccessibilityPermission` 时 `invoke` 仍为 `null`，权限检测被静默跳过 |
| **根因 B** | `waitForTauri` 超时 resolve(false) 后，`initApp()` 仍然执行，`invoke = getTauriInvoke()` 仍为 `null`，导致权限检测被跳过，且无任何警告日志 |
| **根因 C** | Rust 实际返回中文字符串（`"已授权"` / `"未授权"` / `"N/A"`），但 JS 检测条件混用英文 `'Denied'` 和中文 `'未授权'`，`'Denied'` 永远不会匹配，属于历史遗留不一致 |
| **根因 D** | 按钮点击无响应：底层 Rust 命令（`connect_to_peer`、`start_listening` 等）因 `invoke` 未就绪而失败，且没有任何错误提示，用户无法感知 |
| **修改方式** | 1. 删除底部第二个 `DOMContentLoaded` 监听器，将 `initDisplayLayout` 移入 `initApp()` 流程；2. `waitForTauri` 超时后增加 `invoke` 为 null 的明确警告日志；3. 统一 Rust 返回值为英文枚举 `"Granted"` / `"Denied"` / `"NA"`，JS 检测改为 `status !== 'Granted'`；4. 所有按钮事件处理函数增加 `invoke` 为 null 时的明确错误提示；5. 完善全链路诊断日志 |
| **修改次数** | 1（本次为第1次系统性修复，前3次为表面修复） |
| **出现次数** | 4 |
| **测试用例** | `__tests__/accessibility-fix.test.ts` - 覆盖所有根因场景 |

---

## Bug #009：权限引导 + 按钮点击无响应（第5次复发，系统性根因修复）

| 字段 | 内容 |
|------|------|
| **现象** | 1) build 版本启动后，辅助功能权限引导遮罩从不出现；2) 按键悬浮会变色但点击没有任何反应；3) 问题已反复出现 5 次，每次修复只解决表面症状 |
| **根因 A** | `waitForTauri` 超时后仍调用 `initApp()`，但 `invoke` 仍为 null，权限检测被静默跳过，且无任何用户可见的错误提示 |
| **根因 B** | 缺乏在 build 版本中可自动运行的诊断手段，每次修复都是盲目的，无法验证 build 版本中 invoke 是否真的就绪 |
| **根因 C** | 按钮点击无响应：底层 Rust 命令因 `invoke` 未就绪而失败，且没有任何错误提示，用户无法感知 |
| **修改方式** | 1. 新增启动时自动诊断日志模块（`src/startup-diagnostics.js`），写入 `~/Library/Logs/KVMini/startup.log`；2. `waitForTauri` 超时后显示明确错误提示并终止初始化流程；3. 所有按钮 handler 在 `invoke` 为 null 时显示错误 Toast；4. 新增可在 build 版本中运行的专项测试脚本（`scripts/test-accessibility-e2e.js`） |
| **修改次数** | 1（本次为第1次系统性根因修复，前4次为表面修复） |
| **出现次数** | 5 |
| **测试用例** | `src/tests/startup-diagnostics.test.ts` - 覆盖 waitForTauri 超时、invoke 就绪、按钮点击、诊断日志场景 |

---

## Bug #010：macOS WKWebView 外部脚本隔离导致启动函数不可见（Bug #007/008/009 的真正根因）

| 字段 | 内容 |
|------|------|
| **现象** | build 版本启动后，辅助功能权限引导遮罩从不出现，按钮点击无响应。问题反复出现 5+ 次，每次修复只解决表面症状，下次构建后复发 |
| **根因** | macOS WKWebView（Tauri 使用的 WebView 引擎）会在 `<script src="main.js">` 执行完成后，**清除该脚本中新增到 `window` 的属性**，但保留之前脚本中定义的全局函数声明。这导致 `main.js` 中定义的所有启动函数（`kvminiBootstrap`、`initApp`、`waitForTauri` 等）在后续的 `bootstrapApp` 内联脚本中全部不可见（均为 `undefined`），应用永远无法启动。Bug #007/008/009 都是这个根因的表面症状，每次修复都在绕过症状而非解决根因 |
| **修改方式** | **方案 B（调整脚本顺序）**：1. 将前置内联脚本移到 `<script src="main.js">` 之前，只做一件事：`window.tauriInvokeReady = window.__TAURI__?.core?.invoke \|\| window.__TAURI__?.invoke \|\| null`；2. `main.js` 末尾用 IIFE `startApp()` 直接读取 `window.tauriInvokeReady`，有值则直接调用 `initApp()`，无值则走 `waitForTauri(5000)` 降级路径；3. 删除 `bootstrapApp` 复杂内联脚本；4. 删除 `startup-diagnostics.js`（临时诊断工具，已完成使命）；5. 删除所有 `kvminiBootstrap`、`kvminiBootstrapDone` 等历史遗留暴露 |
| **修改次数** | 1（本次为彻底架构修复，前 5 次为表面修复） |
| **出现次数** | 6（含 Bug #007/008/009 的所有复发） |
| **测试用例** | `__tests__/tauri-startup-arch.test.ts` - 18 个测试，覆盖：tauriInvokeReady 有值直接启动、无值走轮询、超时显示错误、版本号降级、遗留函数已删除 |

---

## Bug #012：扫描设备一瞬间结束（discover_devices 命令未注册）

| 字段 | 内容 |
|------|------|
| **现象** | 点击"扫描设备"按钮后，扫描动画一瞬间消失，没有发现任何设备，也没有等待过程 |
| **根因** | `main.rs` 从未注册 `discover_devices` Tauri command。`kvm-core::network::discover_devices` 函数已完整实现（UDP 广播，等待 5 秒），但 `main.rs` 的 `invoke_handler` 中没有暴露这个命令。前端 `invoke('discover_devices')` 立即报错，`catch` 块执行，`setScanningState(false)` 被调用，扫描一瞬间结束 |
| **修改方式** | 在 `main.rs` 中添加 `async fn discover_devices()` Tauri command，调用 `kvm_core::network::discover_devices(DEFAULT_UDP_PORT)`，并注册到 `invoke_handler` |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/discover-devices.test.ts` - 13 个测试，覆盖：命令存在、返回格式、空结果、扫描状态 UI、结果渲染 |

---

## Bug #017：旧版降级拖拽逻辑方向判断反了

| 字段 | 内容 |
|------|------|
| **现象** | 在不支持 HTML5 Drag & Drop API 的环境下，拖拽显示器图标后，布局方向与实际拖拽方向相反（拖到左边变成右边，拖到上面变成下面） |
| **根因** | `main.js` 旧版降级拖拽逻辑（约第1871行）中，`dx < 0` 时返回 `LocalRightOfRemote`（应为 `LocalLeftOfRemote`），`dy < 0` 时返回 `LocalBelowRemote`（应为 `LocalAboveRemote`），水平和垂直方向均判断反了 |
| **修改方式** | 将 `dx < 0 ? 'LocalRightOfRemote' : 'LocalLeftOfRemote'` 改为 `dx < 0 ? 'LocalLeftOfRemote' : 'LocalRightOfRemote'`；将 `dy < 0 ? 'LocalBelowRemote' : 'LocalAboveRemote'` 改为 `dy < 0 ? 'LocalAboveRemote' : 'LocalBelowRemote'` |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/layout-drag.test.ts` - 旧版降级逻辑方向语义验证（4个测试用例） |
| **关键教训** | 旧版降级逻辑与新版 `calcLayoutFromDrop` 纯函数的语义必须保持一致。`dx < 0` 表示本机在远端左侧，应返回 `LocalLeftOfRemote` |

---

## Bug #018：辅助功能权限检测失效（一直显示有权限）

| 字段 | 内容 |
|------|------|
| **现象** | 应用启动后，辅助功能权限引导遮罩从不出现，即使没有授权也显示"有权限"状态 |
| **根因** | Rust `setup` 阶段在 WebView 加载完毕之前就发送了 `accessibility://permission-denied` 事件，此时前端 `accessibility.js` 还未注册事件监听器，事件被丢弃。前端永远收不到 `permission-denied` 事件，遮罩永远不显示 |
| **修改方式** | 在 `accessibility.js` 的 `initAccessibilityEventListeners()` 函数末尾，添加主动查询初始权限状态的逻辑：调用 `invoke('check_accessibility_permission')`，根据返回值决定是否显示遮罩。这样即使 Rust 的事件被丢弃，前端也能在加载完毕后主动获取正确的权限状态 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/accessibility-event-driven.test.ts` - 需求 3.新增 describe 块（4个测试用例） |
| **关键教训** | 事件驱动架构中，如果事件发送方（Rust）比接收方（前端 JS）先启动，事件会丢失。解决方案：接收方加载完毕后主动查询一次当前状态，不完全依赖事件推送 |


以下测试在迁移前就已失败，不属于本次迁移引入的 bug：

1. `legacy-cleanup.test.ts` - 测试期望 `main.rs` 包含 `kill_legacy_process` 命令（该命令已被移除）
2. `i18n.test.ts` - 测试期望 `statusDisconnected` 值为 `"断开"`，但实际值为 `"未连接"`（i18n 文本不匹配）

---

## Bug #010：macOS WKWebView 外部脚本隔离导致启动函数不可见（第6次复发，彻底根因修复）

| 字段 | 内容 |
|------|------|
| **现象** | build 版本启动后，应用完全无法初始化：辅助功能权限引导从不出现，版本号显示硬编码值，所有按钮点击无响应。问题已反复出现 6 次，每次修复只解决表面症状 |
| **根因** | macOS WKWebView（Tauri 使用的 WebView 引擎）会在 `<script src="main.js">` 执行完成后，**清除该脚本中新增到 `window` 的属性**，但保留之前脚本中定义的全局函数声明。这导致 `main.js` 中定义的所有启动函数（`kvminiBootstrap`、`initApp`、`waitForTauri` 等）在后续的 `bootstrapApp` 内联脚本中全部不可见（均为 `undefined`），应用永远无法启动。这是 Bug #007/008/009 的真正根因，之前的所有修复都是在错误的假设下进行的 |
| **问题演化链** | Tauri 2.x 升级 → Bug #007(invoke=null) → 修复(waitForTauri) → Bug #008(第二个监听器冲突) → 修复(删除监听器) → Bug #009(超时后仍调用initApp) → 修复(startup-diagnostics+tauriInvokeReady) → Bug #010(main.js全局变量在bootstrapApp中不可见) → 根因(WKWebView脚本隔离) |
| **修改方式** | **方案 B（调整脚本顺序）**：1. 将 `bootstrapApp` 内联脚本移到 `<script src="main.js">` **之前**，只做一件事：`window.tauriInvokeReady = window.__TAURI__?.core?.invoke \|\| window.__TAURI__?.invoke \|\| null`；2. `main.js` 末尾直接读取 `window.tauriInvokeReady`：有值则直接调用 `initApp()`，无值则走 `waitForTauri(5000)` 轮询降级路径；3. 删除 `kvminiBootstrap`、`onDOMReady`、`bootstrapApp` 等复杂启动逻辑；4. 删除 `startup-diagnostics.js` 引用；5. 简化 `waitForTauri` 返回 `invoke` 函数而非 `boolean` |
| **修改次数** | 1（本次为彻底根因修复） |
| **出现次数** | 6 |
| **测试用例** | `__tests__/tauri-startup-arch.test.ts` - 覆盖 tauriInvokeReady 有值/无值/超时三种场景，前置脚本逻辑，启动流程集成，版本号降级 |
| **关键教训** | WKWebView 的脚本隔离特性：外部脚本（`<script src>`）执行完成后，该脚本中新增到 `window` 的属性会被清除。解决方案是让内联脚本在外部脚本之前执行，只做最小化的工作（捕获 invoke），让外部脚本自己完成启动 |

---

## Bug #011：main.js 因重复变量定义导致 SyntaxError 无法执行

| 字段 | 内容 |
|------|------|
| **现象** | build 版本启动后，`main.js` 完全无法执行：版本号显示 HTML 硬编码值，`initApp` 为 `undefined`，所有功能失效 |
| **根因** | `main.js` 第 1582 行重复定义了 `LAYOUT_TRIGGER_EDGE`、`LAYOUT_PREVIEW_CLASS`、`LAYOUT_DIVIDER_ARROW` 三个常量，而这三个常量已经在 `layout.js` 第 14 行定义。当 `layout.js` 先加载后，`main.js` 加载时 JS 引擎在**解析阶段**（非执行阶段）就报 `SyntaxError: Can't create duplicate variable: 'LAYOUT_TRIGGER_EDGE'`，导致整个 `main.js` 无法执行 |
| **为什么之前没发现** | 这三个常量是在将布局相关代码从 `main.js` 提取到 `layout.js` 时，忘记删除 `main.js` 里的旧定义。在 `eval` 执行时有独立作用域，`const` 不会与全局冲突，所以 eval 模式下不报错 |
| **修改方式** | 删除 `main.js` 里的三个重复常量定义（`LAYOUT_TRIGGER_EDGE`、`LAYOUT_PREVIEW_CLASS`、`LAYOUT_DIVIDER_ARROW`），保留 `layout.js` 里的定义 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **关键教训** | 将代码从一个文件提取到另一个文件时，必须同时删除原文件中的旧定义。重复的全局 `const` 定义会在脚本**解析阶段**（非执行阶段）报 `SyntaxError`，导致整个脚本无法执行，且错误信息不会出现在 `window.onerror` 里（因为脚本根本没有执行） |

---

## Bug #013：扫描设备 UX 反馈混乱（两个独立问题）

| 字段 | 内容 |
|------|------|
| **现象 A** | 第一次点击"扫描设备"，立即弹出"未发现设备，请确保目标设备在同一局域网内"提示框，没有任何扫描等待过程 |
| **现象 B** | 第二次点击"扫描设备"，等待约 5 秒后，扫描动画消失，但没有任何提示，用户不知道扫描是否完成 |
| **根因 A** | `handleScan()` 的 `catch` 分支使用了 `noDevicesAfterScan` 文案（"未发现设备，请确保目标设备在同一局域网内"）。该分支本应处理"命令调用失败"的技术错误，却显示了"没找到设备"的误导性文案。第一次调用时若 invoke 抛出异常，用户看到的是错误的提示 |
| **根因 B** | `handleScan()` 在 `devices.length === 0` 时只调用了 `updateDevicesList([])`，没有任何 Toast 通知。用户扫描完成后完全没有反馈，不知道扫描已结束 |
| **修改方式** | 1. `catch` 分支改用 `scanFailed`（"扫描失败，请重试"）文案，并 `console.error` 输出具体错误；2. 空结果分支在 `updateDevicesList([])` 后调用 `showSuccess(t('noDevicesFound'))`；3. 有结果分支调用 `showSuccess(t('scanFoundDevices').replace('{n}', n))`；4. 在 `src/i18n.js` 中英文配置中新增 `scanFailed`、`scanFoundDevices`、`noDevicesFound`（更新）三个 key |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/discover-devices.test.ts` - Bug #013 describe 块，包含测试用例 A/B/C/D 及 i18n 文案验证 |
| **关键教训** | `catch` 分支的文案必须准确描述"技术错误"而非"业务结果"。扫描完成（无论有无结果）都必须给用户明确的反馈，静默结束是糟糕的 UX |

---

## Bug #015：本机 TLS 证书指纹加载失败

| 字段 | 内容 |
|------|------|
| **现象** | 设置页"安全"分页中，本机 TLS 证书指纹显示"加载失败"，无法正常显示指纹值 |
| **根因** | 前端调用 `invoke('get_local_tls_fingerprint')`，但 Rust 后端从未实现该命令。后端只有 `get_local_ip` 等基础命令，没有任何 TLS 证书生成/加载/指纹计算逻辑 |
| **修改方式** | 1. 在 `src-tauri/Cargo.toml` 添加 `rcgen`、`sha2`、`hex` 依赖；2. 在 `main.rs` 实现 `get_local_tls_fingerprint`（加载或自动生成自签名证书，返回 SHA-256 指纹）和 `regenerate_tls_cert`（删除旧证书并重新生成）；3. 证书存储路径：`{data_local_dir}/KVMini/certs/`；4. 前端 `loadLocalTlsFingerprint` 函数增加错误行显示和重新生成按钮逻辑 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/cert-fingerprint.test.ts` - 7 个测试用例，覆盖加载成功/失败、指纹截断、重新生成等场景 |
| **关键教训** | 前端调用的 invoke 命令必须在后端有对应实现，否则会静默失败。新增 invoke 命令时，需同步更新 `invoke_handler!` 注册列表 |

---

## Bug #019：TLS 加密不一致时无任何提示

| 字段 | 内容 |
|------|------|
| **现象** | 一端开启 TLS 加密，另一端未开启时，连接可能失败或行为异常，但界面没有任何提示，用户无法感知原因 |
| **根因** | 1. `AppConfig` 中没有 `enable_tls` 字段，TLS 开关只是前端 UI 配置，底层网络层不携带 TLS 状态；2. 握手帧 `HandshakeInfo` 不携带 `tls_enabled` 字段，双端无法在握手时交换 TLS 状态；3. 连接成功后没有比较双端 TLS 状态的逻辑，也没有 `tls_mismatch` 事件；4. 前端没有 `tls-mismatch-banner` UI 组件 |
| **修改方式** | 1. `AppConfig` 添加 `enable_tls: bool` 字段（默认 `true`）；2. `HandshakeInfo` 添加 `tls_enabled: bool` 字段，`new_with_tls()` 构造函数携带 TLS 状态；3. `local_handshake_info()` 从配置读取 TLS 状态并携带到握手信息；4. `connect_to_peer` 握手成功后比较双端 TLS 状态，不一致时发送 `connection-status` 事件（`event_type: "tls_mismatch"`）；5. `ConnectionEvent` 添加 `tls_mismatch`、`local_tls`、`remote_tls` 字段；6. 前端 `index.html` 添加 `tls-mismatch-banner` 横幅 UI；7. `main.js` 处理 `tls_mismatch` 事件，显示横幅和警告图标；8. 添加 `tlsDisableAndRetry()` 函数，一键关闭本机 TLS 并重连；9. `styles.css` 添加横幅样式 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/tls-mismatch.test.ts` - 8 个测试用例，覆盖：本机开对端未开、本机未开对端开、一致时不显示、关闭重试按钮、警告图标、关闭按钮、enable_tls 字段、握手携带 tls_mismatch |
| **关键教训** | 网络协议的配置状态（如 TLS）必须在握手时双端交换，不能只在本地 UI 层面维护。不一致时必须给用户明确的提示和操作引导，而不是静默失败 |


| 字段 | 内容 |
|------|------|
| **现象** | 点击"保存设置"按钮后报错：`invalid args 'config' for command 'save_settings': command save_settings missing required key config` |
| **根因** | `main.js` 第 1056 行直接调用 `invoke('save_settings', { settings: appState.settings })`，传入的键名是 `settings`；而 Rust 后端 `save_settings(config: AppConfig)` 期望的键名是 `config`。`platform.js` 中的 `saveSettings(config)` 函数已正确使用 `{ config }` 键名，但 `main.js` 绕过了 `platform.js` 直接调用 `invoke`，使用了错误的键名 |
| **修改方式** | 将 `main.js` 中 `invoke('save_settings', { settings: appState.settings })` 改为 `invoke('save_settings', { config: appState.settings })`；同步修复 `platform.test.ts` 中错误验证 `{ settings }` 键名的测试用例 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/save-settings.test.ts` - 6 个测试用例，覆盖键名正确性、全局一致性、调用成功/失败场景 |
| **关键教训** | 前端应通过 `platform.js` 抽象层调用后端命令，避免直接 `invoke` 绕过抽象层导致参数不一致。直接 `invoke` 时必须与 Rust 函数签名的参数名完全一致 |

---

## Bug #021：中英文切换后大量文字未替换

| 字段 | 内容 |
|------|------|
| **现象** | 设置里选择语言后点保存，界面上大量文字没有变化，仍然显示原语言文本 |
| **根因** | 1. `applyI18n()` 只处理带 `data-i18n` 属性的元素，但 HTML 中大量元素缺少该属性（设置弹窗标题、连接详情面板标签、重连进度、授权界面、取消确认对话框、提示面板等）；2. `i18n.js` 的 MESSAGES 中缺少对应的翻译 key（`detailTitle`、`detailLocal`、`detailPeer`、`discardDialogTitle` 等）；3. `layout.js` 的 `renderMonitorGroup` 硬编码了中文"本机"、"副屏"、"远端"；4. `settings-tabs.js` 的 `SETTINGS_TABS` 数组缺少 `'about'` 分页；5. `applyI18n()` 没有更新 `document.title` 和显示器布局预览中的标签 |
| **修改方式** | 1. 在 `i18n.js` 中英文 MESSAGES 中添加所有缺失的翻译 key（`pageTitle`、`detailTitle`、`detailLocal`、`detailPeer`、`detailEncryption`、`detailLatency`、`detailCertFingerprint`、`labelReconnectProgress`、`licenseGateTitle`、`licenseGateDesc`、`btnActivate`、`licenseGateDivider`、`btnBuyLicense`、`licenseActivated`、`licenseSdkError`、`discardDialogTitle`、`discardDialogMsg`、`btnDiscardChanges`、`btnContinueEditing`、`btnClose`、`labelLocalMonitor`、`labelSecondaryMonitor`、`labelRemoteMonitor`、`labelPrimaryTag`、`tabGeneral`、`tabDisplay`、`tabNetwork`、`tabSecurity`）；2. 在 `index.html` 中给所有硬编码文本元素添加 `data-i18n` 属性；3. 修改 `layout.js` 的 `renderMonitorGroup` 使用 `window.i18n.t()` 翻译标签；4. 在 `settings-tabs.js` 的 `SETTINGS_TABS` 数组中添加 `'about'`；5. 在 `applyI18n()` 中添加 `document.title` 更新和显示器布局预览重新渲染 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/i18n-theme-layout-fix.test.ts` - Bug #021 describe 块（77 个测试用例中的 i18n 相关部分） |
| **关键教训** | 国际化必须覆盖所有用户可见文本，包括动态生成的文本（如 `renderMonitorGroup`）。每次添加新 UI 元素时，必须同时添加 `data-i18n` 属性和对应的翻译 key |

---

## Bug #022：主题切换后界面未变化

| 字段 | 内容 |
|------|------|
| **现象** | 设置里选择主题后点保存，界面颜色没有变化 |
| **根因** | `styles.css` 中有一个语法错误：`[data-theme="dark"] .cert-error-row` 选择器块结束后，紧跟着一段没有选择器的孤立属性声明（`font-size: 11px; color: var(--version-color); ...`），这是 `.settings-version` 的深色主题样式但缺少选择器前缀。这个语法错误导致 CSS 解析器在该位置之后的所有样式规则可能被忽略或错误解析 |
| **修改方式** | 在孤立的属性声明前添加 `[data-theme="dark"] .settings-version` 选择器 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/i18n-theme-layout-fix.test.ts` - Bug #022 describe 块（6 个测试用例） |
| **关键教训** | CSS 中孤立的属性声明（没有选择器）不会报错但会导致后续样式解析异常。修改 CSS 后应检查是否有遗漏的选择器 |

---

## Bug #023：显示器拖拽逻辑不直观（改用 mousedown/mousemove）

| 字段 | 内容 |
|------|------|
| **现象** | 1. 拖拽显示器方块时只有一个半透明 ghost image，元素不会真正跟随鼠标移动；2. 放下后元素不会移动到新位置；3. 同时还有一个下拉菜单可以选择方向，两种交互方式并存造成混乱 |
| **根因** | 使用了 HTML5 Drag & Drop API，该 API 有严重的 UX 限制：拖拽时只有 ghost image、无法自定义拖拽中的视觉效果、放下后不会移动元素。同时保留了下拉菜单作为备选，两种交互并存增加了用户困惑 |
| **修改方式** | 1. 将 `layout.js` 的 `initLayoutDrag` 从 HTML5 Drag & Drop API 改为 `mousedown`/`mousemove`/`mouseup` 实现（参考 Synergy/Barrier 设计）；2. 拖拽时元素真正跟随鼠标移动（切换到绝对定位），放下后自动吸附到最近的边缘方向；3. 移除 `index.html` 中的 `display-layout-select` 下拉菜单和 `draggable="true"` 属性；4. 更新 `main.js` 的 `initDisplayLayout` 和 `saveLayoutConfig` 使用 `window._currentLayout` 替代下拉菜单值；5. 添加拖拽中的视觉反馈 CSS（`.dragging`、`.drag-active`） |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/i18n-theme-layout-fix.test.ts` - Bug #023 describe 块（9 个测试用例） |
| **关键教训** | HTML5 Drag & Drop API 不适合需要精确视觉反馈的拖拽场景。`mousedown`/`mousemove`/`mouseup` 虽然实现更复杂，但 UX 更好。交互方式应该唯一，不要同时提供两种方式让用户困惑 |

| 字段 | 内容 |
|------|------|
| **现象** | `connection-detail-panel` 中延迟显示颜色不生效；`nic-list` 中网卡类型图标不显示 |
| **根因** | 1. `updateDetailLatency` 使用 `latency-high`/`latency-medium` 类名，但 CSS 中没有对应样式；2. `_renderNetworkIfaceList` 只渲染到 `#network-iface-list`，没有渲染到 `#nic-list`（带类型图标的新容器）；3. CSS 中只有 `nic-list-item` 类，没有 `nic-item` 类 |
| **修改方式** | 1. 统一 `updateDetailLatency` 类名为 `latency-warning`（>100ms）和 `latency-critical`（>300ms）；2. CSS 添加 `latency-warning`/`latency-critical` 样式；3. `_renderNetworkIfaceList` 同时渲染到 `#nic-list`，带 Wi-Fi/有线/VPN 类型图标；4. CSS 添加 `nic-item` 作为 `nic-list-item` 的别名 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/connection-detail-panel.test.ts` - 21 个测试用例，覆盖：DOM 存在性、连接成功/断开、TLS 状态、延迟颜色、展开/折叠、网卡说明 UI |
| **关键教训** | CSS 类名必须在 JS 和 CSS 中保持一致。新增 DOM 容器时，需同步更新 JS 渲染逻辑 |

---

### Bug #024 — 设置按钮点击无响应

| 字段 | 内容 |
|------|------|
| **现象** | 点击「设置」按钮无任何反应，设置页面不打开。所有按钮事件均不生效 |
| **根因** | `src/main.js` 第 81 行 `initApp()` 声明为普通函数 `function initApp()`，但第 102 行使用了 `await window.Platform.getPlatform()`。`await` 在非 `async` 函数中导致运行时语法错误，`initApp()` 在该行中断，后续的 `setupEventListeners()` 从未执行，所有 UI 按钮的 click 事件监听器都未绑定 |
| **修改方式** | 1. 将 `function initApp()` 改为 `async function initApp()`；2. 在两处调用 `initApp()` 处添加 `.catch()` 防止 unhandled promise rejection |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | 手动验证：启动应用后点击设置按钮，确认设置页面正常打开 |
| **关键教训** | 使用 `await` 关键字时必须确保所在函数声明为 `async`。非 `async` 函数中的 `await` 在严格模式下是语法错误，在非严格模式下可能是运行时错误。函数初始化阶段的错误会导致整个应用事件系统失效 |

---

### Bug #025 — 每次重装需重新授权辅助功能权限

| 字段 | 内容 |
|------|------|
| **现象** | 每次构建安装新版本后，macOS 辅助功能权限丢失，需要在「系统设置 → 隐私与安全性 → 辅助功能」中重新授权 |
| **根因** | `src-tauri/tauri.conf.json` 中 `signingIdentity: null` 导致 Tauri 构建使用 ad-hoc 签名。ad-hoc 签名每次构建生成不同的 identifier hash（如 `kvmini-99ef01e7bab847b4`），macOS TCC 数据库按此 identifier 记录权限，因此每次重装都被视为新应用需要重新授权 |
| **修改方式** | 1. 更新 `scripts/sign-dev.sh` 支持 release 构建路径（Tauri 2.x 路径 `target/release/bundle/macos/KVMini.app`）；2. 在 `build.sh` 构建完成后自动调用签名脚本；3. 在 `scripts/launch-kvmshare.command` 安装前自动签名；4. 签名使用稳定的自签名证书 "KVMini Dev"，确保每次构建的 identifier 一致 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | 验证步骤：构建 → 安装 → 授权辅助功能 → 重新构建安装 → 确认辅助功能权限仍然保留 |
| **关键教训** | macOS TCC 权限通过代码签名 identity 识别应用。ad-hoc 签名（无证书）每次构建产生不同的 identifier，导致权限不可复用。必须使用稳定的自签名证书签名，确保 identity 跨构建不变 |

---

## Bug #026：同局域网连接自动断开（心跳无 HeartbeatAck 回复）

| 字段 | 内容 |
|------|------|
| **现象** | 两台同局域网设备连接后，约 15 秒内自动断开，提示"连接已断开，自动重连 3 次均失败，请手动重连" |
| **根因** | `dispatch_frame` 收到对方 `Heartbeat` 帧时，只更新了 `last_heartbeat` 时间戳，但**没有回复 `HeartbeatAck`**。而心跳发送方的 `start_heartbeat` 检测的是 `HeartbeatAck` 到达时间（`HEARTBEAT_INTERVAL_SECS * 2 = 10秒`），3次超时（`HEARTBEAT_TIMEOUT_COUNT=3`，约 15 秒）后判定断开 |
| **修改方式** | `read_loop` 和 `dispatch_frame` 增加 `writer` 参数；收到 `KvmFrame::Heartbeat` 时立即回复 `KvmFrame::HeartbeatAck` |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **关键教训** | 心跳保活必须是双向的：发送方发 Heartbeat，接收方必须回复 HeartbeatAck；只更新时间戳而不回包，发送方永远收不到 Ack 会判定断线 |


---

## Bug #027：启动时只提示开了辅助功能权限，漏了输入监控权限

| 字段 | 内容 |
|------|------|
| **现象** | macOS 用户首次启动 KVMini 0.1.35 时，只看到"辅助功能未授权"的全屏引导；当用户点"去授权"回来后看到 App 能用，实际使用中却发现键鼠事件既不能捕获（CGEventTap 被 TCC 拒绝），界面上也不给任何提示。用户被迫自行去系统设置找"输入监控"开关 |
| **根因** | macOS 上 KVMini 同时需要两种权限：① Accessibility（AXIsProcessTrusted，用于注入鼠标/键盘事件）；② Input Monitoring（CGPreflightListenEventAccess，用于 CGEventTap 抓取事件）。v0.1.35 前启动自检只调用了 `check_accessibility_permission`，完全没有检查输入监控；即便输入监控未开，也不会发任何事件给前端，遮罩就永远不显示。更糟的是 `CGPreflightListenEventAccess` 在进程生命周期内有缓存，必须先调用 `CGRequestListenEventAccess()` 主动注册 TCC 列表，否则用户去系统设置都找不到 KVMini 的开关（这块归为 BUG-021 的前置动作） |
| **修改方式** | 1. 后端 `src-tauri/src/platform/macos/permission.rs` 新增 `check_input_monitoring()` / `request_input_monitoring()` / `open_input_monitoring_settings()` 三个函数，用 `#[link(name = "CoreGraphics")]` 声明 `CGPreflightListenEventAccess`/`CGRequestListenEventAccess` 符号；2. `main.rs::check_macos_permission` 扩展为双路检测：先查辅助功能发 `accessibility://permission-*`，再调 `request_input_monitoring()` 主动注册 TCC + 发 `input-monitoring://permission-*` 事件；未授权时 `start_input_monitoring_polling()` 每 3s 轮询一次（上限 5 分钟），授权后立即发 granted 并退出；3. 新增两个 Tauri 命令 `check_input_monitoring_permission` / `open_input_monitoring_settings` 并注册到 invoke_handler；4. 前端 `src/accessibility.js` 重写为"双权限版"：`_currentOverlayKind` 记录当前遮罩种类，`applyOverlayText()` 按种类切换 i18n 文案，同时监听 4 个事件；"前往授权"按钮按种类分别调用不同的后端命令；5. `src/index.html` 遮罩内所有硬编码中文加 `data-i18n` 属性；`src/i18n.js` zh/en 字典新增 `overlayInputMonitoringTitle` / `overlayInputMonitoringDesc`；6. `checkInitialPermissions()` 修复：只有明确收到 'Denied' 才主动显示遮罩，undefined/null/未知值一律保持现状等待事件（避免 mock 场景误触发） |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/permission-dual-check.test.ts`（12 用例，覆盖 i18n 完整性 / HTML data-i18n 标注 / accessibility.js 事件监听注册）；`src-tauri/src/platform/macos/permission.rs::tests`（新增 3 用例：test_status_display / test_check_input_monitoring_no_crash / test_check_accessibility_no_crash） |
| **关键教训** | 多权限场景必须做成"显式清单 + 分别提示"，任何一个未授权都应有对应文案；不要假设"只要主权限过了其它自然也过"。测试 mock 返回 undefined 时，前端逻辑不应把 undefined 等同于未授权，否则会出现"真实环境没事、测试环境误触发"的脆弱断言 |


---

## Bug #018：手动连接成功后误报「TLS 不一致」

| 字段 | 内容 |
|------|------|
| **现象** | 两台 Mac 都已开启 TLS，连接成功后弹横幅「本机已开启 TLS 加密，但对端未开启」 |
| **根因** | 握手协议在 wire 上只传了 `desktop`（VirtualDesktop），并未把 `tls_enabled` 透传给对端；`HandshakeAck.screen_info` 里携带的是空/默认 desktop，导致 connect_to 端解析得到的 `peer_info.tls_enabled = false`，与本机的 `true` 比较后误判为不一致 |
| **修改方式** | 1. 新增 `pub(crate) struct HandshakePayload { desktop, tls_enabled }`，二者均 `#[serde(default)]` 兼容旧格式；2. `HandshakeInfo::screen_info_bytes()` 序列化整个 payload；`parse_payload()` 用 `bytes.windows().any()` 检查字节流中存在 `desktop` 或 `tls_enabled` 字符串才走新路径，否则回退旧 VirtualDesktop 解析；3. `connect_to` / `handle_stream` 双向写入 tls_enabled，`accept_loop` / `start_listening(&self, local_info)` 透传 local_info |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/handshake-tls-payload.test.ts`（5 用例）+ `kvm-core/src/network.rs::tests`（roundtrip / 旧格式兼容 / 字段保留 / 默认值，5 用例） |
| **关键教训** | 跨进程协议变更必须显式写入 wire，不能假设两端"都开了所以一致"；向后兼容用 `serde(default)` + 字符串嗅探回退而非版本字段，零成本无破坏性 |

---

## Bug #019：连接成功后点击「开始共享」按钮没有任何反应

| 字段 | 内容 |
|------|------|
| **现象** | 两端连接成功，状态栏显示 Connected，但点击「开始共享」按钮无任何变化（按钮无 hover 反馈、不调命令、不切状态、键鼠不转发） |
| **根因** | 重大功能缺口：① 后端从未注册 `toggle_sharing` / `toggle_control` Tauri 命令；② 前端 listen('hotkey-triggered') 调的 `invoke('toggle_control')` 是幽灵命令，永远抛错；③ `StateMachine` / `InputCapture` / `InputExecutor` 代码全部存在但从未接入主流程，`platform/mod.rs` 顶部 `#![allow(dead_code)]` 是证据；④ 接收侧 `connection::start_event_loop` 的 `_ => None` 分支把所有 `KvmFrame::InputEvent` 直接丢弃 |
| **修改方式** | 1. 新建 `src-tauri/src/sharing.rs::SharingManager`，托管 `Arc<Mutex<StateMachine>>` + `PlatformCapture` + `PlatformExecutor`；新增 `toggle_sharing` / `get_control_state` Tauri 命令并在 main.rs invoke_handler 注册；`.manage(Arc::new(SharingManager::new()))`；2. `connection::start_event_loop` 新增 FrameReceived(MouseMove/MouseButton/MouseScroll/KeyEvent)→`sm.inject_remote_frame()`、SwitchControl→`on_remote_switch_control`、SwitchControlAck→`on_remote_switch_ack`、ConnectionStateChanged(Disconnected)→`on_disconnected` 转发；3. `KvmConnectionManager::inner_send_frame` 转发到 `inner.send_frame`；SharingManager 通过 `app.try_state::<KvmConnectionManager>()` 动态发送；4. 按 macos-cgevent-tap-hotkey skill 改造 `MacInputCapture`：用专用 std::thread 跑 CFRunLoop（sync_channel 握手验证 Tap 创建成功 + `std::mem::forget(tap)` 保持 CFMachPort 强引用），stop 时 `CFRunLoop::stop` 唤醒退出；5. 前端 `setupEventListeners` 绑定 startSharingBtn click → handleToggleSharing → invoke('toggle_sharing')；listen('control-state-changed') 在 to=LocalControlling/Idle 间切换按钮 data-i18n（btnStartSharing ↔ btnStopSharing）+ 状态栏 + showSuccess 提示；删除 toggle_control / stop_input_capture 幽灵调用 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/sharing-button.test.ts`（6 用例：click→invoke、错误→showError、状态→按钮 i18n、英文模式、字典完整性）+ `src-tauri/src/sharing.rs::tests`（4 用例：raw_to_frame 各分支 + state_name） |
| **关键教训** | 前端调一个未注册的 Tauri 命令时只会在 console.warn，不会有任何用户可见反馈；后端关键链路缺口必须用 grep `invoke_handler!` + 平台 capture/executor 实例化点进行交叉校验，不能凭"代码存在"就假设接入；CGEventTap 在主 RunLoop 上注册时会被 Tauri/tokio 调度抢占，必须自管理子线程 RunLoop |

---

## Bug #020：macOS 防火墙开启 + VPN(utun6) 时两台机器互相扫描不到

| 字段 | 内容 |
|------|------|
| **现象** | 两台 Mac 都打开 KVMini，主界面扫描列表始终为空；ping 通对端 IP，手动输 IP 也能连接成功 |
| **根因** | ① VPN（utun*）会接管系统默认路由：UDP 广播只 bind 在 `0.0.0.0` 时，会优先从 utun 出去（utun 没有 broadcast 概念，包被丢弃），物理网卡上一个广播都没发出去；② 之前 `discover_devices` 只用单 socket 兜底广播，且没有按物理网卡分别 bind+定向广播；③ macOS 防火墙未授权时，应用主动收发 UDP 也会被静默拦截，但前端没有任何提示 |
| **修改方式** | 1. 新建 `src-tauri/src/network_scan.rs`：`LanIface { name, ipv4, broadcast }`、`is_vpn_iface`（按 utun*/ppp*/tun*/tap*/wg*/tailscale/openvpn 等前缀+包含词过滤）、`compute_broadcast(ip,mask) = ip|!mask`、`list_lan_ipv4_ifaces`；2. `discover_devices_multi(udp_port)` 对每张物理网卡 `bind(<iface_ip>:0)` + `set_broadcast(true)` + `send_to(<iface_broadcast>:port, MAGIC)` + 兜底 `send_to(255.255.255.255:port)`，并发收 1.5s 内的响应去重；3. `scan_subnet_multi(tcp_port)` 对每张物理网卡并发跑现有 scan_subnet 合并去重；4. main.rs `discover_devices` 改三段式：多接口 UDP → 单 socket 广播兜底 → 多接口子网 TCP 扫描；`get_local_lan_ip` 优先用 `list_lan_ipv4_ifaces()` 第一张物理网卡；5. `diagnose_network` 检测到 utun*/VPN 时按 locale 返回中英双语可操作提示（含 macOS `socketfilterfw` / Windows `netsh advfirewall` 命令、临时断开 VPN 建议） |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `src-tauri/src/network_scan.rs::tests`（6 用例：is_vpn_iface 各前缀 / compute_broadcast /24 与 /16 / list_lan_ipv4_ifaces 不为空 / detect_vpn_ifaces）+ `__tests__/i18n.test.ts` 新增 vpnInterferenceTip / firewallBlockTipMacos / firewallBlockTipWindows 三个 key 中英双语完整性校验 |
| **关键教训** | UDP 广播在多接口/VPN 环境下必须显式 bind 物理网卡 IP 才能保证从该网卡出去；255.255.255.255 受系统路由表影响不可靠；扫描失败必须给用户**可执行**的诊断信息（具体命令），不能只说"未发现设备" |


---

## Bug #023：试用商业化伪装绕过测试（v0.1.37→v0.1.38）

| 字段 | 内容 |
|------|------|
| **现象** | 之前 v0.1.37 为了快速绕过试用到期，前端把购买/试用 UI 全屏蔽，但 `updateSmartStatus` 的加密徽章 tooltip 仍然写「🔒 TLS 加密」，容易让用户误以为数据已经加密 |
| **根因** | i18n 文案与真实功能不一致：TLS 当前只是握手时交换一个 `tls_enabled` 协商位，应用层数据仍是明文；徽章 tooltip 过度承诺 |
| **修改方式** | 修改 `src/i18n.js` 的 `tlsBadgeOn/tlsBadgeOff`（zh/en）为诚实表述：「🔒 TLS 兼容模式（数据暂未加密，将在 v0.1.39 启用真实 TLS 加密）」；changelog 同步披露 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **关键教训** | 文案必须对真实能力负责：宁可显示"兼容模式"也不能让用户误以为端到端已加密 |

---

## Bug #024：连接详情面板与智能状态面板重复渲染

| 字段 | 内容 |
|------|------|
| **现象** | 主界面连接成功后，存在两个显示相同信息（本机/对端/延迟/证书指纹）的面板：顶部的 `#smart-status-panel` + 下方的 `#connection-detail-panel`，信息重复、UI 杂乱 |
| **根因** | 历史迭代中分两次实现了连接状态显示，没有及时合并；每次状态变更要在两处分别 `updateConnectionDetailPanel` 和状态面板，容易漏掉一处 |
| **修改方式** | 按奥卡姆剃刀原则合并：1. `src/index.html` 删除整块 `#connection-detail-panel`；主面板 `.status-panel` 改名为 `#smart-status-panel`，加入 `#encryption-badge` 和 `<details id="peer-fingerprint-details">` 子元素；2. `src/main.js` 新增 `updateSmartStatus(data)` 统一入口，删除 `updateConnectionDetailPanel/updateDetailLatency/toggleConnectionDetail`；3. `src/i18n.js` 删除冗余 key（detailTitle/detailLocal/detailPeer/detailEncryption/detailLatency），保留 `detailCertFingerprint`；4. `src/styles.css` 删除旧面板样式，新增 `.encryption-badge` 与 `.status-fingerprint-details` |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/smart-status-panel.test.ts`（7 用例：`#connection-detail-panel` 已删除、`#smart-status-panel` 存在、徽章存在、`updateSmartStatus` 导出、🔒/⚠️ 文案、details 包裹指纹） |
| **关键教训** | 状态显示类 UI 必须有单一数据入口，重复实现既浪费心智又容易产生一致性 bug |

---

## Bug #025：src/main.js 与 index.html 中残留硬编码中文

| 字段 | 内容 |
|------|------|
| **现象** | 尽管 v0.1.34 已经接入 i18n，切换到英文后仍有若干控件（未保存提示、错误气泡的默认文案、部分输入框 placeholder）显示中文 |
| **根因** | 新增功能时偷懒直接写字面量（例如 `textContent = '重连中...'` 或 `<span>未保存</span>`），绕过了 `t()` 和 `data-i18n` |
| **修改方式** | 新增 `__tests__/i18n-hardcode-guard.test.ts` 回归守卫：静态扫描 `src/main.js` 和 `src/index.html`，禁止出现汉字字符（白名单注释除外）；按报错逐条加 i18n key + 改用 `t(...)`/`data-i18n` |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/i18n-hardcode-guard.test.ts` |
| **关键教训** | 依赖人工审查不可靠，必须有静态守卫；每次 PR 前跑 `npm test -- i18n-hardcode-guard` |

---

## Bug #026：设置页未保存红点缺乏语义提示

| 字段 | 内容 |
|------|------|
| **现象** | 切换设置 tab 时看到某个 tab 标题右上角有小红点，但不知道代表什么；保存按钮也没有对应反馈 |
| **根因** | `markDirty()` 只是给 tab 加了 `.has-dirty` CSS 类显示红点，没有 tooltip；保存按钮也没有脉冲动画提示「有待保存的修改」 |
| **修改方式** | 1. `src/i18n.js` 新增 `tabDirtyTooltip`（zh: "此分类下有未保存的修改" / en: "Unsaved changes in this section"）；2. `src/settings-tabs.js` 的 `markDirty/clearDirty` 额外同步 button 的 `title` 属性；3. 抽出 `_syncSaveButtonDirtyClass()`，任一 tab dirty 时给 `#settings-save-btn` 加 `.btn-save-dirty`；4. `src/styles.css` 新增 `@keyframes save-pulse` 和 `.btn-save-dirty` 规则实现呼吸动画 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/settings-dirty-tooltip.test.ts`（4 用例：i18n key 存在、button.title 同步、`.btn-save-dirty` class 联动、`save-pulse` keyframes 存在） |
| **关键教训** | UI 反馈必须明示语义；红点/徽章类纯视觉信号必须配合 tooltip 或动画传递含义 |

---

## Bug #028：扫描到设备点击「连接」没有任何响应（plan v0.1.38 BUG-020）

| 字段 | 内容 |
|------|------|
| **现象** | 在 v0.1.37 上，扫描到局域网内的另一台 KVMini 设备后，点「连接」按钮整个界面毫无反应，控制台只留下一句 `command not found: connect_to_device` |
| **根因** | 前端 `connection.js` 和 `main.js` 共 4 处直接 `invoke('connect_to_device', { deviceId })`，但后端 Tauri 命令实际叫 `connect_to_peer`，且参数是 `{ peerIp, port }`。历史重构时命令名改了但前端漏改，discover / manual / history / reconnect 四条路径全被波及。扫描结果里带的是 `item.id`（格式 `name:ip:port`）而不是 IP，需要切分后再传。 |
| **修改方式** | 1. `src/connection.js:148` 统一改为 `invoke('connect_to_peer', { peerIp, port })`；2. `src/main.js` 三处 `connect_to_device` 调用全部更换为 `window.Platform.connectByIp(ip, port)`（内部走 connect_to_peer）；3. 从 `deviceId` 中提取 IP（`parts[1]`），端口取 `appState.defaults.tcpPort`；4. `history.js / connection-detail-panel.js` 重连路径同步修正 |
| **修改次数** | 1 |
| **出现次数** | 1（v0.1.37 发版后被用户反馈） |
| **测试用例** | `__tests__/connect-command.test.ts`（3 用例：前端源码不含 `connect_to_device`；所有 connect 调用使用 peerIp+port 参数；discover-item onClick 提取 IP 而非 deviceId） |
| **关键教训** | 命令名重命名必须同时更新所有调用点，且有静态守卫（grep 禁止出现旧名）防止回归 |

---

## Bug #029：端口默认值混乱（8889 vs 23456）（plan v0.1.38 BUG-021）

| 字段 | 内容 |
|------|------|
| **现象** | 监听端口输入框默认显示 8889，手动连接端口输入框默认也是 8889，但 kvm-core 真源值是 23456；用户保存 23456 后下次打开对话框又弹回 8889；部分路径在 fallback 时又用 23456，整个默认端口体系自相矛盾 |
| **根因** | 早期为了"快速可用"在 `main.js / index.html / history.js` 里写死了 `|| 8889` 和 `value="8889"`，共 8+ 处硬编码；后来 `KvmConfig::default().tcp_port` 改为 23456，但前端硬编码没清，导致"UI 默认值"和"后端真源"两套体系互相打架 |
| **修改方式** | 1. 前端启动流程新增 `initializePortDefaults()`，从 `get_settings` 拉取 `tcp_port` 填入 `appState.defaults.tcpPort`（缺失/为 0 时 fallback 23456）；2. 所有 `|| 8889` 字面量一律改成 `appState.defaults?.tcpPort || 23456`；3. `index.html` 两处 `value="8889"` 删除，由 JS 启动后回填；4. 保存监听端口后同步更新 `appState.defaults.tcpPort`，连接对话框 / 手动连接输入框默认值立即反映新值；5. 新增 `save_settings` 前端 payload 显式构造 snake_case（含 `port`），使后端 AppConfig 能正确接收 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/port-defaults.test.ts`（UI 启动读 get_settings 填默认端口；HTML 不含硬编码 8889；tcp_port==0 时 fallback 23456） |
| **关键教训** | 默认值必须有唯一真源（kvm-core），前端启动必须从后端拉取；字面量禁止写死端口号，统一从 `appState.defaults` 取 |

---

## Bug #030：开启「共享」后两台电脑同时响应键鼠（非真正 KVM）（plan v0.1.38 BUG-022）

| 字段 | 内容 |
|------|------|
| **现象** | 按「开始共享」后，本地键鼠输入会同时作用于两台电脑：对端能看到鼠标光标移动和键盘输入，但本地光标也在动、本地窗口也收到输入。等同于"广播"而不是"KVM 切换"。 |
| **根因** | 两个独立问题叠加：① `MacInputCapture` 的 CGEventTap 使用了 `CGEventTapOptions::ListenOnly`，此模式下回调返回值被系统忽略，根本无法吞事件；② `SharingManager` 没有接入 `EdgeDetector`，鼠标触达屏幕边缘不会自动切换控制权（退化为"仅转发"）。此外 `StateEvent::BlockInput/UnblockInput` 分支是空 `{}`，从未调用 `InputBlocker`。 |
| **修改方式** | 1. **输入屏蔽**：Mac 侧 `CGEventTapOptions::Default` + `Arc<AtomicU8>` 控制状态共享给回调线程，回调按状态返回 `Some(event)`（IDLE 放行）或 `None`（LocalControlling/RemoteControlling 吞掉）；`set_control_state(u8)` 暴露给 SharingManager；Windows Low-Level Hook 对称实现（`HOOK_CONTROL_STATE` thread_local + `should_swallow()`）；2. **边缘切换**：SharingManager 启动 capture 前构建 EdgeDetector（读 LayoutConfig + get_monitors_impl）；MouseMove 分支调用 `detector.detect(x, y)`，`EdgeDetectResult::Triggered` 时触发 `StateMachine::toggle_control` 并处理返回的 StateEvent 序列；3. **BlockInput 实现**：`handle_one` 中 BlockInput → `blocker.block_input()`, UnblockInput → `blocker.unblock_input()`；4. **状态镜像**：StateChanged 事件同步 capture.set_control_state + edge_detector.set_active，保证多线程状态一致 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/kvm-edge-switch.test.ts`（21 用例覆盖：CGEventTap 改 Default、AtomicU8 状态、set_control_state、Windows Hook、EdgeDetector 接入、BlockInput 非空实现、i18n 状态徽标 key、设置页 KVM 分区、snake_case payload）；Rust `sharing::tests::test_block_input_called_on_remote_controlling`、`input_capture::tests::test_idle_state_passes_event_through` |
| **关键教训** | CGEventTap 的 `ListenOnly` vs `Default` 是 macOS 输入拦截的关键区分，文档写得并不显眼，只能靠源码注释和测试守护；跨线程状态共享在回调路径上必须用无锁原语（AtomicU8 + Relaxed），禁止 lock Mutex 以免阻塞实时输入流 |

---

## Bug #031：已授予辅助功能权限后横幅仍提示「需要辅助功能权限」且按钮不可点（v0.1.39 修复）

| 字段 | 内容 |
|------|------|
| **现象** | macOS 上用户在系统设置→隐私与安全性→「辅助功能」里已经打开 KVMini 的开关，但连接成功后的权限横幅仍显示"需要辅助功能权限才能开始键鼠共享，请先前往授权"，且「开始共享」按钮保持禁用状态，用户反复检查辅助功能开关也无法解决 |
| **根因** | `checkPermissionAfterConnect` 同时校验 Accessibility + Input Monitoring 两种权限，任一 Denied 就调 `showPermissionWarning(elements)` 亮横幅，但 `permissionWarningText` 只写了"辅助功能"——当实际缺失的是「输入监控」时用户被完全误导。叠加两个次要问题：① 横幅没有「前往设置」按钮，用户只能按文案去找辅助功能开关（错误的那个）；② `CGPreflightListenEventAccess` 在进程内会缓存结果（`src-tauri/src/platform/macos/permission.rs:108` 已有注释），用户即使发现并授权了输入监控，不完全退出 KVMini 重启也永远 Denied |
| **修改方式** | 1) `src/accessibility.js` 的 `showPermissionWarning(elements)` 签名扩展为 `(elements, kind='accessibility')`，按 kind 切换横幅文案和按钮 `data-kind`；`checkPermissionAfterConnect` 两条分支分别传 `'accessibility'` 和 `'input-monitoring'`；`hidePermissionWarning` 同步隐藏按钮；2) `src/i18n.js` 新增 4 条 key 并中英双语同步：`permissionWarningText`（仅辅助功能版本，措辞明确提到"辅助功能"）、`permissionWarningInputMonitoringText`（含"请完全退出 KVMini（Cmd+Q）再重新打开，权限才会生效"提示）、`permissionWarningBtnAccessibility`、`permissionWarningBtnInputMonitoring`；3) `src/index.html:165` 横幅块新增 `<button id="permission-warning-btn" data-kind="accessibility">`，由 JS 按 kind 切换 `data-kind` 和文案；4) 按钮 `onclick` 按 `data-kind` 路由到后端 `open_accessibility_settings` 或 `open_input_monitoring_settings`（后端零改动，复用已有命令） |
| **修改次数** | 1 |
| **出现次数** | 1（v0.1.38 发版后用户反馈） |
| **测试用例** | `__tests__/permission-warning-kind.test.ts`（11 用例覆盖：zh/en 字典完整性 + 「输入监控」文案必须含"重启"/英文必须含"restart/relaunch" + kind=accessibility/input-monitoring 文案与按钮 data-kind 正确切换 + 不传 kind 默认 accessibility 向后兼容 + hidePermissionWarning 隐藏按钮 + 按钮点击按 kind 调对应 invoke 命令） |
| **关键教训** | 混合多种权限时 UI 文案必须严格对应到具体缺失项，不能用"上位词"兜底；`CGPreflightListenEventAccess` 进程内缓存是物理事实，无法程序化刷新，只能把"必须重启"这一事实透明化告诉用户；按钮的"意图"应当写入 `data-kind` 等 HTML 属性而非隐式状态变量，这样 `onclick` 能通过读 DOM 稳定路由，避免闭包捕获过时值 |

---

## Bug #032：已连接状态下按钮误显示「重连」（v0.1.40 修复）

| 字段 | 内容 |
|------|------|
| **现象** | 连接成功后按钮正确显示"断开连接"，但过几秒后自动变成显示"重连"按钮，用户以为连接已断开，但实际仍处于已连接状态 |
| **根因** | `src/main.js` L454-459 的 `reconnecting` 事件处理器无条件执行 `appState.connected = false` 并调用 `updateConnectionButtons(false)`。而心跳波动（网络瞬间延迟）会触发后端 emit `reconnecting` 事件，即使连接实际并未断开。`updateConnectionButtons(connected)` 直接信任外部传入的 `connected` 参数，没有二次校验 `appState.connected` 的真实状态 |
| **修改方式** | **两层修复（v0.1.40 review 后升级）**：① `updateConnectionButtons` 函数内部改为读取 `appState.connected` 作为单一真相源（single source of truth），忽略外部传入的 `connected` 参数（参数签名保留以兼容现有调用点）；② **根因修复**：`reconnecting` 事件分支不再写 `appState.connected = false`——TCP socket 仍可能存活，仅心跳波动而已；只有真正收到 `disconnected` 事件才置 false。这样确保 reconnecting 心跳波动时按钮文案完全稳定 |
| **修改次数** | 2（首次修在显示层；review 后追加根因层修复） |
| **出现次数** | 1（v0.1.39 使用中发现） |
| **测试用例** | `__tests__/bug-032-reconnect-button.test.js`（4 用例覆盖：appState.connected=true 时传 false 按钮不变 / appState=false 正确切换 / 多次 reconnecting 后按钮稳定 / 真正断连后切换） |
| **关键教训** | **根因优于表象**：第一次只修了"按钮渲染层"是错的——心跳波动会让 `appState.connected` 自身被污染成 false，按钮依然会闪。真正的修复必须让 reconnecting 事件**不修改连接真源**。教训：UI 状态应该由单一真相源驱动，且这个真相源**不能被瞬态事件污染** |

---

## Bug #033：进入「控制远端」后远端鼠标完全不动（v0.1.40 致命修复）

| 字段 | 内容 |
|------|------|
| **现象** | 本机从 Armed 切换到 LocalControlling（本机鼠标被屏蔽、光标隐藏）后，远端鼠标原地不动；松开后本机鼠标能动但同时也能控制远端（两端同时响应，BUG-022 的反向表现） |
| **根因** | `src-tauri/src/platform/macos/input_capture.rs::cg_event_to_raw` 对 `MouseMoved` 事件用 `event.location()` 读取鼠标位置，传入 `RawInputEvent::MouseMove { x, y }` 绝对屏幕坐标。但 `grab()` 调用 `CGAssociateMouseAndMouseCursorPosition(0)` 后，**物理鼠标与屏幕光标位置解耦**——`event.location()` 返回冻结在 grab 瞬间的同一个坐标，永远不变。远端收到的 `MouseMove{x,y}` 是同一个点，`CGWarpMouseCursorPosition` 自然原地不动。竞品（Synergy/Barrier/ShareMouse/Logitech Flow）全部用硬件 delta 而非绝对坐标 |
| **修改方式** | 1) macOS 采集改用 `CGEvent::get_integer_value_field(EventField::MOUSE_EVENT_DELTA_X/Y)` 读硬件位移 delta；2) 协议破坏性升级 `KvmFrame::MouseMove{x,y}` → `MouseMoveDelta{dx:i16,dy:i16}`（opcode 0x33，4 字节 payload），不留绝对坐标兜底，双端必须同时 v0.1.40；3) 新增 `kvm-core/src/remote_cursor.rs::RemoteCursorState` 维护远端虚拟光标位置，`on_delta` 累加 + clamp 屏幕 + 反向边缘检测 + `CGWarpMouseCursorPosition` 落点；4) Windows 侧 WH_MOUSE_LL 用帧差 `dx = x - last_x` 近似硬件 delta；5) `SharingManager.inject_remote_frame` 单独分支处理 MouseMoveDelta，调 `RemoteCursorState::on_delta`，贴反向边缘阈值触发后发 `SwitchBackControl` 回切 Idle |
| **修改次数** | 1 |
| **出现次数** | 1（v0.1.38 + BUG-022 修复后引入，v0.1.40 彻底修复） |
| **测试用例** | `kvm-core/src/remote_cursor.rs` 16 个测试：on_enter 4 方向 / on_delta 累加 / clamp 4 方向 / 反向边缘 4 方向 / i32 溢出防护 / 入态贴边不立即触发 / 离开重置计数；`kvm-core/src/protocol.rs` 5 个 MouseMoveDelta 编解码测试 |
| **关键教训** | 1) 采集端"位置"与"位移"是两个独立的物理量，在光标被 grab/disassociate 的场景下**只有 delta 是可信的**；2) 竞品已用 20+ 年验证的方案不要自创兜底——delta 模式就是 delta 模式，任何"绝对坐标兜底"都是冗余且错误；3) 协议升级时 TCP 可靠传输 + 无累积误差的 i16 delta 比 i32 绝对坐标更紧凑；4) 远端虚拟光标必须是本端（被控方）的**单一 owner**，避免跨进程同步 |

## Bug #034：远端虚拟光标完全不动（v0.1.43 待修复）

| 字段 | 内容 |
|------|------|
| **现象** | 本机撞边进入 LocalControlling 后，远端屏幕上应出现的"虚拟光标"完全不动（用户误以为没连上）。本机光标卡在屏幕边缘只露一半。delta 帧确实到达远端，但 `CGWarpMouseCursorPosition` 调用后光标仍贴回物理鼠标当前位置。|
| **根因** | macOS `MacCursorController::set_position` → `warp_cursor(grabbed=false)` 分支会在 `CGWarpMouseCursorPosition` 之后无条件调用 `CGAssociateMouseAndMouseCursorPosition(1)`（`platform/macos/cursor.rs:140-144`），该调用会**立刻把物理鼠标位置重新映射到光标**，相当于撤销刚才的 warp。被控端（RemoteControlling 态）从未调用 `cursor.grab()`，所以每次 delta warp 后都会走这个 Associate(1) 分支。次要根因：状态机 `switch_to_remote_controlling`（`kvm-core/src/state.rs:177-190`）只产生 `BlockInput`，没有 `ShowCursor`，Windows 上若历史 `ShowCursor` 引用计数为负则光标根本不可见。|
| **修改方式** | (1) `kvm-core/src/state.rs::switch_to_remote_controlling` 追加 `StateEvent::ShowCursor`；对称地从 RemoteControlling 切回 Idle 时 `switch_to_idle` 的 Remote 分支也补 `ShowCursor`。 (2) `sharing.rs::handle_one` 在被控端状态下让 `ShowCursor` 调用 `cursor.show_cursor()`（不是 release，因为没有 grab）。 (3) macOS 端新增"被控模式"标志：在被控端入态时调用 `cursor.begin_remote_display()`（新方法），内部设 `is_grabbed=true` 等价状态让 `warp_cursor` 跳过 `CGAssociate(1)`，但**不**执行 `NSCursor hide`。简化方案：直接让 `MacCursorController` 在 `set_position` 时总是跳过 `CGAssociate(1)`（这个 v0.1.42 的"非 grab 下 Associate(1)"行为实际上没有任何正面作用，只会让 warp 被物理鼠标拉回，属于错误设计，应该奥卡姆剃刀掉）。 (4) Windows 端 `show_cursor` 循环 `ShowCursor(1)` 直到返回值 ≥ 0，抵消负引用计数。|
| **修改次数** | 0（本次首修） |
| **出现次数** | 1 |
| **测试用例** | 新增 `test_switch_to_remote_controlling_produces_show_cursor`（state.rs）、`test_switch_to_idle_from_remote_produces_show_cursor`、`test_set_position_does_not_reassociate`（逻辑层可观测替代） |
| **关键教训** | 奥卡姆剃刀：`warp_cursor(grabbed=false)` 里的 `CGAssociate(1)` 分支（v0.1.42 引入）属于过度设计，没有任何场景会从中受益，反而破坏了被控端的虚拟光标。应直接删除该分支，让 warp 的语义永远是纯 warp。|

---

## Bug #035：远端物理用户键鼠输入无法抢占自动终止共享（v0.1.43 新能力）

| 字段 | 内容 |
|------|------|
| **现象** | 被控端用户想拿回本机控制权时，没有自然的方式：只能让控制端主动撞反向边缘或按快捷键。不符合 Synergy/Logitech Flow/ShareMouse 等竞品的标准用户体验。|
| **根因** | 设计缺失 — 原设计里 `RemoteControlling` 态下 `BlockInput` 吞掉所有本机键鼠，但吞之前没有做"物理输入检测"并主动回切的逻辑。 |
| **修改方式** | (1) `kvm-core/src/preempt.rs` 新增 `PreemptDetector`（无锁 AtomicI64 累加 dx²+dy²，AtomicBool 标记 KeyDown），阈值硬编码 40px（平方=1600）。 (2) `kvm-core/src/protocol.rs::KvmFrame::SwitchBackControl` 扩展为 `{ reason: SwitchBackReason }`，枚举 `EdgeReturn=0` / `Preempt=1`，payload 1 字节；decode 兼容老版本（空 payload=EdgeReturn）。 (3) mac/win 的 input_capture 在 RemoteControlling 态下（且为物理事件：mac 判 `CGEventSourceStateID`；win 判 `LLMHF_INJECTED`）喂 PreemptDetector，触发后通过 `Arc<dyn Fn()>` 回调通知 sharing 层。 (4) `sharing.rs::on_local_physical_preempt()`：发 `SwitchBackControl { reason: Preempt }` → 自切 Idle + UnblockInput + 清理 remote_cursor。 (5) 控制端 `on_remote_switch_back_control` 按 reason emit "kvm://preempted" 事件，前端 `main.js` 监听后 `showToast(t('preempt_resume_toast'), 3000)`，i18n key 中英双语。 |
| **修改次数** | 0（本次首修） |
| **出现次数** | 0（新功能） |
| **测试用例** | `kvm-core/src/preempt.rs`（阈值 39/40 边界、勾股 (30,30)→触发、reset 清零、多次 feed、KeyDown 单次触发）；`kvm-core/src/protocol.rs`（SwitchBackReason 编解码、legacy 0 byte payload 兼容）；前端 i18n 完整性测试同步。 |
| **关键教训** | 奥卡姆剃刀：检测逻辑纯逻辑化放 kvm-core，平台层只做"喂样本 + 合成事件过滤"；触发后被控端自己切 Idle，控制端被动收帧走 local_to_armed，职责清晰不共享状态。 |

---

## Bug #036：Win 被控端断开连接后键鼠锁死（v0.1.44 修复）

| 字段 | 内容 |
|------|------|
| **现象** | Mac 主控端点"停止共享并退出"后，Win 被控端键鼠完全锁死（鼠标键盘都不响应），只能 Ctrl+Alt+Delete 杀进程恢复 |
| **根因** | 多层原因叠加：(1) Win 端 v0.1.43 新增 Hook 层 `should_swallow()`（读 `control_state` 原子变量，RemoteControlling 时返回 1 吞掉所有键鼠事件），这是第一次真正有效的软屏蔽——之前 `BlockInput` API 非管理员权限下静默失败，实际从未屏蔽过；(2) `connection/mod.rs` 事件循环中 `Err(_) => break`——如果 broadcast channel 出现 `RecvError::Lagged`（缓冲区溢出丢事件），事件循环提前退出，后续 `NetworkEvent::ConnectionStateChanged(Disconnected)` 无人消费，`sm.on_disconnected()` 永远不会被调用；(3) `on_disconnected()` 没有兜底保护——如果事件处理链路中任何一步异常（锁竞争、panic），`capture.set_control_state(0)` 和 `blocker.unblock_input()` 可能被跳过 |
| **修改方式** | 三层修复：(1) `connection/mod.rs`：区分 `RecvError::Lagged`（log warn + continue 继续接收）和 `RecvError::Closed`（正常 break），确保 Disconnected 事件不会因 lag 丢失；(2) `sharing.rs::on_disconnected()`：在正常事件处理链路后增加显式兜底——强制 `capture.set_control_state(Idle)` + 检查 `blocker.is_blocked()` 若仍为 true 则强制 `unblock_input()` + 强制 `cursor.recouple_physical_mouse()` + 清理 `remote_cursor`；(3) `platform/windows/input_block.rs`：新增 `force_unblock()` 方法供极端场景使用，并增加模块文档说明 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | 功能测试：Mac 主控端"停止共享并退出" → Win 被控端键鼠必须在 1 秒内恢复响应 |
| **关键教训** | (1) 在"必须恢复"的场景中，不能只依赖事件链路的正确性——必须有**显式兜底**（defense in depth）：事件处理完后再检查一次状态是否真的归零了；(2) broadcast channel 的 Lagged error **绝不能**被当成致命错误处理——丢旧事件 ≠ 系统崩溃，后续的 Disconnected 事件是关键生命周期事件不能丢；(3) 软件工程中"第一次真正生效的屏蔽"会暴露之前从未被发现的恢复路径 bug——新增 BlockInput 能力时必须同步验证所有 UnblockInput 路径 |

---

## Bug #037：被控端 capture（hook/CGEventTap）从未启动，导致抢占失效 + 屏蔽失效 + 两端鼠标错乱（v0.1.55 修复）

| 字段 | 内容 |
|------|------|
| **现象** | (1) Win 被控状态下，本机键鼠操作不能抢回控制权（PreemptDetector 永远不触发）；(2) Mac/Win 被控端的"屏蔽"事实上失效——主控端控制远端时，被控端本机鼠标键盘仍能正常操作本机，造成「主控端 Mac 和被控端 Win 同时各自有鼠标在动」，两端互相错乱 |
| **根因** | 统一根因——被控端从 Idle 直接进入 RemoteControlling 时，`StateMachine::switch_to_remote_controlling()` 产生的事件序列只有 `StateChanged + SendFrame(Ack) + BlockInput + DecoupleCursor`，**缺少 StartCapture**。导致 hook/CGEventTap 从未装上：① Win 端 hook 软屏蔽是吞事件唯一手段，hook 没装 → BlockInput 软屏蔽实际失效；② Mac 端 CGEventTap 在 control_state == REMOTE_CONTROLLING 分支返回 None 是吞事件唯一手段，Tap 没装 → MacInputBlocker 实际失效（注释说"实际屏蔽由 input_capture.rs 完成"）；③ PreemptDetector 依赖 capture 回调喂样本，hook/Tap 没装 → 物理输入抢占永远不触发。**结论**：v0.1.43 引入的"Bug #035 物理输入抢占"在被控端从未真正生效过 |
| **修改方式** | (1) `kvm-core/src/state.rs::switch_to_remote_controlling`：事件序列追加 `StartCapture`，顺序 `StateChanged → SwitchControlAck → StartCapture → BlockInput → DecoupleCursor`（先装 hook 再屏蔽）；(2) `switch_to_idle` 的 RemoteControlling 分支追加 `StopCapture` 对称释放；(3) `src-tauri/src/sharing.rs::inject_remote_frame`：首帧到达时把 `handle_state_events_silent(evs)`（app=None 导致 StartCapture 被忽略）改为 `handle_state_events_with_emit(evs, app)`，确保事件能真实启动 capture；(4) 删除已废弃的 `handle_state_events_silent`（无人使用） |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `kvm-core/src/state.rs` 新增 `test_switch_to_remote_controlling_emits_start_capture`、`test_switch_to_remote_controlling_start_capture_before_block_input`、`test_switch_to_idle_from_remote_emits_stop_capture` |
| **关键教训** | (1) 状态机产生的事件序列**就是契约**——遗漏一个 StartCapture 等于功能事实上不存在，但表现为"间歇性 / 平台差异" bug，极难追踪；(2) silent 版本的事件处理（不传 app）**会悄悄丢弃需要 app 的事件**（如 StartCapture / SendFrame），这种"软静默"反模式必须避免，所有事件处理必须走唯一的链路；(3) 双端共用 kvm-core 状态机 → 一个 bug 必同时影响 Mac/Win，反之单端表现差异时不要先怀疑平台层 |

---

## Bug #038：Mac 主控跨屏拖动到 Win 被控端松开后窗口仍处于拖动状态（v0.1.55 修复）

| 字段 | 内容 |
|------|------|
| **现象** | Mac 主控端按下鼠标左键拖动 → 越屏到 Win 被控端继续拖动 → Mac 物理松开左键 → Win 被控端的窗口仍处于"被拖动"状态（继续随本机鼠标移动），用户必须再点一下才能脱离 |
| **根因** | 主控端按下左键时 pump 任务转发了 MouseDown 帧，被控端注入；跨屏拖动期间撞反向边缘（或 Preempt 抢占、断连）触发 `LocalControlling → Armed/Idle` 状态切换；切换瞬间主控端的 capture 已停止（StopCapture），即便用户随后松开左键，hook/Tap 收到的 MouseUp **不会通过 pump 转发**——但 StateMachine **没有追踪「主控端目前哪些鼠标按键还按着」**，也未在状态切换时给被控端补发 MouseUp 释放帧。结果：被控端注入的 LeftButton Down 永远没有对应的 Up，窗口拖动状态卡死 |
| **修改方式** | (1) `kvm-core/src/state.rs::StateMachine` 新增 `pressed_mouse_buttons: u8` 位掩码字段（bit0=Left, bit1=Right, bit2=Middle, bit3=Side1, bit4=Side2），新增 `track_mouse_button_down/up`、`drain_pressed_mouse_buttons`、`has_pressed_mouse_buttons` 方法；(2) 新增 `StateEvent::ReleaseMouseButtons(Vec<MouseButton>)` 变体；(3) `local_to_armed` / `switch_to_idle` 从 LocalControlling 退出时若有按下按键，drain 后产生 `ReleaseMouseButtons` 事件（位置：StateChanged 后、ReleaseModifiers 前）；(4) `force_idle` 同步清空 pressed_mouse_buttons；(5) `src-tauri/src/sharing.rs::start_capture_and_pump` pump 任务在 `is_local` 分支收到 `KvmFrame::MouseButton{pressed}` 时调 `state.track_mouse_button_*`（含 squash 路径中的非 Move 帧）；(6) `handle_one` 与 `handle_events_on_edge_trigger` 都新增 `ReleaseMouseButtons` 分支，遍历按键发送 `KvmFrame::MouseButton{pressed: false}` 给被控端 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `kvm-core/src/state.rs` 新增 `test_track_mouse_button_down_then_up_cleared`、`test_drain_pressed_mouse_buttons_returns_all`、`test_local_to_armed_with_pressed_buttons_emits_release`、`test_local_to_armed_without_pressed_buttons_no_release`、`test_switch_to_idle_from_local_releases_pressed_buttons`、`test_on_disconnected_from_local_releases_pressed_buttons`、`test_force_idle_clears_pressed_buttons` |
| **关键教训** | (1) 跨进程/跨主机的 Down/Up 配对必须有**主控端补偿**机制——状态切换时主控端 hook 已停，被控端的 Down 永远等不到 Up，唯一能补的就是状态机；(2) 与 `ReleaseModifiers` 设计**对称**：键盘修饰键早就有 release 机制，鼠标按键此前一直缺失，这是设计上的不对称导致的 bug；(3) pump 任务的双路径（squash 与非 squash）必须**同步追踪**所有 MouseButton，否则 squash 路径下的 Down/Up 不计入状态机就会漏发释放帧 |

---

## Bug #040：Win 主控→Mac 被控端 滚轮无响应 + 拖拽无中间过程 + 无法选择文字（v0.1.56 修复）

| 字段 | 内容 |
|------|------|
| **现象** | (1) Win 主控端滚轮操作在 Mac 被控端完全不响应；(2) 拖拽窗口时只有按下和松开有效，中间过程（窗口跟随鼠标移动）不生效；(3) 无法通过拖拽选择文字（LeftMouseDown + MouseMove 不产生选中效果） |
| **根因** | **两个独立根因叠加**：① Mac CGEventTap 在 RemoteControlling 下对所有事件无差别吞掉（包括自注入的合成事件）—— `inject_scroll`/`inject_mouse_button`/`inject_mouse_move` post 的 CGEvent 经过同进程 tap 时全被 `return None` 吞掉，系统从未收到任何注入事件；② `handle_remote_mouse_delta` 使用 `CGWarpMouseCursorPosition` 移动光标 —— Apple 官方文档明确指出此 API "moves cursor WITHOUT generating or posting an event"，不产生 LeftMouseDragged/MouseMoved 事件，macOS WindowServer 无法感知拖拽操作。Win hook 也有同样问题：`should_swallow()` 在 RemoteControlling 下返回 true 时不区分合成/物理事件，但实际表现因 `SetCursorPos` 问题（BUG-039）先暴露 |
| **修改方式** | **(1) Mac CGEventTap 合成事件放行**：在 tap 回调的事件吞/放决策中，将原来的 `LOCAL_CONTROLLING \| REMOTE_CONTROLLING => None` 拆分为两个独立分支—— LocalControlling 仍无差别吞（主控端不注入），RemoteControlling 先检查 `event.get_integer_value_field(K_CG_EVENT_SOURCE_USER_DATA) == KVMINI_INJECT_USER_DATA`，合成事件放行（`Some(event.to_owned())`），物理事件仍吞。**(2) handle_remote_mouse_delta 改用 executor.move_mouse**：去掉 `cursor.set_position(x, y)`（CGWarp），改为 `executor.move_mouse(x, y)`—— Mac 走 `inject_mouse_move`（自动检测左键按下→发 LeftMouseDragged，否则发 MouseMoved），Win 走 `SendInput(MOUSEEVENTF_ABSOLUTE)`（带 INJECTED flag）。**(3) Win hook 合成事件放行**：鼠标和键盘 hook 的 `should_swallow()` 分支中，如果 `is_injected && state == REMOTE_CONTROLLING` 则不吞（放行） |
| **修改次数** | 1 |
| **出现次数** | 1（v0.1.55 BUG-037 修复后 capture 首次启动才暴露；之前 tap/hook 从未启动，注入的事件不经过 tap，问题被掩盖） |
| **测试用例** | `__tests__/bug040-remote-inject-passthrough.test.ts` 共 19 个用例：覆盖 Mac tap 合成事件放行、Win hook 合成事件放行、handle_remote_mouse_delta 使用 move_mouse、LeftMouseDragged 区分、tag_synthetic 链路完整性、Idle/Armed 安全兜底 |
| **关键教训** | (1) 同进程 active CGEventTap 会拦截自己 `event.post(HID)` 注入的事件——这是 macOS 的行为（Apple 文档："event passes through any such event taps"），必须在 tap 回调中用 tag 区分放行；(2) `CGWarpMouseCursorPosition` 只改视觉位置不产生事件——对于需要系统响应的场景（拖拽、选中）必须用 CGEvent post；(3) 竞品 Input Leap/Barrier 的 `OSXScreen::fakeMouseMove` 都是用 `CGEventPost(kCGEventLeftMouseDragged/kCGEventMouseMoved)`，行业标准做法；(4) BUG-037 修复（被控端启动 capture）是正确的，但暴露了"合成事件被自己 tap 吞"这个隐藏 bug——修一个 bug 暴露另一个是正常的级联效应，关键是修复链完整 |

---

## Bug #041：Win 主控→Mac 被控 滚轮无响应（v0.1.59 修复，BUG-040 残留路径补强）

| 字段 | 内容 |
|------|------|
| **现象** | v0.1.58 中 Win 主控端鼠标拖动可用、中键点击可用，但**滚动鼠标滚轮 Mac 被控端窗口完全不响应** |
| **根因** | 静态代码层面 v0.1.57 已对 ScrollWheel 在 RemoteControlling 状态做合成事件 pass-through，链路看似正确。但缺少 `dx == 0 && dy == 0` 的早返回保险（极端边界场景下 Win 滚轮 hook 可能上报 0 delta），且全链路（Win 采集 / sharing 转换 / Mac 注入 / Mac CGEventTap pass-through / 协议编解码）缺少诊断日志，运行时一旦失败无定位手段 |
| **修改方式** | (1) `src-tauri/src/platform/macos/input_inject.rs::inject_scroll` 入口加 `if dx == 0 && dy == 0 { return Ok(()); }` 保险；(2) M1 全链路日志埋点：Win `WM_MOUSEWHEEL/WM_MOUSEHWHEEL` 分支加 debug 日志（delta、is_injected、state），sharing pump 路径专门给 `MouseScroll` 帧加 debug 日志（不受 sample_count%100 限制），SendInput 失败附返回值校验，Mac inject_scroll 加 trace 日志确认 post 已调用；(3) 新增 `__tests__/bug041-scroll-inject.test.ts` 17 个端到端 invariant 守恒测试覆盖 5 层链路 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/bug041-scroll-inject.test.ts` 17 个 |
| **关键教训** | (1) 即使代码静态层面正确，缺乏运行时观测能力的链路 bug 极难定位；(2) 注入函数 entrypoint 必须有 noop 早返回保险，避免上游意外 0 delta 导致的边界 bug；(3) 全链路 invariant 守恒测试是 squash/conversion 路径回归的最强保护 |

---

## Bug #042：连接成功后按钮显示「重连」而非「断开」（v0.1.59 修复）

| 字段 | 内容 |
|------|------|
| **现象** | 连接成功后主界面按钮显示「重连 / Reconnect」而非「断开 / Disconnect」；远端主动断开后按钮文案仍显示「断开」需手动刷新 |
| **根因** | (1) HTML 双按钮模型 `#disconnect-btn` + `#reconnect-btn` 的 hidden 切换由 `connection-status` 事件驱动，`reconnecting` 中间态错误地把 `disconnect-btn` 隐藏并显示 `reconnect-btn`；(2) 后端心跳波动事件 `reconnecting` 触发后 setConnectionStatus 误改按钮文案；(3) 前后端按钮状态没有 single-source-of-truth |
| **修改方式** | 参考 Synergy 3 / Deskflow，重构为单按钮三态：(1) `src/index.html` 删除 disconnect-btn + reconnect-btn 双按钮，改为单一 `#connect-btn[data-state="disconnected"]`；(2) `src/i18n.js` zh/en 都新增 `btnConnecting: '连接中…' / 'Connecting…'`；(3) `src/main.js` appState 新增 `connectionPhase` 字段，新增 `updateConnectButton(phase)` 三态切换函数和 `handleConnectButtonClick` 分发器；setConnectionStatus 末尾同步 connectionPhase + 调 updateConnectButton；(4) reconnecting 事件块只更新状态徽章 className+textContent，**不调** setConnectionStatus 也**不调** updateConnectButton |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/bug042-connect-button-tri-state.test.ts` 18 个 + 重写后的 `bug-032-reconnect-button.test.js` 4 个静态断言 |
| **关键教训** | (1) UI 状态机要做单一真相源：后端 `sharing-state-sync` snapshot 才是按钮状态唯一驱动；(2) 临时性事件（reconnecting）不能误改持久状态；(3) 按钮模型从双按钮 hidden 切换到单按钮 data-state 三态，遵循奥卡姆剃刀，比维护 hidden 互斥逻辑简单一个数量级 |

---

## Bug #043：Win 被控端杀进程后键鼠锁死（v0.1.59 修复，第3次复发）

| 字段 | 内容 |
|------|------|
| **现象** | Mac 远端 KVMini 进程被 kill 后（强杀 / crash / 拔网线 / 关闭窗口），Win 被控端无法使用本机键鼠输入，连 Ctrl+Alt+Tab 安全恢复都无响应 |
| **根因** | 此前修复（BUG-036/037 等）只在 `on_disconnected` 事件路径做兜底，但实际 read_loop 在远端进程被 kill -9 时不一定能立即收到 EOF（特别是 NAT 环境），心跳超时（HEARTBEAT_INTERVAL_SECS × HEARTBEAT_TIMEOUT_COUNT = 5×3 = 15s）窗口太长，期间状态机仍处于 RemoteControlling 完全吞物理输入；StateGuard 应用层 Watchdog 超时阈值 3s 偏短经常误触发被禁用，且无 OS 层兜底 |
| **修改方式** | 纵深防御三层链路：(1) **OS 层 TCP Keepalive**：`kvm-core/src/network.rs` 新增 `apply_tcp_keepalive(stream)` 工具函数（idle 5s + interval 3s × retries 2，约 11s 内 RST），`connect_to` 和 `handle_stream` 双向都应用，失败仅 warn 不阻塞；(2) **应用层 Watchdog**：`src-tauri/src/state_guard.rs` 把 WATCHDOG_TIMEOUT_MS 从 3000 调到 10_000，超时后 trigger_force_idle 调 sm.on_disconnected 触发完整 force_recovery（停 Hook、解 blocker、recouple cursor、清虚拟光标、推送 disconnected snapshot）；(3) **心跳 + 人肉安全阀** 保留作为 L3/L4。三层在 5/10/15s 内逐级生效；(4) 新增 `__tests__/bug043-watchdog-recovery.test.ts` 21 个静态断言 + 2 个 cargo 单元测试 |
| **修改次数** | 4（v0.1.55/56/57 各试过一次，本次 v0.1.59 改为纵深防御） |
| **出现次数** | 4 |
| **测试用例** | `__tests__/bug043-watchdog-recovery.test.ts` + `kvm-core/src/network.rs::test_apply_tcp_keepalive_succeeds_on_live_stream` + `test_keepalive_constants_match_plan_spec` |
| **关键教训** | (1) 安全恢复链路必须有多层独立的兜底，不能依赖单一路径——任何一层失效都不能锁死用户键鼠；(2) TCP Keepalive 是 OS 内核实现，零用户态开销，应作为分布式系统断线感知的标配；(3) Watchdog 超时不能太短（误触发）也不能太长（用户体验差）——10s 是项目实测平衡点；(4) 反复出现 3 次以上的 bug 必须从架构层面解决，不能再"哪里漏了补哪里" |

---

## Bug #044：Win⇄Mac 跨端鼠标切换异常（v0.1.59 修复）

| 字段 | 内容 |
|------|------|
| **现象** | Win 主控端鼠标移到 Mac 被控端后无法切回 Win 控制本机；Mac 端鼠标也无法移动到 Win 切换控制端；整体状态切换乒乓异常 |
| **根因** | (1) **dispatch_state_event 与 handle_one 不对称**：pump 内的边缘触发/抢占路径走 `dispatch_state_event::StateChanged` 分支，缺失 EdgeDetector active 切换 / Watchdog 生命周期管理 / sharing-state-sync snapshot 推送，状态切换"半成品"；(2) **EdgeDetector 累计 hit_count 未清零**：进入 LocalControlling 时 reset_overshoot 缺失，LocalControlling 期间 squash 路径仍喂 detect 累计 hit count，回切到 Armed 后立即触发乒乓；(3) **on_remote_switch_back EdgeReturn 路径无静默期**：v0.1.52 只对 Preempt 加了 800ms 静默期，EdgeReturn 路径主控端鼠标可能仍贴着边缘，回 Armed 瞬间立即重新触发 LocalControlling |
| **修改方式** | (1) `dispatch_state_event` 增加 `edge_detector` + `state_guard` 参数，StateChanged 分支补齐 Watchdog start/stop（被控端 RemoteControlling 入态/离态）+ EdgeDetector::set_active(to == Armed) + reset_overshoot + 入 Armed 时 enter_silence(800) + 推送完整 sharing-state-sync 快照，与 handle_one::StateChanged 行为完全对称；(2) `handle_events_on_edge_trigger` 透传新参数，3 个调用点都更新；(3) pump 入口新增 `state_guard_arc` 和 `edge_detector_for_dispatch` Arc::clone；(4) `on_remote_switch_back` 把 enter_silence(800) + reset_overshoot 从"仅 Preempt"扩展到"所有 reason 通用"；(5) 新增 `__tests__/bug044-cross-end-switch.test.ts` 20 个静态断言覆盖 RC#1/RC#2/RC#3 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/bug044-cross-end-switch.test.ts` 20 个 |
| **关键教训** | (1) 多入口（handle_one + dispatch_state_event）共用同一类 StateEvent 时，**两条路径的副作用必须对称**，否则边缘路径就是隐藏 bug 温床；(2) EdgeDetector 这种带累计计数的状态对象，在状态切换时必须同时考虑 active 开关 + 计数清零 + 静默期三件套，缺一会乒乓；(3) 静默期不能只针对 Preempt——任何回切场景（自然回切 EdgeReturn / 抢占回切 Preempt）的物理鼠标都可能仍贴边，必须通用化 |

---

## Bug #045：Toast 互斥缺失 —— "未连接"和"已连接"同时显示（v0.1.60 修复）

| 字段 | 内容 |
|------|------|
| **现象** | 截图中橙色"未连接"（#reconnect-toast）和绿色"已连接到 30.19.32.36"（#success-toast）同时可见，状态矛盾 |
| **根因** | `showSuccess()` 弹绿色 Toast 时没有清除已显示的橙色 `#reconnect-toast`，反之亦然。两个 Toast 有各自独立的定时器（4s/3s），在网络抖动（先 disconnected 后立即 connected）时会重叠共存 |
| **修改方式** | 三个 Toast 函数（`showSuccess`/`showReconnectToast`/`showError`）入口处统一互斥：弹出自己前先隐藏其他两个 Toast + clearTimeout 对方定时器。`showSuccess` 还额外调 `hideReconnectStatus()` 清除重连进度行。同一时间最多只显示一个 Toast |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `__tests__/bug045-toast-mutex.test.ts` 8 个 |
| **关键教训** | 多个独立的浮层/Toast 元素之间如果表达互斥的语义状态（连接/断连/错误），必须在代码层面做互斥守卫，不能依赖"定时器恰好错开"的运气 |

---

## Bug #046：显示器布局两端不同步 —— 鼠标切过去回不来（v0.1.60 修复）

| 字段 | 内容 |
|------|------|
| **现象** | Mac 作为远端移动到右侧后 Win 没有出现鼠标且无法移动回来；Win 作为远端移动到 Mac 之后无法移动回去 |
| **根因** | 两端的显示器布局（edge direction）完全独立配置（各自读 layout.json），握手时不交换 layout direction。如果两端都配了 LocalLeftOfRemote，鼠标切过去后被控端的 EdgeDetector detect_remote_return() 方向也反了，永远撞不到反向边缘。协议帧中没有 LayoutSync 类型消息 |
| **修改方式** | 类似 Logitech Flow 的方案：(1) `kvm-core/config.rs` 新增 `ScreenLayout::mirror()`（确定性映射 LocalLeft↔LocalRight, LocalAbove↔LocalBelow）+ `to_layout_string()`/`from_layout_string()` 转换函数；(2) `HandshakePayload` 新增 `layout_direction: Option<String>` 字段（`#[serde(default)]` 向后兼容旧版）；(3) `local_handshake_info()` 读取 layout.json 携带到握手包中；(4) 被动方 `handle_stream()` 收到对端 layout 后自动计算镜像值；(5) `NetworkEvent` 新增 `LayoutSynced` 变体；(6) `connection/mod.rs` 事件循环捕获 LayoutSynced → 保存 layout.json + 重建 EdgeDetector + emit `layout-synced` 前端事件；(7) `SharingManager::rebuild_edge_detector()` 用新布局重建边缘检测器；(8) 前端 `main.js` 监听 `layout-synced` 事件，刷新 UI + Toast 提示"已根据对方配置自动调整显示器布局"；(9) i18n 新增 `layoutAutoSynced` 中英文案 |
| **修改次数** | 1 |
| **出现次数** | 1 |
| **测试用例** | `kvm-core/config::tests::test_screen_layout_mirror` + `test_screen_layout_round_trip` + `test_screen_layout_to/from_layout_string` (Rust 9 个) + `network::tests::test_handshake_payload_carries_layout_direction` + `test_handshake_payload_layout_direction_none_compat` (Rust 2 个) + `__tests__/bug046-layout-sync.test.ts` (前端 24 个) = 共 35 个 |
| **关键教训** | (1) P2P 架构下，任何需要"两端一致"的配置都必须在握手时交换/同步，不能依赖用户手动保证；(2) 2 台机器 + 左右方向只有 1 个自由度，镜像是确定性计算（对合映射），不存在冲突；(3) 竞品分析很重要——Logitech Flow 的"发起方为准+自动镜像"方案完美匹配 KVMini 的 P2P 两端模型；(4) `#[serde(default, skip_serializing_if)]` 是协议向后兼容的标准技巧 |

---
