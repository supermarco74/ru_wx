//! Demo: the advanced `Grid` widget — `wxGrid`-style table with
//! images, function-based cells, multiple **column alignments**,
//! **checkboxes**, **progress bars**, **status badges**, **multi-line
//! text**, **stars**, **priority bars** and a variety of **numeric
//! and date formats**.
//!
//! This example is the showcase for the "advanced table" that was
//! missing from `ru_wx` until now. It pulls together every feature
//! exposed by the `Grid` widget:
//!
//! 1. An `ImageList` populated from SVG files at compile time, so the
//!    grid can render small icons next to cell text.
//! 2. A closure-based **value provider** (`set_value_provider`) that
//!    maps `(row, col) -> Cell`. This is the "function cell": every
//!    cell in the table is generated on demand by a single function,
//!    so updating the underlying data and re-running
//!    `grid.refresh()` is enough to repaint the whole table.
//! 3. Per-column **alignment** (left / center / right) so numeric
//!    values line up nicely.
//! 4. **Checkboxes** (`set_checkboxes`) for row selection /
//!    batch-operations.
//! 5. **Progress bars** rendered as a `Cell::Progress` Unicode block
//!    bar (no custom drawing, the ListView just sees a `Text` cell).
//! 6. **Configurable bar styles** (`Cell::Bar` with `BarStyle`):
//!    `Solid` (`█/░`), `Rounded` (`▰/▱`), `Square` (`■/□`),
//!    `Dots` (`●/○`). Demonstrated side-by-side in the discount
//!    column so the different visual feels are obvious.
//! 7. **Status badges** (`Cell::Badge` with `BadgeKind`): a leading
//!    indicator character + text, e.g. `● OK`, `▲ Low`, `■ Featured`,
//!    `✕ Sold out`, `○ Pending`. Replaces plain text in the status
//!    column for a much higher signal-to-noise ratio.
//! 8. **Multi-line text** cells (`Cell::MultiLine`).
//! 9. **Numeric formats** (`Cell::Number` with `NumberFormat`):
//!    `Plain`, `WithThousands`, `WithThousandsEu` (European dot
//!    thousands + comma decimal), `Integer`, `Fixed2`, `Percent`,
//!    `CurrencyEuro` (`€ 1.234,50`), `CurrencyDollar`. Two columns
//!    show the different formats side-by-side.
//! 10. **Date formats** (`Cell::DateTime` with `GridDateFormat`):
//!     `Iso`, `Eu` (day-first), `Us` (month-first), `Long`
//!     (`07-Nov-2025`), `Short` (`Nov 7, 2025`), `IsoDateTime`,
//!     `ShortDateTime`. The "Listed" column cycles through the
//!     major styles.
//! 11. **Star ratings** (`Cell::Stars` with `value` / `max`).
//! 12. **Priority bars** (`Cell::Priority` with `PriorityKind`):
//!     `None`, `Low`, `Medium`, `High`, `Critical` — drawn as a
//!     3-block bar (`░░░`, `▓░░`, `██░`, `█▓░`, `███`) so the user
//!     can tell the level at a glance.
//! 13. A selection-changed event (`on_selection_changed`) wired to
//!     the parent frame, used to drive a status label below the
//!     grid.
//!
//! The data model is a static list of products. Each product has
//! (icon, name, category, price, stock, max_stock, is_popular,
//! discount_pct, description).
//!
//! Run with:
//! ```bash
//! cargo run --example grid_demo
//! ```

#![windows_subsystem = "windows"]

use std::io::Write;
use std::time::Duration;

use ru_wx::{App, BadgeKind, BarStyle, BoxSizer, Cell, ColumnAlign, Font, FontDesc, Frame, Grid, GridDateFormat, ImageList, NumberFormat, PriorityKind, StaticText, Timer};

#[cfg(target_os = "windows")]
use ru_wx::icon::load_svg_bytes_as_hbitmap;

