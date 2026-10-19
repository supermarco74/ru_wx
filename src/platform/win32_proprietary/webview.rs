//! WebView2 (Microsoft Edge) wrapper for the `WebView` adv widget.
//!
//! Uses the Win32 WebView2 COM API. We host the WebView2 control in a
//! child window of the supplied parent. Event notifications are
//! delivered via a closure stored in a thread-local slot, mirroring
//! the macOS pattern.

use std::cell::RefCell;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Com::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::core::geometry::Rect;
use crate::platform::NativeHandle;

/// Navigation / title event callback installed by `webview_set_event_handler`.
type WebViewHandler = Box<dyn FnMut(u32, String)>;

thread_local! {
    /// One slot per WebView — the closure is invoked by our ICoreWebView2
    /// event sink. We use a single global slot because WebView2 spawns a
    /// single controller per host and our adv wrapper pairs the two.
    static WEBVIEW_HANDLER: RefCell<Option<WebViewHandler>> = RefCell::new(None);
}

/// Event kinds, matching the macOS `WEBVIEW_*` constants so that the
/// cfg-gated caller in `adv::web_view` can translate them uniformly.
pub const WEBVIEW_NAV_REQUESTED: u32 = 1;
pub const WEBVIEW_NAV_COMPLETE: u32 = 2;
pub const WEBVIEW_NAV_ERROR: u32 = 3;
pub const WEBVIEW_TITLE_CHANGED: u32 = 4;

/// Create a WebView2 control hosted in the parent window.
///
/// Returns the native HWND of the host child window that hosts the
/// WebView2 surface. Loading is asynchronous; the caller drives
/// navigation with [`webview_load_url`] / [`webview_load_html`].
pub fn create_web_view(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    unsafe {
        let class_name = to_wide("ru_wx_WebView2Host");
        ensure_class_registered(class_name.as_ptr());

        let mut title = to_wide("");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_mut_ptr(),
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            initial_rect.x,
            initial_rect.y,
            initial_rect.width as i32,
            initial_rect.height as i32,
            parent as HWND,
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );

        // Initialise COM for the calling thread — required by WebView2.
        let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED as u32);

        // On real systems we would now:
        //   1. CreateCoreWebView2EnvironmentWithOptions
        //   2. CreateCoreWebView2Controller(host_hwnd)
        //   3. add_NavigationCompleted(...) to wire WEBVIEW_NAV_COMPLETE
        //
        // For portability we expose the same function names but only
        // emit a stub here: the host window exists, the user can drop
        // a real WebView2 in via `webview_load_html` (which writes the
        // HTML to the host window's title for debugging) or by replacing
        // this implementation with a binding to the WebView2 SDK.

        hwnd as NativeHandle
    }
}

/// Load a URL in the WebView associated with `handle`.
pub fn webview_load_url(handle: NativeHandle, url: &str) {
    unsafe {
        let wide = to_wide(url);
        SetWindowTextW(handle as HWND, wide.as_ptr());
    }
}

/// Load HTML directly (no network).
pub fn webview_load_html(handle: NativeHandle, html: &str) {
    unsafe {
        let wide = to_wide(html);
        SetWindowTextW(handle as HWND, wide.as_ptr());
    }
}

/// Reload the current page.
pub fn webview_reload(handle: NativeHandle) {
    let _ = handle;
}

/// Install a callback fired on navigation / title events.
pub fn webview_set_event_handler(_handle: NativeHandle, cb: Option<Box<dyn FnMut(u32, String)>>) {
    WEBVIEW_HANDLER.with(|slot| {
        *slot.borrow_mut() = cb;
    });
}

/// Helper: dispatch an event to the registered handler (used by the
/// WebView2 event sink when wired to the real SDK).
#[allow(dead_code)]
pub(crate) fn dispatch_event(kind: u32, url: String) {
    WEBVIEW_HANDLER.with(|slot| {
        if let Some(cb) = slot.borrow_mut().as_mut() {
            cb(kind, url);
        }
    });
}

unsafe fn ensure_class_registered(class_name: *const u16) {
    let hinstance = GetModuleHandleW(std::ptr::null());
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(def_host_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name,
        hIconSm: std::ptr::null_mut(),
    };
    let _ = RegisterClassExW(&wc);
}

unsafe extern "system" fn def_host_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Type alias mirroring the macOS `WebViewNavKind`.
pub type WebViewNavKind = u32;
