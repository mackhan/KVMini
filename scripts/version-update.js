#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// 文件路径
const packageJsonPath = path.join(__dirname, '..', 'package.json');
const cargoTomlPath = path.join(__dirname, '..', 'src-tauri', 'Cargo.toml');
const workspaceCargoTomlPath = path.join(__dirname, '..', 'Cargo.toml');
const indexHtmlPath = path.join(__dirname, '..', 'src', 'index.html');

// GitHub 仓库（用于 --check-tag 模式）
const GH_REPO = 'mackhan/KVMini';

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

// 检查 GitHub Release tag 是否已存在
function checkTagExists(tag, repo) {
  try {
    execSync(`gh release view ${tag} --repo ${repo}`, { stdio: 'pipe' });
    return true;
  } catch (e) {
    return false;
  }
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

// 更新根目录 workspace Cargo.toml 版本号
function updateWorkspaceCargoToml(newVersion) {
  try {
    let cargoContent = fs.readFileSync(workspaceCargoTomlPath, 'utf8');
    
    // 匹配 [workspace.package] 块中的 version 行
    cargoContent = cargoContent.replace(
      /(\[workspace\.package\][^\[]*version\s*=\s*\")[0-9.]+(\")/s,
      `$1${newVersion}$2`
    );
    
    fs.writeFileSync(workspaceCargoTomlPath, cargoContent);
    console.log(`✅ Updated workspace Cargo.toml version to ${newVersion}`);
  } catch (error) {
    console.error('Error updating workspace Cargo.toml:', error.message);
    process.exit(1);
  }
}

// 更新 src/index.html 中的 app-version meta 标签
function updateIndexHtmlVersion(newVersion) {
  try {
    let htmlContent = fs.readFileSync(indexHtmlPath, 'utf8');
    htmlContent = htmlContent.replace(
      /<meta name="app-version" content="[0-9.]+"/,
      `<meta name="app-version" content="${newVersion}"`
    );
    fs.writeFileSync(indexHtmlPath, htmlContent);
    console.log(`✅ Updated index.html app-version meta to ${newVersion}`);
  } catch (error) {
    console.error('Error updating index.html:', error.message);
    process.exit(1);
  }
}

// 执行所有文件的版本更新
function bumpToVersion(newVersion) {
  updatePackageJson(newVersion);
  updateCargoToml(newVersion);
  updateWorkspaceCargoToml(newVersion);
  updateIndexHtmlVersion(newVersion);
}

// 主函数（无参数模式 —— 无条件 bump）
function main() {
  console.log('🚀 Starting version update...');
  
  const currentVersion = getCurrentVersion();
  console.log(`📋 Current version: ${currentVersion}`);
  
  const newVersion = incrementVersion(currentVersion);
  console.log(`🆕 New version: ${newVersion}`);
  
  bumpToVersion(newVersion);
  
  console.log('🎉 Version update completed successfully!');
  console.log(`\nNext steps:`);
  console.log(`1. Run tests: npm test`);
  console.log(`2. Build release: npm run build`);
  console.log(`3. Commit with message: [${newVersion}] Your changes`);
  
  return newVersion;
}

// --check-tag 模式：检测 tag 存在性，决定是否 bump
// exit 0 = 已 bump（tag 不存在，正常递增）
// exit 2 = 跳过 bump（tag 已存在，另一个平台已构建）
// exit 1 = 错误
function mainCheckTag() {
  console.log('🔍 Checking tag before version bump...');

  const currentVersion = getCurrentVersion();
  const nextVersion = incrementVersion(currentVersion);
  const tag = `v${nextVersion}`;

  console.log(`📋 Current version: ${currentVersion}`);
  console.log(`🔮 Next version would be: ${nextVersion} (tag: ${tag})`);

  // 检查 gh CLI 是否可用
  try {
    execSync('gh --version', { stdio: 'pipe' });
  } catch (e) {
    console.error('❌ gh CLI not found. Install with: brew install gh (macOS) or winget install GitHub.cli (Windows)');
    process.exit(1);
  }

  // 检查 tag 是否已存在于 GitHub
  const tagExists = checkTagExists(tag, GH_REPO);

  if (tagExists) {
    console.log(`✅ Tag ${tag} already exists on GitHub — another platform has already built this version.`);

    // 如果当前版本已经是 nextVersion（代码已 pull 过），直接跳过
    if (currentVersion === nextVersion) {
      console.log(`📌 Current version is already ${nextVersion}, skipping bump.`);
    } else {
      // 当前版本还是旧的，但 tag 已存在 → bump 到与 tag 一致的版本
      console.log(`📌 Bumping ${currentVersion} → ${nextVersion} to match existing tag.`);
      bumpToVersion(nextVersion);
    }
    process.exit(2); // 信号：tag 已存在（跳过 tag 创建）
  } else {
    // tag 不存在，正常 bump
    console.log(`🆕 Tag ${tag} does not exist. Bumping version: ${currentVersion} → ${nextVersion}`);
    bumpToVersion(nextVersion);
    process.exit(0); // 信号：已 bump，需要创建 tag
  }
}

// 入口
if (require.main === module) {
  const args = process.argv.slice(2);
  if (args.includes('--check-tag')) {
    mainCheckTag();
  } else {
    main();
  }
}

module.exports = { validateVersion, getCurrentVersion, incrementVersion, checkTagExists, updatePackageJson, updateCargoToml, updateWorkspaceCargoToml, updateIndexHtmlVersion, bumpToVersion, main };
