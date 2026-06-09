# `log::win32_error` — Win32 error helpers (Windows only)

Thin wrappers around `GetLastError` and `FormatMessageW` to fetch,
format, and log Win32 errors. Ported from wxWidgets' `wxLogSysError`.

**Module gated:** `#[cfg(target_os = "windows")]` — file is empty on
non-Windows targets.

## Functions

### `get_last_win32_error() -> u32`

Reads the calling thread's last-error code via `GetLastError()` from
`windows-sys`.

### `format_win32_error(code: u32) -> String`

- Returns `"No error"` when `code == 0`.
- Otherwise allocates a 512-`u16` buffer and calls `FormatMessageW`
  with `FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS`.
- On `FormatMessageW` failure, returns `"Unknown error (code: {N})"`.
- Trims trailing CR/LF that `FormatMessageW` sometimes appends.

### `log_win32_error(context: &str)`

Convenience: fetches `GetLastError`, formats it, and emits
`log_message(LogLevel::Error, "win32", ...)` with the context and
formatted error.

No-op if `GetLastError` returned 0 (i.e. no error to report).

## Win32 notes

- `FORMAT_MESSAGE_FROM_SYSTEM` (0x00001000) loads the message from
  the system message table.
- `FORMAT_MESSAGE_IGNORE_INSERTS` (0x00000200) skips the `%1`, `%2`
  insertion strings — the raw text is usually good enough.
- Output buffer is 512 `u16` (~1 KB) — more than enough for any
  single system error.

## Example

```rust
use ru_wx::log::win32_error;

unsafe { CreateFileW(...); }
if let Some(err) = std::panic::catch_unwind(|| {
    ru_wx::log::log_win32_error("CreateFileW")
}).ok() {
    // err logged at Error level under component "win32"
}
```

## Cross-references

- [`api_guard.md`](api_guard.md) — automatic equivalent of
  `log_win32_error` for blocks.
- [`mod.md`](mod.md) — `wx_log_sys_error!` macro built on these helpers.
