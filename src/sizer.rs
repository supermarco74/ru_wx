//! Box-style automatic layout (`wxBoxSizer`).
//!
//! [`BoxSizer`] arranges its children along a single axis, either
//! [`Orientation::Horizontal`] or [`Orientation::Vertical`]. Each
//! child is added with [`BoxSizer::add`] and an optional proportion /
//! flags pair.
//!
//! Layout is driven from [`crate::frame::Frame::set_sizer`]; whenever
//! the frame receives a `WM_SIZE` it calls `sizer.layout(0, 0, w, h)`
//! which moves every child with `MoveWindow`.

use crate::widget::WidgetRef;

/// Orientation for BoxSizer
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// An item in the sizer (either a widget, a stretch spacer, or a fixed-size spacer)
enum SizerItem {
    Widget { widget: WidgetRef, proportion: u32 },
    Stretch { proportion: u32 },
    /// A fixed-size empty space, in pixels along the sizer's main axis.
    /// Use to reserve room for a sibling control that is *not* part of
    /// the sizer (e.g. a `StatusBar` at the bottom of a frame).
    FixedSpace { size: i32 },
}

/// A box sizer that arranges widgets horizontally or vertically
pub struct BoxSizer {
    orientation: Orientation,
    items: Vec<SizerItem>,
    padding: i32,
}

impl BoxSizer {
    pub fn new(orientation: Orientation) -> Self {
        BoxSizer {
            orientation,
            items: Vec::new(),
            padding: 5,
        }
    }

    pub fn vertical() -> Self {
        Self::new(Orientation::Vertical)
    }

    pub fn horizontal() -> Self {
        Self::new(Orientation::Horizontal)
    }

    /// Set padding between items
    pub fn set_padding(&mut self, padding: i32) {
        self.padding = padding;
    }

    /// The inter-item padding in pixels. Defaults to `5` in
    /// [`BoxSizer::new`].
    pub fn padding(&self) -> i32 {
        self.padding
    }

    /// The orientation (horizontal or vertical) of the sizer.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Add a widget with proportion 0 (fixed size)
    pub fn add(&mut self, widget: WidgetRef) {
        self.items.push(SizerItem::Widget {
            widget,
            proportion: 0,
        });
    }

    /// Add a widget with a given proportion (for stretching)
    pub fn add_with_proportion(&mut self, widget: WidgetRef, proportion: u32) {
        self.items.push(SizerItem::Widget { widget, proportion });
    }

    /// Add a stretch spacer
    pub fn add_stretch(&mut self, proportion: u32) {
        self.items.push(SizerItem::Stretch { proportion });
    }

    /// Add a fixed-size spacer (in pixels along the sizer's main axis).
    ///
    /// Useful to reserve room for a sibling control that lives inside
    /// the parent frame's client area but is *not* part of this sizer —
    /// the canonical example being a [`crate::StatusBar`] at the bottom
    /// of a frame, which is positioned at the bottom of the client
    /// area by its own resize handler and therefore needs the sizer
    /// to stop a few pixels short of the bottom edge.
    ///
    /// The spacer has proportion 0 and does not stretch — it is always
    /// exactly `size` pixels wide (for a horizontal sizer) or tall
    /// (for a vertical sizer).
    pub fn add_spacer(&mut self, size: i32) {
        self.items.push(SizerItem::FixedSpace { size: size.max(0) });
    }

