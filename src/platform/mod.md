# platform/mod.rs

Platform-specific backends for the UI toolkit. Active-backend re-exports live here.

## Purpose

- Acts as the dispatcher: only the submodule matching the current `cfg(target_os = "windows")` is compiled.
- Re-exports the active backend's items with `pub use win32::*;` so user code can `use crate::platform::*` without knowing which backend is active.

## What it does

- Declares `pub mod win32;` (gated on `cfg(target_os = "windows")`).
- Stubs out future `macos` and `linux` submodules in comments.
- `pub use win32::*;` (gated) re-exports `to_wide`, `next_control_id`, `next_menu_id`, `get_device_caps_dpi`.

## Conventions

- Functions that wrap a single Win32 FFI call live in the matching platform module and are re-exported here.
- Functions that return a handle are allowed to return a null pointer when the underlying call fails; callers are responsible for null-checking. The library also logs the failure through the [`crate::log`] system when appropriate.
- No function in this module panics. `try_into` / lock failures fall back to a sensible default (96 DPI, an empty string, etc.).

## See also

- [`win32.rs`](./win32.md) — the only active backend today.
- [`dpi.rs`](./dpi.md) — DPI awareness is platform-specific and lives one level up.
