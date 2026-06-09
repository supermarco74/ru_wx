//! Win32 error integration — captures and formats GetLastError/FormatMessageW

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Diagnostics::Debug::{
    FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
};

use super::levels::LogLevel;
use super::manager::log_message;

/// Read the current thread's `GetLastError` value.
///
/// Note: on Windows, `GetLastError` is thread-local, so this only
/// reflects the most recent failing Win32 call made on the calling
/// thread.
pub fn get_last_win32_error() -> u32 {
    // SAFETY: FFI call to GetLastError; `FormatMessageW` writes into a 512-u16 stack buffer that we then truncate to `len`.
    unsafe { GetLastError() }
}

/// Resolve `error_code` into a human-readable description using
/// `FormatMessageW`. Returns the literal string `"No error"` when
/// `error_code` is 0, and a synthetic `"Unknown error (code: N)"`
/// string when the system formatter fails.
pub fn format_win32_error(error_code: u32) -> String {
    if error_code == 0 {
        return "No error".to_string();
    }

    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let mut buffer: [u16; 512] = [0; 512];
        let len = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            std::ptr::null(),
            error_code,
            0, // Default language
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            std::ptr::null(),
        );

        if len == 0 {
            return format!("Unknown error (code: {})", error_code);
        }

        // Convert UTF-16 to String, trimming trailing \r\n
        String::from_utf16_lossy(&buffer[..len as usize])
            .trim()
            .to_string()
    }
}

/// Log a Win32 system error with the given `context` message. This
/// fetches `GetLastError`, formats the code with
/// [`format_win32_error`] and emits a single `Error`-level record
/// under the `"win32"` component. If `GetLastError` returns 0 the
/// call is a no-op.
pub fn log_win32_error(context: &str) {
    let error_code = get_last_win32_error();
    if error_code == 0 {
        return;
    }
    let error_msg = format_win32_error(error_code);
    log_message(
        LogLevel::Error,
        "win32",
        format!("{}: {} (error code: {})", context, error_msg, error_code),
    );
}
