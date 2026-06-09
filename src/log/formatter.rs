//! Human-readable formatting of [`LogRecord`] values into `String`.
//!
//! The default [`LogFormatter`] produces a single-line, plain-text
//! representation of a log record: an optional timestamp, the
//! level, the component (if non-empty), the thread name (if
//! enabled and present), and the formatted message. The
//! timestamp and thread-name segments can be toggled
//! independently via [`LogFormatter::with_timestamp`] and
//! [`LogFormatter::with_thread`].

use super::record::LogRecord;
use std::time::UNIX_EPOCH;

/// Formats log records into human-readable strings.
///
/// The default configuration emits a timestamp, the level tag and
/// (when present) the component name. Thread name and additional
/// fields can be toggled via the `with_*` builder methods.
///
/// # Example
/// ```no_run
/// use ru_wx::log::{LogFormatter, LogRecord, LogLevel};
/// let f = LogFormatter::new().with_thread(true);
/// let s = f.format(&LogRecord::new(LogLevel::Warning, "ui", "click".into()));
/// assert!(s.contains("[WARN]"));
/// ```
pub struct LogFormatter {
    show_timestamp: bool,
    show_level: bool,
    show_component: bool,
    show_thread: bool,
}

impl LogFormatter {
    /// Build a formatter with the default field set: timestamp, level
    /// and component name on, thread name off.
    pub fn new() -> Self {
        Self {
            show_timestamp: true,
            show_level: true,
            show_component: true,
            show_thread: false,
        }
    }

    /// Enable or disable the `HH:MM:SS.mmm` timestamp prefix.
    pub fn with_timestamp(mut self, show: bool) -> Self {
        self.show_timestamp = show;
        self
    }

    /// Enable or disable the `[thread-name]` block. The block is
    /// emitted only when the record's thread has a name.
    pub fn with_thread(mut self, show: bool) -> Self {
        self.show_thread = show;
        self
    }

    /// Render `record` into a single-line string with the active
    /// field set, joined by spaces.
    pub fn format(&self, record: &LogRecord) -> String {
        let mut parts = Vec::new();

        if self.show_timestamp {
            let duration = record
                .timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let secs = duration.as_secs();
            let hours = (secs / 3600) % 24;
            let minutes = (secs / 60) % 60;
            let seconds = secs % 60;
            let millis = duration.subsec_millis();
            parts.push(format!(
                "{:02}:{:02}:{:02}.{:03}",
                hours, minutes, seconds, millis
            ));
        }

        if self.show_level {
            parts.push(format!("[{}]", record.level.as_str()));
        }

        if self.show_component && !record.component.is_empty() {
            parts.push(format!("[{}]", record.component));
        }

        if self.show_thread {
            if let Some(ref name) = record.thread_name {
                parts.push(format!("[{}]", name));
            }
        }

        parts.push(record.message.clone());
        parts.join(" ")
    }
}

impl Default for LogFormatter {
    /// Equivalent to [`LogFormatter::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::levels::LogLevel;
    use super::*;

    fn rec(component: &str, message: &str) -> LogRecord {
        LogRecord::new(LogLevel::Error, component, message.to_string())
    }

    #[test]
    fn default_formatter_includes_timestamp_level_component_and_message() {
        let f = LogFormatter::new();
        let s = f.format(&rec("ui", "boom"));
        // HH:MM:SS.mmm [ERROR] [ui] boom
        let parts: Vec<&str> = s.split_whitespace().collect();
        // timestamp, [LEVEL], [component], message
        assert!(parts.len() >= 4, "got {:?}", s);
        assert!(parts[0].contains(':'), "timestamp missing: {:?}", parts[0]);
        assert_eq!(parts[1], "[ERROR]");
        assert_eq!(parts[2], "[ui]");
        assert_eq!(parts[3], "boom");
    }

    #[test]
    fn without_timestamp_only_keeps_level_component_message() {
        let f = LogFormatter::new().with_timestamp(false);
        let s = f.format(&rec("ui", "boom"));
        // [ERROR] [ui] boom
        let parts: Vec<&str> = s.split_whitespace().collect();
        assert_eq!(parts, ["[ERROR]", "[ui]", "boom"]);
    }

    #[test]
    fn empty_component_is_omitted() {
        let f = LogFormatter::new().with_timestamp(false);
        let s = f.format(&rec("", "boom"));
        // [ERROR] boom  (no [component] block)
        let parts: Vec<&str> = s.split_whitespace().collect();
        assert_eq!(parts, ["[ERROR]", "boom"]);
    }

    #[test]
    fn with_thread_false_never_emits_thread_block() {
        // with_thread(false) must not emit the thread block even when the
        // current thread has a name (cargo test threads do).
        let f = LogFormatter::new().with_timestamp(false).with_thread(false);
        let s = f.format(&rec("ui", "boom"));
        let parts: Vec<&str> = s.split_whitespace().collect();
        assert_eq!(parts, ["[ERROR]", "[ui]", "boom"]);
    }

    #[test]
    fn with_thread_true_emits_thread_block_when_thread_has_a_name() {
        // cargo test runs each test on a named thread like
        // "log::formatter::tests::...". We use a Builder to guarantee
        // the test runs on a named thread.
        let name = "ru_wx_test_thread_named";
        let result = std::thread::Builder::new()
            .name(name.to_string())
            .spawn(|| {
                let f = LogFormatter::new().with_timestamp(false).with_thread(true);
                f.format(&rec("ui", "boom"))
            })
            .unwrap()
            .join()
            .unwrap();

        // Expect exactly: [ERROR] [ui] [<thread-name>] boom
        let parts: Vec<&str> = result.split_whitespace().collect();
        assert_eq!(parts[0], "[ERROR]");
        assert_eq!(parts[1], "[ui]");
        assert_eq!(parts[2], format!("[{}]", name));
        assert_eq!(parts[3], "boom");
    }
}
