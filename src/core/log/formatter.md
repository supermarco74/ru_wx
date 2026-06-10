# `log::formatter` — `LogFormatter`

Converts a [`LogRecord`](record.md) into a single-line, human-readable
`String`. Builder pattern toggles each field independently.

## `struct LogFormatter`

```rust
pub struct LogFormatter {
    show_timestamp: bool,   // default: true
    show_level: bool,       // default: true
    show_component: bool,   // default: true
    show_thread: bool,      // default: false
}
```

## Methods

- `pub fn new() -> Self` — defaults: timestamp ON, level ON, component
  ON, thread OFF.
- `pub fn with_timestamp(mut self, show: bool) -> Self` — toggle the
  `HH:MM:SS.mmm` prefix.
- `pub fn with_thread(mut self, show: bool) -> Self` — toggle the
  `[thread-name]` block. The block is emitted only when
  `record.thread_name` is `Some(_)`.
- `pub fn format(&self, record: &LogRecord) -> String` — builds a
  `Vec<String>` of enabled fields, then `join(" ")`.
- `impl Default for LogFormatter` — same as `new()`.

## Output format

Default: `HH:MM:SS.mmm [LEVEL] [component] message`

- Timestamp is derived from `record.timestamp.duration_since(UNIX_EPOCH)`
  using `(secs/3600)%24`, `(secs/60)%60`, `secs%60`, and
  `subsec_millis()`. UTC clock, no timezone suffix.
- Component is shown only if non-empty.
- Thread is shown only if both `show_thread == true` AND
  `record.thread_name.is_some()`.
- Message is always emitted.

## Tests

- `default_formatter_includes_timestamp_level_component_and_message`
- `without_timestamp_only_keeps_level_component_message`
- `empty_component_is_omitted`
- `with_thread_false_never_emits_thread_block`
- `with_thread_true_emits_thread_block_when_thread_has_a_name` — uses
  `std::thread::Builder::new().name(...)` to guarantee a named thread.

## Example

```rust
use ru_wx::log::{LogFormatter, LogRecord, LogLevel};

let f = LogFormatter::new().with_thread(true);
let s = f.format(&LogRecord::new(LogLevel::Warning, "ui", "click".into()));
assert!(s.contains("[WARN]"));
assert!(s.contains("[ui]"));
```