    /// Perform layout within the given bounds.
    /// Positions and sizes all child widgets using native MoveWindow calls.
    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32) {
        #[cfg(target_os = "windows")]
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("f:\\code\\ru_wx\\img\\grid_debug.log")
            {
                let _ = writeln!(
                    f,
                    "[sizer] layout ENTRY x={x} y={y} w={width} h={height} items={}",
                    self.items.len()
                );
            }
        }
        if self.items.is_empty() {
            return;
        }

        let total_items = self.items.len();
        let total_padding = self.padding * (total_items as i32 - 1);

        // Calculate total proportion and fixed space
        let mut total_proportion: u32 = 0;
        let mut fixed_size: i32 = 0;

        for item in &self.items {
            match item {
                SizerItem::Widget { widget, proportion } => {
                    if *proportion == 0 {
                        // Use `try_borrow`: a re-entrant call (e.g. a
                        // WndProc of a sibling / child window) may
                        // briefly hold the borrow. If the borrow is
                        // held, treat the widget as having zero
                        // declared size for this pass — the next
                        // layout cycle will retry.
                        if let Ok(w) = widget.try_borrow() {
                            let rect = w.rect();
                            match self.orientation {
                                Orientation::Vertical => fixed_size += rect.height as i32,
                                Orientation::Horizontal => fixed_size += rect.width as i32,
                            }
                        }
                    } else {
                        total_proportion += proportion;
                    }
                }
                SizerItem::Stretch { proportion } => {
                    total_proportion += proportion;
                }
                SizerItem::FixedSpace { size } => {
                    // A fixed-size spacer always reserves its declared
                    // pixel count, regardless of available space.
                    fixed_size += *size;
                }
            }
        }

        // Available space for proportional items
        let available = match self.orientation {
            Orientation::Vertical => (height as i32) - fixed_size - total_padding,
            Orientation::Horizontal => (width as i32) - fixed_size - total_padding,
        };
        let available = available.max(0);

        // Layout each item
        let mut pos = match self.orientation {
            Orientation::Vertical => y,
            Orientation::Horizontal => x,
        };

        for item in &self.items {
            match item {
                SizerItem::Widget { widget, proportion } => {
                    // Use `try_borrow_mut`: a re-entrant call (e.g. a
                    // child widget's WndProc reacting to a Win32 call
                    // we make inside `set_size`) may briefly hold the
                    // borrow. If the borrow is held, skip this widget
                    // for this pass — the next layout cycle
                    // (triggered by the next WM_SIZE / paint) will
                    // retry. This avoids the
                    // "RefCell already borrowed" panic that would
                    // otherwise abort the process during initial
                    // window show, when Windows dispatches a
                    // synchronous chain of WM_SIZE / WM_PAINT
                    // messages while the sizer is still iterating.
                    let Ok(mut w) = widget.try_borrow_mut() else {
                        continue;
                    };
                    let item_size = if *proportion == 0 {
                        match self.orientation {
                            Orientation::Vertical => w.rect().height as i32,
                            Orientation::Horizontal => w.rect().width as i32,
                        }
                    } else {
                        (available as u32 * proportion)
                            .checked_div(total_proportion)
                            .unwrap_or(0) as i32
                    };

                    match self.orientation {
                        Orientation::Vertical => {
                            w.set_position(x, pos);
                            w.set_size(width, item_size as u32);
                        }
                        Orientation::Horizontal => {
                            w.set_position(pos, y);
                            w.set_size(item_size as u32, height);
                        }
                    }

                    #[cfg(target_os = "windows")]
                    {
                        use std::io::Write;
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("f:\\code\\ru_wx\\img\\grid_debug.log")
                        {
                            let _ = writeln!(
                                f,
                                "[sizer]   item size={item_size} pos={pos} w={width} h={}",
                                match self.orientation {
                                    Orientation::Vertical => height,
                                    Orientation::Horizontal => height,
                                }
                            );
                        }
                    }

                    pos += item_size + self.padding;
                }
                SizerItem::Stretch { proportion } => {
                    let stretch_size = (available as u32 * proportion)
                        .checked_div(total_proportion)
                        .unwrap_or(0) as i32;
                    pos += stretch_size + self.padding;
                }
                SizerItem::FixedSpace { size } => {
                    pos += size + self.padding;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::widget::Widget;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A minimal `Widget` implementation used only by the sizer tests.
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

    #[test]
    fn empty_sizer_layout_does_not_panic() {
        let mut sizer = BoxSizer::horizontal();
        sizer.layout(0, 0, 800, 600);
    }

    #[test]
    fn horizontal_sizer_packs_fixed_size_children() {
        // Two 100x20 children + 5px padding ⇒ total 205 wide.
        let a = MockWidget::new(100, 20);
        let b = MockWidget::new(100, 20);
        let mut sizer = BoxSizer::horizontal();
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(0, 0, 800, 30);

        // First child at x=0, second at x=100 + padding 5 = 105.
        assert_eq!(a.borrow().rect().x, 0);
        assert_eq!(a.borrow().rect().width, 100);
        assert_eq!(b.borrow().rect().x, 105);
        assert_eq!(b.borrow().rect().width, 100);
    }

    #[test]
    fn vertical_sizer_packs_fixed_size_children() {
        let a = MockWidget::new(50, 10);
        let b = MockWidget::new(50, 10);
        let mut sizer = BoxSizer::vertical();
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(0, 0, 100, 200);

        assert_eq!(a.borrow().rect().y, 0);
        assert_eq!(a.borrow().rect().height, 10);
        assert_eq!(b.borrow().rect().y, 15);
    }

    #[test]
    fn horizontal_sizer_distributes_proportional_space() {
        // Two children with proportion 1:1 in an 800-wide parent. The
        // sizer subtracts the (n-1) inter-item padding (5) from the
        // available space, leaving 795 px to split 1:1. Integer division
        // gives 397 / 397 = 794, plus the 5 px gap = 799 px consumed.
        let a = MockWidget::new(0, 0);
        let b = MockWidget::new(0, 0);
        let mut sizer = BoxSizer::horizontal();
        sizer.add_with_proportion(a.clone(), 1);
        sizer.add_with_proportion(b.clone(), 1);

        sizer.layout(0, 0, 800, 30);

        let aw = a.borrow().rect().width;
        let bw = b.borrow().rect().width;
        // both should be positive
        assert!(aw > 0, "first child must get non-zero width");
        assert!(bw > 0, "second child must get non-zero width");
        // the two widths must be equal (1:1 proportion)
        assert_eq!(aw, bw, "1:1 proportion ⇒ equal widths");
        // width + gap + width must consume (at most) the parent width
        let consumed = aw + 5 + bw;
        assert!(consumed <= 800, "must not exceed parent width");
        assert!(consumed >= 798, "must consume at least 798 px");
    }

    #[test]
    fn layout_respects_custom_padding() {
        let a = MockWidget::new(10, 10);
        let b = MockWidget::new(10, 10);
        let mut sizer = BoxSizer::horizontal();
        sizer.set_padding(20);
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(0, 0, 800, 30);

        // Second child starts at 10 + 20 padding.
        assert_eq!(b.borrow().rect().x, 30);
    }

    #[test]
    fn vertical_sizer_aligns_children_to_origin_x() {
        // Vertical sizer aligns each child to the same x (the parent's x).
        let a = MockWidget::new(50, 10);
        let b = MockWidget::new(50, 10);
        let mut sizer = BoxSizer::vertical();
        sizer.add(a.clone());
        sizer.add(b.clone());

        sizer.layout(7, 11, 100, 200);

        assert_eq!(a.borrow().rect().x, 7);
        assert_eq!(b.borrow().rect().x, 7);
        assert_eq!(a.borrow().rect().y, 11);
    }
}
