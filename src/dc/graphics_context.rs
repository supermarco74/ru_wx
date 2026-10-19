//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Vector graphics (`wxGraphicsContext`) — wraps GDI drawing helpers.

use crate::core::geometry::{Colour, Point};
use crate::core::widget::Window;
use crate::dc::pen::{Pen, PenStyle};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::HDC;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

/// Platform-neutral graphics context (`wxGraphicsContext`).
pub struct GraphicsContext {
    pen_colour: Colour,
    pen_width: u32,
    #[cfg(target_os = "windows")]
    hdc: Option<HDC>,
    #[cfg(target_os = "windows")]
    release_hwnd: Option<HWND>,
    #[cfg(target_os = "macos")]
    inner: Option<crate::platform::appkit::MacCgDc>,
    #[cfg(target_os = "macos")]
    flush_view: Option<crate::platform::NativeHandle>,
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self {
            pen_colour: Colour::BLACK,
            pen_width: 1,
            #[cfg(target_os = "windows")]
            hdc: None,
            #[cfg(target_os = "windows")]
            release_hwnd: None,
            #[cfg(target_os = "macos")]
            inner: None,
            #[cfg(target_os = "macos")]
            flush_view: None,
        }
    }
}

impl GraphicsContext {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(target_os = "windows")]
    pub fn with_hdc(hdc: HDC) -> Self {
        Self {
            pen_colour: Colour::BLACK,
            pen_width: 1,
            hdc: Some(hdc),
            release_hwnd: None,
        }
    }

    /// Acquire the client-area DC of `parent` for immediate drawing.
    #[cfg(target_os = "windows")]
    pub fn from_window<W: Window>(parent: &W) -> Self {
        let hwnd = crate::platform::window_handle_as_hwnd(parent);
        // SAFETY: `GetDC` returns a valid DC for a live window handle.
        let hdc = unsafe { GetDC(hwnd) };
        Self {
            pen_colour: Colour::BLACK,
            pen_width: 1,
            hdc: Some(hdc),
            release_hwnd: Some(hwnd),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn from_window<W: Window>(parent: &W) -> Self {
        let handle = crate::platform::window_handle(parent);
        let (w, h) = crate::platform::appkit::MacCgDc::client_size(handle);
        let mut inner = crate::platform::appkit::mac_cg_dc_new(w.max(1), h.max(1));
        inner.attach_flush_view(handle);
        Self {
            pen_colour: Colour::BLACK,
            pen_width: 1,
            inner: Some(inner),
            flush_view: Some(handle),
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn from_window<W: Window>(_parent: &W) -> Self {
        Self::new()
    }

    pub fn set_pen(&mut self, colour: Colour, width: u32) {
        self.pen_colour = colour;
        self.pen_width = width;
    }

    pub fn pen(&self) -> Pen {
        Pen::new(self.pen_colour, self.pen_width, PenStyle::Solid)
    }

    pub fn stroke_line(&mut self, from: Point, to: Point) -> (Point, Point) {
        #[cfg(target_os = "windows")]
        if let Some(hdc) = self.hdc {
            use windows_sys::Win32::Graphics::Gdi::{
                CreatePen, DeleteObject, LineTo, MoveToEx, SelectObject, PS_SOLID,
            };
            // SAFETY: GDI line on the bound device context.
            unsafe {
                let pen = CreatePen(
                    PS_SOLID,
                    self.pen_width as i32,
                    self.pen_colour.to_colorref(),
                );
                let old = SelectObject(hdc, pen as _);
                let mut old_point = std::mem::zeroed();
                MoveToEx(hdc, from.x, from.y, &mut old_point);
                LineTo(hdc, to.x, to.y);
                SelectObject(hdc, old);
                DeleteObject(pen);
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(inner) = self.inner.as_mut() {
            let pen = Pen::new(self.pen_colour, self.pen_width, PenStyle::Solid);
            inner.set_pen(Some(&pen));
            inner.draw_line(from.x, from.y, to.x, to.y);
            inner.flush();
        }
        (from, to)
    }
}

#[cfg(target_os = "windows")]
impl Drop for GraphicsContext {
    fn drop(&mut self) {
        if let (Some(hdc), Some(hwnd)) = (self.hdc.take(), self.release_hwnd.take()) {
            // SAFETY: paired `ReleaseDC` for `GetDC` in `from_window`.
            unsafe {
                ReleaseDC(hwnd, hdc);
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for GraphicsContext {
    fn drop(&mut self) {
        if let Some(mut inner) = self.inner.take() {
            inner.flush();
        }
    }
}
