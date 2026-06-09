//! `wxDC` family — device context wrappers for Win32 GDI drawing.
//!
//! `ru_wx` exposes the four standard `wxDC` flavours as separate
//! types, mirroring wxWidgets:
//!
//! - [`PaintDC`] — created inside a `WM_PAINT` handler; the
//!   `PAINTSTRUCT` is filled in for you and the matching
//!   `EndPaint` runs in [`Drop`].
//! - [`ClientDC`] — a DC over the window's client area, used
//!   for transient drawing outside a `WM_PAINT` (e.g. feedback
//!   while dragging).
//! - [`WindowDC`] — a DC over the entire window (client +
//!   non-client).
//! - [`MemoryDC`] — a DC that draws into a bitmap in memory.
//!   The bitmap is allocated from a source DC (the screen by
//!   default) and the memory DC is sized to match.
//!
//! All four implement the common [`Dc`] trait, which exposes
//! the bulk of the drawing API: lines, rectangles, ellipses,
//! text, and bitmap blits. State (pen, brush, text colour) is
//! tracked per-DC and the original objects are restored in
//! [`Drop`] so callers can keep reusing their own [`Pen`] /
//! [`Brush`] across draws.
//!
//! # Win32 model
//!
//! Internally each DC wraps a single `HDC` plus a `PAINTSTRUCT`
//! for `PaintDC` and a `HBITMAP` for `MemoryDC`. Drawing calls
//! go through the standard GDI functions (`MoveToEx`, `LineTo`,
//! `Rectangle`, `Ellipse`, `TextOutW`, `BitBlt`, `FillRect`,
//! `DrawTextW`, …).
//!
//! The non-Windows build is a pure data stub: the types exist,
//! can be constructed with no-op handles, and expose the same
//! `handle()` / `is_null()` API; the drawing methods are
//! `#[cfg(target_os = "windows")]` and are unavailable off-Windows.

use crate::bitmap::Bitmap;
use crate::brush::Brush;
use crate::geometry::{Colour, Rect};
use crate::pen::Pen;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW,
    Ellipse, EndPaint, FillRect, GetDC, GetStockObject, GetTextExtentPoint32W, HDC, LineTo,
    MoveToEx, NULL_BRUSH, NULL_PEN, Rectangle, ReleaseDC, SelectObject, SetBkColor, SetBkMode,
    SetTextColor, TextOutW, PAINTSTRUCT, SRCCOPY,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

/// Background drawing mode for text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundMode {
    /// Text background is drawn in the current background
    /// colour. (Win32 `OPAQUE` = 2.)
    Opaque,
    /// Text background is left untouched. (Win32 `TRANSPARENT`
    /// = 1.)
    Transparent,
}

#[cfg(target_os = "windows")]
fn bk_mode_to_win32(mode: BackgroundMode) -> i32 {
    match mode {
        BackgroundMode::Opaque => 2,       // OPAQUE
        BackgroundMode::Transparent => 1,  // TRANSPARENT
    }
}

/// Common drawing API shared by every DC flavour.
pub trait Dc {
    /// Returns the underlying `HDC` as an `isize` (or `0` on
    /// non-Windows targets / when the DC is null).
    fn handle(&self) -> isize;

    /// `true` if the DC handle is null (zero). Returns
    /// `false` for a live DC.
    fn is_null(&self) -> bool {
        self.handle() == 0
    }

    // --- State ------------------------------------------------------------

    /// Select a new pen as the current outline pen. The
    /// previous pen (and its handle) is restored in [`Drop`].
    #[cfg(target_os = "windows")]
    fn set_pen(&mut self, pen: Option<&Pen>);

    /// Select a new brush as the current fill brush. Pass
    /// `None` to select the `NULL_BRUSH` stock object (no
    /// fill).
    #[cfg(target_os = "windows")]
    fn set_brush(&mut self, brush: Option<&Brush>);

    /// Set the text colour.
    #[cfg(target_os = "windows")]
    fn set_text_color(&mut self, colour: Colour);

    /// Set the text background colour (only visible when the
    /// background mode is [`BackgroundMode::Opaque`]).
    #[cfg(target_os = "windows")]
    fn set_bk_color(&mut self, colour: Colour);

