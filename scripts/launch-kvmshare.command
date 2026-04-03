#!/bin/bash
# KMShare 启动脚本 - 可拖入 macOS Dock 作为快捷方式
# 使用方法：将此文件拖入 Dock，点击即可启动最新版本

# 定位到项目根目录（脚本所在目录的上一级）
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR" || { osascript -e 'display alert "KMShare" message "找不到项目目录"'; exit 1; }

# 检查 npm 是否可用
if ! command -v npm &>/dev/null; then
    osascript -e 'display alert "KMShare" message "未找到 npm，请先安装 Node.js"'
    exit 1
fi

# 在新终端窗口中启动 tauri dev
osascript <<EOF
tell application "Terminal"
    activate
    do script "cd '$PROJECT_DIR' && npm run dev"
end tell
EOF
