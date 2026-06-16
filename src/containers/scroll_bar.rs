//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Scrollbar control (`wxScrollBar`).
//!
//! On Windows this is realised with the standard Win32 `SCROLLBAR`
//! control class (styles `SBS_HORZ` / `SBS_VERT`). The control is
//! kept simple: it does not draw its own border, has no built-in
//! arrow keys, and reports the full set of `SB_*` request codes
//! through the parent's `WM_HSCROLL` / `WM_VSCROLL` handler.
//!
//! The way the OS routes those notifications is the reason this
//! module's [`ScrollBar::on_scroll`] registration goes through
//! [`Frame::register_scroll_handler`] (which the `frame_wnd_proc`
//! looks up by the scroll bar's `HWND` when it receives a
//! `WM_HSCROLL` / `WM_VSCROLL`). The wrapper converts the
//! raw `(u16, i32)` payload (low word of `wparam` = SB_* request
//! code, high word = thumb position) into a typed
//! [`ScrollEvent`] for the user callback.
//!
//! Range and position use the legacy 16-bit-friendly API
//! (`SBM_SETRANGEREDRAW` / `SBM_SETPOS`). For 32-bit clean ranges
//! (e.g. negative bounds or values > 32767) the underlying
//! `SCROLLINFO` API is needed; the current implementation clamps
//! the range to a 16-bit-wide window via the standard
//! `MAKELONG(min, max)` packing. This matches the typical use of
//! scroll bars (0..100, 0..1000, etc.) and avoids the surface
//! area of the full `SCROLLINFO` struct.

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 scroll bar constants ─────────────────────────────────────────

// Scroll-bar messages (SBM_*). Defined locally because the
// `windows-sys 0.59` surface does not include them in
// `Win32_UI_WindowsAndMessaging`. The numeric values are
// stable across all Win32 SDKs (commctrl.h §Scroll Bar Messages).
#[cfg(target_os = "windows")]
const SBM_SETPOS: u32 = 0x00E0;
#[cfg(target_os = "windows")]
const SBM_GETPOS: u32 = 0x00E1;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SBM_SETRANGE: u32 = 0x00E2;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SBM_GETRANGE: u32 = 0x00E3;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SBM_ENABLE_ARROWS: u32 = 0x00E4;
#[cfg(target_os = "windows")]
const SBM_SETPAGESIZE: u32 = 0x00E5;
#[cfg(target_os = "windows")]
const SBM_SETRANGEREDRAW: u32 = 0x00E6;

// Scroll-bar request codes (SB_*). These are the values that
// arrive in the low word of `wparam` of `WM_HSCROLL` /
// `WM_VSCROLL`. They are stable across all Win32 SDKs.
#[cfg(target_os = "windows")]
const SB_LINEUP: u32 = 0;
#[cfg(target_os = "windows")]
const SB_LINEDOWN: u32 = 1;
#[cfg(target_os = "windows")]
const SB_PAGEUP: u32 = 2;
#[cfg(target_os = "windows")]
const SB_PAGEDOWN: u32 = 3;
#[cfg(target_os = "windows")]
const SB_THUMBPOSITION: u32 = 4;
#[cfg(target_os = "windows")]
const SB_THUMBTRACK: u32 = 5;
#[cfg(target_os = "windows")]
const SB_TOP: u32 = 6;
#[cfg(target_os = "windows")]
const SB_BOTTOM: u32 = 7;
#[cfg(target_os = "windows")]
const SB_ENDSCROLL: u32 = 8;

// Scroll-bar styles (SBS_*). These are the window-style flags
// passed to `CreateWindowExW` to select horizontal vs. vertical
// orientation.
#[cfg(target_os = "windows")]
const SBS_HORZ: u32 = 0x0000;
#[cfg(target_os = "windows")]
const SBS_VERT: u32 = 0x0001;

// ── Public types ──────────────────────────────────────────────────────

/// Orientation of a scroll bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollBarOrientation {
    /// Horizontal scroll bar (`SBS_HORZ`, drawn left-to-right).
    Horizontal,
    /// Vertical scroll bar (`SBS_VERT`, drawn top-to-bottom).
    Vertical,
}