// Embed Bootstrap Icons SVG files at compile time
const STAR_SVG: &[u8] = include_bytes!("../assets/icons/star.svg");
const INFO_SVG: &[u8] = include_bytes!("../assets/icons/info.svg");
const FILE_NEW_SVG: &[u8] = include_bytes!("../assets/icons/file-new.svg");
const FOLDER_OPEN_SVG: &[u8] = include_bytes!("../assets/icons/folder-open.svg");
const EXIT_SVG: &[u8] = include_bytes!("../assets/icons/exit.svg");

macro_rules! step {
    ($($arg:tt)*) => {{
        eprintln!("[grid-demo] {}", format_args!($($arg)*));
        let _ = std::io::stderr().flush();
    }};
}

/// Try to add an SVG image to the list, returning the assigned index
/// (or `None` if SVG rendering failed). On non-Windows targets, this
/// is a stub.
#[cfg(target_os = "windows")]
fn add_svg(images: &ImageList, svg: &[u8], w: u32, h: u32) -> Option<i32> {
    let hb = load_svg_bytes_as_hbitmap(svg, w, h)?;
    images.add_bitmap(hb)
}

#[cfg(not(target_os = "windows"))]
fn add_svg(_images: &ImageList, _svg: &[u8], _w: u32, _h: u32) -> Option<i32> {
    None
}

/// Trim a long URL down to its host (+ optional first path segment)
/// for display purposes. The full URL is still kept in the
/// `Cell::Link::url` field so any click handler can open the
/// original. Examples:
///   `https://docs.example.com`            → `docs.example.com`
///   `https://shop.example.com/notebook`   → `shop.example.com/notebook`
///   `https://archive.example.com/old`     → `archive.example.com/old`
fn short_url_for_display(url: &str) -> String {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    // Take the host + at most one path segment, then truncate to
    // 18 characters so the cell never overflows a narrow column.
    let mut parts = stripped.splitn(3, '/');
    let host = parts.next().unwrap_or(stripped);
    let path = parts.next();
    let mut out = match path {
        Some(p) if !p.is_empty() => format!("{}/{}", host, p),
        _ => host.to_string(),
    };
    if out.chars().count() > 18 {
        // Keep the host (which is the most informative part) and
        // replace the middle with an ellipsis.
        let total = out.chars().count();
        let keep = 16;
        let head: String = out.chars().take(keep - 1).collect();
        let _tail: String = out.chars().skip(total - 1).collect();
        out = format!("{}…", head);
    }
    out
}

