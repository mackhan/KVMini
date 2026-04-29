@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

title KVMini - Windows Build

echo.
echo ============================================
echo   KVMini Windows Build Tool
echo ============================================
echo.

:: 切到脚本所在目录（项目根目录）
cd /d "%~dp0"

:: ── Step 1: 单元测试 ─────────────────────────
echo [1/4] Running tests...
call npm test --silent 2>&1
if %errorlevel% neq 0 (
    echo.
    echo [FAILED] Tests failed. Build aborted.
    pause
    exit /b 1
)
echo [OK] Tests passed.
echo.

:: ── Step 2: 更新版本号 ────────────────────────
echo [2/4] Updating version...
call node scripts/version-update.js 2>&1
if %errorlevel% neq 0 (
    echo.
    echo [FAILED] Version update failed. Build aborted.
    pause
    exit /b 1
)
echo [OK] Version updated.
echo.

:: ── Step 3: 构建 ──────────────────────────────
echo [3/4] Building (tauri build)...
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
    echo [FAILED] Executable not found: %EXE%
    pause
    exit /b 1
)

echo [4/4] Launching KVMini...
start "" "%EXE%"
echo [OK] KVMini launched.
echo.
echo ============================================
echo   Build complete!
echo ============================================
echo.
exit /b 0
