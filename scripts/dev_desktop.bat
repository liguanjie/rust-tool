@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Starting backend server in background...
start "RustTool Server" cmd /k "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cd /d %cd% && cargo run -p rust_tool_server"

echo [RustTool] Starting Tauri desktop dev app...
cd /d "%~dp0..\frontend"
pnpm run tauri:dev

endlocal
