//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Double-precision geometry (`wxPoint2DDouble`, `wxRect2DDouble`, `wxSize2DDouble`).

/// Floating-point point (`wxPoint2DDouble`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Floating-point rectangle (`wxRect2DDouble`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect2D {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Floating-point size (`wxSize2DDouble`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size2D {
    pub width: f64,
    pub height: f64,
}

impl Size2D {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn area(self) -> f64 {
        self.width * self.height
    }
}

impl Rect2D {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Point2D) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    pub fn centre(&self) -> Point2D {
        Point2D::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}
