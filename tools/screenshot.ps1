# screenshot.ps1
# Usage: powershell -ExecutionPolicy Bypass -File screenshot.ps1 -OutPath <path>
#
# Captures the primary screen and saves it as a PNG.
# Pure PowerShell + .NET, no Python, no third-party tools.

param(
    [Parameter(Mandatory = $true)]
    [string]$OutPath
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bitmap.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

Write-Host "saved: $OutPath  ($($bounds.Width)x$($bounds.Height))"
