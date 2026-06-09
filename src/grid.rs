//! `Grid` — an advanced tabular widget (a `wxGrid`-style control).
//!
//! Built on top of the Win32 `SysListView32` common control in report
//! view. Each row is an item, each column is a sub-item, and cells
//! can carry text, an image, or both. The grid can be populated
//! statically (cell by cell) or driven by a closure ("function
//! cell") that maps `(row, col) -> Cell`. The latter makes it easy
//! to back the grid with a `Vec<RowData>` or any other data source
//! and to update everything by re-querying the closure.
//!
//! # Quick example
//! ```no_run
//! use ru_wx::*;
//!
//! let app  = App::new();
//! let frame = Frame::builder().with_title("Grid").with_size(600, 400).build();
//! let grid = Grid::new(&frame);
//!
//! // Build an image list and attach it (5 icons, 16x16, 32-bit colour)
//! let mut images = ImageList::new(16, 16);
//! // images.add_bitmap(load_svg_as_hbitmap(...).unwrap());
//! grid.set_image_list(&images);
//!
//! grid.append_column("ID",    60);
//! grid.append_column("Name",  200);
//! grid.append_column("Score", 80);
//!
//! // Function-based cells: the closure is called for every (row, col).
//! grid.set_value_provider(|row, col| match col {
//!     0 => Cell::Text(format!("#{}", row + 1)),
//!     1 => Cell::Image { idx: row as i32 % 5, text: format!("item {row}") },
//!     _ => Cell::Text(format!("{}", (row as i32) * 10)),
//! });
//!
//! grid.set_row_count(10);
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::dpi::get_dpi_for_window;
use crate::font::Font;
use crate::frame::Frame;
use crate::geometry::Rect;
use crate::image_list::ImageList;
use crate::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ListView constants (kept local to match list_ctrl.rs) ──────

#[cfg(target_os = "windows")]
const LVM_FIRST: u32 = 0x1000;
#[cfg(target_os = "windows")]
const LVM_INSERTCOLUMN: u32 = LVM_FIRST + 27;
#[cfg(target_os = "windows")]
const LVM_INSERTITEM: u32 = LVM_FIRST + 7;
#[cfg(target_os = "windows")]
const LVM_SETITEM: u32 = LVM_FIRST + 6;
#[cfg(target_os = "windows")]
const LVM_DELETEALLITEMS: u32 = LVM_FIRST + 9;
#[cfg(target_os = "windows")]
const LVM_GETITEM: u32 = LVM_FIRST + 5;
#[cfg(target_os = "windows")]
const LVM_SETIMAGELIST: u32 = LVM_FIRST + 3;
#[cfg(target_os = "windows")]
const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = LVM_FIRST + 54;
#[cfg(target_os = "windows")]
const LVM_GETEXTENDEDLISTVIEWSTYLE: u32 = LVM_FIRST + 55;
#[cfg(target_os = "windows")]
const LVM_GETNEXTITEM: u32 = LVM_FIRST + 12;
#[cfg(target_os = "windows")]
const LVM_REDRAWITEMS: u32 = LVM_FIRST + 21;

/// LVCOLUMNW / LVITEMW mask flags
#[cfg(target_os = "windows")]
const LVCF_TEXT: u32 = 4;
#[cfg(target_os = "windows")]
const LVCF_WIDTH: u32 = 2;
#[cfg(target_os = "windows")]
const LVCF_FMT: u32 = 1;
#[cfg(target_os = "windows")]
const LVCFMT_LEFT: u32 = 0;
#[cfg(target_os = "windows")]
const LVCFMT_CENTER: u32 = 2;
#[cfg(target_os = "windows")]
const LVCFMT_RIGHT: u32 = 1;
#[cfg(target_os = "windows")]
const LVIF_TEXT: u32 = 1;
#[cfg(target_os = "windows")]
const LVIF_IMAGE: u32 = 2;

/// Extended styles
#[cfg(target_os = "windows")]
const LVS_REPORT: u32 = 0x0001;
#[cfg(target_os = "windows")]
const LVS_EX_FULLROWSELECT: u32 = 0x00000020;
#[cfg(target_os = "windows")]
const LVS_EX_GRIDLINES: u32 = 0x00000001;
#[cfg(target_os = "windows")]
const LVS_EX_CHECKBOXES: u32 = 0x00000004;
/// `LVS_EX_DOUBLEBUFFER` — paint the control via an off-screen
/// DC and blit the result in one shot. Eliminates the flicker that
/// the default `WM_ERASEBKGND` → `WM_PAINT` pipeline produces on
/// `SysListView32`, especially when the window is resized by a
/// sizer. Safe to combine with every other extended style.
#[cfg(target_os = "windows")]
const LVS_EX_DOUBLEBUFFER: u32 = 0x00010000;
/// `LVS_EX_HEADERDRAGDROP` — let the user drag column headers to
/// re-order the columns. The user-supplied `append_column` order
/// stays the logical "column index" used by `set_cell` /
/// `set_value_provider`; the visual order is what changes.
#[cfg(target_os = "windows")]
const LVS_EX_HEADERDRAGDROP: u32 = 0x00000010;
/// `LVS_EX_AUTOSIZECOLUMNS` — automatically resize columns to fit
/// the header text. Off by default because the user is expected
/// to set widths explicitly via `append_column_with_align`.
#[cfg(target_os = "windows")]
const LVS_EX_AUTOSIZECOLUMNS: u32 = 0x00000080;

/// `LVM_SETCOLUMNWIDTH` — explicit column-width set, used as a
/// belt-and-suspenders follow-up to `LVM_INSERTCOLUMN` because some
/// ListView / DPI combinations ignore the `cx` field of `LVCOLUMNW`
/// and collapse every column to a single-character width.
#[cfg(target_os = "windows")]
const LVM_SETCOLUMNWIDTH: u32 = LVM_FIRST + 30;

/// `LVM_GETCOLUMNWIDTH` — read back the actual column width so we can
/// sanity-check that our `LVM_SETCOLUMNWIDTH` call was actually
/// accepted by the control (it returns a `LRESULT` whose low word is
/// the width in pixels).
#[cfg(target_os = "windows")]
const LVM_GETCOLUMNWIDTH: u32 = LVM_FIRST + 29;

/// `LVM_GETHEADER` — return the `HWND` of the header control that
/// `SysListView32` creates in report view. The header is a child
/// window that owns its own `HFONT`, so `WM_SETFONT` sent to the
/// listview itself only re-paints the cells, not the column titles.
/// In a tight layout with the default icon font that means every
/// header elides to a single character even though the column has
/// 100+ pixels of width. We grab this `HWND` from `set_font` and
/// forward the same `HFONT` so the titles render full-length.
#[cfg(target_os = "windows")]
const LVM_GETHEADER: u32 = LVM_FIRST + 31;

