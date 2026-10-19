//! Open a URL in the user's default system browser.

/// Launch `url` with the platform default handler (typically the default web browser).
/// Returns `true` when the OS accepted the request.
pub fn open_url_in_browser(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSString, NSURL};

        let ns = NSString::from_str(url);
        let Some(ns_url) = NSURL::URLWithString(&ns) else {
            return false;
        };
        let workspace = NSWorkspace::sharedWorkspace();
        return workspace.openURL(&ns_url);
    }
    #[cfg(target_os = "windows")]
    {
        use crate::platform::win32::to_wide;
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;

        let wide_url = to_wide(url);
        let wide_open = to_wide("open");
        // SAFETY: ShellExecuteW with null parent HWND and valid wide strings.
        let result = unsafe {
            ShellExecuteW(
                HWND::default(),
                wide_open.as_ptr(),
                wide_url.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            )
        };
        // Values > 32 indicate success per ShellExecuteW documentation.
        (result as isize) > 32
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
        false
    }
}
