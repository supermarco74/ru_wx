//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Picker control events (`wxDateEvent`, …).

use crate::controls::date_picker_ctrl::Date;
use crate::core::geometry::Colour;

/// Date picker changed (`wxDateEvent`).
#[derive(Debug, Clone, Copy)]
pub struct DatePickerEvent {
    pub date: Option<Date>,
}

impl DatePickerEvent {
    pub const fn new(date: Option<Date>) -> Self {
        Self { date }
    }
}

/// Colour picker changed (`wxColourPickerEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourPickerEvent {
    pub colour: Colour,
}

impl ColourPickerEvent {
    pub const fn new(colour: Colour) -> Self {
        Self { colour }
    }
}

/// File picker changed (`wxFilePickerEvent`).
#[derive(Debug, Clone)]
pub struct FilePickerEvent {
    pub path: String,
}

impl FilePickerEvent {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// Directory picker changed (`wxDirPickerEvent`).
#[derive(Debug, Clone)]
pub struct DirPickerEvent {
    pub path: String,
}

impl DirPickerEvent {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// Font picker changed (`wxFontPickerEvent`).
#[derive(Debug, Clone)]
pub struct FontPickerEvent {
    pub family: String,
    pub point_size: i32,
}

impl FontPickerEvent {
    pub fn new(family: impl Into<String>, point_size: i32) -> Self {
        Self {
            family: family.into(),
            point_size,
        }
    }
}
