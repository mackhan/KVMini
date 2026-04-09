#!/bin/bash
# sign-dev.sh
# 使用自签名证书对 KVMini 开发构建进行代码签名
#
# 用途：每次 cargo build 后运行此脚本，确保可执行文件有稳定的签名 identity，
#       使 macOS TCC 数据库能稳定记录辅助功能权限，避免重新编译后权限失效。
#
# 使用方法：
#   chmod +x scripts/sign-dev.sh
#   cargo build && ./scripts/sign-dev.sh
#
# 或者在 Tauri 开发模式下：
#   npm run tauri dev  （会自动触发 cargo build，之后手动运行此脚本）

set -e

CERT_NAME="KVMini Dev"
# Tauri 开发构建的可执行文件路径
BINARY_PATH="src-tauri/target/debug/kvmini"
# Tauri 应用包路径（如果存在）
APP_BUNDLE_PATH="src-tauri/target/debug/bundle/macos/KVMini.app"

echo "🔏 KVMini 开发构建签名工具"
echo "================================"

# 检查证书是否存在
if ! security find-certificate -c "$CERT_NAME" login.keychain-db &>/dev/null; then
    echo "❌ 未找到证书 \"$CERT_NAME\""
    echo "请先运行：./scripts/create-dev-cert.sh"
    exit 1
fi

# 签名可执行文件
if [ -f "$BINARY_PATH" ]; then
    echo "📦 正在签名可执行文件：$BINARY_PATH"
    codesign --force --sign "$CERT_NAME" \
        --entitlements src-tauri/entitlements.plist \
        "$BINARY_PATH" 2>/dev/null || \
    codesign --force --sign "$CERT_NAME" "$BINARY_PATH"
    echo "✅ 可执行文件签名完成"
else
    echo "⚠️  未找到可执行文件：$BINARY_PATH"
    echo "请先运行：cargo build"
fi

# 签名应用包（如果存在）
if [ -d "$APP_BUNDLE_PATH" ]; then
    echo "📦 正在签名应用包：$APP_BUNDLE_PATH"
    codesign --force --deep --sign "$CERT_NAME" "$APP_BUNDLE_PATH"
    echo "✅ 应用包签名完成"
fi

echo ""
echo "🎉 签名完成！"
echo ""
echo "📌 验证签名："
echo "  codesign -dv \"$BINARY_PATH\""
echo ""
echo "📌 验证辅助功能权限："
echo "  sqlite3 \"/Library/Application Support/com.apple.TCC/TCC.db\" \\"
echo "    \"SELECT * FROM access WHERE service='kTCCServiceAccessibility';\""
echo ""
echo "💡 提示：首次运行应用后，在系统设置 → 隐私与安全性 → 辅助功能中授权一次即可。"
echo "         之后重新编译并运行此脚本签名，权限不会失效。"
