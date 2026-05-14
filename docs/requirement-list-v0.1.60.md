# KVMini v0.1.60 需求清单

> 基于所有历史版本（v0.1.31 ~ v0.1.59）的提示词、bug 记录、用户反馈汇总整理
> 当前版本：v0.1.59 | 整理日期：2026-05-14

---

## 一、已完成需求（v0.1.31 ~ v0.1.59 累计）

### 基础架构
| # | 需求 | 完成版本 | 关联 Bug |
|---|------|---------|---------|
| F-001 | macOS 26 Tahoe 启动兼容（objc2 patch） | v0.1.31 | BUG-001 |
| F-002 | Tauri 2.x 迁移（emit_all→emit, 配置结构, capabilities） | v0.1.31 | BUG-002/003/004 |
| F-003 | WKWebView 脚本隔离修复（前置内联脚本方案） | v0.1.35 | BUG-010 |
| F-004 | 前端全量 i18n 国际化（zh/en 双语切换） | v0.1.35 | BUG-021/025 |
| F-005 | 主题切换（深色/浅色） | v0.1.35 | BUG-022 |

### 连接与发现
| # | 需求 | 完成版本 | 关联 Bug |
|---|------|---------|---------|
| F-006 | UDP 广播 + 子网 TCP 扫描设备发现 | v0.1.35 | BUG-012/013 |
| F-007 | 多网卡 + VPN 环境发现修复（按物理网卡 bind） | v0.1.38 | BUG-020 |
| F-008 | 手动 IP 连接（connect_to_peer） | v0.1.35 | BUG-028 |
| F-009 | TLS 加密协商（握手携带 tls_enabled，不一致时告警） | v0.1.35 | BUG-018/019 |
| F-010 | 自签名 TLS 证书生成 + SHA-256 指纹显示 | v0.1.35 | BUG-015 |
| F-011 | 心跳保活（Heartbeat + HeartbeatAck 双向） | v0.1.37 | BUG-026 |
| F-012 | TCP Keepalive OS 层断线感知（idle 5s + interval 3s × retries 2） | v0.1.59 | BUG-043 |

### 权限管理
| # | 需求 | 完成版本 | 关联 Bug |
|---|------|---------|---------|
| F-013 | macOS 辅助功能权限检测 + 引导遮罩 | v0.1.35 | BUG-005/006/007/008/009/018 |
| F-014 | macOS 输入监控权限检测 + 分类引导 | v0.1.38 | BUG-027 |
| F-015 | 双权限分类提示（辅助功能 vs 输入监控，含重启提示） | v0.1.39 | BUG-031 |
| F-016 | 稳定自签名证书签名（跨构建保持 codesign identity） | v0.1.38 | BUG-025 |

