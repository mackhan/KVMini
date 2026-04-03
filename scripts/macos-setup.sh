#!/bin/bash

# macOS权限设置脚本
# 解决Tauri应用钥匙串重复访问问题

echo "设置macOS权限配置..."

# 确保权限文件存在
if [ ! -f "src-tauri/entitlements.plist" ]; then
    echo "错误: 权限声明文件不存在"
    exit 1
fi

# 验证权限文件格式
plutil -lint src-tauri/entitlements.plist
if [ $? -ne 0 ]; then
    echo "错误: 权限文件格式不正确"
    exit 1
fi

echo "✅ 权限文件验证通过"

# 设置开发环境变量
export MACOSX_DEPLOYMENT_TARGET=10.13
export SDKROOT=$(xcrun --sdk macosx --show-sdk-path)

# 清理之前的构建缓存
echo "清理构建缓存..."
cd src-tauri && cargo clean

# 重新构建应用
echo "重新构建应用..."
cargo tauri build

if [ $? -eq 0 ]; then
    echo "✅ 构建成功！应用现在应该不会重复请求钥匙串访问权限"
    echo ""
    echo "如果仍有权限问题，请尝试："
    echo "1. 在系统偏好设置 -> 安全性与隐私 -> 隐私中移除应用权限"
    echo "2. 重新运行应用并重新授权"
    echo "3. 权限会被记住，下次不会重复请求"
else
    echo "❌ 构建失败，请检查错误信息"
fi