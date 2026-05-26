@echo off
setlocal
cd /d "%~dp0"
call "%~dp0scripts\env.bat"

if "%~1"=="" goto help
if /i "%~1"=="install" goto install
if /i "%~1"=="dev" goto dev
if /i "%~1"=="desktop" goto desktop
if /i "%~1"=="server" goto server
if /i "%~1"=="frontend" goto frontend
if /i "%~1"=="test" goto test
if /i "%~1"=="build" goto build
if /i "%~1"=="build-desktop" goto build_desktop
if /i "%~1"=="run" goto run
if /i "%~1"=="clean" goto clean
goto help

:install
call "%~dp0scripts\install_frontend.bat"
exit /b %errorlevel%

:dev
call "%~dp0scripts\dev_all.bat"
exit /b %errorlevel%

:desktop
call "%~dp0scripts\dev_desktop.bat"
exit /b %errorlevel%

:server
call "%~dp0scripts\dev_server.bat"
exit /b %errorlevel%

:frontend
call "%~dp0scripts\dev_frontend.bat"
exit /b %errorlevel%

:test
call "%~dp0scripts\test_all.bat"
exit /b %errorlevel%

:build
call "%~dp0scripts\build_release.bat"
exit /b %errorlevel%

:build_desktop
call "%~dp0scripts\build_desktop.bat"
exit /b %errorlevel%

:run
call "%~dp0scripts\run_release_server.bat"
exit /b %errorlevel%

:clean
call "%~dp0scripts\clean.bat"
exit /b %errorlevel%

:help
echo RustTool commands:
echo.
echo   rt install    Install frontend dependencies
echo   rt dev        Start backend and frontend dev servers
echo   rt desktop    Start Tauri desktop dev app
echo   rt server     Start Rust backend only
echo   rt frontend   Start Vue frontend only
echo   rt test       Run Rust and frontend tests
echo   rt build      Build frontend and release exe files
echo   rt build-desktop  Build Tauri desktop installer/exe
echo   rt run        Run release server
echo   rt clean      Clean build outputs
echo.
exit /b 0
