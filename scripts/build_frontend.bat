@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Building frontend...
cd frontend
pnpm run build
if errorlevel 1 goto fail

echo.
echo [RustTool] Frontend build finished: frontend\dist
goto end

:fail
echo.
echo [RustTool] Frontend build failed.
exit /b 1

:end
endlocal
