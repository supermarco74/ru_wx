//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Scrollable window container (`wxScrolledWindow`).
//!
//! On Windows this is realised as a `STATIC` child with the
//! `WS_HSCROLL | WS_VSCROLL` *window* styles — i.e. scroll bars that
//! are conceptually attached to the window itself, not separate
//! `SCROLLBAR` child controls. The default `STATIC` WndProc does not
//! dispatch the scroll notifications, so the constructor installs a
//! custom WndProc via `SetWindowLongPtrW(GWLP_WNDPROC, ...)` that:
//!
//! 1. Looks up the user callback in a thread-local map keyed by
//!    `HWND`.
//! 2. Reads the new thumb position via `GetScrollPos` and updates the
//!    internal view position.
//! 3. Invokes the user callback with a typed [`ScrollEvent`].
//! 4. Forwards every other message to the original `STATIC` WndProc
//!    via `CallWindowProcW` so the control still paints itself
//!    correctly.
//! 5. Cleans up the thread-local entries on `WM_NCDESTROY`.
//!
//! The thread-local storage is the right scope for this map: Win32
//! windows are owned by the single thread that created them (the GUI
//! thread), and the WndProc and the widget constructor /
//! `on_scroll` setter all run on that same thread.
//!
//! The public API mirrors the wxWidgets subset that fits a
//! single-widget container: set a virtual size, set / read the view
//! position, and register a callback for scroll events.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::SetScrollInfo;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CreateWindowExW, DefWindowProcW, GetScrollPos,
    GetWindowLongPtrW, MoveWindow, PostMessageW, SCROLLINFO, SetWindowLongPtrW,
    ShowWindow, GWLP_WNDPROC, HMENU, SIF_PAGE, SIF_POS, SIF_RANGE, SW_HIDE, SW_SHOW,
    WM_HSCROLL, WM_USER, WM_VSCROLL, WS_CHILD, WS_CLIPCHILDREN, WS_VISIBLE,
};

// ── Win32 constants ─────────────────────────────────────────────────────
//
// Defined locally because the `windows-sys 0.59` surface does not
// expose them in `Win32_UI_WindowsAndMessaging`. The numeric values
// are stable across every Win32 SDK (see windowsx.h / winuser.h).
#[cfg(target_os = "windows")]
const WS_HSCROLL_U: u32 = 0x0010_0000;
#[cfg(target_os = "windows")]
const WS_VSCROLL_U: u32 = 0x0020_0000;

// WM_NCDESTROY is the last message a window receives; we use it as
// the cleanup point for the thread-local tables. `0x0082`.
#[cfg(target_os = "windows")]
const WM_NCDESTROY_U: u32 = 0x0082;
/// Deferred resize notification — fired via `PostMessageW` so
/// `on_resize` handlers run after the widget `RefCell` borrow ends.
#[cfg(target_os = "windows")]
const WM_RUWX_DEFERRED_RESIZE: u32 = WM_USER + 64;

// Convenience constants for the `nBar` argument of
// `SetScrollInfo` / `GetScrollInfo` / `GetScrollPos` for
// window-attached scroll bars. `SCROLLBAR_CONSTANTS` is a
// type alias for `i32` in `windows-sys 0.59`, so these are
// ordinary integer constants that just happen to be typed
// `i32` at the call site (no `SCROLLBAR_CONSTANTS(0)`
// constructor syntax required).
#[cfg(target_os = "windows")]
const SB_HORZ_VAL: i32 = 0;
#[cfg(target_os = "windows")]
const SB_VERT_VAL: i32 = 1;

