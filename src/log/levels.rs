/// Log severity levels, matching the wxWidgets `wxLogLevel` numeric
/// values. A log record's `LogLevel` decides whether it is forwarded
/// to the active [`LogTarget`](super::target::LogTarget): only records
/// whose level is `<=` the active level (set with
/// [`crate::log::set_log_level`]) make it through.
///
/// The numeric values are part of the documented contract (see the
/// `level_discriminants_match_wxwidgets` test) and must not change.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Unrecoverable error, e.g. an assertion failure. Always emitted
    /// because every other level is greater.
    FatalError = 1,
    /// An error condition that the user should know about.
    Error = 2,
    /// A condition that does not abort the operation but warrants
    /// the user's attention.
    Warning = 3,
    /// A high-level user-visible status message.
    Message = 4,
    /// Informational detail (the level between `Message` and `Verbose`).
    Info = 5,
    /// Verbose tracing, normally hidden in production.
    Verbose = 6,
    /// Debug-level diagnostics, intended for development.
    Debug = 7,
    /// Most detailed tracing; use the [`crate::wx_log_trace!`] macro.
    Trace = 8,
}

impl LogLevel {
    /// Short uppercase identifier for the level, matching the
    /// wxWidgets log formatter conventions.
    ///
    /// # Example
    /// ```
    /// use ru_wx::log::LogLevel;
    /// assert_eq!(LogLevel::Error.as_str(), "ERROR");
    /// assert_eq!(LogLevel::FatalError.as_str(), "FATAL");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FatalError => "FATAL",
            Self::Error => "ERROR",
            Self::Warning => "WARNING",
            Self::Message => "MESSAGE",
            Self::Info => "INFO",
            Self::Verbose => "VERBOSE",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_as_str_matches_wxwidgets_names() {
        assert_eq!(LogLevel::FatalError.as_str(), "FATAL");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Warning.as_str(), "WARNING");
        assert_eq!(LogLevel::Message.as_str(), "MESSAGE");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Verbose.as_str(), "VERBOSE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
    }

    #[test]
    fn level_display_matches_as_str() {
        for lvl in [
            LogLevel::FatalError,
            LogLevel::Error,
            LogLevel::Warning,
            LogLevel::Message,
            LogLevel::Info,
            LogLevel::Verbose,
            LogLevel::Debug,
            LogLevel::Trace,
        ] {
            assert_eq!(format!("{}", lvl), lvl.as_str());
        }
    }

    #[test]
    fn level_ordering_is_fatal_to_trace() {
        // Lower severity values should sort before higher ones, so
        // FatalError (1) is the most severe and Trace (8) the least.
        assert!(LogLevel::FatalError < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Message);
        assert!(LogLevel::Message < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Verbose);
        assert!(LogLevel::Verbose < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn level_discriminants_match_wxwidgets() {
        // These numeric values must match wxWidgets' wxLogLevel constants
        // so we stay wire-compatible with any future cross-tooling.
        assert_eq!(LogLevel::FatalError as u32, 1);
        assert_eq!(LogLevel::Error as u32, 2);
        assert_eq!(LogLevel::Warning as u32, 3);
        assert_eq!(LogLevel::Message as u32, 4);
        assert_eq!(LogLevel::Info as u32, 5);
        assert_eq!(LogLevel::Verbose as u32, 6);
        assert_eq!(LogLevel::Debug as u32, 7);
        assert_eq!(LogLevel::Trace as u32, 8);
    }
}
