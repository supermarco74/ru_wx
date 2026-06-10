# grid_sizer

Two grid-based layout engines: `GridSizer` (uniform cells) and `FlexGridSizer` (rows/columns
that can grow to fill extra space). Pure layout — no Win32 calls, just measurements and
`set_position` / `set_size` on the children.

## When to use

- You need a uniform table-like grid (`GridSizer`).
- You need a grid where some columns or rows expand with the parent (`FlexGridSizer`) —
  classic "form layout" with labels on the left and growing inputs on the right.

For linear layouts use [sizer](sizer.md) (`BoxSizer`). For tabular data display (rows ×
columns with cells, headers, etc.) use [grid](grid.md).

## Public types

```rust
/// Uniform-cell grid sizer.
pub struct GridSizer {
    cols: u32,
    gap_x: i32,
    gap_y: i32,
    items: Vec<Option<WidgetRef>>,
}

/// Grid sizer where selected rows/columns can grow to absorb
/// extra space; non-growable rows/columns keep their minimum size.
pub struct FlexGridSizer {
    cols: u32,
    gap_x: i32,
    gap_y: i32,
    items: Vec<Option<WidgetRef>>,
    growable_rows: Vec<u32>,
    growable_cols: Vec<u32>,
}
```

## Public API

### `GridSizer`

```rust
impl GridSizer {
    /// New grid sizer. `cols` must be ≥ 1.
    pub fn new(cols: u32, gap_x: i32, gap_y: i32) -> Self;

    /// Append a widget to the next available cell.
    pub fn add(&mut self, widget: WidgetRef);

    /// Append an empty cell (spacer). Other widgets keep their
    /// positions; the spacer just occupies the slot.
    pub fn add_spacer(&mut self);

    /// Apply layout within the given rect.
    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32);
}
```

### `FlexGridSizer`

```rust
impl FlexGridSizer {
    /// New flex grid sizer. `cols` must be ≥ 1.
    pub fn new(cols: u32, gap_x: i32, gap_y: i32) -> Self;

    pub fn add(&mut self, widget: WidgetRef);
    pub fn add_spacer(&mut self);

    /// Mark a row as growable. Duplicate `index` calls are idempotent.
    /// Out-of-range `index` is silently skipped.
    pub fn add_growable_row(&mut self, index: u32);

    /// Mark a column as growable. Same idempotency / bounds rules.
    pub fn add_growable_col(&mut self, index: u32);

    /// Apply layout within the given rect.
    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32);
}
```

## Quick start

Two copy-pasteable patterns: a uniform `GridSizer` for a 3×3 icon grid,
and a `FlexGridSizer` for a classic form layout (label + input per row,
right column grows).

```rust,no_run
use ru_wx::prelude::*;

// --- 1. Uniform 3x3 grid of icon buttons ---------------------------------

fn uniform_grid() -> GridSizer {
    let mut grid = GridSizer::new(3, 4, 4);   // 3 columns, 4px x/y gap

    for i in 0..9 {
        let label = format!("Btn {}", i);
        let btn   = Button::new(&frame(), &label);
        btn.on_click(move || println!("clicked {}", i));
        grid.add(btn.as_widget_ref());
    }
    // Empty slot in the middle of the bottom row — keep other widgets in place.
    // grid.add_spacer();
    grid
}

// --- 2. Flex grid for a form layout (label + input, input grows) ---------

fn form_layout() -> FlexGridSizer {
    let mut form = FlexGridSizer::new(2, 4, 4);    // 2 columns

    // Row 0: label + text input.
    form.add(StaticText::new(&frame(), "Name:").as_widget_ref());
    form.add(TextCtrl::new(&frame(), "").as_widget_ref());
    // Row 1: label + text input.
    form.add(StaticText::new(&frame(), "Email:").as_widget_ref());
    form.add(TextCtrl::new(&frame(), "").as_widget_ref());

    // Make the right column (column 1) growable so the inputs fill width.
    form.add_growable_col(1);

    // Make every row share leftover vertical space equally.
    form.add_growable_row(0);
    form.add_growable_row(1);
    form
}

// --- 3. Drive the layout (typically from a frame's WM_SIZE handler) -----

fn apply_layout(form: &mut FlexGridSizer) {
    form.layout(10, 10, 480, 120);   // x, y, width, height
}

// `frame()` is a helper that returns a long-lived `Frame` in your app.
fn frame() -> Frame { unimplemented!() }
```

**Typical workflow**

1. Build the sizer with `GridSizer::new(cols, gap_x, gap_y)` (uniform
   cells) or `FlexGridSizer::new(cols, gap_x, gap_y)` (growable cells).
   `cols` must be ≥ 1.
