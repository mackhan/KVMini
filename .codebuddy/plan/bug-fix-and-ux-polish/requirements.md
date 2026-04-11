# 需求文档：Bug 修复 & 体验打磨

## 引言

本次迭代来源于用户实际使用中发现的 6 类问题，包含 **3 个 Bug 修复** 和 **3 个功能完善**：

1. **[Bug] 保存设置报错** - `save_settings` 命令参数键名不匹配，导致保存失败
2. **[Bug] 显示器拖拽方向错误** - 拖拽后布局方向计算逻辑有误，左右方向判断反了
3. **[Bug] 辅助权限检测失效** - Rust 侧未主动发送权限事件，导致遮罩永远不显示，权限状态显示不准确
4. **[功能] tauri-commerce SDK 集成** - 重新调研并正确集成 SDK，实现应用授权/购买功能
5. **[功能] TLS 加密不一致处理** - 一端开启 TLS 另一端未开启时，需要明确的错误提示和引导
6. **[功能] 连接状态详情显示** - 用户无法知道两端当前的连接方式（IP、端口、加密状态等）

---

## 需求

### 需求 1：[Bug] 修复保存设置报错

**用户故事：** 作为用户，我希望点击"保存设置"后能成功保存，而不是看到报错弹窗。

#### 背景

错误信息：`invalid args 'config' for command 'save_settings': command save_settings missing required key config`

根因：`main.js` 第 1056 行调用 `invoke('save_settings', { settings: appState.settings })`，传入的键名是 `settings`；而 Rust 后端 `save_settings(config: AppConfig)` 期望的键名是 `config`。键名不匹配导致 Tauri 报错。

#### 验收标准

1. WHEN 用户点击"保存设置"按钮 THEN 系统 SHALL 成功调用 `save_settings` 命令，不报错
2. WHEN `save_settings` 调用成功 THEN 系统 SHALL 显示"保存成功"提示
3. IF `save_settings` 调用失败 THEN 系统 SHALL 显示具体错误信息
4. WHEN 修复后 THEN 系统 SHALL 通过回归测试，确认 `save_settings` 参数键名为 `config`

---

### 需求 2：[Bug] 修复显示器拖拽方向错误

**用户故事：** 作为用户，我希望拖拽显示器图标后，布局方向能正确反映我的拖拽意图。

#### 背景

当前 `calcLayoutFromDrop` 函数中，当 `dx < 0`（本机在远端左侧）时，返回 `LocalLeftOfRemote`，但实际应该是 `LocalRightOfRemote`（本机在远端右侧意味着远端在本机左侧，鼠标从本机右边界穿出到远端）。

同时，旧版拖拽逻辑（降级代码）中方向判断也是反的：`dx < 0 ? 'LocalRightOfRemote' : 'LocalLeftOfRemote'`，与新版逻辑不一致。

需要与用户对齐布局方向的语义定义：
- `LocalLeftOfRemote`：本机显示器在远端显示器的**左侧**，鼠标从本机**右边界**穿出到远端
- `LocalRightOfRemote`：本机显示器在远端显示器的**右侧**，鼠标从本机**左边界**穿出到远端
- `LocalAboveRemote`：本机显示器在远端显示器的**上方**，鼠标从本机**下边界**穿出到远端
- `LocalBelowRemote`：本机显示器在远端显示器的**下方**，鼠标从本机**上边界**穿出到远端

#### 验收标准

1. WHEN 用户将本机显示器图标拖到远端图标**左侧**放下 THEN 系统 SHALL 设置布局为 `LocalLeftOfRemote`
2. WHEN 用户将本机显示器图标拖到远端图标**右侧**放下 THEN 系统 SHALL 设置布局为 `LocalRightOfRemote`
3. WHEN 用户将本机显示器图标拖到远端图标**上方**放下 THEN 系统 SHALL 设置布局为 `LocalAboveRemote`
4. WHEN 用户将本机显示器图标拖到远端图标**下方**放下 THEN 系统 SHALL 设置布局为 `LocalBelowRemote`
5. WHEN 拖拽结束 THEN 系统 SHALL 在预览区域实时更新显示器相对位置图示
6. WHEN 拖拽结束且位置有效 THEN 系统 SHALL 自动保存布局配置
7. WHEN 修复后 THEN 系统 SHALL 通过 `layout-drag.test.ts` 中所有测试用例

---

### 需求 3：[Bug] 修复辅助功能权限检测失效

**用户故事：** 作为用户，我希望应用能准确检测我是否已授予辅助功能权限，而不是一直显示"有权限"。

#### 背景

当前架构：Rust 侧应在启动时检测权限并发送 `accessibility://permission-denied` 或 `accessibility://permission-granted` 事件，前端监听事件被动响应。

问题：Rust 侧的权限检测事件发送逻辑可能未正确实现或未在启动时触发，导致前端永远收不到 `permission-denied` 事件，遮罩永远不显示，用户看到的是"一直有权限"。

需要检查：
1. Rust 侧是否在应用启动时主动调用 `check_accessibility()` 并发送对应事件
2. 事件名称是否与前端监听的名称完全一致
3. 权限轮询线程是否正确启动

#### 验收标准

