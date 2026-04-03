#!/bin/bash
# cleanup-legacy.sh - 清理 kvnshare 遗留进程和开机启动项（macOS）
# 用法：bash scripts/cleanup-legacy.sh

set -e

LEGACY_PROCESS="kvnshare"
LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
LAUNCH_DAEMONS_DIR="/Library/LaunchDaemons"

echo "🔍 检查 kvnshare 遗留进程..."

# 检测并终止遗留进程
if pgrep -x "$LEGACY_PROCESS" > /dev/null 2>&1; then
    echo "⚠️  发现 kvnshare 遗留进程，正在终止..."
    pkill -x "$LEGACY_PROCESS" && echo "✅ kvnshare 进程已终止" || echo "❌ 终止失败，请手动执行: sudo pkill -x kvnshare"
else
    echo "✅ 未发现 kvnshare 遗留进程"
fi

# 清理 LaunchAgents 开机启动项
echo ""
echo "🔍 检查 LaunchAgents 开机启动项..."
FOUND_AGENT=false
for plist in "$LAUNCH_AGENTS_DIR"/*kvnshare* "$LAUNCH_AGENTS_DIR"/*KVNShare* 2>/dev/null; do
    if [ -f "$plist" ]; then
        echo "⚠️  发现启动项: $plist"
        launchctl unload "$plist" 2>/dev/null || true
        rm -f "$plist"
        echo "✅ 已移除: $plist"
        FOUND_AGENT=true
    fi
done
if [ "$FOUND_AGENT" = false ]; then
    echo "✅ 未发现 LaunchAgents 启动项"
fi

# 清理 LaunchDaemons 系统级启动项（需要 sudo）
echo ""
echo "🔍 检查 LaunchDaemons 系统级启动项..."
FOUND_DAEMON=false
for plist in "$LAUNCH_DAEMONS_DIR"/*kvnshare* "$LAUNCH_DAEMONS_DIR"/*KVNShare* 2>/dev/null; do
    if [ -f "$plist" ]; then
        echo "⚠️  发现系统启动项: $plist（需要 sudo 权限）"
        sudo launchctl unload "$plist" 2>/dev/null || true
        sudo rm -f "$plist"
        echo "✅ 已移除: $plist"
        FOUND_DAEMON=true
    fi
done
if [ "$FOUND_DAEMON" = false ]; then
    echo "✅ 未发现 LaunchDaemons 启动项"
fi

echo ""
echo "🎉 清理完成！当前运行的 KVM Share 进程："
pgrep -la "KVM Share" 2>/dev/null || echo "（无运行中的 KVM Share 进程）"
