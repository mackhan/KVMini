---
name: update-sdk-and-build-script
overview: 更新 tauri-commerce SDK 到 v0.1.6 并创建带 --reset-purchase 的 build.sh
todos:
  - id: update-sdk-dep
    content: 更新 src-tauri/Cargo.toml 中的 tauri-commerce 依赖，从 tag v0.1.2 改为 commit b8da74f
    status: pending
  - id: update-main-rs
    content: 更新 src-tauri/src/main.rs，在 SDK 初始化中添加 privacy 必填字段（privacy_url、app_description、add_permission）
    status: pending
    dependencies:
      - update-sdk-dep
  - id: create-build-sh
    content: 创建 build.sh 脚本，支持 --reset-purchase 参数清除 Keychain 试用期数据
    status: pending
    dependencies:
      - update-main-rs
  - id: test-build
    content: 运行 build.sh 验证构建成功，并测试 --reset-purchase 参数功能
    status: pending
    dependencies:
      - create-build-sh
---

## 产品概述

KVMini 项目需要更新 tauri-commerce SDK 从 v0.1.2 到最新版本 v0.1.6（commit b8da74f），并创建 `build.sh` 构建脚本，支持 `--reset-purchase` 参数用于开发测试时清除试用期数据。

## 核心功能

### 1. 更新 tauri-commerce SDK 到最新版本

- 当前使用 tag v0.1.2（commit d80858d），最新版本为 v0.1.6（commit b8da74f）
- v0.1.6 新增功能：SMAppService 替代 osascript 实现开机自启动（macOS 13+ 沙盒兼容）、补全 Windows 注册表开机启动实现、新增 update 模块（检查更新）、修复硬编码 + 更新接入指引
- **关键变化**：v0.1.6 的 `privacy` feature 要求必填字段 `privacy_url`、`app_description`、`add_permission`，v0.1.2 没有这些校验

### 2. 创建 build.sh 脚本支持 --reset-purchase

- 默认行为：正常构建项目
- `--reset-purchase` 参数：构建前清除 Keychain 中的试用期数据（用于开发测试）
- macOS 实现：使用 `security` 命令删除 `com.kvmini.app.commerce` 下的 Keychain 条目
- Windows 实现：删除对应注册表项或数据文件

## 技术栈

- Rust + Tauri 2.x
- tauri-commerce SDK v0.1.6（commit b8da74f）
- Shell 脚本（macOS）/ PowerShell（Windows）

## 实现方案

### 整体策略

直接将 `Cargo.toml` 中的 tauri-commerce 依赖从 `tag = "v0.1.2"` 改为指定最新 commit hash `b8da74f`，确保获取到 v0.1.6 的代码。同时更新 `main.rs` 中的 SDK 初始化配置，补充 v0.1.6 要求的 privacy 必填字段。

### 关键技术决策

**1. SDK 版本更新方式**

- 将 `src-tauri/Cargo.toml` 中的 tauri-commerce 依赖从 `tag = "v0.1.2"` 改为 `rev = "b8da74f"`
- 理由：直接指定 commit hash 比 tag 更精确，避免 tag 移动导致意外升级

**2. privacy 配置补充**

- v0.1.6 的 `CommerceConfigBuilder` 新增了 `privacy_url()`、`app_description()`、`add_permission()` 方法
- 必须在 `main.rs` 的 SDK 初始化中添加这些字段，否则 `build()` 会返回错误
- `privacy_url`：指向在线隐私政策页面（可先用占位 URL）
- `app_description`：应用简短描述
- `add_permission`：至少添加一项权限说明（如辅助功能权限）

**3. build.sh 脚本设计**

- 放在项目根目录 `/Users/hanyufei/Documents/Git/KVMini/build.sh`
- 支持参数解析：`--reset-purchase` 清除试用期数据
- macOS 清除逻辑：

```
security delete-generic-password -s com.kvmini.app.commerce -a first_install_timestamp
security delete-generic-password -s com.kvmini.app.commerce -a purchased
security delete-generic-password -s com.kvmini.app.commerce -a last_verify_timestamp
security delete-generic-password -s com.kvmini.app.commerce -a time_jump_detected
```

- 构建完成后自动启动应用

**4. Windows 兼容性**

- `build.sh` 主要面向 macOS 开发，但保留 Windows 适配注释
- Windows 下清除试用期数据：删除 `%APPDATA%\KVMini\` 目录（因为 Windows 版试用期数据存在文件系统而非 Credential Manager）

### 实现注意事项

**性能**：无特殊性能考虑，为构建脚本和配置更新。

**兼容性**：

- 更新 SDK 后需确认 Windows 版本编译通过（v0.1.6 补全了 Windows 注册表开机启动实现）
- `build.sh` 中的 `security` 命令仅 macOS 有效，需做平台判断

**日志**：`build.sh` 输出关键步骤日志，方便调试。

## Agent Extensions

### Skill

- **using-superpowers**
- Purpose: 项目开发前加载 superpowers 工具链
- Expected outcome: 确保开发环境正确配置

- **test-driven-development**
- Purpose: 遵循用户规则严格 TDD
- Expected outcome: 更新 SDK 后所有功能正常

- **verification-before-completion**
- Purpose: 构建完成后验证功能正常
- Expected outcome: 构建成功，应用启动正常