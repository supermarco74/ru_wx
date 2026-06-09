# grid

`Grid` — wxWidgets-style advanced tabular widget (the ru_wx port of `wxGrid`). Backed by
Win32 `SysListView32` in **report view** with full-row select and grid lines. Each row is a
ListView item, each column is a sub-item, and cells can carry text, an image, or both.

## When to use

- Tabular data display: row × column grid with headers, optional icons, selection.
- Backing the grid with a `Vec<RowData>` or any other data source via the *value provider*
  closure.

For tree-style hierarchies, see [tree_ctrl](tree_ctrl.md). For a simple list of strings, see
[list_box](list_box.md) / [list_ctrl](list_ctrl.rs).

## Public types

```rust
/// The display value of a single cell.
#[derive(Clone, Debug)]
pub enum Cell {
    /// Empty (no text, no image).
    Empty,
    /// Plain text.
    Text(String),
    /// Image + text drawn to the right of the image.
    Image { idx: i32, text: String },
    /// Image only.
    ImageOnly(i32),
}

#[derive(Clone)]
pub struct Grid { /* Rc<RefCell<GridInner>> */ }
```

## Public API

```rust
impl Grid {
    /// Create a new grid as a child of `parent` (any `Window`).
    /// Initial rect: 400×300 at (0, 0). Resize via sizer.
    #[cfg(target_os = "windows")]
    pub fn new<W: Window>(parent: &W) -> Self;

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn new<W: Window>(_parent: &W) -> Self;

    /// Append a column. `title` is the header text, `width` is pixels.
    pub fn append_column(&self, title: &str, width: i32);

    /// Set the number of rows. Wipes any previously-existing rows.
    /// If a value provider is installed, the new rows are populated
    /// from it.
    pub fn set_row_count(&self, n: usize);

    /// Set a single static cell. Ignored if a value provider is
    /// installed (the provider always wins).
    pub fn set_cell(&self, row: usize, col: usize, cell: Cell);

    /// Install a closure that, for every `(row, col)`, produces the
    /// cell value. Provider takes priority over `set_cell`. The
    /// provider is re-queried automatically by `set_row_count` and
    /// `refresh`.
    pub fn set_value_provider<F>(&self, f: F)
    where
        F: Fn(usize, usize) -> Cell + 'static;

    /// Re-query the provider for every cell. No-op if no provider.
    pub fn refresh(&self);

    /// Attach an `ImageList` to the small-icon slot. The grid will
    /// draw the image referenced by `Cell::Image { idx, .. }` or
    /// `Cell::ImageOnly(idx)` to the left of the cell text.
    pub fn set_image_list(&self, list: &ImageList);

    /// Index of the currently selected row, or `None`.
    pub fn get_selected_row(&self) -> Option<usize>;

    /// Register a callback for row-selection changes. Receives the
    /// new selection (`Some(row)` or `None`). Debounced — the
    /// control fires two `LVN_ITEMCHANGED` notifications per click
    /// and we only call the user callback on actual changes.
    pub fn on_selection_changed<F>(&self, frame: &Frame, f: F)
    where
        F: FnMut(Option<usize>) + 'static;

    pub fn row_count(&self) -> usize;
    pub fn col_count(&self) -> usize;
    pub fn id(&self) -> u16;
    pub fn as_widget_ref(&self) -> WidgetRef;
}
```

## Quick start

A complete, copy-pasteable example: a 3-column report grid with 100 rows
populated from a static `Vec`, an `ImageList` for the first column, and
a selection-change callback that prints the new row.

```rust,no_run
use ru_wx::prelude::*;

struct Row { name: String, role: String, active: bool }

fn build_grid(frame: &Frame, rows: Vec<Row>) -> Grid {
    let grid = Grid::new(frame);

    // 1. Column headers + pixel widths.
    grid.append_column("Name",  160);
    grid.append_column("Role",  120);
    grid.append_column("On?",    60);

    // 2. (Optional) small-icon image list for the first column.
    let icons = ImageList::new(16, 16)?;
    // icons.add_icon_from_svg_bytes(include_bytes!("../assets/icons/star.svg"))?;
    grid.set_image_list(&icons);

    // 3. Populate via a value provider (always wins over set_cell).
    grid.set_value_provider(move |row, col| {
        let Some(r) = rows.get(row) else { return Cell::Empty; };
        match col {
            0 => Cell::Image { idx: 0, text: r.name.clone() },
            1 => Cell::Text(r.role.clone()),
            2 => Cell::Text(if r.active { "yes" } else { "no" }.into()),
            _ => Cell::Empty,
        }
    });
    grid.set_row_count(100);    // applies the provider to 100 rows

    // 4. React to selection changes (debounced; the control fires
    //    LVN_ITEMCHANGED twice per click).
    grid.on_selection_changed(frame, |row| {
        match row {
            Some(r) => println!("row {} selected", r),
            None    => println!("selection cleared"),
        }
    });

    // 5. Pass to a sizer that lets it grow with the frame.
    // frame.set_sizer({
    //     let mut s = BoxSizer::vertical();
    //     s.add(grid.as_widget_ref(), 1, SizerFlag::Expand);
    //     s
    // });

    grid
}
```

**Typical workflow**

