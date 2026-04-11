#!/usr/bin/env node
/**
 * generate-icons.js
 * 从 SVG 源文件生成所有尺寸的 PNG 图标
 * 用法：node scripts/generate-icons.js
 */

const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

const SVG_PATH = path.join(__dirname, '../src-tauri/icons/icon.svg');
const ICONS_DIR = path.join(__dirname, '../src-tauri/icons');

// 需要生成的尺寸
const SIZES = [16, 32, 64, 128, 256, 512, 1024];

// Tauri 需要的特定文件名
const TAURI_ICONS = [
    { size: 32,  name: '32x32.png' },
    { size: 128, name: '128x128.png' },
    { size: 256, name: '128x128@2x.png' },
    { size: 256, name: '256x256.png' },
    { size: 512, name: '512x512.png' },
    { size: 1024, name: 'icon.png' },
];

async function generateIcons() {
    console.log('📦 开始生成图标...');

    const svgBuffer = fs.readFileSync(SVG_PATH);

    // 生成标准尺寸 PNG
    for (const size of SIZES) {
        const outputPath = path.join(ICONS_DIR, `icon_${size}x${size}.png`);
        await sharp(svgBuffer)
            .resize(size, size)
            .png()
            .toFile(outputPath);
        console.log(`  ✅ ${size}x${size}.png`);
    }

    // 生成 Tauri 需要的特定文件名
    for (const { size, name } of TAURI_ICONS) {
        const outputPath = path.join(ICONS_DIR, name);
        await sharp(svgBuffer)
            .resize(size, size)
            .png()
            .toFile(outputPath);
        console.log(`  ✅ ${name} (${size}x${size})`);
    }

    console.log('\n🎉 图标生成完成！');
    console.log(`📁 输出目录: ${ICONS_DIR}`);
}

generateIcons().catch(err => {
    console.error('❌ 图标生成失败:', err);
    process.exit(1);
});
