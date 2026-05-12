# KVMini v0.1.58 CHANGELOG

> 发布日期：2026-05-12
> 主题：共享剪贴板 + 键位映射统一模式 + sharing.rs 重构

---

## 新增功能

### 1. 共享剪贴板（默认开启）

竞品对齐（InputLeap/Deskflow 设计）：

| 触发时机 | 说明 |
|---------|------|
| 连接建立时 | 立刻同步本机剪贴板一次 |
| 控制权切换时 | 主控端将剪贴板推给被控端 |
| 本机复制时（LocalControlling） | 实时监听变化立即推送 |

- 大小上限 10MB，超限自动跳过
- hash 去重，防止回环同步
- 前端设置页开关（默认开），支持运行时热切换（无需重启）

### 2. 键位映射统一模式（默认开启）

| 场景 | 统一模式（开） | 透明模式（关） |
|------|--------------|--------------|
| Mac 控 Win | Cmd+C → Win 端 Ctrl+C | Cmd → Win键（保留原始） |
| Win 控 Mac | Ctrl+C → Mac 端 Cmd+C | Ctrl → Ctrl（保留原始） |
| Mac 控 Mac | Cmd → Cmd（不变） | 同左 |
| Win 控 Win | Ctrl → Ctrl（不变） | 同左 |

- 前端设置页开关（默认开），支持运行时热切换
- 实现层：Mac 采集端 CGKeyCode(54/55) → XK_Control_L/R；Win 采集端 VK_LCONTROL → XK_Super_L

### 3. Win→Mac 键位修复

- 新增 `kvm_core::protocol::keycode_map::vk_to_keysym()` 函数
- Win 主控端采集时将 VK 转换为 X11 KeySym 协议键码
- 修复之前 Win 控 Mac 时部分特殊键（符号、扩展键）无法注入的问题

---

## 重构

### sharing.rs 重构

- 新增 `dispatch_state_event()` 统一事件处理函数
- `handle_events_on_edge_trigger` 改为调用 `dispatch_state_event`，消除 ~150 行重复代码
- `SharingManager` 新增字段：`clipboard_provider`、`clipboard_sync`
- 新增公共方法：`on_connected`、`on_remote_clipboard`、`sync_clipboard_to_remote`、`set_clipboard_sync_enabled`、`set_key_mapping_unified`
- 新增 Tauri 命令：`apply_sharing_settings`（设置保存后运行时热切换）

---

## 改动文件

```
kvm-core/src/protocol/keycode_map.rs   新增 vk_to_keysym() + cg_cmd_to_keysym_unified() + 11个测试
kvm-core/src/config.rs                  新增 key_mapping_unified 字段（默认 true）
src-tauri/src/platform/windows/input_capture.rs  新增 key_mapping_unified Arc<AtomicU8>，改用 keycode_map::vk_to_keysym
src-tauri/src/platform/macos/input_capture.rs    新增 key_mapping_unified Arc<AtomicU8>，cg_event_to_raw 支持统一模式
src-tauri/src/sharing.rs                剪贴板接入 + dispatch_state_event 重构 + 新 Tauri 命令
src-tauri/src/connection/mod.rs         接入 Clipboard 帧 + Connected 状态调 on_connected
src-tauri/src/main.rs                   注册 apply_sharing_settings 命令
src/index.html                          新增键位映射开关 UI，版本号 0.1.58
src/i18n.js                             新增键位映射中英文翻译
src/settings.js                         新增 keyMappingUnified 字段
src/main.js                             注册 key-mapping-unified DOM 元素，保存设置时调用 apply_sharing_settings
src-tauri/tests/tauri2_migration_test.rs  AppConfig 补全 key_mapping_unified 字段
Cargo.toml / package.json               版本 0.1.57 → 0.1.58
```

---

## 测试覆盖

### kvm-core 新增测试（11 个）

| 测试 | 覆盖点 |
|------|-------|
| `test_vk_to_keysym_alpha_transparent` | 字母键 VK→KeySym 透明模式 |
| `test_vk_to_keysym_special_transparent` | 特殊键透明模式 |
| `test_vk_to_keysym_modifiers_transparent` | 修饰键透明模式 |
| `test_vk_to_keysym_unified_ctrl_becomes_cmd` | 统一模式 Ctrl→Cmd |
| `test_vk_to_keysym_unified_other_keys_unchanged` | 统一模式其他键不变 |
| `test_cg_cmd_to_keysym_unified` | Mac Cmd→Ctrl KeySym |
| `test_vk_to_keysym_arrows` | 方向键映射 |
| `test_vk_to_keysym_fn_keys` | F 功能键映射 |

### kvmini（Win 端）新增测试（4 个）

| 测试 | 覆盖点 |
|------|-------|
| `test_key_mapping_unified_default_on` | 默认开启 |
| `test_set_key_mapping_unified` | 开关切换 |
| `test_unified_mode_ctrl_becomes_cmd` | 统一模式验证 |
| `test_transparent_mode_ctrl_stays_ctrl` | 透明模式验证 |

### 回归

- `cargo test --workspace`：全部通过
- `cargo check --workspace`：0 error，1 warning（预存在）

---

## 架构说明

```
用户保存设置 (clipboardSync=true, keyMappingUnified=true)
    │
    ├─ 前端 save_settings → 后端 config.json 持久化
    │
    └─ 前端 apply_sharing_settings → sharing.rs 运行时热切换（无需重启）
            ├─ clipboard_sync.set_enabled()
            └─ capture.set_key_mapping_unified()
                    ├─ Mac: key_mapping_unified Arc<AtomicU8>
                    │       → cg_event_to_raw 读取 → Cmd CGKeyCode → XK_Control
                    └─ Win: key_mapping_unified Arc<AtomicU8>
                            → hook 回调读取 → VK_LCONTROL → XK_Super_L
```
