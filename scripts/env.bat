@echo off
set "RUSTTOOL_ROOT=%~dp0.."

if exist "%USERPROFILE%\.cargo\bin" (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

