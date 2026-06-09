//! Block-level API error tracing.
//! Wraps a block of Win32 API calls and automatically logs any errors on scope exit.

use super::levels::LogLevel;
use super::manager::log_message;

/// RAII guard for tracing Win32 API errors in a block.
///
/// When the guard is dropped, it checks if a Win32 error occurred during the block
/// and logs it automatically.
///
/// # Example
/// ```
/// use ru_wx::log::ApiGuard;
/// {
///     let _guard = ApiGuard::new("Creating main window");
///     // ... Win32 API calls ...
/// }  // If any call set an error, it's logged here
/// ```
pub struct ApiGuard {
    operation: String,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    initial_error: u32,
}

impl ApiGuard {
    /// Create a new guard around `operation`. The current thread's
    /// `GetLastError` value is captured and then reset to 0 so the
    /// guard only reports errors that occur inside the guarded block.
    pub fn new(operation: &str) -> Self {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let initial_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };

        // Clear the error so we only catch new errors in this block
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            windows_sys::Win32::Foundation::SetLastError(0);
        }

        ApiGuard {
            operation: operation.to_string(),
            #[cfg(target_os = "windows")]
            initial_error,
        }
    }

    /// Manually check the current `GetLastError` value and log a
    /// formatted message if it is non-zero. This is called
    /// automatically from `Drop`, but can be invoked earlier for
    /// finer-grained control.
    pub fn check(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let current_error = windows_sys::Win32::Foundation::GetLastError();
            if current_error != 0 {
                let error_msg = super::win32_error::format_win32_error(current_error);
                log_message(
                    LogLevel::Error,
                    "win32/api",
                    format!(
                        "[{}] {} (error code: {})",
                        self.operation, error_msg, current_error
                    ),
                );
            }
        }
    }
}

impl Drop for ApiGuard {
    fn drop(&mut self) {
        self.check();
    }
}
