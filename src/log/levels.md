# `log::levels` — `LogLevel` enum

Eight-variant severity level for log records. Explicit discriminants match
the wxWidgets `wxLogLevel` C contract and are part of the public API.

## `enum LogLevel`

```rust
#[repr(u32)]
pub enum LogLevel {
    FatalError = 1,
    Error      = 2,
    Warning    = 3,
    Message    = 4,   // default global threshold
    Info       = 5,
    Verbose    = 6,
    Debug      = 7,
    Trace      = 8,
}
```

The numeric values are part of the contract with wxWidgets — do NOT
reorder or renumber.

## Methods

- `pub fn as_str(&self) -> &'static str` — returns `"FATAL"`, `"ERROR"`,
  `"WARNING"`, `"MESSAGE"`, `"INFO"`, `"VERBOSE"`, `"DEBUG"`, or
  `"TRACE"` (all uppercase, 5-7 chars).
- `impl Display` — delegates to `as_str()`.

## Ordering

Implements `PartialOrd`/`Ord` by discriminant, so `Error < Warning <
Message < Info < ...`. Filtering is done with `level <= threshold` (lower
severity numbers = more important).

## Tests

- `level_discriminants_match_wxwidgets` — pins numeric values.
- `level_as_str_*` (3 tests) — pins tag names.

## Example

```rust
use ru_wx::log::LogLevel;

assert_eq!(LogLevel::Error as u32, 2);
assert_eq!(LogLevel::Error.as_str(), "ERROR");
assert!(LogLevel::Error < LogLevel::Warning);
```
