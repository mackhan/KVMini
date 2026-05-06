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

// 决策纯函数：根据 current/next tag 存在性与 BUMP_VERSION 环境变量，
// 决定 bump 行为与退出码。抽出来便于单元测试。
//
// 入参：
//   currentVersion: 当前 package.json 版本
//   currentTagExists: v{current} 是否在远端已发布
//   nextTagExists: v{next} 是否在远端已发布（仅当 currentTagExists=true 时有意义）
//   bumpDisabled: BUMP_VERSION=0 硬开关
// 返回：
//   { action: 'skip' | 'bump', targetVersion: string, exitCode: 0 | 2 }
//     action='skip'：不 bump，使用 currentVersion 作为发布版本
//     action='bump'：bump 到 targetVersion
//     exitCode=0：需要创建 tag；exitCode=2：跳过 tag 创建
function decideBumpAction({ currentVersion, currentTagExists, nextTagExists, bumpDisabled }) {
  const nextVersion = incrementVersion(currentVersion);

  // 硬开关：永远不 bump
  if (bumpDisabled) {
    return {
      action: 'skip',
      targetVersion: currentVersion,
      exitCode: currentTagExists ? 2 : 0,
    };
  }

  // 当前版本尚未发布 → 当前就是待发布版本，不 bump
  if (!currentTagExists) {
    return { action: 'skip', targetVersion: currentVersion, exitCode: 0 };
  }

  // 当前版本已发布 → 需要 bump
  if (nextTagExists) {
    // next 也已发布（另一平台先建）→ bump 对齐但跳过 tag 创建
    return { action: 'bump', targetVersion: nextVersion, exitCode: 2 };
  }
  // 正常 bump 并创建 tag
  return { action: 'bump', targetVersion: nextVersion, exitCode: 0 };
}

// --check-tag 模式：检测 tag 存在性，决定是否 bump
//
// 核心规则（A 方案 · 修订版，避免版本号跳号）：
//   1. 如果 v{current} 在 GitHub 上【不存在】→ 说明当前 package.json 里的版本就是
//      "待发布版本"（手动改的 changelog 版本号），不 bump，直接用当前版本发布。
//   2. 如果 v{current} 已存在 → 当前是"已发布版本"，需要 bump 到 next：
//        a) v{next} 也已存在（另一平台先构建了）→ bump 并跳过 tag 创建 (exit 2)
//        b) v{next} 不存在 → bump 到 next 并创建 tag (exit 0)
//
// 环境变量硬开关：
//   BUMP_VERSION=0 → 永远不 bump，直接用当前版本，exit 0（或 2，若 tag 已存在）
//
// 退出码：
//   0 = 版本已就绪（可能 bump 也可能没），需要创建 tag
//   2 = 跳过 tag 创建（tag 已存在）
//   1 = 错误
function mainCheckTag() {
  console.log('🔍 Checking tag before version bump...');

  const currentVersion = getCurrentVersion();
  const currentTag = `v${currentVersion}`;
  const nextVersion = incrementVersion(currentVersion);
  const nextTag = `v${nextVersion}`;
  const bumpDisabled = process.env.BUMP_VERSION === '0';

  console.log(`📋 Current version: ${currentVersion} (tag: ${currentTag})`);
  if (bumpDisabled) {
    console.log('🔒 BUMP_VERSION=0 set, will skip bump.');
  }

  // gh CLI 检查
  let ghAvailable = true;
  try {
    execSync('gh --version', { stdio: 'pipe' });
  } catch (e) {
    ghAvailable = false;
  }

  if (!ghAvailable) {
    if (bumpDisabled) {
      // 硬开关下允许 gh 缺失，按未发布处理
      console.warn('⚠️  gh CLI unavailable, treating as unreleased.');
      console.log(`📌 Using current version ${currentVersion} as release target.`);
      process.exit(0);
    }
    console.error('❌ gh CLI not found. Install with: brew install gh (macOS) or winget install GitHub.cli (Windows)');
    console.error('   Or set BUMP_VERSION=0 to skip the check entirely.');
    process.exit(1);
  }

  const currentTagExists = checkTagExists(currentTag, GH_REPO);
  // 仅当 currentTagExists=true 时才需要查 next（优化：避免多一次 gh 调用）
  const nextTagExists = currentTagExists ? checkTagExists(nextTag, GH_REPO) : false;

  const decision = decideBumpAction({
    currentVersion,
    currentTagExists,
    nextTagExists,
    bumpDisabled,
  });

  if (decision.action === 'skip') {
    if (bumpDisabled) {
      console.log(`🔒 Skipping bump (BUMP_VERSION=0). Release version: ${currentVersion}`);
    } else {
      console.log(`✨ Tag ${currentTag} does NOT exist on GitHub.`);
      console.log(`📌 Current version ${currentVersion} is the release target. Skipping bump.`);
    }
    if (decision.exitCode === 2) {
      console.log(`📌 Tag ${currentTag} already exists, will skip tag creation.`);
    }
  } else {
    console.log(`✅ Tag ${currentTag} already exists on GitHub (already released).`);
    if (nextTagExists) {
      console.log(`📌 Tag ${nextTag} also exists (another platform built it). Bumping ${currentVersion} → ${decision.targetVersion} to match.`);
    } else {
      console.log(`🆕 Bumping ${currentVersion} → ${decision.targetVersion}`);
    }
    bumpToVersion(decision.targetVersion);
  }

  process.exit(decision.exitCode);
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

module.exports = { validateVersion, getCurrentVersion, incrementVersion, checkTagExists, updatePackageJson, updateCargoToml, updateWorkspaceCargoToml, updateIndexHtmlVersion, bumpToVersion, decideBumpAction, main };
