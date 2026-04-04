# 实施计划：i18n 完整性修复 & 扫描功能修复

- [ ] 1. 修复扫描命令名称错误
   - 在 `main.js` 中找到 `handleConnect()` 函数，将 `invoke('connect_devices')` 改为 `invoke('discover_devices')`
   - 检查 `main.js` 中所有 `invoke()` 调用，确保命令名称与后端 Tauri 注册的命令一致
   - 扫描失败时捕获错误并显示友好提示，而非原始技术错误
   - _需求：4.1、4.3、4.4_

- [ ] 2. 在 i18n.js 中补充所有缺失的翻译键
   - 为以下场景新增 zh/en 翻译键：扫描状态（"扫描中..."/"Scanning..."）、角色徽章（"被控端"/"Server"、"主控端"/"Client"）、延迟检测（"检测中..."/"Checking..."）、自动重连（"重连中({n}/3)..."/"Reconnecting({n}/3)..."）、连接断开（"连接已断开"/"Connection lost"）、本机 IP 获取失败（"无法获取"/"Unavailable"）、快捷键录入提示（"请按下快捷键组合..."/"Press a hotkey combination..."）、监听地址后缀（"或其他本机 IP"/"or other local IP"）
   - 为手动连接错误提示新增翻译键：IP 为空、IP 格式错误、端口超范围、连接超时等
   - 为"监听网卡"和"切换控制焦点"功能说明新增翻译键（中英文各一条）
   - 为显示器布局下拉选项（4 个）、布局方向标签、布局提示文字新增翻译键
   - 确保 zh 和 en 的键集合完全一致，无缺失
   - _需求：1.1～1.15、3.1～3.2、5.1～5.4_

- [ ] 3. 修复 main.js 中动态生成 UI 的硬编码中文
   - `updateRoleUI()`：将"被控端"/"主控端"替换为 `t('role_server')`/`t('role_client')`
   - `startLatencyCheck()`：将"检测中..."替换为 `t('latency_checking')`
   - `handleConnectionLost()`：将重连状态文本替换为 `t('reconnecting', {n})`
   - `loadLocalIp()`：将"无法获取"替换为 `t('ip_unavailable')`
   - `hotkeyInput` focus 事件：将"请按下快捷键组合..."替换为 `t('hotkey_prompt')`
   - `connection-lost` 事件处理：将"连接已断开"替换为 `t('connection_lost')`
   - `updateListenAddressDisplay()`：将"或其他本机 IP"替换为 `t('or_other_local_ip')`
   - _需求：1.8～1.14_

- [ ] 4. 修复 main.js 中设备列表/历史记录/Toast 的硬编码中文
   - 扫描按钮状态文本（"扫描中..."/"扫描设备"）替换为 `t('scanning')`/`t('scan_devices')`
   - 设备列表"连接"按钮替换为 `t('connect')`
   - 历史记录"重连"按钮替换为 `t('reconnect')`
   - 历史记录"手动 IP"标签替换为 `t('manual_ip')`
   - 所有 `showSuccess()`/`showError()` 中的硬编码中文替换为对应翻译键（连接成功、断开连接、设置已保存、角色切换成功等）
   - `handleManualConnect()` 中所有错误提示替换为翻译键
   - _需求：1.1～1.7、1.11、1.15_

- [ ] 5. 修复语言选择器显示与实际语言不一致
   - 在 `index.html` 的语言选择器 `<option>` 上添加 `data-i18n` 属性，或在 `applyI18n()` 中动态更新选项文本
   - 确保 `loadSettingsToUI()` 调用后 `languageSelect.value` 与 `appState.settings.language` 保持一致
   - 验证切换语言后选择器显示值与当前语言同步
   - _需求：2.1～2.5_

- [ ] 6. 修复 index.html 中缺少 data-i18n 属性的元素
   - 为显示器布局下拉框（`display-layout-select`）的 4 个 `<option>` 添加 `data-i18n` 属性
   - 为布局方向标签（"本机位于远端的:"）添加 `data-i18n` 属性
   - 为布局提示文字（`display-layout-hint`）添加 `data-i18n` 属性
   - 为显示器布局说明文字（`settings-hint`）添加 `data-i18n` 属性
   - 确保 `applyI18n()` 能正确更新上述元素
   - _需求：5.1～5.5_

- [ ] 7. 在设置面板中为"监听网卡"和"切换控制焦点"添加功能说明文字
   - 在 `index.html` 的"监听网卡"设置项下方添加 `<p class="setting-hint" data-i18n="listen_iface_hint">` 说明元素
   - 在"切换控制焦点"快捷键设置项下方添加 `<p class="setting-hint" data-i18n="hotkey_hint">` 说明元素
   - 在 `style.css`（或内联样式）中为 `.setting-hint` 添加小字号、灰色辅助文字样式
   - _需求：3.1～3.4_

- [ ] 8. 验证所有修复并对照测试用例逐项检查
   - 对照 TC-1 逐项检查语言切换后无中文残留（扫描按钮、连接状态、历史记录、设备列表、角色徽章、延迟、监听地址）
   - 对照 TC-2 验证语言选择器与实际语言同步（打开/关闭设置面板、重启应用）
   - 对照 TC-3 验证功能说明文字显示正确且随语言切换
   - 对照 TC-4 验证扫描功能正常，不出现 `command not found` 错误
   - 对照 TC-5 验证显示器布局设置翻译完整
   - 对照 TC-6 验证手动连接错误提示翻译完整
   - 对照 TC-7 验证 i18n.js 中 zh/en 键集合完全一致
   - _需求：1～5 全部验收标准_
