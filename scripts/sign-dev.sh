#!/bin/bash
# sign-dev.sh
# 使用自签名证书对 KVMini 构建进行代码签名
#
# 用途：每次构建后运行此脚本，确保可执行文件有稳定的签名 identity，
#       使 macOS TCC 数据库能稳定记录辅助功能权限，避免重新编译后权限失效。
#
# 使用方法：
#   chmod +x scripts/sign-dev.sh
#   ./scripts/sign-dev.sh              # 自动检测 release 或 debug
#   ./scripts/sign-dev.sh --release    # 仅签名 release 构建
#   ./scripts/sign-dev.sh --debug      # 仅签名 debug 构建
#
# 或在 build.sh 中自动调用

set -e

CERT_NAME="KVMini Dev"
ENTITLEMENTS="src-tauri/entitlements.plist"

# Tauri 2.x 构建路径（输出在项目根目录的 target/ 下，不在 src-tauri/target/）
DEBUG_BINARY="target/debug/kvmini"
DEBUG_APP_BUNDLE="target/debug/bundle/macos/KVMini.app"
RELEASE_BINARY="target/release/kvmini"
RELEASE_APP_BUNDLE="target/release/bundle/macos/KVMini.app"

# 也兼容旧路径 (Tauri 1.x 或部分配置)
LEGACY_DEBUG_BINARY="src-tauri/target/debug/kvmini"
LEGACY_DEBUG_APP="src-tauri/target/debug/bundle/macos/KVMini.app"
LEGACY_RELEASE_BINARY="src-tauri/target/release/kvmini"
LEGACY_RELEASE_APP="src-tauri/target/release/bundle/macos/KVMini.app"

# 解析参数
MODE="auto"
for arg in "$@"; do
    case $arg in
        --release) MODE="release" ;;
        --debug)   MODE="debug" ;;
        --auto)    MODE="auto" ;;
        *)
            echo "用法: $0 [--release|--debug|--auto]"
            exit 1
            ;;
    esac
done

echo "🔏 KVMini 构建签名工具"
echo "================================"
echo "签名模式: $MODE"

# 检查是否为 macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "ℹ️  非 macOS 系统，跳过代码签名"
    exit 0
fi

# 检查证书是否存在，不存在则尝试自动创建
if ! security find-certificate -c "$CERT_NAME" login.keychain-db &>/dev/null; then
    echo "⚠️  未找到证书 \"$CERT_NAME\"，尝试自动创建..."
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    if [ -f "$SCRIPT_DIR/create-dev-cert.sh" ]; then
        bash "$SCRIPT_DIR/create-dev-cert.sh"
        # 再次检查
        if ! security find-certificate -c "$CERT_NAME" login.keychain-db &>/dev/null; then
            echo "❌ 证书创建失败，请手动运行：./scripts/create-dev-cert.sh"
            echo "💡 创建后请在 Keychain Access 中将证书信任设为「始终信任」"
            exit 1
        fi
    else
        echo "❌ 未找到证书创建脚本：scripts/create-dev-cert.sh"
        echo "请先运行：./scripts/create-dev-cert.sh"
        exit 1
    fi
fi

echo "✅ 证书 \"$CERT_NAME\" 已就绪"

# 签名函数
sign_binary() {
    local binary="$1"
    local label="$2"
    if [ -f "$binary" ]; then
        echo "📦 正在签名${label}可执行文件：$binary"
        codesign --force --sign "$CERT_NAME" \
            --entitlements "$ENTITLEMENTS" \
            "$binary" 2>/dev/null || \
        codesign --force --sign "$CERT_NAME" "$binary"
        echo "✅ ${label}可执行文件签名完成"
        return 0
    fi
    return 1
}

sign_app_bundle() {
    local app="$1"
    local label="$2"
    if [ -d "$app" ]; then
        echo "📦 正在签名${label}应用包：$app"
        codesign --force --deep --sign "$CERT_NAME" \
            --entitlements "$ENTITLEMENTS" \
            "$app" 2>/dev/null || \
        codesign --force --deep --sign "$CERT_NAME" "$app"
        echo "✅ ${label}应用包签名完成"
        return 0
    fi
    return 1
}

SIGNED=0

# Release 签名
if [ "$MODE" = "release" ] || [ "$MODE" = "auto" ]; then
    # 优先 Tauri 2.x 路径
    sign_binary "$RELEASE_BINARY" "Release " && SIGNED=1
    sign_app_bundle "$RELEASE_APP_BUNDLE" "Release " && SIGNED=1
    # 兼容旧路径
    if [ $SIGNED -eq 0 ]; then
        sign_binary "$LEGACY_RELEASE_BINARY" "Release(legacy) " && SIGNED=1
        sign_app_bundle "$LEGACY_RELEASE_APP" "Release(legacy) " && SIGNED=1
    fi
fi

# Debug 签名
if [ "$MODE" = "debug" ] || [ "$MODE" = "auto" ]; then
    sign_binary "$DEBUG_BINARY" "Debug " && SIGNED=1
    sign_app_bundle "$DEBUG_APP_BUNDLE" "Debug " && SIGNED=1
    # 兼容旧路径
    if [ $SIGNED -eq 0 ]; then
        sign_binary "$LEGACY_DEBUG_BINARY" "Debug(legacy) " && SIGNED=1
        sign_app_bundle "$LEGACY_DEBUG_APP" "Debug(legacy) " && SIGNED=1
    fi
fi

echo ""
if [ $SIGNED -gt 0 ]; then
    echo "🎉 签名完成！"
    echo ""
    echo "📌 验证签名："
    if [ -d "$RELEASE_APP_BUNDLE" ]; then
        echo "  codesign -dv \"$RELEASE_APP_BUNDLE\""
    elif [ -d "$DEBUG_APP_BUNDLE" ]; then
        echo "  codesign -dv \"$DEBUG_APP_BUNDLE\""
    fi
    echo ""
    echo "💡 提示：首次运行应用后，在系统设置 → 隐私与安全性 → 辅助功能中授权一次即可。"
    echo "         之后重新编译并运行此脚本签名，权限不会失效。"
else
    echo "⚠️  未找到任何可签名的构建产物"
    echo "请先运行构建命令"
fi
