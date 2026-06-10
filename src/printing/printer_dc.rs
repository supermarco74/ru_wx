//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Device context for printing (`wxPrinterDC`).

use crate::core::geometry::Size;
use crate::dc::pen::{Pen, PenStyle};
use crate::core::geometry::Colour;

/// GDI printing device context stub (`wxPrinterDC`).
pub struct PrinterDC {
    page_size: Size,
    pen: Pen,
}

impl PrinterDC {
    pub fn new(page_size: Size) -> Self {
        Self {
            page_size,
            pen: Pen::new(Colour::BLACK, 1, PenStyle::Solid),
        }
    }

    pub fn page_size(&self) -> Size {
        self.page_size
    }

    pub fn draw_line(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> (i32, i32, i32, i32) {
        let _ = self.pen;
        (x1, y1, x2, y2)
    }

    pub fn start_page(&mut self) -> bool {
        true
    }

    pub fn end_page(&mut self) -> bool {
        true
    }
}
