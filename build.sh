#!/bin/bash
# KVMini 构建脚本（本地构建 + 上传 GitHub Releases）
#
# 用法:
#   ./build.sh                    # 正常构建（本地）
#   ./build.sh --reset-purchase   # 清除试用期数据后构建（开发测试用）
#   ./build.sh --upload           # 构建后自动生成 latest.json 并上传到 GitHub Releases
#
# 前置条件（仅 --upload 时需要）:
#   brew install gh && gh auth login
#   GitHub 仓库可以是 Private，Releases 仍然公开可访问
#
# 多平台发布说明:
#   - macOS aarch64：在 M 系列 Mac 上运行 ./build.sh --upload
#   - macOS x86_64 ：在 Intel Mac 或 Rosetta 下运行 ./build.sh --upload
#   - Windows      ：在 Windows 上运行 BuildForWin.bat 后再手动上传产物
#   - latest.json  ：每次上传会自动更新，包含当前平台的下载链接和签名

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
UPLOAD_RELEASE=false
SINGLE_ARCH=false   # 仅构建当前架构（默认在 macOS 上同时构建 aarch64 + x86_64）

for arg in "$@"; do
    case $arg in
        --reset-purchase)
            RESET_PURCHASE=true
            shift
            ;;
        --upload)
            UPLOAD_RELEASE=true
            shift
            ;;
        --single-arch)
            SINGLE_ARCH=true
            shift
            ;;
        *)
            log_error "未知参数: $arg"
            echo "用法: $0 [--reset-purchase] [--upload] [--single-arch]"
            echo "  --single-arch  仅构建当前架构（默认 macOS 上同时构建 aarch64 + x86_64）"
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

# 预检：签名私钥（必须在 bump 版本号之前检查，避免构建失败时反复 bump）
if [ -z "$TAURI_SIGNING_PRIVATE_KEY" ]; then
    log_error "未设置 TAURI_SIGNING_PRIVATE_KEY 环境变量，无法构建 updater 签名"
    log_error "请先执行: export TAURI_SIGNING_PRIVATE_KEY=\$(cat ~/.tauri/KVMini-updater.key)"
    exit 1
fi

# 智能版本号管理：检测 tag，决定是否 bump
TAG_EXISTS=false
log_info "检查版本号..."
set +e  # 暂时关闭 errexit（version-update 用 exit code 2 表示 tag 已存在）
node scripts/version-update.js --check-tag
BUMP_RESULT=$?
set -e
if [ $BUMP_RESULT -eq 2 ]; then
    log_info "tag 已存在，跳过版本号递增（另一个平台已构建）"
    TAG_EXISTS=true
elif [ $BUMP_RESULT -eq 0 ]; then
    log_info "版本号已递增"
elif [ $BUMP_RESULT -ne 0 ]; then
    log_error "版本号更新失败（exit code: $BUMP_RESULT）"
    exit 1
fi

# 执行构建
log_info "开始构建 KVMini..."
echo ""

if ! command -v npm >/dev/null 2>&1; then
    log_error "未找到 npm，请先安装 Node.js"
    exit 1
fi

# 决定要构建的目标架构
BUILD_TARGETS=()
if [[ "$OSTYPE" == "darwin"* ]] && [ "$SINGLE_ARCH" = false ]; then
    # macOS 默认同时构建 Apple Silicon + Intel
    BUILD_TARGETS=("aarch64-apple-darwin" "x86_64-apple-darwin")
    log_info "macOS 双架构构建：${BUILD_TARGETS[*]}"

    # 校验 Rust target 已安装
    INSTALLED_TARGETS=$(rustup target list --installed 2>/dev/null || echo "")
    for t in "${BUILD_TARGETS[@]}"; do
        if ! echo "$INSTALLED_TARGETS" | grep -q "^$t$"; then
            log_warn "Rust target $t 未安装，正在安装..."
            rustup target add "$t" || {
                log_error "无法安装 Rust target $t"
                exit 1
            }
        fi
    done
else
    # Windows 或 --single-arch：用默认 npm run build（当前架构）
    BUILD_TARGETS=("")  # 空字符串代表不传 --target
fi

for target in "${BUILD_TARGETS[@]}"; do
    if [ -z "$target" ]; then
        log_info "构建当前平台默认架构..."
        npm run build
    else
        log_info "构建目标架构: $target"
        npx tauri build --target "$target"
    fi
done

log_info "✓ 构建完成"

