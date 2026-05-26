@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Building frontend first...
cd frontend
pnpm run build
if errorlevel 1 goto fail

cd ..
echo.
echo [RustTool] Building Rust release binaries...
cargo build --release
if errorlevel 1 goto fail

echo.
echo [RustTool] Release build finished.
echo [RustTool] Server: target\release\rust_tool_server.exe
echo [RustTool] CLI:    target\release\rust-tool.exe
goto end

:fail
echo.
echo [RustTool] Release build failed.
exit /b 1

:end
endlocal
