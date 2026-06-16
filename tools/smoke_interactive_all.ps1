# smoke_interactive_all.ps1 — launch every demo/minitest, interact, verify graphics.
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$ErrorActionPreference = 'Continue'

$typeName = 'RuWxInteract_' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$src = @"
using System;
using System.Runtime.InteropServices;
public class $typeName {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT wr);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT cr);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder sb, int n);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetWindowTextLengthW(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool f);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll", SetLastError=true)]
    public static extern uint SendInput(uint n, INPUT[] p, int cb);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT {
        public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT {
        public ushort wVk, wScan; public uint dwFlags, time; public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT {
        public uint type;
        public MOUSEINPUT mi;
        public KEYBDINPUT ki;
    }
    public const uint INPUT_MOUSE = 0;
    public const uint INPUT_KEYBOARD = 1;
    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;
    public const uint MOUSEEVENTF_RIGHTDOWN = 0x0008;
    public const uint MOUSEEVENTF_RIGHTUP = 0x0010;
    public const uint KEYEVENTF_KEYUP = 0x0002;
    public const int VK_TAB = 0x09;
    public const int VK_SPACE = 0x20;
    public const int VK_RETURN = 0x0D;
    public const int SW_RESTORE = 9;
}
"@
Add-Type -TypeDefinition $src -Language CSharpVersion3 -ReferencedAssemblies System.Drawing
$W = [type]$typeName

$crateRoot = Split-Path $PSScriptRoot -Parent
$root      = Split-Path $crateRoot -Parent
$demosDir  = Join-Path $crateRoot 'examples\examples_win32'
$mtDir     = Join-Path $crateRoot 'examples\minitest_win32'
$stamp     = Get-Date -Format 'yyyyMMdd_HHmmss'
$outDir    = Join-Path $root "img\interactive_$stamp"
$logDir    = Join-Path $root "logs\interactive_$stamp"
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
New-Item -ItemType Directory -Path $logDir -Force | Out-Null

# Per-demo click profiles (client-area fractions 0..1).
$profiles = @{
    'advanced_ui_demo'    = @(@{x=0.55;y=0.08;desc='tab_grid'},@{x=0.65;y=0.08;desc='tab_webview'},@{x=0.45;y=0.08;desc='tab_preview'},@{x=0.15;y=0.12;desc='ribbon_btn'},@{x=0.60;y=0.45;desc='content'})
    'aui_toolbar_demo'    = @(@{x=0.10;y=0.10;desc='tool1'},@{x=0.18;y=0.10;desc='tool2'},@{x=0.12;y=0.42;desc='dock_top'},@{x=0.28;y=0.42;desc='dock_bottom'})
    'cust_table_grid'     = @(@{x=0.50;y=0.35;desc='grid_cell'},@{x=0.08;y=0.28;desc='check_all'},@{x=0.35;y=0.28;desc='price_sort'},@{x=0.55;y=0.18;desc='tab_string'})
    'esempio2'            = @(@{x=0.12;y=0.10;desc='toolbar_new'},@{x=0.20;y=0.10;desc='toolbar_open'},@{x=0.35;y=0.55;desc='editor'},@{x=0.20;y=0.78;desc='stacca_btn'})
    'grid_demo'           = @(@{x=0.35;y=0.28;desc='grid_row'},@{x=0.55;y=0.22;desc='grid_header'},@{x=0.70;y=0.40;desc='grid_cell2'})
    'input_controls_demo' = @(@{x=0.35;y=0.08;desc='tab_choices'},@{x=0.55;y=0.08;desc='tab_lists'},@{x=0.75;y=0.08;desc='tab_actions'},@{x=0.50;y=0.35;desc='control'})
    'music_player'        = @(@{x=0.10;y=0.10;desc='tool_open'},@{x=0.18;y=0.10;desc='tool_playlist'},@{x=0.50;y=0.35;desc='playlist_area'})
    'showcase_all'        = @(@{x=0.22;y=0.14;desc='tab_numeric'},@{x=0.35;y=0.14;desc='tab_pickers'},@{x=0.48;y=0.14;desc='tab_data'},@{x=0.12;y=0.10;desc='toolbar_icon'})
    'window_with_button'  = @(@{x=0.50;y=0.38;desc='click_me'},@{x=0.50;y=0.52;desc='svg_btn'})
    'icon_tray_demo'      = @(@{x=0.50;y=0.40;desc='center'},@{x=0.30;y=0.25;desc='btn_area'})
    'repro_2btn_textctrl' = @(@{x=0.15;y=0.20;desc='btn1'},@{x=0.30;y=0.20;desc='btn2'},@{x=0.50;y=0.40;desc='textctrl'})
    'repro_3buttons'      = @(@{x=0.15;y=0.25;desc='btn1'},@{x=0.35;y=0.25;desc='btn2'},@{x=0.55;y=0.25;desc='btn3'})
    'repro_4buttons'      = @(@{x=0.12;y=0.25;desc='btn1'},@{x=0.30;y=0.25;desc='btn2'},@{x=0.48;y=0.25;desc='btn3'},@{x=0.66;y=0.25;desc='btn4'})
    'repro_statictext'    = @(@{x=0.50;y=0.20;desc='label'},@{x=0.20;y=0.35;desc='btn1'})
    'repro_textctrl'      = @(@{x=0.50;y=0.35;desc='textctrl'},@{x=0.20;y=0.25;desc='btn1'})
    'mt_button'           = @(@{x=0.25;y=0.25;desc='plain_btn'},@{x=0.50;y=0.35;desc='color_btn'},@{x=0.70;y=0.30;desc='svg_btn'})
    'mt_tab'              = @(@{x=0.30;y=0.12;desc='tab2'},@{x=0.50;y=0.12;desc='tab3'},@{x=0.50;y=0.45;desc='content'})
    'mt_checkbox_radio'   = @(@{x=0.20;y=0.25;desc='checkbox'},@{x=0.20;y=0.40;desc='radio1'},@{x=0.20;y=0.50;desc='radio2'})
    'mt_list_ctrl'        = @(@{x=0.40;y=0.35;desc='list_row'},@{x=0.40;y=0.45;desc='list_row2'})
    'mt_tree_ctrl'        = @(@{x=0.25;y=0.30;desc='tree_node'},@{x=0.25;y=0.40;desc='tree_child'})
    'mt_modern_style'     = @(@{x=0.20;y=0.18;desc='dark_mode_cb'},@{x=0.20;y=0.35;desc='mica_radio'},@{x=0.50;y=0.85;desc='reapply_btn'})
    'mt_slider_gauge'     = @(@{x=0.50;y=0.35;desc='slider'},@{x=0.50;y=0.55;desc='gauge'})
    'mt_menu'             = @(@{x=0.08;y=0.03;desc='menu_file'},@{x=0.50;y=0.40;desc='content'})
    'mt_property_grid'    = @(@{x=0.35;y=0.30;desc='prop_row'},@{x=0.60;y=0.30;desc='prop_value'})
    'mt_status_bar'       = @(@{x=0.50;y=0.40;desc='content'},@{x=0.20;y=0.92;desc='status_field'})
}

$defaultClicks = @(
    @{x=0.50;y=0.50;desc='center'}
    @{x=0.25;y=0.15;desc='top_area'}
    @{x=0.75;y=0.40;desc='right_mid'}
    @{x=0.40;y=0.70;desc='bottom_area'}
)

$stubExes = @('stub_app_demo', 'cross_platform_stubs')

function Focus-Window {
    param([IntPtr]$Hwnd)
    [void][W]::ShowWindow($Hwnd, [W]::SW_RESTORE)
    [void][W]::BringWindowToTop($Hwnd)
    $nullPid = [uint32]0
    $dstTid = [W]::GetWindowThreadProcessId($Hwnd, [ref]$nullPid)
    $curTid = [W]::GetCurrentThreadId()
    [void][W]::AttachThreadInput($curTid, $dstTid, $true)
    [void][W]::SetForegroundWindow($Hwnd)
    Start-Sleep -Milliseconds 120
    [void][W]::AttachThreadInput($curTid, $dstTid, $false)
}

function Get-ClientScreenPoint {
    param([IntPtr]$Hwnd, [double]$Fx, [double]$Fy)
    $cr = New-Object ($typeName+'+RECT')
    [void][W]::GetClientRect($Hwnd, [ref]$cr)
    $cw = [math]::Max(1, $cr.R - $cr.L)
    $ch = [math]::Max(1, $cr.B - $cr.T)
    $pt = New-Object ($typeName+'+POINT')
    $pt.X = [int]($cw * $Fx)
    $pt.Y = [int]($ch * $Fy)
    [void][W]::ClientToScreen($Hwnd, [ref]$pt)
    return @{ X = $pt.X; Y = $pt.Y; CW = $cw; CH = $ch }
}

function Send-LeftClick {
    param([int]$X, [int]$Y)
    [void][W]::SetCursorPos($X, $Y)
    Start-Sleep -Milliseconds 60
    $sz = [System.Runtime.InteropServices.Marshal]::SizeOf([type]($typeName+'+INPUT'))
    $down = New-Object ($typeName+'+INPUT')
    $down.type = [W]::INPUT_MOUSE
    $down.mi.dwFlags = [W]::MOUSEEVENTF_LEFTDOWN
    $up = New-Object ($typeName+'+INPUT')
    $up.type = [W]::INPUT_MOUSE
    $up.mi.dwFlags = [W]::MOUSEEVENTF_LEFTUP
    [void][W]::SendInput(2, @($down, $up), $sz)
    Start-Sleep -Milliseconds 150
}

function Send-RightClick {
    param([int]$X, [int]$Y)
    [void][W]::SetCursorPos($X, $Y)
    Start-Sleep -Milliseconds 60
    $sz = [System.Runtime.InteropServices.Marshal]::SizeOf([type]($typeName+'+INPUT'))
    $down = New-Object ($typeName+'+INPUT')
    $down.type = [W]::INPUT_MOUSE
    $down.mi.dwFlags = [W]::MOUSEEVENTF_RIGHTDOWN
    $up = New-Object ($typeName+'+INPUT')
    $up.type = [W]::INPUT_MOUSE
    $up.mi.dwFlags = [W]::MOUSEEVENTF_RIGHTUP
    [void][W]::SendInput(2, @($down, $up), $sz)
    Start-Sleep -Milliseconds 150
}

function Send-Key {
    param([int]$Vk)
    $sz = [System.Runtime.InteropServices.Marshal]::SizeOf([type]($typeName+'+INPUT'))
    $down = New-Object ($typeName+'+INPUT')
    $down.type = [W]::INPUT_KEYBOARD
    $down.ki.wVk = [uint16]$Vk
    $up = New-Object ($typeName+'+INPUT')
    $up.type = [W]::INPUT_KEYBOARD
    $up.ki.wVk = [uint16]$Vk
    $up.ki.dwFlags = [W]::KEYEVENTF_KEYUP
    [void][W]::SendInput(2, @($down, $up), $sz)
    Start-Sleep -Milliseconds 80
}

function Save-Capture {
    param([IntPtr]$Hwnd, [string]$Path)
    $wr = New-Object ($typeName+'+RECT')
    [void][W]::GetWindowRect($Hwnd, [ref]$wr)
    $ww = $wr.R - $wr.L
    $wh = $wr.B - $wr.T
    if ($ww -lt 4 -or $wh -lt 4) { return $false }
    Add-Type -AssemblyName System.Drawing -ErrorAction SilentlyContinue
    $bmp = New-Object System.Drawing.Bitmap($ww, $wh, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    $ok = [W]::PrintWindow($Hwnd, $hdc, 0x2)
    $g.ReleaseHdc($hdc); $g.Dispose()
    if ($ok) { $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png) }
    $bmp.Dispose()
    return $ok
}

function Test-ImageMetrics {
    param([string]$Path, [int]$Step = 6)
    $m = @{ Blank = $true; Variance = 0; DiffPct = $null }
    if (-not (Test-Path $Path)) { return $m }
    try {
        Add-Type -AssemblyName System.Drawing -ErrorAction SilentlyContinue
        $bmp = [System.Drawing.Bitmap]::FromFile($Path)
        $w = $bmp.Width; $h = $bmp.Height
        if ($w -lt 4 -or $h -lt 4) { $bmp.Dispose(); return $m }
        $first = $bmp.GetPixel([math]::Min(2,$w-1), [math]::Min(2,$h-1))
        $diff = 0; $samples = 0; $sumR = 0L; $sumG = 0L; $sumB = 0L
        for ($y = 0; $y -lt $h; $y += $Step) {
            for ($x = 0; $x -lt $w; $x += $Step) {
                $c = $bmp.GetPixel($x, $y)
                if ($c.R -ne $first.R -or $c.G -ne $first.G -or $c.B -ne $first.B) { $diff++ }
                $sumR += $c.R; $sumG += $c.G; $sumB += $c.B
                $samples++
            }
        }
        $m.Blank = ($diff -lt ($samples * 0.02))
        $avgR = $sumR / $samples; $avgG = $sumG / $samples; $avgB = $sumB / $samples
        $var = 0.0
        for ($y = 0; $y -lt $h; $y += $Step) {
            for ($x = 0; $x -lt $w; $x += $Step) {
                $c = $bmp.GetPixel($x, $y)
                $dr = $c.R - $avgR; $dg = $c.G - $avgG; $db = $c.B - $avgB
                $var += ($dr*$dr + $dg*$dg + $db*$db)
            }
        }
        $m.Variance = [math]::Round($var / $samples, 1)
        $bmp.Dispose()
    } catch {}
    return $m
}

function Compare-Images {
    param([string]$A, [string]$B, [int]$Step = 8)
    if (-not ((Test-Path $A) -and (Test-Path $B))) { return $null }
    try {
        Add-Type -AssemblyName System.Drawing -ErrorAction SilentlyContinue
        $ba = [System.Drawing.Bitmap]::FromFile($A)
        $bb = [System.Drawing.Bitmap]::FromFile($B)
        $w = [math]::Min($ba.Width, $bb.Width)
        $h = [math]::Min($ba.Height, $bb.Height)
        $diff = 0; $samples = 0
        for ($y = 0; $y -lt $h; $y += $Step) {
            for ($x = 0; $x -lt $w; $x += $Step) {
                $ca = $ba.GetPixel($x, $y); $cb = $bb.GetPixel($x, $y)
                if ($ca.R -ne $cb.R -or $ca.G -ne $cb.G -or $ca.B -ne $cb.B) { $diff++ }
                $samples++
            }
        }
        $ba.Dispose(); $bb.Dispose()
        return [math]::Round(100.0 * $diff / $samples, 1)
    } catch { return $null }
}

function Test-StubExe {
    param([string]$ExePath, [string]$Name)
    $r = [ordered]@{ Category='demo'; Name=$Name; Status='UNKNOWN'; Graphics='-'; Interact='-'; DiffPct=$null; Notes='' }
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $ExePath
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.WaitForExit(8000)
    $stdout = $p.StandardOutput.ReadToEnd()
    $stderr = $p.StandardError.ReadToEnd()
    $stdout | Out-File (Join-Path $logDir ($Name + '_stdout.log')) -Encoding UTF8
    $stderr | Out-File (Join-Path $logDir ($Name + '_stderr.log')) -Encoding UTF8
    if ($Name -eq 'stub_app_demo') {
        if ($p.ExitCode -eq 0 -and $stderr -match 'stub backend') { $r.Status='STUB_OK'; $r.Interact='n/a'; $r.Graphics='n/a' }
        else { $r.Status='STUB_FAIL'; $r.Notes="exit=$($p.ExitCode)" }
        return [pscustomobject]$r
    }
    if ($stdout -match 'stub demo OK' -and $p.ExitCode -eq 0) {
        $r.Status='STUB_OK'; $r.Interact='console'; $r.Graphics='n/a'
    } else {
        $r.Status='STUB_FAIL'; $r.Notes="exit=$($p.ExitCode) stdout=$($stdout.Trim())"
    }
    return [pscustomobject]$r
}

function Test-InteractiveExe {
    param([string]$ExePath, [string]$Name, [string]$Category, [int]$WaitMs = 6000)

    $r = [ordered]@{
        Category = $Category; Name = $Name; Status = 'UNKNOWN'
        Graphics = '-'; Interact = '-'; DiffPct = $null; Notes = ''
    }

    if ($Name -in $stubExes) { return Test-StubExe -ExePath $ExePath -Name $Name }

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $ExePath
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    try { $p = [System.Diagnostics.Process]::Start($psi) }
    catch { $r.Status='START_FAIL'; $r.Notes=$_.Exception.Message; return [pscustomobject]$r }

    $hwnd = [IntPtr]::Zero
    $loops = [math]::Max(1, [int]($WaitMs / 200))
    for ($i = 0; $i -lt $loops; $i++) {
        Start-Sleep -Milliseconds 200
        if ($p.HasExited) { break }
        try { $p.Refresh(); if ($p.MainWindowHandle -ne [IntPtr]::Zero) { $hwnd = $p.MainWindowHandle; break } } catch {}
    }

    if ($p.HasExited) {
        $r.Status = 'CRASH_START'
        $r.Notes = "exit=$($p.ExitCode)"
        return [pscustomobject]$r
    }
    if ($hwnd -eq [IntPtr]::Zero) {
        try { $p.Kill() } catch {}
        $r.Status = 'NO_WINDOW'
        return [pscustomobject]$r
    }

    Focus-Window -Hwnd $hwnd
    Start-Sleep -Milliseconds 300

    $dir = Join-Path $outDir $Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    $before = Join-Path $dir 'before.png'
    $after  = Join-Path $dir 'after.png'

    if (-not (Save-Capture -Hwnd $hwnd -Path $before)) {
        try { $p.Kill() } catch {}
        $r.Status = 'CAPTURE_FAIL'
        return [pscustomobject]$r
    }

    $beforeM = Test-ImageMetrics -Path $before
    if ($beforeM.Blank) {
        try { $p.Kill() } catch {}
        $r.Status = 'BLANK_BEFORE'
        $r.Graphics = 'blank'
        return [pscustomobject]$r
    }
    $r.Graphics = "var=$($beforeM.Variance)"

    $clicks = if ($profiles.ContainsKey($Name)) { $profiles[$Name] } else { $defaultClicks }
    $clickCount = 0
    foreach ($c in $clicks) {
        if ($p.HasExited) { break }
        $pt = Get-ClientScreenPoint -Hwnd $hwnd -Fx $c.x -Fy $c.y
        Send-LeftClick -X $pt.X -Y $pt.Y
        $clickCount++
    }

    if (-not $p.HasExited) {
        $pt = Get-ClientScreenPoint -Hwnd $hwnd -Fx 0.60 -Fy 0.50
        Send-RightClick -X $pt.X -Y $pt.Y
        $clickCount++
        foreach ($vk in @([W]::VK_TAB, [W]::VK_TAB, [W]::VK_TAB, [W]::VK_SPACE)) {
            if ($p.HasExited) { break }
            Send-Key -Vk $vk
            $clickCount++
        }
    }

    Start-Sleep -Milliseconds 400
    if ($p.HasExited) {
        $r.Status = 'CRASH_INTERACT'
        $r.Interact = "$clickCount actions then exit=$($p.ExitCode)"
        return [pscustomobject]$r
    }

    Focus-Window -Hwnd $hwnd
    if (-not (Save-Capture -Hwnd $hwnd -Path $after)) {
        try { $p.Kill() } catch {}
        $r.Status = 'CAPTURE_AFTER_FAIL'
        $r.Interact = "$clickCount actions"
        return [pscustomobject]$r
    }

    $afterM = Test-ImageMetrics -Path $after
    if ($afterM.Blank) {
        try { $p.Kill() } catch {}
        $r.Status = 'BLANK_AFTER'
        $r.Interact = "$clickCount actions"
        $r.Graphics = 'blank_after'
        return [pscustomobject]$r
    }

    $diff = Compare-Images -A $before -B $after
    $r.DiffPct = $diff
    $r.Interact = "$clickCount actions"

    if ($afterM.Variance -lt 50 -and $beforeM.Variance -gt 200) {
        $r.Status = 'GRAPHICS_DEGRADED'
        $r.Notes = "var before=$($beforeM.Variance) after=$($afterM.Variance)"
    } elseif ($diff -ge 0.5) {
        $r.Status = 'OK'
        $r.Notes = 'visual_change'
    } else {
        $r.Status = 'OK_STATIC'
        $r.Notes = 'survived_no_visual_change'
    }

    try { $p.Kill() } catch {}
    Start-Sleep -Milliseconds 150
    return [pscustomobject]$r
}

Get-Process | Where-Object {
    $_.ProcessName -match '^(showcase_all|grid_demo|advanced_ui|cust_table|aui_toolbar|esempio2|music_player|input_controls|icon_tray|window_with|repro_|mt_|cross_platform|stub_app)'
} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400

$all = @()
Write-Host "`n=== MAIN DEMOS (interactive) ==="
foreach ($f in (Get-ChildItem "$demosDir\*.exe" | Sort-Object Name)) {
    Write-Host "  $($f.BaseName) ..."
    $wait = if ($f.BaseName -eq 'icon_tray_demo') { 8000 } else { 6000 }
    $all += Test-InteractiveExe -ExePath $f.FullName -Name $f.BaseName -Category 'demo' -WaitMs $wait
}

Write-Host "`n=== MINITESTS (interactive) ==="
foreach ($f in (Get-ChildItem "$mtDir\*.exe" | Sort-Object Name)) {
    Write-Host "  $($f.BaseName) ..."
    $all += Test-InteractiveExe -ExePath $f.FullName -Name $f.BaseName -Category 'minitest' -WaitMs 4500
}

$csv = Join-Path $logDir 'summary.csv'
$all | Export-Csv -Path $csv -NoTypeInformation

$ok = $all | Where-Object { $_.Status -in @('OK','OK_STATIC','STUB_OK') }
$fail = $all | Where-Object { $_.Status -notin @('OK','OK_STATIC','STUB_OK') }
$visual = ($all | Where-Object { $_.Status -eq 'OK' }).Count

Write-Host "`n==== INTERACTIVE SUMMARY ===="
Write-Host "Total: $($all.Count)  Pass: $($ok.Count)  Fail: $($fail.Count)  With visual change: $visual"
Write-Host "Screenshots: $outDir"
Write-Host "CSV:         $csv"

Write-Host "`n---- Results table ----"
$all | Sort-Object Category, Name | Format-Table Category, Name, Status, Interact, DiffPct, Graphics, Notes -AutoSize

if ($fail.Count -gt 0) {
    Write-Host "---- FAILURES ----"
    $fail | Format-Table Category, Name, Status, Notes -AutoSize
    exit 2
}
exit 0
