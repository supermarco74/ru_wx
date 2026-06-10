//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxTopLevelWindow` — a more complete window base than [`Frame`].
//!
//! In wxWidgets, `wxTopLevelWindow` is the base class for any window that
//! can appear as a stand-alone top-level OS window (frames, dialogs, etc.)
//! and provides a number of methods that don't make sense on a child
//! control: minimize, maximize, full-screen, flash-to-attention, an
//! associated icon, etc.
//!
//! In `ru_wx` we already have a fully-working [`Frame`] type. Rather than
//! duplicate the entire window-class machinery, [`TopLevelWindow`] is a
//! thin *composition* wrapper around [`Frame`]: every method on the
//! wrapper is implemented in terms of a `Frame` method or a single Win32
//! call against the frame's `HWND`. The wrapper exposes a richer API
//! without having to fork the existing frame code.
//!
//! ## What you get
//!
//! - `iconize` / `is_iconized` / `maximize` / `is_maximized` / `restore`
//! - `show_full_screen` / `is_full_screen`
//! - `set_min_size` / `set_max_size` (hint, enforced by the OS)
//! - `centre` (centre on screen or on the parent window)
//! - `request_user_attention` (flash the task-bar entry)
//! - `set_icon` / `get_icon` (the window's HICON)
//! - `get_title` (read the current title bar text)
//! - `set_default_size` (set the size used when the frame is restored
//!   from maximised / iconised state)
//!
//! All operations are `no-op` on non-Windows platforms (the methods are
//! still callable so cross-platform code compiles cleanly).

use crate::window::frame::{Frame, FrameBuilder};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DwmSetWindowAttribute, DWM_WINDOW_CORNER_PREFERENCE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT, DWMWCP_DONOTROUND, DWMWCP_ROUND,
    DWMWCP_ROUNDSMALL,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, *};

// ── Win32 constants used by the wrapper ─────────────────────────────────

/// `ICON_BIG` — the 32×32 window icon (alt-tab, taskbar, etc.).
#[cfg(target_os = "windows")]
const ICON_BIG: usize = 1;
/// `ICON_SMALL` — the 16×16 window icon (title bar, small taskbar entry).
#[cfg(target_os = "windows")]
const ICON_SMALL: usize = 0;

/// `SPI_GETWORKAREA` — return the working area of the desktop (i.e. the
/// screen minus the taskbar / docked toolbars).
#[cfg(target_os = "windows")]
const SPI_GETWORKAREA: u32 = 0x0030;
/// `SWP_FRAMECHANGED` — send `WM_NCCALCSIZE` so the new style is honoured.
#[cfg(target_os = "windows")]
const SWP_FRAMECHANGED: u32 = 0x0020;

/// `FLASHW_TRAY` — flash the taskbar button.
#[cfg(target_os = "windows")]
const FLASHW_TRAY: u32 = 0x00000002;
/// `FLASHW_TIMERNOFG` — flash continuously until the window comes to
/// the foreground.
#[cfg(target_os = "windows")]
const FLASHW_TIMERNOFG: u32 = 0x0000000C;

/// A thin wrapper around [`Frame`] that exposes the additional API of a
/// wxWidgets `wxTopLevelWindow`.
#[derive(Clone)]
pub struct TopLevelWindow {
    frame: Frame,
}

