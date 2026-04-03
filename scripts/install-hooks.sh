#!/bin/bash
# install-hooks.sh - 安装 git hooks
# 用法：bash scripts/install-hooks.sh

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

if [ ! -d "$HOOKS_DIR" ]; then
    echo "❌ 未找到 .git/hooks 目录，请在 git 仓库根目录下运行此脚本"
    exit 1
fi

# 安装 post-merge hook
cp "$REPO_ROOT/scripts/post-merge.hook" "$HOOKS_DIR/post-merge"
chmod +x "$HOOKS_DIR/post-merge"
echo "✅ post-merge hook 已安装：$HOOKS_DIR/post-merge"

echo ""
echo "🎉 git hooks 安装完成！"
echo "   每次执行 git pull 后，将自动运行跨平台功能一致性检查。"
