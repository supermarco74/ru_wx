# `log::manager` — global log state and filter chain

Holds the active [`LogTarget`](target.md), the global level threshold,
and the per-component level overrides. The manager is process-wide.

## Globals (private `static`)

| Static | Type | Initial |
|--------|------|---------|
| `GLOBAL_TARGET` | `OnceLock<Mutex<Arc<dyn LogTarget>>>` | `StderrTarget::new()` |
| `GLOBAL_LEVEL` | `AtomicU32` | `4` (`LogLevel::Message`) |
| `COMPONENT_LEVELS` | `OnceLock<Mutex<HashMap<String, LogLevel>>>` | empty |
| `THREAD_TARGET` | `thread_local! RefCell<Option<Arc<dyn LogTarget>>>` | `None` |
| `THREAD_SUSPENDED` | `thread_local! RefCell<bool>` | `false` |

## Public functions

### Target management

- `set_active_target(target: Arc<dyn LogTarget>) -> Arc<dyn LogTarget>`
  — installs `target`, returns the previously active one (handy for
  RAII restoration).
- `get_active_target() -> Arc<dyn LogTarget>` — thread-local override
  wins over the process-wide one.

### Global level

- `set_log_level(level: LogLevel)` — stores `level as u32` into
  `GLOBAL_LEVEL` with `Ordering::Relaxed`.
- `get_log_level() -> LogLevel` — translates the atomic back to the
  enum; defaults to `Trace` for unknown values.
- `is_level_enabled(level: LogLevel) -> bool` — true when level passes
  the global filter AND the current thread is not suspended. Ignores
  per-component rules.

### Component rules

- `set_component_level(component: &str, level: LogLevel)` — inserts
  into `COMPONENT_LEVELS`. The component name is hierarchical and
  `/`-separated; a rule on `"ui"` also applies to `"ui/dialog"`,
  `"ui/dialog/buttons"`, etc.

### `log_message`

```rust
pub fn log_message(level: LogLevel, component: &str, message: String)
```

1. If `!is_level_enabled(level)`, return.
2. If `component` is non-empty, walk the component hierarchy from most
   specific to least: `"ui/button/click"` → `"ui/button"` → `"ui"` →
   `""`. Use the first rule found. If a rule exists and `level >
   comp_level`, return.
3. Build a `LogRecord` and forward to the active target.
4. If `level == FatalError`, call `std::process::abort()`.

### Thread suspension

- `suspend()` — sets `THREAD_SUSPENDED` to `true` on the current
  thread. All log calls short-circuit.
- `resume()` — clears the flag.
- Both are `#[allow(dead_code)]` — currently used only by the
  test-suite plumbing.

## Tests

- `log_message_writes_to_active_buffer_target`
- `log_message_filters_by_global_level`
- `component_level_overrides_global`
- `component_level_hierarchy_walks_up_slash_separated_components`

All four use:

- `static TEST_LOCK: std::sync::Mutex<()>` — to serialise because the
  state is global.
- `ScopedTarget` — saves and restores the active target.
- `ScopedLevel` — saves and restores the global level.

## Example

```rust
use std::sync::Arc;
use ru_wx::log::{set_active_target, BufferTarget, LogLevel,
                 set_log_level, set_component_level, log_message};

let buf = Arc::new(BufferTarget::new());
set_active_target(buf.clone());
set_log_level(LogLevel::Message);
set_component_level("ui", LogLevel::Trace);

log_message(LogLevel::Trace, "ui/button", "click".into());
log_message(LogLevel::Info,  "ui/button", "hidden".into());
log_message(LogLevel::Error, "net",      "disconnect".into());

let msgs = buf.get_messages();
assert_eq!(msgs.len(), 2);
```