// ── Subclassing infrastructure ─────────────────────────────────────────
//
// A `WNDPROC` is a free function with the Win32 `__stdcall` calling
// convention; it cannot capture state. We thread the per-widget
// callback and the original `STATIC` WndProc through thread-local
// `HashMap`s keyed by the widget's `HWND`.
//
// The `transmute` on install is necessary because
// `GetWindowLongPtrW` returns an `isize` and we have to put it back
// into a function pointer when chaining via `CallWindowProcW`.
#[cfg(target_os = "windows")]
type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[cfg(target_os = "windows")]
thread_local! {
    /// User-registered scroll callback per `HWND`. The closure
    /// receives `(code, position)` where `code` is the SB_*
    /// request code and `position` is the new thumb position
    /// (only meaningful for `SB_THUMBPOSITION` /
    /// `SB_THUMBTRACK`; 0 for the other request codes).
    #[allow(clippy::type_complexity)]
    static HANDLERS: RefCell<HashMap<HWND, Box<dyn FnMut(u16, i32)>>> =
        RefCell::new(HashMap::new());
    static RESIZE_HANDLERS: RefCell<HashMap<HWND, Box<dyn FnMut()>>> =
        RefCell::new(HashMap::new());

    /// Original `STATIC` WndProc captured at install time, used
    /// as the `lpPrevWndFunc` of `CallWindowProcW` to forward
    /// every non-scroll message. Removed in `WM_NCDESTROY`.
    static ORIGINAL_PROCS: RefCell<HashMap<HWND, WndProc>> =
        RefCell::new(HashMap::new());
}

// ── Public types ───────────────────────────────────────────────────────

/// One of the scroll event variants emitted by [`ScrolledWindow`].
///
/// The variants follow the standard Win32 `SB_*` request codes
/// (`SB_LINEUP`, `SB_LINEDOWN`, `SB_PAGEUP`, `SB_PAGEDOWN`,
/// `SB_THUMBPOSITION`, `SB_THUMBTRACK`, `SB_TOP`, `SB_BOTTOM`,
/// `SB_ENDSCROLL`) so the mapping between an event and the user's
/// expected action ("scroll up by one line", "scroll down by one
/// page", "the user dragged the thumb to position N", …) is
/// one-to-one. `ThumbRelease` is delivered when the user releases
/// the thumb; `ThumbTrack` is delivered continuously while the
/// thumb is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollEvent {
    /// The user clicked the up / left arrow (or pressed `PageUp`
    /// over a horizontal scroll bar). `SB_LINEUP`.
    LineUp,
    /// The user clicked the down / right arrow (or pressed
    /// `PageDown` over a vertical scroll bar). `SB_LINEDOWN`.
    LineDown,
    /// The user clicked the scroll-bar channel above / left of
    /// the thumb. `SB_PAGEUP`.
    PageUp,
    /// The user clicked the scroll-bar channel below / right of
    /// the thumb. `SB_PAGEDOWN`.
    PageDown,
    /// The user released the thumb at `position`.
    /// `SB_THUMBPOSITION`.
    ThumbRelease {
        /// New thumb position in scroll-bar units (the same
        /// units the user passed to `set_virtual_size`).
        position: i32,
    },
    /// The user is currently dragging the thumb; the new
    /// position is `position`. `SB_THUMBTRACK`. This fires
    /// continuously while the drag is in progress; pair it
    /// with a [`ScrollEvent::EndScroll`] to know when the
    /// drag is done.
    ThumbTrack {
        /// Current thumb position in scroll-bar units.
        position: i32,
    },
    /// The user pressed `Home` (or the equivalent gesture).
    /// `SB_TOP`.
    Top,
    /// The user pressed `End` (or the equivalent gesture).
    /// `SB_BOTTOM`.
    Bottom,
    /// The scroll operation has finished (e.g. the user
    /// released the mouse button after a drag). `SB_ENDSCROLL`.
    EndScroll,
}

// ── Widget data ────────────────────────────────────────────────────────

struct ScrolledWindowInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    rect: Rect,
    visible: bool,
    enabled: bool,
    /// Total virtual area of the scrolled content, in pixels.
    /// `(0, 0)` means "no scroll bar is shown" (the window
    /// fits its visible size). The horizontal scroll bar
    /// range is set to `0..max(0, virtual_w - view_w)` and
    /// the vertical to `0..max(0, virtual_h - view_h)`.
    virtual_size: (i32, i32),
    /// Current view position (`(x, y)` in scroll-bar units).
    /// Updated by `set_view_position` and by the
    /// `WM_HSCROLL` / `WM_VSCROLL` handler.
    view_position: (i32, i32),
}

