@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Cleaning Rust target...
cargo clean

echo.
echo [RustTool] Removing frontend dist...
if exist "frontend\dist" rmdir /s /q "frontend\dist"

echo.
echo [RustTool] Clean finished.

endlocal
