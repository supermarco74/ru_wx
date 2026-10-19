param(
    [string]$ProcName = "aui_manager_demo"
)

Add-Type -ReferencedAssemblies "System.Drawing" -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class WinInfo {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern int GetDpiForWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetSystemDpiForProcess(IntPtr hProcess);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int w, int h, bool repaint);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$proc = Get-Process -Name $ProcName -ErrorAction SilentlyContinue
if ($proc) {
    $hwnd = $proc.MainWindowHandle
    Write-Host "Window handle: $hwnd"
    [WinInfo]::ShowWindow($hwnd, 9) | Out-Null
    [WinInfo]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Milliseconds 500

    $wr = New-Object WinInfo+RECT
    [WinInfo]::GetWindowRect($hwnd, [ref]$wr) | Out-Null
    Write-Host ("Window rect: Left={0} Top={1} Right={2} Bottom={3} (size {4}x{5})" -f $wr.Left, $wr.Top, $wr.Right, $wr.Bottom, ($wr.Right - $wr.Left), ($wr.Bottom - $wr.Top))

    $cr = New-Object WinInfo+RECT
    [WinInfo]::GetClientRect($hwnd, [ref]$cr) | Out-Null
    Write-Host ("Client rect: Left={0} Top={1} Right={2} Bottom={3} (size {4}x{5})" -f $cr.Left, $cr.Top, $cr.Right, $cr.Bottom, ($cr.Right - $cr.Left), ($cr.Bottom - $cr.Top))

    $dpi = [WinInfo]::GetDpiForWindow($hwnd)
    Write-Host "Window DPI: $dpi (scale factor $([math]::Round($dpi/96.0, 3)))"
} else {
    Write-Host "Process not found"
}
