//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Drag feedback image (`wxDragImage`).

use crate::core::geometry::Point;
use crate::dc::bitmap::Bitmap;

/// Semi-transparent bitmap shown during drag (`wxDragImage`).
#[derive(Debug, Clone)]
pub struct DragImage {
    pub bitmap: Bitmap,
    pub hotspot: Point,
    pub visible: bool,
}

impl DragImage {
    pub fn new(bitmap: Bitmap) -> Self {
        Self {
            bitmap,
            hotspot: Point::new(0, 0),
            visible: false,
        }
    }

    pub fn with_hotspot(mut self, x: i32, y: i32) -> Self {
        self.hotspot = Point::new(x, y);
        self
    }

    pub fn show(&mut self, x: i32, y: i32) {
        self.hotspot = Point::new(x, y);
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.hotspot = Point::new(x, y);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}
