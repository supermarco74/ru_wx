//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Device context for printing (`wxPrinterDC`).

use crate::core::geometry::Size;
use crate::dc::pen::{Pen, PenStyle};
use crate::core::geometry::Colour;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::HDC;

/// GDI printing device context (`wxPrinterDC`).
pub struct PrinterDC {
    page_size: Size,
    pen: Pen,
    #[cfg(target_os = "windows")]
    hdc: Option<HDC>,
}

impl PrinterDC {
    pub fn new(page_size: Size) -> Self {
        Self {
            page_size,
            pen: Pen::new(Colour::BLACK, 1, PenStyle::Solid),
            #[cfg(target_os = "windows")]
            hdc: None,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn from_hdc(hdc: HDC, page_size: Size) -> Self {
        Self {
            page_size,
            pen: Pen::new(Colour::BLACK, 1, PenStyle::Solid),
            hdc: Some(hdc),
        }
    }

    pub fn page_size(&self) -> Size {
        self.page_size
    }

    pub fn draw_line(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> (i32, i32, i32, i32) {
        #[cfg(target_os = "windows")]
        if let Some(hdc) = self.hdc {
            use windows_sys::Win32::Graphics::Gdi::{
                CreatePen, DeleteObject, LineTo, MoveToEx, PS_SOLID, SelectObject,
            };
            // SAFETY: GDI pen/line drawing on a printer DC.
            unsafe {
                let pen = CreatePen(
                    PS_SOLID,
                    self.pen.width as i32,
                    self.pen.colour.to_colorref(),
                );
                let old = SelectObject(hdc, pen as _);
                let mut old_point = std::mem::zeroed();
                MoveToEx(hdc, x1, y1, &mut old_point);
                LineTo(hdc, x2, y2);
                SelectObject(hdc, old);
                DeleteObject(pen);
            }
        }
        (x1, y1, x2, y2)
    }

    pub fn start_page(&mut self) -> bool {
        #[cfg(target_os = "windows")]
        if let Some(hdc) = self.hdc {
            use windows_sys::Win32::Storage::Xps::StartPage;
            // SAFETY: GDI spooler page start on a printer DC.
            return unsafe { StartPage(hdc) > 0 };
        }
        true
    }

    pub fn end_page(&mut self) -> bool {
        #[cfg(target_os = "windows")]
        if let Some(hdc) = self.hdc {
            use windows_sys::Win32::Storage::Xps::EndPage;
            // SAFETY: GDI spooler page end on a printer DC.
            return unsafe { EndPage(hdc) > 0 };
        }
        true
    }

    #[cfg(target_os = "windows")]
    pub fn hdc(&self) -> Option<HDC> {
        self.hdc
    }
}
