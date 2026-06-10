#requires -Version 5.1
<#
.SYNOPSIS
    Fixes the over-prefixed `crate::dc::dc::xxx` paths.
#>

Set-Location 'F:\code\ru_wx\ru_wx'

$files = Get-ChildItem -Path 'src' -Recurse -Filter '*.rs' -File
$total = 0
$changed = 0

foreach ($file in $files) {
    $text = [System.IO.File]::ReadAllText($file.FullName)
    $newText = $text -replace 'crate::dc::dc::', 'crate::dc::'
    if ($newText -ne $text) {
        $diffs = ([regex]::Matches($text, 'crate::dc::dc::')).Count
        [System.IO.File]::WriteAllText($file.FullName, $newText, [System.Text.Encoding]::UTF8)
        $relPath = $file.FullName -replace '.*\\src\\', 'src\'
        Write-Host "  $relPath : $diffs"
        $total += $diffs
        $changed++
    }
}

Write-Host ""
Write-Host "=== Summary ==="
Write-Host "Files changed: $changed"
Write-Host "Total fixed: $total"
