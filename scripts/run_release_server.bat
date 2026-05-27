@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

if not exist "target\release\rust_tool_server.exe" (
  echo [RustTool] target\release\rust_tool_server.exe not found.
  echo [RustTool] Run scripts\build_release.bat first.
  exit /b 1
)

echo [RustTool] Starting release server...
if "%RUSTTOOL_SERVER_PORT%"=="" (
  set "RUSTTOOL_SERVER_PORT=8080"
)
if "%RUSTTOOL_SERVER_HOST%"=="" (
  set "RUSTTOOL_SERVER_HOST=127.0.0.1"
)
echo [RustTool] Open http://%RUSTTOOL_SERVER_HOST%:%RUSTTOOL_SERVER_PORT%
target\release\rust_tool_server.exe %*

endlocal
