---
name: 修复 Windows 权限弹框问题
overview: 修复 Windows 版本错误显示辅助功能权限弹框的问题。前端主动查询权限状态时未检查 Windows 平台，导致 Rust 返回 "N/A" 时错误显示遮罩。
todos:
  - id: fix-windows-permission-overlay
    content: 修复 src/accessibility.js：添加 Windows 平台检查，避免显示权限遮罩
    status: completed
  - id: run-tests
    content: 运行测试套件验证修复不破坏现有功能
    status: completed
    dependencies:
      - fix-windows-permission-overlay
  - id: rebuild
    content: 重新构建验证修复生效
    status: completed
    dependencies:
      - run-tests
---

## 问题描述

Windows 版本 KVMini 错误地显示了 macOS 辅助功能权限遮罩（弹框）。Windows 系统不需要辅助功能权限，因此不应显示此弹框。

## 问题根因

1. `src/accessibility.js` 第 70-87 行：`initAccessibilityEventListeners()` 函数在启动时主动调用 `invoke('check_accessibility_permission')` 查询权限状态
2. 第 76 行判断 `if (status !== 'Granted')` 会显示权限遮罩
3. Rust 侧 `check_accessibility_permission()` 在非 macOS 平台返回 `"N/A"`
4. `"N/A" !== 'Granted'` 为 true，导致 Windows 上错误显示权限遮罩
5. 对比：`checkPermissionAfterConnect()` 函数第 126 行有 `if (isWindowsPlatform() || !invoke)` 检查，但 `initAccessibilityEventListeners()` 缺少此检查

## 修复目标

在 `initAccessibilityEventListeners()` 函数中添加 Windows 平台检查，Windows 平台直接跳过权限查询和遮罩显示逻辑。

## 修复方案

在第 70 行 `if (invoke) {` 前添加 Windows 平台检查：

```javascript
// Windows 平台不需要辅助功能权限检查
if (isWindowsPlatform()) {
    console.log('[Accessibility] Windows 平台，跳过权限检查');
    return;
}
```

`isWindowsPlatform()` 函数已在文件中定义（第 108-112 行）。

## 技术实现

### 修改文件

**文件**: `src/accessibility.js`

**修改位置**: 第 67-70 行之间（主动请求初始权限状态逻辑之前）

**修改内容**:

```javascript
    // 主动请求初始权限状态
    // 解决时序问题：Rust setup 阶段发送事件时前端 JS 可能还未加载完毕，
    // 事件会被丢弃。因此前端加载完毕后主动查询一次当前权限状态。
    // Windows 平台不需要辅助功能权限检查
    if (isWindowsPlatform()) {
        console.log('[Accessibility] Windows 平台，跳过权限检查');
        return;
    }
    if (invoke) {
        // ... 原有逻辑
    }
```

### 测试验证

1. 运行现有测试套件 `npm test` 确保没有破坏现有功能
2. 验证 Windows 平台逻辑：检查测试覆盖 Windows 平台不显示遮罩的场景

### 构建验证

执行 `npm run build` 验证构建成功