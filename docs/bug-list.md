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
| **现象** | 应用在 macOS 26 beta 上启动后立即崩溃，无法显示主界面 |
| **根因** | `objc2-0.6.4` 的 `MainThreadMarker::new()` 使用 `pthread_main_np() == 1` 检测主线程。macOS 26 beta 在 `applicationDidFinishLaunching` 回调中，`pthread_main_np()` 返回 **`-1`**（线程初始化未完成），而代码只检查 `== 1`，导致返回 `None`，`tao/app_state.rs:288` 的 `.unwrap()` 触发 panic。崩溃链路：`did_finish_launching` → `AppState::launched` → `MainThreadMarker::new().unwrap()` → panic |
| **修改方式** | 1. 将 `objc2-0.6.4` 复制到 `patches/objc2-0.6.4/`；2. 修改 `patches/objc2-0.6.4/src/main_thread_marker.rs` 第 49 行：`pthread_main_np() == 1` → `pthread_main_np() != 0`（符合 Apple 头文件语义：非零即主线程）；3. 在根 `Cargo.toml` 添加 `[patch.crates-io] objc2 = { path = "patches/objc2-0.6.4" }` |
| **修改次数** | 2 |
| **出现次数** | 2 |
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

## 已知预存在的测试失败（与迁移无关）

以下测试在迁移前就已失败，不属于本次迁移引入的 bug：

1. `legacy-cleanup.test.ts` - 测试期望 `main.rs` 包含 `kill_legacy_process` 命令（该命令已被移除）
2. `i18n.test.ts` - 测试期望 `statusDisconnected` 值为 `"断开"`，但实际值为 `"未连接"`（i18n 文本不匹配）
