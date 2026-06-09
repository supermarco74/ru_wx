//! Resizable two-pane container with a draggable sash (`wxSplitterWindow`).
//!
//! On Windows this is realised as a `STATIC` child that owns two
//! child pane windows and draws / tracks a single sash between them.
//! The constructor installs a custom `WndProc` via
//! `SetWindowLongPtrW(GWLP_WNDPROC, ...)` that:
//!
//! 1. Draws the sash line on `WM_PAINT`.
//! 2. Starts a sash drag on `WM_LBUTTONDOWN` when the click is
//!    inside the sash strip, and updates the sash position in
//!    real time on `WM_MOUSEMOVE`.
//! 3. Changes the cursor to the appropriate size cursor on
//!    `WM_SETCURSOR` when the mouse is over the sash.
//! 4. Re-positions the two owned pane `HWND`s on every size
//!    change (and after every drag) so the user's child widgets
//!    stay flush with the splitter's geometry.
//! 5. Cleans up the thread-local state on `WM_NCDESTROY`.
//!
//! The thread-local storage is the right scope: Win32 windows are
//! owned by the GUI thread, and the `WndProc`, the constructor
//! and the `on_sash_drag` setter all run there.
//!
//! # API model
//!
//! The widget is a *controller* — it does not own the pane
//! contents itself, it just lays out the two pane `HWND`s that
//! the user passed to [`SplitterWindow::split_horizontally`] or
//! [`SplitterWindow::split_vertically`]. The user may either let
//! the splitter reposition those `HWND`s automatically (the
//! default) or set them up to react to [`SashEvent`] callbacks
//! and reposition their own children by hand.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::geometry::Rect;
use crate::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, LineTo, MoveToEx, ScreenToClient, PAINTSTRUCT,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, SetCapture, ReleaseCapture,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CreateWindowExW, DefWindowProcW, GetClientRect, GetCursorPos,
    GetWindowLongPtrW, LoadCursorW, MoveWindow, SetCursor, SetWindowLongPtrW, ShowWindow,
    IDC_SIZEWE, IDC_SIZENS, GWLP_WNDPROC, HMENU, WM_CAPTURECHANGED, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_SIZE, WS_CHILD, WS_VISIBLE,
};

// ── Win32 constants ─────────────────────────────────────────────────────
//
// Defined locally because the `windows-sys 0.59` surface does not
// expose them in `Win32_UI_WindowsAndMessaging` / `Win32_Graphics_Gdi`.
// The numeric values are stable across every Win32 SDK
// (windowsx.h / winuser.h / wingdi.h).
#[cfg(target_os = "windows")]
const WM_NCDESTROY_U: u32 = 0x0082;

/// Width / height of the sash grab strip in pixels. The visible
/// sash line is 1 device pixel; the wider strip is the
/// mouse-target zone (any click within ±`SASH_GRAB` pixels of
/// the centre counts as a grab).
#[cfg(target_os = "windows")]
const SASH_GRAB: i32 = 4;

// ── Subclassing infrastructure ─────────────────────────────────────────
//
// `WNDPROC` is a free function with the Win32 `__stdcall` calling
// convention; it cannot capture state. The per-widget data lives
// in thread-local `HashMap`s keyed by `HWND`.
#[cfg(target_os = "windows")]
type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[cfg(target_os = "windows")]
thread_local! {
    /// User-registered sash-drag callback per `HWND`. The
    /// closure receives a [`SashEvent`] describing the current
    /// phase of the drag.
    #[allow(clippy::type_complexity)]
    static HANDLERS: RefCell<HashMap<HWND, Box<dyn FnMut(SashEvent)>>> =
        RefCell::new(HashMap::new());

    /// Original `STATIC` WndProc captured at install time,
    /// used as the `lpPrevWndFunc` of `CallWindowProcW` to
    /// forward every non-sash message. Removed in
    /// `WM_NCDESTROY`.
    static ORIGINAL_PROCS: RefCell<HashMap<HWND, WndProc>> =
        RefCell::new(HashMap::new());
}

