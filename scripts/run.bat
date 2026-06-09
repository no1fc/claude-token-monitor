@echo off
REM ============================================================
REM  Claude Token Monitor - run immediately (double-click me)
REM  Launches the built release app if present; otherwise falls
REM  back to dev mode (keeps this window open).
REM ============================================================
setlocal
cd /d "%~dp0.."

set "EXE=src-tauri\target\release\claude-token-monitor.exe"

if exist "%EXE%" (
    echo Starting Claude Token Monitor...
    start "" "%EXE%"
    goto :eof
)

echo No release build found.
echo Launching in development mode - keep this window open.
echo (To create a standalone build, run: npm run tauri build)
echo.
call npm install
call npm run tauri dev
endlocal
