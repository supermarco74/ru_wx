# sizer

`BoxSizer` — the simplest layout primitive. A linear stack of widgets, horizontal or vertical,
with optional padding, stretchable items, fixed-pixel spacers, and proportional sizing.

## When to use

- You need a single row or column of widgets that resizes with the parent.
- You want `padding`, `add_stretch(proportion)`, and `add_spacer(size)` semantics identical to
  wxWidgets' `wxBoxSizer`.

For multi-row/column grids, see [grid_sizer](grid_sizer.md). For grid *cells* in a tabular
widget, see [grid](grid.md).

## Public types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

enum SizerItem {
    Widget { widget: WidgetRef, proportion: u32 },
    Stretch { proportion: u32 },
    FixedSpace { size: i32 },
}

pub struct BoxSizer {
    orientation: Orientation,
    items: Vec<SizerItem>,
    padding: i32,   // default 5
}
```

`SizerItem` is private. Stretch and FixedSpace have no visible type — they're just values passed
to `add_stretch` and `add_spacer`.

## Public API

```rust
impl BoxSizer {
    /// New box sizer. Default padding = 5 px on all sides.
    pub fn new(orientation: Orientation) -> Self;
    /// Convenience: vertical.
    pub fn vertical() -> Self;
    /// Convenience: horizontal.
    pub fn horizontal() -> Self;

    pub fn set_padding(&mut self, padding: i32);
    pub fn padding(&self) -> i32;
    pub fn orientation(&self) -> Orientation;

    /// Append a widget with `proportion = 0` (fixed size).
    pub fn add(&mut self, widget: WidgetRef);

    /// Append a widget with the given proportion. Proportion > 0
    /// means "share extra space" — two widgets with proportion 1
    /// each get half the surplus, 1:2 splits 1/3 vs 2/3, etc.
    pub fn add_with_proportion(&mut self, widget: WidgetRef, proportion: u32);

    /// Append a stretchable empty region. Use to push widgets to
    /// the right / bottom of a sizer.
    pub fn add_stretch(&mut self, proportion: u32);

    /// Reserve `size` pixels of fixed space (no widget). Typical
    /// use: reserve room for a `StatusBar` above the sizer, or
    /// for a fixed-width gap between two groups of widgets.
    pub fn add_spacer(&mut self, size: i32);

    /// Compute and apply the layout. Called from `Frame::on_size`
    /// with the inner client area (`x`, `y`, `width`, `height`).
    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32);
}
```

## Quick start

```rust,no_run
use ru_wx::prelude::*;

let frame = Frame::new("BoxSizer demo", 600, 400);

// 1. Pick an orientation.
let sizer = BoxSizer::vertical();
// or: BoxSizer::new(Orientation::Vertical);
// or: BoxSizer::horizontal();

// 2. (Optional) tighten / loosen the inset between items.
sizer.set_padding(8);

// 3. Add widgets. `add` keeps them at their natural size; the
//    `proportion` is 0 (fixed). Use `add_with_proportion(w, n)`
//    to make the widget share leftover space.
let title = StaticText::new(&frame, "Hello, world!");
let button = Button::new(&frame, "Click me");
sizer.add(title.as_widget_ref());
sizer.add(button.as_widget_ref());

// 4. Push the next widgets to the bottom of the sizer with a
//    stretch (proportion > 0, no widget).
sizer.add_stretch(1);
let footer = StaticText::new(&frame, "status: idle");
sizer.add(footer.as_widget_ref());

// 5. Reserve a fixed-pixel gap (e.g. above a non-sizer sibling like
//    a status bar that the frame lays out separately).
sizer.add_spacer(20);

// 6. Make the sizer the frame's layout manager. From here on the
//    frame calls `sizer.layout(x, y, w, h)` automatically on every
//    `WM_SIZE`.
frame.set_sizer(sizer);
```

**Typical workflow**

1. `BoxSizer::vertical()` (or `.horizontal()`) to pick the axis.
2. `add(widget.as_widget_ref())` for each child at its natural size.
3. `add_with_proportion(widget, n)` to make the child share leftover
   space — `n:u32` is a weight; two `proportion: 1` children get 50/50,
   `1:2` gives 1/3 vs 2/3.
4. `add_stretch(n)` to push subsequent widgets to the far end of the
   sizer (e.g. align a status line to the bottom of a vertical sizer).
5. `add_spacer(px)` to reserve a fixed-pixel gap (e.g. before a
   non-sizer sibling like a `StatusBar` that the frame positions
   separately).
6. `frame.set_sizer(sizer)` to install the sizer. The frame recomputes
   the available area on every resize and calls `sizer.layout`.

**Proportion semantics**

- `proportion = 0` → fixed size (whatever `Widget::rect()` reports).
- `proportion > 0` → share leftover space in proportion to the sum of
  all non-zero proportions. Gaps and fixed-size children are subtracted
  from the available budget *first*; the gap itself never grows.

**Pairing with non-sizer siblings**

If the frame also owns a `StatusBar` (laid out manually by the frame),
leave room for it with `add_spacer(STATUS_BAR_HEIGHT)` at the top or
bottom of the sizer, and let the frame subtract the status bar from
the available area before calling `layout`.

## Layout algorithm

- **Padding** is applied as a uniform inset on all four sides (5 px by default).
- **Widgets with `proportion == 0`** keep the size reported by their `Widget::rect()` (typically
  set at construction).
- **Widgets with `proportion > 0`** split the leftover space after fixed-size items. The share
  each one gets is `proportion / sum_of_proportions`.
- **`add_stretch(n)`** is a phantom widget that takes only proportional space — it shrinks or
  grows but never owns a fixed pixel budget.
- **`add_spacer(n)`** reserves `n` pixels unconditionally. It does not push subsequent
  widgets; it simply consumes `n` from the available width/height. This is the right tool when
  you need to leave room for a non-sizer sibling (e.g. a `StatusBar` at the bottom of the
  frame, computed manually and laid out before the sizer is asked to fill the rest).
- **`x`, `y`** are absolute coordinates inside the parent — typically `(0, 0)` of the frame's
  client area, adjusted for any non-sizer siblings (status bar, menu bar, etc.).

## Win32 notes

- `BoxSizer` itself does no direct Win32 calls. It works by calling each child's
  `set_position` / `set_size`, both of which route to `MoveWindow` on the widget's `HWND`.
- The frame calls `sizer.layout(...)` in its `WM_SIZE` handler, computing the available area
  by subtracting the status bar / menu bar / toolbar heights from the raw client rect.

## Tests

6 unit tests using a `MockWidget` that records `set_position` / `set_size` calls. The tests
pin: single-column full-width behaviour, two-column gap math, multi-row wrap, origin offset,
zero-size safety, and `add_stretch` priority.

## Cross-references

- [frame](../window/frame.md) — owns the sizer, calls `sizer.layout` on `WM_SIZE`.
- [widget](../core/widget.md) — `WidgetRef` and the `Widget` trait that `BoxSizer` calls into.
- [grid_sizer](grid_sizer.md) — the 2D counterpart.
- [status_bar](../chrome/status_bar.md) — non-sizer sibling; pair with `add_spacer(STATUS_BAR_HEIGHT)`.
- [prelude](../prelude.md)
