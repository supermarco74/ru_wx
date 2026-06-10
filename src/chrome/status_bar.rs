//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxStatusBar — a 1..N-field status bar at the bottom of a frame.
//!
//! On Windows this wraps the standard `msctls_statusbar32` common control.
//! The control is created as a child of the frame and positioned at the
//! bottom; the frame's sizer (if any) does not manage it. Field widths
//! are computed as a simple even split of the frame's client width.
//!
//! Use [`StatusBar::new`] to create the control, then call
//! [`StatusBar::set_status_text`] to write text into a particular field.

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::Widget;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
#[allow(unused_imports)]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 StatusBar constants ───────────────────────────────────────────
//
// We use the `…W` (Unicode / UTF-16) variants of every text message,
// because the buffers we hand the control come from `to_wide`, which
// produces a null-terminated UTF-16 (`wchar_t*`) string. The ANSI
// variants (`SB_SETTEXT = 0x0401`, `SB_GETTEXT = 0x0402`,
// `SB_GETTEXTLENGTH = 0x0403`) read `lParam` as a `char*` and stop at
// the first NUL byte — but a UTF-16 string contains NUL high bytes
// between every ASCII character, so the ANSI variants would store at
// most one character (the first UTF-16 code unit) and report a length
// of 1 regardless of the actual content. The `…W` variants
// (`SB_SETTEXTW = 0x040B`, `SB_GETTEXTW = 0x040D`,
// `SB_GETTEXTLENGTHW = 0x040C`) read `lParam` as a `wchar_t*` and
// honour the full UTF-16 string.
#[cfg(target_os = "windows")]
const SB_SETTEXTW: u32 = 0x040B;
#[cfg(target_os = "windows")]
const SB_GETTEXTW: u32 = 0x040D;
#[cfg(target_os = "windows")]
const SB_GETTEXTLENGTHW: u32 = 0x040C;
#[cfg(target_os = "windows")]
const SB_SETPARTS: u32 = 0x0404;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Kept for reference — the bug it caused is documented above.
const SB_SETTEXT_ANSI: u32 = 0x0401;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SB_GETTEXT_ANSI: u32 = 0x0402;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SB_GETTEXTLENGTH_ANSI: u32 = 0x0403;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — for future parts-count helper
const SB_GETPARTS: u32 = 0x0406;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — simple-mode toggle
const SB_SIMPLE: u32 = 0x0409;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — minimum-height setter
const SB_SETMINHEIGHT: u32 = 0x0408;

/// StatusBar style: include a sizing grip on the right edge.
#[cfg(target_os = "windows")]
const SBARS_SIZEGRIP: u32 = 0x0100;

/// Default height of the status bar, in pixels. Used by the resize
/// handler registered in [`StatusBar::new`] to position the bar at the
/// bottom of the parent frame and re-apply field widths on every
/// `WM_SIZE`. The Win32 `msctls_statusbar32` control's default
/// (≈ 18 px at 100% DPI plus a 1-px border) is rounded up so the
/// `MoveWindow` call always reserves enough vertical space.
#[cfg(target_os = "windows")]
const STATUS_BAR_HEIGHT: i32 = 22;

// ── Inner type ─────────────────────────────────────────────────────────

struct StatusBarInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    /// Number of fields.
    fields: usize,
    /// Cached text of each field (used by `get_status_text` on platforms
    /// other than Windows; on Windows the value is re-queried from the
    /// control via `SB_GETTEXT`).
    texts: Vec<String>,
    rect: Rect,
    visible: bool,
}

// ── Public type ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct StatusBar {
    inner: Rc<RefCell<StatusBarInner>>,
}

