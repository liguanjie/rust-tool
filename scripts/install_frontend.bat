@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Installing frontend dependencies with pnpm...
cd frontend
pnpm install
if errorlevel 1 goto fail

echo.
echo [RustTool] Frontend dependencies are ready.
goto end

:fail
echo.
echo [RustTool] Failed to install frontend dependencies.
exit /b 1

:end
endlocal