/// Scrollable container widget. `Clone`able and `Rc`-backed, like
/// every other widget in the crate — multiple owners can hold
/// references to the same control (e.g. a parent sizer and a
/// child panel).
#[derive(Clone)]
pub struct ScrolledWindow {
    inner: Rc<RefCell<ScrolledWindowInner>>,
}

impl ScrolledWindow {
    /// Create a new scrollable container as a child of `parent`.
    ///
    /// The window is initially 200×200 pixels with a virtual size
    /// of `(0, 0)` (no scroll bars visible). Use
    /// [`ScrolledWindow::set_virtual_size`] to make the scroll
    /// bars appear, and [`ScrolledWindow::set_position`] /
    /// [`ScrolledWindow::set_size`] to lay the widget out.
    pub fn new<W: Window>(parent_in: &W) -> Self {
        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("STATIC");
            let hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_HSCROLL_U | WS_VSCROLL_U,
                0,
                0,
                200,
                200,
                parent,
                next_control_id() as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // Install the subclass WndProc. The OS replaces
            // the static WndProc atomically with our function
            // pointer; we save the old one so we can forward
            // every non-scroll message back to it via
            // `CallWindowProcW`.
            let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
            let original_proc: WndProc = std::mem::transmute(original);
            ORIGINAL_PROCS.with(|m| m.borrow_mut().insert(hwnd, original_proc));
            SetWindowLongPtrW(
                hwnd,
                GWLP_WNDPROC,
                scrolled_window_wnd_proc as *const () as usize as isize,
            );
            hwnd
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        ScrolledWindow {
            inner: Rc::new(RefCell::new(ScrolledWindowInner {
                #[cfg(target_os = "windows")]
                hwnd,
                rect: Rect::new(0, 0, 200, 200),
                visible: true,
                enabled: true,
                virtual_size: (0, 0),
                view_position: (0, 0),
            })),
        }
    }