/// Image list slot for the small (cell) image list.
#[cfg(target_os = "windows")]
const LVSIL_SMALL: u32 = 1;

/// Flag for `LVM_GETNEXTITEM`: the next item that has the LVIS_SELECTED state.
#[cfg(target_os = "windows")]
const LVNI_SELECTED: u32 = 2;

/// `WM_SETFONT` — install a custom `HFONT` on the control. Used by
/// [`Grid::set_font`] so the caller can pick a smaller / larger face
/// than the system default; the smaller face is the difference
/// between column headers truncating to a single character and
/// rendering their full title in a tight layout.
#[cfg(target_os = "windows")]
const WM_SETFONT: u32 = 0x0030;

/// `WM_GETFONT` — read back the `HFONT` currently installed on a
/// control. Used by the `set_font` debug path to verify that the
/// `WM_SETFONT` we just issued actually replaced the font, and to
/// spot the cases where the OS resets the font to a system default
/// after we set it (e.g. DPI-scaling events, owner-window
/// `WM_SETFONT` propagation).
#[cfg(target_os = "windows")]
const WM_GETFONT: u32 = 0x0031;

/// `WM_FONT` — companion to `WM_SETFONT`; the control's owner
/// receives this when its font changes.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const WM_FONT: u32 = 0x0018;

// ── Win32 structs ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
#[allow(clippy::upper_case_acronyms)]
struct LVCOLUMNW {
    mask: u32,
    fmt: i32,
    cx: i32,
    psz_text: *const u16,
    cch_text_max: i32,
    i_sub_item: i32,
    i_image: i32,
    i_order: i32,
    cx_min: i32,
    cx_default: i32,
    cx_ideal: i32,
}

// Compile-time measurement of the LVCOLUMNW struct. The Win32 SDK
// says the struct is 52 bytes on x64 (pointer-aligned) and 44 bytes
// on x86. If this ever changes, the column data will be written at
// the wrong offsets and the ListView will silently render wrong
// widths / wrong headers. The actual size is also logged at
// process start-up to the diagnostic file so the value is visible.
#[cfg(target_os = "windows")]
const _LVCOLUMNW_SIZE: usize = std::mem::size_of::<LVCOLUMNW>();

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
#[allow(clippy::upper_case_acronyms)]
struct LVITEMW {
    mask: u32,
    i_item: i32,
    i_sub_item: i32,
    state: u32,
    state_mask: u32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    l_param: isize,
    i_indent: i32,
    i_group_id: i32,
    c_columns: u32,
    pu_columns: *mut u32,
    pi_col_fmt: *mut i32,
    i_group: i32,
}

// ── Cell value ───────────────────────────────────────────────────────

/// Column alignment. Maps to the `fmt` field of `LVCOLUMNW`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnAlign {
    /// Text hugs the left edge of the column.
    Left,
    /// Text is centered horizontally inside the column.
    Center,
    /// Text hugs the right edge of the column (good for numeric values).
    Right,
}

impl Default for ColumnAlign {
    fn default() -> Self {
        ColumnAlign::Left
    }
}

impl ColumnAlign {
    /// Map to the `LVCFMT_*` flag the Win32 SDK expects in the `fmt`
    /// field of `LVCOLUMNW`.
    #[cfg(target_os = "windows")]
    fn as_lvfmt(self) -> u32 {
        match self {
            ColumnAlign::Left => LVCFMT_LEFT,
            ColumnAlign::Center => LVCFMT_CENTER,
            ColumnAlign::Right => LVCFMT_RIGHT,
        }
    }
}

/// Visual style of a [`Cell::Badge`]. A badge is a short text label
/// drawn with a leading character (the *indicator*) and a closing
/// bracket, e.g. `●  OK  `, `▲  Low  `, `■  Hot  `, `✕  Sold  `.
///
/// The indicator and bracket characters are not configurable: the
/// 5 built-in kinds give enough visual variety for a status column
/// without bringing a font/colour system into the cell model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    /// Filled circle `●` — generic "OK" / normal status.
    Ok,
    /// Filled triangle `▲` — warning / low / trending down.
    Warn,
    /// Filled square `■` — emphasised / "hot" / featured.
    Hot,
    /// Cross `✕` — error / out-of-stock / failed.
    Bad,
    /// Hollow circle `○` — neutral / idle / pending.
    Neutral,
}

impl BadgeKind {
    fn indicator(self) -> &'static str {
        match self {
            BadgeKind::Ok => "\u{25CF}",      // ●
            BadgeKind::Warn => "\u{25B2}",    // ▲
            BadgeKind::Hot => "\u{25A0}",     // ■
            BadgeKind::Bad => "\u{2715}",     // ✕
            BadgeKind::Neutral => "\u{25CB}", // ○
        }
    }
}

/// Visual style of a [`Cell::Bar`]. Different styles use different
/// Unicode block characters to suggest different "feels" — solid
/// (the default [`Cell::Progress`]), gradient, dotted, etc. — without
/// pulling in a font / colour system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarStyle {
    /// `█` / `░` — solid blocks. The default for [`Cell::Progress`].
    Solid,
    /// `▰` / `▱` — black / white squares with rounded corners.
    Rounded,
    /// `■` / `□` — fully black / outlined squares.
    Square,
    /// `●` / `○` — round dots, one per cell of the bar.
    Dots,
}

impl BarStyle {
    fn chars(self) -> (char, char) {
        match self {
            BarStyle::Solid => ('\u{2588}', '\u{2591}'),
            BarStyle::Rounded => ('\u{25B0}', '\u{25B1}'),
            BarStyle::Square => ('\u{25A0}', '\u{25A1}'),
            BarStyle::Dots => ('\u{25CF}', '\u{25CB}'),
        }
    }
}

/// Numeric formatting for [`Cell::Number`]. The grid has no
/// custom-drawing code, so "formatting" is implemented by
/// stringifying the value with the requested thousands separator
/// and decimal style. All variants produce pure ASCII / UTF-8
/// strings; no locale lookup is performed at the grid level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberFormat {
    /// `1234.5` (no thousands separator, full precision).
    Plain,
    /// `1,234.5` — comma thousands, dot decimal.
    WithThousands,
    /// `1.234,5` — dot thousands, comma decimal (European).
    WithThousandsEu,
    /// `1,234` — integer with thousands, no decimal.
    Integer,
    /// `1,234.50` — two decimal places, always shown.
    Fixed2,
    /// `12.3%` — value rendered as a percentage of 100.
    Percent,
    /// `€ 1,234.50` — euro-prefixed, two decimals, thousands.
    CurrencyEuro,
    /// `$ 1,234.50` — dollar-prefixed, two decimals, thousands.
    CurrencyDollar,
}

