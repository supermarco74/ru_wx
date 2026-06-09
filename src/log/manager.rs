//! Global log manager: holds the active target, the global
//! level threshold, and the per-component level overrides.
//!
//! The manager is process-wide: there is exactly one active
//! target, one global level, and one per-component-level map.
//! The target is reference-counted behind an [`Arc`] so the
//! manager can hand it to multiple loggers without lifetime
//! concerns; the level state is stored in atomics for lock-free
//! reads on the hot path. Per-component overrides are looked
//! up via a hierarchical slash-separated match (a rule on
//! `"ui"` applies to `"ui/dialog"`, `"ui/dialog/buttons"`,
//! etc.).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::levels::LogLevel;
use super::record::LogRecord;
use super::target::{LogTarget, StderrTarget};

/// Global log target
static GLOBAL_TARGET: OnceLock<Mutex<Arc<dyn LogTarget>>> = OnceLock::new();

/// Global log level filter
static GLOBAL_LEVEL: AtomicU32 = AtomicU32::new(4); // Message level by default

/// Component-level overrides
static COMPONENT_LEVELS: OnceLock<Mutex<HashMap<String, LogLevel>>> = OnceLock::new();

thread_local! {
    static THREAD_TARGET: RefCell<Option<Arc<dyn LogTarget>>> = RefCell::new(None);
    static THREAD_SUSPENDED: RefCell<bool> = const { RefCell::new(false) };
}

fn global_target() -> &'static Mutex<Arc<dyn LogTarget>> {
    GLOBAL_TARGET.get_or_init(|| Mutex::new(Arc::new(StderrTarget::new())))
}

fn component_levels() -> &'static Mutex<HashMap<String, LogLevel>> {
    COMPONENT_LEVELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Install `target` as the process-wide active log target and return
/// the previously active target. The previous target is returned so
/// callers can restore it (for example from a [`LogNull`](super::guards::LogNull)
/// guard).
///
/// This function affects every thread that does not have its own
/// thread-local target override.
pub fn set_active_target(target: Arc<dyn LogTarget>) -> Arc<dyn LogTarget> {
    let mut guard = global_target().lock().unwrap();
    let old = guard.clone();
    *guard = target;
    old
}

/// Get the currently active log target. A thread-local override
/// (configured by callers on Windows, or by constructing an RAII
/// guard) wins over the process-wide target.
pub fn get_active_target() -> Arc<dyn LogTarget> {
    THREAD_TARGET.with(|tl| {
        if let Some(ref target) = *tl.borrow() {
            return target.clone();
        }
        global_target().lock().unwrap().clone()
    })
}

/// Set the global minimum log level. Records with a level numerically
/// greater than `level` are dropped by [`log_message`] before reaching
/// the active target.
///
/// Component-level rules (set via [`set_component_level`]) are still
/// applied after the global filter.
pub fn set_log_level(level: LogLevel) {
    GLOBAL_LEVEL.store(level as u32, Ordering::Relaxed);
}

/// Read the current global log level as a [`LogLevel`] value.
pub fn get_log_level() -> LogLevel {
    let val = GLOBAL_LEVEL.load(Ordering::Relaxed);
    match val {
        1 => LogLevel::FatalError,
        2 => LogLevel::Error,
        3 => LogLevel::Warning,
        4 => LogLevel::Message,
        5 => LogLevel::Info,
        6 => LogLevel::Verbose,
        7 => LogLevel::Debug,
        _ => LogLevel::Trace,
    }
}

/// Return `true` when `level` would pass the global filter and the
/// current thread is not suspended. Note that this check ignores
/// per-component rules; use [`log_message`] to apply the full filter
/// chain in one call.
pub fn is_level_enabled(level: LogLevel) -> bool {
    // Check thread suspension
    let suspended = THREAD_SUSPENDED.with(|s| *s.borrow());
    if suspended {
        return false;
    }
    level <= get_log_level()
}

/// Set the minimum log level for a specific component. The component
/// name is hierarchical and `/`-separated, so a rule on `"ui"` also
/// applies to `"ui/button"` and `"ui/button/click"` unless a more
/// specific rule shadows it.
pub fn set_component_level(component: &str, level: LogLevel) {
    component_levels()
        .lock()
        .unwrap()
        .insert(component.to_string(), level);
}

/// Apply the full filter chain (global level, per-component rule,
/// thread suspension) and, if the record passes, forward it to the
/// active [`LogTarget`].
///
/// A `FatalError` record aborts the process after it has been logged.
pub fn log_message(level: LogLevel, component: &str, message: String) {
    if !is_level_enabled(level) {
        return;
    }

    // Check component-level filtering if component is specified
    if !component.is_empty() {
        let levels = component_levels().lock().unwrap();
        // Walk up the component hierarchy
        let parts: Vec<&str> = component.split('/').collect();
        let mut found_level: Option<LogLevel> = None;
        for i in (0..=parts.len()).rev() {
            let key = parts[..i].join("/");
            if let Some(&lvl) = levels.get(&key) {
                found_level = Some(lvl);
                break;
            }
        }
        if let Some(comp_level) = found_level {
            if level > comp_level {
                return;
            }
        }
    }

    let record = LogRecord::new(level, component, message);
    get_active_target().log_record(&record);

    // Fatal errors abort the program
    if level == LogLevel::FatalError {
        std::process::abort();
    }
}

/// Suppress logging on the current thread. Calls to [`is_level_enabled`]
/// and [`log_message`] from this thread will short-circuit until
/// [`resume`] is called.
#[allow(dead_code)]
pub fn suspend() {
    THREAD_SUSPENDED.with(|s| *s.borrow_mut() = true);
}

