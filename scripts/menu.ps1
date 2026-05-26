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

while ($true) {
    Clear-Host
    Write-Host "========================================"
    Write-Host " RustTool"
    Write-Host "========================================"
    Write-Host ""
    Write-Host "  1. 安装前端依赖"
    Write-Host "  2. 启动开发环境"
    Write-Host "  3. 运行测试"
    Write-Host "  4. 打包 release exe"
    Write-Host "  5. 运行 release 服务"
    Write-Host "  6. 清理构建产物"
    Write-Host "  0. 退出"
    Write-Host ""

    $choice = Read-Host "请选择操作"

    switch ($choice) {
        "1" { Invoke-RustToolCommand "install" }
        "2" { Invoke-RustToolCommand "dev" }
        "3" { Invoke-RustToolCommand "test" }
        "4" { Invoke-RustToolCommand "build" }
        "5" { Invoke-RustToolCommand "run" }
        "6" { Invoke-RustToolCommand "clean" }
        "0" { exit 0 }
        default {
            Write-Host ""
            Write-Host "无效选项，请重新选择。"
            Read-Host "按回车继续"
        }
    }
}