impl NumberFormat {
    /// Render `value` according to this format.
    fn render(self, value: f64) -> String {
        match self {
            NumberFormat::Plain => format!("{}", value),
            NumberFormat::WithThousands | NumberFormat::Integer => {
                Self::format_with_sep(value, ',', '.', false)
            }
            NumberFormat::WithThousandsEu => Self::format_with_sep(value, '.', ',', false),
            NumberFormat::Fixed2 => {
                if self == NumberFormat::Integer {
                    Self::format_with_sep(value, ',', '.', false)
                } else {
                    Self::format_with_sep(value, ',', '.', true)
                }
            }
            NumberFormat::Percent => {
                let s = Self::format_with_sep(value, ',', '.', true);
                format!("{}%", s)
            }
            NumberFormat::CurrencyEuro => {
                let s = Self::format_with_sep(value, ',', '.', true);
                format!("\u{20AC} {}", s)
            }
            NumberFormat::CurrencyDollar => {
                let s = Self::format_with_sep(value, ',', '.', true);
                format!("$ {}", s)
            }
        }
    }
    fn format_with_sep(value: f64, thou: char, dec: char, always_2dp: bool) -> String {
        // Negative numbers: print the minus sign, then the digits.
        let neg = value < 0.0;
        let abs = value.abs();
        // Split into integer and fractional parts without going
        // through `f64::round` (which loses precision near 1e16).
        let (int_part, frac_part) = {
            // 2 decimal places when always_2dp, otherwise truncate
            // to the natural precision of the input.
            let scaled = if always_2dp { abs * 100.0 } else { abs };
            let int_scaled = scaled.trunc() as i64;
            let int_part = int_scaled / if always_2dp { 100 } else { 1 };
            let frac = if always_2dp {
                int_scaled % 100
            } else {
                let frac_scaled = (scaled - scaled.trunc()) * 100.0;
                frac_scaled.round() as i64
            };
            (int_part, frac)
        };
        // Insert thousands separators into the integer part.
        let int_str = int_part.to_string();
        let mut grouped = String::new();
        for (i, c) in int_str.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                grouped.push(thou);
            }
            grouped.push(c);
        }
        let int_grouped: String = grouped.chars().rev().collect();
        let sign = if neg { "-" } else { "" };
        if always_2dp || (!always_2dp && frac_part > 0) {
            format!(
                "{}{}{}{:02}",
                sign,
                int_grouped,
                dec,
                frac_part.abs()
            )
        } else {
            format!("{}{}", sign, int_grouped)
        }
    }
}

/// Date / time formatting for [`Cell::DateTime`]. The grid does
/// not own a date parser, so the caller passes a pre-parsed ISO
/// `YYYY-MM-DD` or `YYYY-MM-DD HH:MM:SS` string and the variant
/// picks the output style. All variants are UTF-8 / ASCII.
///
/// Renamed to `GridDateFormat` to avoid colliding with the
/// `DateFormat` type exported from `date_picker_ctrl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridDateFormat {
    /// `2025-11-07` — ISO 8601 date.
    Iso,
    /// `07/11/2025` — European / day-first.
    Eu,
    /// `11/07/2025` — US / month-first.
    Us,
    /// `07-Nov-2025` — long month name, day-first.
    Long,
    /// `Nov 7, 2025` — short month, US style.
    Short,
    /// `2025-11-07 14:30` — ISO date + 24h time (assumes `HH:MM`
    /// suffix on the input).
    IsoDateTime,
    /// `Nov 7, 2:30 PM` — short month + 12h time.
    ShortDateTime,
}

const MONTH_LONG: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];
const MONTH_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl GridDateFormat {
    /// Render `iso` according to this format. `iso` is expected to
    /// be either `YYYY-MM-DD` (date only) or
    /// `YYYY-MM-DD HH:MM[:SS]` (date + time). If parsing fails
    /// the input is returned verbatim.
    fn render(self, iso: &str) -> String {
        // Parse the date portion. Find the first '-' to get year.
        let mut parts = iso.splitn(3, '-');
        let y = match parts.next() {
            Some(y) => match y.parse::<u32>() {
                Ok(n) => n,
                Err(_) => return iso.to_string(),
            },
            None => return iso.to_string(),
        };
        let m = match parts.next() {
            Some(m) => match m.parse::<u32>() {
                Ok(n) if (1..=12).contains(&n) => n,
                _ => return iso.to_string(),
            },
            None => return iso.to_string(),
        };
        // Day may be followed by space + time, or end of string.
        let rest = match parts.next() {
            Some(r) => r,
            None => return iso.to_string(),
        };
        // Day is the first two characters of `rest`.
        if rest.len() < 2 {
            return iso.to_string();
        }
        let d = match rest[..2].parse::<u32>() {
            Ok(n) => n,
            Err(_) => return iso.to_string(),
        };
        // Optional time component (everything from char 2 onward).
        let time = rest.get(2..).unwrap_or("").trim();
        let mi = m as usize - 1;
        match self {
            GridDateFormat::Iso => format!("{:04}-{:02}-{:02}", y, m, d),
            GridDateFormat::Eu => format!("{:02}/{:02}/{:04}", d, m, y),
            GridDateFormat::Us => format!("{:02}/{:02}/{:04}", m, d, y),
            GridDateFormat::Long => {
                format!("{:02}-{}-{:04}", d, MONTH_LONG[mi], y)
            }
            GridDateFormat::Short => {
                format!("{} {}, {}", MONTH_SHORT[mi], d, y)
            }
            GridDateFormat::IsoDateTime => {
                if time.is_empty() {
                    format!("{:04}-{:02}-{:02}", y, m, d)
                } else {
                    format!("{:04}-{:02}-{:02} {}", y, m, d, time)
                }
            }
            GridDateFormat::ShortDateTime => {
                if time.is_empty() {
                    format!("{} {}, {}", MONTH_SHORT[mi], d, y)
                } else {
                    // Convert 24h `HH:MM` to `H:MM AM/PM`. If the
                    // string isn't `HH:MM`, return it verbatim.
                    let mut hhmm = time.splitn(2, ':');
                    let hh = match hhmm.next() {
                        Some(h) => match h.parse::<u32>() {
                            Ok(n) if n < 24 => n,
                            _ => return iso.to_string(),
                        },
                        None => return iso.to_string(),
                    };
                    let mm = hhmm.next().unwrap_or("00");
                    let (h12, ampm) = if hh == 0 {
                        (12, "AM")
                    } else if hh < 12 {
                        (hh, "AM")
                    } else if hh == 12 {
                        (12, "PM")
                    } else {
                        (hh - 12, "PM")
                    };
                    format!(
                        "{} {}, {} {}:{} {}",
                        MONTH_SHORT[mi], d, y, h12, mm, ampm
                    )
                }
            }
        }
    }
}

