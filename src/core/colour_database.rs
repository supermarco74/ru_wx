//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Named colours (`wxColourDatabase`).

use crate::core::geometry::Colour;

/// Named colour lookup (`wxColourDatabase`).
#[derive(Debug, Default)]
pub struct ColourDatabase;

impl ColourDatabase {
    pub fn new() -> Self {
        Self
    }

    pub fn find(&self, name: &str) -> Option<Colour> {
        let key = name.to_ascii_lowercase();
        NAMED_COLOURS.iter().find(|(n, _)| *n == key).map(|(_, c)| *c)
    }

    pub fn find_name(&self, colour: Colour) -> Option<String> {
        NAMED_COLOURS
            .iter()
            .find(|(_, c)| *c == colour)
            .map(|(n, _)| n.to_string())
    }

    pub fn names(&self) -> Vec<&'static str> {
        NAMED_COLOURS.iter().map(|(n, _)| *n).collect()
    }
}

const NAMED_COLOURS: &[(&str, Colour)] = &[
    ("black", Colour::new(0, 0, 0, 255)),
    ("white", Colour::new(255, 255, 255, 255)),
    ("red", Colour::new(255, 0, 0, 255)),
    ("green", Colour::new(0, 128, 0, 255)),
    ("blue", Colour::new(0, 0, 255, 255)),
    ("yellow", Colour::new(255, 255, 0, 255)),
    ("grey", Colour::LIGHT_GREY),
    ("lightgray", Colour::LIGHT_GREY),
    ("darkgrey", Colour::new(128, 128, 128, 255)),
];
