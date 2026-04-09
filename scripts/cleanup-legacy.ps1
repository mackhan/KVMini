# cleanup-legacy.ps1 - 清理 kvnshare 遗留进程和开机启动项（Windows）
# 用法：PowerShell -ExecutionPolicy Bypass -File scripts\cleanup-legacy.ps1

$LegacyProcess = "kvnshare"
$RegistryRunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"

Write-Host "🔍 检查 kvnshare 遗留进程..." -ForegroundColor Cyan

# 检测并终止遗留进程
$processes = Get-Process -Name $LegacyProcess -ErrorAction SilentlyContinue
if ($processes) {
    Write-Host "⚠️  发现 kvnshare 遗留进程，正在终止..." -ForegroundColor Yellow
    $processes | Stop-Process -Force
    Write-Host "✅ kvnshare 进程已终止" -ForegroundColor Green
} else {
    Write-Host "✅ 未发现 kvnshare 遗留进程" -ForegroundColor Green
}

# 清理注册表开机启动项（当前用户）
Write-Host ""
Write-Host "🔍 检查注册表开机启动项（当前用户）..." -ForegroundColor Cyan
$runValues = Get-ItemProperty -Path $RegistryRunKey -ErrorAction SilentlyContinue
if ($runValues) {
    $kvnshareKeys = $runValues.PSObject.Properties | Where-Object { $_.Name -like "*kvnshare*" -or $_.Name -like "*KVNShare*" }
    if ($kvnshareKeys) {
        foreach ($key in $kvnshareKeys) {
            Remove-ItemProperty -Path $RegistryRunKey -Name $key.Name -ErrorAction SilentlyContinue
            Write-Host "✅ 已移除启动项: $($key.Name)" -ForegroundColor Green
        }
    } else {
        Write-Host "✅ 未发现注册表启动项" -ForegroundColor Green
    }
}

# 清理注册表开机启动项（所有用户，需要管理员权限）
$runAllKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Run"
Write-Host ""
Write-Host "🔍 检查注册表开机启动项（所有用户）..." -ForegroundColor Cyan
$runAllValues = Get-ItemProperty -Path $runAllKey -ErrorAction SilentlyContinue
if ($runAllValues) {
    $kvnshareAllKeys = $runAllValues.PSObject.Properties | Where-Object { $_.Name -like "*kvnshare*" -or $_.Name -like "*KVNShare*" }
    if ($kvnshareAllKeys) {
        foreach ($key in $kvnshareAllKeys) {
            try {
                Remove-ItemProperty -Path $runAllKey -Name $key.Name -ErrorAction Stop
                Write-Host "✅ 已移除系统启动项: $($key.Name)" -ForegroundColor Green
            } catch {
                Write-Host "❌ 移除系统启动项失败（需要管理员权限）: $($key.Name)" -ForegroundColor Red
            }
        }
    } else {
        Write-Host "✅ 未发现系统注册表启动项" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "🎉 清理完成！" -ForegroundColor Green
$kvmProcesses = Get-Process -Name "KVM*" -ErrorAction SilentlyContinue
if ($kvmProcesses) {
    Write-Host "当前运行的 KVMini 进程：" -ForegroundColor Cyan
    $kvmProcesses | Format-Table Name, Id, CPU -AutoSize
} else {
    Write-Host "（无运行中的 KVMini 进程）" -ForegroundColor Gray
}
