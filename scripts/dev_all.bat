@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Starting backend and frontend in separate windows...
start "RustTool Server" cmd /k "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cd /d %cd% && cargo run -p rust_tool_server"
start "RustTool Frontend" cmd /k "cd /d %cd%\frontend && pnpm run dev"

echo.
echo [RustTool] Backend:  http://127.0.0.1:8080
echo [RustTool] Frontend: http://127.0.0.1:5173
echo.
echo Close the opened terminal windows to stop services.

endlocal
