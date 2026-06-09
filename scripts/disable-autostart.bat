@echo off
REM ============================================================
REM  Disable auto-start by removing the Startup-folder shortcut
REM  created by enable-autostart.bat.
REM ============================================================
setlocal

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$lnk=Join-Path ([Environment]::GetFolderPath('Startup')) 'Claude Token Monitor.lnk';" ^
  "if (Test-Path $lnk) { Remove-Item $lnk -Force; Write-Host 'Autostart disabled (shortcut removed).' }" ^
  "else { Write-Host 'No Startup shortcut found - nothing to remove.' }"

echo.
pause
endlocal