# macOS 自动签名（确保辅助功能权限在重装后不丢失）
if [[ "$OSTYPE" == "darwin"* ]]; then
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    SIGN_SCRIPT="$SCRIPT_DIR/scripts/sign-dev.sh"
    if [ ! -f "$SIGN_SCRIPT" ]; then
        SIGN_SCRIPT="scripts/sign-dev.sh"
    fi
    if [ -f "$SIGN_SCRIPT" ]; then
        echo ""
        log_info "正在对构建产物进行代码签名..."
        chmod +x "$SIGN_SCRIPT"
        # sign-dev.sh 默认只签 target/release，需要再额外对每个 --target 子目录签名
        bash "$SIGN_SCRIPT" --release || {
            log_warn "默认产物签名失败（不影响构建）"
        }

        # 对每个交叉编译的 target 子目录单独签名
        DEV_CERT="Developer ID Application: han yufei (8F772XVA9L)"
        ENTITLEMENTS_FILE="src-tauri/entitlements.plist"
        if security find-identity -v -p codesigning 2>/dev/null | grep -q "$DEV_CERT"; then
            for target in "${BUILD_TARGETS[@]}"; do
                [ -z "$target" ] && continue
                CROSS_APP="target/$target/release/bundle/macos/KVMini.app"
                CROSS_BIN="target/$target/release/kvmini"
                if [ -d "$CROSS_APP" ]; then
                    log_info "签名 $target 产物..."
                    codesign --force --sign "$DEV_CERT" --options runtime \
                        --entitlements "$ENTITLEMENTS_FILE" "$CROSS_BIN" 2>/dev/null || true
                    codesign --force --deep --sign "$DEV_CERT" --options runtime \
                        --entitlements "$ENTITLEMENTS_FILE" "$CROSS_APP" \
                        && log_info "✓ $target 签名完成" \
                        || log_warn "$target 签名失败"
                fi
            done
        fi
    else
        log_warn "未找到签名脚本，跳过代码签名"
    fi
fi

