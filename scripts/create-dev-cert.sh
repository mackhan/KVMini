#!/bin/bash
# create-dev-cert.sh
# 为 KVMini 开发环境创建自签名代码签名证书
#
# 用途：macOS 辅助功能权限（TCC）通过代码签名 identity 识别应用。
#       使用自签名证书签名后，重新编译不会导致权限失效（只要证书不变）。
#
# 使用方法：
#   chmod +x scripts/create-dev-cert.sh
#   ./scripts/create-dev-cert.sh
#
# 参考：OneNoteCopy 项目的自签名证书方案

set -e

CERT_NAME="KVMini Dev"
KEYCHAIN="login.keychain-db"

echo "🔐 KVMini 开发证书创建工具"
echo "================================"

# 检查证书是否已存在
if security find-certificate -c "$CERT_NAME" "$KEYCHAIN" &>/dev/null; then
    echo "✅ 证书 \"$CERT_NAME\" 已存在，无需重新创建。"
    echo ""
    echo "如需重新创建，请先删除旧证书："
    echo "  security delete-certificate -c \"$CERT_NAME\" $KEYCHAIN"
    exit 0
fi

echo "📋 正在创建自签名证书 \"$CERT_NAME\"..."
echo ""

# 使用 Keychain Access 创建自签名证书
# 通过 security 命令行工具创建代码签名证书
/usr/bin/security create-certificate \
    -k "$KEYCHAIN" \
    -s "$CERT_NAME" \
    -t codeSign \
    -Z SHA256 \
    2>/dev/null || {
    # 如果 security create-certificate 不支持，使用 openssl 方式
    echo "⚠️  security create-certificate 不可用，使用 openssl 方式..."
    create_cert_with_openssl
}

echo ""
echo "✅ 证书 \"$CERT_NAME\" 创建成功！"
echo ""
echo "📌 下一步："
echo "  1. 打开 Keychain Access（钥匙串访问）"
echo "  2. 找到证书 \"$CERT_NAME\""
echo "  3. 双击证书 → 展开「信任」→ 将「代码签名」设为「始终信任」"
echo "  4. 运行 ./scripts/sign-dev.sh 对应用进行签名"
echo ""
echo "💡 提示：证书创建后，辅助功能权限授权一次即可，重新编译不会失效。"

create_cert_with_openssl() {
    # 使用 openssl 生成自签名证书并导入 Keychain
    TMPDIR=$(mktemp -d)
    KEY_FILE="$TMPDIR/kvmini-dev.key"
    CERT_FILE="$TMPDIR/kvmini-dev.crt"
    P12_FILE="$TMPDIR/kvmini-dev.p12"

    # 生成私钥
    openssl genrsa -out "$KEY_FILE" 2048 2>/dev/null

    # 生成自签名证书（有效期 10 年）
    openssl req -new -x509 \
        -key "$KEY_FILE" \
        -out "$CERT_FILE" \
        -days 3650 \
        -subj "/CN=$CERT_NAME/O=KVMini Dev/C=CN" \
        -extensions v3_ca \
        2>/dev/null

    # 打包为 p12 格式（无密码）
    openssl pkcs12 -export \
        -out "$P12_FILE" \
        -inkey "$KEY_FILE" \
        -in "$CERT_FILE" \
        -passout pass: \
        2>/dev/null

    # 导入到 Keychain
    security import "$P12_FILE" \
        -k "$KEYCHAIN" \
        -T /usr/bin/codesign \
        -P "" \
        2>/dev/null

    rm -rf "$TMPDIR"
    echo "✅ 证书通过 openssl 方式创建成功"
}
