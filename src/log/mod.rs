//! Logging system ported from wxWidgets wxLog.
//!
//! Provides pluggable log targets, level-based filtering, RAII guards,
//! and Win32 API error integration.

mod api_guard;
mod formatter;
mod guards;
mod levels;
mod manager;
mod record;
mod target;
#[cfg(target_os = "windows")]
mod win32_error;

pub use api_guard::ApiGuard;
pub use formatter::LogFormatter;
pub use guards::LogNull;
pub use levels::LogLevel;
pub use manager::{
    get_active_target, get_log_level, is_level_enabled, log_message, set_active_target,
    set_component_level, set_log_level,
};
pub use record::LogRecord;
pub use target::{BufferTarget, ChainTarget, LogTarget, NullTarget, StderrTarget};
#[cfg(target_os = "windows")]
pub use win32_error::{format_win32_error, get_last_win32_error, log_win32_error};

/// Log an error message at the `Error` level.
///
/// The message is filtered by the active log level (see
/// [`set_log_level`]) and any per-component rules (see
/// [`set_component_level`]) before being forwarded to the active
/// [`LogTarget`].
///
/// # Example
/// ```no_run
/// use ru_wx::wx_log_error;
/// wx_log_error!("failed to open file: {}", "config.toml");
/// ```
#[macro_export]
macro_rules! wx_log_error {
    ($($arg:tt)*) => {
        $crate::log::log_message(
            $crate::log::LogLevel::Error,
            "",
            format!($($arg)*),
        );
    };
}

/// Log a warning message at the `Warning` level.
///
/// Same filtering rules as [`wx_log_error`]. Use for conditions that
/// the user should be aware of but that do not abort the operation.
///
/// # Example
/// ```no_run
/// use ru_wx::wx_log_warning;
/// wx_log_warning!("retrying connection (attempt {})", 3);
/// ```
#[macro_export]
macro_rules! wx_log_warning {
    ($($arg:tt)*) => {
        $crate::log::log_message(
            $crate::log::LogLevel::Warning,
            "",
            format!($($arg)*),
        );
    };
}

/// Log an informational message at the `Message` level.
///
/// Use for high-level user-visible status updates. Filtered out when
/// the active level is `Error` or `Warning`.
///
/// # Example
/// ```no_run
/// use ru_wx::wx_log_message;
/// wx_log_message!("loaded {} records", 42);
/// ```
#[macro_export]
macro_rules! wx_log_message {
    ($($arg:tt)*) => {
        $crate::log::log_message(
            $crate::log::LogLevel::Message,
            "",
            format!($($arg)*),
        );
    };
}

/// Log a debug message at the `Debug` level.
///
/// **Only emitted in debug builds** (`#[cfg(debug_assertions)]`).
/// Use for verbose diagnostic output that should not appear in
/// release binaries.
///
/// # Example
/// ```no_run
/// use ru_wx::wx_log_debug;
/// wx_log_debug!("value of x = {}", 7);
/// ```
#[macro_export]
macro_rules! wx_log_debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::log::log_message(
            $crate::log::LogLevel::Debug,
            "",
            format!($($arg)*),
        );
    };
}

/// Log a trace message with a `component` name at the `Trace` level.
///
/// The `component` argument is matched against any per-component rules
/// (see [`set_component_level`]) before the message is forwarded.
/// Hierarchical components are supported: a rule on `"ui"` applies
/// to `"ui/dialog"` and `"ui/dialog/buttons"`.
///
/// # Example
/// ```no_run
/// use ru_wx::wx_log_trace;
/// wx_log_trace!("ui/dialog", "open: title={}", "Hello");
/// ```
#[macro_export]
macro_rules! wx_log_trace {
    ($component:expr, $($arg:tt)*) => {
        $crate::log::log_message(
            $crate::log::LogLevel::Trace,
            $component,
            format!($($arg)*),
        );
    };
}

/// Log a Win32 system error with context (Windows only).
///
/// Captures `GetLastError()` at the call site, formats the message
/// with `format_message`, and emits it as a `LogLevel::Error` message
/// with the user-supplied context string prepended.
///
/// # Example
/// ```no_run
/// # #[cfg(target_os = "windows")]
/// # {
/// use ru_wx::wx_log_sys_error;
/// wx_log_sys_error!("CreateFileW({})", "missing.txt");
/// # }
/// ```
#[cfg(target_os = "windows")]
#[macro_export]
macro_rules! wx_log_sys_error {
    ($($arg:tt)*) => {
        $crate::log::log_win32_error(&format!($($arg)*));
    };
}

/// Wrap a block of Win32 API calls with automatic error logging.
///
/// The `$name` is the function name (or any short context string) to
/// include in the auto-logged error. The wrapped block is run
/// unchanged; if any Win32 API call inside the block sets
/// `GetLastError()`, the error is logged when `$body` returns.
///
/// This is a no-op on non-Windows platforms (the block runs but
/// nothing is logged).
///
/// # Example
/// ```no_run
/// use ru_wx::wx_api_block;
/// wx_api_block!("CreateFileW", {
///     // ... call into Win32 ...
/// });
/// ```
#[macro_export]
macro_rules! wx_api_block {
    ($name:expr, $body:block) => {{
        let _api_guard = $crate::log::ApiGuard::new($name);
        $body
    }};
}
