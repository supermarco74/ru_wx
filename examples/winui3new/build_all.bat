@echo off
REM ============================================================================
REM build_all.bat — Compila tutti i demo WinUI3/XAML e copia gli .exe qui
REM
REM Uso:
REM   build_all.bat          (debug build)
REM   build_all.bat release  (release build)
REM ============================================================================

setlocal EnableDelayedExpansion

REM Vai alla root del crate (due livelli sopra examples/winui3new)
cd /d "%~dp0..\.."

set PROFILE=debug
set CARGO_FLAGS=
if /i "%~1"=="release" (
    set PROFILE=release
    set CARGO_FLAGS=--release
)

echo ============================================
echo  Build tutti i demo WinUI3/XAML (%PROFILE%)
echo ============================================
echo.

REM Elenco di tutti i demo step
set DEMOS=wx_step1_window wx_step1_events ^
 wx_step2_button wx_step2_text wx_step2_input wx_step2_checks wx_step2_slider wx_step2_combo ^
 wx_step3_panel wx_step3_sizer wx_step3_tab wx_step3_splitter wx_step3_scroll ^
 wx_step4_menu wx_step4_toolbar wx_step4_status wx_step4_context ^
 wx_step5_msgbox ^
 wx_step6_icon ^
 wx_step7_tree wx_step7_listbox wx_step7_listctrl ^
 wx_step8_grid

REM Costruisci la riga di comando cargo con tutti gli --example
set EXAMPLES=
for %%d in (%DEMOS%) do (
    set EXAMPLES=!EXAMPLES! --example %%d
)

echo Compilazione in corso...
cargo build %CARGO_FLAGS% --features winui3_xaml %EXAMPLES%
if errorlevel 1 (
    echo.
    echo ERRORE: compilazione fallita.
    exit /b 1
)

echo.
echo Copia eseguibili in examples\winui3new\ ...
set COPIED=0
set FAILED=0
for %%d in (%DEMOS%) do (
    set SRC=target\%PROFILE%\examples\%%d.exe
    set DST=examples\winui3new\%%d.exe
    if exist "!SRC!" (
        copy /y "!SRC!" "!DST!" >nul
        set /a COPIED+=1
    ) else (
        echo   MANCANTE: !SRC!
        set /a FAILED+=1
    )
)

REM ── Bootstrap DLL ─────────────────────────────────────────────────────────
REM I demo richiedono Microsoft.WindowsAppRuntime.Bootstrap.dll accanto
REM all'exe. Il bootstrap del crate (src/platform/winui3_xaml/bootstrap.rs)
REM targetta il canale 2.0: con una DLL 1.x MddBootstrapInitialize2 fallisce
REM con 0x80670016 "Package dependency criteria could not be resolved".
REM Fonte primaria: DLL 2.0 da un'app WinUI3 installata sul sistema.
echo.
echo Copia Microsoft.WindowsAppRuntime.Bootstrap.dll ...
set BOOTSTRAP_SRC=
for /f "delims=" %%p in ('dir /b /s "C:\Program Files\WindowsApps\Microsoft.WindowsFeedbackHub_*\Microsoft.WindowsAppRuntime.Bootstrap.dll" 2^>nul') do (
    if not defined BOOTSTRAP_SRC set "BOOTSTRAP_SRC=%%p"
)
REM Fallback 1.x (canale errato: i demo falliranno con 0x80670016).
if not defined BOOTSTRAP_SRC if exist "C:\WindowsAppSDK\runtimes\win-x64\native\Microsoft.WindowsAppRuntime.Bootstrap.dll" (
    set "BOOTSTRAP_SRC=C:\WindowsAppSDK\runtimes\win-x64\native\Microsoft.WindowsAppRuntime.Bootstrap.dll"
)
if not defined BOOTSTRAP_SRC if exist "C:\Program Files\MuseHub\current\Microsoft.WindowsAppRuntime.Bootstrap.dll" (
    set "BOOTSTRAP_SRC=C:\Program Files\MuseHub\current\Microsoft.WindowsAppRuntime.Bootstrap.dll"
)

if defined BOOTSTRAP_SRC (
    copy /y "!BOOTSTRAP_SRC!" "target\%PROFILE%\examples\Microsoft.WindowsAppRuntime.Bootstrap.dll" >nul
    copy /y "!BOOTSTRAP_SRC!" "examples\winui3new\Microsoft.WindowsAppRuntime.Bootstrap.dll" >nul
    echo   OK: !BOOTSTRAP_SRC!
) else (
    echo   ATTENZIONE: bootstrap DLL non trovata. I demo non si avvieranno.
    echo   Copiare manualmente Microsoft.WindowsAppRuntime.Bootstrap.dll 2.x.
)

echo.
echo ============================================
echo  Completato: %COPIED% eseguibili copiati
if %FAILED% gtr 0 echo  Attenzione: %FAILED% eseguibili mancanti
echo ============================================

endlocal
