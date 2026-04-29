@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

title KVMini - Windows Build Tool

echo.
echo ============================================
echo   KVMini Windows Build Tool
echo ============================================
echo.

:: 切到脚本所在目录（项目根目录）
cd /d "%~dp0"

:: ── 解析参数 ────────────────────────────────
set UPLOAD_RELEASE=false
for %%a in (%*) do (
    if "%%a"=="--upload" set UPLOAD_RELEASE=true
)

:: ── Step 1: 单元测试 ─────────────────────────
echo [1/5] Running tests...
call npm test --silent 2>&1
if %errorlevel% neq 0 (
    echo.
    echo [FAILED] Tests failed. Build aborted.
    pause
    exit /b 1
)
echo [OK] Tests passed.
echo.

:: ── Step 2: 智能版本号管理 ───────────────────
echo [2/5] Checking version...
set TAG_EXISTS=false
call node scripts/version-update.js --check-tag
set BUMP_RESULT=%errorlevel%
if %BUMP_RESULT%==2 (
    echo [OK] Tag exists on GitHub, skipping bump (another platform already built^).
    set TAG_EXISTS=true
) else if %BUMP_RESULT%==0 (
    echo [OK] Version bumped.
) else (
    echo.
    echo [FAILED] Version check failed (exit code: %BUMP_RESULT%^). Build aborted.
    pause
    exit /b 1
)
echo.

:: ── Step 3: 构建 ──────────────────────────────
echo [3/5] Building (tauri build)...
echo       This may take a few minutes...
call npm run build 2>&1
if %errorlevel% neq 0 (
    echo.
    echo [FAILED] Build failed. Check output above.
    pause
    exit /b 1
)
echo [OK] Build succeeded.
echo.

:: ── Step 4: 启动程序 ──────────────────────────
set EXE=target\release\kvmini.exe
if not exist "%EXE%" (
    echo [WARN] Executable not found: %EXE%, skipping launch.
) else (
    echo [4/5] Launching KVMini...
    start "" "%EXE%"
    echo [OK] KVMini launched.
)
echo.

:: ── Step 5: 上传到 GitHub Releases（可选） ───
if "%UPLOAD_RELEASE%"=="false" goto :build_done

echo [5/5] Uploading to GitHub Releases...
echo.

:: 检查 gh CLI
where gh >nul 2>&1
if %errorlevel% neq 0 (
    echo [FAILED] gh CLI not found. Install with: winget install GitHub.cli
    pause
    exit /b 1
)

:: 检查 gh 登录
gh auth status >nul 2>&1
if %errorlevel% neq 0 (
    echo [FAILED] gh CLI not logged in. Run: gh auth login
    pause
    exit /b 1
)

:: 读取版本号（从 package.json）
for /f "tokens=2 delims=:, " %%v in ('findstr /r "\"version\"" package.json') do (
    set VERSION=%%~v
    goto :got_version
)
:got_version
set TAG=v%VERSION%
echo   Version: %VERSION%  Tag: %TAG%
set GH_REPO=mackhan/KVMini

:: 查找构建产物
set ARTIFACT_COUNT=0
set ARTIFACTS=

:: NSIS exe
for %%f in (src-tauri\target\release\bundle\nsis\*.exe) do (
    set /a ARTIFACT_COUNT+=1
    set ARTIFACTS=!ARTIFACTS! "%%f"
)
:: NSIS zip (updater)
set WIN_ZIP=
for %%f in (src-tauri\target\release\bundle\nsis\*.nsis.zip) do (
    set /a ARTIFACT_COUNT+=1
    set ARTIFACTS=!ARTIFACTS! "%%f"
    set WIN_ZIP=%%f
)
:: NSIS sig
set WIN_SIG=
for %%f in (src-tauri\target\release\bundle\nsis\*.nsis.zip.sig) do (
    set /a ARTIFACT_COUNT+=1
    set ARTIFACTS=!ARTIFACTS! "%%f"
    set WIN_SIG=%%f
)

if %ARTIFACT_COUNT%==0 (
    echo [FAILED] No build artifacts found.
    pause
    exit /b 1
)
echo   Found %ARTIFACT_COUNT% artifact(s).

:: 生成 latest.json（使用 Node.js 跨平台脚本）
set LATEST_JSON=src-tauri\target\release\bundle\latest.json
set EXISTING_ARG=

:: 尝试下载已有 latest.json
gh release view %TAG% --repo %GH_REPO% >nul 2>&1
if %errorlevel%==0 (
    set TMP_LATEST=%TEMP%\kvmini-latest-old.json
    gh release download %TAG% --repo %GH_REPO% --pattern "latest.json" --output "!TMP_LATEST!" >nul 2>&1
    if !errorlevel!==0 (
        set EXISTING_ARG=--existing "!TMP_LATEST!"
        echo   Downloaded existing latest.json for merge.
    ) else (
        del /f "!TMP_LATEST!" >nul 2>&1
    )
)

:: 生成/合并 latest.json
if defined WIN_ZIP if defined WIN_SIG (
    for %%z in (%WIN_ZIP%) do set WIN_ZIP_NAME=%%~nxz
    set WIN_URL=https://github.com/%GH_REPO%/releases/download/%TAG%/!WIN_ZIP_NAME!
    call node scripts/generate-latest-json.js --version "%VERSION%" --tag "%TAG%" --platform "windows-x86_64" --url "!WIN_URL!" --signature-file "%WIN_SIG%" --output "%LATEST_JSON%" !EXISTING_ARG!
    set ARTIFACTS=!ARTIFACTS! "%LATEST_JSON%"
    echo   Generated latest.json.
) else (
    echo   [WARN] No updater signature file found, skipping latest.json.
)

:: 清理临时文件
if defined TMP_LATEST del /f "%TMP_LATEST%" >nul 2>&1

:: 创建 tag（如果不存在）
if "%TAG_EXISTS%"=="false" (
    git rev-parse %TAG% >nul 2>&1
    if !errorlevel! neq 0 (
        echo   Creating tag: %TAG%
        git tag -a %TAG% -m "Release %TAG%"
    )
    :: 推送 tag 到 GitHub
    git remote get-url github >nul 2>&1
    if !errorlevel!==0 (
        git push github %TAG% 2>nul
        if !errorlevel! neq 0 echo   [WARN] Tag push failed (may already exist on remote^).
    ) else (
        echo   [WARN] github remote not configured, skipping tag push.
    )
) else (
    echo   Tag %TAG% already exists on GitHub, skipping creation.
)

:: 创建或更新 Release
gh release view %TAG% --repo %GH_REPO% >nul 2>&1
if %errorlevel%==0 (
    echo   Release %TAG% exists, uploading artifacts...
    gh release upload %TAG% %ARTIFACTS% --clobber --repo %GH_REPO%
) else (
    echo   Creating Release %TAG%...
    gh release create %TAG% %ARTIFACTS% --title "KVMini %TAG%" --notes "KVMini %TAG% release" --draft --repo %GH_REPO%
    echo   Release created as draft. Visit GitHub to edit and publish.
)

echo.
echo   Upload complete!

:build_done
echo.
echo ============================================
echo   Build complete!
echo ============================================
echo.
exit /b 0
