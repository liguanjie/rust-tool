@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Running Rust tests...
cargo test
if errorlevel 1 goto fail

echo.
echo [RustTool] Running frontend tests...
cd frontend
pnpm run test:run
if errorlevel 1 goto fail

echo.
echo [RustTool] All tests passed.
goto end

:fail
echo.
echo [RustTool] Tests failed.
exit /b 1

:end
endlocal
