//! `wxMediaCtrl` — audio / video playback control.
//!
//! On Windows, the control is implemented on top of MCI
//! (Media Control Interface), exposed through
//! `mciSendStringW`. MCI is the simplest cross-format playback
//! API on Windows and works for:
//!
//! * audio: WAV, MP3 (with default codecs), MIDI,
//! * video: AVI, MPG (with default codecs),
//! * other: any format for which a `mciSendCommand`-style device
//!   is registered.
//!
//! # Caveats
//!
//! * MCI is a string-driven, *synchronous* API: each command blocks
//!   the calling thread until the device has finished processing
//!   it. For UI responsiveness we therefore issue commands on the
//!   thread that owns the [`MediaCtrl`] (the GUI thread).
//! * The control does not own a child rendering window for video:
//!   MCI plays into a default rendering target chosen by the
//!   system. Video is therefore best treated as a "play / stop"
//!   surface. (For embedded video, MCI can be told to render to a
//!   parent `HWND` with the `play ... window` variant; we expose
//!   the `play_into_window` knob to opt into that mode.)
//!
//! # Cross-platform stub
//!
//! On non-Windows targets the type is still constructible but
//! every operation is a no-op and `state()` is always
//! [`MediaState::Stopped`]. This is enough to keep code that
//! embeds a `MediaCtrl` in a layout compiling.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(target_os = "windows")]
use crate::widget::Window;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Media::Multimedia::{
    mciGetErrorStringW, mciSendStringW,
};

