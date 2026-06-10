//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Mouse cursor (`wxCursor` / `wxStockCursor`).

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    LoadCursorW, SetCursor, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_IBEAM, IDC_SIZEALL,
    IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_UPARROW, IDC_WAIT,
};

/// Built-in system cursors (`wxStockCursor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockCursor {
    Arrow,
    Wait,
    IBeam,
    Cross,
    Hand,
    SizeAll,
    SizeNs,
    SizeWe,
    SizeNwse,
    SizeNesw,
    UpArrow,
}

impl StockCursor {
    #[cfg(target_os = "windows")]
    fn id(&self) -> *const u16 {
        match self {
            Self::Arrow => IDC_ARROW,
            Self::Wait => IDC_WAIT,
            Self::IBeam => IDC_IBEAM,
            Self::Cross => IDC_CROSS,
            Self::Hand => IDC_HAND,
            Self::SizeAll => IDC_SIZEALL,
            Self::SizeNs => IDC_SIZENS,
            Self::SizeWe => IDC_SIZEWE,
            Self::SizeNwse => IDC_SIZENWSE,
            Self::SizeNesw => IDC_SIZENESW,
            Self::UpArrow => IDC_UPARROW,
        }
    }

    pub fn set(&self) {
        #[cfg(target_os = "windows")]
        unsafe {
            let h = LoadCursorW(std::ptr::null_mut(), self.id());
            SetCursor(h);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = self;
    }
}

/// Cursor handle (`wxCursor`).
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    stock: StockCursor,
}

impl Cursor {
    pub fn new(stock: StockCursor) -> Self {
        Self { stock }
    }

    pub fn set(&self) {
        self.stock.set();
    }
}