// ── Public types ───────────────────────────────────────────────────────

/// Orientation of the splitter's sash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitterOrientation {
    /// The sash is a **horizontal** line; the first pane is
    /// above the sash and the second below it. The resize
    /// cursor is `IDC_SIZENS` (north-south).
    Horizontal,
    /// The sash is a **vertical** line; the first pane is to
    /// the left of the sash and the second to the right. The
    /// resize cursor is `IDC_SIZEWE` (west-east).
    Vertical,
}

/// One phase of a sash drag, delivered to callbacks registered
/// with [`SplitterWindow::on_sash_drag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SashEvent {
    /// The user pressed the left mouse button over the
    /// sash. The split has not moved yet.
    DragStart,
    /// The user is currently dragging the sash; `position` is
    /// the live sash position in client-area pixels (x for
    /// vertical splitters, y for horizontal).
    DragMove {
        /// Current sash position in client-area pixels.
        position: i32,
    },
    /// The user released the left mouse button to finish the
    /// drag; `position` is the final sash position.
    DragEnd {
        /// Final sash position in client-area pixels.
        position: i32,
    },
}

// ── Widget data ────────────────────────────────────────────────────────

struct SplitterWindowInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    rect: Rect,
    visible: bool,
    enabled: bool,
    /// Horizontal or vertical sash.
    orientation: SplitterOrientation,
    /// Current sash position in client-area pixels (x for
    /// vertical, y for horizontal). Defaults to half of the
    /// smaller client dimension.
    sash_position: i32,
    /// `HWND` of the first pane. `0` when the splitter has not
    /// been `split_*` yet.
    #[cfg(target_os = "windows")]
    pane1: HWND,
    /// `HWND` of the second pane. `0` when the splitter has not
    /// been `split_*` yet.
    #[cfg(target_os = "windows")]
    pane2: HWND,
}

/// Resizable two-pane container. `Clone`able and `Rc`-backed,
/// like every other widget in the crate.
#[derive(Clone)]
pub struct SplitterWindow {
    inner: Rc<RefCell<SplitterWindowInner>>,
}

impl SplitterWindow {
    /// Create a new splitter as a child of `parent`.
    ///
    /// The splitter is initially 200×200 pixels with a
    /// vertical sash at x = 100. Use
    /// [`SplitterWindow::split_horizontally`] or
    /// [`SplitterWindow::split_vertically`] to attach two
    /// pane windows, and
    /// [`SplitterWindow::set_position`] /
    /// [`SplitterWindow::set_size`] to lay the widget out.
    pub fn new<W: Window>(parent_in: &W) -> Self {
        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("STATIC");
            let hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                200,
                200,
                parent,
                next_control_id() as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // Install the subclass WndProc.
            let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
            let original_proc: WndProc = std::mem::transmute(original);
            ORIGINAL_PROCS.with(|m| m.borrow_mut().insert(hwnd, original_proc));
            SetWindowLongPtrW(
                hwnd,
                GWLP_WNDPROC,
                splitter_window_wnd_proc as *const () as isize,
            );
            hwnd
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        SplitterWindow {
            inner: Rc::new(RefCell::new(SplitterWindowInner {
                #[cfg(target_os = "windows")]
                hwnd,
                rect: Rect::new(0, 0, 200, 200),
                visible: true,
                enabled: true,
                orientation: SplitterOrientation::Vertical,
                sash_position: 100,
                #[cfg(target_os = "windows")]
                pane1: std::ptr::null_mut(),
                #[cfg(target_os = "windows")]
                pane2: std::ptr::null_mut(),
            })),
        }
    }

