param(
    [string]$ProcName = "aui_manager_demo"
)

Add-Type -ReferencedAssemblies "System.Drawing" -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class EnumW {
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr hWnd, EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, System.Text.StringBuilder lpClassName, int nMaxCount);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    public static System.Collections.Generic.List<string> Enumerate(IntPtr hwnd) {
        var list = new System.Collections.Generic.List<string>();
        EnumChildWindows(hwnd, (h, l) => {
            var cls = new System.Text.StringBuilder(256);
            GetClassName(h, cls, 256);
            var txt = new System.Text.StringBuilder(256);
            GetWindowText(h, txt, 256);
            RECT r; GetWindowRect(h, out r);
            bool vis = IsWindowVisible(h);
            list.Add(string.Format("hwnd={0} class={1} text='{2}' screenRect=({3},{4})-({5},{6}) size={7}x{8} visible={9}",
                h, cls, txt, r.Left, r.Top, r.Right, r.Bottom, r.Right-r.Left, r.Bottom-r.Top, vis));
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
"@

$proc = Get-Process -Name $ProcName -ErrorAction SilentlyContinue
if ($proc) {
    $hwnd = $proc.MainWindowHandle
    Write-Host "Main window:"
    $r = New-Object EnumW+RECT
    [EnumW]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    Write-Host ("  rect: ({0},{1})-({2},{3}) size {4}x{5}" -f $r.Left, $r.Top, $r.Right, $r.Bottom, ($r.Right-$r.Left), ($r.Bottom-$r.Top))
    $c = New-Object EnumW+RECT
    [EnumW]::GetClientRect($hwnd, [ref]$c) | Out-Null
    Write-Host ("  client: ({0},{1})-({2},{3}) size {4}x{5}" -f $c.Left, $c.Top, $c.Right, $c.Bottom, ($c.Right-$c.Left), ($c.Bottom-$c.Top))
    Write-Host ""
    Write-Host "Child windows (screen coords):"
    [EnumW]::Enumerate($hwnd) | ForEach-Object { Write-Host "  $_" }
} else {
    Write-Host "Process not found"
}
