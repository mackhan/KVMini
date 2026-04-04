#!/bin/bash

# 重置 KVM Share 辅助功能权限（TCC 缓存清理）
# 用途：当系统显示已授权但应用仍提示未授权时使用
# 使用方法：bash scripts/reset-accessibility-permission.sh

BUNDLE_ID="com.kvmshare.app"
APP_NAME="KVM Share"

echo "🔧 KVM Share 辅助功能权限重置工具"
echo "======================================"
echo ""

# 检查是否为 macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo "❌ 此脚本仅适用于 macOS"
    exit 1
fi

echo "📋 当前 TCC 数据库中的辅助功能授权记录："
echo ""
sudo sqlite3 /Library/Application\ Support/com.apple.TCC/TCC.db \
    "SELECT client, auth_value, last_modified FROM access WHERE service='kTCCServiceAccessibility' AND client LIKE '%kvm%' OR client LIKE '%KVM%';" 2>/dev/null || echo "（无记录或无权限读取）"
echo ""

echo "🗑️  正在清除 $APP_NAME 的辅助功能权限记录..."

# 方法1：使用 tccutil 重置（推荐，不需要 SIP 关闭）
echo "→ 使用 tccutil 重置 Bundle ID: $BUNDLE_ID"
tccutil reset Accessibility "$BUNDLE_ID" 2>/dev/null
if [ $? -eq 0 ]; then
    echo "✅ tccutil 重置成功"
else
    echo "⚠️  tccutil 重置失败，尝试备用方法..."
    # 方法2：直接删除 TCC 数据库记录（需要 sudo）
    sudo sqlite3 /Library/Application\ Support/com.apple.TCC/TCC.db \
        "DELETE FROM access WHERE service='kTCCServiceAccessibility' AND client='$BUNDLE_ID';" 2>/dev/null
    if [ $? -eq 0 ]; then
        echo "✅ 数据库记录清除成功"
    else
        echo "❌ 清除失败，请手动操作："
        echo "   系统设置 → 隐私与安全性 → 辅助功能 → 找到 KVM Share → 删除后重新添加"
    fi
fi

echo ""
echo "✅ 完成！请按以下步骤重新授权："
echo ""
echo "  1. 重新打开 KVM Share 应用"
echo "  2. 应用会弹出权限引导页面"
echo "  3. 点击「前往授权」按钮"
echo "  4. 在系统设置中找到 KVM Share，打开开关"
echo "  5. 返回应用，权限授予后自动继续"
echo ""
echo "💡 提示：如果仍然无效，请尝试重启 Mac 后再操作"
