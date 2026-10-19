//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Per-day calendar styling (`wxCalendarDateAttr`).

use crate::controls::date_picker_ctrl::Date;
use crate::core::geometry::Colour;

/// Highlight attributes for a calendar day (`wxCalendarDateAttr`).
#[derive(Debug, Clone, Copy)]
pub struct CalendarDateAttr {
    pub date: Date,
    pub background: Colour,
    pub foreground: Colour,
    pub bold: bool,
}

impl CalendarDateAttr {
    pub fn new(date: Date, background: Colour, foreground: Colour) -> Self {
        Self {
            date,
            background,
            foreground,
            bold: false,
        }
    }

    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
}
