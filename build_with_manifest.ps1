# build_with_manifest.ps1
# ----------------------------------------------------------------------------
# ru_wx build wrapper.
#
# Why this script exists:
#   The library's build.rs uses `embed-resource` to compile `app.rc` into a
#   static .lib (app.lib) that contains the Common Controls v6 manifest.
#   However, cargo does NOT forward `cargo:rustc-link-lib` from a library
#   build script to downstream `[[example]]` targets, so the example .exes
#   are linked WITHOUT `app.lib` and therefore have no embedded manifest.
#   On Windows 11 this triggers 0xc0000142 (DLL initialization failure)
#   because Common Controls v6 is never activated.
#
# What this script does:
#   1. Forwards all arguments to `cargo build` (so `.\build_with_manifest.ps1
#      --release --example input_controls_demo` works).
#   2. Locates `mt.exe` (the Windows SDK Manifest Tool).
#   3. Walks the resulting target/<profile>/examples/*.exe files and runs
#      `mt.exe -manifest app.manifest -outputresource:<exe>;1` to embed
#      the manifest into each .exe.
#
# Usage:
#   .\build_with_manifest.ps1              # debug build of all examples
#   .\build_with_manifest.ps1 --release    # release build
#   .\build_with_manifest.ps1 -p ru_wx --example input_controls_demo
# ----------------------------------------------------------------------------
$ErrorActionPreference = "Stop"

# Resolve paths
$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path
$Manifest    = Join-Path $ScriptDir "app.manifest"
$TargetDir   = Join-Path $ScriptDir "target"

# Find mt.exe from the Windows 10/11 SDK. Search all known SDK versions.
$MtExe = $null
$SdkRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
if (Test-Path $SdkRoot) {
    $candidates = Get-ChildItem -Path $SdkRoot -Directory -ErrorAction SilentlyContinue |
                  Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
                  Sort-Object { [version]($_.Name -replace '_','.') } -Descending
    foreach ($sdk in $candidates) {
        $candidate = Join-Path $sdk.FullName "x64\mt.exe"
        if (Test-Path $candidate) { $MtExe = $candidate; break }
    }
}
if (-not $MtExe) {
    Write-Error "Could not find mt.exe under '$SdkRoot'. Install the Windows 10/11 SDK."
    exit 1
}
Write-Host "[build_with_manifest] using mt.exe: $MtExe"

# Step 1: run cargo build
Write-Host "[build_with_manifest] running: cargo build $args"
& cargo build @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Step 2: figure out the profile (debug vs release) and the artifact dir
$profile = "debug"
if ($args | Where-Object { $_ -eq "--release" -or $_ -eq "-r" }) { $profile = "release" }
$ExamplesDir = Join-Path $TargetDir "$profile\examples"
if (-not (Test-Path $ExamplesDir)) {
    Write-Host "[build_with_manifest] no examples directory at $ExamplesDir - nothing to embed."
    exit 0
}

# Step 3: embed the manifest into every example .exe
$exeFiles = Get-ChildItem -Path $ExamplesDir -Filter "*.exe" -File -ErrorAction SilentlyContinue
if (-not $exeFiles) {
    Write-Host "[build_with_manifest] no .exe files found in $ExamplesDir - nothing to embed."
    exit 0
}

$ok = 0
$fail = 0
foreach ($exe in $exeFiles) {
    Write-Host "[build_with_manifest] embedding manifest into $($exe.Name)"
    & $MtExe -manifest $Manifest -outputresource:"$($exe.FullName)";1
    if ($LASTEXITCODE -eq 0) {
        $ok++
    } else {
        Write-Warning "mt.exe failed for $($exe.Name) (exit=$LASTEXITCODE)"
        $fail++
    }
}

Write-Host ""
Write-Host "[build_with_manifest] done. embedded=$ok failed=$fail profile=$profile"
if ($fail -gt 0) { exit 1 }
exit 0
