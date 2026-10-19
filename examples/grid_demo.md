# `grid_demo.rs` — `Grid` advanced table widget demo

## Purpose
Showcases the **`Grid`** widget — a `wxGrid`-style advanced table with
function-driven cells, **web product photos**, Lucide icons, themes, and
13 data columns.

## Run
```bash
cargo run --example grid_demo
```

Requires network on first run for product thumbnails (`curl` + picsum.photos).
Offline: Lucide SVG icons (ISC, embedded) are used automatically.

## Assets & licensing (cross-platform)

| Source | Used for | License | macOS-only? |
|--------|----------|---------|-------------|
| **Lucide** SVG (`assets/icons/lucide/`) | Fallback cell icons | ISC | No |
| **Bootstrap Icons** SVG (menu toggle) | Optional icon set | MIT | No |
| **picsum.photos** JPEG | Product photo column | Free placeholder service | No |
| **System UI font** (`FontDesc::new("", 12)`) | Cell text | OS default (SF on Mac, Segoe on Win) | No — not hard-coded to Apple names |
| **Grid / Cell API** | Table logic | ru_wx (GPL) | No |

Nothing in this demo depends on Apple proprietary SDKs beyond the normal
AppKit backend that `ru_wx` already uses on macOS. Fonts are **not** set to
`.SF Pro` or other macOS-specific face names in code.

## Default presentation (v0.6.4+)
- Font: **12 pt** system UI (`Grid::set_font_desc`)
- Row height: **40 px** on macOS (`Grid::set_row_height`)
- Icons: **32×32** px; column **Photo** shows JPEG from picsum or Lucide fallback
- Theme: **`GridAppearance::modern()`** — blue header, slate text, indigo selection, zebra rows

## What it shows
- `GridIcons::lucide_modern(32)` + runtime `load_product_photos()` via `curl`
- `set_value_provider` — all `Cell` variants (Badge, Bar, Stars, Number, DateTime, Link…)
- Checkboxes, column context menu, header sort, selection callback
- Menu **Aspetto** — themes, icon sets, font sizes

## Platform notes

| Feature | Windows | macOS |
|---------|---------|-------|
| Backend | `SysListView32` | `NSTableView` view-based |
| Photos | `ImageList` + PNG/JPEG bytes | `NSImageView` |
| Default row height | ListView default | configurable (`set_row_height`) |

Future Windows roadmap: **WinUI 3** (see `grid.md`).

## Cross-references
- [`grid.rs`](../src/containers/grid.rs) — `Grid`, `Cell`, `GridAppearance`
- [`grid_icons.rs`](../src/containers/grid_icons.rs) — Lucide / Bootstrap sets
- [`cust_table_grid.rs`](cust_table_grid.rs) — full 15-column workbench demo