/// Priority level for [`Cell::Priority`]. Drawn as a 3-step
/// coloured bar made of `█` (full), `▓` (mid), `░` (low) so the
/// user can tell critical / high / medium / low / none at a
/// glance without any custom drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorityKind {
    /// `░░░` — no work needed / deprecated.
    None,
    /// `▓░░` — backlog item.
    Low,
    /// `██░` — scheduled for the current sprint.
    Medium,
    /// `█▓░` — needs attention soon.
    High,
    /// `███` — on fire / blocking the release.
    Critical,
}

impl PriorityKind {
    fn bars(self) -> &'static str {
        match self {
            PriorityKind::None => "\u{2591}\u{2591}\u{2591}",
            PriorityKind::Low => "\u{2593}\u{2591}\u{2591}",
            PriorityKind::Medium => "\u{2588}\u{2588}\u{2591}",
            PriorityKind::High => "\u{2588}\u{2593}\u{2591}",
            PriorityKind::Critical => "\u{2588}\u{2588}\u{2588}",
        }
    }
    fn label(self) -> &'static str {
        match self {
            PriorityKind::None => "None",
            PriorityKind::Low => "Low",
            PriorityKind::Medium => "Medium",
            PriorityKind::High => "High",
            PriorityKind::Critical => "Critical",
        }
    }
}

/// The display value of a single cell.
#[derive(Clone, Debug)]
pub enum Cell {
    /// Cell is empty (no text, no image).
    Empty,
    /// Plain text.
    Text(String),
    /// Image (referencing an entry in the attached `ImageList`) plus
    /// a text label drawn to the right of the image.
    Image { idx: i32, text: String },
    /// Image only, no text.
    ImageOnly(i32),
    /// Text rendered as a horizontal progress bar. `value` is the
    /// current value, `max` is the maximum (so a half-filled bar
    /// shows `value/max ≈ 0.5`). `label`, if `Some`, replaces the
    /// default `"v/m"` suffix (e.g. `"███████░░░ 70%"`).
    ///
    /// Implemented with Unicode block characters (`█ U+2588` for
    /// filled, `░ U+2591` for empty) — no custom drawing required,
    /// the ListView just sees a `Text` cell.
    Progress {
        value: u32,
        max: u32,
        label: Option<String>,
    },
    /// Multi-line text. Newline characters (`\n`) are converted to
    /// `\r` because Win32 ListView cells use `\r` as the line
    /// separator inside a single sub-item.
    MultiLine(String),
    /// Configurable progress bar with a selectable visual style.
    /// `width` controls the number of segments in the bar (default
    /// in [`Cell::Progress`] is 10); `style` picks the block
    /// characters; `label`, if `Some`, replaces the default `"v/m"`
    /// suffix. Use this variant when [`Cell::Progress`] (the solid
    /// `█`/`░` style) does not match the rest of the row.
    Bar {
        value: u32,
        max: u32,
        width: usize,
        style: BarStyle,
        label: Option<String>,
    },
    /// Status badge: a leading indicator character, a space, the
    /// text, a trailing space. Compact, centred nicely, and
    /// visually distinct from a plain text cell. Example outputs:
    /// `● OK`, `▲ Low`, `■ Featured`, `✕ Sold out`, `○ Pending`.
    Badge { text: String, kind: BadgeKind },
    /// Numeric value rendered with one of the [`NumberFormat`]
    /// styles. The grid has no custom drawing, so the value is
    /// stringified in the request format and pushed as a `Text`
    /// cell. Pair with a right-aligned column for tidy numeric
    /// columns.
    Number { value: f64, format: NumberFormat },
    /// Date or date+time. `iso` is the input in `YYYY-MM-DD` or
    /// `YYYY-MM-DD HH:MM[:SS]` form; `format` picks the output
    /// style. See [`GridDateFormat`].
    DateTime { iso: String, format: GridDateFormat },
    /// Hyperlink. Rendered as `\u{2192} {text}` (a leading right
    /// arrow + the visible label) so the cell stands out from a
    /// plain `Text` cell. The `url` is stored for callers that
    /// want to wire it to a click handler; the grid itself does
    /// not open URLs.
    Link { text: String, url: String },
    /// Priority bar: 3 unicode block characters showing the
    /// priority level (none / low / medium / high / critical)
    /// followed by the textual label. Pair with a left-aligned
    /// column.
    Priority { kind: PriorityKind },
    /// Star rating out of `max` (clamped to 1..=10). Rendered as
    /// `★★★☆☆` etc. using `★ U+2605` (filled) and `☆ U+2606`
    /// (empty). Useful for product reviews / customer feedback.
    Stars { value: u32, max: u32 },
}

impl Cell {
    fn text(&self) -> String {
        match self {
            Cell::Text(t) => t.clone(),
            Cell::Image { text, .. } => text.clone(),
            Cell::MultiLine(t) => t.replace('\n', "\r"),
            Cell::Progress {
                value,
                max,
                label,
            } => Self::render_bar(*value, *max, 10, BarStyle::Solid, label.as_deref()),
            Cell::Bar {
                value,
                max,
                width,
                style,
                label,
            } => Self::render_bar(*value, *max, *width, *style, label.as_deref()),
            Cell::Badge { text, kind } => {
                format!("{} {}", kind.indicator(), text)
            }
            Cell::Number { value, format } => format.render(*value),
            Cell::DateTime { iso, format } => format.render(iso),
            Cell::Link { text, url } => {
                if url.is_empty() {
                    format!("\u{2192} {}", text)
                } else {
                    format!("\u{2192} {} ({})", text, url)
                }
            }
            Cell::Priority { kind } => {
                format!("{} {}", kind.bars(), kind.label())
            }
            Cell::Stars { value, max } => Self::render_stars(*value, *max),
            _ => String::new(),
        }
    }
    fn image(&self) -> Option<i32> {
        match self {
            Cell::Image { idx, .. } => Some(*idx),
            Cell::ImageOnly(idx) => Some(*idx),
            _ => None,
        }
    }
    fn has_text(&self) -> bool {
        !matches!(self, Cell::Empty | Cell::ImageOnly(_))
    }