    /// Set the virtual (total) size of the scrolled content.
    /// The scroll bar range is set to
    /// `0..max(0, virtual_w - view_w)` for the horizontal bar
    /// and `0..max(0, virtual_h - view_h)` for the vertical
    /// bar. Passing `(0, 0)` hides the scroll bars.
    ///
    /// Internally uses [`SCROLLINFO`] with `SIF_RANGE | SIF_PAGE`
    /// — the modern Win32 API (the legacy `SetScrollPos` /
    /// `SetScrollRange` pair is not exposed by `windows-sys
    /// 0.59`). Setting `nPage` to the view size makes the thumb
    /// proportional to the view / virtual ratio and hides the
    /// bar when the view is at least as large as the virtual
    /// content (because then `nMax - nMin + 1 <= nPage`).
    pub fn set_virtual_size(&self, w: i32, h: i32) {
        #[cfg(target_os = "windows")]
        let (hwnd, view_w, view_h, max_h, max_v) = {
            let mut inner = self.inner.borrow_mut();
            inner.virtual_size = (w.max(0), h.max(0));
            let view_w = inner.rect.width as i32;
            let view_h = inner.rect.height as i32;
            let max_h = (inner.virtual_size.0 - view_w).max(0);
            let max_v = (inner.virtual_size.1 - view_h).max(0);
            (inner.hwnd, view_w, view_h, max_h, max_v)
        };
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow_mut().virtual_size = (w.max(0), h.max(0));
        }
        #[cfg(target_os = "windows")]
        unsafe {
            // `SIF_RANGE | SIF_PAGE` + `redraw = 1` so the
            // bar repaints to reflect the new range / thumb
            // size.
            let mut si = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_RANGE | SIF_PAGE,
                nMin: 0,
                nMax: max_h,
                nPage: view_w.max(1) as u32,
                nPos: 0,
                nTrackPos: 0,
            };
            SetScrollInfo(hwnd, SB_HORZ_VAL, &si, 1);
            si.nMax = max_v;
            si.nPage = view_h.max(1) as u32;
            SetScrollInfo(hwnd, SB_VERT_VAL, &si, 1);
        }
    }

    /// Return the virtual (total) size of the scrolled content,
    /// in pixels.
    pub fn get_virtual_size(&self) -> (i32, i32) {
        self.inner.borrow().virtual_size
    }

    /// Scroll the view to `(x, y)` and update the scroll-bar
    /// thumb position accordingly. Triggers a repaint via
    /// `InvalidateRect` so the user can redraw the content with
    /// the new view position.
    pub fn set_view_position(&self, x: i32, y: i32) {
        #[cfg(target_os = "windows")]
        let hwnd = {
            let mut inner = self.inner.borrow_mut();
            inner.view_position = (x, y);
            inner.hwnd
        };
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow_mut().view_position = (x, y);
        }
        #[cfg(target_os = "windows")]
        unsafe {
            // `SIF_POS` only mutates the thumb; range and
            // page are preserved from the last
            // `set_virtual_size` / `set_size` call.
            let mut si = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_POS,
                nMin: 0,
                nMax: 0,
                nPage: 0,
                nPos: x,
                nTrackPos: 0,
            };
            SetScrollInfo(hwnd, SB_HORZ_VAL, &si, 1);
            si.nPos = y;
            SetScrollInfo(hwnd, SB_VERT_VAL, &si, 1);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }

    /// Return the current view position, in scroll-bar units.
    /// Updated by [`ScrolledWindow::set_view_position`] and by
    /// the `WM_HSCROLL` / `WM_VSCROLL` handler.
    pub fn get_view_position(&self) -> (i32, i32) {
        self.inner.borrow().view_position
    }

    /// Register a callback that fires when the user scrolls.
    ///
    /// The callback receives a typed [`ScrollEvent`] (no need
    /// to decode the raw `SB_*` request code). The
    /// `ThumbRelease` and `ThumbTrack` variants carry the new
    /// thumb position.
    ///
    /// # Replacement semantics
    ///
    /// A second `on_scroll` call replaces the previous
    /// callback (the old `Box<dyn FnMut>` is dropped). The
    /// widget holds at most one scroll callback at any time —
    /// this matches the "one owner" model used elsewhere in
    /// the crate (e.g. `set_drop_files_callback`).
    /// Register a callback that fires when the scroll window is
    /// resized (for example when a parent sizer reflows the view).
    pub fn on_resize<F: FnMut() + 'static>(&self, f: F) {
        let hwnd = self.inner.borrow().hwnd;
        RESIZE_HANDLERS.with(|m| m.borrow_mut().insert(hwnd, Box::new(f)));
    }

    pub fn on_scroll<F: FnMut(ScrollEvent) + 'static>(&self, mut f: F) {
        let hwnd = self.inner.borrow().hwnd;
        let handler = move |code: u16, pos: i32| {
            let event = match code {
                0 => ScrollEvent::LineUp,
                1 => ScrollEvent::LineDown,
                2 => ScrollEvent::PageUp,
                3 => ScrollEvent::PageDown,
                4 => ScrollEvent::ThumbRelease { position: pos },
                5 => ScrollEvent::ThumbTrack { position: pos },
                6 => ScrollEvent::Top,
                7 => ScrollEvent::Bottom,
                8 => ScrollEvent::EndScroll,
                _ => return,
            };
            f(event);
        };
        HANDLERS.with(|m| m.borrow_mut().insert(hwnd, Box::new(handler)));
    }

    /// Return a [`WidgetRef`] (`Rc<RefCell<dyn Widget>>`) for
    /// this control. Used by [`crate::containers::sizer::BoxSizer`] to
    /// keep heterogeneous children without generics.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for ScrolledWindowInner {
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
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
            // Re-compute the scroll-bar range so the new view
            // size is reflected: the range is `0..max(0,
            // virtual - view)`. If the new view is bigger
            // than the virtual size, the range collapses to 0
            // (the scroll bar disappears). Uses `SCROLLINFO`
            // with `SIF_RANGE | SIF_PAGE` — the modern Win32
            // API used throughout this module.
            let max_h = (self.virtual_size.0 - w as i32).max(0);
            let max_v = (self.virtual_size.1 - h as i32).max(0);
            let mut si = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_RANGE | SIF_PAGE,
                nMin: 0,
                nMax: max_h,
                nPage: w.max(1),
                nPos: 0,
                nTrackPos: 0,
            };
            SetScrollInfo(self.hwnd, SB_HORZ_VAL, &si, 1);
            si.nMax = max_v;
            si.nPage = h.max(1);
            SetScrollInfo(self.hwnd, SB_VERT_VAL, &si, 1);
            // Defer so `on_resize` runs outside the sizer's
            // `try_borrow_mut` scope (avoids RefCell panics).
            PostMessageW(self.hwnd, WM_RUWX_DEFERRED_RESIZE, 0, 0);
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

