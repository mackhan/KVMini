#!/bin/bash
# check-platform-parity.sh - 跨平台功能一致性检查脚本
# 用法：bash scripts/check-platform-parity.sh
# 在 git pull 后自动运行，检测两端功能差异

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN_RS="$REPO_ROOT/src-tauri/src/main.rs"
PLATFORM_RS="$REPO_ROOT/src-tauri/src/platform.rs"
MAIN_JS="$REPO_ROOT/src/main.js"
PLATFORM_JS="$REPO_ROOT/src/platform.js"
INDEX_HTML="$REPO_ROOT/src/index.html"

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

ISSUES=0

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  KVMini 跨平台功能一致性检查${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo ""

# ─── 1. 检查 Rust 后端命令注册 ───
echo -e "${CYAN}[1/4] 检查 Rust 后端命令注册...${NC}"

# 从 main.rs 提取已注册的命令
REGISTERED_COMMANDS=$(grep -oE '[a-z_]+,' "$MAIN_RS" | grep -v '//' | tr -d ',' | sort -u 2>/dev/null || true)

# 从各 .rs 文件提取已定义的 #[tauri::command]
DEFINED_COMMANDS=$(grep -rh '#\[tauri::command\]' "$REPO_ROOT/src-tauri/src/" -A1 | grep 'pub.*fn ' | grep -oE 'fn [a-z_]+' | awk '{print $2}' | sort -u 2>/dev/null || true)

# 检查定义了但未注册的命令
UNREGISTERED=""
while IFS= read -r cmd; do
    if [ -n "$cmd" ] && ! echo "$REGISTERED_COMMANDS" | grep -q "^${cmd}$"; then
        UNREGISTERED="$UNREGISTERED\n  ⚠️  命令 '$cmd' 已定义但未在 main.rs 中注册"
        ISSUES=$((ISSUES + 1))
    fi
done <<< "$DEFINED_COMMANDS"

if [ -n "$UNREGISTERED" ]; then
    echo -e "${YELLOW}发现未注册的命令：${NC}"
    echo -e "$UNREGISTERED"
    echo -e "  参考文件：$MAIN_RS"
else
    echo -e "${GREEN}  ✅ 所有 Tauri 命令均已注册${NC}"
fi

# ─── 2. 检查前端 Platform 抽象层完整性 ───
echo ""
echo -e "${CYAN}[2/4] 检查前端 Platform 抽象层完整性...${NC}"

# 从 platform.rs 提取命令名
RUST_COMMANDS=$(grep -oE "pub fn [a-z_]+" "$PLATFORM_RS" "$REPO_ROOT/src-tauri/src/connection_manager.rs" "$REPO_ROOT/src-tauri/src/network.rs" "$REPO_ROOT/src-tauri/src/usb.rs" 2>/dev/null | grep -v 'test\|new\|discover_all\|connect_to_device_impl\|disconnect\|get_connection_status_impl\|get_devices\|scan_devices\|connect_to_device_usb\|disconnect_device\|show_notification\|kill_legacy_process_macos\|kill_legacy_process_windows' | awk -F'fn ' '{print $2}' | sort -u || true)

# 从 platform.js 提取已封装的命令
JS_COMMANDS=$(grep -oE "safeInvoke\('[a-z_]+'" "$PLATFORM_JS" 2>/dev/null | grep -oE "'[a-z_]+'" | tr -d "'" | sort -u || true)

MISSING_IN_JS=""
while IFS= read -r cmd; do
    if [ -n "$cmd" ] && ! echo "$JS_COMMANDS" | grep -q "^${cmd}$"; then
        MISSING_IN_JS="$MISSING_IN_JS\n  ⚠️  Rust 命令 '$cmd' 未在 platform.js 中封装"
        ISSUES=$((ISSUES + 1))
    fi
done <<< "$RUST_COMMANDS"

if [ -n "$MISSING_IN_JS" ]; then
    echo -e "${YELLOW}platform.js 中缺少封装：${NC}"
    echo -e "$MISSING_IN_JS"
    echo -e "  参考文件：$PLATFORM_JS"
else
    echo -e "${GREEN}  ✅ platform.js 封装完整${NC}"
fi

# ─── 3. 检查前端 UI 组件完整性 ───
echo ""
echo -e "${CYAN}[3/4] 检查前端 UI 组件完整性...${NC}"

REQUIRED_IDS=(
    "connection-status"
    "connection-type"
    "latency"
    "connect-btn"
    "disconnect-btn"
    "settings-btn"
    "devices-list"
    "scan-btn"
    "scan-animation"
    "history-list"
    "error-toast"
    "success-toast"
    "settings-modal"
    "hotkey-input"
    "prefer-connection-type"
    "auto-reconnect"
    "auto-start"
    "start-minimized"
    # 跨网段直连新增组件
    "manual-ip-input"
    "manual-port-input"
    "manual-connect-btn"
    "manual-connect-status"
    # 监听地址配置新增组件
    "listen-ip-select"
    "listen-port-input"
    "listen-address-display"
)

MISSING_UI=""
for id in "${REQUIRED_IDS[@]}"; do
    if ! grep -q "id=\"$id\"" "$INDEX_HTML" 2>/dev/null; then
        MISSING_UI="$MISSING_UI\n  ⚠️  UI 组件缺失：id='$id'"
        ISSUES=$((ISSUES + 1))
    fi
done

if [ -n "$MISSING_UI" ]; then
    echo -e "${YELLOW}index.html 中缺少 UI 组件：${NC}"
    echo -e "$MISSING_UI"
    echo -e "  参考文件：$INDEX_HTML"
else
    echo -e "${GREEN}  ✅ 所有必要 UI 组件均存在${NC}"
fi

# ─── 4. 检查 alert 使用（应替换为 Toast）───
echo ""
echo -e "${CYAN}[4/4] 检查是否存在 alert() 调用...${NC}"

ALERT_USAGE=$(grep -n "alert(" "$MAIN_JS" 2>/dev/null | grep -v '//' || true)
if [ -n "$ALERT_USAGE" ]; then
    echo -e "${YELLOW}  ⚠️  main.js 中仍存在 alert() 调用：${NC}"
    echo "$ALERT_USAGE" | while IFS= read -r line; do
        echo "    $line"
    done
    echo -e "  请替换为 showError() 或 showSuccess() 函数"
    ISSUES=$((ISSUES + 1))
else
    echo -e "${GREEN}  ✅ 未发现 alert() 调用${NC}"
fi

# ─── 结果汇总 ───
echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
if [ "$ISSUES" -eq 0 ]; then
    echo -e "${GREEN}  ✅ 跨平台功能检查通过！两端功能完全一致。${NC}"
else
    echo -e "${RED}  ❌ 发现 $ISSUES 个问题，请补全后再提交代码。${NC}"
    echo -e "${YELLOW}  提示：补全功能后请编写对应测试用例，测试通过后方可提交。${NC}"
fi
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo ""

exit $ISSUES
