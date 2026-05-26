@echo off
setlocal
cd /d "%~dp0..\frontend"
call "%~dp0env.bat"

echo [RustTool] Building Tauri desktop app...
pnpm run tauri:build
if errorlevel 1 goto fail

echo.
echo [RustTool] Desktop build finished.
echo [RustTool] Bundles are under frontend\src-tauri\target\release\bundle
goto end

:fail
echo.
echo [RustTool] Desktop build failed.
exit /b 1

:end
endlocal
