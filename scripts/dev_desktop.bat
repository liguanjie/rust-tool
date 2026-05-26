@echo off
setlocal
cd /d "%~dp0..\frontend"
call "%~dp0env.bat"

echo [RustTool] Starting Tauri desktop dev app...
pnpm run tauri:dev

endlocal
