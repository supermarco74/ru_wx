# message_box.rs

Standalone helper that wraps Win32 `MessageBoxW` to show a modal OK / Cancel / Yes / No prompt with an optional icon. The function-style API; no struct.

## Purpose
Quick, single-call "show a prompt" surface for the simplest cases. For a class-based, configurable equivalent see [`message_dialog.rs`](./message_dialog.md).

## Key Types
- `MessageBoxStyle` — `Ok`, `OkCancel`, `YesNo`, `YesNoCancel`.
- `MessageBoxIcon` — `Information`, `Warning`, `Error`, `Question`.
- `MessageBoxResult` — `Ok`, `Cancel`, `Yes`, `No`. Returned from `message_box`.

## Key Functions
- `message_box(parent: HWND, message: &str, title: &str, style: MessageBoxStyle, icon: MessageBoxIcon) -> MessageBoxResult` — Windows-only. Spawns a `MessageBoxW`. The `parent` is the owning `HWND`; pass `std::ptr::null_mut()` for an unowned (top-level) prompt.

## Win32 Notes
- Win32 `MessageBoxW` with style flags: `MB_OK = 0`, `MB_OKCANCEL = 1`, `MB_YESNOCANCEL = 3`, `MB_YESNO = 4`.
- Icon flags: `MB_ICONINFORMATION = 0x40`, `MB_ICONWARNING = 0x30`, `MB_ICONERROR = 0x10`, `MB_ICONQUESTION = 0x20`.
- Return-code → `MessageBoxResult` mapping: `IDOK = 1` → `Ok`; `IDCANCEL = 2` → `Cancel`; `IDYES = 6` → `Yes`; `IDNO = 7` → `No`.
- The function is synchronous: it does not return until the user dismisses the box.
- On non-Windows hosts the function is a stub that returns `MessageBoxResult::Ok` (no UI shown).

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning top-level window.
let r = message_box(
    frame.hwnd(),                       // pass null_mut() for an unowned (top-level) prompt
    "Save changes before closing?",
    "Confirm",
    MessageBoxStyle::YesNoCancel,
    MessageBoxIcon::Question,
);
match r {
    MessageBoxResult::Yes    => { /* save */ }
    MessageBoxResult::No     => { /* discard */ }
    MessageBoxResult::Cancel => { /* abort */ }
    _ => {}
}
```

The call is **synchronous**: it blocks until the user dismisses the box.

## See Also
- [`message_dialog.rs`](./message_dialog.md) — object-oriented wrapper with setters
- [`dialog.rs`](./dialog.rs / dialog.md) — general-purpose dialog for richer prompts
