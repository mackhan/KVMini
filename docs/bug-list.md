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

## 已知预存在的测试失败（与迁移无关）

以下测试在迁移前就已失败，不属于本次迁移引入的 bug：

1. `legacy-cleanup.test.ts` - 测试期望 `main.rs` 包含 `kill_legacy_process` 命令（该命令已被移除）
2. `i18n.test.ts` - 测试期望 `statusDisconnected` 值为 `"断开"`，但实际值为 `"未连接"`（i18n 文本不匹配）
