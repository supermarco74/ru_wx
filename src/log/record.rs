//! Single log entry with the metadata needed to render it.
//!
//! A [`LogRecord`] is what the logging macros
//! ([`wx_log_error!`](crate::wx_log_error),
//! [`wx_log_trace!`](crate::wx_log_trace), etc.) construct and
//! what every [`LogTarget`](super::target::LogTarget) consumes.
//! The record owns its own copy of the level, the component
//! string, the message string, and the timestamp; the
//! formatter and the targets treat the record as an
//! immutable value.

use super::levels::LogLevel;
use std::time::SystemTime;

/// A single log record with metadata. Created by the logging macros
/// (e.g. [`crate::wx_log_error!`]) and forwarded to the active
/// [`LogTarget`](super::target::LogTarget).
pub struct LogRecord {
    /// Severity level of the record.
    pub level: LogLevel,
    /// Hierarchical component name (e.g. `"ui/dialog/buttons"`).
    /// Used for per-component filtering via
    /// [`crate::log::set_component_level`].
    pub component: String,
    /// The formatted user message.
    pub message: String,
    /// Wall-clock timestamp captured at construction.
    pub timestamp: SystemTime,
    /// The name of the OS thread that emitted the record, if the
    /// thread was given one (via [`std::thread::Builder::name`]).
    pub thread_name: Option<String>,
    /// Source file of the emitting call site (debug builds only).
    #[cfg(debug_assertions)]
    pub file: Option<&'static str>,
    /// Source line of the emitting call site (debug builds only).
    #[cfg(debug_assertions)]
    pub line: u32,
}

impl LogRecord {
    /// Build a new record with the current timestamp and the calling
    /// thread's name. The `component` and `message` are copied so the
    /// record owns its data independently of the caller's lifetime.
    pub fn new(level: LogLevel, component: &str, message: String) -> Self {
        Self {
            level,
            component: component.to_string(),
            message,
            timestamp: SystemTime::now(),
            thread_name: std::thread::current().name().map(|s| s.to_string()),
            #[cfg(debug_assertions)]
            file: None,
            #[cfg(debug_assertions)]
            line: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_new_copies_component_and_message() {
        let r = LogRecord::new(LogLevel::Warning, "ui/button", "clicked".to_string());
        assert_eq!(r.level, LogLevel::Warning);
        assert_eq!(r.component, "ui/button");
        assert_eq!(r.message, "clicked");
    }

    #[test]
    fn record_new_owns_strings_independently() {
        // We pass a &str component — the record must own its copy.
        let component_str = String::from("net");
        let r = LogRecord::new(LogLevel::Info, &component_str, "hello".to_string());
        drop(component_str);
        assert_eq!(r.component, "net");
        assert_eq!(r.message, "hello");
    }
}
