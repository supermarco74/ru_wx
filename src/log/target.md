# `log::target` — pluggable log output destinations

`LogTarget` trait plus four built-in targets. All targets must be
`Send + Sync` so they can be installed once and shared.

## `trait LogTarget`

```rust
pub trait LogTarget: Send + Sync {
    fn log_record(&self, record: &LogRecord);
    fn flush(&self);
}
```

- `log_record` — emit a single record. Should not block; the manager
  serialises dispatch but does not bound the work.
- `flush` — push any internal buffers. Default behaviour: no-op.

## Built-in targets

### `StderrTarget` — default

```rust
pub struct StderrTarget { formatter: LogFormatter }
```

- Formats each record with a default `LogFormatter` and `eprintln!`s
  it.
- `flush()` calls `std::io::stderr().flush()`.
- Constructed by `set_active_target` on first use.

### `NullTarget` — silent

Unit struct. `log_record` and `flush` are both no-ops. Used by
[`LogNull`](guards.md).

### `BufferTarget` — in-memory

```rust
pub struct BufferTarget {
    messages: Mutex<Vec<String>>,
    formatter: LogFormatter,
}
```

- Accumulates formatted strings in a `Mutex<Vec<String>>`.
- `get_messages() -> Vec<String>` — snapshot (does not clear).
- `clear()` — empties the buffer.
- Useful for tests and "show last N log lines in About box" UIs.

### `ChainTarget` — fan-out

```rust
pub struct ChainTarget {
    primary: Arc<dyn LogTarget>,
    secondary: Arc<dyn LogTarget>,
}
```

- `log_record` forwards to `primary` then `secondary`.
- `flush` forwards to both.

## Tests

- `null_target_drops_messages` — calling on a `NullTarget` must not
  panic.
- `buffer_target_collects_and_returns_messages`
- `buffer_target_clear_empties_messages`
- `chain_target_sends_to_both`

## Example

```rust
use std::sync::Arc;
use ru_wx::log::{BufferTarget, ChainTarget, StderrTarget, set_active_target};

let stderr = Arc::new(StderrTarget::new());
let buf    = Arc::new(BufferTarget::new());
let chain  = Arc::new(ChainTarget::new(stderr, buf.clone()));

set_active_target(chain);
// ... logs go to BOTH stderr and the in-memory buffer ...
let recent = buf.get_messages();
```
