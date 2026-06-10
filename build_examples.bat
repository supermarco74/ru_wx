@echo off
REM ============================================================
REM  build_examples.bat  -  ru_wx Windows build helper
REM
REM  Builds all examples (regular + minitests) in release mode
REM  and copies the .exe artifacts into:
REM    examples\examples_win32\   (regular demos)
REM    examples\minitest_win32\   (mt_* per-component minitests)
REM ============================================================

setlocal EnableDelayedExpansion

REM Always run from this script's folder (the ru_wx crate root).
cd /d "%~dp0"

REM Force Cargo output into this crate's local `target\` folder.
REM Without this, IDE/sandbox sessions may redirect `CARGO_TARGET_DIR`
REM to a temp cache so `build_examples.bat` copies stale/missing exes.
set "CARGO_TARGET_DIR=%~dp0target"

echo.
echo === ru_wx :: build_examples ================================
echo  Project root : %CD%
echo  CARGO_TARGET_DIR : %CARGO_TARGET_DIR%
echo.

REM ---- 1. Build everything in release mode -------------------
echo [1/3] Compiling all examples in release mode...
cargo build --release --examples
if errorlevel 1 (
    echo.
    echo *** Cargo build FAILED. Aborting. ***
    exit /b 1
)
echo.

REM ---- 2. Prepare destination folders -------------------------
echo [2/3] Preparing destination folders...
if not exist "examples\examples_win32"  mkdir "examples\examples_win32"
if not exist "examples\minitest_win32"  mkdir "examples\minitest_win32"

REM Wipe stale binaries so the folders mirror the current build.
del /Q "examples\examples_win32\*.exe"  >nul 2>&1
del /Q "examples\minitest_win32\*.exe"  >nul 2>&1

REM ---- 3. Distribute the .exe artifacts -----------------------
echo [3/3] Copying compiled binaries...

REM Regular demos: every .exe under target\release\examples\ that
REM does NOT start with "mt_".
set REG_COUNT=0
for %%F in (target\release\examples\*.exe) do (
    set "NAME=%%~nxF"
    set "PREFIX=!NAME:~0,3!"
    if /I not "!PREFIX!"=="mt_" (
        copy /Y "%%F" "examples\examples_win32\" >nul
        set /a REG_COUNT+=1
    )
)

REM Minitests: only files starting with "mt_".
set MT_COUNT=0
for %%F in (target\release\examples\mt_*.exe) do (
    copy /Y "%%F" "examples\minitest_win32\" >nul
    set /a MT_COUNT+=1
)

echo.
echo === Done ====================================================
echo  Regular examples copied : !REG_COUNT!  -^>  examples\examples_win32\
echo  Minitests copied        : !MT_COUNT!  -^>  examples\minitest_win32\
echo.

endlocal
exit /b 0