### 键鼠共享核心
| # | 需求 | 完成版本 | 关联 Bug |
|---|------|---------|---------|
| F-017 | SharingManager + toggle_sharing 按钮接入 | v0.1.38 | BUG-019(#2) |
| F-018 | CGEventTap Default 模式 + AtomicU8 输入屏蔽 | v0.1.40 | BUG-030 |
| F-019 | 纯 delta 鼠标模式（告别绝对坐标，对齐 Synergy/Barrier） | v0.1.40 | BUG-033 |
| F-020 | RemoteCursorState 远端虚拟光标 | v0.1.43 | BUG-034 |
| F-021 | EdgeDetector 边缘检测 + 过冲量防误触 | v0.1.40 | — |
| F-022 | 物理输入抢占回切（PreemptDetector，40px 阈值） | v0.1.43 | BUG-035 |
| F-023 | 被控端 capture 启动修复（CGEventTap/Hook 装上） | v0.1.55 | BUG-037 |
| F-024 | 合成事件放行（Mac tag_synthetic + Win is_injected 判断） | v0.1.56 | BUG-040 |
| F-025 | 跨端鼠标按键释放补偿（ReleaseMouseButtons） | v0.1.55 | BUG-038 |
| F-026 | Win LL Hook 断连兜底恢复（defense in depth） | v0.1.44 | BUG-036 |
| F-027 | Watchdog 应用层安全阀（10s 超时 force_idle） | v0.1.59 | BUG-043 |
| F-028 | 跨端鼠标双向切换稳定（dispatch_state_event 对称化 + 静默期通用化） | v0.1.59 | BUG-044 |

### 键盘 & 剪贴板
| # | 需求 | 完成版本 | 关联 Bug |
|---|------|---------|---------|
| F-029 | X11 KeySym 跨平台键码中间层 | v0.1.51 | — |
| F-030 | 键位映射统一模式（Win↔Mac 修饰键自动转换） | v0.1.58 | — |
| F-031 | 共享剪贴板（跨端文本复制粘贴） | v0.1.58 | — |

### UI/UX
| # | 需求 | 完成版本 | 关联 Bug |
|---|------|---------|---------|
| F-032 | 显示器拖拽布局（mousedown/mousemove 方案） | v0.1.35 | BUG-017/023 |
| F-033 | 单按钮三态连接模型（Connect/Connecting/Disconnect） | v0.1.59 | BUG-042 |
| F-034 | 智能状态面板（合并连接详情 + 加密徽章） | v0.1.38 | BUG-024(#2) |
| F-035 | 设置脏标记红点 + 保存按钮脉冲动画 | v0.1.38 | BUG-026(#2) |
| F-036 | 端口默认值统一（唯一真源 kvm-core tcp_port） | v0.1.38 | BUG-029 |
| F-037 | 全链路诊断日志基础设施（RUST_LOG 分级） | v0.1.59 | — |
| F-038 | Win→Mac 滚轮端到端修复 | v0.1.59 | BUG-041 |

---

## 二、v0.1.60 新增 Bug（待修复）

### BUG-045：Toast 互斥缺失 —— "未连接"和"已连接"同时显示

| 字段 | 内容 |
|------|------|
| **现象** | 截图中橙色"未连接"（#reconnect-toast）和绿色"已连接到 30.19.32.36"（#success-toast）同时可见 |
| **根因** | `showSuccess()` 弹绿色 Toast 时没有清除已显示的橙色 `#reconnect-toast`，反之亦然。两个 Toast 有各自独立的定时器（4s/3s），在网络抖动（先 disconnected 后立即 connected）时会重叠共存 |
| **方案** | showSuccess 时强制隐藏 reconnect-toast + 清除其定时器；showReconnectToast 时强制隐藏 success-toast + 清除其定时器。互斥同时只显示一个。同时 connected 时清除重连进度行 |
| **优先级** | P2（体验问题） |

### BUG-046：显示器布局两端不同步 —— 导致鼠标切过去回不来

| 字段 | 内容 |
|------|------|
| **现象** | Mac 作为远端移动到右侧后 Win 没有出现鼠标且无法移动回来；Win 作为远端移动到 Mac 之后无法移动回去 |
| **根因** | 两端的显示器布局（edge direction）完全独立配置（各自读 layout.json），握手时不交换 layout direction。如果两端都配了 LocalLeftOfRemote，鼠标切过去后被控端的 EdgeDetector detect_remote_return() 方向也反了，永远撞不到反向边缘。协议帧中没有 LayoutSync 类型消息 |
| **方案** | 握手时交换 layout direction + 自动镜像（类似 Logitech Flow）。A 说 LocalLeftOfRemote → B 自动设为 LocalRightOfRemote。如果两端已互补则不改。详见竞品分析 |
| **优先级** | P0（核心功能阻塞） |

---

## 三、v0.1.60 待开发需求

### F-039：Toast 互斥显示（BUG-045 修复）

- **scope**: `src/main.js` — showSuccess / showReconnectToast
- **改动点**:
  1. `showSuccess()` 入口添加：隐藏 `#reconnect-toast` + `clearTimeout(reconnectToastTimer)`
  2. `showReconnectToast()` 入口添加：隐藏 `#success-toast` + `clearTimeout(successToastTimer)`
  3. connected 时 `setConnectionStatus('connected')` 清除重连进度行
- **测试**: Toast 互斥守卫测试

### F-040：显示器布局握手同步（BUG-046 修复，核心功能）

- **方案**: 方案 A — 发起连接方为准 + 被动方静默镜像 + Toast 提示（类似 Logitech Flow）
- **设计决策**:
  - **以谁为准**：发起连接方（主动点"连接"的一端）定义布局，被动方强制接受镜像值
  - **冲突处理**：不存在冲突可能——2台机器只有1个自由度，镜像是确定性计算
  - **用户体验**：被动方 layout 被静默覆盖并保存，UI 自动刷新 + Toast 提示"已根据对方配置自动调整布局"
- **scope**: `kvm-core/src/network.rs` + `kvm-core/src/config.rs` + `src-tauri/src/sharing.rs` + `src-tauri/src/config.rs` + `src/main.js` + `src/i18n.js`
- **改动点**:
  1. **协议扩展**: `HandshakePayload` 新增 `layout_direction: Option<String>` 字段（`#[serde(default)]` 向后兼容旧版）
  2. **kvm-core 镜像函数**: `ScreenLayout::mirror()` 返回确定的镜像值（LocalLeft↔LocalRight, LocalAbove↔LocalBelow）
  3. **握手交换**: `HandshakeInfo` 新增 `layout_direction` 字段；`connect_to()` / `handle_stream()` 发送握手时携带本机 layout
  4. **被动方自动镜像**: `handle_stream()` 解析对端 layout → 调用 `mirror()` → 覆盖本地 `layout.json` 并保存
  5. **NetworkEvent 扩展**: 新增 `LayoutSynced { new_layout: String, was_changed: bool }` 事件
  6. **Tauri 层转发**: `sharing.rs` 监听 `LayoutSynced` → emit `layout-synced` 前端事件
  7. **前端响应**: `main.js` 监听 `layout-synced`，如果 `was_changed=true` 则刷新 UI + 显示 Toast
  8. **i18n**: 新增 `layoutAutoSynced` / `layoutAutoSyncedDetail` 中英文文案
- **测试**:
  - kvm-core: `ScreenLayout::mirror()` 四个方向测试
  - kvm-core: `HandshakePayload` 带/不带 layout_direction 序列化兼容测试
  - 前端: layout-synced 事件处理测试

---

## 四、历史全部需求来源追溯（提示词→需求映射）

> 按时间线追溯每个提示词产生的需求

### v0.1.31 — macOS 26 兼容
- 提示词：macOS Tahoe beta 崩溃 → F-001 objc2 patch
- 提示词：升级 Tauri 2.x → F-002 迁移

### v0.1.35 — 基础功能完整
- 提示词：品牌名 KVM Share → KVMini → BUG-005 品牌修正
- 提示词：权限引导从不出现 → F-013 辅助功能权限
- 提示词：扫描设备一瞬间结束 → F-006 设备发现
- 提示词：WKWebView 脚本隔离 → F-003 启动架构
- 提示词：全面 i18n → F-004 国际化
- 提示词：主题切换 → F-005 主题
- 提示词：TLS 加密 → F-009/010 TLS 体系
- 提示词：连接详情面板 → F-034 智能状态面板

### v0.1.37~v0.1.38 — 连接稳定性
- 提示词：心跳断连 → F-011 心跳保活
- 提示词：输入监控权限 → F-014/015 双权限
- 提示词：VPN 环境扫不到 → F-007 多网卡发现
- 提示词：点连接没反应 → BUG-028/029 命令名+端口修复
- 提示词：开始共享没反应 → F-017 SharingManager 接入
- 提示词：共享后两端同时响应 → F-018 输入屏蔽

### v0.1.40 — 竞品级鼠标控制
- 提示词：远端鼠标完全不动 → F-019 纯 delta 模式
- 提示词：对标 Synergy/Barrier → F-020/021/022 虚拟光标+边缘检测+抢占

### v0.1.43~v0.1.44 — 稳定性增强
- 提示词：虚拟光标不动 → F-020 RemoteCursorState
- 提示词：物理输入抢占 → F-022 PreemptDetector
- 提示词：Win 断连键鼠锁死 → F-026 LL Hook 兜底

### v0.1.51~v0.1.55 — 键盘+跨端完善
- 提示词：Win 控 Mac 键盘不对 → F-029/030 KeySym + 键位映射
- 提示词：被控端屏蔽失效 → F-023 capture 启动修复
- 提示词：拖拽卡死 → F-025 鼠标按键释放补偿

### v0.1.56~v0.1.58 — 注入+剪贴板
- 提示词：滚轮/拖拽/选中不工作 → F-024 合成事件放行
- 提示词：剪贴板共享 → F-031 跨端剪贴板
- 提示词：键位映射 → F-030 统一模式

### v0.1.59 — 稳定性+体验
- 提示词：滚轮无响应 → F-038 滚轮修复
- 提示词：按钮文案错乱 → F-033 三态按钮
- 提示词：杀进程锁死（第4次）→ F-027 Watchdog + F-012 TCP Keepalive
- 提示词：跨端切换乒乓 → F-028 对称化+静默期

### v0.1.60（本次）
- 提示词：截图中"未连接"和"已连接"同时显示 → **BUG-045 Toast 互斥**
- 提示词：Mac 作为远端移到右侧后 Win 回不来 → **BUG-046 布局不同步**

---

## 五、未来 Backlog（低优先级 / 待确认）

| # | 需求 | 来源 | 优先级 |
|---|------|------|--------|
| B-001 | 真实 TLS 数据加密（当前仅握手协商位） | BUG-023 TLS 文案 | P3 |
| B-002 | 上下布局支持（当前仅左右） | 竞品对标 Synergy | P3 |
| B-003 | 3+ 设备支持 | 竞品对标 Synergy | P4 |
| B-004 | macOS 手势跨端传递 | v0.1.58 竞品研究 | P4 |
| B-005 | 跨端文件拖拽传输 | Logitech Flow 对标 | P4 |
| B-006 | 自动发现配对（零配置） | Logitech Flow 对标 | P3 |
| B-007 | 设备连接历史 + 一键重连 | UX 优化 | P3 |
| B-008 | Windows UAC 权限自动提升引导 | v0.1.41 发布总结 | P3 |
| B-009 | 系统托盘常驻 + 开机自启 | UX 基础需求 | P2 |
| B-010 | 屏幕分辨率动态适配（DPI Scaling） | 多显示器场景 | P3 |

---

## 六、测试覆盖统计（v0.1.59）

| 模块 | 测试文件数 | 测试用例数 |
|------|-----------|-----------|
| Jest 前端 | 67 个 .ts/.js | ~400+ |
| Rust kvm-core | state/protocol/remote_cursor/preempt/network | ~80+ |
| Rust src-tauri | sharing/permission/network_scan | ~30+ |
| **总计** | — | **500+** |

---

*本文档由 AI 基于 v0.1.31~v0.1.59 全部提示词、bug 记录、产品设计文档、开发总结自动整理生成。*
