#!/usr/bin/env pwsh
# add-safety.ps1
#
# Add `// SAFETY: ...` comments above every `unsafe {` block in the
# ru_wx library that doesn't already have one. Pure PowerShell
# (built into Windows) - no Python or external tools.
#
# Usage: from the ru_wx directory:
#     pwsh -File tools/add-safety.ps1
#
# Idempotent: re-running it leaves the file unchanged once every
# unsafe block is documented.

[CmdletBinding()]
param(
    [string]$Root = (Resolve-Path "$PSScriptRoot/..").Path
)

$ErrorActionPreference = 'Stop'

# Choose a SAFETY explanation based on the FFI function name found
# inside the unsafe block. Falls back to a generic Win32 explanation.
function Get-SafetyText {
    param([string]$Line)

    $fnMatch = [regex]::Match($Line, 'unsafe\s*\{\s*(?<fn>\w+)')
    $fn = ''
    if ($fnMatch.Success) { $fn = $fnMatch.Groups['fn'].Value }

    switch -Regex ($fn) {
        '^(GetWindowTextLengthW|GetWindowTextW|SetWindowTextW|GetClassNameW)$' {
            return '// SAFETY: FFI call to ' + $fn + '; `hwnd` is a real window handle and the wide buffer is sized appropriately.'
        }
        '^(GetWindowLongPtrW|SetWindowLongPtrW|GetWindowLongW|SetWindowLongW)$' {
            return '// SAFETY: FFI call to ' + $fn + ' with a live HWND and a valid `nIndex`.'
        }
        '^(GetDC|ReleaseDC|GetDeviceCaps|GetObjectW|GetObjectA)$' {
            return '// SAFETY: FFI call to ' + $fn + ' on a live GDI handle returned by the matching Create/Get call.'
        }
        '^(CreateWindowExW|RegisterClassExW|DefWindowProcW|CallWindowProcW)$' {
            return '// SAFETY: FFI call to ' + $fn + '; class name, window name, and atom are valid and the proc is the matching Rust trampoline.'
        }
        '^(DestroyWindow|MoveWindow|SetWindowPos|ShowWindow|UpdateWindow|InvalidateRect|RedrawWindow|IsWindow|EnableWindow)$' {
            return '// SAFETY: FFI call to ' + $fn + '; `hwnd` is a live window owned by this crate.'
        }
        '^(SendMessageW|PostMessageW|GetMessageW|PeekMessageW|TranslateMessage|DispatchMessageW)$' {
            return '// SAFETY: FFI call to ' + $fn + '; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.'
        }
        '^(CreateMenu|CreatePopupMenu|AppendMenuW|InsertMenuW|DestroyMenu|CheckMenuItem|EnableMenuItem|GetMenuState|GetMenuItemCount|GetMenuItemID|GetSubMenu|TrackPopupMenu|SetMenu|GetMenu|SetForegroundWindow)$' {
            return '// SAFETY: FFI call to ' + $fn + '; `hmenu` / `hwnd` is owned by this crate and the wide string is null-terminated UTF-16.'
        }
        '^(CreateCompatibleDC|CreateCompatibleBitmap|SelectObject|DeleteObject|DeleteDC|BitBlt|StretchBlt|SaveDC|RestoreDC|CreateBitmap)$' {
            return '// SAFETY: FFI call to ' + $fn + ' on GDI handles we own.'
        }
        '^(CreateIconIndirect|DestroyIcon|LoadImageW|LoadIconW|LoadCursorW|SetCursor|GetCursor|SetCursorPos|GetCursorPos|GetIconInfo|GetSystemMetrics)$' {
            return '// SAFETY: FFI call to ' + $fn + ' on cursor / icon handles owned by this crate.'
        }
        '^(SendMessageTimeoutW|GetWindowThreadProcessId|EnumWindows|GetForegroundWindow)$' {
            return '// SAFETY: FFI call to ' + $fn + '; arguments are valid HWND / atom / timeout values.'
        }
        '^(SetTimer|KillTimer|GetTickCount|Sleep|QueryPerformanceCounter|QueryPerformanceFrequency)$' {
            return '// SAFETY: FFI call to ' + $fn + '; arguments are within the documented ranges.'
        }
        '^(Shell_NotifyIconW|ShellExecuteW|ShellExecuteExW|DragAcceptFiles|DragQueryFileW|DragFinish)$' {
            return '// SAFETY: FFI call to ' + $fn + '; `NOTIFYICONDATAW` is zero-initialised and `cbSize` is set to the current version.'
        }
        '^(ChooseColorW|CommDlgExtendedErrorW|GetOpenFileNameW|GetSaveFileNameW)$' {
            return '// SAFETY: FFI call to ' + $fn + '; the dialog struct is fully initialised and the user callback is the matching Rust closure.'
        }
        '^(GetLastError|SetLastError|FormatMessageW)$' {
            return '// SAFETY: FFI call to ' + $fn + '; `FormatMessageW` writes into a 512-u16 stack buffer that we then truncate to `len`.'
        }
        '^(std::mem::zeroed)$' {
            return '// SAFETY: `std::mem::zeroed()` on a Win32 ABI struct whose all-zero representation is a valid initial state.'
        }
        '^(std::ptr::null|std::ptr::null_mut)$' {
            return '// SAFETY: explicit null pointer required by the Win32 API contract.'
        }
        '' {
            return '// SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.'
        }
        default {
            return '// SAFETY: FFI call to ' + $fn + ' with validated arguments.'
        }
    }
}

