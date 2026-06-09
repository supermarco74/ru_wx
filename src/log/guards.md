# `log::guards` — `LogNull` RAII guard

Swaps in a `NullTarget` for the duration of a scope; restores the
previous target on drop. Mirrors wxWidgets' `wxLogNull`.

## `struct LogNull`

```rust
pub struct LogNull {
    previous_target: Arc<dyn LogTarget>,
}
```

## Constructor

- `impl LogNull { pub fn new() -> Self }` — calls
  `set_active_target(Arc::new(NullTarget))` and stores the previous
  target.
- `impl Default for LogNull` — same as `new()`.

## Drop behaviour

- `impl Drop for LogNull` — restores `previous_target` by calling
  `set_active_target(previous_target)`.

## Use cases

- Temporarily silence a third-party callback that logs noise.
- Wrap a noisy test fixture.
- Mute a known-spammy initialisation path.

## Example

```rust
use ru_wx::log::LogNull;

{
    let _silence = LogNull::new();
    // All log calls in this scope are dropped.
    wx_log_error!("this will NOT be emitted");
}
// Previous target restored here.
```

## Win32 note

- Internally uses `set_active_target` (process-wide). On threads that
  have their own thread-local target, the thread-local one wins
  (see [`manager.rs`](manager.md#thread_target)).

## Tests

No unit tests in this module (paired tests for the target-swapping
behaviour live in [`target.rs`](target.md) and [`manager.rs`](manager.md)).