/// Strongly-typed scroll event delivered to user callbacks.
///
/// The variant names mirror the Win32 `SB_*` request codes; the
/// position payload (where applicable) is wrapped in
/// [`ScrollEvent::ThumbTrack`] / [`ScrollEvent::ThumbRelease`]
/// to make the type-safe signature self-documenting. The mapping
/// is the same for horizontal and vertical scroll bars — the
/// `SB_LINELEFT` / `SB_LINERIGHT` / `SB_PAGELEFT` /
/// `SB_PAGERIGHT` / `SB_LEFT` / `SB_RIGHT` aliases collapse
/// into the same four variants on the user side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollEvent {
    /// `SB_LINEUP` / `SB_LINELEFT` — user pressed the up (or
    /// left) arrow button.
    LineUp,
    /// `SB_LINEDOWN` / `SB_LINERIGHT` — user pressed the down
    /// (or right) arrow button.
    LineDown,
    /// `SB_PAGEUP` / `SB_PAGELEFT` — user clicked the page-up
    /// (or page-left) area, or pressed the `Page Up` (or
    /// `Page Left`) key.
    PageUp,
    /// `SB_PAGEDOWN` / `SB_PAGERIGHT` — user clicked the
    /// page-down (or page-right) area, or pressed `Page Down`.
    PageDown,
    /// `SB_THUMBPOSITION` — user released the thumb. `position`
    /// is the new value the thumb was dropped at. This is the
    /// variant the typical "the user picked a new scroll
    /// position" callback should react to.
    ThumbRelease {
        /// The new thumb position.
        position: i32,
    },
    /// `SB_THUMBTRACK` — user is dragging the thumb;
    /// `position` is the current in-flight value. Fires
    /// repeatedly while the drag is in progress, so callbacks
    /// that do expensive work (e.g. repainting a viewport)
    /// should typically debounce or use [`ScrollEvent::ThumbRelease`]
    /// instead.
    ThumbTrack {
        /// The current thumb position.
        position: i32,
    },
    /// `SB_TOP` / `SB_LEFT` — user pressed `Ctrl+Home` (or the
    /// keyboard equivalent for "scroll to start").
    Top,
    /// `SB_BOTTOM` / `SB_RIGHT` — user pressed `Ctrl+End`.
    Bottom,
    /// `SB_ENDSCROLL` — last event in a scroll sequence. The
    /// `Frame` delivers this once after the user finishes
    /// dragging the thumb or after each individual button /
    /// page / line scroll.
    EndScroll,
}

// ── Inner type ────────────────────────────────────────────────────────

struct ScrollBarInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    scroll_frame: Option<Frame>,
    /// The control id assigned at construction (via
    /// [`next_control_id`]). Currently exposed through
    /// [`ScrollBar::id`] for symmetry with the other
    /// widget types; the frame's `WM_HSCROLL` / `WM_VSCROLL`
    /// dispatch keys on the control's `HWND`, not the id.
    id: u16,
    rect: Rect,
    visible: bool,
    enabled: bool,
    min: i32,
    max: i32,
    position: i32,
    page_size: i32,
    orientation: ScrollBarOrientation,
}

#[derive(Clone)]
pub struct ScrollBar {
    inner: Rc<RefCell<ScrollBarInner>>,
}

impl ScrollBar {
    /// Create a new scroll bar with the default range `0..100`
    /// and a page size of `10`. Use [`ScrollBar::new_full`] for
    /// full control over the initial range and page size.
    pub fn new<W: Window>(parent: &W, orientation: ScrollBarOrientation) -> Self {
        Self::new_full(parent, orientation, 0, 100, 10)
    }

    /// Create a new scroll bar with explicit range and page
    /// size. The thumb is placed at `min` on creation; use
    /// [`ScrollBar::set_position`] to move it.
    pub fn new_full<W: Window>(
        parent: &W,
        orientation: ScrollBarOrientation,
        min: i32,
        max: i32,
        page_size: i32,
    ) -> Self {
        let id = next_control_id();
        let (default_w, default_h) = match orientation {
            ScrollBarOrientation::Horizontal => (200u32, 16u32),
            ScrollBarOrientation::Vertical => (16u32, 200u32),
        };

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent_hwnd = parent.hwnd();
            let wide_class = to_wide("SCROLLBAR");
            let mut style = WS_CHILD | WS_VISIBLE;
            style |= match orientation {
                ScrollBarOrientation::Horizontal => SBS_HORZ,
                ScrollBarOrientation::Vertical => SBS_VERT,
            };
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                style,
                0,
                0,
                default_w as i32,
                default_h as i32,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent, orientation);

