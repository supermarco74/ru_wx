# platform/win32.rs

Win32 platform utilities: UTF-16 conversion, HDC-DPI query, control/menu ID allocators.

## Purpose

The single source of truth for "Win32-adjacent" helpers that don't belong to a specific widget module. Anything in here is callable from the rest of the codebase without dragging a widget in.

## Public functions

- **`to_wide(s: &str) -> Vec<u16>`** — convert a Rust `&str` to a null-terminated wide string (UTF-16 LE) suitable for any Win32 API taking `LPCWSTR` / `LPCTSTR`. Internally `s.encode_utf16().chain(once(0)).collect()`.
- **`get_device_caps_dpi(hdc: HDC) -> u32`** (Windows-only) — wraps `GetDeviceCaps(..., LOGPIXELSX)`. Null HDC or call failure → 96 (the standard baseline). Used as a fallback for old Windows versions that don't have `GetDpiForWindow`.
- **`next_control_id() -> u16`** — returns a fresh `u16` from a process-global `AtomicU16` counter starting at 100. Used as the child-window `HMENU` ID for every control the library creates.
- **`next_menu_id() -> u16`** — same idea, separate counter starting at 9000 to avoid collision with control IDs. Used as the `wParam` ID for every `WM_COMMAND` menu item.

## Win32 notes

- The two `AtomicU16` counters use `Ordering::Relaxed` because each ID only needs to be unique within the process; a strict ordering is not required.
- `get_device_caps_dpi` carries `#[allow(clippy::not_unsafe_ptr_arg_deref)]` because the FFI `HDC` raw pointer is the only argument and the lint doesn't see the `unsafe { GetDeviceCaps(...) }` block.
- `LOGPIXELSX.try_into().unwrap_or(0)` widens the `c_int` constant to whatever `GetDeviceCaps` wants; on any widening failure it falls back to `0`, which `GetDeviceCaps` will reject and the function will then return 96.

## See also

- [`platform/mod.rs`](mod.md) — re-exports everything from here.
- [`dpi.rs`](../core/dpi.md) — uses the same `GetDpiFor*` / `GetDeviceCaps` family at a higher level.