    /// Render a value/max pair as a Unicode block bar with the given
    /// width and style. Used by both [`Cell::Progress`] (which
    /// hard-codes `width=10, style=Solid`) and [`Cell::Bar`].
    fn render_bar(
        value: u32,
        max: u32,
        width: usize,
        style: BarStyle,
        label: Option<&str>,
    ) -> String {
        let pct = if max == 0 {
            0.0
        } else {
            (value as f32 / max as f32).clamp(0.0, 1.0)
        };
        // Always render `width` segments (filled or empty) so the
        // bar is the same length in every row of the column.
        let width = width.max(1);
        let filled = (pct * width as f32).round() as usize;
        let empty = width.saturating_sub(filled);
        let (fc, ec) = style.chars();
        let bar: String =
            fc.to_string().repeat(filled) + &ec.to_string().repeat(empty);
        match label {
            Some(l) => format!("{bar} {l}"),
            None => format!("{bar} {value}/{max}"),
        }
    }

    /// Render `value` out of `max` as a string of `★` (filled) and
    /// `☆` (empty) characters. `max` is clamped to `1..=10` so
    /// the cell can never exceed the column width.
    fn render_stars(value: u32, max: u32) -> String {
        let max = max.clamp(1, 10) as usize;
        let value = (value as usize).min(max);
        let filled = "\u{2605}".repeat(value);
        let empty = "\u{2606}".repeat(max - value);
        format!("{filled}{empty}")
    }
}

// ── Inner state ──────────────────────────────────────────────────────

struct GridInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    col_count: usize,
    row_count: usize,
    /// Width (in pixels) of each column, in insertion order. Stored
    /// so we can re-apply the widths after the control is resized
    /// by the sizer — some ListView / DPI combinations silently
    /// collapse column widths when the control is moved and the
    /// header does not refresh itself.
    col_widths: Vec<i32>,
    /// Alignment of each column, parallel to `col_widths`.
    col_aligns: Vec<ColumnAlign>,
    /// Static cells set via `set_cell`. The provider (if set) takes
    /// priority over this.
    cells: HashMap<(usize, usize), Cell>,
    /// Optional closure that, if set, is queried for every (row, col).
    provider: Option<Box<dyn Fn(usize, usize) -> Cell>>,
    /// Tracks the last selection so that the selection-changed callback
    /// fires only on actual changes (the control fires two
    /// `LVN_ITEMCHANGED` notifications per click).
    last_selection: Option<usize>,
    on_sel_change: Option<Box<dyn FnMut(Option<usize>)>>,
    enabled: bool,
    visible: bool,
}

// ── Public type ──────────────────────────────────────────────────────

/// Advanced tabular widget.
///
/// Cloneable (internal state is `Rc<RefCell<_>>`).
#[derive(Clone)]
pub struct Grid {
    inner: Rc<RefCell<GridInner>>,
}

