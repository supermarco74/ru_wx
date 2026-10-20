# capture_all.ps1 - lancia ogni .exe di una cartella e ne cattura la finestra
# Uso: ./capture_all.ps1 -ExeDir <dir> -OutDir <dir> [-MaxSeconds 4] [-Skip 'pattern*']
#
# Riutilizza la stessa tecnica di `capture_example.ps1` (PrintWindow via
# GDI, che funziona anche senza schermo fisico sul runner) ma in batch,
# cosi' la CI puo' verificare in un colpo solo tutti gli esempi.
param(
    [Parameter(Mandatory = $true)][string]$ExeDir,
    [Parameter(Mandatory = $true)][string]$OutDir,
    [int]$MaxSeconds = 4,
    [string]$Skip = ""
)

$ErrorActionPreference = "Continue"

if (-not (Test-Path $ExeDir)) {
    Write-Error "ExeDir not found: $ExeDir"
    exit 1
}
if (-not (Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}

$src = @"
using System;
using System.Runtime.InteropServices;
public class W {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowDC(IntPtr h);
    [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr h);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleBitmap(IntPtr h, int w, int hh);
    [DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr d, IntPtr o);
    [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr o);
    [DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr d);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
Add-Type -TypeDefinition $src
Add-Type -AssemblyName System.Drawing

$exes = Get-ChildItem -Path $ExeDir -Filter "*.exe" | Sort-Object Name
if ($Skip -ne "") {
    $exes = $exes | Where-Object { $_.Name -notlike $Skip }
}
if ($exes.Count -eq 0) {
    Write-Error "No .exe to capture in $ExeDir (Skip='$Skip')."
    exit 1
}

$failed = @()

foreach ($f in $exes) {
    $name = $f.BaseName
    $png = Join-Path $OutDir "$name.png"
    Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

    $p = Start-Process -FilePath $f.FullName -PassThru
    # Gli esempi console non hanno una coda di messaggi: WaitForInputIdle
    # solleva, ma non e' un errore perche' il processo viene comunque
    # classificato dalla exit code qui sotto.
    try { $null = $p.WaitForInputIdle(5000) } catch { }

    # La finestra principale puo' comparire in ritardo.
    $h = [IntPtr]::Zero
    for ($i = 0; $i -lt $MaxSeconds; $i++) {
        if ($p.HasExited) { break }
        $p.Refresh()
        if ($p.MainWindowHandle -ne [IntPtr]::Zero -and $p.MainWindowTitle -ne "") {
            $h = $p.MainWindowHandle
            break
        }
        Start-Sleep -Milliseconds 500
    }

    if ($p.HasExited) {
        # Un esempio che termina da solo con codice 0 ha completato il
        # suo corso senza crash: i demo stub chiudono da soli dopo pochi
        # tick di idle. Solo un codice diverso da 0 e' un fallimento.
        if ($p.ExitCode -eq 0) {
            Write-Host "[OK]   $name self-closed cleanly (no capture)"
        } else {
            Write-Host "[FAIL] $name exited early (code $($p.ExitCode))"
            $failed += $name
        }
        continue
    }
    if ($h -eq [IntPtr]::Zero) {
        Write-Host "[FAIL] $name has no main window"
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        $failed += $name
        continue
    }

    [W+RECT]$r = New-Object W+RECT
    $null = [W]::GetWindowRect($h, [ref]$r)
    $w = $r.R - $r.L
    $hh = $r.B - $r.T
    if ($w -le 0 -or $hh -le 0) {
        Write-Host "[FAIL] $name window collapsed (${w}x${hh})"
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        $failed += $name
        continue
    }

    $sdc = [W]::GetWindowDC($h)
    $dc = [W]::CreateCompatibleDC($sdc)
    $bmp = [W]::CreateCompatibleBitmap($sdc, $w, $hh)
    $old = [W]::SelectObject($dc, $bmp)
    $null = [W]::PrintWindow($h, $dc, 2)
    $im = [System.Drawing.Image]::FromHbitmap($bmp)
    $im.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
    $im.Dispose()
    $null = [W]::SelectObject($dc, $old)
    $null = [W]::DeleteObject($bmp)
    $null = [W]::DeleteDC($dc)
    $null = [W]::ReleaseDC($h, $sdc)

    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Write-Host "[OK]   $name ${w}x${hh} -> $png"
}

Write-Host ""
Write-Host "PASS=$($exes.Count - $failed.Count)  FAIL=$($failed.Count)"
if ($failed.Count -gt 0) {
    Write-Host "Failed: $($failed -join ', ')"
    exit 2
}
exit 0