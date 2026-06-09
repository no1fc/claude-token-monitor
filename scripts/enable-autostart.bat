@echo off
REM ============================================================
REM  Enable auto-start at login by placing a shortcut to the
REM  built app in the Windows Startup folder.
REM  (Alternative to the in-app "Start automatically" toggle.
REM   Running both is safe - the app prevents duplicate windows.)
REM ============================================================
setlocal
cd /d "%~dp0.."

set "EXE=%CD%\src-tauri\target\release\claude-token-monitor.exe"

if not exist "%EXE%" (
    echo Release build not found at:
    echo   "%EXE%"
    echo Build it first with:  npm run tauri build
    echo.
    pause
    exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$startup=[Environment]::GetFolderPath('Startup');" ^
  "$lnk=Join-Path $startup 'Claude Token Monitor.lnk';" ^
  "$ws=New-Object -ComObject WScript.Shell;" ^
  "$s=$ws.CreateShortcut($lnk);" ^
  "$s.TargetPath='%EXE%';" ^
  "$s.WorkingDirectory=[IO.Path]::GetDirectoryName('%EXE%');" ^
  "$s.Description='Claude Token Monitor';" ^
  "$s.Save();" ^
  "Write-Host ('Autostart enabled -> ' + $lnk)"

if errorlevel 1 (
    echo.
    echo [ERROR] Failed to create the Startup shortcut.
    pause
    exit /b 1
)

echo.
echo The app will now launch automatically when you sign in.
pause
endlocal
