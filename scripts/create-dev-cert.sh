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

set -e

CERT_NAME="KVMini Dev"
KEYCHAIN="login.keychain-db"

echo "🔐 KVMini 开发证书创建工具"
echo "================================"

# 检查是否为 macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "ℹ️  非 macOS 系统，无需创建代码签名证书"
    exit 0
fi

# 检查证书是否已存在且可用于代码签名
if security find-identity -v -p codesigning 2>/dev/null | grep -q "$CERT_NAME"; then
    echo "✅ 证书 \"$CERT_NAME\" 已存在且可用于代码签名，无需重新创建。"
    echo ""
    echo "如需重新创建，请先删除旧证书："
    echo "  security delete-certificate -c \"$CERT_NAME\" $KEYCHAIN"
    exit 0
fi

# 如果证书存在但不可用于代码签名，先删除
if security find-certificate -c "$CERT_NAME" "$KEYCHAIN" &>/dev/null; then
    echo "⚠️  发现旧证书但不可用于代码签名，正在删除..."
    security delete-certificate -c "$CERT_NAME" "$KEYCHAIN" 2>/dev/null || true
fi

echo ""
echo "📋 正在创建自签名证书 \"$CERT_NAME\"..."
echo ""

TMPDIR_CERT=$(mktemp -d)

# 创建 OpenSSL 配置文件（必须包含 CodeSigning 扩展）
cat > "$TMPDIR_CERT/ext.cnf" <<EXTEOF
[req]
distinguished_name = dn
prompt = no
x509_extensions = v3_cs

[dn]
CN = $CERT_NAME
O = KVMini Dev
C = CN

[v3_cs]
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
basicConstraints = critical, CA:false
subjectKeyIdentifier = hash
EXTEOF

# 生成私钥
openssl genrsa -out "$TMPDIR_CERT/key.pem" 2048 2>/dev/null

# 生成自签名证书（有效期 10 年，带 CodeSigning 扩展）
openssl req -new -x509 \
    -key "$TMPDIR_CERT/key.pem" \
    -out "$TMPDIR_CERT/cert.pem" \
    -days 3650 \
    -config "$TMPDIR_CERT/ext.cnf" \
    -extensions v3_cs \
    2>/dev/null

# 打包为 p12 格式（使用 SHA1-3DES 确保 macOS security 命令兼容）
openssl pkcs12 -export \
    -out "$TMPDIR_CERT/kv.p12" \
    -inkey "$TMPDIR_CERT/key.pem" \
    -in "$TMPDIR_CERT/cert.pem" \
    -passout pass:kvmini_temp \
    -certpbe PBE-SHA1-3DES \
    -keypbe PBE-SHA1-3DES \
    -macalg sha1 \
    2>/dev/null

# 导入到 Keychain
security import "$TMPDIR_CERT/kv.p12" \
    -k ~/Library/Keychains/"$KEYCHAIN" \
    -P "kvmini_temp" \
    -T /usr/bin/codesign \
    -T /usr/bin/security \
    2>/dev/null

# 设置信任（代码签名用途）
security add-trusted-cert -d -r trustRoot -p codeSign \
    -k ~/Library/Keychains/"$KEYCHAIN" \
    "$TMPDIR_CERT/cert.pem" \
    2>/dev/null

rm -rf "$TMPDIR_CERT"

# 验证
if security find-identity -v -p codesigning 2>/dev/null | grep -q "$CERT_NAME"; then
    echo "✅ 证书 \"$CERT_NAME\" 创建并信任成功！"
    echo ""
    echo "💡 提示：证书创建后，辅助功能权限授权一次即可，重新编译不会失效。"
else
    echo "⚠️  证书已导入但可能需要手动信任："
    echo "  1. 打开 Keychain Access（钥匙串访问）"
    echo "  2. 找到证书 \"$CERT_NAME\""
    echo "  3. 双击证书 → 展开「信任」→ 将「代码签名」设为「始终信任」"
fi