impl Grid {
    /// Create a new grid as a child of `parent`.
    #[cfg(target_os = "windows")]
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent_hwnd = parent.hwnd();
            let wide_class = to_wide("SysListView32");
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | LVS_REPORT | WS_BORDER,
                0,
                0,
                400,
                300,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let grid = Grid {
            inner: Rc::new(RefCell::new(GridInner {
                hwnd,
                id,
                rect: Rect::new(0, 0, 400, 300),
                col_count: 0,
                row_count: 0,
                col_widths: Vec::new(),
                col_aligns: Vec::new(),
                cells: HashMap::new(),
                provider: None,
                last_selection: None,
                on_sel_change: None,
                enabled: true,
                visible: true,
            })),
        };
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let h = grid.inner.borrow().hwnd;
            // Full-row select + visible grid lines for a classic
            // report-view look. DOUBLEBUFFER keeps the per-frame
            // redraws flicker-free when the sizer resizes the
            // control; HEADERDRAGDROP lets the user reorder the
            // columns at runtime. The `set_checkboxes(true)` call
            // toggles the CHECKBOXES bit on top of this base set.
            SendMessageW(
                h,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                0,
                (LVS_EX_FULLROWSELECT
                    | LVS_EX_GRIDLINES
                    | LVS_EX_DOUBLEBUFFER
                    | LVS_EX_HEADERDRAGDROP) as isize,
            );
        }
        // One-shot diagnostic: log the actual size of LVCOLUMNW so
        // we can spot struct-layout regressions at run-time.
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
                    "[grid] LVCOLUMNW size_of = {} bytes (expected 48 on x86 / 52 on x64)",
                    _LVCOLUMNW_SIZE
                );
            }
        }
        grid
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn new<W: Window>(_parent: &W) -> Self {
        Grid {
            inner: Rc::new(RefCell::new(GridInner {
                col_count: 0,
                row_count: 0,
                col_widths: Vec::new(),
                col_aligns: Vec::new(),
                cells: HashMap::new(),
                provider: None,
                last_selection: None,
                on_sel_change: None,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Append a column with the given title and width (pixels),
    /// left-aligned (the Win32 default). Equivalent to
    /// [`Self::append_column_with_align`] with `ColumnAlign::Left`.
    pub fn append_column(&self, title: &str, width: i32) {
        self.append_column_with_align(title, width, ColumnAlign::Left)
    }

    /// Append a column with the given title, width (pixels) and text
    /// alignment. Use [`ColumnAlign::Right`] for numeric values such as
    /// prices, [`ColumnAlign::Center`] for short labels such as ratings,
    /// and [`ColumnAlign::Left`] for free-form text (the default).
    pub fn append_column_with_align(&self, title: &str, width: i32, align: ColumnAlign) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // The `width` argument is a **logical** (96-DPI) pixel
            // value — the same coordinate system the user types in
            // for `Frame::with_size`. The Win32 ListView API, however,
            // works in **physical** pixels of the monitor the control
            // lives on. With `PerMonitorV2` DPI awareness (declared by
            // `app.manifest`) the OS does NOT auto-scale column
            // widths, so a logical 70-px column becomes 56 physical
            // px on a 1.25× display and the header is silently
            // truncated to a single character. Scale once here, pass
            // the physical value to Win32, but store the **logical**
            // value in `col_widths` so the user-facing value is
            // preserved across re-apply / resize cycles.
            let hwnd = self.inner.borrow().hwnd;
            let dpi = get_dpi_for_window(hwnd);
            let physical_w = dpi.scale(width);
            let wide = to_wide(title);
            let col = LVCOLUMNW {
                mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
                fmt: align.as_lvfmt() as i32,
                cx: physical_w,
                psz_text: wide.as_ptr(),
                cch_text_max: wide.len() as i32,
                i_sub_item: 0,
                i_image: 0,
                i_order: 0,
                cx_min: 0,
                cx_default: 0,
                cx_ideal: 0,
            };
            let idx = self.inner.borrow().col_count;
            SendMessageW(
                hwnd,
                LVM_INSERTCOLUMN,
                idx,
                &col as *const LVCOLUMNW as isize,
            );
            // Belt-and-suspenders: follow up with an explicit
            // LVM_SETCOLUMNWIDTH so the requested width is honored
            // even on systems where the `cx` field of LVCOLUMNW is
            // ignored (observed on Windows 11 25H2 with PerMonitorV2
            // DPI scaling, where every column collapsed to ~20 px
            // and the headers were truncated to a single character).
            SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, idx, physical_w as isize);
            // DIAGNOSTIC: read back the width so we can see whether
            // the SET was actually accepted or silently ignored.
            // (writes to a file because this is a GUI subsystem app
            // and has no console to print stderr to).
            let actual = SendMessageW(hwnd, LVM_GETCOLUMNWIDTH, idx, 0);
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("f:\\code\\ru_wx\\img\\grid_debug.log")
            {
                let _ = writeln!(
                    f,
                    "[grid] append_column idx={} requested_logical={} physical_w={} fmt={} after_SET={} (struct_size={})",
                    idx,
                    width,
                    physical_w,
                    align.as_lvfmt(),
                    actual,
                    std::mem::size_of::<LVCOLUMNW>()
                );
            }
            // Remember the requested width/align so we can re-apply
            // them if the control is later moved/resized by a sizer
            // (some ListView / DPI combinations silently collapse
            // column widths when the parent is moved). Store the
            // LOGICAL value so the user-facing value is preserved.
            {
                let mut i = self.inner.borrow_mut();
                i.col_widths.push(width);
                i.col_aligns.push(align);
            }
            // Force a full repaint of the visible items so the new
            // column header text + width takes effect immediately.
            SendMessageW(hwnd, LVM_REDRAWITEMS, 0, idx as isize);
            self.inner.borrow_mut().col_count += 1;
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (title, width, align);
    }

    /// Enable or disable the built-in checkbox column.
    ///
    /// When enabled, the ListView adds a state-image column at the
    /// far left of every row; the checked state of an item can be
    /// read / flipped with [`Self::is_checked`] and
    /// [`Self::set_checked`].
    pub fn set_checkboxes(&self, enabled: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            // Read the current extended styles, add or remove the
            // CHECKBOXES bit, and write them back so other bits
            // (GRIDLINES, FULLROWSELECT, …) are preserved.
            let current = SendMessageW(
                hwnd,
                LVM_GETEXTENDEDLISTVIEWSTYLE,
                0,
                0,
            );
            let new = if enabled {
                (current as u32) | LVS_EX_CHECKBOXES
            } else {
                (current as u32) & !LVS_EX_CHECKBOXES
            };
            SendMessageW(hwnd, LVM_SETEXTENDEDLISTVIEWSTYLE, 0, new as isize);
        }
    }

    /// Return whether the given row's checkbox is currently checked.
    /// Only meaningful when [`Self::set_checkboxes`] was called with
    /// `true`. Returns `false` for any out-of-range row.
    pub fn is_checked(&self, row: usize) -> bool {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let mut item = LVITEMW {
                mask: 0x0008, /* LVIF_STATE */
                i_item: row as i32,
                i_sub_item: 0,
                state: 0,
                state_mask: 0xF000, /* LVIS_STATEIMAGEMASK */
                psz_text: std::ptr::null_mut(),
                cch_text_max: 0,
                i_image: 0,
                l_param: 0,
                i_indent: 0,
                i_group_id: 0,
                c_columns: 0,
                pu_columns: std::ptr::null_mut(),
                pi_col_fmt: std::ptr::null_mut(),
                i_group: 0,
            };
            // LVM_GETITEM returns the requested state in `item.state`.
            let _r = SendMessageW(
                hwnd,
                LVM_GETITEM,
                0,
                &mut item as *mut LVITEMW as isize,
            );
            // The state-image index is stored in bits 12..15. Index
            // 2 = checked, anything else (incl. 1 = unchecked) = not
            // checked.
            ((item.state >> 12) & 0x0F) == 2
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = row;
            false
        }
    }

    /// Programmatically check or uncheck the checkbox for the given
    /// row. Only meaningful when [`Self::set_checkboxes`] was called
    /// with `true`.
    pub fn set_checked(&self, row: usize, checked: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            // State-image index: 0 = no checkbox (we never set this),
            // 1 = unchecked, 2 = checked. The image index lives in
            // bits 12..15 of the `state` field.
            let state_img_idx: u32 = if checked { 2 } else { 1 };
            let item = LVITEMW {
                mask: 0x0008, /* LVIF_STATE */
                i_item: row as i32,
                i_sub_item: 0,
                state: state_img_idx << 12,
                state_mask: 0xF000, /* LVIS_STATEIMAGEMASK */
                psz_text: std::ptr::null_mut(),
                cch_text_max: 0,
                i_image: 0,
                l_param: 0,
                i_indent: 0,
                i_group_id: 0,
                c_columns: 0,
                pu_columns: std::ptr::null_mut(),
                pi_col_fmt: std::ptr::null_mut(),
                i_group: 0,
            };
            SendMessageW(hwnd, LVM_SETITEM, 0, &item as *const LVITEMW as isize);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (row, checked);
    }

    /// Set the number of rows. Wipes any previously-existing rows.
    /// If a value provider has been set, the new rows are populated
    /// from it.
    pub fn set_row_count(&self, n: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            SendMessageW(hwnd, LVM_DELETEALLITEMS, 0, 0);

            // Insert empty rows (no text, no image) — `apply_cell` will
            // fill them in afterwards.
            for i in 0..n {
                let item = LVITEMW {
                    mask: 0,
                    i_item: i as i32,
                    i_sub_item: 0,
                    state: 0,
                    state_mask: 0,
                    psz_text: std::ptr::null_mut(),
                    cch_text_max: 0,
                    i_image: 0,
                    l_param: 0,
                    i_indent: 0,
                    i_group_id: 0,
                    c_columns: 0,
                    pu_columns: std::ptr::null_mut(),
                    pi_col_fmt: std::ptr::null_mut(),
                    i_group: 0,
                };
                SendMessageW(hwnd, LVM_INSERTITEM, 0, &item as *const LVITEMW as isize);
            }
        }
        self.inner.borrow_mut().row_count = n;
        self.inner.borrow_mut().cells.clear();
        self.inner.borrow_mut().last_selection = None;
        drop(self.inner.borrow_mut());
        self.refresh();
    }

    /// Set the value of one static cell. Ignored if a value provider
    /// has been set (the provider always wins).
    pub fn set_cell(&self, row: usize, col: usize, cell: Cell) {
        self.inner.borrow_mut().cells.insert((row, col), cell);
        self.apply_cell(row, col);
    }

    /// Install a closure that is called for every `(row, col)` to
    /// produce the cell value. Once installed, the provider takes
    /// priority over any cells set via [`Self::set_cell`].
    ///
    /// The provider is re-queried automatically by
    /// [`Self::set_row_count`] and [`Self::refresh`].
    pub fn set_value_provider<F>(&self, f: F)
    where
        F: Fn(usize, usize) -> Cell + 'static,
    {
        self.inner.borrow_mut().provider = Some(Box::new(f));
        self.refresh();
    }

    /// Re-query the value provider for every cell. Cheap, no-op if
    /// no provider has been set.
    pub fn refresh(&self) {
        let (n_rows, n_cols) = {
            let i = self.inner.borrow();
            (i.row_count, i.col_count)
        };
        for r in 0..n_rows {
            for c in 0..n_cols {
                self.apply_cell(r, c);
            }
        }
    }

    /// Re-apply every column's stored width and alignment to the
    /// underlying control. Call this after the control is moved or
    /// resized by a sizer if the column widths appear to have
    /// collapsed — some ListView / DPI combinations silently zero
    /// the widths when the parent is moved.
    pub fn apply_column_widths(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let i = self.inner.borrow();
            let hwnd = i.hwnd;
            // `col_widths` stores the LOGICAL (96-DPI) value the user
            // asked for. Convert to physical pixels for the Win32
            // call so the headers are fully visible on high-DPI
            // monitors (PerMonitorV2 does NOT auto-scale for us).
            let dpi = get_dpi_for_window(hwnd);
            for (idx, &w) in i.col_widths.iter().enumerate() {
                let physical_w = dpi.scale(w);
                SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, idx, physical_w as isize);
            }
        }
    }

    /// Force the ListView to repaint every visible item and the
    /// header. Useful after programmatically checking/unchecking
    /// rows in batch, or after a sizer has changed the visible
    /// geometry.
    pub fn force_refresh(&self) {
        self.refresh();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            // RedrawItems requires first/last item indices. Use
            // INT_MIN..INT_MAX so the entire range is repainted.
            SendMessageW(hwnd, LVM_REDRAWITEMS, i32::MIN as usize, i32::MAX as isize);
        }
    }

    /// Attach an image list to the small-icon slot. The grid will
    /// draw the image referenced by `Cell::Image { idx, .. }` or
    /// `Cell::ImageOnly(idx)` to the left of the cell text.
    pub fn set_image_list(&self, list: &ImageList) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            SendMessageW(hwnd, LVM_SETIMAGELIST, LVSIL_SMALL as usize, list.handle());
        }
    }

    /// Install a custom font on the control.
    ///
    /// The default `SysListView32` font is the system icon font,
    /// which on a 96 DPI screen is `Segoe UI 9pt`. That face is
    /// large enough that even a 200-px column header can elide to
    /// a single character in a tight layout. Use this method to
    /// pick a smaller / narrower face (e.g. `FontDesc::new("Segoe UI", 8)`)
    /// so every column header renders its full title.
    ///
    /// `redraw` is forwarded to `WM_SETFONT`'s `lParam` — pass
    /// `true` to repaint immediately, `false` to defer.
    ///
    /// Note: the report-view header is a *child* window of the
    /// listview and owns its own `HFONT`, so we also forward
    /// `WM_SETFONT` to it via `LVM_GETHEADER`. Without that, the
    /// cells would shrink to 8pt but the column titles would
    /// still be drawn in the system icon font and elide to a
    /// single character.
    pub fn set_font(&self, font: &Font, redraw: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI calls with validated arguments. `LVM_GETHEADER`
        // returns a real child `HWND` of the listview in report view, and
        // `WM_SETFONT` accepts any `HFONT` (including one selected by another
        // control — the kernel merely records it on the window).
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let hfont = font.hfont() as usize;
            eprintln!("[grid] set_font hwnd=0x{:x} hfont=0x{:x} redraw={}", hwnd as isize as u64, hfont, redraw);
            let prev = SendMessageW(hwnd, WM_GETFONT, 0, 0);
            eprintln!("[grid]   pre-set WM_GETFONT -> 0x{:x}", prev as isize as u64);
            let rv = SendMessageW(
                hwnd,
                WM_SETFONT,
                hfont,
                redraw as isize,
            );
            eprintln!("[grid]   post WM_SETFONT rv=0x{:x}", rv as isize as u64);
            let now = SendMessageW(hwnd, WM_GETFONT, 0, 0);
            eprintln!("[grid]   post-set WM_GETFONT -> 0x{:x}", now as isize as u64);
            // Forward the same HFONT to the header child so the column
            // titles render at the same metrics as the cell text.
            let header_hwnd = SendMessageW(
                hwnd,
                LVM_GETHEADER,
                0,
                0,
            ) as isize as HWND;
            eprintln!("[grid]   header_hwnd=0x{:x}", header_hwnd as isize as u64);
            if !header_hwnd.is_null() {
                let prev_h = SendMessageW(header_hwnd, WM_GETFONT, 0, 0);
                eprintln!("[grid]   header pre-set WM_GETFONT -> 0x{:x}", prev_h as isize as u64);
                SendMessageW(
                    header_hwnd,
                    WM_SETFONT,
                    hfont,
                    redraw as isize,
                );
                let now_h = SendMessageW(header_hwnd, WM_GETFONT, 0, 0);
                eprintln!("[grid]   header post-set WM_GETFONT -> 0x{:x}", now_h as isize as u64);
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (font, redraw);
    }

    /// Return the index of the currently selected row, if any.
    pub fn get_selected_row(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let r = SendMessageW(
                hwnd,
                LVM_GETNEXTITEM,
                -1i32 as isize as usize,
                LVNI_SELECTED as isize,
            );
            if r >= 0 {
                Some(r as usize)
            } else {
                None
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }

    /// Register a callback invoked whenever the row selection
    /// changes. The argument is the new selection (`Some(row)` or
    /// `None` when the selection is cleared).
    ///
    /// Internally this registers a notify handler with the parent
    /// [`Frame`]; the Grid's stored `last_selection` is used to
    /// debounce the duplicate `LVN_ITEMCHANGED` notifications that
    /// the control sends per click.
    pub fn on_selection_changed<F>(&self, frame: &Frame, f: F)
    where
        F: FnMut(Option<usize>) + 'static,
    {
        self.inner.borrow_mut().on_sel_change = Some(Box::new(f));

        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_notify_handler(
            id,
            Box::new(move |_code| {
                #[cfg(target_os = "windows")]
                {
                    let new_sel = {
                        let hwnd = inner.borrow().hwnd;
                        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                        let r = unsafe {
                            SendMessageW(
                                hwnd,
                                LVM_GETNEXTITEM,
                                -1i32 as isize as usize,
                                LVNI_SELECTED as isize,
                            )
                        };
                        if r >= 0 {
                            Some(r as usize)
                        } else {
                            None
                        }
                    };
                    let changed = {
                        let mut i = inner.borrow_mut();
                        if i.last_selection != new_sel {
                            i.last_selection = new_sel;
                            true
                        } else {
                            false
                        }
                    };
                    if changed {
                        let cb = inner.borrow_mut().on_sel_change.take();
                        if let Some(mut c) = cb {
                            c(new_sel);
                            inner.borrow_mut().on_sel_change = Some(c);
                        }
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = inner;
                }
            }),
        );
    }

    /// Number of rows currently configured.
    pub fn row_count(&self) -> usize {
        self.inner.borrow().row_count
    }
    /// Number of columns currently configured.
    pub fn col_count(&self) -> usize {
        self.inner.borrow().col_count
    }

    /// The control's Win32 ID.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }

    // ── internals ───────────────────────────────────────────────────

    /// Compute the cell value at `(row, col)` (provider takes priority
    /// over the static map) and push it to the underlying control.
    fn apply_cell(&self, row: usize, col: usize) {
        // Step 1: decide what to display. The provider closure may
        // borrow `inner`, so keep this scope tight.
        let cell: Cell = {
            let i = self.inner.borrow();
            if let Some(p) = i.provider.as_ref() {
                p(row, col)
            } else {
                i.cells.get(&(row, col)).cloned().unwrap_or(Cell::Empty)
            }
        };

        // Step 2: push it to the ListView.
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            if cell.has_text() {
                let wide = to_wide(&cell.text());
                let mut item = LVITEMW {
                    mask: LVIF_TEXT,
                    i_item: row as i32,
                    i_sub_item: col as i32,
                    state: 0,
                    state_mask: 0,
                    psz_text: wide.as_ptr() as *mut u16,
                    cch_text_max: wide.len() as i32,
                    i_image: 0,
                    l_param: 0,
                    i_indent: 0,
                    i_group_id: 0,
                    c_columns: 0,
                    pu_columns: std::ptr::null_mut(),
                    pi_col_fmt: std::ptr::null_mut(),
                    i_group: 0,
                };
                if let Some(idx) = cell.image() {
                    item.mask |= LVIF_IMAGE;
                    item.i_image = idx;
                }
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SendMessageW(hwnd, LVM_SETITEM, 0, &item as *const LVITEMW as isize);
                }
            } else if let Some(idx) = cell.image() {
                // Image-only cell: send a minimal item that just sets
                // the image.
                let item = LVITEMW {
                    mask: LVIF_IMAGE,
                    i_item: row as i32,
                    i_sub_item: col as i32,
                    state: 0,
                    state_mask: 0,
                    psz_text: std::ptr::null_mut(),
                    cch_text_max: 0,
                    i_image: idx,
                    l_param: 0,
                    i_indent: 0,
                    i_group_id: 0,
                    c_columns: 0,
                    pu_columns: std::ptr::null_mut(),
                    pi_col_fmt: std::ptr::null_mut(),
                    i_group: 0,
                };
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SendMessageW(hwnd, LVM_SETITEM, 0, &item as *const LVITEMW as isize);
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = cell;
        }
    }
}

// ── Widget trait ─────────────────────────────────────────────────────

impl Widget for GridInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        // ENTRY LOG — written FIRST, before any other work, so we can
        // see exactly when the sizer is laying us out.
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
                    "[grid] set_size ENTRY w={w} h={h} prev_w={} prev_h={} col_widths.len={}",
                    self.rect.width,
                    self.rect.height,
                    self.col_widths.len()
                );
            }
        }
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // `w` and `h` come from the sizer already in **physical**
            // pixels (the parent frame applied DPI scaling before
            // calling us), so pass them straight through to
            // `MoveWindow`. The column widths in `col_widths`, on the
            // other hand, are stored in **logical** pixels (the value
            // the user typed in `append_column_with_align`); convert
            // them to physical before sending to the Win32 API.
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
            // Re-apply stored column widths after a resize. On some
            // ListView / DPI combinations the control silently
            // collapses every column to ~20 px when it is moved,
            // and only an explicit `LVM_SETCOLUMNWIDTH` brings the
            // headers back. Cheap (one SendMessage per column) and
            // idempotent.
            use std::io::Write;
            let mut dbg = String::new();
            // `get_dpi_for_window` may fail for a brief moment
            // between `CreateWindowExW` and the first WM_NCCREATE
            // processing; in that window the OS returns a default
            // system DPI (96) for the freshly-created HWND. We
            // tolerate that by falling back to the system DPI.
            let dpi = get_dpi_for_window(self.hwnd);
            for (idx, &cw) in self.col_widths.iter().enumerate() {
                let physical_cw = dpi.scale(cw);
                SendMessageW(self.hwnd, LVM_SETCOLUMNWIDTH, idx, physical_cw as isize);
                let actual = SendMessageW(self.hwnd, LVM_GETCOLUMNWIDTH, idx, 0);
                use std::fmt::Write as _;
                let _ = write!(
                    dbg,
                    "  col {idx}: req_logical={cw} req_physical={physical_cw} actual={actual}\n"
                );
            }
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("f:\\code\\ru_wx\\img\\grid_debug.log")
            {
                let _ = writeln!(
                    f,
                    "[grid] set_size EXIT w={w} h={h} x={} y={} re-applied {} columns:\n{dbg}",
                    self.rect.x,
                    self.rect.y,
                    self.col_widths.len()
                );
            }
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }
    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}
