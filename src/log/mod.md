# `log` — module root

Process-wide logging system ported from wxWidgets' `wxLog`. Pluggable targets,
hierarchical level filtering, RAII guards, and Win32 API error integration.

## Submodules

| File | Purpose |
|------|---------|
| [`levels.rs`](levels.md) | `LogLevel` enum (8 variants) |
| [`record.rs`](record.md) | `LogRecord` value type |
| [`target.rs`](target.md) | `LogTarget` trait + 4 built-in targets |
| [`manager.rs`](manager.md) | Global target, level, component rules |
| [`formatter.rs`](formatter.md) | `LogFormatter` (timestamp/level/component/thread) |
| [`guards.rs`](guards.md) | `LogNull` RAII guard |
| [`api_guard.rs`](api_guard.md) | `ApiGuard` RAII guard (Windows) |
| [`win32_error.rs`](win32_error.md) | Win32 error formatting (Windows) |

## Logging macros

All macros take a `format!`-style argument list and forward through
[`log_message`](manager.md#log_message):

| Macro | Level | Notes |
|-------|-------|-------|
| `wx_log_error!(...)` | `Error` | always emitted (subject to filtering) |
| `wx_log_warning!(...)` | `Warning` | |
| `wx_log_message!(...)` | `Message` | default global threshold |
| `wx_log_debug!(...)` | `Debug` | compiled out in release |
| `wx_log_trace!(component, ...)` | `Trace` | requires explicit component name |
| `wx_log_sys_error!(...)` | `Error` | Windows only; uses `GetLastError` |
| `wx_api_block!(name, { body })` | — | wraps block in `ApiGuard` |

The `wx_log_debug!` macro is gated on `debug_assertions` and compiles to
nothing in release builds.

## Re-exports

- Types: `ApiGuard`, `LogFormatter`, `LogNull`, `LogLevel`,
  `LogRecord`, `StderrTarget`, `BufferTarget`, `NullTarget`, `ChainTarget`
- Manager: `set_active_target`, `get_active_target`, `set_log_level`,
  `get_log_level`, `set_component_level`, `is_level_enabled`,
  `log_message`, `suspend`, `resume`
- Win32: `get_last_win32_error`, `format_win32_error`, `log_win32_error`
  (all `#[cfg(target_os = "windows")]`)

## Typical usage

```rust
use ru_wx::log::{wx_log_error, wx_log_message, set_log_level, LogLevel};

// Tune verbosity
set_log_level(LogLevel::Warning);

// Emit
wx_log_message!("user clicked button at ({},{})", x, y);
wx_log_error!("failed to open file: {}", err);
```

## Tests

Module-level tests live in each submodule. The manager tests use a
`TEST_LOCK` to serialise because the log state is process-wide global.