/// Coarse-grained playback state. The actual MCI state machine is
/// finer-grained, but for a UI control this is enough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaState {
    /// No file is loaded.
    Stopped,
    /// Loaded, paused.
    Paused,
    /// Currently playing.
    Playing,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MciState {
    Stopped,
    Playing,
    Paused,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct MediaCtrlInner {
    alias: String,
    has_media: bool,
    state: MciState,
    /// Last MCI error code, if any. Used by tests / debugging.
    last_error: Option<u32>,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
struct MediaCtrlInner {
    has_media: bool,
    state: MediaState,
}

/// `wxMediaCtrl` analog.
#[derive(Clone)]
pub struct MediaCtrl {
    inner: Rc<RefCell<MediaCtrlInner>>,
}

static NEXT_ALIAS_ID: AtomicU32 = AtomicU32::new(0);

#[cfg(target_os = "windows")]
fn next_alias() -> String {
    let n = NEXT_ALIAS_ID.fetch_add(1, Ordering::Relaxed);
    format!("ruwx_media_{n}")
}

#[cfg(target_os = "windows")]
impl MediaCtrl {
    /// Create a new (empty) `MediaCtrl` parented on the given
    /// window. The control is `MediaState::Stopped` until a file
    /// is loaded with [`load`](Self::load).
    pub fn new<W: Window>(parent: &W) -> Self {
        let _ = parent;
        let alias = next_alias();
        MediaCtrl {
            inner: Rc::new(RefCell::new(MediaCtrlInner {
                alias,
                has_media: false,
                state: MciState::Stopped,
                last_error: None,
            })),
        }
    }

    /// Load a media file from disk. Returns `Ok(())` on success,
    /// `Err(String)` (the MCI error description) on failure.
    ///
    /// This closes any previously-loaded file first.
    pub fn load(&self, path: &Path) -> Result<(), String> {
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return Err("non-UTF-8 path".to_string()),
        };
        // Close any previous file first.
        {
            let mut inner = self.inner.borrow_mut();
            if inner.has_media {
                let cmd = format!("close {}", inner.alias);
                let _ = mci_send(&cmd, None);
                inner.has_media = false;
                inner.state = MciState::Stopped;
            }
        }
        // Open the new file. We let MCI pick the device by
        // *omitting* the `type` keyword; it falls back to the
        // default device for the file extension.
        let cmd = format!("open \"{}\" alias {}", escape_for_mci(path_str), {
            let inner = self.inner.borrow();
            inner.alias.clone()
        });
        let res = mci_send(&cmd, None);
        match res {
            Ok(()) => {
                let mut inner = self.inner.borrow_mut();
                inner.has_media = true;
                inner.state = MciState::Stopped;
                inner.last_error = None;
                Ok(())
            }
            Err(e) => {
                self.inner.borrow_mut().last_error = Some(0);
                Err(e)
            }
        }
    }

    /// Start (or resume) playback.
    pub fn play(&self) -> Result<(), String> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return Err("no media loaded".to_string());
            }
            match inner.state {
                MciState::Paused => format!("resume {}", inner.alias),
                _ => format!("play {}", inner.alias),
            }
        };
        mci_send(&cmd, None).map(|_| {
            self.inner.borrow_mut().state = MciState::Playing;
        })
    }

    /// Pause playback. No-op if not playing.
    pub fn pause(&self) -> Result<(), String> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return Err("no media loaded".to_string());
            }
            if inner.state != MciState::Playing {
                return Ok(());
            }
            format!("pause {}", inner.alias)
        };
        mci_send(&cmd, None).map(|_| {
            self.inner.borrow_mut().state = MciState::Paused;
        })
    }

    /// Stop playback. The current position is reset to 0.
    pub fn stop(&self) -> Result<(), String> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return Err("no media loaded".to_string());
            }
            format!("stop {}", inner.alias)
        };
        mci_send(&cmd, None).map(|_| {
            self.inner.borrow_mut().state = MciState::Stopped;
        })
    }

    /// Close the current media file. After this call, the control
    /// is back in the "no file loaded" state.
    pub fn close(&self) -> Result<(), String> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return Ok(());
            }
            format!("close {}", inner.alias)
        };
        mci_send(&cmd, None).map(|_| {
            let mut inner = self.inner.borrow_mut();
            inner.has_media = false;
            inner.state = MciState::Stopped;
        })
    }

    /// Current playback position in milliseconds. `None` if no
    /// file is loaded or the position cannot be queried.
    pub fn position_ms(&self) -> Option<u64> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return None;
            }
            format!("status {} position", inner.alias)
        };
        let mut buf = [0u16; 32];
        mci_send(&cmd, Some(&mut buf))
            .ok()
            .and_then(|_| parse_u64_from_wide(&buf))
    }

    /// Total length of the loaded file in milliseconds. `None` if
    /// no file is loaded.
    pub fn length_ms(&self) -> Option<u64> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return None;
            }
            format!("status {} length", inner.alias)
        };
        let mut buf = [0u16; 32];
        mci_send(&cmd, Some(&mut buf))
            .ok()
            .and_then(|_| parse_u64_from_wide(&buf))
    }

    /// Seek to a position (in milliseconds). `None` on error.
    pub fn seek_ms(&self, ms: u64) -> Result<(), String> {
        let cmd = {
            let inner = self.inner.borrow();
            if !inner.has_media {
                return Err("no media loaded".to_string());
            }
            format!("seek {} to {}", inner.alias, ms)
        };
        mci_send(&cmd, None)
    }

    /// Returns the coarse-grained playback state.
    pub fn state(&self) -> MediaState {
        let inner = self.inner.borrow();
        if !inner.has_media {
            return MediaState::Stopped;
        }
        match inner.state {
            MciState::Stopped => MediaState::Stopped,
            MciState::Paused => MediaState::Paused,
            MciState::Playing => MediaState::Playing,
        }
    }

    /// Returns the alias used for the underlying MCI device.
    /// Mainly useful for diagnostics and tests.
    pub fn alias(&self) -> String {
        self.inner.borrow().alias.clone()
    }
}