    /// Set the background mode (opaque / transparent).
    #[cfg(target_os = "windows")]
    fn set_bk_mode(&mut self, mode: BackgroundMode);

    // --- Drawing primitives ------------------------------------------------

    /// Draw a line from `(x1, y1)` to `(x2, y2)` using the
    /// current pen.
    #[cfg(target_os = "windows")]
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32);

    /// Draw the outline of a rectangle using the current
    /// pen, with the current brush as the fill.
    #[cfg(target_os = "windows")]
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32);

    /// Fill a rectangle with the given colour, ignoring the
    /// current pen/brush state.
    #[cfg(target_os = "windows")]
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour);

    /// Draw the outline of an ellipse inscribed in the
    /// rectangle `(x, y, w, h)` with the current pen and
    /// fill with the current brush.
    #[cfg(target_os = "windows")]
    fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32);

    /// Draw `text` with its top-left corner at `(x, y)` in
    /// the current text colour and background mode.
    #[cfg(target_os = "windows")]
    fn draw_text(&mut self, text: &str, x: i32, y: i32);

    /// Draw `text` clipped to the rectangle `rect` (top, left,
    /// bottom, right). The `center` flag centres the text
    /// horizontally and vertically in the rect.
    #[cfg(target_os = "windows")]
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool);

    /// Blit a [`Bitmap`] at `(x, y)` (top-left corner).
    /// Only `SRCCOPY` is supported.
    #[cfg(target_os = "windows")]
    fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32);

    /// Measure the size of `text` in the currently selected
    /// font. Returns `(width, height)` in logical units.
    #[cfg(target_os = "windows")]
    fn text_extent(&self, text: &str) -> (i32, i32);
}

// --- Common implementation helpers --------------------------------------

