#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// 文件路径
const packageJsonPath = path.join(__dirname, '..', 'package.json');
const cargoTomlPath = path.join(__dirname, '..', 'src-tauri', 'Cargo.toml');

// 验证版本格式
function validateVersion(version) {
  const versionRegex = /^\d+\.\d+\.\d+$/;
  if (!versionRegex.test(version)) {
    throw new Error(`Invalid version format: ${version}. Expected format: x.y.z`);
  }
}

// 读取当前版本号
function getCurrentVersion() {
  try {
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    return packageJson.version;
  } catch (error) {
    console.error('Error reading package.json:', error.message);
    process.exit(1);
  }
}

// 递增版本号 (patch版本)
function incrementVersion(version) {
  validateVersion(version);
  const parts = version.split('.').map(Number);
  parts[2] += 1; // 递增patch版本
  return parts.join('.');
}

// 更新package.json版本号
function updatePackageJson(newVersion) {
  try {
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    packageJson.version = newVersion;
    fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
    console.log(`✅ Updated package.json version to ${newVersion}`);
  } catch (error) {
    console.error('Error updating package.json:', error.message);
    process.exit(1);
  }
}

// 更新Cargo.toml版本号
function updateCargoToml(newVersion) {
  try {
    let cargoContent = fs.readFileSync(cargoTomlPath, 'utf8');
    
    // 使用正则表达式匹配版本号行
    cargoContent = cargoContent.replace(
      /^(version\s*=\s*\")[0-9.]+(\")/m,
      `$1${newVersion}$2`
    );
    
    fs.writeFileSync(cargoTomlPath, cargoContent);
    console.log(`✅ Updated Cargo.toml version to ${newVersion}`);
  } catch (error) {
    console.error('Error updating Cargo.toml:', error.message);
    process.exit(1);
  }
}

// 主函数
function main() {
  console.log('🚀 Starting version update...');
  
  const currentVersion = getCurrentVersion();
  console.log(`📋 Current version: ${currentVersion}`);
  
  const newVersion = incrementVersion(currentVersion);
  console.log(`🆕 New version: ${newVersion}`);
  
  updatePackageJson(newVersion);
  updateCargoToml(newVersion);
  
  console.log('🎉 Version update completed successfully!');
  console.log(`\nNext steps:`);
  console.log(`1. Run tests: npm test`);
  console.log(`2. Build release: npm run build`);
  console.log(`3. Commit with message: [${newVersion}] Your changes`);
  
  return newVersion;
}

// 如果是直接执行
if (require.main === module) {
  main();
}

module.exports = { validateVersion, getCurrentVersion, incrementVersion, updatePackageJson, updateCargoToml, main };