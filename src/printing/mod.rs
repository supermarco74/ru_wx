//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Printing subsystem (`wxPrintDialog`, `wxPrinter`, …).

pub mod postscript_dc;
pub mod preview_control_bar;
pub mod preview_frame;
pub mod printer_dc;

use crate::core::geometry::Size;

/// Page description for layout (`wxPrintout`).
pub struct Printout {
    pub title: String,
    pub page_count: u32,
}

impl Printout {
    pub fn new(title: &str, page_count: u32) -> Self {
        Self {
            title: title.to_string(),
            page_count,
        }
    }

    pub fn has_page(&self, page: u32) -> bool {
        page > 0 && page <= self.page_count
    }
}

/// Printer facade (`wxPrinter`).
#[derive(Debug, Default)]
pub struct Printer {
    last_error: Option<String>,
}

impl Printer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn print(&mut self, out: &Printout) -> bool {
        if out.page_count == 0 {
            self.last_error = Some("No pages".into());
            return false;
        }
        self.last_error = None;
        true
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

/// Print dialog settings (`wxPrintDialog`).
#[derive(Debug, Clone)]
pub struct PrintDialog {
    pub copies: u32,
    pub collate: bool,
    pub from_page: u32,
    pub to_page: u32,
}

impl Default for PrintDialog {
    fn default() -> Self {
        Self {
            copies: 1,
            collate: false,
            from_page: 1,
            to_page: 1,
        }
    }
}

impl PrintDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_modal(&mut self) -> bool {
        #[cfg(target_os = "windows")]
        {
            use std::mem;
            use windows_sys::Win32::UI::Controls::Dialogs::{
                PrintDlgW, PRINTDLGW, PD_ALLPAGES, PD_RETURNDC,
            };

            // SAFETY: standard Win32 print dialog.
            unsafe {
                let mut pd: PRINTDLGW = mem::zeroed();
                pd.lStructSize = mem::size_of::<PRINTDLGW>() as u32;
                pd.Flags = PD_ALLPAGES | PD_RETURNDC;
                pd.nFromPage = self.from_page as u16;
                pd.nToPage = self.to_page as u16;
                pd.nCopies = self.copies as u16;
                if PrintDlgW(&mut pd) != 0 {
                    self.from_page = pd.nFromPage as u32;
                    self.to_page = pd.nToPage as u32;
                    self.copies = pd.nCopies as u32;
                    return true;
                }
                false
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            true
        }
    }
}

/// Page setup (`wxPageSetupDialog`).
#[derive(Debug, Clone)]
pub struct PageSetupDialog {
    pub paper_size: Size,
    pub margin_mm: u32,
}

impl Default for PageSetupDialog {
    fn default() -> Self {
        Self {
            paper_size: Size::new(2100, 2970),
            margin_mm: 10,
        }
    }
}

impl PageSetupDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_modal(&mut self) -> bool {
        true
    }
}

/// Print preview host (`wxPrintPreview`).
pub struct PrintPreview {
    printout: Printout,
    current_page: u32,
}

impl PrintPreview {
    pub fn new(printout: Printout) -> Self {
        Self {
            printout,
            current_page: 1,
        }
    }

    pub fn current_page(&self) -> u32 {
        self.current_page
    }

    pub fn set_current_page(&mut self, page: u32) {
        if self.printout.has_page(page) {
            self.current_page = page;
        }
    }
}
