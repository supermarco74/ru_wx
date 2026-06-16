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
        let hwnd = parent.hwnd();
        // SAFETY: `GetDC` returns a valid DC for a live window handle.
        let hdc = unsafe { GetDC(hwnd) };
        Self {
            pen_colour: Colour::BLACK,
            pen_width: 1,
            hdc: Some(hdc),
            release_hwnd: Some(hwnd),
        }
    }

    #[cfg(not(target_os = "windows"))]
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

    pub fn stroke_line(&self, from: Point, to: Point) -> (Point, Point) {
        #[cfg(target_os = "windows")]
        if let Some(hdc) = self.hdc {
            use windows_sys::Win32::Graphics::Gdi::{
                CreatePen, DeleteObject, LineTo, MoveToEx, PS_SOLID, SelectObject,
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
