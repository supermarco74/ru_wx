param(
    [string]$OutFile = "f:\code\ru_wx\img\aui_dpi_v1.png",
    [string]$ProcName = "aui_manager_demo",
    [int]$W = 1628,
    [int]$H = 1024
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

$proc = Get-Process -Name $ProcName -ErrorAction SilentlyContinue
if ($proc) {
    $hwnd = $proc.MainWindowHandle
    Write-Host "Window handle: $hwnd"
    [WinCap]::ShowWindow($hwnd, 9) | Out-Null
    [WinCap]::MoveWindow($hwnd, 0, 0, $W, $H, $true) | Out-Null
    [WinCap]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Seconds 1
    $r = New-Object WinCap+RECT
    [WinCap]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    Write-Host "Window rect: Left=$($r.Left) Top=$($r.Top) Right=$($r.Right) Bottom=$($r.Bottom)"
    $w = 0; $h = 0
    $ok = [WinCap]::CaptureWindow($hwnd, $OutFile, [ref]$w, [ref]$h)
    Write-Host "Capture: ok=$ok size=${w}x${h} out=$OutFile"
} else {
    Write-Host "Process not found"
}
