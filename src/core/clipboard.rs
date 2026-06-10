//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! System clipboard (`wxClipboard`) — text on Win32.
//!
//! All methods are synchronous and should be called from the UI thread.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HGLOBAL;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn GlobalFree(h: HGLOBAL) -> HGLOBAL;
}

/// Clipboard access (text only for now).
pub struct Clipboard;

impl Clipboard {
    /// Place UTF-8 text on the clipboard. Returns `false` on failure.
    #[cfg(target_os = "windows")]
    pub fn set_text(text: &str) -> bool {
        // SAFETY: Standard Win32 clipboard sequence with global memory we own.
        unsafe {
            if OpenClipboard(0 as _) == 0 {
                return false;
            }
            let _ = EmptyClipboard();
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = wide.len() * 2;
            let hmem = GlobalAlloc(0x0002, bytes); // GMEM_MOVEABLE
            if hmem.is_null() {
                CloseClipboard();
                return false;
            }
            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                GlobalFree(hmem);
                CloseClipboard();
                return false;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes);
            GlobalUnlock(hmem);
            let handle = SetClipboardData(13, hmem as _); // CF_UNICODETEXT = 13
            let ok = !handle.is_null();
            if !ok {
                GlobalFree(hmem);
            }
            CloseClipboard();
            ok
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_text(_text: &str) -> bool {
        false
    }

    /// Read Unicode text from the clipboard.
    #[cfg(target_os = "windows")]
    pub fn get_text() -> Option<String> {
        unsafe {
            if OpenClipboard(0 as _) == 0 {
                return None;
            }
            let handle = GetClipboardData(13);
            if handle.is_null() {
                CloseClipboard();
                return None;
            }
            let ptr = GlobalLock(handle);
            if ptr.is_null() {
                CloseClipboard();
                return None;
            }
            let wide = std::slice::from_raw_parts(ptr as *const u16, {
                let mut len = 0usize;
                while *((ptr as *const u16).add(len)) != 0 {
                    len += 1;
                    if len > 1_000_000 {
                        break;
                    }
                }
                len
            });
            let text = String::from_utf16_lossy(wide);
            GlobalUnlock(handle);
            CloseClipboard();
            Some(text)
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn get_text() -> Option<String> {
        None
    }

    /// Clear the clipboard.
    #[cfg(target_os = "windows")]
    pub fn clear() -> bool {
        unsafe {
            if OpenClipboard(0 as _) == 0 {
                return false;
            }
            let ok = EmptyClipboard() != 0;
            CloseClipboard();
            ok
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn clear() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn clipboard_api_exists_off_windows() {
        #[cfg(not(target_os = "windows"))]
        {
            assert!(!super::Clipboard::set_text("x"));
            assert!(super::Clipboard::get_text().is_none());
        }
    }
}
