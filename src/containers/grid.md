# grid

`Grid` — wxWidgets-style advanced tabular widget (the ru_wx port of `wxGrid`).

| Platform | Backend | Notes |
|----------|---------|-------|
| **Windows** | Win32 `SysListView32` report view | Full-row select, grid lines, `NM_CUSTOMDRAW`, Explorer / Win11 theme |
| **macOS** | AppKit `NSTableView` (view-based cells) | Native progress bars, priority indicators, multi-select |

## Feature matrix

| Area | Windows | macOS |
|------|---------|-------|
| Columns (add/remove/width/align/order/reorder/autosize) | yes | yes |
| Rows (count/append/delete/sort/scroll/select) | yes | yes |
| Data (`set_cell`, provider, `refresh`) | yes | yes |
| Checkboxes, `ImageList`, row/cell styles, themes | yes | yes |
| Header sort, context menus, truncated-cell tooltips | yes | yes |
| Row/column highlight (reference cell / header click) | yes | yes |
| **Progress / Bar (`Cell::Progress`, `Cell::Bar` Solid)** | Unicode block bar | `NSProgressIndicator` |
| **Bar alternate styles** (Rounded, Square, Dots) | Unicode | Unicode (same glyphs as Windows) |
| **Priority (`Cell::Priority`)** | Unicode 3-block bar | `NSLevelIndicator` (relevancy) |
| **Multi-line (`Cell::MultiLine`)** | `\r` in ListView text | `NSTextField` multiline |
| Row height (`set_row_height`) | approx. via font scaling | exact `NSTableView` row height |
| Win11 Explorer theme (`apply_win11_theme`) | yes | n/a |
| Inline editing (F2 / dbl-click) | `EDIT` overlay | floating `NSTextField` |
| Enter → `on_row_activated` | `LVN_ITEMACTIVATE` | `keyDown` Return |
| Header divider dbl-click → autosize | `HDN_DIVIDERDBLCLICKW` | header `mouseDown` dbl-click |
| Clipboard copy (row / checked / all TSV) | yes | yes |
| Paste TSV (`paste_from_clipboard`, Cmd/Ctrl+V) | yes | yes |
| Export file (`write_tsv_file`) | yes | yes |
| Focused cell (`focused_cell`, `set_focused_cell`) | yes | yes |
| Keyboard navigation (arrows, Tab) | yes | yes |
| Multi-row selection callback | `on_selection_rows_changed` | `on_selection_rows_changed` |
| Hide/show column | yes (width 0) | yes (`NSTableColumn` hidden) |
| Row labels column (`enable_row_labels`) | yes (leading `#` col) | yes (leading `#` col) |

### Not implemented (both platforms)

- Frozen columns / rows
- Merged cells
- Row drag-and-drop reorder
- Built-in column filter / search (use app-level logic or `cust_table_grid`)
- Print / PDF export (use `write_tsv_file` + external tools)
- Full `GridTable` trait integration (model lives in `Grid` + provider today)
- `GridCellRenderer` in live cells (off-screen only)
- Typed editors (`GridCellNumberEditor`, …) — use `on_cell_edited` + validation in app code

## Interactive setup

One call wires sort, highlight, context menus, editing, keyboard navigation, and tooltips:

```rust
grid.enable_interactive_features(&frame);
```

Or enable individually: `enable_header_click_sort`, `enable_row_column_highlight`,
`enable_cell_editing`, `enable_keyboard_navigation`, `enable_row_labels`, …

## Rich cell types

```rust
pub enum Cell {
    Empty,
    Text(String),
    MultiLine(String),
    Image { idx: i32, text: String },
    ImageOnly(i32),
    Progress { value: u32, max: u32, label: Option<String> },
    Bar { value: u32, max: u32, width: usize, style: BarStyle, label: Option<String> },
    Badge { text: String, kind: BadgeKind },
    Number { value: f64, format: NumberFormat },
    DateTime { iso: String, format: GridDateFormat },
    Link { text: String, url: String },
    Priority { kind: PriorityKind },
    Stars { value: u32, max: u32 },
}
```

Sort order respects column type via `ColumnSortKind` (`Auto`, `Text`, `Number`, …).

## Editing & clipboard

```rust
grid.enable_cell_editing(&frame);
grid.on_cell_edited(|logical_row, col, text| {
    // return false to reject the edit
    true
});

grid.copy_row_to_clipboard(display_row);
grid.copy_checked_rows_to_clipboard();
grid.copy_all_to_clipboard();
grid.paste_from_clipboard(start_row, start_col); // returns rows pasted
grid.write_tsv_file(path.as_ref())?;
```

Shortcuts when the grid has focus:

| Key | Action |
|-----|--------|
| F2 | begin edit on focused cell |
| Enter | `on_row_activated` |
| Cmd+V / Ctrl+V | paste TSV at focused cell |
| Arrow keys | move focused cell |
| Tab / Shift+Tab | next / previous column |

`GridCellEditor` tracks active edit state internally; prefer the `Grid` API above.

## Selection

```rust
grid.get_selected_row() -> Option<usize>;          // first selected row
grid.selected_rows() -> Vec<usize>;              // all selected (multi-select on macOS)
grid.on_selection_changed(&frame, |row| { ... }); // debounced single-row callback
grid.on_selection_rows_changed(&frame, |rows| { ... }); // full selection set
```

Display vs logical row indices: after `sort_by_column`, use `logical_row(display_row)`.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

let grid = Grid::new(&frame);
grid.append_column("Name", 160);
grid.append_column("Score", 80);
grid.set_value_provider(|row, col| match col {
    0 => Cell::Text(format!("Row {row}")),
    1 => Cell::Number { value: row as f64 * 10.0, format: NumberFormat::Integer },
    _ => Cell::Empty,
});
grid.set_row_count(50);
grid.enable_interactive_features(&frame);
grid.enable_row_labels(true);
```

See `examples/grid_demo.rs` for the full showcase (photos, badges, bars, sort, themes).

## Win32 notes

- Window class: `SysListView32`, report view, `LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES | …`
- Column widths stored in **logical** (96-DPI) pixels; scaled on `LVM_SETCOLUMNWIDTH`
- Selection debounced in `on_selection_changed` (double `LVN_ITEMCHANGED` per click)
- Optional row-label column inserts at ListView index 0 (`lv_leading_cols`)

## macOS notes

- `NSTableView` inside `NSScrollView`; view-based cells with reuse identifiers
- Sort via `NSSortDescriptor` + native header sort indicators
- Checkbox column, optional row-label column, then data columns
- Inline editor commits on click elsewhere, Return, or Escape

## Cross-references

- [image_list](../dc/image_list.md) — cell icons
- [frame](../window/frame.md) — notify dispatch for selection / sort / edit
- [list_ctrl](list_ctrl.rs) — flat single-column list
- [tree_ctrl](../controls/tree_ctrl.md) — hierarchical data
- [grid_cell_editor](grid_cell_editor.rs) — low-level edit state helper

## Tests

Unit tests for sort comparison live in `grid.rs` (`compare_cells*`). Interactive coverage:
`cargo run --example grid_demo`.