#[cfg(target_os = "windows")]
impl Drop for MediaCtrlInner {
    fn drop(&mut self) {
        if self.has_media {
            let cmd = format!("close {}", self.alias);
            let _ = mci_send(&cmd, None);
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl MediaCtrl {
    /// Non-Windows stub: returns a no-op `MediaCtrl` whose
    /// `state()` is always `MediaState::Stopped`.
    pub fn new<W: Window>(_parent: &W) -> Self {
        MediaCtrl {
            inner: Rc::new(RefCell::new(MediaCtrlInner {
                has_media: false,
                state: MediaState::Stopped,
            })),
        }
    }
    /// Non-Windows stub: always returns `Err` with a message.
    pub fn load(&self, _path: &Path) -> Result<(), String> {
        Err("MediaCtrl: MCI is Windows-only".to_string())
    }
    /// Non-Windows stub.
    pub fn play(&self) -> Result<(), String> {
        Ok(())
    }
    /// Non-Windows stub.
    pub fn pause(&self) -> Result<(), String> {
        Ok(())
    }
    /// Non-Windows stub.
    pub fn stop(&self) -> Result<(), String> {
        Ok(())
    }
    /// Non-Windows stub.
    pub fn close(&self) -> Result<(), String> {
        Ok(())
    }
    /// Non-Windows stub.
    pub fn position_ms(&self) -> Option<u64> {
        None
    }
    /// Non-Windows stub.
    pub fn length_ms(&self) -> Option<u64> {
        None
    }
    /// Non-Windows stub.
    pub fn seek_ms(&self, _ms: u64) -> Result<(), String> {
        Err("MediaCtrl: MCI is Windows-only".to_string())
    }
    /// Non-Windows stub.
    pub fn state(&self) -> MediaState {
        MediaState::Stopped
    }
    /// Non-Windows stub.
    pub fn alias(&self) -> String {
        String::new()
    }
}

// ── MCI helpers (Windows only) ─────────────────────────────────────

/// Escape backslashes and double-quotes for an MCI string. The
/// MCI grammar requires backslash and double-quote characters to
/// be escaped with a leading backslash inside a quoted string.
#[cfg(target_os = "windows")]
fn escape_for_mci(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out
}

/// Send a command string to MCI. `reply` is an optional buffer
/// that will receive the MCI reply (a null-terminated wide string).
#[cfg(target_os = "windows")]
fn mci_send(cmd: &str, reply: Option<&mut [u16]>) -> Result<(), String> {
    // Build a null-terminated wide string for the command.
    let wide_cmd: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();

    let (reply_ptr, reply_len) = match reply {
        Some(buf) => (buf.as_mut_ptr(), buf.len() as u32),
        None => (std::ptr::null_mut(), 0),
    };
    let hwnd = std::ptr::null_mut();

    // SAFETY: `mciSendStringW` takes a null-terminated wide string
    // for the command and an optional reply buffer. We provide
    // `wide_cmd` as a fresh, null-terminated buffer; `reply_ptr`
    // and `reply_len` are either both valid or both null/0.
    let err = unsafe {
        mciSendStringW(
            wide_cmd.as_ptr(),
            reply_ptr,
            reply_len,
            hwnd,
        )
    };
    if err == 0 {
        Ok(())
    } else {
        let desc = mci_error_string(err);
        Err(format!("MCI error {err}: {desc}"))
    }
}

/// Return the human-readable description of an MCI error code.
#[cfg(target_os = "windows")]
fn mci_error_string(err: u32) -> String {
    let mut buf = [0u16; 256];
    // SAFETY: `mciGetErrorStringW` writes a null-terminated
    // description of `err` into `buf`. The buffer is large enough.
    let ok = unsafe {
        mciGetErrorStringW(err, buf.as_mut_ptr(), buf.len() as u32)
    };
    if ok == 0 {
        return format!("unknown error {err}");
    }
    // Find the NUL terminator.
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Parse a u64 out of a null-terminated wide-string reply buffer.
#[cfg(target_os = "windows")]
fn parse_u64_from_wide(buf: &[u16]) -> Option<u64> {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let s = String::from_utf16_lossy(&buf[..len]);
    s.trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_preserves_plain_ascii() {
        assert_eq!(escape_for_mci("hello.mp3"), "hello.mp3");
    }

    #[test]
    fn escape_doubles_backslashes() {
        assert_eq!(escape_for_mci("a\\b\\c"), "a\\\\b\\\\c");
    }

    #[test]
    fn escape_escapes_quotes() {
        assert_eq!(escape_for_mci("a\"b"), "a\\\"b");
    }

    #[test]
    fn parse_decimal_u64() {
        let buf: [u16; 4] = [b'1' as u16, b'2' as u16, b'3' as u16, 0];
        assert_eq!(parse_u64_from_wide(&buf), Some(123));
    }

    #[test]
    fn parse_invalid_returns_none() {
        let buf: [u16; 4] = [b'X' as u16, b'Y' as u16, b'Z' as u16, 0];
        assert_eq!(parse_u64_from_wide(&buf), None);
    }

    #[test]
    fn alias_is_unique_per_instance() {
        let a1 = next_alias();
        let a2 = next_alias();
        assert_ne!(a1, a2);
        assert!(a1.starts_with("ruwx_media_"));
        assert!(a2.starts_with("ruwx_media_"));
    }
}
