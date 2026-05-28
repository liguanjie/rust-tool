$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$root = Split-Path -Parent $PSScriptRoot
$rt = Join-Path $root "rt.bat"

function Invoke-RustToolCommand {
    param([string] $Command)

    Write-Host ""
    & $rt $Command
    Write-Host ""
    Read-Host "按回车返回菜单"
}

function Get-PortStatus {
    param([int]$Port)
    $conn = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
    if ($conn) {
        $pid = $conn[0].OwningProcess
        $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
        $name = if ($proc) { $proc.Name } else { "Unknown" }
        return [PSCustomObject]@{
            Active = $true
            PID = $pid
            Name = $name
        }
    }
    return [PSCustomObject]@{
        Active = $false
        PID = $null
        Name = $null
    }
}

function Stop-PortProcess {
    param([int]$Port)
    $conns = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue
    $killed = 0
    if ($conns) {
        $pids = $conns.OwningProcess | Select-Object -Unique
        foreach ($pid in $pids) {
            if ($pid -and $pid -ne 0 -and $pid -ne $PID) {
                $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
                if ($proc) {
                    Write-Host "正在停止端口 $Port 的关联进程: $($proc.Name) (PID: $pid)..." -ForegroundColor Yellow
                    Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
                    $killed++
                }
            }
        }
    }
    return $killed
}

while ($true) {
    Clear-Host
    
    # 获取服务运行状态
    $backStatus = Get-PortStatus 5172
    $frontStatus = Get-PortStatus 5173

    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host " RustTool 控制台开发面板" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host " [状态监测]" -ForegroundColor Gray
    
    if ($backStatus.Active) {
        Write-Host "   - 后端服务 (127.0.0.1:5172): " -NoNewline -ForegroundColor Gray
        Write-Host "[● 运行中] (PID: $($backStatus.PID), $($backStatus.Name))" -ForegroundColor Green
    } else {
        Write-Host "   - 后端服务 (127.0.0.1:5172): " -NoNewline -ForegroundColor Gray
        Write-Host "[○ 已停止]" -ForegroundColor Red
    }

    if ($frontStatus.Active) {
        Write-Host "   - 前端网页 (127.0.0.1:5173): " -NoNewline -ForegroundColor Gray
        Write-Host "[● 运行中] (PID: $($frontStatus.PID), $($frontStatus.Name))" -ForegroundColor Green
    } else {
        Write-Host "   - 前端网页 (127.0.0.1:5173): " -NoNewline -ForegroundColor Gray
        Write-Host "[○ 已停止]" -ForegroundColor Red
    }
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "  1. 安装前端依赖"
    Write-Host ""
    Write-Host "  [开发调试]" -ForegroundColor DarkGray
    Write-Host "  2. 启动 Web 开发环境 (网页前端 + 后端服务)"
    Write-Host "  3. ★ 启动桌面开发版 (Tauri 桌面 + 后端服务)" -ForegroundColor Yellow
    Write-Host "  4. 启动单独后端服务 (仅启动 5172 端口)"
    Write-Host ""
    Write-Host "  [构建与测试]" -ForegroundColor DarkGray
    Write-Host "  5. 运行测试 (Rust + Web)"
    Write-Host "  6. 打包 Web release exe (内嵌前端单文件)"
    Write-Host "  7. ★ 打包桌面版 exe (Tauri 独立客户端)" -ForegroundColor Yellow
    Write-Host "  8. 运行 Web release 服务"
    Write-Host ""
    Write-Host "  [维护管理]" -ForegroundColor DarkGray
    Write-Host "  9. ★ 一键停止所有开发服务 (释放 5172 与 5173 端口)" -ForegroundColor Yellow
    Write-Host "  10. 清理构建产物"
    Write-Host "  0. 退出"
    Write-Host ""

    $choice = Read-Host "请选择操作"

    switch ($choice) {
        "1" { Invoke-RustToolCommand "install" }
        "2" {
            if ($backStatus.Active -or $frontStatus.Active) {
                $confirm = Read-Host "检测到服务已在后台运行，是否重新启动？(Y/N)"
                if ($confirm -ieq 'y') {
                    Write-Host "正在释放旧服务端口..."
                    $null = Stop-PortProcess 5172
                    $null = Stop-PortProcess 5173
                    Start-Sleep -Seconds 1
                    Invoke-RustToolCommand "dev"
                }
            } else {
                Invoke-RustToolCommand "dev"
            }
        }
        "3" {
            if ($backStatus.Active) {
                $confirm = Read-Host "检测到后端服务已在运行，启动桌面版前是否重启后端？(Y/N)"
                if ($confirm -ieq 'y') {
                    $null = Stop-PortProcess 5172
                    Start-Sleep -Seconds 1
                }
            }
            Invoke-RustToolCommand "desktop"
        }
        "4" {
            if ($backStatus.Active) {
                Write-Host "后端服务已在运行 (PID: $($backStatus.PID))，无需重复启动。" -ForegroundColor Yellow
                Read-Host "按回车返回菜单"
            } else {
                Invoke-RustToolCommand "server"
            }
        }
        "5" { Invoke-RustToolCommand "test" }
        "6" { Invoke-RustToolCommand "build" }
        "7" { Invoke-RustToolCommand "build-desktop" }
        "8" { Invoke-RustToolCommand "run" }
        "9" {
            Write-Host "正在清理后台开发服务进程..." -ForegroundColor Cyan
            $k2 = Stop-PortProcess 5172
            $k3 = Stop-PortProcess 5173
            Write-Host "清理完毕。共终止了 $($k2 + $k3) 个关联进程。" -ForegroundColor Green
            Read-Host "按回车返回菜单"
        }
        "10" { Invoke-RustToolCommand "clean" }
        "0" { exit 0 }
        default {
            Write-Host ""
            Write-Host "无效选项，请重新选择。" -ForegroundColor Red
            Read-Host "按回车继续"
        }
    }
}
