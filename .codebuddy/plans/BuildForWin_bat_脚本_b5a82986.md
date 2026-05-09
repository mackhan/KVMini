---
name: BuildForWin bat 脚本
overview: 在项目根目录创建 BuildForWin.bat，一键完成测试、版本更新、构建，并自动拉起构建好的 kvmini.exe 运行。
todos:
  - id: create-build-bat
    content: 创建 BuildForWin.bat：测试→版本更新→构建→拉起运行，含彩色输出和错误处理
    status: completed
---

## 用户需求

在项目根目录创建 `BuildForWin.bat`，一键完成 Windows 版本的构建并自动拉起运行。

## 产品概述

一个 Windows 批处理脚本，按顺序执行：测试 → 版本更新 → 构建 → 启动，任何步骤失败立即停止并给出清晰提示。

## 核心功能

- 彩色分阶段输出（利用 Windows ANSI 转义或标题提示区分步骤）
- 依次执行：`npm test` → `node scripts/version-update.js` → `npm run build` → 启动 `target\release\kvmini.exe`
- 每步失败时打印错误信息、暂停等待用户确认后退出
- 构建成功后显示版本号和产物路径，再拉起 exe

## 技术栈

- Windows Batch Script (`.bat`)，使用 `cmd /v` 延迟变量扩展
- 利用 `PowerShell -Command` 读取 `package.json` 中的版本号（无需额外依赖）
- ANSI 颜色通过 `echo` + ESC 序列（Windows 10+ 终端原生支持）

## 实现方案

脚本核心逻辑：

1. 开启 `ANSI` 支持（`reg add HKCU\Console /v VirtualTerminalLevel /t REG_DWORD /d 1 /f`，静默执行，已支持则无副作用）
2. 封装 `:check_error` 子程序，每步后调用，非零退出码则打印红色错误、`pause` 后 `exit /b 1`
3. 从 `package.json` 读取版本号用于构建完成后的提示

## 目录结构

```
KVMini/
└── BuildForWin.bat   # [NEW] 一键构建并运行脚本
```

## 关键实现细节

- `setlocal enabledelayedexpansion` 用于变量在代码块内正确展开
- 每个命令后立即 `if %errorlevel% neq 0 goto :error_exit`
- 构建完成后用 `start "" "target\release\kvmini.exe"` 非阻塞拉起应用
- 显示当前时间戳方便排查构建耗时