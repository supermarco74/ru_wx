//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Mirrored device context (`wxMirrorDC`).

use crate::dc::dc::Dc;

/// Horizontal mirror wrapper (`wxMirrorDC`).
pub struct MirrorDC<'a, D: Dc + ?Sized> {
    inner: &'a mut D,
    mirror_x: bool,
}

impl<'a, D: Dc + ?Sized> MirrorDC<'a, D> {
    pub fn new(inner: &'a mut D, mirror_x: bool) -> Self {
        Self { inner, mirror_x }
    }

    pub fn is_mirrored(&self) -> bool {
        self.mirror_x
    }

    pub fn inner_mut(&mut self) -> &mut D {
        self.inner
    }
}