        let sb = ScrollBar {
            inner: Rc::new(RefCell::new(ScrollBarInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "windows")]
                scroll_frame: None,
                id,
                rect: Rect::new(0, 0, default_w, default_h),
                visible: true,
                enabled: true,
                min,
                max,
                position: min,
                page_size,
                orientation,
            })),
        };
        sb.set_range(min, max);
        sb.set_page_size(page_size);
        sb
    }

    /// Set the scroll bar's range. The thumb position is
    /// clamped to the new range.
    ///
    /// Internally uses `SBM_SETRANGEREDRAW`, which packs
    /// `(min, max)` into the 32-bit `lparam` with `min` in the
    /// low word and `max` in the high word. This is fine for
    /// the common case of 16-bit-friendly ranges (0..65535).
    /// For ranges that need the full 32 bits, the
    /// `SCROLLINFO` API would be required — not currently
    /// exposed by this module.
    pub fn set_range(&self, min: i32, max: i32) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.min = min;
            inner.max = max;
            if inner.position < min {
                inner.position = min;
            } else if inner.position > max {
                inner.position = max;
            }
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // SBM_SETRANGEREDRAW: wparam = 0 (unused), lparam = MAKELONG(min, max)
            let lparam = ((max as u32) << 16) | ((min as u32) & 0xFFFF);
            SendMessageW(
                self.inner.borrow().hwnd,
                SBM_SETRANGEREDRAW,
                0,
                lparam as isize,
            );
        }
    }

    /// Return the current range as `(min, max)`. Reads the
    /// cached values set by the most recent
    /// [`ScrollBar::set_range`] call — the underlying Win32
    /// control does not have a synchronous getter for the
    /// range set via the legacy `SBM_SETRANGE*` messages, and
    /// going through `GetScrollInfo` is a larger surface than
    /// this module needs.
    pub fn get_range(&self) -> (i32, i32) {
        let inner = self.inner.borrow();
        (inner.min, inner.max)
    }

    /// Set the current thumb position. The value is clamped
    /// to the current range, and the change is applied with
    /// `SBM_SETPOS` (`wparam = 1` so the control repaints).
    pub fn set_position(&self, pos: i32) {
        let pos = {
            let mut inner = self.inner.borrow_mut();
            let p = pos.max(inner.min).min(inner.max);
            inner.position = p;
            p
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                SBM_SETPOS,
                1,
                pos as isize,
            );
        }
    }

    /// Get the current thumb position. The value is read live
    /// from the control via `SBM_GETPOS` and cached in the
    /// `ScrollBarInner` for subsequent `get_position` calls.
    pub fn get_position(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live scroll bar window and `msg` / `wParam` / `lParam` are valid for that window.
            let v = unsafe { SendMessageW(hwnd, SBM_GETPOS, 0, 0) } as i32;
            self.inner.borrow_mut().position = v;
            v
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().position
        }
    }

    /// Set the page size (the "large step" used by
    /// `SB_PAGEDOWN` / `SB_PAGEUP`).
    pub fn set_page_size(&self, size: i32) {
        self.inner.borrow_mut().page_size = size;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                SBM_SETPAGESIZE,
                0,
                size as isize,
            );
        }
    }

    /// Get the page size previously set by
    /// [`ScrollBar::set_page_size`] (or the initial value
    /// passed to [`ScrollBar::new_full`]).
    pub fn get_page_size(&self) -> i32 {
        self.inner.borrow().page_size
    }

    /// Get the orientation of this scroll bar.
    pub fn orientation(&self) -> ScrollBarOrientation {
        self.inner.borrow().orientation
    }

    /// The control's id. Scroll bars are children of the
    /// frame, so they get a child control id assigned by
    /// [`next_control_id`]. The id is not currently used by
    /// the scroll-bar's own message dispatch (it is the
    /// `HWND` that the frame's `WM_HSCROLL` / `WM_VSCROLL`
    /// arm keys on), but it is exposed for symmetry with
    /// other widgets.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Register a callback fired on every scroll event (line /
    /// page / thumb / top / bottom / end-scroll). The
    /// callback receives a [`ScrollEvent`] describing the
    /// request, with the thumb position already extracted
    /// from the high word of `wparam` for the
    /// `ThumbTrack` / `ThumbRelease` variants.
    ///
    /// Internally wraps the typed `FnMut(ScrollEvent)`
    /// callback into the generic `FnMut(u16, i32)` signature
    /// expected by [`Frame::register_scroll_handler`], so
    /// `frame.rs` does not need to know about
    /// [`ScrollEvent`]. The conversion is the reverse of
    /// `SB_*` → variant: see the `match code` block below
    /// for the exact mapping.
    ///
    /// The same callback can be registered multiple times
    /// for the same scroll bar — the underlying
    /// `Frame::register_scroll_handler` uses the `HWND` as
    /// the key, so the most recent registration wins. To
    /// chain multiple callbacks, wrap them in a single
    /// closure.
    pub fn on_scroll<F: FnMut(ScrollEvent) + 'static>(
        &self,
        frame: &Frame,
        mut callback: F,
    ) {
        #[cfg(target_os = "windows")]
        let hwnd = self.inner.borrow().hwnd;
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().scroll_frame = Some(frame.clone());
            let inner = self.inner.clone();
            let wrapper = move |code: u16, pos: i32| {
                // Sync the cached position with the control's actual
                // position. SB_THUMBPOSITION / SB_THUMBTRACK carry
                // the position in `wparam`'s high word, but reading
                // the live value via SBM_GETPOS is the most
                // robust way to keep the cache in sync across all
                // event types (line/page scroll events also update
                // the position).
                {
                    let hwnd = inner.borrow().hwnd;
                    // SAFETY: FFI call to SendMessageW; `hwnd` is a live scroll bar window and the message / params are valid.
                    let v = unsafe { SendMessageW(hwnd, SBM_GETPOS, 0, 0) } as i32;
                    inner.borrow_mut().position = v;
                }
                let ev = match code as u32 {
                    SB_LINEUP => ScrollEvent::LineUp,
                    SB_LINEDOWN => ScrollEvent::LineDown,
                    SB_PAGEUP => ScrollEvent::PageUp,
                    SB_PAGEDOWN => ScrollEvent::PageDown,
                    SB_THUMBPOSITION => ScrollEvent::ThumbRelease { position: pos },
                    SB_THUMBTRACK => ScrollEvent::ThumbTrack { position: pos },
                    SB_TOP => ScrollEvent::Top,
                    SB_BOTTOM => ScrollEvent::Bottom,
                    SB_ENDSCROLL => ScrollEvent::EndScroll,
                    _ => return, // unknown SB_* code — ignore
                };
                callback(ev);
            };
            frame.register_scroll_handler(hwnd as isize, wrapper);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (frame, callback);
        }
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