# 上传到 GitHub Releases（可选）
if [ "$UPLOAD_RELEASE" = true ]; then
    echo ""
    log_info "准备上传到 GitHub Releases..."

    # 检查 gh CLI 是否安装
    if ! command -v gh >/dev/null 2>&1; then
        log_error "未找到 gh CLI，请先安装: brew install gh"
        exit 1
    fi

    # 检查 gh 登录状态
    if ! gh auth status >/dev/null 2>&1; then
        log_error "gh CLI 未登录，请先运行: gh auth login"
        exit 1
    fi

    # 从 Cargo.toml 读取版本号
    VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    TAG="v${VERSION}"
    log_info "版本号: ${VERSION}，tag: ${TAG}"

    # 查找构建产物
    ARTIFACTS=()

    # 双架构产物收集（macOS）
    # darwin-aarch64：默认 target/release 路径（npm run build 在 arm64 机器上的输出）
    #                 或显式 target/aarch64-apple-darwin/release（双架构构建）
    # darwin-x86_64 ：target/x86_64-apple-darwin/release
    declare -A MAC_TAR     # platform_key -> tar.gz 路径
    declare -A MAC_SIG     # platform_key -> sig 路径

    # 候选基础目录（按优先级）
    MAC_DIRS_AARCH64=(
        "target/aarch64-apple-darwin/release/bundle"
        "target/release/bundle"
    )
    MAC_DIRS_X86_64=(
        "target/x86_64-apple-darwin/release/bundle"
    )

    # 收集函数：从指定目录列表中找 dmg / app.tar.gz / sig
    collect_mac_artifacts() {
        local platform_key="$1"
        shift
        local dirs=("$@")
        for base in "${dirs[@]}"; do
            for f in "$base"/dmg/*.dmg; do
                [ -f "$f" ] && ARTIFACTS+=("$f")
            done
            for f in "$base"/macos/*.app.tar.gz; do
                if [ -f "$f" ] && [ -z "${MAC_TAR[$platform_key]:-}" ]; then
                    ARTIFACTS+=("$f")
                    MAC_TAR[$platform_key]="$f"
                fi
            done
            for f in "$base"/macos/*.app.tar.gz.sig; do
                if [ -f "$f" ] && [ -z "${MAC_SIG[$platform_key]:-}" ]; then
                    ARTIFACTS+=("$f")
                    MAC_SIG[$platform_key]="$f"
                fi
            done
        done
    }

    if [[ "$OSTYPE" == "darwin"* ]]; then
        collect_mac_artifacts "darwin-aarch64" "${MAC_DIRS_AARCH64[@]}"
        collect_mac_artifacts "darwin-x86_64"  "${MAC_DIRS_X86_64[@]}"
    fi

    # Windows NSIS（用户下载安装包）
    for f in target/release/bundle/nsis/*.exe; do
        [ -f "$f" ] && ARTIFACTS+=("$f")
    done
    # Windows NSIS zip（updater 用）
    for f in target/release/bundle/nsis/*.nsis.zip; do
        [ -f "$f" ] && ARTIFACTS+=("$f")
    done
    # Windows NSIS zip sig
    WIN_SIG=""
    WIN_ZIP=""
    for f in target/release/bundle/nsis/*.nsis.zip.sig; do
        if [ -f "$f" ]; then
            ARTIFACTS+=("$f")
            WIN_SIG="$f"
            WIN_ZIP="${f%.sig}"
        fi
    done

    if [ ${#ARTIFACTS[@]} -eq 0 ]; then
        log_error "未找到构建产物，请先构建"
        exit 1
    fi

    log_info "找到 ${#ARTIFACTS[@]} 个构建产物:"
    for f in "${ARTIFACTS[@]}"; do
        echo "  - $(basename "$f")"
    done

    # 生成 latest.json（使用 Node.js 跨平台脚本）
    LATEST_JSON="target/release/bundle/macos/latest.json"
    GH_REPO="mackhan/KVMini"

    # 尝试下载已有 Release 的 latest.json（用于合并其他平台条目）
    EXISTING_ARG=""
    TMP_LATEST=""
    if gh release view "$TAG" --repo "$GH_REPO" >/dev/null 2>&1; then
        TMP_LATEST=$(mktemp /tmp/kvmini-latest-XXXXXX.json)
        if gh release download "$TAG" --repo "$GH_REPO" --pattern "latest.json" --output "$TMP_LATEST" >/dev/null 2>&1; then
            EXISTING_ARG="--existing $TMP_LATEST"
            log_info "已下载旧 latest.json 用于合并"
        else
            rm -f "$TMP_LATEST"
            TMP_LATEST=""
        fi
    fi

    # 收集要写入 latest.json 的 (platform, url, sig) 三元组
    declare -a LATEST_ENTRIES   # 每条形如 "platform|url|sig_file"
    for plat in "darwin-aarch64" "darwin-x86_64"; do
        tar_path="${MAC_TAR[$plat]:-}"
        sig_path="${MAC_SIG[$plat]:-}"
        if [ -n "$tar_path" ] && [ -n "$sig_path" ]; then
            url="https://github.com/mackhan/KVMini/releases/download/${TAG}/$(basename "$tar_path")"
            LATEST_ENTRIES+=("${plat}|${url}|${sig_path}")
        fi
    done
    if [ -n "$WIN_ZIP" ] && [ -n "$WIN_SIG" ]; then
        url="https://github.com/mackhan/KVMini/releases/download/${TAG}/$(basename "$WIN_ZIP")"
        LATEST_ENTRIES+=("windows-x86_64|${url}|${WIN_SIG}")
    fi

    if [ ${#LATEST_ENTRIES[@]} -gt 0 ]; then
        # 多次调用 generate-latest-json.js，每次合并一个 platform
        CURRENT_EXISTING_ARG="$EXISTING_ARG"
        for entry in "${LATEST_ENTRIES[@]}"; do
            IFS='|' read -r plat url sig_file <<< "$entry"
            log_info "写入 latest.json 平台条目: $plat"
            node scripts/generate-latest-json.js \
                --version "$VERSION" \
                --tag "$TAG" \
                --platform "$plat" \
                --url "$url" \
                --signature-file "$sig_file" \
                --output "$LATEST_JSON" \
                $CURRENT_EXISTING_ARG
            # 后续条目以刚生成的 latest.json 为基础继续合并
            CURRENT_EXISTING_ARG="--existing $LATEST_JSON"
        done
        log_info "生成 latest.json:"
        cat "$LATEST_JSON"
        ARTIFACTS+=("$LATEST_JSON")
    else
        log_warn "未找到任何 updater 签名文件，跳过 latest.json 生成"
    fi

    # 清理临时文件
    [ -n "$TMP_LATEST" ] && rm -f "$TMP_LATEST"

    # 创建 tag（如果不存在）
    if [ "$TAG_EXISTS" = false ]; then
        if ! git rev-parse "$TAG" >/dev/null 2>&1; then
            log_info "创建 tag: ${TAG}"
            git tag -a "$TAG" -m "Release ${TAG}"
        fi
        # 推送 tag 到 GitHub
        if git remote get-url github >/dev/null 2>&1; then
            git push github "$TAG" 2>/dev/null || log_warn "tag 推送失败（可能已存在于远端）"
        else
            log_warn "未配置 github remote，跳过推送 tag"
        fi
    else
        log_info "tag ${TAG} 已存在于 GitHub，跳过创建"
    fi

    # 创建或更新 Release
    if gh release view "$TAG" --repo "$GH_REPO" >/dev/null 2>&1; then
        log_info "Release ${TAG} 已存在，上传产物..."
        gh release upload "$TAG" "${ARTIFACTS[@]}" --clobber --repo "$GH_REPO"
    else
        log_info "创建 Release ${TAG}..."
        gh release create "$TAG" "${ARTIFACTS[@]}" \
            --title "KVMini ${TAG}" \
            --notes "KVMini ${TAG} release" \
            --draft \
            --repo "$GH_REPO"
        log_info "Release 已创建为草稿，请前往 GitHub 编辑发布说明后发布"
    fi

    log_info "✓ 上传完成"
fi