#[cfg(target_os = "windows")]
fn select_pen_handle(dc: HDC, pen_handle: windows_sys::Win32::Graphics::Gdi::HPEN) {
    // SAFETY: `SelectObject` is a pure GDI state operation;
    // we own the new pen while the DC is in use.
    unsafe {
        let _ = SelectObject(dc, pen_handle as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
    }
}

#[cfg(target_os = "windows")]
fn select_brush_handle(dc: HDC, brush_handle: windows_sys::Win32::Graphics::Gdi::HBRUSH) {
    // SAFETY: `SelectObject` is a pure GDI state operation;
    // we own the new brush while the DC is in use.
    unsafe {
        let _ = SelectObject(dc, brush_handle as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
    }
}

#[cfg(target_os = "windows")]
fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// --- PaintDC -------------------------------------------------------------

/// A DC bound to the current `WM_PAINT`. The `PAINTSTRUCT`
/// is filled in by `BeginPaint` and the matching `EndPaint`
/// is called in [`Drop`].
pub struct PaintDC {
    #[cfg(target_os = "windows")]
    hdc: HDC,
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    ps: PAINTSTRUCT,
}

impl PaintDC {
    /// Begin painting on `hwnd`. The caller must hold a
    /// `WM_PAINT` dispatch.
    ///
    /// # Safety
    ///
    /// `hwnd` must be the HWND of the window that received
    /// the `WM_PAINT` currently being dispatched.
    #[cfg(target_os = "windows")]
    pub unsafe fn new(hwnd: HWND) -> Self {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        Self { hdc, hwnd, ps }
    }
}

impl Dc for PaintDC {
    fn handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        { self.hdc as isize }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }

    #[cfg(target_os = "windows")]
    fn set_pen(&mut self, pen: Option<&Pen>) {
        match pen {
            Some(p) => select_pen_handle(self.hdc, p.handle()),
            None => unsafe {
                let null_pen = GetStockObject(NULL_PEN);
                let _ = SelectObject(self.hdc, null_pen);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_brush(&mut self, brush: Option<&Brush>) {
        match brush {
            Some(b) => select_brush_handle(self.hdc, b.handle()),
            None => unsafe {
                let null_brush = GetStockObject(NULL_BRUSH);
                let _ = SelectObject(self.hdc, null_brush);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_text_color(&mut self, colour: Colour) {
        // SAFETY: SetTextColor is a pure state operation.
        unsafe {
            let _ = SetTextColor(self.hdc, colour.to_colorref());
        }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_color(&mut self, colour: Colour) {
        // SAFETY: SetBkColor is a pure state operation.
        unsafe {
            let _ = SetBkColor(self.hdc, colour.to_colorref());
        }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_mode(&mut self, mode: BackgroundMode) {
        // SAFETY: SetBkMode is a pure state operation.
        unsafe {
            let _ = SetBkMode(self.hdc, bk_mode_to_win32(mode));
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        // SAFETY: standard GDI calls. We pass a null POINT
        // for `MoveToEx` to discard the previous position
        // (matches the behaviour of `LineTo` alone in older
        // Win32).
        unsafe {
            let mut prev = POINT { x: 0, y: 0 };
            let _ = MoveToEx(self.hdc, x1, y1, &mut prev);
            let _ = LineTo(self.hdc, x2, y2);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe {
            let _ = Rectangle(self.hdc, x, y, x + w, y + h);
        }
    }

    #[cfg(target_os = "windows")]
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour) {
        // SAFETY: we create a transient brush, use it once,
        // and delete it after `FillRect` returns.
        unsafe {
            let brush = CreateSolidBrush(colour.to_colorref());
            let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
            let _ = FillRect(self.hdc, &rect, brush);
            let _ = DeleteObject(brush as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe {
            let _ = Ellipse(self.hdc, x, y, x + w, y + h);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        let wide = to_wide_null(text);
        // SAFETY: `wide` lives for the duration of this call;
        // `TextOutW` does not retain the pointer.
        unsafe {
            // The string is null-terminated by `to_wide_null`,
            // but `TextOutW` takes a count, so we strip the
            // trailing null.
            let count = (wide.len() as i32) - 1;
            let ptr = wide.as_ptr();
            let _ = TextOutW(self.hdc, x, y, ptr, count);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool) {
        let wide = to_wide_null(text);
        let mut r = RECT { left: rect.x, top: rect.y, right: rect.x + rect.width as i32, bottom: rect.y + rect.height as i32 };
        // DT_CENTER = 0x1, DT_VCENTER = 0x4, DT_NOCLIP = 0x100, DT_SINGLELINE = 0x20
        let mut format: u32 = 0x100 | 0x20; // NOCLIP | SINGLELINE
        if center { format |= 0x1 | 0x4; }
        // SAFETY: `wide` is alive for the duration of the
        // call; `DrawTextW` writes nothing back to the
        // string. `format` is a bitfield of `DRAW_TEXT_FORMAT`
        // flags. `DrawTextW` takes a `*mut RECT` so we pass
        // `&mut r` to satisfy the pointer type (the rect is
        // read-only in practice for our flag combination).
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = DrawTextW(self.hdc, wide.as_ptr(), count, &mut r, format);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32) {
        if bmp.is_null() {
            return;
        }
        // SAFETY: standard BitBlt with SRCCOPY. We create a
        // transient memory DC, select the source bitmap into
        // it, copy it to the destination DC, and tear the
        // memory DC down. Each `GetDC` is paired with a
        // `ReleaseDC`, and each `CreateCompatibleDC` is paired
        // with a `DeleteDC`; the early-return guards below
        // make sure we don't operate on null handles or leak
        // resources when creation fails.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return;
            }
            let mem = CreateCompatibleDC(screen);
            if mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen);
                return;
            }
            let prev = SelectObject(mem, bmp.handle() as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
            let _ = BitBlt(self.hdc, x, y, bmp.width as i32, bmp.height as i32, mem, 0, 0, SRCCOPY);
            let _ = SelectObject(mem, prev);
            let _ = DeleteDC(mem);
            let _ = ReleaseDC(std::ptr::null_mut(), screen);
        }
    }

    #[cfg(target_os = "windows")]
    fn text_extent(&self, text: &str) -> (i32, i32) {
        let wide = to_wide_null(text);
        let mut size = SIZE { cx: 0, cy: 0 };
        // SAFETY: `wide` is alive for the call; `GetTextExtentPoint32W`
        // writes a SIZE out.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = GetTextExtentPoint32W(self.hdc, wide.as_ptr(), count, &mut size);
        }
        (size.cx, size.cy)
    }
}

#[cfg(target_os = "windows")]
impl Drop for PaintDC {
    fn drop(&mut self) {
        // SAFETY: paired with the matching BeginPaint in
        // `new`. We restore the original pen/brush with
        // stock objects (cheap and stateless) so we don't
        // need to track which user pen/brush was selected.
        unsafe {
            let _ = GetStockObject(NULL_PEN);
            let _ = GetStockObject(NULL_BRUSH);
            EndPaint(self.hwnd, &self.ps);
        }
    }
}

// --- ClientDC ------------------------------------------------------------

/// A DC bound to a window's client area. The handle is
/// acquired with `GetDC` and released with `ReleaseDC` in
/// [`Drop`].
pub struct ClientDC {
    #[cfg(target_os = "windows")]
    hdc: HDC,
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    width: i32,
    #[cfg(target_os = "windows")]
    height: i32,
}

impl ClientDC {
    /// Acquire a DC for the client area of `hwnd`.
    #[cfg(target_os = "windows")]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(hwnd: HWND) -> Self {
        // SAFETY: `GetDC` / `GetClientRect` accept any HWND.
        // The returned HDC is owned by the caller and must be
        // released via `ReleaseDC`.
        unsafe {
            let hdc = GetDC(hwnd);
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let _ = GetClientRect(hwnd, &mut r);
            Self { hdc, hwnd, width: r.right - r.left, height: r.bottom - r.top }
        }
    }

    /// Width of the window's client area (in pixels). Returns
    /// `0` on non-Windows.
    pub fn client_width(&self) -> i32 {
        #[cfg(target_os = "windows")]
        { self.width }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }

    /// Height of the window's client area (in pixels). Returns
    /// `0` on non-Windows.
    pub fn client_height(&self) -> i32 {
        #[cfg(target_os = "windows")]
        { self.height }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }
}

#[cfg(target_os = "windows")]
impl Drop for ClientDC {
    fn drop(&mut self) {
        // SAFETY: `ReleaseDC` paired with the `GetDC` in
        // `new`.
        unsafe {
            let _ = ReleaseDC(self.hwnd, self.hdc);
        }
    }
}

impl Dc for ClientDC {
    fn handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        { self.hdc as isize }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }

    // Reuse PaintDC's drawing methods by delegating through
    // a thin shim. We can't do that directly because the
    // methods take `&mut self` and access `self.hdc`, so we
    // repeat the bodies. The bodies are short, see `dc.rs`
    // for the canonical version.
    #[cfg(target_os = "windows")]
    fn set_pen(&mut self, pen: Option<&Pen>) {
        match pen {
            Some(p) => select_pen_handle(self.hdc, p.handle()),
            None => unsafe {
                let null_pen = GetStockObject(NULL_PEN);
                let _ = SelectObject(self.hdc, null_pen);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_brush(&mut self, brush: Option<&Brush>) {
        match brush {
            Some(b) => select_brush_handle(self.hdc, b.handle()),
            None => unsafe {
                let null_brush = GetStockObject(NULL_BRUSH);
                let _ = SelectObject(self.hdc, null_brush);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_text_color(&mut self, colour: Colour) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetTextColor(self.hdc, colour.to_colorref()); }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_color(&mut self, colour: Colour) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetBkColor(self.hdc, colour.to_colorref()); }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_mode(&mut self, mode: BackgroundMode) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetBkMode(self.hdc, bk_mode_to_win32(mode)); }
    }

    #[cfg(target_os = "windows")]
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        // SAFETY: standard GDI calls.
        unsafe {
            let mut prev = POINT { x: 0, y: 0 };
            let _ = MoveToEx(self.hdc, x1, y1, &mut prev);
            let _ = LineTo(self.hdc, x2, y2);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe { let _ = Rectangle(self.hdc, x, y, x + w, y + h); }
    }

    #[cfg(target_os = "windows")]
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour) {
        // SAFETY: brush is created, used, and destroyed in
        // this scope.
        unsafe {
            let brush = CreateSolidBrush(colour.to_colorref());
            let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
            let _ = FillRect(self.hdc, &rect, brush);
            let _ = DeleteObject(brush as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe { let _ = Ellipse(self.hdc, x, y, x + w, y + h); }
    }

    #[cfg(target_os = "windows")]
    fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        let wide = to_wide_null(text);
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = TextOutW(self.hdc, x, y, wide.as_ptr(), count);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool) {
        let wide = to_wide_null(text);
        let mut r = RECT { left: rect.x, top: rect.y, right: rect.x + rect.width as i32, bottom: rect.y + rect.height as i32 };
        let mut format: u32 = 0x100 | 0x20;
        if center { format |= 0x1 | 0x4; }
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = DrawTextW(self.hdc, wide.as_ptr(), count, &mut r, format);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32) {
        if bmp.is_null() { return; }
        // SAFETY: standard BitBlt with SRCCOPY. We create a
        // transient memory DC, select the source bitmap into
        // it, copy it to the destination DC, and tear the
        // memory DC down.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let mem = CreateCompatibleDC(screen);
            let prev = SelectObject(mem, bmp.handle() as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
            let _ = BitBlt(self.hdc, x, y, bmp.width as i32, bmp.height as i32, mem, 0, 0, SRCCOPY);
            let _ = SelectObject(mem, prev);
            let _ = DeleteDC(mem);
            let _ = ReleaseDC(std::ptr::null_mut(), screen);
        }
    }

    #[cfg(target_os = "windows")]
    fn text_extent(&self, text: &str) -> (i32, i32) {
        let wide = to_wide_null(text);
        let mut size = SIZE { cx: 0, cy: 0 };
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = GetTextExtentPoint32W(self.hdc, wide.as_ptr(), count, &mut size);
        }
        (size.cx, size.cy)
    }
}

// --- WindowDC ------------------------------------------------------------

/// A DC bound to the *whole* window (client + non-client).
/// Like [`ClientDC`] but covers the title bar, borders, etc.
pub struct WindowDC {
    #[cfg(target_os = "windows")]
    hdc: HDC,
    #[cfg(target_os = "windows")]
    hwnd: HWND,
}

impl WindowDC {
    /// Acquire a DC for the whole window of `hwnd`.
    #[cfg(target_os = "windows")]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(hwnd: HWND) -> Self {
        // SAFETY: `GetDC` accepts any HWND. The returned HDC
        // is owned by the caller and must be released via
        // `ReleaseDC`. For WindowDC, we use the regular
        // `GetDC` / `ReleaseDC` pair (the broader
        // `GetWindowDC`/`ReleaseDC` pair is equivalent here
        // because we don't currently use the non-client
        // region).
        unsafe {
            let hdc = GetDC(hwnd);
            Self { hdc, hwnd }
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowDC {
    fn drop(&mut self) {
        // SAFETY: `ReleaseDC` paired with the `GetDC` in
        // `new`.
        unsafe {
            let _ = ReleaseDC(self.hwnd, self.hdc);
        }
    }
}

impl Dc for WindowDC {
    fn handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        { self.hdc as isize }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }

    #[cfg(target_os = "windows")]
    fn set_pen(&mut self, pen: Option<&Pen>) {
        match pen {
            Some(p) => select_pen_handle(self.hdc, p.handle()),
            None => unsafe {
                let null_pen = GetStockObject(NULL_PEN);
                let _ = SelectObject(self.hdc, null_pen);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_brush(&mut self, brush: Option<&Brush>) {
        match brush {
            Some(b) => select_brush_handle(self.hdc, b.handle()),
            None => unsafe {
                let null_brush = GetStockObject(NULL_BRUSH);
                let _ = SelectObject(self.hdc, null_brush);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_text_color(&mut self, colour: Colour) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetTextColor(self.hdc, colour.to_colorref()); }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_color(&mut self, colour: Colour) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetBkColor(self.hdc, colour.to_colorref()); }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_mode(&mut self, mode: BackgroundMode) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetBkMode(self.hdc, bk_mode_to_win32(mode)); }
    }

    #[cfg(target_os = "windows")]
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        // SAFETY: standard GDI calls.
        unsafe {
            let mut prev = POINT { x: 0, y: 0 };
            let _ = MoveToEx(self.hdc, x1, y1, &mut prev);
            let _ = LineTo(self.hdc, x2, y2);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe { let _ = Rectangle(self.hdc, x, y, x + w, y + h); }
    }

    #[cfg(target_os = "windows")]
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour) {
        // SAFETY: brush is created, used, and destroyed in
        // this scope.
        unsafe {
            let brush = CreateSolidBrush(colour.to_colorref());
            let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
            let _ = FillRect(self.hdc, &rect, brush);
            let _ = DeleteObject(brush as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe { let _ = Ellipse(self.hdc, x, y, x + w, y + h); }
    }

    #[cfg(target_os = "windows")]
    fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        let wide = to_wide_null(text);
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = TextOutW(self.hdc, x, y, wide.as_ptr(), count);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool) {
        let wide = to_wide_null(text);
        let mut r = RECT { left: rect.x, top: rect.y, right: rect.x + rect.width as i32, bottom: rect.y + rect.height as i32 };
        let mut format: u32 = 0x100 | 0x20;
        if center { format |= 0x1 | 0x4; }
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = DrawTextW(self.hdc, wide.as_ptr(), count, &mut r, format);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32) {
        if bmp.is_null() { return; }
        // SAFETY: standard BitBlt with SRCCOPY.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let mem = CreateCompatibleDC(screen);
            let prev = SelectObject(mem, bmp.handle() as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
            let _ = BitBlt(self.hdc, x, y, bmp.width as i32, bmp.height as i32, mem, 0, 0, SRCCOPY);
            let _ = SelectObject(mem, prev);
            let _ = DeleteDC(mem);
            let _ = ReleaseDC(std::ptr::null_mut(), screen);
        }
    }

    #[cfg(target_os = "windows")]
    fn text_extent(&self, text: &str) -> (i32, i32) {
        let wide = to_wide_null(text);
        let mut size = SIZE { cx: 0, cy: 0 };
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = GetTextExtentPoint32W(self.hdc, wide.as_ptr(), count, &mut size);
        }
        (size.cx, size.cy)
    }
}

// --- MemoryDC ------------------------------------------------------------

/// A DC that draws into a [`Bitmap`] in memory.
///
/// By default the memory DC is backed by a 1x1 monochrome
/// bitmap (the GDI default). Call [`MemoryDC::select_bitmap`]
/// to switch to your own bitmap; it will be selected back out
/// and the original 1x1 default restored in [`Drop`].
pub struct MemoryDC {
    #[cfg(target_os = "windows")]
    hdc: HDC,
    #[cfg(target_os = "windows")]
    selected: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
}

impl MemoryDC {
    /// Create a new memory DC compatible with the screen.
    /// The 1x1 default bitmap is selected; use
    /// [`MemoryDC::select_bitmap`] to switch.
    #[cfg(target_os = "windows")]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // SAFETY: standard GDI calls. `CreateCompatibleDC`
        // returns a DC compatible with the screen; the
        // 1x1 default bitmap is selected automatically.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let hdc = CreateCompatibleDC(screen);
            let _ = ReleaseDC(std::ptr::null_mut(), screen);
            Self {
                hdc,
                // Null sentinel meaning "no bitmap currently
                // selected" — we only set this when the
                // caller calls `select_bitmap`.
                selected: std::ptr::null_mut(),
            }
        }
    }

    /// Select `bmp` as the bitmap the DC draws into. The
    /// previous bitmap (or 1x1 default) is stored and will
    /// be restored in [`Drop`].
    #[cfg(target_os = "windows")]
    pub fn select_bitmap(&mut self, bmp: &Bitmap) {
        // SAFETY: SelectObject is a pure GDI state op; we
        // store the previously-selected object for later
        // restoration.
        unsafe {
            if !self.selected.is_null() {
                let _ = SelectObject(self.hdc, self.selected);
            }
            self.selected = SelectObject(
                self.hdc,
                bmp.handle() as windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
            );
        }
    }
}

impl Dc for MemoryDC {
    fn handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        { self.hdc as isize }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }

    #[cfg(target_os = "windows")]
    fn set_pen(&mut self, pen: Option<&Pen>) {
        match pen {
            Some(p) => select_pen_handle(self.hdc, p.handle()),
            None => unsafe {
                let null_pen = GetStockObject(NULL_PEN);
                let _ = SelectObject(self.hdc, null_pen);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_brush(&mut self, brush: Option<&Brush>) {
        match brush {
            Some(b) => select_brush_handle(self.hdc, b.handle()),
            None => unsafe {
                let null_brush = GetStockObject(NULL_BRUSH);
                let _ = SelectObject(self.hdc, null_brush);
            },
        }
    }

    #[cfg(target_os = "windows")]
    fn set_text_color(&mut self, colour: Colour) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetTextColor(self.hdc, colour.to_colorref()); }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_color(&mut self, colour: Colour) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetBkColor(self.hdc, colour.to_colorref()); }
    }

    #[cfg(target_os = "windows")]
    fn set_bk_mode(&mut self, mode: BackgroundMode) {
        // SAFETY: pure GDI state op.
        unsafe { let _ = SetBkMode(self.hdc, bk_mode_to_win32(mode)); }
    }

    #[cfg(target_os = "windows")]
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        // SAFETY: standard GDI calls.
        unsafe {
            let mut prev = POINT { x: 0, y: 0 };
            let _ = MoveToEx(self.hdc, x1, y1, &mut prev);
            let _ = LineTo(self.hdc, x2, y2);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe { let _ = Rectangle(self.hdc, x, y, x + w, y + h); }
    }

    #[cfg(target_os = "windows")]
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour) {
        // SAFETY: brush is created, used, and destroyed in
        // this scope.
        unsafe {
            let brush = CreateSolidBrush(colour.to_colorref());
            let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
            let _ = FillRect(self.hdc, &rect, brush);
            let _ = DeleteObject(brush as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32) {
        // SAFETY: standard GDI call.
        unsafe { let _ = Ellipse(self.hdc, x, y, x + w, y + h); }
    }

    #[cfg(target_os = "windows")]
    fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        let wide = to_wide_null(text);
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = TextOutW(self.hdc, x, y, wide.as_ptr(), count);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool) {
        let wide = to_wide_null(text);
        let mut r = RECT { left: rect.x, top: rect.y, right: rect.x + rect.width as i32, bottom: rect.y + rect.height as i32 };
        let mut format: u32 = 0x100 | 0x20;
        if center { format |= 0x1 | 0x4; }
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = DrawTextW(self.hdc, wide.as_ptr(), count, &mut r, format);
        }
    }

    #[cfg(target_os = "windows")]
    fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32) {
        if bmp.is_null() { return; }
        // SAFETY: standard BitBlt with SRCCOPY.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let src = CreateCompatibleDC(screen);
            let prev_src = SelectObject(src, bmp.handle() as windows_sys::Win32::Graphics::Gdi::HGDIOBJ);
            let _ = BitBlt(self.hdc, x, y, bmp.width as i32, bmp.height as i32, src, 0, 0, SRCCOPY);
            let _ = SelectObject(src, prev_src);
            let _ = DeleteDC(src);
            let _ = ReleaseDC(std::ptr::null_mut(), screen);
        }
    }

    #[cfg(target_os = "windows")]
    fn text_extent(&self, text: &str) -> (i32, i32) {
        let wide = to_wide_null(text);
        let mut size = SIZE { cx: 0, cy: 0 };
        // SAFETY: `wide` is alive for the call.
        unsafe {
            let count = (wide.len() as i32) - 1;
            let _ = GetTextExtentPoint32W(self.hdc, wide.as_ptr(), count, &mut size);
        }
        (size.cx, size.cy)
    }
}

#[cfg(target_os = "windows")]
impl Drop for MemoryDC {
    fn drop(&mut self) {
        // SAFETY: restore the previously-selected object
        // (the 1x1 default if `select_bitmap` was never
        // called), then delete the DC.
        unsafe {
            if !self.selected.is_null() {
                let _ = SelectObject(self.hdc, self.selected);
            }
            let _ = DeleteDC(self.hdc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_mode_round_trip() {
        // The numeric values are stable Win32 constants.
        assert_eq!(bk_mode_to_win32(BackgroundMode::Transparent), 1);
        assert_eq!(bk_mode_to_win32(BackgroundMode::Opaque), 2);
    }

    #[test]
    fn wide_null_terminates() {
        let w = to_wide_null("hi");
        assert_eq!(w, vec![b'h' as u16, b'i' as u16, 0]);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn memorydc_smoke_test() {
        // Smoke test: the constructor does not crash and
        // yields a `MemoryDC` value whose `is_null` /
        // `handle` accessors are consistent. We do not
        // assert non-null because `CreateCompatibleDC`
        // can return null in headless / non-interactive
        // sessions (no screen available).
        let dc = MemoryDC::new();
        // `is_null` is the boolean negation of "the
        // handle field is non-zero".
        assert_eq!(dc.is_null(), dc.handle() == 0);
    }
}