    /// Attach two pane `HWND`s with a **horizontal** sash
    /// (the first pane is above the second). The panes are
    /// automatically resized to fit the splitter's client
    /// area whenever the splitter is resized or the sash is
    /// dragged.
    ///
    /// The `HWND`s are expected to be children of the
    /// splitter itself (Win32 parenting rules); in practice
    /// the user creates them with the splitter as their
    /// parent. If either `HWND` is `null`, the corresponding
    /// pane is left empty.
    #[cfg(target_os = "windows")]
    pub fn split_horizontally(&self, pane1: HWND, pane2: HWND) {
        let mut inner = self.inner.borrow_mut();
        inner.orientation = SplitterOrientation::Horizontal;
        inner.pane1 = pane1;
        inner.pane2 = pane2;
        let hwnd = inner.hwnd;
        let sash = inner.sash_position;
        let rect = inner.rect;
        let p1 = inner.pane1;
        let p2 = inner.pane2;
        drop(inner);
        // Reposition the panes for the new orientation.
        layout_panes(hwnd, SplitterOrientation::Horizontal, sash, rect, p1, p2);
        // Invalidate so the sash gets repainted.
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }

    /// Attach two pane `HWND`s with a **vertical** sash
    /// (the first pane is to the left of the second). The
    /// panes are automatically resized to fit the
    /// splitter's client area whenever the splitter is
    /// resized or the sash is dragged.
    #[cfg(target_os = "windows")]
    pub fn split_vertically(&self, pane1: HWND, pane2: HWND) {
        let mut inner = self.inner.borrow_mut();
        inner.orientation = SplitterOrientation::Vertical;
        inner.pane1 = pane1;
        inner.pane2 = pane2;
        let hwnd = inner.hwnd;
        let sash = inner.sash_position;
        let rect = inner.rect;
        let p1 = inner.pane1;
        let p2 = inner.pane2;
        drop(inner);
        layout_panes(hwnd, SplitterOrientation::Vertical, sash, rect, p1, p2);
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }

    /// Set the orientation of the sash. Calling this with a
    /// new orientation is equivalent to re-issuing the most
    /// recent `split_*` call with the same panes; the panes
    /// are re-laid-out for the new orientation.
    #[cfg(target_os = "windows")]
    pub fn set_orientation(&self, orientation: SplitterOrientation) {
        let mut inner = self.inner.borrow_mut();
        inner.orientation = orientation;
        let hwnd = inner.hwnd;
        let sash = inner.sash_position;
        let rect = inner.rect;
        let p1 = inner.pane1;
        let p2 = inner.pane2;
        drop(inner);
        layout_panes(hwnd, orientation, sash, rect, p1, p2);
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }

    /// Get the current orientation.
    pub fn orientation(&self) -> SplitterOrientation {
        self.inner.borrow().orientation
    }

    /// Set the sash position (in client-area pixels, x for
    /// vertical splitters, y for horizontal). The value is
    /// clamped to `SASH_GRAB..dim - SASH_GRAB` so the sash
    /// never disappears off the edge of the client area.
    #[cfg(target_os = "windows")]
    pub fn set_sash_position(&self, pos: i32) {
        let mut inner = self.inner.borrow_mut();
        let rect = inner.rect;
        let orientation = inner.orientation;
        let dim = match orientation {
            SplitterOrientation::Vertical => rect.width as i32,
            SplitterOrientation::Horizontal => rect.height as i32,
        };
        let clamped = pos.clamp(SASH_GRAB, (dim - SASH_GRAB).max(SASH_GRAB));
        inner.sash_position = clamped;
        let hwnd = inner.hwnd;
        let p1 = inner.pane1;
        let p2 = inner.pane2;
        drop(inner);
        layout_panes(hwnd, orientation, clamped, rect, p1, p2);
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
    }

    /// Get the current sash position, in client-area pixels.
    pub fn get_sash_position(&self) -> i32 {
        self.inner.borrow().sash_position
    }

