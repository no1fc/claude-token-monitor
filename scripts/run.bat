@echo off
REM ============================================================
REM  Claude Token Monitor - run immediately (double-click me)
REM  Launches the built release app if present; otherwise falls
REM  back to dev mode (keeps this window open).
REM ============================================================
setlocal
title Claude Token Monitor
cd /d "%~dp0.."

set "EXE=src-tauri\target\release\claude-token-monitor.exe"

if exist "%EXE%" (
    echo Starting Claude Token Monitor...
    start "" "%EXE%"
    goto :end
)

echo No release build found.
echo Launching in development mode - keep this window open.
echo (To create a standalone build, run: npm run tauri build)
echo.

where npm >nul 2>nul
if errorlevel 1 (
    echo [ERROR] npm was not found on PATH. Install Node.js first:
    echo   https://nodejs.org/
    echo.
    pause
    goto :end
)

call npm install
if errorlevel 1 (
    echo.
    echo [ERROR] npm install failed - see the messages above.
    pause
    goto :end
)

call npm run tauri dev
if errorlevel 1 (
    echo.
    echo [ERROR] Dev launch failed. A common cause on a fresh shell is that
    echo         Rust/cargo is not on PATH. Install Rust from:
    echo   https://rustup.rs/
    echo         then close and reopen this window.
    pause
)

:end
endlocal