1. WHEN 应用启动且辅助功能权限未授予 THEN 系统 SHALL 显示辅助功能权限引导遮罩
2. WHEN 应用启动且辅助功能权限已授予 THEN 系统 SHALL 不显示遮罩，直接进入主界面
3. WHEN 用户在系统设置中授予权限后返回应用 THEN 系统 SHALL 在 2 秒内自动隐藏遮罩
4. WHEN 权限状态发生变化 THEN 系统 SHALL 通过 Tauri 事件实时通知前端
5. IF 权限检测 API 调用失败 THEN 系统 SHALL 记录日志并默认显示遮罩（安全降级）
6. WHEN 修复后 THEN 系统 SHALL 通过 `accessibility-event-driven.test.ts` 中所有测试用例

---

### 需求 4：[功能] 正确集成 tauri-commerce SDK

**用户故事：** 作为产品所有者，我希望正确集成 tauri-commerce SDK，以便为 KVMini 实现应用内购买或授权验证功能。

#### 背景

SDK 地址：https://github.com/mackhan/tauri-commerce

需要在开发前：
1. 阅读 SDK README，了解安装方式（npm 包 / Rust crate / 两者都有）
2. 了解 SDK 提供的 API（授权检查、购买流程、License Key 验证等）
3. 与用户对齐授权模式（App Store 内购 / License Key / 订阅制）

#### 验收标准

1. WHEN 应用启动 THEN 系统 SHALL 通过 tauri-commerce SDK 检查授权状态
2. IF 用户未授权 THEN 系统 SHALL 展示购买/激活引导界面（使用 SDK 提供的 UI 或自定义 UI）
3. IF 用户已授权 THEN 系统 SHALL 正常进入主界面，不显示购买提示
4. WHEN 用户点击购买/激活 THEN 系统 SHALL 调用 tauri-commerce 提供的支付/激活流程
5. WHEN SDK 初始化失败（网络异常等）THEN 系统 SHALL 显示友好错误提示，不阻塞主界面启动
6. IF 授权状态发生变化 THEN 系统 SHALL 实时更新 UI 状态（无需重启）

---

### 需求 5：[功能] TLS 加密不一致时的处理与提示

**用户故事：** 作为用户，我希望当两端 TLS 加密配置不一致时，能收到明确的提示和引导，而不是静默连接失败。

#### 背景

场景分析：
- **本机开 TLS，对端未开**：本机发起 TLS 握手，对端以明文响应，握手失败，连接断开
- **本机未开 TLS，对端开了**：对端期望 TLS 握手，本机发送明文，对端拒绝连接
- **两端都开 TLS**：正常加密连接
- **两端都未开 TLS**：正常明文连接（不推荐，但可用）

#### 验收标准

1. WHEN 连接失败且原因为 TLS 握手错误 THEN 系统 SHALL 显示"加密配置不一致"提示，而非通用连接失败
2. WHEN 显示 TLS 不一致提示 THEN 系统 SHALL 同时显示本机当前 TLS 状态（开/关）
3. WHEN 显示 TLS 不一致提示 THEN 系统 SHALL 提供"关闭本机 TLS 重试"和"提示对端开启 TLS"两个操作按钮
4. IF 本机未开 TLS 但对端开了 TLS THEN 系统 SHALL 在连接前检测并提示用户开启 TLS
5. WHEN 两端 TLS 状态不一致 THEN 系统 SHALL 在连接状态区域显示警告图标
6. WHEN 用户点击"关闭本机 TLS 重试" THEN 系统 SHALL 自动关闭 TLS 并重新发起连接

---

### 需求 6：[功能] 连接状态详情面板

**用户故事：** 作为用户，我希望能看到当前两端的连接详情（IP、端口、加密状态、延迟等），以便了解连接质量和排查问题。

#### 验收标准

1. WHEN 连接成功 THEN 系统 SHALL 在主界面显示连接详情区域，包含：
   - 本机 IP 和监听端口
   - 对端 IP 和端口
   - 加密状态（TLS 开/关，以及证书指纹前 8 位）
   - 连接延迟（RTT，毫秒）
2. WHEN 连接断开 THEN 系统 SHALL 隐藏连接详情区域或显示"未连接"状态
3. WHEN 连接延迟超过 100ms THEN 系统 SHALL 以黄色显示延迟数值
4. WHEN 连接延迟超过 300ms THEN 系统 SHALL 以红色显示延迟数值并显示警告
5. WHEN 用户点击连接详情区域 THEN 系统 SHALL 展开显示完整的连接信息（包含完整证书指纹）
6. WHEN 加密状态为"未加密" THEN 系统 SHALL 显示警告图标提示用户开启 TLS

---

### 附：网卡说明 UI 优化（补充需求 2 的 UI 细节）

**用户故事：** 作为用户，我希望在设置页看到清晰的网卡说明，理解"为什么有两个网卡"。

#### 验收标准

1. WHEN 用户打开"网络"分页 THEN 系统 SHALL 在网卡选择器上方显示说明文字：
   > "您的设备有多个网络接口（如 Wi-Fi、有线网、VPN 等），选择'所有网卡'可接受来自任意网络的连接；选择具体网卡可限制只接受该网络的连接。"
2. WHEN 网卡列表中有多个网卡 THEN 系统 SHALL 在每个网卡条目旁显示网卡类型图标（Wi-Fi / 有线 / VPN）
3. WHEN 用户悬停在网卡条目上 THEN 系统 SHALL 显示该网卡的详细信息（接口名称、MAC 地址、连接类型）
