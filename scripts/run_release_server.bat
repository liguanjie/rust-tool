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
echo [RustTool] Open http://127.0.0.1:8080
target\release\rust_tool_server.exe

endlocal
