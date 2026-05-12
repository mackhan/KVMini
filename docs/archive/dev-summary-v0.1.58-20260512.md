# v0.1.58 开发总结（2026-05-12）

## 需求来源

用户要求：
1. 研究竞品手势支持（区分 Mac 控 Win / Win 控 Mac / Mac 控 Mac）
2. 加入共享剪贴板功能，默认开启，带开关
3. Win/Mac 按键区别参考竞品做好映射关系
4. 重构代码，尽量模块化解耦，避免改一处影响其他模块

## 竞品研究结论

**手势**：Synergy/Barrier/InputLeap 均不支持触控板精度手势（3指、Pinch），只传普通滚轮帧。Logitech Flow 通过硬件级别支持，软件 KVM 无法复制。目前 KVMini 滚轮实现已与竞品对齐，暂不升级精度滚动（Win 端无公开 API）。

**剪贴板**：InputLeap/Deskflow 采用混合触发策略：
- 主路径：鼠标离开屏幕时批量发送
- 补充：复制时若当前不活跃立即发送
- KVMini 采用三触发点方案（连接建立/切换控制/实时变化）更全面。

**键位映射**：Deskflow 用 Universal KeyID 中间层；KVMini 用 X11 KeySym（行业标准），更轻量。统一模式默认开启，对主要用户群（Mac 用户）开箱即用。

## 技术方案

### 剪贴板接入路径

```
ClipboardProvider（平台适配层，已有 Mac/Win 实现）
    → ClipboardSync（kvm-core，去重管理）
    → SharingManager.on_connected / sync_clipboard_to_remote / on_remote_clipboard
    → connection/mod.rs（Clipboard 帧分发）
```

### 键位映射路径

```
设置页 key-mapping-unified 开关
    → apply_sharing_settings Tauri 命令
    → SharingManager.set_key_mapping_unified
    → PlatformCapture.set_key_mapping_unified（Arc<AtomicU8>）
    → hook 回调（Win）/ CGEventTap 回调（Mac）无锁读取
    → keycode_map::vk_to_keysym(vk, unified) / cg_cmd_to_keysym_unified(cg_code)
```

### dispatch_state_event 重构

消除 `handle_one` 和 `handle_events_on_edge_trigger` 约 150 行重复代码，统一事件处理逻辑。`handle_one` 保留特有的 Watchdog/EdgeDetector/StartCapture 复杂路径，`handle_events_on_edge_trigger` 简化为循环调用 `dispatch_state_event`。

## 文件改动统计

- 新增代码：~500 行（含测试）
- 删除代码：~250 行（重复/旧函数）
- 净增：~250 行
- 测试：新增 15 个单元测试，全部通过
