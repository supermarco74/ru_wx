# Capture screenshot of the mt_status_bar window.
param(
    [string]$OutFile = "c:\Users\marco\Documents\code\test wxdragon\mt_status_bar_window.png"
)

Add-Type -ReferencedAssemblies "System.Drawing" -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Drawing;
using System.Drawing.Imaging;
public class WinCap {
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, int nFlags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int W, int H, bool bRepaint);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public static bool CaptureWindow(IntPtr hWnd, string outPath, out int width, out int height) {
        RECT r; GetWindowRect(hWnd, out r);
        int w = r.Right - r.Left;
        int h = r.Bottom - r.Top;
        width = w; height = h;
        if (w <= 0 || h <= 0) { return false; }
        using (var bmp = new Bitmap(w, h, PixelFormat.Format32bppArgb))
        using (var g = Graphics.FromImage(bmp)) {
            IntPtr hdc = g.GetHdc();
            bool ok = PrintWindow(hWnd, hdc, 0x2);
            g.ReleaseHdc(hdc);
            bmp.Save(outPath, ImageFormat.Png);
            return ok;
        }
    }
}
"@

$proc = Get-Process -Name "mt_status_bar" -ErrorAction SilentlyContinue
if (-not $proc) { Write-Host "mt_status_bar not running"; exit 1 }
$hwnd = $proc.MainWindowHandle
Write-Host "Window handle: $hwnd"
[WinCap]::ShowWindow($hwnd, 9) | Out-Null
[WinCap]::MoveWindow($hwnd, 50, 30, 600, 400, $true) | Out-Null
[WinCap]::SetForegroundWindow($hwnd) | Out-Null
$w = 0; $h = 0
$ok = [WinCap]::CaptureWindow($hwnd, $OutFile, [ref]$w, [ref]$h)
Write-Host "Capture: ok=$ok size=${w}x${h} out=$OutFile"