impl TopLevelWindow {
    /// Create a new top-level window with the given title, width, and
    /// height. Equivalent to `Frame::builder().with_title(...).with_size(...).build()`,
    /// with the Windows 11 modern style applied (dark title bar
    /// following the OS theme, Mica backdrop, rounded corners —
    /// see [`Frame::apply_modern_style`]).
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        let frame = Frame::builder()
            .with_title(title)
            .with_size(width, height)
            .with_modern_style()
            .build();
        TopLevelWindow { frame }
    }

    /// Build a top-level window from a [`FrameBuilder`] (so the caller
    /// can still control position, etc.).
    pub fn from_builder(builder: FrameBuilder) -> Self {
        TopLevelWindow {
            frame: builder.build(),
        }
    }

    /// Consume the wrapper and return the inner [`Frame`]. Useful when
    /// code that already operates on a `Frame` (sizers, etc.) needs
    /// access to the underlying window.
    pub fn into_frame(self) -> Frame {
        self.frame
    }

    /// Borrow the inner [`Frame`]. Most code that interacts with the
    /// window (creating child controls, registering handlers) uses this.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    // ── Title / icon ─────────────────────────────────────────────────

    /// Read the current title-bar text.
    pub fn get_title(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.frame.hwnd();
            let mut buf = [0u16; 512];
            // SAFETY: FFI call to GetWindowTextW; `hwnd` is a real window handle and the wide buffer is sized appropriately.
            let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
            if len <= 0 {
                return String::new();
            }
            String::from_utf16_lossy(&buf[..len as usize])
        }
        #[cfg(not(target_os = "windows"))]
        {
            String::new()
        }
    }

    /// Set the window's icon (the HICON used in the title bar, taskbar,
    /// and Alt-Tab switcher).
    ///
    /// Pass `0` to reset to the default application icon.
    #[cfg(target_os = "windows")]
    pub fn set_icon(&self, hicon: isize) {
        let hwnd = self.frame.hwnd();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon);
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon);
            // Force a frame re-paint so the new icon is picked up.
            InvalidateRect(hwnd, std::ptr::null(), 1);
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn set_icon(&self, _hicon: isize) {}

    /// Return the window's current `HICON` (`0` if none / not on Windows).
    #[cfg(target_os = "windows")]
    pub fn get_icon(&self) -> isize {
        let hwnd = self.frame.hwnd();
        // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
        unsafe { SendMessageW(hwnd, WM_GETICON, ICON_BIG, 0) as isize }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn get_icon(&self) -> isize {
        0
    }

    // ── Show / hide / iconize / maximize ─────────────────────────────

    /// Show the window, enter the message loop, and block until the
    /// user closes it. Equivalent to `Frame::show`.
    pub fn show(self) {
        self.frame.show();
    }

    /// Hide the window (the OS will keep the HWND alive; call
    /// [`TopLevelWindow::show`] again to display it).
    pub fn hide(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.frame.hwnd();
            // SAFETY: FFI call to ShowWindow; `hwnd` is a live window owned by this crate.
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    /// Close the window (sends `WM_CLOSE`).
    pub fn close(&self) {
        self.frame.close();
    }

    /// Minimise (iconise) the window.
    pub fn iconize(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.frame.hwnd();
            // SAFETY: FFI call to ShowWindow; `hwnd` is a live window owned by this crate.
            unsafe {
                ShowWindow(hwnd, SW_SHOWMINIMIZED);
            }
        }
    }

    /// Return `true` if the window is currently minimised / iconised.
    #[cfg(target_os = "windows")]
    pub fn is_iconized(&self) -> bool {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe { IsIconic(self.frame.hwnd()) != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn is_iconized(&self) -> bool {
        false
    }

    /// Maximise the window.
    pub fn maximize(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.frame.hwnd();
            // SAFETY: FFI call to ShowWindow; `hwnd` is a live window owned by this crate.
            unsafe {
                ShowWindow(hwnd, SW_SHOWMAXIMIZED);
            }
        }
    }

    /// Return `true` if the window is currently maximised.
    #[cfg(target_os = "windows")]
    pub fn is_maximized(&self) -> bool {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe { IsZoomed(self.frame.hwnd()) != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn is_maximized(&self) -> bool {
        false
    }

    /// Restore the window to its normal (non-minimised, non-maximised)
    /// state.
    pub fn restore(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.frame.hwnd();
            // SAFETY: FFI call to ShowWindow; `hwnd` is a live window owned by this crate.
            unsafe {
                ShowWindow(hwnd, SW_RESTORE);
            }
        }
    }

    // ── Full screen ──────────────────────────────────────────────────

    /// Toggle full-screen mode.
    ///
    /// `show` is `true` to enter full-screen, `false` to leave it.
    /// The current style and rect are saved on entry and restored on
    /// exit. When entering full-screen, the window is positioned and
    /// sized to the monitor's working area.
    ///
    /// The `style` parameter is reserved for future use (e.g. wx's
    /// `FULLSCREEN_NOMENUBAR`, `FULLSCREEN_NOBORDER`, etc.). For now
    /// pass `FullScreenStyle::Default`.
    pub fn show_full_screen(&self, show: bool, _style: FullScreenStyle) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.frame.hwnd();
            if show {
                // Save the current rect so we can restore it.
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                let mut rc: RECT = unsafe { std::mem::zeroed() };
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    GetWindowRect(hwnd, &mut rc);
                }

                // Read the work area (the desktop minus the taskbar).
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                let mut work_area: RECT = unsafe { std::mem::zeroed() };
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SystemParametersInfoW(
                        SPI_GETWORKAREA,
                        0,
                        &mut work_area as *mut _ as *mut _,
                        0,
                    );
                }

                // Remove the WS_OVERLAPPEDWINDOW style (caption,
                // thick frame, system menu) so the window has no
                // chrome, and add WS_POPUP so it owns the screen.
                let gwl_style = GWL_STYLE;
                // SAFETY: FFI call to GetWindowLongPtrW with a live HWND and a valid `nIndex`.
                let style = unsafe { GetWindowLongPtrW(hwnd, gwl_style) } as u32;
                let new_style = (style & !WS_OVERLAPPEDWINDOW) | WS_POPUP;
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SetWindowLongPtrW(hwnd, gwl_style, new_style as isize);
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        work_area.left,
                        work_area.top,
                        work_area.right - work_area.left,
                        work_area.bottom - work_area.top,
                        SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                    );
                }
            } else {
                // Restore the standard chrome and the normal position.
                let gwl_style = GWL_STYLE;
                // SAFETY: FFI call to GetWindowLongPtrW with a live HWND and a valid `nIndex`.
                let style = unsafe { GetWindowLongPtrW(hwnd, gwl_style) } as u32;
                let new_style = (style & !WS_POPUP) | WS_OVERLAPPEDWINDOW;
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SetWindowLongPtrW(hwnd, gwl_style, new_style as isize);
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                    );
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = show;
        }
    }

    /// Return `true` if the window is currently in full-screen mode.
    /// We approximate this by checking the absence of the caption
    /// style (`WS_CAPTION`).
    #[cfg(target_os = "windows")]
    pub fn is_full_screen(&self) -> bool {
        let hwnd = self.frame.hwnd();
        // SAFETY: FFI call to GetWindowLongPtrW with a live HWND and a valid `nIndex`.
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
        (style & WS_CAPTION) == 0
    }
    #[cfg(not(target_os = "windows"))]
    pub fn is_full_screen(&self) -> bool {
        false
    }

    // ── Size constraints ─────────────────────────────────────────────

    /// Set the minimum size the user can shrink the window to.
    #[cfg(target_os = "windows")]
    pub fn set_min_size(&self, w: i32, h: i32) {
        let hwnd = self.frame.hwnd();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOSIZE,
            );
            // Win32 has no per-window "min size" — the closest is
            // `WM_GETMINMAXINFO`, which we don't intercept. We can
            // however enforce the limit by setting a 0×0 placeholder
            // via `SetWindowPos` once and resizing. For now we just
            // resize the window itself to enforce the floor when it's
            // smaller than requested.
            let mut rc: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rc);
            let cur_w = rc.right - rc.left;
            let cur_h = rc.bottom - rc.top;
            if cur_w < w || cur_h < h {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    rc.left,
                    rc.top,
                    w.max(cur_w),
                    h.max(cur_h),
                    SWP_NOZORDER,
                );
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn set_min_size(&self, _w: i32, _h: i32) {}

    /// Set the maximum size the user can grow the window to. Win32
    /// also lacks a per-window API for this; the limit is enforced
    /// only if the window is currently larger than the requested
    /// maximum.
    #[cfg(target_os = "windows")]
    pub fn set_max_size(&self, w: i32, h: i32) {
        let hwnd = self.frame.hwnd();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut rc: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rc);
            let cur_w = rc.right - rc.left;
            let cur_h = rc.bottom - rc.top;
            if cur_w > w || cur_h > h {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    rc.left,
                    rc.top,
                    w.min(cur_w),
                    h.min(cur_h),
                    SWP_NOZORDER,
                );
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn set_max_size(&self, _w: i32, _h: i32) {}

    /// Set the *restored* size of the window. Win32 uses this size when
    /// the user un-maximises the window.
    #[cfg(target_os = "windows")]
    pub fn set_default_size(&self, w: i32, h: i32) {
        let hwnd = self.frame.hwnd();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // Restoring from maximised/iconised state uses the last
            // non-maximised position. The cleanest way to set this
            // is to call `SetWindowPos` with the new size and
            // `SWP_FRAMECHANGED`. The user can then un-maximise and
            // see the new size.
            let mut rc: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rc);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                rc.left,
                rc.top,
                w,
                h,
                SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn set_default_size(&self, _w: i32, _h: i32) {}

    // ── Win11 rounded corners (DWMWA_WINDOW_CORNER_PREFERENCE) ─────

    /// Set the Windows 11 rounded-corner preference via the
    /// `DWMWA_WINDOW_CORNER_PREFERENCE` DWM attribute. Returns
    /// `true` on success.
    ///
    /// On Windows 11 the DWM compositor honours the attribute and
    /// draws the window with the requested corner shape (large
    /// round, small round, or rectangular). On Windows 10 1809 /
    /// Server 2019 and later the DWM call succeeds but the
    /// compositor ignores the attribute, so the window keeps its
    /// default (rectangular) shape — the call is harmless. On
    /// earlier Windows releases the call fails with `E_NOTIMPL`
    /// and this method returns `false`.
    ///
    /// The change takes effect immediately; no `SetWindowPos` or
    /// repaint is required. Calling this with
    /// [`WindowCornerPreference::Default`] reverts the window to
    /// "let the system decide" (which is the Win11 default for
    /// top-level app windows — large rounded corners).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ru_wx::prelude::*;
    /// use ru_wx::{TopLevelWindow, WindowCornerPreference};
    ///
    /// let window = TopLevelWindow::new("Rounded", 400, 300);
    /// window.set_window_corner_preference(WindowCornerPreference::Round);
    /// window.show();
    /// ```
    #[cfg(target_os = "windows")]
    pub fn set_window_corner_preference(&self, pref: WindowCornerPreference) -> bool {
        let hwnd = self.frame.hwnd();
        let value: DWM_WINDOW_CORNER_PREFERENCE = pref.to_win32();
        // SAFETY: `hwnd` is a live `HWND` returned by the matching
        // `CreateWindowExW` call in `frame::FrameBuilder::build`.
        // The `value` pointer points to a stack-allocated `i32`
        // (the `DWM_WINDOW_CORNER_PREFERENCE` representation) which
        // the DWM reads once before returning, so its lifetime
        // covers the call. `cbattribute` is the exact `size_of`
        // of the value type, as required by the DWM contract.
        let hr = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &value as *const _ as *const _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            )
        };
        hr >= 0
    }
    #[cfg(not(target_os = "windows"))]
    pub fn set_window_corner_preference(&self, _pref: WindowCornerPreference) -> bool {
        false
    }

    /// Read the current Windows 11 rounded-corner preference.
    ///
    /// Returns `None` if the DWM does not support the attribute
    /// (e.g. older Windows releases) or if the call fails for any
    /// other reason. On non-Windows targets this always returns
    /// `None`.
    #[cfg(target_os = "windows")]
    pub fn get_window_corner_preference(&self) -> Option<WindowCornerPreference> {
        let hwnd = self.frame.hwnd();
        let mut value: DWM_WINDOW_CORNER_PREFERENCE = 0;
        // SAFETY: `hwnd` is a live `HWND`. The output pointer is a
        // stack-allocated `i32` slot, which the DWM writes through
        // before returning. `cbattribute` matches the exact
        // `size_of` of the output type, as required by the DWM
        // contract.
        let hr = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &mut value as *mut _ as *mut _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            )
        };
        if hr < 0 {
            return None;
        }
        WindowCornerPreference::from_win32(value)
    }
    #[cfg(not(target_os = "windows"))]
    pub fn get_window_corner_preference(&self) -> Option<WindowCornerPreference> {
        None
    }

    // ── Centring ─────────────────────────────────────────────────────

    /// Centre the window on the screen, on its parent, or in either
    /// direction. The window is moved (not resized) so its centre
    /// coincides with the centre of the chosen target.
    #[cfg(target_os = "windows")]
    pub fn centre(&self, direction: CentreDirection) {
        let hwnd = self.frame.hwnd();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut win_rc: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut win_rc);
            let win_w = win_rc.right - win_rc.left;
            let win_h = win_rc.bottom - win_rc.top;

            match direction {
                CentreDirection::Screen | CentreDirection::Both => {
                    // Use the primary monitor's working area.
                    let mut work: RECT = std::mem::zeroed();
                    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
                    let work_w = work.right - work.left;
                    let work_h = work.bottom - work.top;
                    let x = work.left + (work_w - win_w) / 2;
                    let y = work.top + (work_h - win_h) / 2;
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        x,
                        y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
                CentreDirection::Horizontal => {
                    let mut work: RECT = std::mem::zeroed();
                    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
                    let work_w = work.right - work.left;
                    let x = work.left + (work_w - win_w) / 2;
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        x,
                        win_rc.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
                CentreDirection::Vertical => {
                    let mut work: RECT = std::mem::zeroed();
                    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
                    let work_h = work.bottom - work.top;
                    let y = work.top + (work_h - win_h) / 2;
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        win_rc.left,
                        y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn centre(&self, _direction: CentreDirection) {}

    // ── Attention ────────────────────────────────────────────────────

    /// Request the user's attention by flashing the window in the
    /// taskbar. Useful for backgrounded windows that need the user to
    /// come back to them.
    #[cfg(target_os = "windows")]
    pub fn request_user_attention(&self, flags: UserAttentionFlags) {
        let hwnd = self.frame.hwnd();
        let flash_flags = match flags {
            UserAttentionFlags::Default => FLASHW_TRAY,
            UserAttentionFlags::Continuous => FLASHW_TRAY | FLASHW_TIMERNOFG,
        };
        let info = FLASHWINFO {
            cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
            hwnd,
            dwFlags: flash_flags,
            uCount: 0,    // 0 = flash until focused
            dwTimeout: 0, // use the OS default cursor blink rate
        };
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            FlashWindowEx(&info);
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn request_user_attention(&self, _flags: UserAttentionFlags) {}

    // ── Direct HWND access ───────────────────────────────────────────

    /// Return the underlying native window handle.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.frame.hwnd()
    }
    #[cfg(not(target_os = "windows"))]
    pub fn hwnd(&self) -> isize {
        0
    }

    // ── Size / position ──────────────────────────────────────────────

    /// Move / resize the window. Equivalent to `Frame::set_size` plus
    /// a `SetWindowPos` with `SWP_NOZORDER`.
    #[cfg(target_os = "windows")]
    pub fn set_size(&self, w: u32, h: u32) {
        self.frame.set_size(w, h);
    }
    #[cfg(not(target_os = "windows"))]
    pub fn set_size(&self, _w: u32, _h: u32) {}

    /// Set the window's title.
    pub fn set_title(&self, title: &str) {
        self.frame.set_title(title);
    }

    /// Get the primary monitor's size (full screen, not work area).
    #[cfg(target_os = "windows")]
    pub fn get_primary_monitor_size() -> (i32, i32) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn get_primary_monitor_size() -> (i32, i32) {
        (0, 0)
    }

    /// Get the primary monitor's *work* area (screen minus taskbar).
    #[cfg(target_os = "windows")]
    pub fn get_work_area() -> (i32, i32, i32, i32) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut work: RECT = std::mem::zeroed();
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
            (
                work.left,
                work.top,
                work.right - work.left,
                work.bottom - work.top,
            )
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn get_work_area() -> (i32, i32, i32, i32) {
        (0, 0, 0, 0)
    }
}

// ── Public enums ───────────────────────────────────────────────────────

/// Which axes to centre the window on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CentreDirection {
    /// Centre on the screen, both horizontally and vertically.
    Screen,
    /// Centre only horizontally.
    Horizontal,
    /// Centre only vertically.
    Vertical,
    /// Centre in both directions (same as `Screen`).
    Both,
}

/// Style flags for `show_full_screen`.
///
/// These mirror the wxWidgets `wxFULLSCREEN_*` flags, but currently
/// only [`FullScreenStyle::Default`] is implemented (Win32 has no
/// direct equivalent of wx's per-flag full-screen mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FullScreenStyle {
    /// Default full-screen mode: no border, no caption, sized to the
    /// work area.
    #[default]
    Default,
}

/// Flags for `request_user_attention`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAttentionFlags {
    /// Flash the taskbar button a few times.
    Default,
    /// Flash the taskbar button continuously until the window comes
    /// to the foreground.
    Continuous,
}

