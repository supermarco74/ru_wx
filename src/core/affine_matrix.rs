//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! 2-D affine transforms (`wxAffineMatrix2D`).

use crate::core::geometry::Point;

/// 2×3 affine matrix: | m11 m12 m13 |
///                    | m21 m22 m23 |
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffineMatrix2D {
    pub m11: f64,
    pub m12: f64,
    pub m13: f64,
    pub m21: f64,
    pub m22: f64,
    pub m23: f64,
}

impl Default for AffineMatrix2D {
    fn default() -> Self {
        Self::identity()
    }
}

impl AffineMatrix2D {
    pub const fn identity() -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m13: 0.0,
            m21: 0.0,
            m22: 1.0,
            m23: 0.0,
        }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.m13 += dx;
        self.m23 += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.m11 *= sx;
        self.m12 *= sy;
        self.m21 *= sx;
        self.m22 *= sy;
        self.m13 *= sx;
        self.m23 *= sy;
    }

    pub fn transform_point(&self, p: Point) -> Point {
        let x = self.m11 * p.x as f64 + self.m12 * p.y as f64 + self.m13;
        let y = self.m21 * p.x as f64 + self.m22 * p.y as f64 + self.m23;
        Point::new(x.round() as i32, y.round() as i32)
    }

    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            m11: self.m11 * other.m11 + self.m12 * other.m21,
            m12: self.m11 * other.m12 + self.m12 * other.m22,
            m13: self.m11 * other.m13 + self.m12 * other.m23 + self.m13,
            m21: self.m21 * other.m11 + self.m22 * other.m21,
            m22: self.m21 * other.m12 + self.m22 * other.m22,
            m23: self.m21 * other.m13 + self.m22 * other.m23 + self.m23,
        }
    }
}
