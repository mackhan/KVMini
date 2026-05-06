#!/usr/bin/env node

/**
 * generate-latest-json.js
 * 生成/合并 Tauri updater 需要的 latest.json 文件。
 * Mac 和 Win 共用，替代 build.sh 中的 python3 内嵌脚本。
 *
 * 用法（CLI）:
 *   node scripts/generate-latest-json.js \
 *     --version 0.1.28 \
 *     --tag v0.1.28 \
 *     --platform darwin-aarch64 \
 *     --url https://github.com/mackhan/KVMini/releases/download/v0.1.28/KVMini.app.tar.gz \
 *     --signature <sig-content> \
 *     --output path/to/latest.json \
 *     [--existing path/to/old-latest.json]
 *
 * 也可以作为模块 require，调用 generateLatestJson(options)。
 */

const fs = require('fs');
const path = require('path');

/**
 * 生成/合并 latest.json
 * @param {Object} options
 * @param {string} options.version     - 版本号（如 "0.1.28"）
 * @param {string} options.tag         - tag 名（如 "v0.1.28"）
 * @param {string} options.platformKey - 平台 key（如 "darwin-aarch64", "windows-x86_64"）
 * @param {string} options.url         - 产物下载 URL
 * @param {string} options.signature   - 签名内容
 * @param {string} options.outputPath  - 输出 latest.json 路径
 * @param {string} [options.existingPath] - 已有的 latest.json 路径（用于合并其他平台条目）
 * @returns {Object} 生成的 JSON 对象
 */
function generateLatestJson(options) {
  const { version, tag, platformKey, url, signature, outputPath, existingPath } = options;

  // 基础数据
  const data = {
    version: version,
    notes: `KVMini ${tag} release`,
    pub_date: new Date().toISOString(),
    platforms: {}
  };

  // 尝试合并已有的 latest.json
  if (existingPath) {
    try {
      const old = JSON.parse(fs.readFileSync(existingPath, 'utf8'));
      data.platforms = old.platforms || {};
      // 保留旧的 notes（用户可能已编辑过）
      if (old.notes) data.notes = old.notes;
    } catch (e) {
      console.log(`[warn] Cannot parse existing latest.json: ${e.message}`);
    }
  }

  // 添加/覆盖本次平台条目
  if (url && signature) {
    data.platforms[platformKey] = {
      signature: signature.trim(),
      url: url
    };
  }

  // 确保输出目录存在
  const outputDir = path.dirname(outputPath);
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  // 写入文件
  fs.writeFileSync(outputPath, JSON.stringify(data, null, 2), 'utf8');

  console.log(`[info] latest.json generated: ${outputPath}`);
  console.log(`[info] platforms: ${Object.keys(data.platforms).join(', ')}`);

  return data;
}

// CLI 入口
if (require.main === module) {
  const args = process.argv.slice(2);

  function getArg(name) {
    const idx = args.indexOf(`--${name}`);
    return idx !== -1 && idx + 1 < args.length ? args[idx + 1] : null;
  }

  const version = getArg('version');
  const tag = getArg('tag');
  const platformKey = getArg('platform');
  const url = getArg('url');
  const outputPath = getArg('output');
  const existingPath = getArg('existing');

  // signature 可能很长，支持从文件读取
  let signature = getArg('signature');
  const sigFile = getArg('signature-file');
  if (sigFile && fs.existsSync(sigFile)) {
    signature = fs.readFileSync(sigFile, 'utf8').trim();
  }

  if (!version || !tag || !platformKey || !url || !signature || !outputPath) {
    console.error('Usage: node generate-latest-json.js --version <ver> --tag <tag> --platform <key> --url <url> --signature <sig> --output <path> [--existing <path>] [--signature-file <path>]');
    process.exit(1);
  }

  generateLatestJson({ version, tag, platformKey, url, signature, outputPath, existingPath });
}

module.exports = { generateLatestJson };