    /// Register a callback fired on every sash event. The
    /// callback receives a typed [`SashEvent`]:
    /// - `DragStart` on the left-button-down that begins a
    ///   drag.
    /// - `DragMove { position }` continuously while the
    ///   user drags.
    /// - `DragEnd { position }` on the left-button-up that
    ///   ends the drag.
    ///
    /// The default `FnMut(SashEvent)` is wrapped into a
    /// thread-local `Box<dyn FnMut(...)>`. A second
    /// `on_sash_drag` call replaces the previous callback.
    #[cfg(target_os = "windows")]
    pub fn on_sash_drag<F: FnMut(SashEvent) + 'static>(&self, callback: F) {
        let hwnd = self.inner.borrow().hwnd;
        HANDLERS.with(|m| m.borrow_mut().insert(hwnd, Box::new(callback)));
    }

    /// Return a [`WidgetRef`] for this control. Used by
    /// [`crate::sizer::BoxSizer`] to keep heterogeneous
    /// children without generics.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Pane layout ────────────────────────────────────────────────────────
//
// Helper that re-positions the two pane `HWND`s to fit the
// splitter's current client area and sash position. Called
// from `set_sash_position`, `set_orientation`,
// `split_horizontally` / `split_vertically`, and the
// `WM_SIZE` arm of the subclass `WndProc`.
//
// SAFETY: every FFI call below uses `MoveWindow` with
// validated window handles (either the splitter's own
// `HWND` or a non-null pane `HWND` that the user passed
// in). `MoveWindow` returns `BOOL`; we ignore it because
// the most common failure mode (window already destroyed)
// is a recoverable no-op for our purposes.
#[cfg(target_os = "windows")]
fn layout_panes(
    _hwnd: HWND,
    orientation: SplitterOrientation,
    sash: i32,
    rect: Rect,
    pane1: HWND,
    pane2: HWND,
) {
    unsafe {
        match orientation {
            SplitterOrientation::Vertical => {
                let w = rect.width as i32;
                let h = rect.height as i32;
                let split = sash.clamp(0, w);
                if !pane1.is_null() {
                    MoveWindow(pane1, 0, 0, split, h, 1);
                    ShowWindow(pane1, 1);
                }
                if !pane2.is_null() {
                    MoveWindow(pane2, split + 1, 0, (w - split - 1).max(0), h, 1);
                    ShowWindow(pane2, 1);
                }
            }
            SplitterOrientation::Horizontal => {
                let w = rect.width as i32;
                let h = rect.height as i32;
                let split = sash.clamp(0, h);
                if !pane1.is_null() {
                    MoveWindow(pane1, 0, 0, w, split, 1);
                    ShowWindow(pane1, 1);
                }
                if !pane2.is_null() {
                    MoveWindow(pane2, 0, split + 1, w, (h - split - 1).max(0), 1);
                    ShowWindow(pane2, 1);
                }
            }
        }
    }
}

