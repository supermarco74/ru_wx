# Build All Examples Script
# Compila tutti gli esempi con entrambi i backend

$ErrorActionPreference = "Continue"

$examplesDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $examplesDir
$winui3Dir = Join-Path $examplesDir "examples_winui3"
$win32Dir = Join-Path $examplesDir "examples_win32"

# Output di Cargo fuori dal progetto (vedi .cargo\config.toml).
$targetRoot = Join-Path $env:USERPROFILE "target\ru_wx"

# Crea cartelle output
if (-not (Test-Path $winui3Dir)) { New-Item -ItemType Directory -Path $winui3Dir -Force | Out-Null }
if (-not (Test-Path $win32Dir)) { New-Item -ItemType Directory -Path $win32Dir -Force | Out-Null }

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Building ALL Examples" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

Set-Location $projectRoot

# ===== ESEMPI PRINCIPALI =====
$examples = @(
    "window_with_button",
    "input_controls_demo",
    "showcase_all",
    "mt_listbox_winui3"
)

# ===== MINITEST =====
$minitests = @(
    "mt_button", "mt_menu", "mt_context_menu", "mt_checkbox_radio",
    "mt_text_ctrl", "mt_choice_combo", "mt_list_box", "mt_slider_gauge",
    "mt_status_bar", "mt_static_text", "mt_static_bitmap", "mt_static_box",
    "mt_static_line", "mt_splitter", "mt_scrolled", "mt_scroll_bar",
    "mt_tab", "mt_tree_ctrl", "mt_list_ctrl"
)

# ==============================================
# WINUI3 (con --features winui3)
# ==============================================
Write-Host "`n========================================" -ForegroundColor Yellow
Write-Host "Building WINUI3 Examples" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Yellow

$winui3_ok = 0
$winui3_fail = 0

foreach ($ex in ($examples + $minitests)) {
    $result = cargo build --release --features winui3 --example $ex 2>&1
    if ($LASTEXITCODE -eq 0) {
        $src = Join-Path $targetRoot "release\examples\$ex.exe"
        if (Test-Path $src) {
            Copy-Item $src -Destination $winui3Dir -Force
            Write-Host "  [OK] $ex.exe" -ForegroundColor Green
            $winui3_ok++
        }
    } else {
        Write-Host "  [SKIP] $ex" -ForegroundColor Gray
        $winui3_fail++
    }
}

# ==============================================
# WIN32 (default, senza feature)
# ==============================================
Write-Host "`n========================================" -ForegroundColor Yellow
Write-Host "Building WIN32 Examples" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Yellow

$win32_ok = 0
$win32_fail = 0

foreach ($ex in ($examples + $minitests)) {
    $result = cargo build --release --example $ex 2>&1
    if ($LASTEXITCODE -eq 0) {
        $src = Join-Path $targetRoot "release\examples\$ex.exe"
        if (Test-Path $src) {
            Copy-Item $src -Destination $win32Dir -Force
            Write-Host "  [OK] $ex.exe" -ForegroundColor Green
            $win32_ok++
        }
    } else {
        Write-Host "  [SKIP] $ex" -ForegroundColor Gray
        $win32_fail++
    }
}

# ==============================================
# RISULTATO
# ==============================================
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Build Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

Write-Host "`nWINUI3:" -ForegroundColor Yellow
Write-Host "  Built: $winui3_ok" -ForegroundColor Green
Write-Host "  Skipped: $winui3_fail" -ForegroundColor Gray

Write-Host "`nWIN32:" -ForegroundColor Yellow
Write-Host "  Built: $win32_ok" -ForegroundColor Green
Write-Host "  Skipped: $win32_fail" -ForegroundColor Gray

Write-Host "`nExecutables:" -ForegroundColor Cyan
Write-Host "  WinUI3: $winui3Dir" -ForegroundColor Gray
Write-Host "  Win32: $win32Dir" -ForegroundColor Gray
