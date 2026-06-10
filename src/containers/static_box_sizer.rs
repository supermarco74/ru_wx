//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Static box sizer (`wxStaticBoxSizer`).

use crate::containers::sizer::{BoxSizer, Orientation};
use crate::core::widget::WidgetRef;

/// Vertical sizer framed by a [`crate::StaticBox`] (`wxStaticBoxSizer`).
pub struct StaticBoxSizer {
    box_widget: WidgetRef,
    inner: BoxSizer,
    inset: i32,
}

impl StaticBoxSizer {
    pub fn new(box_widget: WidgetRef) -> Self {
        Self {
            box_widget,
            inner: BoxSizer::vertical(),
            inset: 8,
        }
    }

    pub fn set_inset(&mut self, inset: i32) {
        self.inset = inset.max(0);
    }

    pub fn add(&mut self, widget: WidgetRef) {
        self.inner.add(widget);
    }

    pub fn add_with_proportion(&mut self, widget: WidgetRef, proportion: u32) {
        self.inner.add_with_proportion(widget, proportion);
    }

    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32) {
        if let Ok(mut w) = self.box_widget.try_borrow_mut() {
            w.set_position(x, y);
            w.set_size(width, height);
        }
        let ix = x + self.inset;
        let iy = y + self.inset;
        let iw = width.saturating_sub((self.inset * 2) as u32);
        let ih = height.saturating_sub((self.inset * 2) as u32);
        self.inner.layout(ix, iy, iw, ih);
    }

    pub fn orientation(&self) -> Orientation {
        self.inner.orientation()
    }
}
