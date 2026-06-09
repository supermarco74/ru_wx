# Smoke-check: start each minitest exe, verify it stays alive briefly,
# capture stderr if it dies, then kill it.
$names = @(
    "mt_button",
    "mt_checkbox_radio",
    "mt_choice_combo",
    "mt_bitmap_combo",
    "mt_context_menu",
    "mt_list_box",
    "mt_menu",
    "mt_slider_gauge",
    "mt_status_bar",
    "mt_status_bar_input",
    "mt_status_bar_minimal",
    "mt_tab",
    "mt_text_ctrl",
    "mt_tree_ctrl",
    "mt_static_line",
    "mt_static_box",
    "mt_static_bitmap",
    "mt_splitter",
    "mt_scrolled",
    "mt_scroll_bar",
    "mt_dc",
    "mt_listbook",
    "mt_choicebook",
    "mt_treebook",
    "mt_toolbook",
    "mt_mini_frame",
    "mt_splash_screen",
    "mt_tip_window",
    "mt_mdi",
    "mt_wizard",
    "mt_property_sheet_dialog",
    "mt_property_grid",
    "mt_window_corners",
    "mt_animation",
    "mt_gl_canvas",
    "mt_media_ctrl"
)

$root = "f:\code\ru_wx\ru_wx\target\debug\examples"
$logDir = "f:\code\ru_wx\ru_wx\target\smoke"
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

$results = @()
foreach ($n in $names) {
    $exe = Join-Path $root "$n.exe"
    if (-not (Test-Path $exe)) {
        $results += [pscustomobject]@{ Name = $n; Status = "MISSING_EXE"; ExitCode = -1; ErrTail = "" }
        continue
    }
    $errLog = Join-Path $logDir "$n.err"
    $outLog = Join-Path $logDir "$n.out"
    if (Test-Path $errLog) { Remove-Item $errLog -Force }
    if (Test-Path $outLog) { Remove-Item $outLog -Force }

    $p = Start-Process -FilePath $exe -PassThru -NoNewWindow -RedirectStandardOutput $outLog -RedirectStandardError $errLog
    Start-Sleep -Seconds 2
    if ($p.HasExited) {
        $results += [pscustomobject]@{ Name = $n; Status = "CRASHED_EARLY"; ExitCode = $p.ExitCode; ErrTail = (Get-Content $errLog -Raw -ErrorAction SilentlyContinue) }
    } else {
        try { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } catch {}
        $results += [pscustomobject]@{ Name = $n; Status = "OK_ALIVE_2s"; ExitCode = 0; ErrTail = "" }
    }
}

$ok = ($results | Where-Object { $_.Status -like "OK_*" }).Count
$bad = $results | Where-Object { $_.Status -notlike "OK_*" }
Write-Host ""
Write-Host "==== SMOKE CHECK SUMMARY ===="
foreach ($r in $results) {
    Write-Host "$($r.Name) : $($r.Status) (exit=$($r.ExitCode))"
}

Write-Host ""
Write-Host "PASS=$ok  FAIL=$($results.Count - $ok)"
if ($bad.Count -gt 0) {
    Write-Host ""
    Write-Host "---- FAILURES ----"
    foreach ($r in $bad) {
        Write-Host "## $($r.Name)  status=$($r.Status)  exit=$($r.ExitCode)"
        if ($r.ErrTail) { Write-Host $r.ErrTail }
    }
    exit 2
}
exit 0
