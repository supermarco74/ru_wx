//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Extended property-grid editors (round 15).

use crate::adv::property_grid::{PropertyGrid, PropertyValue};
use crate::core::font::FontDesc;
use crate::core::geometry::Colour;

/// Colour property helper (`wxColourProperty`).
#[derive(Debug, Clone)]
pub struct ColourProperty {
    pub colour: Colour,
}

impl ColourProperty {
    pub fn new(colour: Colour) -> Self {
        Self { colour }
    }

    pub fn to_value(&self) -> PropertyValue {
        PropertyValue::String(format!(
            "#{:02x}{:02x}{:02x}",
            self.colour.r, self.colour.g, self.colour.b
        ))
    }
}

/// Font property helper (`wxFontProperty`).
#[derive(Debug, Clone)]
pub struct FontProperty {
    pub desc: FontDesc,
}

impl FontProperty {
    pub fn new(desc: FontDesc) -> Self {
        Self { desc }
    }

    pub fn to_value(&self) -> PropertyValue {
        PropertyValue::String(self.desc.face_name.clone())
    }
}

/// Named category header (`wxPropertyCategory`).
#[derive(Debug, Clone)]
pub struct PropertyCategory {
    pub label: String,
}

impl PropertyCategory {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

/// Bottom help strip (`wxPropertyGrid` help text).
#[derive(Debug, Default, Clone)]
pub struct PropertyHelpStrip {
    pub text: String,
}

impl PropertyHelpStrip {
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
}

/// Column splitter position between name/value columns.
#[derive(Debug, Clone, Copy)]
pub struct PropertyColumnSplitter {
    pub position_px: u32,
}

impl PropertyColumnSplitter {
    pub const fn new(position_px: u32) -> Self {
        Self { position_px }
    }
}

/// Convenience helpers for [`PropertyGrid`].
pub trait PropertyGridExtras {
    fn append_category(&mut self, category: &PropertyCategory);
    fn append_colour(&mut self, name: &str, prop: &ColourProperty);
    fn append_font(&mut self, name: &str, prop: &FontProperty);
    fn set_help_strip(&mut self, strip: &PropertyHelpStrip);
    fn set_column_split(&mut self, splitter: PropertyColumnSplitter);
}

impl PropertyGridExtras for PropertyGrid {
    fn append_category(&mut self, category: &PropertyCategory) {
        self.append(&format!("— {} —", category.label), PropertyValue::String(String::new()));
    }

    fn append_colour(&mut self, name: &str, prop: &ColourProperty) {
        self.append(name, prop.to_value());
    }

    fn append_font(&mut self, name: &str, prop: &FontProperty) {
        self.append(name, prop.to_value());
    }

    fn set_help_strip(&mut self, strip: &PropertyHelpStrip) {
        let _ = strip;
    }

    fn set_column_split(&mut self, splitter: PropertyColumnSplitter) {
        let _ = splitter;
    }
}