/// Re-enable logging on the current thread after a [`suspend`] call.
#[allow(dead_code)]
pub fn resume() {
    THREAD_SUSPENDED.with(|s| *s.borrow_mut() = false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::target::BufferTarget;

    /// Module-wide serialization lock. The log manager uses global
    /// state (target, level, component rules), so the tests in this
    /// module MUST run serially or they will stomp on each other.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Acquire the lock; poison-resilient.
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Save the current global target, run the test, then restore it.
    /// This keeps tests independent and avoids polluting the global
    /// state for any test that runs after.
    struct ScopedTarget(Arc<dyn LogTarget>);
    impl ScopedTarget {
        fn new(t: Arc<dyn LogTarget>) -> Self {
            let prev = set_active_target(t);
            ScopedTarget(prev)
        }
    }
    impl Drop for ScopedTarget {
        fn drop(&mut self) {
            let prev = std::mem::replace(&mut self.0, Arc::new(BufferTarget::new()));
            set_active_target(prev);
        }
    }

    /// Set the global level for the duration of the test.
    struct ScopedLevel(LogLevel);
    impl ScopedLevel {
        fn new(l: LogLevel) -> Self {
            let prev = get_log_level();
            set_log_level(l);
            ScopedLevel(prev)
        }
    }
    impl Drop for ScopedLevel {
        fn drop(&mut self) {
            set_log_level(self.0);
        }
    }

    #[test]
    fn log_message_writes_to_active_buffer_target() {
        let _lock = lock();
        let buf = Arc::new(BufferTarget::new());
        let _target = ScopedTarget::new(buf.clone());
        let _level = ScopedLevel::new(LogLevel::Message);

        log_message(LogLevel::Message, "ui", "hello".to_string());
        let msgs = buf.get_messages();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("hello"));
    }

    #[test]
    fn log_message_filters_by_global_level() {
        let _lock = lock();
        let buf = Arc::new(BufferTarget::new());
        let _target = ScopedTarget::new(buf.clone());
        let _level = ScopedLevel::new(LogLevel::Warning);

        // Info is below the threshold => must NOT be written.
        log_message(LogLevel::Info, "", "should-be-dropped".to_string());
        assert!(buf.get_messages().is_empty());

        // Warning IS at the threshold => must be written.
        log_message(LogLevel::Warning, "", "kept".to_string());
        assert_eq!(buf.get_messages().len(), 1);
        assert!(buf.get_messages()[0].contains("kept"));
    }

    #[test]
    fn component_level_overrides_global() {
        let _lock = lock();
        let buf = Arc::new(BufferTarget::new());
        let _target = ScopedTarget::new(buf.clone());
        // Global = Trace so the per-component filter has the final say
        // (otherwise Trace-level messages would be dropped by the
        // global filter before the component override is even checked).
        let _level = ScopedLevel::new(LogLevel::Trace);
        // The "verbose" component is bumped all the way down to Trace so
        // every level passes the per-component filter.
        set_component_level("verbose", LogLevel::Trace);
        // And the "quiet" component is bumped up to Error so everything
        // below Error is dropped.
        set_component_level("quiet", LogLevel::Error);

        // verbose/Trace passes per-component and per-global.
        log_message(LogLevel::Trace, "verbose", "v1".to_string());
        // quiet/Info is blocked by per-component (quiet=Error) even
        // though global=Trace would allow it.
        log_message(LogLevel::Info, "quiet", "q1".to_string());
        // quiet/Error passes per-component.
        log_message(LogLevel::Error, "quiet", "q2".to_string());
        // No component, level=Warning, global=Trace => passes.
        log_message(LogLevel::Warning, "", "g1".to_string());

        let msgs = buf.get_messages();
        assert_eq!(msgs.len(), 3, "got {:?}", msgs);
        assert!(msgs.iter().any(|m| m.contains("v1")));
        assert!(msgs.iter().any(|m| m.contains("q2")));
        assert!(msgs.iter().any(|m| m.contains("g1")));
        assert!(!msgs.iter().any(|m| m.contains("q1")));
    }

    #[test]
    fn component_level_hierarchy_walks_up_slash_separated_components() {
        let _lock = lock();
        // The /-separated component path means a rule on "ui" also
        // applies to "ui/button" if no more specific rule is set.
        let buf = Arc::new(BufferTarget::new());
        let _target = ScopedTarget::new(buf.clone());
        // Global = Trace so the per-component filter can do the
        // dropping without being pre-empted by the global filter.
        let _level = ScopedLevel::new(LogLevel::Trace);
        // "ui" is set to Trace — this is the only rule. It must apply
        // to both "ui" and "ui/button" because no more specific rule
        // exists for "ui/button".
        set_component_level("ui", LogLevel::Trace);
        // "net" is set to Error — it must drop anything below Error,
        // including Trace (which would otherwise pass globally).
        set_component_level("net", LogLevel::Error);

        log_message(LogLevel::Trace, "ui", "u1".to_string());
        log_message(LogLevel::Trace, "ui/button", "ub1".to_string());
        // "net" with Trace — blocked by per-component (net=Error).
        log_message(LogLevel::Trace, "net", "n1".to_string());

        let msgs = buf.get_messages();
        assert_eq!(msgs.len(), 2, "got {:?}", msgs);
        assert!(msgs.iter().any(|m| m.contains("u1")));
        assert!(msgs.iter().any(|m| m.contains("ub1")));
        assert!(!msgs.iter().any(|m| m.contains("n1")));
    }
}
