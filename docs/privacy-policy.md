# KVMini Privacy Policy / KVMini 隐私政策

*Last updated: 2026-04-29*

---

## English

### Overview

KVMini is a cross-platform keyboard and mouse sharing tool that enables users to share one set of keyboard and mouse across multiple computers on the same local network. We are committed to protecting your privacy and being transparent about how our software works.

### Data Collection

**KVMini does NOT collect, transmit, or store any personal data to external servers.** All operations occur locally on your devices and within your local network.

Specifically:

- **No analytics or telemetry**: We do not track your usage patterns, device information, or any other metrics.
- **No account required**: KVMini does not require user registration or login.
- **No cloud services**: All data remains on your local devices and local network.

### Local Network Communication

KVMini communicates between devices on your local network (LAN) for the following purposes:

- **Device Discovery**: UDP broadcast packets are sent on the local network to discover other KVMini instances. These packets contain only the application name, version, and listening port.
- **Keyboard & Mouse Data**: When connected, keyboard and mouse events are transmitted between devices over TCP. When TLS is enabled (default), all data is encrypted with TLS 1.3.
- **Clipboard Sharing**: When enabled, clipboard text content is synchronized between connected devices over the same encrypted connection.

### Local Data Storage

KVMini stores the following data locally on your device:

- **Settings**: User preferences (hotkey, connection settings, theme, language) are stored in the application's local data directory.
- **Connection History**: Records of previously connected devices (IP addresses, device names) are stored locally for quick reconnection.
- **TLS Certificates**: Self-signed TLS certificates are generated and stored locally for encrypted communication.
- **License Data (macOS)**: Trial period and purchase status are stored in the macOS Keychain.

### Permissions

KVMini requires the following system permissions:

- **Accessibility Permission (macOS)**: Required to capture and inject keyboard/mouse events for cross-device sharing.
- **Network Access**: Required for device discovery and keyboard/mouse data transmission on the local network.

### Auto-Update

KVMini checks for updates via GitHub Releases. This involves:

- An HTTPS request to `github.com` to check for the latest version.
- No personal information is sent during this check.
- Updates are downloaded directly from GitHub Releases.

### Third-Party Services

- **GitHub Releases**: Used for software distribution and auto-update checks.
- **Apple App Store (macOS)**: Used for in-app purchases (if applicable).

### Data Security

- All network communication supports TLS 1.3 encryption (enabled by default).
- TLS certificates use TOFU (Trust On First Use) model with fingerprint verification.
- No data is transmitted outside your local network except for update checks.

### Changes to This Policy

We may update this privacy policy from time to time. Changes will be posted in the application's GitHub repository.

### Contact

If you have questions about this privacy policy, please contact us at: **support@kvmini.app**

---

## 中文

### 概述

KVMini 是一款跨平台键鼠共享工具，允许用户在同一局域网内的多台电脑之间共享一套键盘和鼠标。我们致力于保护您的隐私，并对软件的工作方式保持透明。

### 数据收集

**KVMini 不会向外部服务器收集、传输或存储任何个人数据。** 所有操作均在您的本地设备和局域网内完成。

具体而言：

- **无分析或遥测**：我们不追踪您的使用模式、设备信息或任何其他指标。
- **无需账号**：KVMini 不需要用户注册或登录。
- **无云服务**：所有数据保留在您的本地设备和局域网中。

### 局域网通信

KVMini 在局域网内进行设备间通信，用于以下目的：

- **设备发现**：在局域网内发送 UDP 广播包来发现其他 KVMini 实例。这些数据包仅包含应用名称、版本号和监听端口。
- **键鼠数据**：连接后，键盘和鼠标事件通过 TCP 在设备间传输。启用 TLS 时（默认开启），所有数据均使用 TLS 1.3 加密。
- **剪贴板共享**：启用后，剪贴板文本内容通过相同的加密连接在连接的设备间同步。

### 本地数据存储

KVMini 在您的设备上本地存储以下数据：

- **设置**：用户偏好（快捷键、连接设置、主题、语言）存储在应用程序的本地数据目录中。
- **连接历史**：之前连接过的设备记录（IP 地址、设备名称）存储在本地，用于快速重连。
- **TLS 证书**：自签名 TLS 证书在本地生成和存储，用于加密通信。
- **授权数据（macOS）**：试用期和购买状态存储在 macOS 钥匙串中。

### 权限说明

KVMini 需要以下系统权限：

- **辅助功能权限（macOS）**：用于捕获和注入键盘鼠标事件，实现跨设备共享。
- **网络访问**：用于局域网内的设备发现和键鼠数据传输。

### 自动更新

KVMini 通过 GitHub Releases 检查更新，包括：

- 向 `github.com` 发送 HTTPS 请求以检查最新版本。
- 检查过程中不发送任何个人信息。
- 更新包直接从 GitHub Releases 下载。

### 第三方服务

- **GitHub Releases**：用于软件分发和自动更新检查。
- **Apple App Store（macOS）**：用于应用内购买（如适用）。

### 数据安全

- 所有网络通信支持 TLS 1.3 加密（默认开启）。
- TLS 证书采用 TOFU（首次信任）模型，支持指纹验证。
- 除更新检查外，不会向局域网外传输任何数据。

### 政策变更

我们可能会不时更新本隐私政策。变更将在应用程序的 GitHub 仓库中发布。

### 联系方式

如对本隐私政策有疑问，请联系我们：**support@kvmini.app**
