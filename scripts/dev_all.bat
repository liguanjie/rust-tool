@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Starting backend and frontend in separate windows...
start "RustTool Server" cmd /k "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cd /d %cd% && cargo run -p rust_tool_server -- %*"
start "RustTool Frontend" cmd /k "cd /d %cd%\frontend && pnpm run dev"

echo.
if "%RUSTTOOL_SERVER_PORT%"=="" (
  set "RUSTTOOL_SERVER_PORT=8080"
)
if "%RUSTTOOL_SERVER_HOST%"=="" (
  set "RUSTTOOL_SERVER_HOST=127.0.0.1"
)
echo [RustTool] Backend:  http://%RUSTTOOL_SERVER_HOST%:%RUSTTOOL_SERVER_PORT%
echo [RustTool] Frontend: http://127.0.0.1:5173
echo.
echo Close the opened terminal windows to stop services.

endlocal
