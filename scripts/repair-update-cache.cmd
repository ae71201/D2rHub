@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0repair-update-cache.ps1"
set repair_result=%errorlevel%
pause
exit /b %repair_result%