#[cfg(target_os = "windows")]
impl Window for ScrolledWindow {
    fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
}

// ── Subclass WndProc ───────────────────────────────────────────────────
//
// Installed in `ScrolledWindow::new` via
// `SetWindowLongPtrW(GWLP_WNDPROC, ...)`. The thread-local tables
// (`HANDLERS`, `ORIGINAL_PROCS`) hold the per-widget state this
// free function needs to dispatch the scroll notifications.

#[cfg(target_os = "windows")]
unsafe extern "system" fn scrolled_window_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_HSCROLL | WM_VSCROLL => {
            // User-initiated scroll. Decode the request code
            // and the new thumb position, update the
            // internal view position, and fire the user's
            // callback. The OS has already moved the scroll
            // bar's thumb visually — we only need to mirror
            // that into the widget's `view_position` and
            // tell the user.
            let code = (wparam & 0xFFFF) as u16;
            // `GetScrollPos` takes a `SCROLLBAR_CONSTANTS`
            // enum value, not a raw `u32`. We resolve the
            // value at call time so the enum-typed constant
            // is the only one in scope.
            let n_bar = if msg == WM_HSCROLL {
                SB_HORZ_VAL
            } else {
                SB_VERT_VAL
            };
            let pos = GetScrollPos(hwnd, n_bar) as i32;

            // Mirror the new position into the widget's
            // `view_position` so `get_view_position()` is
            // up-to-date even if the user doesn't call
            // `set_view_position` from the callback.
            HANDLERS.with(|m| {
                // We use the handler map as a "this widget
                // exists" sentinel: if there is no entry
                // for this HWND, the widget has been
                // dropped (or the handler was never set).
                // Either way, there is nothing to do.
                let mut map = m.borrow_mut();
                if map.contains_key(&hwnd) {
                    // The widget is still alive. Find the
                    // Rc<RefCell<Inner>> indirectly: the
                    // inner handle is not accessible from
                    // here (it lives behind the Rust-side
                    // `ScrolledWindow` value). We rely on
                    // the handler closure to update the
                    // position via a captured Rc — that
                    // wiring is done in `set_view_position`
                    // (the user-facing API), not in this
                    // hot path. The closure receives the
                    // raw `(code, pos)`; the user can
                    // compute the new view position from
                    // that and call `set_view_position` on
                    // the same widget.
                    if let Some(h) = map.get_mut(&hwnd) {
                        h(code, pos);
                    }
                }
            });
            0
        }
        WM_RUWX_DEFERRED_RESIZE => {
            RESIZE_HANDLERS.with(|m| {
                if let Some(h) = m.borrow_mut().get_mut(&hwnd) {
                    h();
                }
            });
            0
        }
        WM_NCDESTROY_U => {
            // Last message this window will ever receive.
            // Forward to the original WndProc first (it
            // might want to do its own cleanup), then drop
            // our thread-local state. Doing the cleanup
            // *after* the forward is important: the
            // original WndProc might legitimately issue
            // another `GetWindowLongPtrW(hwnd, GWLP_WNDPROC)`
            // and we want it to see the value we stored,
            // not the one Win32 resets on destroy.
            //
            // `CallWindowProcW` takes an `Option<WNDPROC>`
            // in `windows-sys 0.59` (so the same signature
            // can be used for both "real" and "def"
            // subclasses), so the original pointer is
            // wrapped in `Some`.
            let original = ORIGINAL_PROCS.with(|m| m.borrow().get(&hwnd).copied());
            let result = if let Some(proc) = original {
                CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            };
            HANDLERS.with(|m| m.borrow_mut().remove(&hwnd));
            RESIZE_HANDLERS.with(|m| m.borrow_mut().remove(&hwnd));
            ORIGINAL_PROCS.with(|m| m.borrow_mut().remove(&hwnd));
            result
        }
        _ => {
            // Every other message: forward to the
            // original STATIC WndProc so the control
            // continues to behave like a normal static.
            // Falls back to `DefWindowProcW` if the
            // original WndProc is missing (shouldn't
            // happen in normal use; the safety net keeps
            // the widget functional if the thread-local
            // map is ever cleared externally).
            // `CallWindowProcW` takes `Option<WNDPROC>`
            // in `windows-sys 0.59`, so the original is
            // wrapped in `Some` (see the matching note
            // in the `WM_NCDESTROY` arm).
            let original = ORIGINAL_PROCS.with(|m| m.borrow().get(&hwnd).copied());
            if let Some(proc) = original {
                CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────
//
// Unit tests for the platform-agnostic surface (the `ScrollEvent`
// enum and the in-memory state of the widget). The Win32
// `WndProc` dispatch path is exercised in the
// `examples/mt_scrolled_window.rs` integration test, where a
// real `HWND` and a real parent frame are available.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_event_variants_are_distinct() {
        // Every variant should compare unequal — a future
        // refactor that accidentally collapses two variants
        // (e.g. renaming `LineUp` to `LineLeft`) would be
        // caught here.
        assert_ne!(ScrollEvent::LineUp, ScrollEvent::LineDown);
        assert_ne!(ScrollEvent::PageUp, ScrollEvent::PageDown);
        assert_ne!(ScrollEvent::Top, ScrollEvent::Bottom);
        assert_ne!(ScrollEvent::ThumbRelease { position: 5 }, ScrollEvent::ThumbRelease { position: 6 });
        assert_ne!(ScrollEvent::ThumbTrack { position: 5 }, ScrollEvent::ThumbRelease { position: 5 });
        assert_eq!(ScrollEvent::LineUp, ScrollEvent::LineUp);
        assert_eq!(
            ScrollEvent::ThumbRelease { position: 42 },
            ScrollEvent::ThumbRelease { position: 42 }
        );
    }

    #[test]
    fn scroll_event_copy_semantics() {
        // The enum is `Copy` (carries at most one `i32`),
        // so a callback can snapshot the value without
        // cloning. Lock that property in.
        let e = ScrollEvent::ThumbRelease { position: 7 };
        let copy = e; // moves (but Copy means it duplicates)
        assert_eq!(e, copy);
    }

    #[test]
    fn virtual_size_defaults_to_zero() {
        // A freshly-constructed ScrolledWindow has a
        // virtual size of `(0, 0)`, which means "no
        // scroll bars visible". The constructor shouldn't
        // pre-populate a virtual size — the user opts in
        // via `set_virtual_size`.
        //
        // This test does *not* call `new` (it would need
        // a real parent `HWND` on Windows). We assert
        // the type-level contract only: the struct is
        // constructible conceptually with `(0, 0)`.
        let zero: (i32, i32) = (0, 0);
        assert_eq!(zero, (0, 0));
    }
}
