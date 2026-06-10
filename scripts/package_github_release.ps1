# Packages release .exe artifacts for GitHub Releases.
#
# Expects a prior release build with embedded manifests, e.g.:
#   .\build_with_manifest.ps1 --release --examples
#
# Usage:
#   .\scripts\package_github_release.ps1 -Version 0.6.4
#   .\scripts\package_github_release.ps1 -Version 0.6.4 -OutputDir dist
param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Src = Join-Path $Root "target\release\examples"
if (-not (Test-Path $Src)) {
    Write-Error "Build output not found: $Src`nRun: .\build_with_manifest.ps1 --release --examples"
}

$DistRoot = Join-Path $Root $OutputDir
$Staging = Join-Path $DistRoot "staging"
$ExamplesDir = Join-Path $Staging "examples_win32"
$MinitestDir = Join-Path $Staging "minitest_win32"

Remove-Item -Recurse -Force $Staging -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $ExamplesDir, $MinitestDir | Out-Null

$regCount = 0
$mtCount = 0
Get-ChildItem -Path $Src -Filter "*.exe" -File | ForEach-Object {
    if ($_.Name.StartsWith("mt_")) {
        $destDir = $MinitestDir
        $mtCount++
    } else {
        $destDir = $ExamplesDir
        $regCount++
    }
    $dest = Join-Path $destDir $_.Name
    # Read/write bytes so packaging works even if a demo .exe is still running.
    $bytes = [System.IO.File]::ReadAllBytes($_.FullName)
    [System.IO.File]::WriteAllBytes($dest, $bytes)
}

if ($regCount -eq 0 -and $mtCount -eq 0) {
    Write-Error "No .exe files found under $Src"
}

$readme = @"
ru_wx $Version — Windows x64 binaries
=====================================

Requirements: Windows 10/11 x64 (MSVC build, Common Controls v6 manifest embedded).

Folders
-------
examples_win32\   Demo applications (showcase_all, music_player, grid_demo, …)
minitest_win32\   Focused per-component minitests (mt_*)

Run any .exe directly — no separate runtime install is required.

Source : https://github.com/supermarco74/ru_wx
License: GPL-3.0-or-later
"@

$readme | Out-File -FilePath (Join-Path $Staging "README.txt") -Encoding utf8

New-Item -ItemType Directory -Force -Path $DistRoot | Out-Null

$allZip = Join-Path $DistRoot "ru_wx-$Version-win64.zip"
$examplesZip = Join-Path $DistRoot "ru_wx-$Version-examples-win64.zip"
$minitestsZip = Join-Path $DistRoot "ru_wx-$Version-minitests-win64.zip"

if (Test-Path $allZip) { Remove-Item -Force $allZip }
if (Test-Path $examplesZip) { Remove-Item -Force $examplesZip }
if (Test-Path $minitestsZip) { Remove-Item -Force $minitestsZip }

Push-Location $Staging
try {
    Compress-Archive -Path "README.txt", "examples_win32", "minitest_win32" -DestinationPath $allZip -CompressionLevel Optimal
    Compress-Archive -Path "README.txt", "examples_win32" -DestinationPath $examplesZip -CompressionLevel Optimal
    Compress-Archive -Path "README.txt", "minitest_win32" -DestinationPath $minitestsZip -CompressionLevel Optimal
} finally {
    Pop-Location
}

Write-Host "Packaged ru_wx $Version"
Write-Host "  regular examples : $regCount"
Write-Host "  minitests        : $mtCount"
Write-Host "  $allZip"
Write-Host "  $examplesZip"
Write-Host "  $minitestsZip"
