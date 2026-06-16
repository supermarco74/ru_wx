# smoke_all_demos.ps1 — launch every compiled demo/minitest, verify window + capture PNG.
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$ErrorActionPreference = 'Continue'

$src = @"
using System;
using System.Runtime.InteropServices;
public class SmokeWin {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder sb, int n);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetWindowTextLengthW(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
Add-Type -TypeDefinition $src -Language CSharpVersion3 -ReferencedAssemblies System.Drawing

$crateRoot  = Split-Path $PSScriptRoot -Parent
$root       = Split-Path $crateRoot -Parent
$demosDir   = Join-Path $crateRoot 'examples\examples_win32'
$mtDir      = Join-Path $crateRoot 'examples\minitest_win32'
$stamp      = Get-Date -Format 'yyyyMMdd_HHmmss'
$outDir     = Join-Path $root "img\smoke_$stamp"
$logDir     = Join-Path $root "logs\smoke_$stamp"
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
New-Item -ItemType Directory -Path $logDir -Force | Out-Null

function Test-BlankImage {
    param([string]$Path, [int]$SampleStep = 8)
    try {
        Add-Type -AssemblyName System.Drawing
        $bmp = [System.Drawing.Bitmap]::FromFile($Path)
        $w = $bmp.Width; $h = $bmp.Height
        if ($w -lt 4 -or $h -lt 4) { $bmp.Dispose(); return $true }
        $first = $bmp.GetPixel(1, 1)
        $diff = 0; $samples = 0
        for ($y = 0; $y -lt $h; $y += $SampleStep) {
            for ($x = 0; $x -lt $w; $x += $SampleStep) {
                $c = $bmp.GetPixel($x, $y)
                if ($c.R -ne $first.R -or $c.G -ne $first.G -or $c.B -ne $first.B) { $diff++ }
                $samples++
            }
        }
        $bmp.Dispose()
        return ($diff -lt ($samples * 0.02))
    } catch { return $true }
}

function Test-SingleExe {
    param(
        [string]$ExePath,
        [string]$Name,
        [string]$Category,
        [int]$WaitMs = 6000,
        [int]$MinW = 120,
        [int]$MinH = 80,
        [switch]$SkipCapture
    )

    $result = [ordered]@{
        Category = $Category
        Name     = $Name
        Status   = 'UNKNOWN'
        ExitCode = $null
        Size     = '-'
        Title    = '-'
        Visible  = $false
        Blank    = $null
        Notes    = ''
    }

    if (-not (Test-Path $ExePath)) {
        $result.Status = 'MISSING'
        return [pscustomobject]$result
    }

    $stdoutPath = Join-Path $logDir ($Name + '_stdout.log')
    $stderrPath = Join-Path $logDir ($Name + '_stderr.log')
    Remove-Item $stdoutPath, $stderrPath -ErrorAction SilentlyContinue

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $ExePath
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true
    $psi.CreateNoWindow = $true

    try { $p = [System.Diagnostics.Process]::Start($psi) }
    catch {
        $result.Status = 'START_FAIL'
        $result.Notes = $_.Exception.Message
        return [pscustomobject]$result
    }

    $loops = [math]::Max(1, [int]($WaitMs / 200))
    $hwnd = [IntPtr]::Zero
    for ($i = 0; $i -lt $loops; $i++) {
        Start-Sleep -Milliseconds 200
        if ($p.HasExited) { break }
        try {
            $p.Refresh()
            if ($p.MainWindowHandle -ne [IntPtr]::Zero) {
                $hwnd = $p.MainWindowHandle
                break
            }
        } catch {}
    }

    if ($p.HasExited) {
        $result.ExitCode = $p.ExitCode
        try { $p.StandardOutput.ReadToEnd() | Out-File $stdoutPath -Encoding UTF8 } catch {}
        try { $p.StandardError.ReadToEnd()  | Out-File $stderrPath -Encoding UTF8 } catch {}
        $hex = if ($null -ne $p.ExitCode) { '0x{0:X8}' -f [uint32]$p.ExitCode } else { '?' }
        $result.Status = "EXITED($hex)"
        if ($p.ExitCode -eq -1073741515 -or $p.ExitCode -eq 3221225781) {
            $result.Notes = 'STATUS_DLL_NOT_FOUND / missing manifest?'
        }
        return [pscustomobject]$result
    }

    if ($hwnd -eq [IntPtr]::Zero) {
        try { $p.Kill() } catch {}
        $result.Status = 'NO_WINDOW'
        return [pscustomobject]$result
    }

    Start-Sleep -Milliseconds 400
    $rawLen = [SmokeWin]::GetWindowTextLengthW($hwnd)
    $len = 1024
    if ($rawLen -gt 0 -and $rawLen -lt 1024) { $len = $rawLen + 1 }
    $sb = New-Object System.Text.StringBuilder $len
    [SmokeWin]::GetWindowTextW($hwnd, $sb, $len) | Out-Null
    $title = $sb.ToString()

    $r = New-Object SmokeWin+RECT
    [SmokeWin]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $ww = $r.R - $r.L
    $wh = $r.B - $r.T
    $vis = [SmokeWin]::IsWindowVisible($hwnd)
    $result.Size = "{0}x{1}" -f $ww, $wh
    $result.Title = $title
    $result.Visible = $vis

    if ($ww -lt $MinW -or $wh -lt $MinH) {
        $result.Status = 'TOO_SMALL'
        $result.Notes = "min ${MinW}x${MinH}"
    } elseif (-not $vis) {
        $result.Status = 'NOT_VISIBLE'
    } else {
        $result.Status = 'OK'
    }

    if (-not $SkipCapture) {
        $png = Join-Path $outDir ($Name + '.png')
        try {
            Add-Type -AssemblyName System.Drawing
            $bmp = New-Object System.Drawing.Bitmap($ww, $wh, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $hdc = $g.GetHdc()
            $pwOk = [SmokeWin]::PrintWindow($hwnd, $hdc, 0x2)
            $g.ReleaseHdc($hdc)
            $g.Dispose()
            $bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
            $bmp.Dispose()
            if (-not $pwOk) {
                if ($result.Status -eq 'OK') { $result.Status = 'CAPTURE_FAIL' }
                $result.Notes += ' PrintWindow=false'
            } else {
                $blank = Test-BlankImage -Path $png
                $result.Blank = $blank
                if ($blank -and $result.Status -eq 'OK') {
                    $result.Status = 'BLANK_UI'
                    $result.Notes = 'screenshot mostly uniform color'
                }
            }
        } catch {
            if ($result.Status -eq 'OK') { $result.Status = 'CAPTURE_ERR' }
            $result.Notes += " $($_.Exception.Message)"
        }
    }

    try { $p.Kill() } catch {}
    Start-Sleep -Milliseconds 200
    try {
        $p.StandardOutput.ReadToEnd() | Out-File $stdoutPath -Encoding UTF8
        $p.StandardError.ReadToEnd()  | Out-File $stderrPath -Encoding UTF8
    } catch {}

    return [pscustomobject]$result
}

# Kill stray processes from prior runs
Get-Process | Where-Object {
    $_.ProcessName -match '^(showcase_all|grid_demo|advanced_ui|cust_table|aui_toolbar|esempio2|music_player|input_controls|icon_tray|window_with|repro_|mt_)'
} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

$allResults = @()

Write-Host ''
Write-Host '=== MAIN DEMOS ==='
$demos = Get-ChildItem "$demosDir\*.exe" | Sort-Object Name
foreach ($f in $demos) {
    $name = $f.BaseName
    Write-Host "Testing $name ..."
    $extraWait = 0
    $minW = 120; $minH = 80
    if ($name -eq 'icon_tray_demo') { $extraWait = 2000 }
    if ($name -in @('stub_app_demo','cross_platform_stubs')) { $minW = 50; $minH = 50 }
    $r = Test-SingleExe -ExePath $f.FullName -Name $name -Category 'demo' -WaitMs (6000 + $extraWait) -MinW $minW -MinH $minH
    $allResults += $r
    Write-Host ("  -> {0}  {1}  {2}" -f $r.Status, $r.Size, $r.Title)
}

Write-Host ''
Write-Host '=== MINITESTS ==='
$mts = Get-ChildItem "$mtDir\*.exe" | Sort-Object Name
foreach ($f in $mts) {
    $name = $f.BaseName
    Write-Host "Testing $name ..."
    $r = Test-SingleExe -ExePath $f.FullName -Name $name -Category 'minitest' -WaitMs 4000 -MinW 100 -MinH 60
    $allResults += $r
    if ($r.Status -ne 'OK') {
        Write-Host ("  -> {0}  {1}" -f $r.Status, $r.Notes)
    }
}

$reportPath = Join-Path $logDir 'summary.csv'
$allResults | Export-Csv -Path $reportPath -NoTypeInformation

Write-Host ''
Write-Host '==== SUMMARY ===='
$ok = ($allResults | Where-Object { $_.Status -eq 'OK' }).Count
$fail = $allResults | Where-Object { $_.Status -ne 'OK' }
Write-Host ("Total: $($allResults.Count)  OK: $ok  FAIL: $($fail.Count)")
Write-Host ("Screenshots: $outDir")
Write-Host ("Logs/CSV:    $logDir")

if ($fail.Count -gt 0) {
    Write-Host ''
    Write-Host '---- FAILURES ----'
    $fail | Format-Table Category, Name, Status, Size, Title, Notes -AutoSize
    exit 2
}

Write-Host ''
Write-Host 'All demos passed smoke check.'
exit 0
