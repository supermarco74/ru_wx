param(
    [string]$OutFile = "f:\code\ru_wx\img\aui_dpi_v3.png",
    [string]$ProcName = "aui_manager_demo"
)

Add-Type -ReferencedAssemblies "System.Drawing" -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Drawing;
using System.Drawing.Imaging;
public class WinCap2 {
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, int nFlags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr hdc);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleBitmap(IntPtr hdc, int w, int h);
    [DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr hdc, IntPtr obj);
    [DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr hdc);
    [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr obj);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public static bool CaptureClientArea(IntPtr hWnd, string outPath) {
        RECT r; GetClientRect(hWnd, out r);
        int w = r.Right - r.Left;
        int h = r.Bottom - r.Top;
        if (w <= 0 || h <= 0) { return false; }
        using (var bmp = new Bitmap(w, h, PixelFormat.Format32bppArgb))
        using (var g = Graphics.FromImage(bmp)) {
            g.Clear(System.Drawing.Color.White);
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
    [WinCap2]::ShowWindow($hwnd, 9) | Out-Null
    [WinCap2]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Seconds 1
    $r = New-Object WinCap2+RECT
    [WinCap2]::GetClientRect($hwnd, [ref]$r) | Out-Null
    Write-Host "Client rect: Left=$($r.Left) Top=$($r.Top) Right=$($r.Right) Bottom=$($r.Bottom) (size $($r.Right - $r.Left)x$($r.Bottom - $r.Top))"
    $ok = [WinCap2]::CaptureClientArea($hwnd, $OutFile)
    Write-Host "Capture: ok=$ok out=$OutFile"
} else {
    Write-Host "Process not found"
}
