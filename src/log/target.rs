//! Pluggable log output destinations.
//!
//! Every log destination implements the [`LogTarget`] trait:
//! it exposes a thread-safe `write` method that consumes a
//! [`LogRecord`](super::record::LogRecord) and a `flush`
//! method for buffered targets. Four concrete targets are
//! shipped with the crate:
//! - [`StderrTarget`] — write each record to `stderr`;
//! - [`BufferTarget`] — accumulate records in an in-memory
//!   `Vec` for later inspection or test assertion;
//! - [`NullTarget`] — discard every record (used by the
//!   [`LogNull`](super::LogNull) guard);
//! - [`ChainTarget`] — fan-out to a list of inner targets,
//!   useful for "log to stderr and also keep the last N
//!   records in a buffer for the About box".
//!
//! A target can be set as the process-wide active target with
//! [`set_active_target`](super::set_active_target); the manager
//! holds it behind an [`Arc`] so swapping targets is cheap.

use super::formatter::LogFormatter;
use super::record::LogRecord;
use std::sync::{Arc, Mutex};

/// Trait for log output targets. All targets must be thread-safe
/// (`Send + Sync`) so they can be installed as the active target
/// once and shared across the whole process.
pub trait LogTarget: Send + Sync {
    /// Emit a single formatted log record. Implementations should
    /// not block; the logging system serialises the dispatch but
    /// does not bound the time a target may spend on its work.
    fn log_record(&self, record: &LogRecord);
    /// Flush any internal buffers. The default in the built-in
    /// targets is a no-op; targets that buffer output (e.g. a
    /// file logger with a `BufWriter`) should override it.
    fn flush(&self);
}

/// Logs to standard error (the default target installed at
/// process start). Each record is formatted with a default
/// [`LogFormatter`] before being written.
pub struct StderrTarget {
    formatter: LogFormatter,
}

impl StderrTarget {
    /// Create a new stderr target with a default formatter.
    pub fn new() -> Self {
        Self {
            formatter: LogFormatter::new(),
        }
    }
}

impl Default for StderrTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl LogTarget for StderrTarget {
    fn log_record(&self, record: &LogRecord) {
        let formatted = self.formatter.format(record);
        eprintln!("{}", formatted);
    }
    fn flush(&self) {
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }
}

/// Suppresses all log output. Use the [`crate::log::LogNull`] guard
/// to swap the active target for a `NullTarget` for the duration
/// of a scope.
pub struct NullTarget;

impl LogTarget for NullTarget {
    fn log_record(&self, _record: &LogRecord) {}
    fn flush(&self) {}
}

/// Buffers all messages in memory so they can be retrieved later
/// via [`BufferTarget::get_messages`]. Useful for tests and for
/// collecting diagnostics in a UI dialog.
pub struct BufferTarget {
    messages: Mutex<Vec<String>>,
    formatter: LogFormatter,
}

impl BufferTarget {
    /// Create a new empty in-memory buffer.
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            formatter: LogFormatter::new(),
        }
    }

    /// Return a snapshot of every record that has been logged so
    /// far, in order. The internal buffer is **not** cleared.
    pub fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }

    /// Discard every buffered record. Subsequent calls to
    /// [`BufferTarget::get_messages`] will return an empty vector.
    pub fn clear(&self) {
        self.messages.lock().unwrap().clear();
    }
}

impl Default for BufferTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl LogTarget for BufferTarget {
    fn log_record(&self, record: &LogRecord) {
        let formatted = self.formatter.format(record);
        self.messages.lock().unwrap().push(formatted);
    }
    fn flush(&self) {}
}

/// Chains two targets together. Each record is forwarded to
/// **both** the `primary` and the `secondary` target. `flush()`
/// is also forwarded to both.
pub struct ChainTarget {
    primary: Arc<dyn LogTarget>,
    secondary: Arc<dyn LogTarget>,
}

impl ChainTarget {
    /// Build a new chain that forwards to `primary` first, then
    /// `secondary`.
    pub fn new(primary: Arc<dyn LogTarget>, secondary: Arc<dyn LogTarget>) -> Self {
        Self { primary, secondary }
    }
}

impl LogTarget for ChainTarget {
    fn log_record(&self, record: &LogRecord) {
        self.primary.log_record(record);
        self.secondary.log_record(record);
    }
    fn flush(&self) {
        self.primary.flush();
        self.secondary.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::levels::LogLevel;

    fn rec(component: &str, message: &str) -> LogRecord {
        LogRecord::new(LogLevel::Warning, component, message.to_string())
    }

    #[test]
    fn null_target_drops_messages() {
        let n = NullTarget;
        n.log_record(&rec("ui", "boom")); // must not panic
        n.flush();
    }

    #[test]
    fn buffer_target_collects_and_returns_messages() {
        let b = BufferTarget::new();
        b.log_record(&rec("ui", "first"));
        b.log_record(&rec("ui", "second"));

        let msgs = b.get_messages();
        assert_eq!(msgs.len(), 2);
        // Default formatter includes level/component/message.
        assert!(msgs[0].contains("first"));
        assert!(msgs[1].contains("second"));
    }

    #[test]
    fn buffer_target_clear_empties_messages() {
        let b = BufferTarget::new();
        b.log_record(&rec("ui", "boom"));
        assert_eq!(b.get_messages().len(), 1);

        b.clear();
        assert!(b.get_messages().is_empty());
    }

    #[test]
    fn chain_target_sends_to_both() {
        let a = Arc::new(BufferTarget::new());
        let b = Arc::new(BufferTarget::new());
        let chain = ChainTarget::new(a.clone(), b.clone());
        chain.log_record(&rec("ui", "boom"));
        chain.flush();

        let a_msgs = a.get_messages();
        let b_msgs = b.get_messages();
        assert_eq!(a_msgs.len(), 1);
        assert_eq!(b_msgs.len(), 1);
        assert!(a_msgs[0].contains("boom"));
        assert!(b_msgs[0].contains("boom"));
    }
}
