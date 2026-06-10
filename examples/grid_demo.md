# `grid_demo.rs` — `Grid` advanced table widget demo

## Purpose
Showcases the **`Grid`** widget — a `wxGrid`-style advanced table
supporting per-cell text, images, image+text, and function-driven cell
content. This is the only control in the demo (kept focused).

## Run
```bash
cargo run --example grid_demo
```

## What it shows
- `Grid::new(parent)` + `set_image_list(&images)` — bind icons to the table
- `grid.append_column(label, width_px)` — column definition
- `grid.set_row_count(n)` — row count
- `grid.set_value_provider(|row, col| -> Cell)` — function-based cell data
- `grid.on_selection_changed(&frame, |sel| ...)` — row-click event
- `Cell::{Text, ImageOnly, Image, Empty}` — cell content variants
- `ImageList::new(16, 16)` + `images.add_bitmap(hb)` — 16×16 icon list
- `ru_wx::dc::icon::load_svg_bytes_as_hbitmap(svg, w, h)` — direct SVG → HBITMAP

## Embedded assets
5 SVG icons from `assets/icons/` at compile time (16×16 in this demo):
- `STAR_SVG`, `INFO_SVG`, `FILE_NEW_SVG`, `FOLDER_OPEN_SVG`, `EXIT_SVG`

Indices:
| Idx | Icon         | Meaning              |
|-----|--------------|----------------------|
| 0   | star         | featured / popular   |
| 1   | info         | digital / docs       |
| 2   | file-new     | new release          |
| 3   | folder-open  | project              |
| 4   | exit         | discontinued         |

## Data model
8 static products, each a `(icon_idx, name, category, price, stock, is_popular)` tuple:

```
(0, "Espresso Machine",   "Kitchen",     599.99,  12,  true)
(2, "Project Notebook",   "Stationery",   14.50, 250, false)
(3, "Code Repository Pro", "Software",     49.00, 999,  true)
(1, "API Documentation",  "Digital",       0.00,   0, false)
(3, "Design System Kit",  "Software",     89.00,  43,  true)
(2, "Mechanical Keyboard","Peripherals", 129.00,  18,  true)
(4, "Discontinued Item",  "Archive",      29.99,   0, false)
(0, "Limited Edition Mug","Kitchen",      24.00,  60,  true)
```

## Top-level flow
1. Build a 720×480 frame (re-applies size as a belt-and-suspenders measure).
2. Build a 16×16 `ImageList` and add all 5 SVGs.
3. Create the `Grid`; attach the image list; append 6 columns.
4. Define the closure value provider:
   - col 0 → `Cell::ImageOnly(icon_idx)` — icon-only column
   - col 1-2 → `Cell::Text(name|category)` — plain text
   - col 3 → `Cell::Text(format!("{:.2}", price))` — currency format
   - col 4 → `Cell::Image { idx, text }` — green check / red cross + count
   - col 5 → `Cell::Image { idx, text }` — featured / standard badge
5. `grid.set_row_count(products.len())`.
6. Build a `BoxSizer::vertical` (4 px padding) → grid (proportion 1) + status label.
7. Wire `on_selection_changed` to update the status label with row details.
8. `app.run(frame)`.

## Key APIs exercised
- `Grid::new(&frame)` — 5-column-default; you add columns and rows explicitly.
- `grid.set_image_list(&images)` — bound to LVSIL_SMALL.
- `grid.append_column("Type", 60)` — title + width in pixels.
- `grid.set_row_count(8)` — must be called after the value provider.
- `grid.set_value_provider(F)` where `F: Fn(usize, usize) -> Cell + 'static`.
- `grid.refresh()` — not used here (static data); would be called to
  re-invoke the provider on the visible cells.
- `grid.on_selection_changed(&frame, |sel: Option<usize>| ...)` —
  `Some(row)` when the user selects a row, `None` on clear. Internally
  debounces `LVN_ITEMCHANGED` floods so the closure fires once per
  actual change.
- `grid.row_count()` / `grid.col_count()` — getters.
- `Cell::Text(String)` / `Cell::ImageOnly(i32)` / `Cell::Image { idx, text }` / `Cell::Empty` — content variants.
- `ru_wx::dc::icon::load_svg_bytes_as_hbitmap(svg, w, h) -> Option<HBITMAP>` — Windows-only; on non-Windows targets this is stubbed to `None`.

## Win32 / platform notes
- The `Grid` is implemented on top of `SysListView32` (LVS_REPORT) with
  one image-list slot. Each row maps to a `LVITEMW`; each cell is
  rendered by the owner-drawn `NM_CUSTOMDRAW` handler which invokes
  the value provider.
- `grid.refresh()` walks the visible cell range, calls the provider for
  each `(row, col)`, and invalidates the affected rectangles.
- `LVN_ITEMCHANGED` notifications are debounced by tracking the previous
  `iItem` and only firing `on_selection_changed` on a real change.
- The `add_svg` helper is `#[cfg(target_os = "windows")]`-gated; on other
  targets it's a no-op stub so the file still compiles.

## Cross-references
- See `src/grid.rs` for the `Grid` widget and `Cell` enum
- See `src/image_list.rs` for `add_bitmap`
- See `src/icon.rs` for `load_svg_bytes_as_hbitmap`
- See `src/sizer.rs` and `src/box_sizer` (in `src/sizer.rs`) for sizers
- See `src/static_text.rs` for the status label