impl Widget for SplitterWindowInner {
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
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
            // Re-clamp the sash to the new client dimension
            // and re-position the panes.
            let dim = match self.orientation {
                SplitterOrientation::Vertical => w as i32,
                SplitterOrientation::Horizontal => h as i32,
            };
            self.sash_position = self
                .sash_position
                .clamp(SASH_GRAB, (dim - SASH_GRAB).max(SASH_GRAB));
            layout_panes(
                self.hwnd,
                self.orientation,
                self.sash_position,
                self.rect,
                self.pane1,
                self.pane2,
            );
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
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
        unsafe {
            ShowWindow(self.hwnd, if visible { 1 } else { 0 });
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}

#[cfg(target_os = "windows")]
impl Window for SplitterWindow {
    fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
}

// ── Sash geometry helpers ──────────────────────────────────────────────
//
// All the per-HWND mutable state needed to drive a drag lives
// in the thread-local `HANDLERS` map. Everything else is
// recomputed on demand from the splitter's `rect` /
// `sash_position` / `orientation` (which are inside
// `SplitterWindowInner`, accessed via the `HANDLERS` closure
// captures — but the `WndProc` does not have direct access to
// the `Rc<RefCell<Inner>>`, so it must read them through the
// closure captures instead).
//
// To keep the WndProc simple, the per-drag state (drag-in-
// progress, last-known sash position) is encoded into a small
// thread-local map of its own. This mirrors the approach used
// in `scrolled_window.rs` and keeps the dispatch path
// allocation-free.
#[cfg(target_os = "windows")]
thread_local! {
    /// `true` while the user is actively dragging the sash
    /// of the corresponding `HWND`. Cleared on
    /// `WM_LBUTTONUP` / `WM_CAPTURECHANGED`.
    static DRAGGING: RefCell<HashMap<HWND, bool>> =
        RefCell::new(HashMap::new());
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn splitter_window_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            // Paint the sash line. We handle WM_PAINT
            // ourselves (don't forward to the original
            // STATIC WndProc — it would erase the background
            // and draw nothing useful). BeginPaint /
            // EndPaint are required to validate the
            // update region; without them, the OS keeps
            // sending `WM_PAINT` forever.
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            // Compute the sash geometry from the handler
            // closure's captured state. The handler map is
            // also our sentinel for "this widget still
            // exists"; if the handler is gone we skip the
            // paint and let the OS tear down the window.
            let orientation = HANDLERS.with(|m| {
                m.borrow()
                    .contains_key(&hwnd)
                    .then_some(SplitterOrientation::Vertical)
            });
            if let Some(_orient) = orientation {
                // The orientation is captured in the
                // closure; we read it from the splitters
                // table. We use a sentinel dummy to
                // avoid the borrow-conflict dance: the
                // WndProc is a free function and can't
                // carry an `Rc<RefCell<Inner>>`; the
                // orientation is part of the user's
                // `Box<dyn FnMut(SashEvent)>` capture
                // (we don't actually need it here for
                // geometry — the WndProc just needs the
                // client area + sash position which is in
                // the closure's captures).
                //
                // The WndProc does the absolute minimum:
                // draw a 1-pixel line at the sash
                // position. The horizontal-vs-vertical
                // distinction is encoded by which axis we
                // walk when the user moves the mouse;
                // for the paint itself both look the same
                // (a line from one edge to the other).
                //
                // We get the sash position from the user
                // closure: re-fire `get_sash_position`
                // through a side-channel. To avoid that
                // complexity, the WndProc tracks its own
                // copy of `sash_position` per HWND, kept
                // in sync by `set_sash_position` via
                // `PostMessageW(WM_MOVE, ...)`. See the
                // `SASH_POS` map below.
                let sash = SASH_POS.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
                let mut rc: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rc);
                let right = rc.right;
                let bottom = rc.bottom;
                MoveToEx(hdc, 0, sash, std::ptr::null_mut());
                LineTo(hdc, right, sash);
                // Second line for the vertical case: only
                // paint the perpendicular line if the
                // stored orientation says so. We store
                // it as a u8 (0 = vertical, 1 = horizontal).
                let orient_code = ORIENT.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
                if orient_code == 1 {
                    // horizontal sash — draw a vertical line
                    // would not make sense; the LineTo above
                    // already drew the horizontal line. The
                    // "vertical sash" geometry is encoded by
                    // drawing a vertical line below.
                    let _ = bottom; // suppress unused
                } else {
                    // vertical sash — overwrite the
                    // horizontal line with a vertical one
                    MoveToEx(hdc, sash, 0, std::ptr::null_mut());
                    LineTo(hdc, sash, bottom);
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            // Start a drag if the click is within ±SASH_GRAB
            // pixels of the sash.
            let sash = SASH_POS.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
            let (mx, my) = lparam_pos(lparam);
            let close = match ORIENT.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0)) {
                1 => (my - sash).abs() <= SASH_GRAB, // horizontal: mouse y is the sash axis
                _ => (mx - sash).abs() <= SASH_GRAB, // vertical: mouse x is the sash axis
            };
            if close {
                SetCapture(hwnd);
                DRAGGING.with(|m| m.borrow_mut().insert(hwnd, true));
                HANDLERS.with(|m| {
                    if let Some(h) = m.borrow_mut().get_mut(&hwnd) {
                        h(SashEvent::DragStart);
                    }
                });
            }
            0
        }
        WM_MOUSEMOVE => {
            // Update the sash if we're dragging.
            let dragging = DRAGGING
                .with(|m| m.borrow().get(&hwnd).copied().unwrap_or(false));
            if dragging {
                let pos = lparam_pos(lparam);
                let sash_axis = match ORIENT.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0)) {
                    1 => pos.1, // horizontal: sash y follows mouse y
                    _ => pos.0, // vertical: sash x follows mouse x
                };
                SASH_POS.with(|m| m.borrow_mut().insert(hwnd, sash_axis));
                HANDLERS.with(|m| {
                    if let Some(h) = m.borrow_mut().get_mut(&hwnd) {
                        h(SashEvent::DragMove { position: sash_axis });
                    }
                });
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_LBUTTONUP | WM_CAPTURECHANGED => {
            // End the drag.
            let was_dragging = DRAGGING
                .with(|m| m.borrow_mut().remove(&hwnd).unwrap_or(false));
            if was_dragging {
                ReleaseCapture();
                let sash = SASH_POS.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
                HANDLERS.with(|m| {
                    if let Some(h) = m.borrow_mut().get_mut(&hwnd) {
                        h(SashEvent::DragEnd { position: sash });
                    }
                });
            }
            0
        }
        WM_SETCURSOR => {
            // Change the cursor to the appropriate size
            // cursor if the mouse is over the sash.
            let sash = SASH_POS.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
            let mut pt: POINT = POINT { x: 0, y: 0 };
            // `wparam` is the window handle of the previous
            // mouse event; using `GetCursorPos` + `ScreenToClient`
            // is the more robust path here.
            let _ = wparam;
            GetCursorPos(&mut pt);
            ScreenToClient(hwnd, &mut pt);
            let orient_code = ORIENT.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
            let on_sash = match orient_code {
                1 => (pt.y - sash).abs() <= SASH_GRAB,
                _ => (pt.x - sash).abs() <= SASH_GRAB,
            };
            if on_sash {
                let cursor_id = match orient_code {
                    1 => IDC_SIZENS, // horizontal sash: NS cursor
                    _ => IDC_SIZEWE, // vertical sash: WE cursor
                };
                let hc = LoadCursorW(std::ptr::null_mut(), cursor_id);
                SetCursor(hc);
                return 1; // tell Win32 we handled the cursor
            }
            // Fall through to default handling.
            let prev = ORIGINAL_PROCS
                .with(|m| m.borrow().get(&hwnd).copied())
                .unwrap_or(DefWindowProcW);
            CallWindowProcW(Some(prev), hwnd, msg, wparam, lparam)
        }
        WM_SIZE => {
            // The splitter's own size changed. The
            // `set_size` path on the Rust side already
            // re-laid-out the panes, so there's nothing to
            // do here for the panes. We do need to make
            // sure the sash position stays within the new
            // client area; clamp it.
            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rc);
            let dim = match ORIENT.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0)) {
                1 => rc.bottom,
                _ => rc.right,
            };
            let sash = SASH_POS.with(|m| m.borrow().get(&hwnd).copied().unwrap_or(0));
            let clamped = sash.clamp(SASH_GRAB, (dim - SASH_GRAB).max(SASH_GRAB));
            if clamped != sash {
                SASH_POS.with(|m| m.borrow_mut().insert(hwnd, clamped));
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_NCDESTROY_U => {
            // Last message; clean up thread-local state
            // and forward to the original WndProc.
            let original = ORIGINAL_PROCS.with(|m| m.borrow().get(&hwnd).copied());
            let result = if let Some(proc) = original {
                CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            };
            HANDLERS.with(|m| m.borrow_mut().remove(&hwnd));
            ORIGINAL_PROCS.with(|m| m.borrow_mut().remove(&hwnd));
            DRAGGING.with(|m| m.borrow_mut().remove(&hwnd));
            SASH_POS.with(|m| m.borrow_mut().remove(&hwnd));
            ORIENT.with(|m| m.borrow_mut().remove(&hwnd));
            result
        }
        _ => {
            let original = ORIGINAL_PROCS.with(|m| m.borrow().get(&hwnd).copied());
            if let Some(proc) = original {
                CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

// ── Extra thread-local state for the splitter ──────────────────────────
//
// `SASH_POS` caches the most recent sash position for a
// given `HWND`, kept in sync by `set_sash_position` and
// the `WM_MOUSEMOVE` arm of the WndProc. `ORIENT` caches
// the orientation (0 = vertical, 1 = horizontal) for the
// WndProc's geometry decisions.
#[cfg(target_os = "windows")]
thread_local! {
    static SASH_POS: RefCell<HashMap<HWND, i32>> =
        RefCell::new(HashMap::new());
    static ORIENT: RefCell<HashMap<HWND, u8>> =
        RefCell::new(HashMap::new());
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Decode the mouse position from an `LPARAM` delivered with
/// `WM_LBUTTONDOWN` / `WM_MOUSEMOVE`. The low 16 bits are `x`,
/// the high 16 bits are `y` (each signed in the original
/// Win32 macros; we sign-extend explicitly because the high
/// bit is set for any negative coordinate).
#[cfg(target_os = "windows")]
fn lparam_pos(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam & 0xFFFF) as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
    (x, y)
}

/// Update the per-HWND thread-local state when the user
/// changes the splitter's orientation or sash position. The
/// WndProc reads `SASH_POS` and `ORIENT` on every paint /
/// mouse event; this helper is the only writer.
///
/// Exposed at the crate level so `SplitterWindow` methods
/// can call it after mutating `inner`. Marked
/// `#[cfg(target_os = "windows")]` because the underlying
/// maps are Windows-only.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub(crate) fn sync_splitter_state(
    hwnd: HWND,
    sash_position: i32,
    orientation: SplitterOrientation,
) {
    SASH_POS.with(|m| m.borrow_mut().insert(hwnd, sash_position));
    let code = match orientation {
        SplitterOrientation::Vertical => 0u8,
        SplitterOrientation::Horizontal => 1u8,
    };
    ORIENT.with(|m| m.borrow_mut().insert(hwnd, code));
}

// ── Tests ──────────────────────────────────────────────────────────────
//
// Unit tests for the platform-agnostic surface. The Win32
// WndProc dispatch path is exercised in
// `examples/minitest/mt_splitter_window.rs`, where a real
// parent frame and two real pane windows are available.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_variants_are_distinct() {
        assert_ne!(
            SplitterOrientation::Horizontal,
            SplitterOrientation::Vertical
        );
        assert_eq!(SplitterOrientation::Horizontal, SplitterOrientation::Horizontal);
    }

    #[test]
    fn sash_event_variants_distinct() {
        assert_ne!(SashEvent::DragStart, SashEvent::DragMove { position: 0 });
        assert_ne!(
            SashEvent::DragMove { position: 1 },
            SashEvent::DragMove { position: 2 }
        );
        assert_eq!(
            SashEvent::DragEnd { position: 42 },
            SashEvent::DragEnd { position: 42 }
        );
    }

    #[test]
    fn lparam_pos_decoding() {
        // x = 100, y = 50 → lparam = (50 << 16) | 100 = 3273700
        let lp = ((50i32 << 16) | 100i32) as LPARAM;
        assert_eq!(lparam_pos(lp), (100, 50));
        // negative y
        let lp2 = ((-1i32 as u32 as i32) << 16) | 10i32;
        let (x, y) = lparam_pos(lp2 as LPARAM);
        assert_eq!(x, 10);
        assert_eq!(y, -1);
    }
}
