# capture_example.ps1 - lancia un esempio ru_wx e cattura la finestra in PNG
# Uso: powershell -ExecutionPolicy Bypass -File capture_example.ps1 -ExePath <exe> -OutPath <png> [-ProcessName <nome>] [-KeepRunning]
param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][string]$OutPath,
    [string]$ProcessName,
    [switch]$KeepRunning
)
$ErrorActionPreference = "Continue"

if (-not $ProcessName) {
    $ProcessName = [System.IO.Path]::GetFileNameWithoutExtension($ExePath)
}

Get-Process $ProcessName -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 400

$p = Start-Process -FilePath $ExePath -PassThru
Write-Host "STEP1 launched pid=$($p.Id)"
$idle = $p.WaitForInputIdle(5000)
Write-Host "STEP2 WaitForInputIdle=$idle"
Start-Sleep -Seconds 2

# La main window può apparire in ritardo (app pesanti): riprova finché
# il titolo non è vuoto o scadono ~8 secondi.
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 8; $i++) {
    $p.Refresh()
    if ($p.MainWindowHandle -ne [IntPtr]::Zero -and $p.MainWindowTitle -ne "") {
        $h = $p.MainWindowHandle
        break
    }
    Start-Sleep -Seconds 1
}
$h = $p.MainWindowHandle
Write-Host "STEP3 hwnd=$h title='$($p.MainWindowTitle)'"

if ($h -eq [IntPtr]::Zero) {
    Write-Host "FATAL: no hwnd"
    if (-not $KeepRunning) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }
    exit 1
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

[W+RECT]$r = New-Object W+RECT
$ok = [W]::GetWindowRect($h, [ref]$r)
$w  = $r.R - $r.L
$hh = $r.B - $r.T
Write-Host "STEP4 GetWindowRect ok=$ok size=${w}x${hh}"

if ($w -le 0 -or $hh -le 0) {
    Write-Host "FATAL: window collapsed (size=${w}x${hh})"
    if (-not $KeepRunning) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }
    exit 2
}

Add-Type -AssemblyName System.Drawing
$sdc = [W]::GetWindowDC($h)
$dc  = [W]::CreateCompatibleDC($sdc)
$bmp = [W]::CreateCompatibleBitmap($sdc, $w, $hh)
$old = [W]::SelectObject($dc, $bmp)
$po  = [W]::PrintWindow($h, $dc, 2)
Write-Host "STEP5 PrintWindow=$po"

$im = [System.Drawing.Image]::FromHbitmap($bmp)
$im.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$im.Dispose()
[W]::SelectObject($dc, $old) | Out-Null
[W]::DeleteObject($bmp)       | Out-Null
[W]::DeleteDC($dc)            | Out-Null
[W]::ReleaseDC($h, $sdc)      | Out-Null

if (-not $KeepRunning) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }
Write-Host "STEP6 OK: $OutPath  size=${w}x${hh}"
