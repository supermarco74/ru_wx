//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid layout engines: `GridSizer` and `FlexGridSizer`.
//!
//! `GridSizer` arranges widgets in a uniform grid where every cell has
//! the same size. `FlexGridSizer` allows specified rows and/or columns
//! to grow proportionally when extra space is available.

use crate::core::widget::WidgetRef;

// ---------------------------------------------------------------------------
// GridSizer — uniform cells
// ---------------------------------------------------------------------------

/// A grid sizer with uniform (equal-sized) cells.
///
/// The number of columns is fixed at creation; rows are calculated
/// automatically from the number of items added.
pub struct GridSizer {
    cols: u32,
    gap_x: i32,
    gap_y: i32,
    items: Vec<Option<WidgetRef>>,
}

impl GridSizer {
    /// Create a new grid sizer with the given number of columns.
    ///
    /// `gap_x` / `gap_y` specify horizontal and vertical spacing between cells.
    pub fn new(cols: u32, gap_x: i32, gap_y: i32) -> Self {
        assert!(cols > 0, "GridSizer requires at least 1 column");
        GridSizer {
            cols,
            gap_x,
            gap_y,
            items: Vec::new(),
        }
    }

    /// Add a widget to the next available cell.
    pub fn add(&mut self, widget: WidgetRef) {
        self.items.push(Some(widget));
    }

    /// Add an empty cell (spacer).
    pub fn add_spacer(&mut self) {
        self.items.push(None);
    }

    /// Number of rows, auto-calculated from items and columns.
    fn rows(&self) -> u32 {
        if self.items.is_empty() {
            return 0;
        }
        (self.items.len() as u32).div_ceil(self.cols)
    }

    /// Perform layout within the given bounds.
    ///
    /// Divides the available space equally among all cells and positions
    /// each widget accordingly.
    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let rows = self.rows();
        if rows == 0 || self.cols == 0 {
            return;
        }

        let cell_width = if self.cols > 1 {
            ((width as i32) - (self.cols as i32 - 1) * self.gap_x) / self.cols as i32
        } else {
            width as i32
        };
        let cell_height = if rows > 1 {
            ((height as i32) - (rows as i32 - 1) * self.gap_y) / rows as i32
        } else {
            height as i32
        };

        // Clamp to non-negative
        let cell_width = cell_width.max(0);
        let cell_height = cell_height.max(0);