fn main() {
    step!("start");
    let app = App::new();
    // The window is sized to fit a 1024×768 screen (the smallest
    // display the demo has to support). The total of all 13
    // columns (Type, Product, Category, Price, Stock, Status,
    // Discount, Sales, Listed, URL, Rating, Priority, Weight) is
    // ~975 px; the listview gets the full 1000-px window width
    // (frame chrome on Win32 11 is ~8 px), so the columns never
    // scroll horizontally and the headers stay fully visible.
    let frame = Frame::builder()
        .with_title("ru_wx — Grid Demo (Advanced Table)")
        .with_size(1000, 720)
        .build();
    frame.set_size(1000, 720);
    step!("frame created, hwnd={:?}", frame.hwnd());

    // ── Build the image list ─────────────────────────────────────────
    // 5 icons, 16x16, 32-bit colour (alpha is preserved). Indices:
    //   0 star        (featured / popular)
    //   1 info        (digital / docs)
    //   2 file-new    (new release)
    //   3 folder-open (project)
    //   4 exit        (discontinued)
    let images = ImageList::new(16, 16);
    let _ = add_svg(&images, STAR_SVG, 16, 16);
    let _ = add_svg(&images, INFO_SVG, 16, 16);
    let _ = add_svg(&images, FILE_NEW_SVG, 16, 16);
    let _ = add_svg(&images, FOLDER_OPEN_SVG, 16, 16);
    let _ = add_svg(&images, EXIT_SVG, 16, 16);
    step!(
        "image list built: {}x{}, {} icons",
        images.width(),
        images.height(),
        5
    );

    // ── Create the grid and attach the image list ─────────────────────
    let grid = Grid::new(&frame);
    grid.set_image_list(&images);
    // Install a smaller-than-default font (Segoe UI 8pt) so every
    // column header renders its full title even in the narrow
    // columns. The default `SysListView32` face is 9pt; on a 96-DPI
    // display that 1-pt reduction is the difference between a
    // header truncated to a single character and one that fits
    // the word "Description" inside a 95-px column.
    let font = Font::new(FontDesc::new("Segoe UI", 8));
    grid.set_font(&font, true);
    step!("grid created + font set");

    // ── Enable checkboxes ───────────────────────────────────────────
    // Adds a state-image column at the far left of every row. The
    // checked state of each item can be read with `is_checked` /
    // flipped with `set_checked`. Pre-check rows 0 and 2 so the
    // checkbox styling is visible immediately.
    grid.set_checkboxes(true);
    step!("checkboxes enabled");

    // ── Columns ──────────────────────────────────────────────────────
    // The 13 columns total ~975 px which fits comfortably inside a
    // 1000-px window. The extra 20-px is the built-in checkbox
    // state-image column. Per-column alignment:
    //   • text columns       → left
    //   • numeric            → right (Price, Sales, Weight)
    //   • short labels       → center (Stock, Status, Discount, Rate, Prio)
    //   • free-form          → left (Product, URL)
    //
    // Column-width rationale (why these values, given the new badge /
    // bar / format content):
    //   • "Type"  (50)   — 16-px icon + 8-px padding + ~26 px slack
    //   • "Product" (125)— "Code Repository Pro" = 18 chars at 8pt ≈ 110 px
    //   • "Category"(65) — longest = "Peripherals" (11 chars) ≈ 65 px
    //   • "Price"  (75)  — "€ 1.234,50" = 11 chars ≈ 70 px
    //   • "Stock"  (80)  — 10-segment bar (Solid) + " 999u" label
    //   • "Status" (75)  — 1-char indicator + " Sold out" (8 chars)
    //   • "Disc%"  (85)  — 10-segment bar (4 styles) + "-15%" label
    //   • "Sales"  (55)  — integer with thousands (max "98,421")
    //   • "Listed" (75)  — `07-Nov-2025` (11 chars) or `07/11/2025` (10)
    //   • "URL"    (120) — `→ shop.example.com/no…` (max 18 chars + arrow)
    //   • "Rate"   (50)  — 5 ★ + 5 ☆ = 10 chars, centred
    //   • "Prio"   (65)  — 3-block bar + " Critical" (8 chars)
    //   • "Wt"     (55)  — "1.2 kg" / "350 ml" / "500+" (max 7 chars)
    grid.append_column_with_align("Type",     50, ColumnAlign::Left);
    grid.append_column_with_align("Product",  125, ColumnAlign::Left);
    grid.append_column_with_align("Category", 65,  ColumnAlign::Left);
    grid.append_column_with_align("Price (\u{20AC})", 75, ColumnAlign::Right);
    grid.append_column_with_align("Stock",    80,  ColumnAlign::Center);
    grid.append_column_with_align("Status",   75,  ColumnAlign::Center);
    grid.append_column_with_align("Disc%",    85,  ColumnAlign::Center);
    grid.append_column_with_align("Sales",    55,  ColumnAlign::Right);
    grid.append_column_with_align("Listed",   75,  ColumnAlign::Center);
    grid.append_column_with_align("URL",      120, ColumnAlign::Left);
    grid.append_column_with_align("Rate",     50,  ColumnAlign::Center);
    grid.append_column_with_align("Prio",     65,  ColumnAlign::Left);
    grid.append_column_with_align("Wt",       55,  ColumnAlign::Right);
    step!("{} columns appended", grid.col_count());

    // ── Data model (static; the provider reads it by index) ───────────
    // (icon, name, category, price, stock, max_stock, is_popular, discount_pct, description, sales, listed_date, url, rating, max_rating, priority, weight_label)
    let products: [(
        i32, &str, &str, f32, u32, u32, bool, u32, &str,
        f64, &str, &str, u32, u32, PriorityKind, &str,
    ); 10] = [
        (0, "Espresso Machine",   "Kitchen",     599.99, 12, 20, true,  15, "Premium\nbean-to-cup\nespresso maker",        4823.0,  "2024-03-15", "https://shop.example.com/espresso", 5, 5, PriorityKind::High,     "12 kg"),
        (2, "Project Notebook",   "Stationery",   14.50,250,300, false,  0, "Hardcover\ndotted\n192 pages",               18120.0, "2023-09-01", "https://shop.example.com/notebook", 4, 5, PriorityKind::Low,      "350 g"),
        (3, "Code Repository Pro","Software",     49.00,999,999, true,  20, "Git hosting\nwith CI/CD\npipelines",        23987.0, "2022-11-20", "https://code.example.com/pro",      5, 5, PriorityKind::Critical, "—"),
        (1, "API Documentation",  "Digital",       0.00,  0,100, false,  0, "Free\nopen-source\ndocs portal",             98421.0, "2021-06-08", "https://docs.example.com",         4, 5, PriorityKind::Medium,   "0 B"),
        (3, "Design System Kit",  "Software",     89.00, 43,100, true,  10, "500+\ncomponents\nFigma library",             1209.0, "2024-01-12", "https://figma.example.com/dsk",    5, 5, PriorityKind::High,     "1.2 GB"),
        (2, "Mechanical Keyboard","Peripherals", 129.00, 18, 30, true,   5, "Hot-swap\nRGB\ntactile switches",              764.0, "2024-05-22", "https://shop.example.com/kbd",     4, 5, PriorityKind::Medium,   "980 g"),
        (4, "Discontinued Item",  "Archive",      29.99,  0,  0, false,100, "Last-chance\nclearance\nwhile supplies last",    0.0, "2019-04-10", "https://archive.example.com/old",  1, 5, PriorityKind::None,     "n/a"),
        (0, "Limited Edition Mug","Kitchen",      24.00, 60,100, true,  25, "Ceramic\n350ml\ndishwasher safe",               312.0, "2025-02-28", "https://shop.example.com/mug",     5, 5, PriorityKind::Low,      "350 ml"),
        (3, "Cloud Backup Plan",  "Service",       9.99,  0,  0, false,  0, "1 TB\nencrypted\nauto-sync",                  5412.0, "2024-08-14", "https://cloud.example.com/plan",   3, 5, PriorityKind::Medium,   "1 TB"),
        (2, "Conference Ticket",  "Events",      349.00, 80,100, true,   0, "3-day\nfull access\nworkshops included",         150.0, "2025-09-30", "https://events.example.com/2025", 5, 5, PriorityKind::Critical, "1 ea"),
    ];

    // ── Function cells: one closure drives every cell of the table ───
    // This is the "function cell". The closure is
    // `Fn(usize, usize) -> Cell` and is called once per visible cell
    // on every `refresh()`. Returning different `Cell` variants gives
    // the grid its full expressivity (text, image, image+text,
    // progress bar, multi-line, empty, badge, stars, priority,
    // link, number, datetime) without the caller having to push
    // cells one-by-one.
    grid.set_value_provider(move |row, col| {
        let p = &products[row % products.len()];
        match col {
            // Icon-only cell, references the product's icon index.
            0 => Cell::ImageOnly(p.0),

            // Plain text.
            1 => Cell::Text(p.1.to_string()),
            2 => Cell::Text(p.2.to_string()),

            // Price as a `Cell::Number` with a row-dependent
            // `NumberFormat` so all four major numeric styles
            // (US thousands, EU thousands, percent, currency) are
            // visible in the same column. This is the "format"
            // side of "components and formats": every cell shows
            // a different visual treatment of a number.
            3 => {
                let format = match row % 4 {
                    0 => NumberFormat::CurrencyEuro,
                    1 => NumberFormat::WithThousandsEu,
                    2 => NumberFormat::Fixed2,
                    _ => NumberFormat::Percent,
                };
                Cell::Number { value: p.3 as f64, format }
            }

            // Stock as a progress bar: shows fill ratio
            // (stock / max_stock) and a textual suffix. This is a
            // "format" that conveys magnitude at a glance, without
            // any custom drawing. We cycle through the four
            // `BarStyle` variants by row so all four visual
            // treatments (Solid / Rounded / Square / Dots) are
            // visible in the same column at the same time.
            4 => {
                if p.5 == 0 {
                    Cell::Text("\u{2014}".to_string())
                } else {
                    let style = match row % 4 {
                        0 => BarStyle::Solid,
                        1 => BarStyle::Rounded,
                        2 => BarStyle::Square,
                        _ => BarStyle::Dots,
                    };
                    Cell::Bar {
                        value: p.4,
                        max: p.5,
                        width: 8,
                        style,
                        label: Some(format!("{}u", p.4)),
                    }
                }
            }

            // Status as a centred *badge* (`Cell::Badge`). The
            // indicator character (`● ▲ ■ ✕ ○`) gives an at-a-glance
            // hint of the kind, and the `BadgeKind` enum forces a
            // consistent vocabulary across the table.
            5 => {
                if p.5 == 0 {
                    Cell::Badge {
                        text: "Sold out".to_string(),
                        kind: BadgeKind::Bad,
                    }
                } else if p.4 < p.5 / 4 {
                    Cell::Badge {
                        text: "Low".to_string(),
                        kind: BadgeKind::Warn,
                    }
                } else if p.6 {
                    Cell::Badge {
                        text: "Featured".to_string(),
                        kind: BadgeKind::Hot,
                    }
                } else if p.4 == p.5 {
                    Cell::Badge {
                        text: "Full".to_string(),
                        kind: BadgeKind::Ok,
                    }
                } else {
                    Cell::Badge {
                        text: "Standard".to_string(),
                        kind: BadgeKind::Neutral,
                    }
                }
            }

            // Discount as a progress bar. The same 4-style rotation
            // as Stock — by row, not by column — so that scrolling
            // vertically shows the visual variety. A "100%" discount
            // (the discontinued item) is rendered as an empty bar
            // with a "clearance" label.
            6 => {
                if p.7 == 0 {
                    Cell::Text("\u{2014}".to_string())
                } else if p.7 == 100 {
                    Cell::Bar {
                        value: 100,
                        max: 100,
                        width: 8,
                        style: BarStyle::Solid,
                        label: Some("clearance".to_string()),
                    }
                } else {
                    let style = match row % 4 {
                        0 => BarStyle::Solid,
                        1 => BarStyle::Rounded,
                        2 => BarStyle::Square,
                        _ => BarStyle::Dots,
                    };
                    Cell::Bar {
                        value: p.7,
                        max: 100,
                        width: 8,
                        style,
                        label: Some(format!("-{}%", p.7)),
                    }
                }
            }

            // Sales as a `Cell::Number` with thousands separator.
            // The format is also row-dependent so multiple
            // `NumberFormat` variants (Integer, Plain,
            // WithThousands) appear in the same column.
            7 => {
                let format = match row % 3 {
                    0 => NumberFormat::Integer,
                    1 => NumberFormat::Plain,
                    _ => NumberFormat::WithThousands,
                };
                Cell::Number { value: p.9, format }
            }

            // "Listed" date. Different rows use different
            // `GridDateFormat` variants (Iso, Eu, Us, Long, Short)
            // so the format catalogue is visible side-by-side in
            // the same column.
            8 => {
                let format = match row % 5 {
                    0 => GridDateFormat::Iso,
                    1 => GridDateFormat::Eu,
                    2 => GridDateFormat::Us,
                    3 => GridDateFormat::Long,
                    _ => GridDateFormat::Short,
                };
                Cell::DateTime {
                    iso: p.10.to_string(),
                    format,
                }
            }

            // Hyperlink rendered with a leading `\u{2192}` arrow.
            9 => Cell::Link {
                text: short_url_for_display(p.11).to_string(),
                url: p.11.to_string(),
            },

            // Star rating out of `max`. Clamped by the renderer
            // to 1..=10; here we only ever pass 1..=5.
            10 => Cell::Stars {
                value: p.12,
                max: p.13,
            },

            // 3-block bar + label: `None`, `Low`, `Medium`, `High`,
            // `Critical`. Left-aligned column.
            11 => Cell::Priority { kind: p.14 },

            // Weight: a plain text cell with units (kg / g / ml /
            // GB / TB / ea). Demonstrates that `Cell::Text` can
            // carry domain-specific formatting (e.g. "1.2 GB")
            // alongside the structured `Cell` variants.
            12 => Cell::Text(p.15.to_string()),

            _ => Cell::Empty,
        }
    });

    grid.set_row_count(products.len());
    step!(
        "grid populated: {} rows × {} cols",
        grid.row_count(),
        grid.col_count()
    );

    // NOTE: the pre-check + initial paint that used to live here was
    // moved to a one-shot timer below. The reason is that calling
    // `set_checked` (or a full `force_refresh()`) BEFORE `app.run()`
    // blocks the calling thread inside the first `SendMessageW`
    // because the ListView tries to dispatch a synchronous parent
    // notification and the message loop is not yet running. The
    // user-visible result is a frozen process with a never-shown
    // window. The one-shot timer fires after the loop is up, so
    // both operations complete normally and the window appears.

    // ── Status label (always visible at the bottom) ──────────────────
    let status = StaticText::new(
        &frame,
        "Click a row to see its details. The selection event updates this label.",
    );
    status.as_widget_ref().borrow_mut().set_size(0, 22);

    // ── Sizer: grid (proportion 1) + status label ─────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(4);
    sizer.add_with_proportion(grid.as_widget_ref(), 1);
    sizer.add(status.as_widget_ref());
    frame.set_sizer(sizer);
    step!("frame sizer set");

    // ── Selection event ─────────────────────────────────────────────
    // `on_selection_changed` registers a notify handler with the
    // parent frame. The argument is `Some(row)` when the user
    // selects a row, or `None` when they clear the selection. The
    // grid internally debounces the duplicate LVN_ITEMCHANGED
    // notifications that the control sends per click, so this
    // closure fires once per actual change.
    let s_for_sel = status.clone();
    let g_for_sel = grid.clone();
    grid.on_selection_changed(&frame, move |sel| match sel {
        Some(row) => {
            let p = &products[row % products.len()];
            let avail = if p.4 > 0 { "available" } else { "OUT OF STOCK" };
            let checked = if g_for_sel.is_checked(row) { "[x]" } else { "[ ]" };
            s_for_sel.set_label(&format!(
                "{checked} Row {row}: \"{}\" \u{2014} {} ({}, \u{20AC}{:.2}, {avail}) | sales: {} | rating: {}/{} | priority: {:?}",
                p.1,
                p.2,
                if p.6 { "featured" } else { "standard" },
                p.3,
                p.9 as i64,
                p.12,
                p.13,
                p.14,
            ));
        }
        None => {
            s_for_sel.set_label("(no row selected)");
        }
    });
    step!("selection callback registered");

    // ── One-shot timer for the initial pre-check + force-refresh ──────
    // `set_checked` and `force_refresh` MUST run while the message
    // loop is processing messages; doing them before `app.run()`
    // deadlocks the first `SendMessageW` because the ListView wants
    // to dispatch a synchronous NM_CUSTOMDRAW / LVN_ITEMCHANGED to
    // the parent. A 100-ms one-shot timer fires after the loop is
    // up, runs the two calls, and lets the loop dispatch the
    // resulting notifications normally.
    let g_for_init = grid.clone();
    let init_timer = Timer::new(&frame);
    init_timer.on_tick(move || {
        g_for_init.set_checked(0, true);
        g_for_init.set_checked(2, true);
        g_for_init.set_checked(7, true);
        g_for_init.force_refresh();
        step!("3 rows pre-checked (via one-shot timer)");
    });
    init_timer.start_one_shot(Duration::from_millis(100));
    step!("initial-paint one-shot timer armed (100 ms)");

    // ── Run the event loop ───────────────────────────────────────────
    step!("about to run event loop");
    app.run(frame);
    step!("event loop returned");
}
