# `log::api_guard` — `ApiGuard` (Windows only)

RAII guard that automatically logs a Win32 API error if a call between
construction and drop fails. Pattern is identical to wxWidgets'
`wxAPIBlock`.

**Module gated:** `#[cfg(target_os = "windows")]` — file is empty on
non-Windows targets.

## `struct ApiGuard`

```rust
#[cfg(target_os = "windows")]
pub struct ApiGuard {
    operation: String,
    initial_error: u32,
}
```

## Constructor

- `pub fn new(operation: impl Into<String>) -> Self`
  - `operation` is a human-readable description (e.g. `"CreateWindow"`).
  - Reads `GetLastError()` and stores it as `initial_error`.
  - Calls `SetLastError(0)` to start the block with a clean slate.

## Drop behaviour

On drop, `check()` is invoked:

1. Calls `GetLastError()`.
2. If the new value differs from `initial_error`, calls
   `log_message(LogLevel::Error, "win32/api", ...)` with the operation
   name and the formatted error.
3. If the new value equals `initial_error`, the block is silent (no
   log entry).

## Win32 notes

- Uses `windows-sys` `Win32::System::Diagnostics::Debug::{GetLastError, SetLastError}`.
- The reset to 0 is essential: it lets us detect *any* error that
  occurred during the block, even if the underlying call does not
  itself set a "success" code.

## Companion macro

Use [`wx_api_block!`](mod.md#logging-macros) to wrap a block
automatically:

```rust
wx_api_block!("CreateWindowExW", {
    unsafe { CreateWindowExW(...) }
});
```

## Tests

No unit tests in this module (Windows API error simulation is
non-portable; tests live in the integration suite instead).

## Example

```rust
use ru_wx::log::ApiGuard;

{
    let _guard = ApiGuard::new("ReadFile");
    // ... call ReadFile ...
    // if it set a Win32 error, the guard logs on scope exit
}
```
