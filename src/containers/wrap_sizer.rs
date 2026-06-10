//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Flow layout with line wrap (`wxWrapSizer`).

use crate::core::widget::WidgetRef;

/// Horizontal flow sizer that wraps to the next row (`wxWrapSizer`).
pub struct WrapSizer {
    gap_x: i32,
    gap_y: i32,
    items: Vec<WidgetRef>,
}

impl WrapSizer {
    pub fn new(gap_x: i32, gap_y: i32) -> Self {
        Self {
            gap_x,
            gap_y,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, widget: WidgetRef) {
        self.items.push(widget);
    }

    pub fn layout(&mut self, x: i32, y: i32, width: u32, _height: u32) {
        let mut cx = x;
        let mut cy = y;
        let mut row_h = 0i32;
        let max_x = x + width as i32;

        for widget in &self.items {
            let rect = widget.borrow().rect();
            let w = rect.width as i32;
            let h = rect.height as i32;
            if cx + w > max_x && cx > x {
                cx = x;
                cy += row_h + self.gap_y;
                row_h = 0;
            }
            let mut wmut = widget.borrow_mut();
            wmut.set_position(cx, cy);
            wmut.set_size(rect.width, rect.height);
            cx += w + self.gap_x;
            row_h = row_h.max(h);
        }
    }
}