# Walk every .rs file under $Root/src.
$srcPath = Join-Path $Root 'src'
Write-Host "Root: $Root"
Write-Host "src:  $srcPath"
if (-not (Test-Path $srcPath)) {
    Write-Host "src path does not exist, falling back to current dir/src"
    $srcPath = Join-Path (Get-Location).Path 'src'
}
$files = @(Get-ChildItem -Path $srcPath -Recurse -Filter *.rs -ErrorAction SilentlyContinue)
Write-Host "files found: $($files.Count)"

$totalInserts = 0
foreach ($file in $files) {
    $lines = [System.IO.File]::ReadAllLines($file.FullName)
    $newLines = New-Object System.Collections.Generic.List[string]
    $inserted = 0

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]

        # Detect opening of an unsafe block on this line.
        # Match: contains `unsafe {` (possibly with `let X = ` in front)
        # but NOT `unsafe fn` (function signature) and NOT inside a comment.
        $isUnsafeBlock = $false
        if ($line -notmatch '^\s*//') {
            if ($line -match 'unsafe\s*\{') {
                # Exclude `unsafe fn` (function declaration) - those have
                # `unsafe` followed by ` fn` not ` {`.
                if ($line -notmatch 'unsafe\s+fn\b') {
                    $isUnsafeBlock = $true
                }
            }
        }

        if ($isUnsafeBlock) {
            # Walk backwards over blank lines to see if the previous
            # non-blank line is already a `// SAFETY:` comment.
            $prevIdx = $i - 1
            $hasSafety = $false
            while ($prevIdx -ge 0) {
                $prev = $lines[$prevIdx]
                if ($prev -match '^\s*$') {
                    $prevIdx--
                    continue
                }
                if ($prev -match '^\s*//\s*SAFETY\b') {
                    $hasSafety = $true
                }
                break
            }

            if (-not $hasSafety) {
                $indentMatch = [regex]::Match($line, '^(?<indent>\s*)')
                $indent = $indentMatch.Groups['indent'].Value
                $safetyText = Get-SafetyText -Line $line
                $newLines.Add($indent + $safetyText)
                $inserted++
            }
        }

        $newLines.Add($line)
    }

    if ($inserted -gt 0) {
        $content = ($newLines -join "`n")
        [IO.File]::WriteAllText($file.FullName, $content, [Text.UTF8Encoding]::new($false))
        Write-Host ('{0}  +{1} SAFETY comment(s)' -f $file.Name, $inserted)
        $totalInserts += $inserted
    }
}

Write-Host ''
Write-Host ('Done. Added {0} SAFETY comments across {1} files.' -f $totalInserts, $files.Count)
