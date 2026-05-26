@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Starting Vue dev server...
cd frontend
pnpm run dev

endlocal
