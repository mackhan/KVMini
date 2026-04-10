# 需求文档：辅助功能权限引导彻底修复

## 引言

辅助功能权限引导问题已反复出现超过 3 次，每次修复都只解决了表面症状，未找到真正根因。本次通过系统化调试（Phase 1 根因调查）发现了 **3 个独立的真实根因**，需要一次性彻底修复。

### 已确认的根因（证据来源：代码阅读）

**根因 A：`main.js` 底部存在第二个 `DOMContentLoaded` 监听器**

```javascript
// main.js 底部（约第1900行）
document.addEventListener('DOMContentLoaded', () => {
    setTimeout(initDisplayLayout, 500);
    setTimeout(checkAccessibilityPermission, 800); // ← invoke 此时为 null！
});
```

这个监听器在 `waitForTauri()` 完成之前就触发，800ms 后调用 `checkAccessibilityPermission` 时 `invoke` 仍为 `null`，权限检测被静默跳过。

**根因 B：`waitForTauri` 超时后 `invoke` 仍为 null，但 `initApp()` 照常执行**

`waitForTauri` 超时 resolve(false) 后，`initApp()` 仍然执行，`invoke = getTauriInvoke()` 仍为 `null`，导致权限检测被跳过。

**根因 C：Rust 返回值与 JS 检测条件存在历史遗留不一致**

- Rust 实际返回：`"已授权"` / `"未授权"` / `"N/A"`（中文）
- JS 检测条件：`status === 'Denied' || status === '未授权'`
- `'Denied'` 是英文，永远不会出现，属于历史遗留的防御性代码，但不是当前主要问题

**根因 D：按钮按了没反应（功能问题）**

用户反映"按键悬浮会变色但是按了没反应"，这说明即使权限引导出现了，底层的 Rust 命令（如 `connect_to_peer`、`start_listening` 等）也可能因为 `invoke` 未就绪而失败，且没有明确的错误提示。

---

## 需求

### 需求 1：彻底修复权限引导触发时序

**用户故事：** 作为一名 macOS 用户，我希望应用启动后立即看到辅助功能权限引导遮罩（如果未授权），以便我能完成授权后正常使用。

#### 验收标准

1. WHEN 应用启动 AND `window.__TAURI__` 就绪 AND 辅助功能权限未授权 THEN 系统 SHALL 显示全屏权限引导遮罩
2. WHEN `waitForTauri()` 完成（无论成功或超时）THEN 系统 SHALL 调用 `checkAccessibilityPermission(invoke)`
3. IF `invoke` 为 null THEN 系统 SHALL 打印警告日志，不静默跳过
4. WHEN `main.js` 底部第二个 `DOMContentLoaded` 监听器中的 `setTimeout(checkAccessibilityPermission, 800)` THEN 系统 SHALL 删除该重复调用（权限检测统一在 `initApp()` 末尾调用）
5. WHEN `initDisplayLayout` 的延迟初始化 THEN 系统 SHALL 改为在 `waitForTauri` 完成后调用，而非独立的 `DOMContentLoaded`

### 需求 2：修复 Rust 返回值与 JS 检测条件不一致

**用户故事：** 作为开发者，我希望权限状态的判断逻辑清晰且不依赖字符串魔法值，以便未来维护时不会因为字符串不匹配导致权限检测失效。

#### 验收标准

1. WHEN Rust 返回权限状态 THEN 系统 SHALL 统一使用英文枚举字符串（`"Granted"` / `"Denied"` / `"NA"`）
2. WHEN JS 检测权限状态 THEN 系统 SHALL 只检查 `status === 'Denied'`，不再依赖中文字符串
3. IF Rust 返回值修改 THEN 系统 SHALL 同步更新 `accessibility.js` 中的检测条件
4. WHEN 权限状态为任何非 `"Granted"` 的值（包括 `"Denied"`、`"NA"`、`"Unknown"`）THEN 系统 SHALL 在 macOS 上显示权限引导遮罩

### 需求 3：修复按钮点击无响应问题

**用户故事：** 作为用户，我希望点击按钮后有明确的反馈（成功或错误提示），以便我知道操作是否生效。

#### 验收标准

1. WHEN 用户点击任意功能按钮 AND `invoke` 为 null THEN 系统 SHALL 显示明确的错误提示（"Tauri 环境未就绪，请重启应用"）
2. WHEN `invoke` 调用后端命令失败 THEN 系统 SHALL 在界面上显示具体错误信息，不静默失败
3. WHEN 应用启动完成 THEN 系统 SHALL 在控制台打印 `invoke` 就绪状态，方便调试

### 需求 4：完善诊断日志

**用户故事：** 作为开发者，我希望应用启动时有完整的诊断日志，以便快速定位权限引导不出现的原因。

#### 验收标准

1. WHEN `waitForTauri()` 完成 THEN 系统 SHALL 打印 `window.__TAURI__` 就绪状态和 `invoke` 获取结果
2. WHEN `checkAccessibilityPermission` 被调用 THEN 系统 SHALL 打印调用时机、`invoke` 状态、Rust 返回值
3. WHEN 权限遮罩显示或隐藏 THEN 系统 SHALL 打印对应日志
4. WHEN `invoke` 为 null 时任何函数被调用 THEN 系统 SHALL 打印包含函数名的警告日志

### 需求 5：回归测试覆盖

**用户故事：** 作为开发者，我希望有自动化测试覆盖所有权限引导场景，以便未来修改不会再次引入回归。

#### 验收标准

1. WHEN 测试运行 THEN 系统 SHALL 验证：`waitForTauri` 完成后 `checkAccessibilityPermission` 被调用
2. WHEN 测试运行 THEN 系统 SHALL 验证：Rust 返回 `"Denied"` 时遮罩显示
3. WHEN 测试运行 THEN 系统 SHALL 验证：Rust 返回 `"Granted"` 时遮罩隐藏
4. WHEN 测试运行 THEN 系统 SHALL 验证：`invoke` 为 null 时打印警告日志
5. WHEN 测试运行 THEN 系统 SHALL 验证：底部第二个 `DOMContentLoaded` 不再包含 `checkAccessibilityPermission` 调用
