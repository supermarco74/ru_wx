//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Vector graphics (`wxGraphicsContext`) — wraps GDI drawing helpers.

use crate::core::geometry::{Colour, Point};
use crate::dc::pen::{Pen, PenStyle};

/// Platform-neutral graphics context (`wxGraphicsContext`).
pub struct GraphicsContext {
    pen_colour: Colour,
    pen_width: u32,
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self {
            pen_colour: Colour::BLACK,
            pen_width: 1,
        }
    }
}

impl GraphicsContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_pen(&mut self, colour: Colour, width: u32) {
        self.pen_colour = colour;
        self.pen_width = width;
    }

    pub fn pen(&self) -> Pen {
        Pen::new(self.pen_colour, self.pen_width, PenStyle::Solid)
    }

    pub fn stroke_line(&self, from: Point, to: Point) -> (Point, Point) {
        (from, to)
    }
}
