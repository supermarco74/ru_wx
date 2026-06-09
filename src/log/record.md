# `log::record` — `LogRecord` value type

Immutable value representing a single log entry. Owns its own data; safe
to pass by reference or to ship across threads.

## `struct LogRecord`

```rust
pub struct LogRecord {
    pub level: LogLevel,
    pub component: String,     // hierarchical, "/" separated
    pub message: String,
    pub timestamp: SystemTime,
    pub thread_name: Option<String>,

    #[cfg(debug_assertions)]
    pub file: Option<&'static str>,
    #[cfg(debug_assertions)]
    pub line: u32,
}
```

## Methods

- `pub fn new(level: LogLevel, component: &str, message: String) -> Self`
  — copies `component` and `message` into owned `String`s, captures
  `SystemTime::now()` and the current thread name (via
  `std::thread::current().name().map(...).to_owned()`).

## Design notes

- All fields are `pub` — treat the record as immutable data.
- Strings are owned, so dropping the original source is safe.
- File/line are `#[cfg(debug_assertions)]` only — release builds don't
  carry source-location info (smaller records, faster formatting).
- `thread_name` is `None` when the thread is unnamed.

## Tests

- `record_new_copies_component_and_message` — verifies copies, not
  references.
- `record_new_owns_strings_independently` — dropping the source
  arguments must not affect the record.

## Example

```rust
use ru_wx::log::{LogRecord, LogLevel};
use std::time::SystemTime;

let r = LogRecord::new(LogLevel::Warning, "ui/button", "clicked".into());
assert_eq!(r.level, LogLevel::Warning);
assert_eq!(r.component, "ui/button");
assert!(r.timestamp <= SystemTime::now());
```
