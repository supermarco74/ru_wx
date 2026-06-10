//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! PostScript device context (`wxPostScriptDC`).

use crate::core::geometry::Size;
use crate::printing::printer_dc::PrinterDC;

/// PostScript output context (`wxPostScriptDC`).
pub struct PostScriptDC {
    inner: PrinterDC,
    output_path: String,
}

impl PostScriptDC {
    pub fn new(page_size: Size, output_path: &str) -> Self {
        Self {
            inner: PrinterDC::new(page_size),
            output_path: output_path.to_string(),
        }
    }

    pub fn page_size(&self) -> Size {
        self.inner.page_size()
    }

    pub fn output_path(&self) -> &str {
        &self.output_path
    }

    pub fn begin_document(&mut self) -> bool {
        self.inner.start_page()
    }

    pub fn end_document(&mut self) -> bool {
        self.inner.end_page()
    }
}
