#!/bin/bash
# KVMini - 生成 tauri-plugin-updater 所需的 Ed25519 密钥对
# 用法: ./scripts/generate-updater-key.sh
#
# 生成后：
#   1. 公钥内容 → 复制到 src-tauri/tauri.conf.json 的 plugins.updater.pubkey
#   2. 私钥内容 → 添加到 GitHub Secrets: TAURI_SIGNING_PRIVATE_KEY
#   3. 私钥密码 → 添加到 GitHub Secrets: TAURI_SIGNING_PRIVATE_KEY_PASSWORD（可选）

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  KVMini Updater 签名密钥生成工具${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo ""

# 密钥存储目录
KEY_DIR="$HOME/.tauri"
KEY_FILE="$KEY_DIR/kvmini.key"
PUB_FILE="$KEY_DIR/kvmini.key.pub"

# 检查是否已存在密钥
if [ -f "$KEY_FILE" ]; then
    echo -e "${YELLOW}[WARN]${NC} 密钥已存在: $KEY_FILE"
    echo -e "${YELLOW}[WARN]${NC} 重新生成将使已安装的旧版本无法接收更新！"
    echo ""
    read -p "是否覆盖？(y/N) " -r
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${GREEN}[INFO]${NC} 已取消，使用现有密钥"
        echo ""
        echo -e "${GREEN}公钥内容:${NC}"
        cat "$PUB_FILE"
        echo ""
        echo -e "${YELLOW}私钥路径:${NC} $KEY_FILE"
        exit 0
    fi
fi

# 创建目录
mkdir -p "$KEY_DIR"

# 生成密钥对（使用 tauri signer）
echo -e "${GREEN}[INFO]${NC} 正在生成 Ed25519 密钥对..."
echo ""

# 检查 tauri CLI 是否可用
if command -v cargo-tauri >/dev/null 2>&1; then
    cargo tauri signer generate -w "$KEY_FILE"
elif npx tauri --version >/dev/null 2>&1; then
    npx tauri signer generate -- -w "$KEY_FILE"
else
    echo -e "${RED}[ERROR]${NC} 未找到 tauri CLI"
    echo "请先安装: cargo install tauri-cli"
    echo "或使用: npm install -g @tauri-apps/cli"
    exit 1
fi

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ 密钥对已生成！${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}📁 密钥文件:${NC}"
echo "   私钥: $KEY_FILE"
echo "   公钥: $PUB_FILE"
echo ""
echo -e "${CYAN}📋 接下来请执行以下步骤:${NC}"
echo ""
echo -e "  ${GREEN}1.${NC} 将公钥内容复制到 src-tauri/tauri.conf.json:"
echo -e "     ${YELLOW}plugins.updater.pubkey${NC}"
echo ""
echo -e "  ${GREEN}2.${NC} 在 GitHub 仓库设置中添加 Secrets:"
echo -e "     ${YELLOW}TAURI_SIGNING_PRIVATE_KEY${NC} = 私钥文件内容"
echo -e "     ${YELLOW}TAURI_SIGNING_PRIVATE_KEY_PASSWORD${NC} = 密钥密码（如果设置了的话）"
echo ""
echo -e "${RED}⚠️  警告: 私钥一旦丢失，已安装旧版的用户将无法接收更新！${NC}"
echo -e "${RED}   请妥善备份 $KEY_FILE${NC}"
echo ""
echo -e "${GREEN}公钥内容（用于 tauri.conf.json）:${NC}"
echo "────────────────────────────────────────────────"
cat "$PUB_FILE" 2>/dev/null || echo "(公钥将在上方 tauri signer 输出中显示)"
echo ""
echo "────────────────────────────────────────────────"
