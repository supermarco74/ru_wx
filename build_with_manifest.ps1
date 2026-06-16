# build_with_manifest.ps1 — compatibility wrapper
#
# The Win11 manifest (Common Controls v6 + PerMonitorV2) is embedded
# automatically by `ru_wx/build.rs` via `embed-resource`. Every binary
# that links this crate — examples, tests, downstream apps — picks up
# `app.lib` without an extra `mt.exe` post-processing step.
#
# This script now forwards directly to `cargo build`.
#
# Usage:
#   .\build_with_manifest.ps1 --release --examples
#   .\build_with_manifest.ps1 -p ru_wx --example input_controls_demo
$ErrorActionPreference = "Stop"

Write-Host "[build_with_manifest] manifest is linked from ru_wx/build.rs (no mt.exe step)"
Write-Host "[build_with_manifest] running: cargo build $args"
& cargo build @args
exit $LASTEXITCODE
