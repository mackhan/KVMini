#!/usr/bin/env node
/**
 * generate-icons.js - 从 SVG 生成所有尺寸的图标文件
 * 使用 sharp 库将 icon.svg 转换为各种尺寸的 PNG
 */

const sharp = require('sharp');
const path = require('path');
const fs = require('fs');

const ICONS_DIR = path.join(__dirname, '../src-tauri/icons');
const SVG_PATH = path.join(ICONS_DIR, 'icon.svg');

// 需要生成的尺寸
const SIZES = [16, 32, 64, 128, 256, 512, 1024];

// 文件名映射（Tauri 需要的标准文件名）
const FILE_NAMES = {
    16:   ['icon_16x16.png', '32x32.png'],
    32:   ['icon_32x32.png'],
    64:   ['icon_64x64.png'],
    128:  ['icon_128x128.png', '128x128.png'],
    256:  ['icon_256x256.png', '256x256.png', '128x128@2x.png'],
    512:  ['icon_512x512.png', '512x512.png'],
    1024: ['icon_1024x1024.png', 'icon.png'],
};

async function generateIcons() {
    console.log('[图标生成] 开始从 SVG 生成所有尺寸图标...');

    const svgBuffer = fs.readFileSync(SVG_PATH);

    for (const size of SIZES) {
        const fileNames = FILE_NAMES[size] || [`icon_${size}x${size}.png`];
        for (const fileName of fileNames) {
            const outputPath = path.join(ICONS_DIR, fileName);
            await sharp(svgBuffer)
                .resize(size, size)
                .png()
                .toFile(outputPath);
            console.log(`  ✅ 生成 ${fileName} (${size}x${size})`);
        }
    }

    console.log('[图标生成] 所有 PNG 图标生成完成！');
    console.log('[图标生成] 注意：icon.icns 和 icon.ico 需要手动生成或使用 tauri icon 命令');
}

generateIcons().catch(err => {
    console.error('[图标生成] 失败:', err);
    process.exit(1);
});
