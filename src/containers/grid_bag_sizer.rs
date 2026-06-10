//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid bag layout (`wxGridBagSizer`).

use crate::core::widget::WidgetRef;

/// Placement of one item in a [`GridBagSizer`].
#[derive(Debug, Clone)]
pub struct GridBagPosition {
    pub col: u32,
    pub row: u32,
    pub colspan: u32,
    pub rowspan: u32,
}

impl GridBagPosition {
    pub const fn new(col: u32, row: u32) -> Self {
        Self {
            col,
            row,
            colspan: 1,
            rowspan: 1,
        }
    }

    pub fn with_span(mut self, colspan: u32, rowspan: u32) -> Self {
        self.colspan = colspan.max(1);
        self.rowspan = rowspan.max(1);
        self
    }
}

struct GridBagItem {
    widget: WidgetRef,
    pos: GridBagPosition,
}

/// Sizer with per-cell spans (`wxGridBagSizer`).
pub struct GridBagSizer {
    gap_x: i32,
    gap_y: i32,
    items: Vec<GridBagItem>,
}

impl GridBagSizer {
    pub fn new(gap_x: i32, gap_y: i32) -> Self {
        Self {
            gap_x,
            gap_y,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, widget: WidgetRef, pos: GridBagPosition) {
        self.items.push(GridBagItem { widget, pos });
    }

    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32) {
        if self.items.is_empty() {
            return;
        }
        let max_col = self
            .items
            .iter()
            .map(|i| i.pos.col + i.pos.colspan)
            .max()
            .unwrap_or(1);
        let max_row = self
            .items
            .iter()
            .map(|i| i.pos.row + i.pos.rowspan)
            .max()
            .unwrap_or(1);
        let cell_w = ((width as i32 - (max_col as i32 - 1) * self.gap_x) / max_col as i32).max(1);
        let cell_h = ((height as i32 - (max_row as i32 - 1) * self.gap_y) / max_row as i32).max(1);

        for item in &self.items {
            let px = x + (item.pos.col as i32) * (cell_w + self.gap_x);
            let py = y + (item.pos.row as i32) * (cell_h + self.gap_y);
            let pw = item.pos.colspan as i32 * cell_w + (item.pos.colspan as i32 - 1) * self.gap_x;
            let ph = item.pos.rowspan as i32 * cell_h + (item.pos.rowspan as i32 - 1) * self.gap_y;
            let mut w = item.widget.borrow_mut();
            w.set_position(px, py);
            w.set_size(pw.max(1) as u32, ph.max(1) as u32);
        }
    }
}
