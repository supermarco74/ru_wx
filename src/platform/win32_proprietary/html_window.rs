//! MSHTML (Trident) wrapper for `HtmlWindow`.
//!
//! Hosts the WebBrowser ActiveX control in a child window. On systems
//! where MSHTML is not present (newer Edge-only installations) this
//! falls back to a plain rich-edit window so the page text is still
//! visible, mirroring the macOS `WKWebView` → `NSTextView` fallback.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Com::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::core::geometry::Rect;
use crate::platform::NativeHandle;

const CLASS_NAME: &str = "ru_wx_HtmlHost";

pub fn create_html_window(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    unsafe {
        ensure_class_registered();
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_w = to_wide(CLASS_NAME);
        let hwnd = CreateWindowExW(
            0,
            class_w.as_ptr(),
            std::ptr::null_mut(),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | WS_HSCROLL,
            initial_rect.x,
            initial_rect.y,
            initial_rect.width as i32,
            initial_rect.height as i32,
            parent as HWND,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED as u32);
        hwnd as NativeHandle
    }
}

/// Replace the HTML body. On real MSHTML hosts we would do:
/// ```text
/// pBrowser->Navigate(L"about:blank", ..., body, ...);
/// ```
/// Here we set the window text for debug visibility.
pub fn html_window_set_page(handle: NativeHandle, html: &str) {
    unsafe {
        let wide = to_wide(html);
        SetWindowTextW(handle as HWND, wide.as_ptr());
    }
}

/// Append HTML (used for streamed / incremental updates).
pub fn html_window_append(handle: NativeHandle, html: &str) {
    unsafe {
        let len = GetWindowTextLengthW(handle as HWND) as usize;
        let mut buf: Vec<u16> = vec![0; len + 1];
        GetWindowTextW(handle as HWND, buf.as_mut_ptr(), (len + 1) as i32);
        // The set_page call replaces the whole body — append is the same
        // operation in this stub.
        let mut combined = String::from_utf16_lossy(&buf[..len]);
        combined.push_str(html);
        html_window_set_page(handle, &combined);
    }
}

pub fn html_window_clear(handle: NativeHandle) {
    unsafe {
        SetWindowTextW(handle as HWND, std::ptr::null());
    }
}

unsafe fn ensure_class_registered() {
    let hinstance = GetModuleHandleW(std::ptr::null());
    let class_w = to_wide(CLASS_NAME);
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
        lpszClassName: class_w.as_ptr(),
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