2. Append widgets in **row-major order** with `add(widget_ref)`. For
   `GridSizer`, after `cols` items the next one wraps to the next row
   automatically. Use `add_spacer()` to reserve an empty cell.
3. For `FlexGridSizer`, mark growable rows / columns with
   `add_growable_row(index)` / `add_growable_col(index)`. Duplicate calls
   are idempotent; out-of-range indices are silently skipped.
4. Apply the layout by calling `layout(x, y, width, height)` — typically
   from the frame's `WM_SIZE` handler, using the frame's client-area
   size minus any chrome (toolbar, status bar).
5. To rebuild, **mutate the sizer and call `layout` again**. There is no
   "commit" — `layout` is the commit.

**Notes**

- `GridSizer` measures a **single uniform cell size** (container / cols,
  minus gaps) and applies it to every cell. Extra empty space in tall /
  wide containers is not absorbed — it is just blank.
- `FlexGridSizer` measures each widget's min-size, then distributes
  leftover space to the **growable** rows / columns. The gap is *not*
  growable; it is part of the minimum budget.
- `add_spacer()` advances the row-major cursor but does not bind a
  widget. Use it to skip a slot in a `GridSizer`, or to reserve a slot
  in a `FlexGridSizer` (it does not affect min-size measurement).
- Both sizers call each child's `set_position` / `set_size`, which route
  to `MoveWindow` on the widget's `HWND`. There are no other Win32 calls.
- Pure layout: no native handle, no events, no callbacks. Pair with a
  sizer-driven frame (see [sizer.md](sizer.md) and the `Frame::set_sizer`
  pattern).

## Layout algorithm

### `GridSizer::layout`

1. Compute `rows = (items + cols - 1) / cols` (auto-wrap).
2. `cell_width  = (width  - (cols-1) * gap_x) / cols` (clamped to ≥ 0).
3. `cell_height = (height - (rows-1) * gap_y) / rows` (clamped to ≥ 0).
4. For each item at index `i`: `row = i / cols`, `col = i % cols`. Position at
   `(x + col*(cell_w + gap_x), y + row*(cell_h + gap_y))`, size to `(cell_w, cell_h)`.
5. Empty cells (`add_spacer`) do not call `set_position` / `set_size` on any widget — the
   slot is simply not laid out.

### `FlexGridSizer::layout`

1. **Measure**: walk every item, take its `Widget::rect()`, and update
   `col_min_widths[col] = max(col_min_widths[col], w)` and
   `row_min_heights[row] = max(row_min_heights[row], h)`. Empty cells contribute nothing.
2. **Compute totals**: `total_min_width = sum(col_min_widths) + (cols-1)*gap_x`, similarly for
   height.
3. **Distribute extra**: `extra_width = max(0, width - total_min_width)`. If there is at least
   one growable column, `col_extra = extra_width / growable_col_count` is added to each
   growable column's minimum. Same for rows. The **gap is not growable** — it is part of the
   minimum budget and stays constant.
4. **Position**: each cell at `(x + sum(col_min_widths[..col]) + col*gap_x,
   y + sum(row_min_heights[..row]) + row*gap_y)`, sized to the (possibly grown)
   `(col_min_widths[col], row_min_heights[row])`.

## Win32 notes

No direct Win32 calls. The sizers call each child's `set_position` / `set_size`, both of
which route to `MoveWindow` on the widget's `HWND`.

## Tests

13 unit tests per sizer (26 total), all using a `MockWidget` that records
`set_position` / `set_size` calls. The tests pin:

**`GridSizer` (8)**: empty layout safety; single-column full-width behaviour; two-column
gap math; multi-row wrap; origin offset; zero-size safety; clamping when gap exceeds
container; spacer keeps other widgets in place; panic on `cols = 0`.

**`FlexGridSizer` (14)**: empty layout safety; min-size-from-widgets; growable column gets
extra width; growable row gets extra height; multiple growable cols share extra equally;
gaps applied *before* extra distribution (so the gap itself never grows); no growable
leaves extra unused; spacer does not move widgets; duplicate `add_growable_col` is
idempotent; out-of-range growable index is silently skipped (both row and col); origin
offset; panic on `cols = 0`.

## Cross-references

- [sizer](sizer.md) — the linear counterpart. `BoxSizer` for "stack vertically", `GridSizer`
  / `FlexGridSizer` for "arrange in a table".
- [widget](../core/widget.md) — `WidgetRef` and the `Widget` trait.
- [frame](../window/frame.md) — typically owns the sizer and calls `layout` on `WM_SIZE`.
- [grid](grid.md) — *not* a layout engine; it's a data display widget with cells.
- [prelude](../prelude.md)
