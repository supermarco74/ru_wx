//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! RAII guards that temporarily override the active log target or
//! suppress all logging for the lifetime of the guard.
//!
//! The two public types in this module are
//! [`LogNull`](super::LogNull) (suppresses all log forwarding
//! while it is alive) and [`ApiGuard`](super::ApiGuard) (logs
//! the last `Win32` API error on scope exit). The two types
//! cooperate: a [`LogNull`](super::LogNull) guard will also
//! silence the [`ApiGuard`](super::ApiGuard)-emitted
//! `GetLastError()` message if it is the outermost guard, so
//! the canonical "log once on failure" pattern is
//! `ApiGuard::new(name)` inside a `LogNull` block.

use super::manager;
use super::target::{LogTarget, NullTarget};
use std::sync::Arc;

/// RAII guard that suppresses all logging while in scope.
/// Equivalent to wxLogNull in wxWidgets.
///
/// # Example
/// ```
/// use ru_wx::log::LogNull;
/// {
///     let _guard = LogNull::new();
///     // All logging suppressed here
/// }
/// // Logging resumes after guard is dropped
/// ```
pub struct LogNull {
    previous_target: Arc<dyn LogTarget>,
}

impl LogNull {
    /// Install a [`NullTarget`] as the active log target for the
    /// lifetime of this guard, replacing whatever target was active
    /// before. The previous target is restored on drop.
    pub fn new() -> Self {
        let previous = manager::set_active_target(Arc::new(NullTarget));
        LogNull {
            previous_target: previous,
        }
    }
}

impl Default for LogNull {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LogNull {
    fn drop(&mut self) {
        manager::set_active_target(self.previous_target.clone());
    }
}
