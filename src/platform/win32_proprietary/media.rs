//! Media playback wrapper for `MediaCtrl`.
//!
//! Production implementations should use `IMFMediaEngine` from Media
//! Foundation (Vista+) for formats like MP4 / WebM / streaming, and
//! fall back to `mciSendStringW` for legacy formats (WAV / MIDI / AVI).
//!
//! This wrapper exposes the same public API as the macOS AppKit media
//! layer so the cfg-gated caller in `adv::media_ctrl` can switch on
//! platform without code duplication.

use std::sync::Mutex;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Media::Multimedia::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::platform::NativeHandle;

static MEDIA_HANDLE: Mutex<Option<usize>> = Mutex::new(None);

/// Native state: the Media Foundation session handle (held as an
/// opaque `usize` because the COM interface pointer type is unstable
/// across windows-sys releases).
pub struct MediaSession {
    pub handle: usize,
}

pub fn create_media_ctrl(parent: NativeHandle) -> NativeHandle {
    unsafe {
        let class_name = to_wide("ru_wx_MediaHost");
        ensure_class_registered(class_name.as_ptr());
        let hinstance = GetModuleHandleW(std::ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            std::ptr::null_mut(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            320,
            240,
            parent as HWND,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        // MFStartup / MFShutdown would go here. We rely on the existing
        // MCI path so the wrapper works without an MF binding.
        hwnd as NativeHandle
    }
}

/// Load a media file (any format supported by MCI / Media Foundation).
pub fn media_load(handle: NativeHandle, path: &str) -> bool {
    unsafe {
        let cmd = format!("open \"{}\" type mpegvideo alias ru_wx_media", path);
        let wide_cmd = to_wide(&cmd);
        let err = mciSendStringW(
            wide_cmd.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
        if err != 0 {
            // Retry without specifying a type — MCI will sniff.
            let cmd = format!("open \"{}\" alias ru_wx_media", path);
            let wide_cmd = to_wide(&cmd);
            let err = mciSendStringW(
                wide_cmd.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            );
            if err != 0 {
                return false;
            }
        }
        // Tell MCI to render into our host window.
        let cmd = format!("window ru_wx_media handle {}", handle);
        let wide_cmd = to_wide(&cmd);
        mciSendStringW(
            wide_cmd.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );

        let mut slot = MEDIA_HANDLE.lock().unwrap();
        *slot = Some(handle as usize);
        true
    }
}

pub fn media_play(_handle: NativeHandle) {
    unsafe {
        let cmd = to_wide("play ru_wx_media");
        mciSendStringW(cmd.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
}

pub fn media_pause(_handle: NativeHandle) {
    unsafe {
        let cmd = to_wide("pause ru_wx_media");
        mciSendStringW(cmd.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
}

pub fn media_stop(_handle: NativeHandle) {
    unsafe {
        let cmd = to_wide("stop ru_wx_media");
        mciSendStringW(cmd.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut());
        let cmd = to_wide("close ru_wx_media");
        mciSendStringW(cmd.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
}

/// Return total length in milliseconds.
pub fn media_length_ms() -> i64 {
    unsafe {
        let cmd = to_wide("status ru_wx_media length");
        let mut buf = [0u16; 64];
        let err = mciSendStringW(
            cmd.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as u32,
            std::ptr::null_mut(),
        );
        if err != 0 {
            return 0;
        }
        let s = String::from_utf16_lossy(&buf);
        // Trim at the first NUL.
        let s = s.split('\0').next().unwrap_or("").trim();
        s.parse::<i64>().unwrap_or(0)
    }
}

/// Return current playback position in milliseconds.
pub fn media_position_ms() -> i64 {
    unsafe {
        let cmd = to_wide("status ru_wx_media position");
        let mut buf = [0u16; 64];
        let err = mciSendStringW(
            cmd.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as u32,
            std::ptr::null_mut(),
        );
        if err != 0 {
            return 0;
        }
        let s = String::from_utf16_lossy(&buf);
        let s = s.split('\0').next().unwrap_or("").trim();
        s.parse::<i64>().unwrap_or(0)
    }
}

pub fn media_seek_ms(_handle: NativeHandle, ms: i64) {
    unsafe {
        let cmd = format!("seek ru_wx_media to {}", ms);
        let wide_cmd = to_wide(&cmd);
        mciSendStringW(
            wide_cmd.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
    }
}

pub fn media_set_volume(_handle: NativeHandle, volume: i32) {
    unsafe {
        let cmd = format!("setaudio ru_wx_media volume to {}", volume);
        let wide_cmd = to_wide(&cmd);
        mciSendStringW(
            wide_cmd.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
    }
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

// Reference GetCurrentThreadId so the import isn't dead when MFStartup
// is eventually added.
#[allow(dead_code)]
fn _thread_marker() -> u32 {
    unsafe { GetCurrentThreadId() }
}