#[cfg(target_os = "windows")]
impl Drop for ScrollBar {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) == 1 {
            let inner = self.inner.borrow();
            if let Some(ref frame) = inner.scroll_frame {
                frame.unregister_scroll_handler(inner.hwnd as isize);
            }
        }
    }
}

impl Widget for ScrollBarInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                self.rect.x,
                self.rect.y,
                w as i32,
                h as i32,
                1,
            );
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    //! Unit tests for the platform-agnostic part of the public
    //! surface (no Win32 window creation needed).
    use super::*;

    #[test]
    fn line_up_is_a_distinct_variant() {
        assert_ne!(ScrollEvent::LineUp, ScrollEvent::LineDown);
        assert_ne!(ScrollEvent::LineUp, ScrollEvent::PageUp);
    }

    #[test]
    fn thumb_release_carries_position() {
        let ev = ScrollEvent::ThumbRelease { position: 42 };
        // `assert!(matches!(...))` gives a structured failure
        // message; the previous `match ... { _ => panic!(...) }`
        // pattern emitted a hard-coded string with no payload.
        if let ScrollEvent::ThumbRelease { position } = ev {
            assert_eq!(position, 42);
        } else {
            panic!("expected ThumbRelease, got a different variant");
        }
    }

    #[test]
    fn thumb_track_carries_position() {
        let ev = ScrollEvent::ThumbTrack { position: -7 };
        if let ScrollEvent::ThumbTrack { position } = ev {
            assert_eq!(position, -7);
        } else {
            panic!("expected ThumbTrack, got a different variant");
        }
    }

    #[test]
    fn orientation_distinct_variants() {
        assert_ne!(
            ScrollBarOrientation::Horizontal,
            ScrollBarOrientation::Vertical
        );
        assert_eq!(ScrollBarOrientation::Horizontal as u32, 0);
        assert_eq!(ScrollBarOrientation::Vertical as u32, 1);
    }

    #[test]
    fn end_scroll_is_a_distinct_variant() {
        assert_ne!(ScrollEvent::EndScroll, ScrollEvent::LineUp);
    }
}