        for (idx, item) in self.items.iter().enumerate() {
            let row = (idx as u32) / self.cols;
            let col = (idx as u32) % self.cols;

            let cx = x + col as i32 * (cell_width + self.gap_x);
            let cy = y + row as i32 * (cell_height + self.gap_y);

            if let Some(ref widget) = item {
                let mut w = widget.borrow_mut();
                w.set_position(cx, cy);
                w.set_size(cell_width as u32, cell_height as u32);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// FlexGridSizer — flexible rows/columns
// ---------------------------------------------------------------------------

/// A grid sizer where specified rows and/or columns can grow to fill
/// extra space, while non-growable rows/columns keep their minimum size.
pub struct FlexGridSizer {
    cols: u32,
    gap_x: i32,
    gap_y: i32,
    items: Vec<Option<WidgetRef>>,
    growable_rows: Vec<u32>,
    growable_cols: Vec<u32>,
}

impl FlexGridSizer {
    /// Create a new flex grid sizer with the given number of columns.
    pub fn new(cols: u32, gap_x: i32, gap_y: i32) -> Self {
        assert!(cols > 0, "FlexGridSizer requires at least 1 column");
        FlexGridSizer {
            cols,
            gap_x,
            gap_y,
            items: Vec::new(),
            growable_rows: Vec::new(),
            growable_cols: Vec::new(),
        }
    }

    /// Add a widget to the next available cell.
    pub fn add(&mut self, widget: WidgetRef) {
        self.items.push(Some(widget));
    }

    /// Add an empty cell (spacer).
    pub fn add_spacer(&mut self) {
        self.items.push(None);
    }

    /// Mark a row as growable — it will share extra vertical space.
    pub fn add_growable_row(&mut self, index: u32) {
        if !self.growable_rows.contains(&index) {
            self.growable_rows.push(index);
        }
    }

    /// Mark a column as growable — it will share extra horizontal space.
    pub fn add_growable_col(&mut self, index: u32) {
        if !self.growable_cols.contains(&index) {
            self.growable_cols.push(index);
        }
    }

    /// Number of rows, auto-calculated from items and columns.
    fn rows(&self) -> u32 {
        if self.items.is_empty() {
            return 0;
        }
        (self.items.len() as u32).div_ceil(self.cols)
    }

    /// Perform layout within the given bounds.
    ///
    /// 1. Measure the minimum size for each row and column.
    /// 2. Distribute remaining space proportionally among growable rows/cols.
    /// 3. Position each widget.
    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let rows = self.rows();
        if rows == 0 || self.cols == 0 {
            return;
        }

        // Step 1: Measure minimum heights for each row and widths for each column.
        let mut col_min_widths = vec![0i32; self.cols as usize];
        let mut row_min_heights = vec![0i32; rows as usize];

        for (idx, item) in self.items.iter().enumerate() {
            let row = (idx as u32) / self.cols;
            let col = (idx as u32) % self.cols;

            if let Some(ref widget) = item {
                let rect = widget.borrow().rect();
                let w = rect.width as i32;
                let h = rect.height as i32;
                if w > col_min_widths[col as usize] {
                    col_min_widths[col as usize] = w;
                }
                if h > row_min_heights[row as usize] {
                    row_min_heights[row as usize] = h;
                }
            }
        }

        // Step 2: Calculate total minimum space needed (including gaps).
        let total_min_width: i32 =
            col_min_widths.iter().sum::<i32>() + (self.cols as i32 - 1) * self.gap_x;
        let total_min_height: i32 =
            row_min_heights.iter().sum::<i32>() + (rows as i32 - 1) * self.gap_y;

        // Extra space beyond minimums.
        let extra_width = (width as i32 - total_min_width).max(0);
        let extra_height = (height as i32 - total_min_height).max(0);

        // Distribute extra space among growable columns.
        let growable_col_count = self.growable_cols.len() as i32;
        let col_extra = if growable_col_count > 0 && extra_width > 0 {
            extra_width / growable_col_count
        } else {
            0
        };
        for &col in &self.growable_cols {
            if (col as usize) < col_min_widths.len() {
                col_min_widths[col as usize] += col_extra;
            }
        }

        // Distribute extra space among growable rows.
        let growable_row_count = self.growable_rows.len() as i32;
        let row_extra = if growable_row_count > 0 && extra_height > 0 {
            extra_height / growable_row_count
        } else {
            0
        };
        for &row in &self.growable_rows {
            if (row as usize) < row_min_heights.len() {
                row_min_heights[row as usize] += row_extra;
            }
        }

        // Step 3: Position each widget.
        for (idx, item) in self.items.iter().enumerate() {
            let row = (idx as u32) / self.cols;
            let col = (idx as u32) % self.cols;

            // Calculate x position: sum of widths of preceding columns + gaps.
            let cx: i32 =
                x + col_min_widths[..col as usize].iter().sum::<i32>() + col as i32 * self.gap_x;
            // Calculate y position: sum of heights of preceding rows + gaps.
            let cy: i32 =
                y + row_min_heights[..row as usize].iter().sum::<i32>() + row as i32 * self.gap_y;

            let cw = col_min_widths[col as usize].max(0) as u32;
            let ch = row_min_heights[row as usize].max(0) as u32;

            if let Some(ref widget) = item {
                let mut w = widget.borrow_mut();
                w.set_position(cx, cy);
                w.set_size(cw, ch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::Rect;
    use crate::core::widget::Widget;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A minimal `Widget` implementation used only by the grid sizer tests.
    /// On Windows the real widget would route `set_position` / `set_size`
    /// to `MoveWindow`; the mock simply records the most recent call.
    struct MockWidget {
        rect: Rect,
        visible: bool,
        enabled: bool,
    }

    impl MockWidget {
        // Returning a trait object from a `new` constructor would
        // normally trigger `clippy::new_ret_no_self`; silence it
        // because the alternative (a free function or a different
        // name) is awkward in test code.
        #[allow(clippy::new_ret_no_self)]
        fn new(w: u32, h: u32) -> Rc<RefCell<dyn Widget>> {
            Rc::new(RefCell::new(MockWidget {
                rect: Rect::new(0, 0, w, h),
                visible: true,
                enabled: true,
            }))
        }
    }

    impl Widget for MockWidget {
        fn native_handle(&self) -> isize {
            0
        }
        fn set_position(&mut self, x: i32, y: i32) {
            self.rect.x = x;
            self.rect.y = y;
        }
        fn set_size(&mut self, w: u32, h: u32) {
            self.rect.width = w;
            self.rect.height = h;
        }
        fn rect(&self) -> Rect {
            self.rect
        }
        fn is_visible(&self) -> bool {
            self.visible
        }
        fn set_visible(&mut self, visible: bool) {
            self.visible = visible;
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }
    }

    // ---- GridSizer ----

    #[test]
    fn grid_sizer_empty_layout_does_not_panic() {
        let mut sizer = GridSizer::new(2, 0, 0);
        sizer.layout(0, 0, 800, 600);
    }

    #[test]
    fn grid_sizer_single_column_uses_full_width() {
        let a = MockWidget::new(10, 10);
        let b = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(0, 0, 300, 100);

        // Single column -> cell width = full width = 300.
        assert_eq!(a.borrow().rect().width, 300);
        assert_eq!(b.borrow().rect().width, 300);
        // Two cells in one column -> two rows, height = 100 / 2 = 50.
        assert_eq!(a.borrow().rect().height, 50);
        assert_eq!(b.borrow().rect().height, 50);
        // b is in the second row, so y = 0 + 1 * 50 = 50.
        assert_eq!(b.borrow().rect().y, 50);
    }

    #[test]
    fn grid_sizer_two_columns_with_gap() {
        let a = MockWidget::new(10, 10);
        let b = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(2, 10, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(0, 0, 210, 50);

        // (210 - 1 * 10) / 2 = 100 per cell.
        assert_eq!(a.borrow().rect().width, 100);
        assert_eq!(b.borrow().rect().width, 100);
        assert_eq!(a.borrow().rect().x, 0);
        // b.x = 0 + 1 * (100 + 10) = 110.
        assert_eq!(b.borrow().rect().x, 110);
    }

    #[test]
    fn grid_sizer_wraps_to_multiple_rows() {
        let a = MockWidget::new(10, 10);
        let b = MockWidget::new(10, 10);
        let c = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(2, 10, 5);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add(c.clone());

        sizer.layout(0, 0, 210, 70);

        // 3 items / 2 cols = 2 rows.
        // cell_w = (210 - 1 * 10) / 2 = 100.
        // cell_h = (70 - 1 * 5) / 2 = 32.
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(a.borrow().rect().y, 0);
        assert_eq!(a.borrow().rect().width, 100);
        assert_eq!(a.borrow().rect().height, 32);

        assert_eq!(b.borrow().rect().x, 110);
        assert_eq!(b.borrow().rect().y, 0);

        // Third item wraps to row 1, col 0.
        // y = 0 + 1 * (32 + 5) = 37.
        assert_eq!(c.borrow().rect().x, 0);
        assert_eq!(c.borrow().rect().y, 37);
    }

    #[test]
    fn grid_sizer_respects_origin_offset() {
        let a = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(1, 0, 0);
        sizer.add(a.clone());

        sizer.layout(50, 75, 100, 100);

        assert_eq!(a.borrow().rect().x, 50);
        assert_eq!(a.borrow().rect().y, 75);
    }

    #[test]
    fn grid_sizer_zero_size_does_not_panic() {
        let a = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(2, 0, 0);
        sizer.add(a.clone());

        sizer.layout(0, 0, 0, 0);

        // cell_width = (0 - 0) / 2 = 0, max(0) = 0.
        assert_eq!(a.borrow().rect().width, 0);
        assert_eq!(a.borrow().rect().height, 0);
    }

    #[test]
    fn grid_sizer_clamps_to_zero_when_gap_exceeds_size() {
        // 4 items in a 3-column grid with 50px gaps inside a 10x10 box.
        // Both cell_width and cell_height compute to negative values and
        // must be clamped to zero instead of producing negative sizes.
        let a = MockWidget::new(10, 10);
        let b = MockWidget::new(10, 10);
        let c = MockWidget::new(10, 10);
        let d = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(3, 50, 50);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add(c.clone());
        sizer.add(d.clone());

        // 4 items / 3 cols = 2 rows.
        // cell_w = (10 - 2 * 50) / 3 = -30 -> max(0) = 0.
        // cell_h = (10 - 1 * 50) / 2 = -20 -> max(0) = 0.
        sizer.layout(0, 0, 10, 10);

        assert_eq!(a.borrow().rect().width, 0);
        assert_eq!(a.borrow().rect().height, 0);
        assert_eq!(d.borrow().rect().width, 0);
        assert_eq!(d.borrow().rect().height, 0);
    }

    #[test]
    fn grid_sizer_spacer_keeps_other_widgets_in_place() {
        let a = MockWidget::new(10, 10);
        let mut sizer = GridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add_spacer();

        sizer.layout(0, 0, 100, 200);

        // Spacer occupies a cell but does not move `a`.
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(a.borrow().rect().y, 0);
    }

    #[test]
    #[should_panic(expected = "GridSizer requires at least 1 column")]
    fn grid_sizer_panics_on_zero_cols() {
        let _ = GridSizer::new(0, 0, 0);
    }

    // ---- FlexGridSizer ----

    #[test]
    fn flex_grid_sizer_empty_layout_does_not_panic() {
        let mut sizer = FlexGridSizer::new(2, 0, 0);
        sizer.layout(0, 0, 800, 600);
    }

    #[test]
    fn flex_grid_sizer_uses_max_min_size_per_row_and_col() {
        // Two widgets of different sizes in a 1-column sizer.
        // The single column's minimum width must be the max of both
        // widgets' widths; each row's minimum height is the widget's
        // own height.
        let a = MockWidget::new(50, 50);
        let b = MockWidget::new(80, 30);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(0, 0, 500, 500);

        // col_min_widths = [80], row_min_heights = [50, 30].
        // No growable rows or columns, so widgets keep their min size.
        assert_eq!(a.borrow().rect().width, 80);
        assert_eq!(a.borrow().rect().height, 50);
        assert_eq!(b.borrow().rect().width, 80);
        assert_eq!(b.borrow().rect().height, 30);
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(a.borrow().rect().y, 0);
        assert_eq!(b.borrow().rect().x, 0);
        assert_eq!(b.borrow().rect().y, 50);
    }

    #[test]
    fn flex_grid_sizer_growable_col_gets_extra_width() {
        let a = MockWidget::new(50, 50);
        let b = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(2, 0, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add_growable_col(1);

        sizer.layout(0, 0, 300, 100);

        // col_min_widths = [50, 50]; total_min_width = 100.
        // extra_width = 300 - 100 = 200; col_extra = 200 / 1 = 200.
        // col_min_widths[1] = 50 + 200 = 250.
        assert_eq!(a.borrow().rect().width, 50);
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(b.borrow().rect().width, 250);
        // b.x = 0 + 50 + 0 = 50.
        assert_eq!(b.borrow().rect().x, 50);
    }

    #[test]
    fn flex_grid_sizer_growable_row_gets_extra_height() {
        let a = MockWidget::new(50, 50);
        let b = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add_growable_row(1);

        sizer.layout(0, 0, 100, 300);

        // row_min_heights = [50, 50]; total_min_height = 100.
        // extra_height = 300 - 100 = 200; row_extra = 200.
        // row_min_heights[1] = 50 + 200 = 250.
        assert_eq!(a.borrow().rect().height, 50);
        assert_eq!(a.borrow().rect().y, 0);
        assert_eq!(b.borrow().rect().height, 250);
        // b.y = 0 + 50 + 0 = 50.
        assert_eq!(b.borrow().rect().y, 50);
    }

    #[test]
    fn flex_grid_sizer_multiple_growable_cols_share_extra_equally() {
        let a = MockWidget::new(50, 50);
        let b = MockWidget::new(50, 50);
        let c = MockWidget::new(50, 50);
        let d = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(2, 0, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add(c.clone());
        sizer.add(d.clone());
        sizer.add_growable_col(0);
        sizer.add_growable_col(1);

        sizer.layout(0, 0, 500, 100);

        // col_min_widths = [50, 50]; total_min_width = 100.
        // extra_width = 500 - 100 = 400; col_extra = 400 / 2 = 200.
        // col_min_widths[0] = 250, col_min_widths[1] = 250.
        assert_eq!(a.borrow().rect().width, 250);
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(b.borrow().rect().width, 250);
        // b.x = 0 + 250 + 0 = 250.
        assert_eq!(b.borrow().rect().x, 250);
        assert_eq!(c.borrow().rect().width, 250);
        // c is row 1: c.y = 0 + 50 + 0 = 50.
        assert_eq!(c.borrow().rect().y, 50);
        assert_eq!(d.borrow().rect().width, 250);
    }

    #[test]
    fn flex_grid_sizer_gaps_applied_before_extra_distribution() {
        // Gap is part of the minimum-size budget; it must be subtracted
        // from the available space *before* extra is distributed, so the
        // gap itself never grows.
        let a = MockWidget::new(50, 50);
        let b = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(2, 10, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add_growable_col(0);
        sizer.add_growable_col(1);

        sizer.layout(0, 0, 310, 100);

        // col_min_widths = [50, 50]; total_min_width = 50 + 50 + 10 = 110.
        // extra_width = 310 - 110 = 200; col_extra = 200 / 2 = 100.
        // col_min_widths[0] = 150, col_min_widths[1] = 150.
        assert_eq!(a.borrow().rect().width, 150);
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(b.borrow().rect().width, 150);
        // b.x = 0 + 150 + 10 = 160 (width + gap, not width + width).
        assert_eq!(b.borrow().rect().x, 160);
    }

    #[test]
    fn flex_grid_sizer_no_growable_leaves_extra_unused() {
        // With no growable rows or columns the widgets keep their
        // minimum size; leftover space is simply discarded.
        let a = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());

        sizer.layout(0, 0, 1000, 1000);

        // a is the only widget in the only column/row; no growable
        // entries means no redistribution of the 950 extra pixels.
        assert_eq!(a.borrow().rect().width, 50);
        assert_eq!(a.borrow().rect().height, 50);
    }

    #[test]
    fn flex_grid_sizer_spacer_does_not_move_widgets() {
        let a = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add_spacer();

        sizer.layout(0, 0, 100, 100);

        // Spacer occupies a cell, but it has zero size and contributes
        // nothing to the min-size tables, so `a` keeps its position.
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(a.borrow().rect().y, 0);
    }

    #[test]
    fn flex_grid_sizer_duplicate_growable_col_is_idempotent() {
        // add_growable_col deduplicates via `contains`; calling it
        // multiple times for the same column must not double-allocate
        // the extra space.
        let a = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add_growable_col(0);
        sizer.add_growable_col(0);
        sizer.add_growable_col(0);

        sizer.layout(0, 0, 500, 100);

        // col_min_widths = [50]; extra_width = 450.
        // growable_col_count = 1 (deduped); col_extra = 450.
        // a.width = 50 + 450 = 500.
        assert_eq!(a.borrow().rect().width, 500);
    }

    #[test]
    fn flex_grid_sizer_growable_index_out_of_range_is_skipped() {
        // A growable index larger than the column count must not crash;
        // the bounds check inside the redistribution loop simply skips it.
        let a = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(2, 0, 0);
        sizer.add(a.clone());
        sizer.add_growable_col(99);

        sizer.layout(0, 0, 200, 100);

        // a is in col 0 (not growable) so it stays at its minimum width.
        assert_eq!(a.borrow().rect().width, 50);
    }

    #[test]
    fn flex_grid_sizer_growable_row_index_out_of_range_is_skipped() {
        // Same defensive behaviour for `growable_rows`.
        let a = MockWidget::new(50, 50);
        let b = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());
        sizer.add(b.clone());
        sizer.add_growable_row(42);

        sizer.layout(0, 0, 100, 200);

        // Neither row 0 nor row 1 is growable, so the widgets keep
        // their minimum heights.
        assert_eq!(a.borrow().rect().height, 50);
        assert_eq!(b.borrow().rect().height, 50);
    }

    #[test]
    fn flex_grid_sizer_origin_offset_is_applied() {
        let a = MockWidget::new(50, 50);
        let mut sizer = FlexGridSizer::new(1, 0, 0);
        sizer.add(a.clone());

        sizer.layout(20, 30, 100, 100);

        assert_eq!(a.borrow().rect().x, 20);
        assert_eq!(a.borrow().rect().y, 30);
    }

    #[test]
    #[should_panic(expected = "FlexGridSizer requires at least 1 column")]
    fn flex_grid_sizer_panics_on_zero_cols() {
        let _ = FlexGridSizer::new(0, 0, 0);
    }
}