impl StatusBar {
    /// Create a new status bar with the given number of fields and attach
    /// it to the bottom of `frame`. The bar's height is the default
    /// (Windows chooses it).
    pub fn new(frame: &Frame, fields: usize) -> Self {
        let fields = fields.max(1);
        let id = 0; // StatusBar does not need a real id (no command dispatch)
        let sb = StatusBar {
            inner: Rc::new(RefCell::new(StatusBarInner {
                #[cfg(target_os = "windows")]
                hwnd: std::ptr::null_mut(),
                fields,
                texts: vec![String::new(); fields],
                rect: Rect::new(0, 0, 0, 0),
                visible: true,
            })),
        };

        #[cfg(target_os = "windows")]
        {
            let wide_class = to_wide("msctls_statusbar32");
            let parent = frame.hwnd();
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP,
                    0,
                    0,
                    0,
                    0,
                    parent,
                    id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            sb.inner.borrow_mut().hwnd = hwnd;

            // `apply_field_widths` queries the parent frame's client rect
            // to size the fields, but the frame is not yet shown at this
            // point so the client area is 0×0. We therefore seed the
            // widths with `0` (one giant field spanning nothing) and let
            // the resize handler installed below recompute them once the
            // frame is on screen and the first `WM_SIZE` arrives.
            sb.inner.borrow().apply_field_widths(0);

            // Register a resize handler on the parent frame that, on
            // every `WM_SIZE`, repositions the bar at the bottom of the
            // frame and re-applies the field widths.
            //
            // Why is this needed? `SB_SETPARTS` (the Win32 message used
            // to set field widths) is a one-shot call: Win32 does not
            // automatically re-apply the widths when the parent is
            // resized, so without this handler the fields would keep
            // their initial 0-width (or, before the fix, the narrow
            // widths computed when the parent was 0×0). The fix is the
            // exact same `SB_SETPARTS` call, just re-issued with the
            // current parent width on every resize.
            //
            // We capture a `Weak<RefCell<StatusBarInner>>` so the
            // handler becomes a no-op once the `StatusBar` is dropped
            // (no risk of touching a freed control).
            let inner_weak = Rc::downgrade(&sb.inner);
            let parent_hwnd = parent; // SAFETY: parent is a valid HWND for the lifetime of the closure
            frame.add_resize_handler(move |_, _| {
                let Some(inner_rc) = inner_weak.upgrade() else {
                    return;
                };
                let sb_hwnd = {
                    let data = inner_rc.borrow();
                    if data.hwnd.is_null() {
                        return;
                    }
                    data.hwnd
                };
                // Query the parent's CURRENT client rect inside the
                // callback rather than trusting the `(w, h)` lparam
                // value. This is the only reliable source of truth for
                // "where is the bottom of the client area right now":
                //   - At construction time the parent is 0×0, so
                //     lparam-derived values are wrong.
                //   - DPI-aware message loops may re-dispatch WM_SIZE
                //     with cached or stale values during shell
                //     minimise/restore transitions.
                //   - Re-parenting or re-showing the parent can also
                //     leave lparam out of sync with reality.
                // SAFETY: `parent_hwnd` is valid for the lifetime of
                // the frame and the closure. `RECT` is plain data and
                // is initialised by the call.
                let mut client_rect: RECT = unsafe { std::mem::zeroed() };
                let ok = unsafe { GetClientRect(parent_hwnd, &mut client_rect) };
                if ok == 0 {
                    return;
                }
                let cw = client_rect.right - client_rect.left;
                let ch = client_rect.bottom - client_rect.top;
                if cw <= 0 || ch <= 0 {
                    return;
                }
                // SAFETY: `sb_hwnd` is alive (the `Weak` upgrade proved
                // it) and `MoveWindow` is safe to call with any
                // non-zero dimensions inside the parent's client area.
                unsafe {
                    MoveWindow(
                        sb_hwnd,
                        0,
                        ch - STATUS_BAR_HEIGHT,
                        cw,
                        STATUS_BAR_HEIGHT,
                        1, // bRepaint = TRUE
                    );
                    // Bring the status bar to the top of the Z-order so
                    // sibling controls managed by the frame's sizer
                    // (which fills the whole client area and lays out
                    // children edge-to-edge) do not paint over it.
                    SetWindowPos(
                        sb_hwnd,
                        HWND_TOP,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                inner_rc.borrow().apply_field_widths(cw);
                // Re-apply the cached text of every non-empty field.
                //
                // Two reasons this is required:
                //   1. **Stale-cache loss on resize.** Win32's
                //      `msctls_statusbar32` control does not guarantee
                //      that text set via `SB_SETTEXT` survives a later
                //      `SB_SETPARTS` (the parts-resize message we just
                //      sent). If the new part boundaries differ from
                //      the old ones — or even if the same call is just
                //      re-issued at a different parent width — the
                //      control can clear the text in the affected
                //      fields. The text is kept in `state.texts` (a
                //      per-field `String` cache) so we can replay it
                //      here.
                //   2. **Pre-show `set_status_text` calls.** Callers
                //      typically build the bar and set its initial
                //      text *before* the parent frame is shown (the
                //      `aui_toolbar_demo` does exactly this: it calls
                //      `set_status_text` immediately after
                //      `StatusBar::new`, then `app.run` shows the
                //      frame). At that point no `WM_SIZE` has fired,
                //      `apply_field_widths(0)` is a no-op, and the
                //      `SB_SETTEXT` that `set_status_text` sends is
                //      rejected by the control because no parts exist
                //      yet. The `SendMessageW` call returns `FALSE`
                //      and the text is silently lost. Re-applying
                //      here — *after* `apply_field_widths` has
                //      configured the parts — guarantees the text
                //      reaches a properly-partitioned control.
                //
                // Only non-empty cached strings are re-applied, so
                // explicitly-cleared fields stay cleared.
                let cached_texts: Vec<String> = inner_rc.borrow().texts.clone();
                for (i, text) in cached_texts.iter().enumerate() {
                    if text.is_empty() {
                        continue;
                    }
                    let wide = to_wide(text);
                    let wparam = i & 0xFF;
                    // SAFETY: `wide` is alive for the synchronous
                    // `SendMessageW` call, `sb_hwnd` is a valid
                    // status-bar control (proved by the `Weak`
                    // upgrade and the null-check above), and the
                    // `wparam` is the zero-based field index in the
                    // low byte — exactly what `SB_SETTEXTW` expects.
                    // We use the `…W` (UTF-16) variant because the
                    // buffer is a `wchar_t*`; the ANSI variant
                    // truncates the string to its first UTF-16 code
                    // unit (see the constants block above).
                    unsafe {
                        SendMessageW(sb_hwnd, SB_SETTEXTW, wparam, wide.as_ptr() as isize);
                    }
                }
            });
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = frame;
        }
        sb
    }

    /// Set the text in field `i` (zero-based).
    ///
    /// `i` is clamped into the valid range. To clear a field, pass an
    /// empty string.
    pub fn set_status_text(&self, text: &str, i: usize) {
        {
            let mut state = self.inner.borrow_mut();
            if i < state.texts.len() {
                state.texts[i] = text.to_string();
            } else {
                return;
            }
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let state = self.inner.borrow();
            let wide = to_wide(text);
            let wparam = i & 0xFF;
            // Use the `…W` (UTF-16) variant of the message — see
            // the constants block at the top of this file for why
            // the ANSI variant (0x0401) truncates wide strings to
            // their first UTF-16 code unit.
            SendMessageW(state.hwnd, SB_SETTEXTW, wparam, wide.as_ptr() as isize);
        }
    }

    /// Return the text in field `i` (zero-based), or an empty string if
    /// `i` is out of range.
    pub fn get_status_text(&self, i: usize) -> String {
        #[cfg(target_os = "windows")]
        {
            let state = self.inner.borrow();
            if i >= state.fields {
                return String::new();
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let len = unsafe { SendMessageW(state.hwnd, SB_GETTEXTLENGTHW, i, 0) };
            if len <= 0 {
                return state.texts.get(i).cloned().unwrap_or_default();
            }
            let low = (len & 0xFFFF) as usize;
            let mut buf = vec![0u16; low + 1];
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result =
                unsafe { SendMessageW(state.hwnd, SB_GETTEXTW, i, buf.as_mut_ptr() as isize) };
            if result == 0 {
                return state.texts.get(i).cloned().unwrap_or_default();
            }
            let actual = (result & 0xFFFF) as usize;
            String::from_utf16_lossy(&buf[..actual.min(low)])
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner
                .borrow()
                .texts
                .get(i)
                .cloned()
                .unwrap_or_default()
        }
    }

    /// Return the number of fields.
    pub fn get_fields_count(&self) -> usize {
        self.inner.borrow().fields
    }

    /// Return the native window handle.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Hand out a `WidgetRef` (the `Rc<RefCell<dyn Widget>>` used by
    /// sizers) for the underlying status-bar control. Lets you call
    /// the `Widget` trait methods (`set_visible`, `is_visible`,
    /// `set_size`, …) on the status bar from outside this module.
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        self.inner.clone()
    }

    /// Return whether the status bar is currently visible.
    pub fn is_visible(&self) -> bool {
        self.inner.borrow().is_visible()
    }

    /// Show or hide the status bar.
    pub fn set_visible(&self, visible: bool) {
        self.inner.borrow_mut().set_visible(visible);
    }
}

// ── Inherent impl on the inner (no public API surface) ────────────────

#[cfg(target_os = "windows")]
impl StatusBarInner {
    /// Compute field widths and apply them to the control. The widths
    /// are a simple even split of the parent frame's client width, with
    /// the right edge of each field at `total * (i+1) / n` for
    /// `i = 0..n`. The last field's right edge is set to `-1` to tell
    /// Win32 to use the right edge of the control itself.
    ///
    /// `total` is the parent width in pixels; passing `0` (or any
    /// non-positive value) is a no-op, used at construction time when
    /// the parent's client rect is still 0×0.
    fn apply_field_widths(&self, total: i32) {
        if total <= 0 || self.fields == 0 {
            return;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut widths: Vec<i32> = (1..=self.fields)
                .map(|i| (total * (i as i32)) / (self.fields as i32))
                .collect();
            // Last width: -1 to indicate the right edge of the window
            if let Some(last) = widths.last_mut() {
                *last = -1;
            }
            SendMessageW(
                self.hwnd,
                SB_SETPARTS,
                self.fields,
                widths.as_mut_ptr() as isize,
            );
        }
    }
}

// ── Widget trait ───────────────────────────────────────────────────────

impl Widget for StatusBarInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        0
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
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // Status bar has no enabled state.
    }
}
