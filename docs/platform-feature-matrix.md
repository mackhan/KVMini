# 跨平台功能对照表

> 最后更新：2026-04-03  
> 基于代码审计：`src-tauri/src/main.rs`、`src/main.js`、`src/index.html`

---

## 一、Tauri 后端命令（`#[tauri::command]`）

| 命令名称 | 所在文件 | macOS | Windows | 说明 |
|---------|---------|-------|---------|------|
| `greet` | `main.rs` | ✅ | ✅ | 测试命令，两端均有 |
| `discover_devices` | `connection_manager.rs` | ✅ | ✅ | 发现所有设备（网络+USB） |
| `connect_to_device` | `connection_manager.rs` | ✅ | ✅ | 连接到指定设备 |
| `disconnect_devices` | `connection_manager.rs` | ✅ | ✅ | 断开连接 |
| `get_connection_status` | `connection_manager.rs` | ✅ | ✅ | 获取连接状态 |
| `discover_network_devices` | `network.rs` | ✅ | ✅ | 发现局域网设备 |
| `connect_network_device` | `network.rs` | ✅ | ✅ | 通过网络连接 |
| `scan_usb_devices` | `usb.rs` | ✅ | ✅ | 扫描USB设备 |
| `connect_usb_device` | `usb.rs` | ✅ | ✅ | 通过USB连接 |
| `show_tray_notification` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：连接/断开时系统通知 |
| `get_connection_history` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：获取连接历史记录 |
| `save_connection_history` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：保存连接历史记录 |
| `clear_connection_history` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：清除连接历史记录 |
| `get_settings` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：读取应用设置 |
| `save_settings` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：保存应用设置 |
| `register_global_hotkey` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：注册全局快捷键 |
| `kill_legacy_process` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：清理 kvnshare 遗留进程 |
| `get_latency` | ❌ 缺失 | ❌ | ❌ | **两端均缺失**：实时延迟检测（当前为模拟值） |

---

## 二、系统托盘（Tray）

| 功能 | macOS | Windows | 说明 |
|------|-------|---------|------|
| 系统托盘图标 | ❌ 缺失 | ❌ 缺失 | **两端均缺失**：需在 `main.rs` 中配置 `SystemTray` |
| 托盘菜单：显示/隐藏窗口 | ❌ 缺失 | ❌ 缺失 | 需要 `tauri::SystemTrayMenu` |
| 托盘菜单：快速连接/断开 | ❌ 缺失 | ❌ 缺失 | 需要 `tauri::SystemTrayMenu` |
| 托盘菜单：退出 | ❌ 缺失 | ❌ 缺失 | 需要 `tauri::SystemTrayMenu` |
| 托盘图标状态变化（连接中/已连接） | ❌ 缺失 | ❌ 缺失 | 需要动态更换托盘图标 |

---

## 三、前端界面功能

| 功能模块 | macOS | Windows | 说明 |
|---------|-------|---------|------|
| 主界面布局（设备列表、状态、按钮） | ✅ | ✅ | 两端共用同一套前端代码 |
| 连接状态显示 | ✅ | ✅ | 已实现 |
| 连接方式显示 | ✅ | ✅ | 已实现（但未更新值） |
| 延迟显示 | ⚠️ 模拟 | ⚠️ 模拟 | 当前为硬编码 `5ms`，非真实值 |
| 设备列表展示 | ✅ | ✅ | 已实现 |
| 设备扫描动画 | ❌ 缺失 | ❌ 缺失 | **两端均缺失**：扫描时无动画提示 |
| 设置面板（完整） | ❌ 缺失 | ❌ 缺失 | **两端均缺失**：当前为 `alert('设置功能开发中...')` |
| 快捷键配置 UI | ❌ 缺失 | ❌ 缺失 | **两端均缺失** |
| 连接历史记录列表 | ❌ 缺失 | ❌ 缺失 | **两端均缺失** |
| 快速重连按钮 | ❌ 缺失 | ❌ 缺失 | **两端均缺失** |
| 系统通知（连接/断开提示） | ❌ 缺失 | ❌ 缺失 | **两端均缺失** |
| 错误提示 UI（非 alert） | ❌ 缺失 | ❌ 缺失 | **两端均缺失**：当前用 `alert` 显示错误 |

---

## 四、平台隔离现状

| 项目 | 现状 | 目标 |
|------|------|------|
| Rust 条件编译 | ❌ 无任何 `#[cfg(target_os)]` | 系统托盘、通知、快捷键需要平台隔离 |
| 前端平台判断 | ❌ 无 | 通过 `platform.js` 抽象层统一封装 |
| `tauri.conf.json` 平台配置 | ✅ 已有 macOS/Windows 分区 | 继续完善 |

---

## 五、遗留问题

| 问题 | 平台 | 说明 |
|------|------|------|
| `kvnshare` 遗留进程 | macOS | 早期版本进程在后台运行，需清理 |
| `window.__TAURI__` 解构错误 | 两端 | ✅ 已修复（任务1） |

---

## 六、待补全功能优先级

| 优先级 | 功能 | 对应任务 |
|--------|------|---------|
| P0 | 设置面板（替换 alert） | 任务3 |
| P0 | 系统托盘（两端） | 任务4 |
| P0 | 系统通知（连接/断开） | 任务4 |
| P1 | 全局快捷键 | 任务4 |
| P1 | 连接历史记录 | 任务6 |
| P1 | 设备扫描动画 | 任务3 |
| P1 | 真实延迟检测 | 任务4 |
| P2 | 平台抽象层 `platform.js` | 任务5 |
| P2 | kvnshare 遗留进程清理 | 任务7 |