1. Create the grid with `Grid::new(parent)`. It is a 400×300 child at
   `(0, 0)` — resize it through a sizer, not by direct `MoveWindow`.
2. Define the columns with `append_column(title, width_px)`. Call this
   once per column, in display order, **before** `set_row_count`.
3. (Optional) attach an `ImageList` via `set_image_list(&icons)` so that
   `Cell::Image { idx, text }` and `Cell::ImageOnly(idx)` render the
   corresponding small icon to the left of the cell text.
4. Populate the rows:
   - Static, one-off data: call `set_cell(row, col, cell)` for each
     cell, then `set_row_count(n)`.
   - Backing data behind a `Vec` / database: install a closure via
     `set_value_provider(|row, col| -> Cell)`, then `set_row_count(n)`.
     The provider always wins; it's re-queried on `set_row_count` and
     on `refresh()`.
5. Read the current selection with `get_selected_row() -> Option<usize>`
   and react to changes via `on_selection_changed(frame, |row| ...)`.
6. Call `refresh()` after any external mutation of the data the
   provider reads.

**Notes**

- `set_row_count` **wipes all previously existing rows** before applying
  the provider / `set_cell` values. There's no incremental "add a row"
  API yet — to insert in the middle, rebuild.
- The control fires `LVN_ITEMCHANGED` twice per click; the crate's
  selection callback is debounced and only fires when the value
  actually changed.
- `get_selected_row` returns `None` for an empty selection or for a
  control that hasn't been clicked yet. With `LVS_EX_FULLROWSELECT` the
  *row* is selected, not the individual cell.
- Cross-platform: `Grid::new` is a stub on non-Windows. All other
  methods compile and are safe no-ops on the stub.
- For a flat single-column list, use [list_box](list_box.md) or
  [list_ctrl](list_ctrl.rs). For a hierarchy, use
  [tree_ctrl](tree_ctrl.md).

## Win32 notes

- Window class: `SysListView32`. Styles: `WS_CHILD | WS_VISIBLE | WS_BORDER | LVS_REPORT`
  with `WS_EX_CLIENTEDGE` for the sunken look. Extended styles set via
  `LVM_SETEXTENDEDLISTVIEWSTYLE`: `LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES` (the classic
  report-view look).
- Two local FFI structs — `LVCOLUMNW` (column insert) and `LVITEMW` (item/sub-item write) —
  are declared `#[repr(C)]` to match `<commctrl.h>` exactly. A compile-time
  `const _LVCOLUMNW_SIZE` and a one-shot log to `grid_debug.log` make any struct-layout
  regression visible at process start-up.
- `append_column` does an `LVM_INSERTCOLUMN` with `LVCF_TEXT | LVCF_WIDTH` and then
  **also** a follow-up `LVM_SETCOLUMNWIDTH` for the same column. The follow-up is a
  belt-and-suspenders fix: on Windows 11 25H2 with PerMonitorV2 DPI scaling, the `cx` field
  of `LVCOLUMNW` is silently ignored and every column collapses to ~20 px, truncating
  headers to a single character. The explicit `LVM_SETCOLUMNWIDTH` honours the requested
  width on those systems. A diagnostic read-back (`LVM_GETCOLUMNWIDTH`) is logged to
  `grid_debug.log` so any future regression is visible.
- `set_row_count` does `LVM_DELETEALLITEMS` then `LVM_INSERTITEM` × `n` (with empty items),
  then `apply_cell` is called for every (row, col) to populate the cells from the static
  map or the provider.
- `set_image_list` calls `LVM_SETIMAGELIST(LVSIL_SMALL, list.handle())`.
- `get_selected_row` uses `LVM_GETNEXTITEM(-1, LVNI_SELECTED)`.
- `on_selection_changed` registers a `WM_NOTIFY` handler on the parent `Frame`. The handler:
  1. Re-queries the current selection with `LVM_GETNEXTITEM(-1, LVNI_SELECTED)`.
  2. Compares against the stored `last_selection`; updates it.
  3. Only fires the user callback when the value actually changed (this debounces the
     double-`LVN_ITEMCHANGED` per click).
- `on_selection_changed` is wired on every platform; on non-Windows hosts the inner
  notify handler is never invoked (no real `HWND`), so the callback simply never fires —
  this matches the cross-platform ergonomics of `Frame::set_drop_files_callback`.

## Cross-platform

`new` is a stub on non-Windows targets. All other methods compile and are no-ops on the
stub. Code that builds with `Grid::new(&frame).set_row_count(10).set_value_provider(...)`
will compile everywhere.

## Tests

No unit tests in this module — the control is interactive. Manual coverage via
`examples/grid_demo.rs`.

## Cross-references

- [image_list](image_list.md) — required to display cell icons (`set_image_list`).
- [frame](frame.md) — `Grid::new` accepts any `Window`; the selection callback is delivered
  via the parent `Frame`'s notify dispatch.
- [list_ctrl](list_ctrl.rs) — flat single-column counterpart (no columns, no headers).
- [tree_ctrl](tree_ctrl.md) — hierarchical counterpart.
- [sizer](sizer.md) — typical layout is a single `BoxSizer` containing just the grid (with
  `add_with_proportion(grid_ref, 1)` to make it grow).
- [prelude](prelude.md)
