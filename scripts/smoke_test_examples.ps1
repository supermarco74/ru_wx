# Smoke-test: launch each example, verify it stays alive (window opened).
param(
    [int]$WaitMs = 1800,
    [string]$TargetDir = "f:\code\ru_wx\ru_wx\target\release\examples"
)

$ErrorActionPreference = "Continue"
$results = @()

Get-ChildItem -Path $TargetDir -Filter "*.exe" | Sort-Object Name | ForEach-Object {
    $name = $_.Name
    $path = $_.FullName
    $proc = $null
    try {
        $proc = Start-Process -FilePath $path -PassThru -WindowStyle Normal
        Start-Sleep -Milliseconds $WaitMs
        $proc.Refresh()
        if ($proc.HasExited) {
            $code = $proc.ExitCode
            $results += [PSCustomObject]@{ Name = $name; Status = "CRASHED"; ExitCode = $code }
        } else {
            $results += [PSCustomObject]@{ Name = $name; Status = "OK"; ExitCode = $null }
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    } catch {
        $results += [PSCustomObject]@{ Name = $name; Status = "ERROR"; ExitCode = $_.Exception.Message }
    } finally {
        if ($proc -and -not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }
}

$results | Format-Table -AutoSize
$failed = $results | Where-Object { $_.Status -ne "OK" }
Write-Host "`nTotal: $($results.Count)  OK: $(($results | Where-Object Status -eq 'OK').Count)  Failed: $($failed.Count)"
if ($failed.Count -gt 0) {
    Write-Host "FAILED:"
    $failed | ForEach-Object { Write-Host "  $($_.Name) -> $($_.Status) $($_.ExitCode)" }
    exit 1
}
exit 0
