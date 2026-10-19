# Build WinUI3 Examples Script
# Compila gli esempi con backend WinUI3 (feature winui3)

$ErrorActionPreference = "Continue"

$examplesDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $examplesDir
$outputDir = Join-Path $examplesDir "examples_winui3"

# Output di Cargo fuori dal progetto (vedi .cargo\config.toml).
$targetRoot = Join-Path $env:USERPROFILE "target\ru_wx"

# Crea cartella output se non esiste
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Building WinUI3 Examples" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Backend: WinUI3 (--features winui3)" -ForegroundColor Gray
Write-Host "Output: $outputDir" -ForegroundColor Gray
Write-Host ""

Set-Location $projectRoot

# ===== ESEMPI PRINCIPALI =====
Write-Host "`n=== ESEMPI ===" -ForegroundColor Yellow

$examples = @(
    "window_with_button",
    "input_controls_demo",
    "showcase_all"
)

$built = 0
$failed = 0

foreach ($example in $examples) {
    Write-Host "Building $example..." -ForegroundColor White
    
    # Compila con feature winui3
    $result = cargo build --release --features winui3 --example $example 2>&1
    $exitCode = $LASTEXITCODE
    
    if ($exitCode -eq 0) {
        $sourceExe = Join-Path $targetRoot "release\examples\$example.exe"
        $destExe = Join-Path $outputDir "$example.exe"
        if (Test-Path $sourceExe) {
            Copy-Item $sourceExe -Destination $destExe -Force
            Write-Host "  [OK] $example.exe" -ForegroundColor Green
            $built++
        }
    } else {
        Write-Host "  [FAIL] $example" -ForegroundColor Red
        $failed++
    }
}

# ===== MINITEST =====
Write-Host "`n=== MINITEST ===" -ForegroundColor Yellow

$minitests = @(
    "mt_button",
    "mt_menu",
    "mt_context_menu",
    "mt_checkbox_radio",
    "mt_text_ctrl",
    "mt_choice_combo",
    "mt_list_box",
    "mt_slider_gauge",
    "mt_status_bar",
    "mt_static_text",
    "mt_static_bitmap",
    "mt_static_box",
    "mt_static_line",
    "mt_splitter",
    "mt_scrolled",
    "mt_scroll_bar",
    "mt_tab",
    "mt_tree_ctrl",
    "mt_list_ctrl"
)

foreach ($mt in $minitests) {
    Write-Host "Building $mt..." -ForegroundColor White
    
    $result = cargo build --release --features winui3 --example $mt 2>&1
    $exitCode = $LASTEXITCODE
    
    if ($exitCode -eq 0) {
        $sourceExe = Join-Path $targetRoot "release\examples\$mt.exe"
        $destExe = Join-Path $outputDir "$mt.exe"
        if (Test-Path $sourceExe) {
            Copy-Item $sourceExe -Destination $destExe -Force
            Write-Host "  [OK] $mt.exe" -ForegroundColor Green
            $built++
        }
    } else {
        Write-Host "  [FAIL] $mt" -ForegroundColor Red
        $failed++
    }
}

# ===== RISULTATO =====
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Build Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Built: $built" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor $(if ($failed -gt 0) { "Red" } else { "Green" })
Write-Host ""

# Lista finale
Write-Host "Executables in examples_winui3:" -ForegroundColor Cyan
Get-ChildItem $outputDir -Filter "*.exe" | Sort-Object Name | ForEach-Object { 
    Write-Host "  $($_.Name)" -ForegroundColor White 
}
