#!/bin/bash
# KVMini 构建脚本
# 用法:
#   ./build.sh              # 正常构建
#   ./build.sh --reset-purchase  # 清除试用期数据后构建（开发测试用）

set -e  # 遇到错误立即退出

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 解析参数
RESET_PURCHASE=false

for arg in "$@"; do
    case $arg in
        --reset-purchase)
            RESET_PURCHASE=true
            shift
            ;;
        *)
            log_error "未知参数: $arg"
            echo "用法: $0 [--reset-purchase]"
            exit 1
            ;;
    esac
done

# 清除试用期数据
if [ "$RESET_PURCHASE" = true ]; then
    log_warn "正在清除试用期数据（开发测试模式）..."

    # 检测操作系统
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS: 删除 Keychain 中的试用期和购买状态
        log_info "检测到 macOS 系统"

        # 删除试用期数据 (com.kvmini.app.commerce)
        if security find-generic-password -s "com.kvmini.app.commerce" >/dev/null 2>&1; then
            security delete-generic-password -s "com.kvmini.app.commerce" >/dev/null 2>&1
            log_info "✓ 已删除 Keychain: com.kvmini.app.commerce"
        else
            log_info "· Keychain 中无试用期数据"
        fi

        # 删除 IAP 购买状态 (com.kvmini.app.iap)
        if security find-generic-password -s "com.kvmini.app.iap" >/dev/null 2>&1; then
            security delete-generic-password -s "com.kvmini.app.iap" >/dev/null 2>&1
            log_info "✓ 已删除 Keychain: com.kvmini.app.iap"
        else
            log_info "· Keychain 中无 IAP 购买状态"
        fi

        # 可选：删除应用数据目录
        APP_SUPPORT="$HOME/Library/Application Support/KVMini"
        if [ -d "$APP_SUPPORT" ]; then
            log_warn "是否删除应用数据目录? (y/N)"
            read -r response
            if [[ "$response" =~ ^[Yy]$ ]]; then
                rm -rf "$APP_SUPPORT"
                log_info "✓ 已删除应用数据: $APP_SUPPORT"
            fi
        fi

    elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
        # Windows: 删除注册表项和本地数据
        log_info "检测到 Windows 系统"

        # 删除试用期数据（注册表）
        reg delete "HKCU\Software\KVMini" /f >/dev/null 2>&1 && log_info "✓ 已删除注册表: HKCU\\Software\\KVMini" || log_info "· 注册表中无数据"

        # 删除应用数据目录
        APP_DATA="$APPDATA/KVMini"
        if [ -d "$APP_DATA" ]; then
            rm -rf "$APP_DATA"
            log_info "✓ 已删除应用数据: $APP_DATA"
        fi

    else
        log_error "不支持的操作系统: $OSTYPE"
        exit 1
    fi

    echo ""
    log_info "✓ 试用期数据清除完成"
    echo ""
fi

# 执行构建
log_info "开始构建 KVMini..."
echo ""

if command -v npm >/dev/null 2>&1; then
    npm run build
else
    log_error "未找到 npm，请先安装 Node.js"
    exit 1
fi

log_info "✓ 构建完成"

# macOS 自动签名（确保辅助功能权限在重装后不丢失）
if [[ "$OSTYPE" == "darwin"* ]]; then
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    SIGN_SCRIPT="$SCRIPT_DIR/scripts/sign-dev.sh"
    # 如果 build.sh 在项目根目录，sign-dev.sh 在 scripts/ 下
    if [ ! -f "$SIGN_SCRIPT" ]; then
        SIGN_SCRIPT="scripts/sign-dev.sh"
    fi
    if [ -f "$SIGN_SCRIPT" ]; then
        echo ""
        log_info "正在对构建产物进行代码签名..."
        chmod +x "$SIGN_SCRIPT"
        bash "$SIGN_SCRIPT" --release || {
            log_warn "代码签名失败（不影响构建），但每次安装后可能需要重新授权辅助功能权限"
            log_warn "请运行 ./scripts/create-dev-cert.sh 创建签名证书后重试"
        }
    else
        log_warn "未找到签名脚本，跳过代码签名"
    fi
fi