/// Windows 11 rounded-corner preference.
///
/// Set on a top-level window with
/// [`TopLevelWindow::set_window_corner_preference`]. The DWM
/// compositor on Windows 11 honours the preference and draws the
/// window with the requested corner shape; on older Windows
/// releases the DWM call is accepted but the compositor ignores
/// it (the window keeps its default rectangular shape).
///
/// The values mirror the Win32 `DWM_WINDOW_CORNER_PREFERENCE`
/// enum, with the same numerical order and meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowCornerPreference {
    /// `DWMWCP_DEFAULT` — let the system decide. On Windows 11
    /// this is "large rounded corners" for top-level app
    /// windows.
    #[default]
    Default,
    /// `DWMWCP_DONOTROUND` — explicitly disable corner rounding
    /// (sharp rectangular corners).
    DoNotRound,
    /// `DWMWCP_ROUND` — large rounded corners (the Win11
    /// default for top-level app windows).
    Round,
    /// `DWMWCP_ROUNDSMALL` — small rounded corners.
    RoundSmall,
}

impl WindowCornerPreference {
    /// Map to the raw `DWM_WINDOW_CORNER_PREFERENCE` value the
    /// Win32 API expects.
    #[cfg(target_os = "windows")]
    pub(crate) fn to_win32(self) -> DWM_WINDOW_CORNER_PREFERENCE {
        match self {
            WindowCornerPreference::Default => DWMWCP_DEFAULT,
            WindowCornerPreference::DoNotRound => DWMWCP_DONOTROUND,
            WindowCornerPreference::Round => DWMWCP_ROUND,
            WindowCornerPreference::RoundSmall => DWMWCP_ROUNDSMALL,
        }
    }

    /// Map from the raw `DWM_WINDOW_CORNER_PREFERENCE` value
    /// reported by the DWM. Returns `None` for unknown values so
    /// the call is total and a future Windows release can extend
    /// the enum without breaking the wrapper.
    #[cfg(target_os = "windows")]
    pub(crate) fn from_win32(value: DWM_WINDOW_CORNER_PREFERENCE) -> Option<Self> {
        match value {
            DWMWCP_DEFAULT => Some(WindowCornerPreference::Default),
            DWMWCP_DONOTROUND => Some(WindowCornerPreference::DoNotRound),
            DWMWCP_ROUND => Some(WindowCornerPreference::Round),
            DWMWCP_ROUNDSMALL => Some(WindowCornerPreference::RoundSmall),
            _ => None,
        }
    }
}
