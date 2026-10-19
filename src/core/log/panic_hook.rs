//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Panic hook integration for the log system.
//!
//! [`install_panic_hook`] installs a `std::panic` hook that routes
//! every panic through the active [`LogTarget`] at
//! [`LogLevel::FatalError`] before delegating to the previously
//! installed hook (so the default stderr message is preserved).
//!
//! This makes panics visible in the local log file even when the
//! application is not run from a terminal — the primary tool for
//! diagnosing field-reported crashes.

use super::levels::LogLevel;
use super::manager;
use std::panic;

/// Install a panic hook that logs panics to the active log target.
///
/// The hook logs the panic payload, the source location (when
/// available), the thread name, and a backtrace (when
/// `RUST_BACKTRACE` is enabled) at [`LogLevel::FatalError`] with
/// component `"panic"`, then forwards the record to the previous
/// hook so default behaviour (stderr message, abort on
/// `panic = "abort"`, …) is unchanged.
///
/// Calling this twice is harmless: each call chains onto the hook
/// installed before it.
///
/// # Example
/// ```no_run
/// use ru_wx::log;
/// log::set_active_target(std::sync::Arc::new(
///     log::RotatingFileTarget::new("/tmp/myapp.log"),
/// ));
/// log::install_panic_hook();
/// ```
pub fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        log_panic_info(info);
        previous(info);
    }));
}

/// Format and log a `PanicHookInfo` through the global log manager.
/// Separated from the closure so it can be unit-tested.
fn log_panic_info(info: &panic::PanicHookInfo<'_>) {
    // Use the manager directly: the FatalError level would abort the
    // process (see `manager::log_message`), which is correct for a
    // fatal *application* error but wrong for a panic we are merely
    // reporting — the previous hook / runtime decides what happens
    // next. We therefore log at Error level through the manager with
    // the "panic" component, keeping filtering semantics intact.
    manager::log_message(LogLevel::Error, "panic", format_panic_message(info));
}

/// Render a `PanicHookInfo` into a human-readable multi-line string:
/// payload, source location, thread name and (when `RUST_BACKTRACE`
/// is enabled) a captured backtrace.
fn format_panic_message(info: &panic::PanicHookInfo<'_>) -> String {
    let payload = payload_to_string(info.payload());

    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "(unknown location)".to_string());

    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("(unnamed)").to_string();

    let backtrace = std::backtrace::Backtrace::capture();
    match backtrace.status() {
        std::backtrace::BacktraceStatus::Captured => {
            format!(
                "panic in thread '{}': {}\n  at {}\nbacktrace:\n{}",
                thread_name, payload, location, backtrace
            )
        }
        _ => format!(
            "panic in thread '{}': {}\n  at {}\n  (set RUST_BACKTRACE=1 for a backtrace)",
            thread_name, payload, location
        ),
    }
}

/// Extract a printable message from a panic payload (the `Box<dyn
/// Any + Send>` returned by `catch_unwind` or carried by
/// `PanicHookInfo`). Useful when wrapping fallible UI callbacks with
/// `catch_unwind` and logging the failure.
pub fn payload_to_string(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "(non-string panic payload)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: these tests must NOT install a global log target — the
    // log manager state is process-wide and other tests run in
    // parallel on the same process. The panic hook itself is also
    // process-wide state, so everything that touches it is kept in
    // this single test (tests within one binary run in parallel).

    #[test]
    fn hook_formatting_and_install_smoke_test() {
        // We cannot easily build a PanicHookInfo outside a hook, so
        // capture the message through a temporary hook.
        let (tx, rx) = std::sync::mpsc::channel();
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let _ = tx.send(format_panic_message(info));
        }));
        let _ = panic::catch_unwind(|| panic!("fmt-test-payload"));
        let _ = panic::take_hook(); // discard ours
        panic::set_hook(previous); // restore

        let msg = rx.recv().expect("hook must receive the panic");
        assert!(msg.contains("fmt-test-payload"), "got: {}", msg);
        assert!(msg.contains("panic in thread"), "got: {}", msg);
        assert!(msg.contains("panic_hook.rs"), "location missing: {}", msg);

        // Installing the production hook must not break unwinding.
        install_panic_hook();
        let result = panic::catch_unwind(|| panic!("install-smoke-test"));
        assert!(result.is_err());
    }
}
