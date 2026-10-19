//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
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
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicU32, Ordering};

use crate::containers::grid_advanced::{
    is_merge_covered, register_merge, row_text_matches_filter, tsv_to_pdf_bytes,
};
use crate::containers::grid_cell_renderer::GridColumnRenderer;
use crate::containers::grid_coords::GridCoords;
use crate::containers::grid_range::GridRange;
use crate::containers::grid_table::GridTable;
use crate::core::font::{Font, FontDesc};
use crate::core::geometry::{Colour, Rect};
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::dc::image_list::ImageList;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use crate::dialogs::font_dialog::FontDialog;
use crate::window::frame::Frame;
use crate::window::popup_menu::PopupMenu;

#[cfg(target_os = "windows")]
use crate::platform::as_hwnd;
use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use crate::platform::ScaleByDpi;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::POINT;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{InvalidateRect, ScreenToClient};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetAsyncKeyState, SetFocus, VK_CONTROL, VK_DOWN, VK_F2, VK_LEFT, VK_RETURN,
    VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP, VK_V,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const ES_AUTOHSCROLL: u32 = 0x0080;
#[cfg(target_os = "windows")]
const EM_SETSEL: u32 = 0x00B1;

// ── Win32 ListView constants (kept local to match list_ctrl.rs) ──────

#[cfg(target_os = "windows")]
const LVM_FIRST: u32 = 0x1000;
// Use the **wide** ListView message variants. `LVM_FIRST + 5/6/7/27`
// are the ANSI (`...A`) versions; passing a `LVCOLUMNW` / `LVITEMW`
// with UTF-16 `pszText` to the ANSI handler makes the control read
// the text as single-byte and show only the first character.
#[cfg(target_os = "windows")]
const LVM_INSERTCOLUMN: u32 = LVM_FIRST + 97; // LVM_INSERTCOLUMNW
#[cfg(target_os = "windows")]
const LVM_INSERTITEM: u32 = LVM_FIRST + 77; // LVM_INSERTITEMW
#[cfg(target_os = "windows")]
const LVM_SETITEM: u32 = LVM_FIRST + 76; // LVM_SETITEMW
#[cfg(target_os = "windows")]
const LVM_DELETEALLITEMS: u32 = LVM_FIRST + 9;
#[cfg(target_os = "windows")]
const LVM_GETITEM: u32 = LVM_FIRST + 75; // LVM_GETITEMW
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
#[cfg(target_os = "windows")]
const LVM_DELETECOLUMN: u32 = LVM_FIRST + 98; // LVM_DELETECOLUMNW
#[cfg(target_os = "windows")]
const LVM_DELETEITEM: u32 = LVM_FIRST + 8;
#[cfg(target_os = "windows")]
const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
#[cfg(target_os = "windows")]
const LVM_SUBITEMHITTEST: u32 = LVM_FIRST + 87; // LVM_SUBITEMHITTESTW

/// `NM_CUSTOMDRAW` — parent must return `CDRF_*` flags.
#[cfg(target_os = "windows")]
pub(crate) const NM_CUSTOMDRAW: u32 = 0xFFFFFFF4;

#[cfg(target_os = "windows")]
const CDDS_PREPAINT: u32 = 0x0000_0001;
#[cfg(target_os = "windows")]
const CDDS_ITEMPREPAINT: u32 = 0x0000_0002;
#[cfg(target_os = "windows")]
const CDDS_SUBITEM: u32 = 0x0002_0000;
#[cfg(target_os = "windows")]
const CDRF_DODEFAULT: u32 = 0x0000_0000;
#[cfg(target_os = "windows")]
const CDRF_NEWFONT: u32 = 0x0000_0002;
#[cfg(target_os = "windows")]
const CDRF_NOTIFYITEMDRAW: u32 = 0x0000_0010;
#[cfg(target_os = "windows")]
const CDRF_NOTIFYSUBITEMDRAW: u32 = 0x0000_0020;
#[cfg(target_os = "windows")]
const CDIS_SELECTED: u32 = 0x0000_0001;

#[cfg(target_os = "windows")]
const HDM_FIRST: u32 = 0x1200;
#[cfg(target_os = "windows")]
const HDM_HITTEST: u32 = HDM_FIRST + 6;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::{
    HDF_SORTDOWN, HDF_SORTUP, HDITEMW, HDI_FORMAT, HDM_GETITEMW, HDM_SETITEMW,
    HDN_DIVIDERDBLCLICKW, HDN_ITEMCLICKW, LVM_ENSUREVISIBLE, LVM_GETCOLUMNORDERARRAY,
    LVM_GETSELECTEDCOUNT, LVM_SETCOLUMNORDERARRAY, LVN_GETINFOTIPW, LVN_ITEMACTIVATE,
    LVSCW_AUTOSIZE, LVSCW_AUTOSIZE_USEHEADER, LVS_EX_LABELTIP, NMHEADERW, NMITEMACTIVATE,
    NMLVGETINFOTIPW,
};

/// `LVN_ITEMACTIVATE` — double-click / Enter on a row.
#[cfg(target_os = "windows")]
pub(crate) const LVN_ITEMACTIVATE_GRID: u32 = LVN_ITEMACTIVATE;

/// `LVN_GETINFOTIPW` — ListView requests tooltip text for a cell.
#[cfg(target_os = "windows")]
pub(crate) const LVN_GETINFOTIP_GRID: u32 = LVN_GETINFOTIPW;

#[cfg(target_os = "windows")]
const LVM_SETITEMSTATE: u32 = LVM_FIRST + 43;
#[cfg(target_os = "windows")]
const LVIS_SELECTED: u32 = 0x0002;
#[cfg(target_os = "windows")]
const LVIS_FOCUSED: u32 = 0x0001;

/// Image list slot for the small (cell) image list.
#[cfg(target_os = "windows")]
const LVSIL_SMALL: u32 = 1;

/// Flag for `LVM_GETNEXTITEM`: the next item that has the LVIS_SELECTED state.
#[cfg(target_os = "windows")]
const LVNI_SELECTED: u32 = 2;

/// `LVN_ITEMCHANGED` — ListView notification when an item's state
/// changes (selection, focus, checkbox, …). Computed as
/// `LVN_FIRST - 1` = `(0U - 100U) - 1` = `0xFFFFFF9B`.
#[cfg(target_os = "windows")]
const LVN_ITEMCHANGED: u32 = 0xFFFFFF9B;

/// `WM_SETFONT` — install a custom `HFONT` on the control. Used by
/// [`Grid::set_font`] so the caller can pick a smaller / larger face
/// than the system default; the smaller face is the difference
/// between column headers truncating to a single character and
/// rendering their full title in a tight layout.
#[cfg(target_os = "windows")]
const WM_SETFONT: u32 = 0x0030;

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

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct NmHdr {
    hwnd_from: HWND,
    id_from: usize,
    code: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct NmCustomDraw {
    hdr: NmHdr,
    dw_draw_stage: u32,
    hdc: isize,
    rc: RECT,
    dw_item_spec: usize,
    u_item_state: u32,
    l_item_l_param: isize,
    clr_text: u32,
    clr_text_bk: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct NmLvCustomDraw {
    nmcd: NmCustomDraw,
    dw_item_type: u32,
    i_sub_item: i32,
    dw_state: u32,
    dw_state_mask: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct LvHitTestInfo {
    pt: POINT,
    flags: u32,
    i_item: i32,
    i_sub_item: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct HdHitTestInfo {
    pt: POINT,
    flags: u32,
    i_item: i32,
}

// ── Cell value ───────────────────────────────────────────────────────

/// Sort direction for [`Grid::sort_by_column`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortOrder {
    /// No active sort.
    None,
    /// A → Z / smallest → largest.
    Ascending,
    /// Z → A / largest → smallest.
    Descending,
}

/// How [`Grid::sort_by_column`] compares values in a column.
///
/// With [`ColumnSortKind::Auto`] (the default) the grid inspects each
/// [`Cell`] variant and sorts numbers, dates, progress bars, star
/// ratings and priorities by their underlying value rather than the
/// rendered text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColumnSortKind {
    /// Infer from [`Cell`] type (recommended).
    #[default]
    Auto,
    /// Case-insensitive lexical order on display text.
    Text,
    /// Numeric order (`1,234.5`, `€ 10.00`, …).
    Number,
    /// Integer order (truncates fractional part).
    Integer,
    /// Chronological order using ISO `YYYY-MM-DD` strings when present.
    Date,
}

/// Optional foreground / background colours for one cell, row, or the
/// whole grid (via [`Grid::set_alternating_row_colors`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridCellStyle {
    /// Text colour. `None` keeps the system default.
    pub foreground: Option<Colour>,
    /// Cell background colour. `None` keeps the system default.
    pub background: Option<Colour>,
}

/// Colour palette for the grid body, header, and selection highlight.
///
/// Apply with [`Grid::set_appearance`] (optionally passing a [`Frame`]
/// so custom-draw hooks are installed). Presets:
/// [`Self::win11`], [`Self::classic`], [`Self::modern`], [`Self::warm`],
/// [`Self::dark`].
///
/// When [`Self::system_theme`] is `true` the control uses the native
/// Windows 11 Explorer visual style; only explicit per-row / per-cell
/// overrides from [`Grid::set_row_style`] / [`Grid::set_cell_style`]
/// are painted on top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridAppearance {
    /// Use the OS visual style (`SetWindowTheme` + no global custom colours).
    pub system_theme: bool,
    /// Background for even display rows.
    pub alternating_even: Colour,
    /// Background for odd display rows.
    pub alternating_odd: Colour,
    /// Default cell text colour (when no per-cell override exists).
    pub default_text: Colour,
    /// Column-header text colour.
    pub header_foreground: Colour,
    /// Column-header background colour.
    pub header_background: Colour,
    /// Text colour of the selected row.
    pub selection_foreground: Colour,
    /// Background colour of the selected row.
    pub selection_background: Colour,
    /// Height of the column-header row (pixels).
    pub header_height: i32,
    /// Default row height (pixels).
    pub row_height: i32,
}

impl GridAppearance {
    /// Native Windows 11 Explorer list + header (recommended default).
    pub fn win11() -> Self {
        Self {
            system_theme: true,
            alternating_even: Colour::new(255, 255, 255, 255),
            alternating_odd: Colour::new(255, 255, 255, 255),
            default_text: Colour::new(0, 0, 0, 255),
            header_foreground: Colour::new(0, 0, 0, 255),
            header_background: Colour::new(255, 255, 255, 255),
            selection_foreground: Colour::new(255, 255, 255, 255),
            selection_background: crate::core::tokens::system_accent_colour(),
            header_height: 24,
            row_height: 22,
        }
    }

    /// White / light-grey stripes, system-like selection.
    pub fn classic() -> Self {
        Self {
            system_theme: false,
            alternating_even: Colour::new(255, 255, 255, 255),
            alternating_odd: Colour::new(243, 246, 252, 255),
            default_text: Colour::new(33, 37, 41, 255),
            header_foreground: Colour::new(33, 37, 41, 255),
            header_background: Colour::new(233, 236, 239, 255),
            selection_foreground: Colour::new(255, 255, 255, 255),
            selection_background: crate::core::tokens::system_accent_colour(),
            header_height: 24,
            row_height: 22,
        }
    }

    /// Crisp blue header, soft stripes, vivid selection.
    pub fn modern() -> Self {
        Self {
            system_theme: false,
            alternating_even: Colour::new(255, 255, 255, 255),
            alternating_odd: Colour::new(248, 250, 252, 255),
            default_text: Colour::new(30, 41, 59, 255),
            header_foreground: Colour::new(255, 255, 255, 255),
            header_background: Colour::new(30, 64, 175, 255),
            selection_foreground: Colour::new(255, 255, 255, 255),
            selection_background: Colour::new(59, 130, 246, 255),
            header_height: 28,
            row_height: 24,
        }
    }

    /// Cream stripes with an amber header bar.
    pub fn warm() -> Self {
        Self {
            system_theme: false,
            alternating_even: Colour::new(255, 253, 247, 255),
            alternating_odd: Colour::new(254, 243, 226, 255),
            default_text: Colour::new(68, 47, 32, 255),
            header_foreground: Colour::new(255, 251, 235, 255),
            header_background: Colour::new(180, 83, 9, 255),
            selection_foreground: Colour::new(255, 251, 235, 255),
            selection_background: Colour::new(217, 119, 6, 255),
            header_height: 26,
            row_height: 24,
        }
    }

    /// Dark stripes for low-light environments.
    pub fn dark() -> Self {
        Self {
            system_theme: false,
            alternating_even: Colour::new(38, 42, 48, 255),
            alternating_odd: Colour::new(48, 53, 60, 255),
            default_text: Colour::new(226, 232, 240, 255),
            header_foreground: Colour::new(241, 245, 249, 255),
            header_background: Colour::new(15, 23, 42, 255),
            selection_foreground: Colour::new(255, 255, 255, 255),
            selection_background: Colour::new(71, 85, 105, 255),
            header_height: 22,
            row_height: 20,
        }
    }
}

impl Default for GridAppearance {
    fn default() -> Self {
        Self::win11()
    }
}

/// Column alignment. Maps to the `fmt` field of `LVCOLUMNW`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColumnAlign {
    /// Text hugs the left edge of the column.
    #[default]
    Left,
    /// Text is centered horizontally inside the column.
    Center,
    /// Text hugs the right edge of the column (good for numeric values).
    Right,
}

impl ColumnAlign {
    /// Map to the `LVCFMT_*` flag the Win32 SDK expects in the `fmt`
    /// field of `LVCOLUMNW`.
    #[cfg(target_os = "windows")]
    fn as_lvfmt(self) -> u32 {
        self.as_grid_align() as u32
    }

    /// Column alignment code shared by Win32 and AppKit backends
    /// (`0` = left, `1` = right, `2` = center).
    fn as_grid_align(self) -> i32 {
        match self {
            ColumnAlign::Left => 0,
            ColumnAlign::Right => 1,
            ColumnAlign::Center => 2,
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

    fn sort_rank(self) -> u8 {
        match self {
            BadgeKind::Neutral => 0,
            BadgeKind::Ok => 1,
            BadgeKind::Hot => 2,
            BadgeKind::Warn => 3,
            BadgeKind::Bad => 4,
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
            NumberFormat::Fixed2 => Self::format_with_sep(value, ',', '.', true),
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
        if always_2dp || frac_part > 0 {
            format!("{}{}{}{:02}", sign, int_grouped, dec, frac_part.abs())
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
    /// `07/11/2025` — European / day-first (alias for [`GridDateFormat::Eu`]).
    European,
    /// `11/07/2025` — US / month-first.
    Us,
    /// `11/07/2025` — US / month-first (alias for [`GridDateFormat::Us`]).
    American,
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
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTH_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
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
            GridDateFormat::Eu | GridDateFormat::European => format!("{:02}/{:02}/{:04}", d, m, y),
            GridDateFormat::Us | GridDateFormat::American => format!("{:02}/{:02}/{:04}", m, d, y),
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
                    format!("{} {}, {} {}:{} {}", MONTH_SHORT[mi], d, y, h12, mm, ampm)
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

    fn sort_rank(self) -> u8 {
        match self {
            PriorityKind::None => 0,
            PriorityKind::Low => 1,
            PriorityKind::Medium => 2,
            PriorityKind::High => 3,
            PriorityKind::Critical => 4,
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
    /// plain `Text` cell. Call [`Grid::enable_link_clicks`] (or
    /// [`Grid::enable_interactive_features`]) so a single click opens
    /// `url` in the system default browser.
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
    /// Plain-text rendering used by grid painting and [`super::grid_cell_renderer::GridCellRenderer`].
    pub fn display_text(&self) -> String {
        self.text()
    }

    pub fn text(&self) -> String {
        match self {
            Cell::Text(t) => t.clone(),
            Cell::Image { text, .. } => text.clone(),
            Cell::MultiLine(t) => t.replace('\n', "\r"),
            Cell::Progress { value, max, label } => {
                Self::render_bar(*value, *max, 10, BarStyle::Solid, label.as_deref())
            }
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
        let bar: String = fc.to_string().repeat(filled) + &ec.to_string().repeat(empty);
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

type RowContextMenuHandler = Box<dyn FnMut(&Frame, usize, usize)>;
type SortChangedHandler = Box<dyn FnMut(Option<usize>, Option<SortOrder>)>;
type RowActivatedHandler = Box<dyn FnMut(usize, usize)>;
type ColumnContextMenuHandler = Box<dyn FnMut(&Frame, usize)>;
type SelChangeHandler = Box<dyn FnMut(Option<usize>)>;
type SelRowsChangeHandler = Box<dyn FnMut(&[usize])>;
type FocusedCellChangedHandler = Box<dyn FnMut(usize, usize)>;
type CellEditedHandler = Box<dyn FnMut(usize, usize, &str) -> bool>;
#[cfg(target_os = "windows")]
type CellTooltipProvider = Box<dyn Fn(usize, usize) -> Option<String>>;

struct GridInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    header_hwnd: HWND,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    rect: Rect,
    col_count: usize,
    row_count: usize,
    /// Column titles, parallel to `col_widths`.
    col_titles: Vec<String>,
    /// Width (in pixels) of each column, in insertion order. Stored
    /// so we can re-apply the widths after the control is resized
    /// by the sizer — some ListView / DPI combinations silently
    /// collapse column widths when the control is moved and the
    /// header does not refresh itself.
    col_widths: Vec<i32>,
    /// Alignment of each column, parallel to `col_widths`.
    col_aligns: Vec<ColumnAlign>,
    /// Sort comparison mode for each column (parallel to `col_widths`).
    col_sort_kinds: Vec<ColumnSortKind>,
    /// Per-cell colour overrides.
    cell_styles: HashMap<(usize, usize), GridCellStyle>,
    /// Per-row colour overrides (applied when no cell override exists).
    row_styles: HashMap<usize, GridCellStyle>,
    /// Alternating even / odd row backgrounds.
    alternating_rows: Option<(Colour, Colour)>,
    /// Body / header / selection palette.
    appearance: GridAppearance,
    /// Current logical font description.
    font_desc: FontDesc,
    /// Keeps the active `HFONT` alive between `set_font` calls.
    #[cfg(target_os = "windows")]
    font: Option<Font>,
    /// Whether list subclass + `NM_CUSTOMDRAW` are wired.
    #[cfg(target_os = "windows")]
    visual_hooks_enabled: bool,
    /// Frame used for custom-draw registration (kept for theme changes).
    #[cfg(target_os = "windows")]
    visual_frame: Option<Frame>,
    /// Column index targeted by the last header / body context menu.
    context_menu_col: usize,
    /// Display row targeted by the last row context menu.
    context_menu_row: usize,
    /// Column under the cursor for the last row context menu.
    context_menu_row_col: usize,
    /// Active sort column / direction (drives header sort arrows).
    sort_col: Option<usize>,
    sort_order: Option<SortOrder>,
    /// Click column header to sort (`HDN_ITEMCLICK`).
    header_click_sort: bool,
    /// Click row/column reference cells to highlight the full row or column.
    row_col_highlight_enabled: bool,
    /// Display row highlighted via the row reference cell (checkbox / col 0).
    highlighted_row: Option<usize>,
    /// Data column highlighted via the column header.
    highlighted_col: Option<usize>,
    /// Right-click on a data row opens the row context menu.
    row_context_menu_enabled: bool,
    /// Serial used to name dynamically added columns.
    next_dynamic_col: usize,
    /// Whether [`Grid::enable_column_context_menu`] has been called.
    context_menu_enabled: bool,
    /// Frame passed to [`Grid::enable_column_context_menu`] (menu owner).
    context_frame: Option<Frame>,
    /// Invoked on row right-click when [`Self::row_context_menu_enabled`].
    on_row_context_menu: Option<RowContextMenuHandler>,
    /// Invoked on column-header right-click when [`Self::context_menu_enabled`].
    /// When unset, [`Self::popup_column_context_menu`] is shown.
    on_column_context_menu: Option<ColumnContextMenuHandler>,
    /// Fired when sort column / direction changes (header click or API).
    on_sort_changed: Option<SortChangedHandler>,
    /// Fired on double-click / Enter (`LVN_ITEMACTIVATE`).
    on_row_activated: Option<RowActivatedHandler>,
    #[cfg(target_os = "windows")]
    row_activate_hooks_enabled: bool,
    /// Optional per-cell tooltip text (`LVN_GETINFOTIPW`).
    #[cfg(target_os = "windows")]
    cell_tooltip_provider: Option<CellTooltipProvider>,
    #[cfg(target_os = "windows")]
    infotip_hooks_enabled: bool,
    /// Maps each **display** row index to the **logical** (data) row
    /// that supplies its cell values. After [`Grid::sort_by_column`]
    /// this is a permutation of `0..row_count`; otherwise it is the
    /// identity `0, 1, 2, …`.
    row_perm: Vec<usize>,
    /// Static cells set via `set_cell`. The provider (if set) takes
    /// priority over this. Keys use **logical** row indices.
    cells: HashMap<(usize, usize), Cell>,
    /// Optional closure that, if set, is queried for every (row, col).
    provider: Option<Box<dyn Fn(usize, usize) -> Cell>>,
    /// Tracks the last selection so that the selection-changed callback
    /// fires only on actual changes (the control fires two
    /// `LVN_ITEMCHANGED` notifications per click).
    last_selection: Option<usize>,
    on_sel_change: Option<SelChangeHandler>,
    /// Debounced multi-row selection snapshot.
    last_selection_rows: Vec<usize>,
    on_sel_rows_change: Option<SelRowsChangeHandler>,
    /// Hidden data columns (parallel to `col_widths`).
    col_hidden: Vec<bool>,
    /// Optional leading row-number column (display-only).
    row_labels_enabled: bool,
    row_label_width: i32,
    /// Inline cell editing (F2 / double-click on text cells).
    cell_editing_enabled: bool,
    /// Focused cell using **display** row and **data** column indices.
    focused_cell: Option<(usize, usize)>,
    on_focused_cell_changed: Option<FocusedCellChangedHandler>,
    on_cell_edited: Option<CellEditedHandler>,
    #[cfg(target_os = "windows")]
    /// Extra ListView columns inserted before data columns (row labels).
    lv_leading_cols: usize,
    #[cfg(target_os = "windows")]
    active_editor: Option<WinGridEditor>,
    /// Logical row height in pixels (macOS exact; Windows via font scaling).
    row_height: f64,
    /// Active inline editor state (`GridCellEditor`).
    cell_editor: Option<crate::GridCellEditor>,
    /// Global text filter (lowercase substring match across all columns).
    filter_query: String,
    /// Per-column lowercase substring filters.
    column_filters: HashMap<usize, String>,
    /// Merge anchors `(logical_row, col) → (row_span, col_span)`.
    merge_spans: HashMap<(usize, usize), (usize, usize)>,
    /// Optional live column renderers (override display text).
    col_renderers: HashMap<usize, Rc<dyn GridColumnRenderer>>,
    /// Optional wx-style table model (wired via [`Grid::set_grid_table`]).
    grid_table: Option<Rc<RefCell<dyn GridTable>>>,
    /// Number of leading **data** columns kept frozen while scrolling.
    frozen_col_count: usize,
    /// Number of leading **display** rows kept frozen (reserved; row freeze uses offset).
    frozen_row_count: usize,
    /// Drag a row reference cell to reorder display rows.
    row_drag_enabled: bool,
    on_row_reordered: Option<Box<dyn FnMut(usize, usize)>>,
    /// Single-click on [`Cell::Link`] opens the URL in the default browser.
    link_clicks_enabled: bool,
    #[cfg(target_os = "windows")]
    /// Companion ListView for frozen columns (may equal `hwnd` when none frozen).
    frozen_hwnd: HWND,
    #[cfg(target_os = "macos")]
    frozen_stub_handle: crate::platform::NativeHandle,
    enabled: bool,
    visible: bool,
    /// Hub di notifiche nativo registrato (una sola volta) sulla frame.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    notify_hub: bool,
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct WinGridEditor {
    hwnd: HWND,
    display_row: usize,
    col: usize,
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
            let parent_hwnd = crate::platform::window_handle_as_hwnd(parent);
            let wide_class = to_wide("SysListView32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | LVS_REPORT,
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
                header_hwnd: std::ptr::null_mut(),
                id,
                rect: Rect::new(0, 0, 400, 300),
                col_count: 0,
                row_count: 0,
                col_titles: Vec::new(),
                col_widths: Vec::new(),
                col_aligns: Vec::new(),
                col_sort_kinds: Vec::new(),
                cell_styles: HashMap::new(),
                row_styles: HashMap::new(),
                alternating_rows: None,
                appearance: GridAppearance::default(),
                font_desc: FontDesc::default(),
                font: None,
                visual_hooks_enabled: false,
                visual_frame: None,
                context_menu_col: 0,
                context_menu_row: 0,
                context_menu_row_col: 0,
                sort_col: None,
                sort_order: None,
                header_click_sort: false,
                row_col_highlight_enabled: false,
                highlighted_row: None,
                highlighted_col: None,
                row_context_menu_enabled: false,
                next_dynamic_col: 1,
                context_menu_enabled: false,
                context_frame: None,
                on_row_context_menu: None,
                on_column_context_menu: None,
                on_sort_changed: None,
                on_row_activated: None,
                row_activate_hooks_enabled: false,
                cell_tooltip_provider: None,
                infotip_hooks_enabled: false,
                row_perm: Vec::new(),
                cells: HashMap::new(),
                provider: None,
                last_selection: None,
                on_sel_change: None,
                last_selection_rows: Vec::new(),
                on_sel_rows_change: None,
                col_hidden: Vec::new(),
                row_labels_enabled: false,
                row_label_width: 40,
                cell_editing_enabled: false,
                focused_cell: None,
                on_focused_cell_changed: None,
                on_cell_edited: None,
                lv_leading_cols: 0,
                active_editor: None,
                row_height: 24.0,
                cell_editor: None,
                filter_query: String::new(),
                column_filters: HashMap::new(),
                merge_spans: HashMap::new(),
                col_renderers: HashMap::new(),
                grid_table: None,
                frozen_col_count: 0,
                frozen_row_count: 0,
                row_drag_enabled: false,
                on_row_reordered: None,
                link_clicks_enabled: false,
                frozen_hwnd: hwnd,
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
        let _ = _LVCOLUMNW_SIZE;
        grid.apply_explorer_theme();
        grid
    }

    /// Backend GTK4 (Linux): TreeView + ListStore reali.
    #[cfg(target_os = "linux")]
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();
        let parent_handle = crate::platform::window_handle(parent);
        let stub_handle =
            crate::platform::gtk::create_grid_ctrl(parent_handle, id, Rect::new(0, 0, 400, 300));
        Grid {
            inner: Rc::new(RefCell::new(GridInner {
                stub_handle,
                parent_handle,
                id,
                rect: Rect::new(0, 0, 400, 300),
                col_count: 0,
                row_count: 0,
                col_titles: Vec::new(),
                col_widths: Vec::new(),
                col_aligns: Vec::new(),
                col_sort_kinds: Vec::new(),
                cell_styles: HashMap::new(),
                row_styles: HashMap::new(),
                alternating_rows: None,
                appearance: GridAppearance::default(),
                font_desc: FontDesc::default(),
                context_menu_col: 0,
                context_menu_row: 0,
                context_menu_row_col: 0,
                sort_col: None,
                sort_order: None,
                header_click_sort: false,
                row_col_highlight_enabled: false,
                highlighted_row: None,
                highlighted_col: None,
                row_context_menu_enabled: false,
                next_dynamic_col: 1,
                context_menu_enabled: false,
                context_frame: None,
                on_row_context_menu: None,
                on_column_context_menu: None,
                on_sort_changed: None,
                on_row_activated: None,
                row_perm: Vec::new(),
                cells: HashMap::new(),
                provider: None,
                last_selection: None,
                on_sel_change: None,
                last_selection_rows: Vec::new(),
                on_sel_rows_change: None,
                col_hidden: Vec::new(),
                row_labels_enabled: false,
                row_label_width: 40,
                cell_editing_enabled: false,
                focused_cell: None,
                on_focused_cell_changed: None,
                on_cell_edited: None,
                row_height: 24.0,
                cell_editor: None,
                filter_query: String::new(),
                column_filters: HashMap::new(),
                merge_spans: HashMap::new(),
                col_renderers: HashMap::new(),
                grid_table: None,
                frozen_col_count: 0,
                frozen_row_count: 0,
                row_drag_enabled: false,
                on_row_reordered: None,
                link_clicks_enabled: false,
                enabled: true,
                visible: true,
                notify_hub: false,
            })),
        }
    }

    /// Fallback in-memory per piattaforme senza backend dedicato.
    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    pub fn new<W: Window>(_parent: &W) -> Self {
        Grid {
            inner: Rc::new(RefCell::new(GridInner {
                id: 0,
                rect: Rect::new(0, 0, 300, 200),
                col_count: 0,
                row_count: 0,
                col_titles: Vec::new(),
                col_widths: Vec::new(),
                col_aligns: Vec::new(),
                col_sort_kinds: Vec::new(),
                cell_styles: HashMap::new(),
                row_styles: HashMap::new(),
                alternating_rows: None,
                appearance: GridAppearance::default(),
                font_desc: FontDesc::default(),
                context_menu_col: 0,
                context_menu_row: 0,
                context_menu_row_col: 0,
                sort_col: None,
                sort_order: None,
                header_click_sort: false,
                row_col_highlight_enabled: false,
                highlighted_row: None,
                highlighted_col: None,
                row_context_menu_enabled: false,
                next_dynamic_col: 1,
                context_menu_enabled: false,
                context_frame: None,
                on_row_context_menu: None,
                on_column_context_menu: None,
                on_sort_changed: None,
                on_row_activated: None,
                row_perm: Vec::new(),
                cells: HashMap::new(),
                provider: None,
                last_selection: None,
                on_sel_change: None,
                last_selection_rows: Vec::new(),
                on_sel_rows_change: None,
                col_hidden: Vec::new(),
                row_labels_enabled: false,
                row_label_width: 40,
                cell_editing_enabled: false,
                focused_cell: None,
                on_focused_cell_changed: None,
                on_cell_edited: None,
                row_height: 24.0,
                cell_editor: None,
                filter_query: String::new(),
                column_filters: HashMap::new(),
                merge_spans: HashMap::new(),
                col_renderers: HashMap::new(),
                grid_table: None,
                frozen_col_count: 0,
                frozen_row_count: 0,
                row_drag_enabled: false,
                on_row_reordered: None,
                link_clicks_enabled: false,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// macOS AppKit grid.
    #[cfg(target_os = "macos")]
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();
        let parent_handle = crate::platform::window_handle(parent);
        let stub_handle =
            crate::platform::appkit::create_grid_ctrl(parent_handle, id, Rect::new(0, 0, 400, 300));
        Grid {
            inner: Rc::new(RefCell::new(GridInner {
                stub_handle,
                parent_handle,
                id,
                rect: Rect::new(0, 0, 400, 300),
                col_count: 0,
                row_count: 0,
                col_titles: Vec::new(),
                col_widths: Vec::new(),
                col_aligns: Vec::new(),
                col_sort_kinds: Vec::new(),
                cell_styles: HashMap::new(),
                row_styles: HashMap::new(),
                alternating_rows: None,
                appearance: GridAppearance::default(),
                font_desc: FontDesc::default(),
                context_menu_col: 0,
                context_menu_row: 0,
                context_menu_row_col: 0,
                sort_col: None,
                sort_order: None,
                header_click_sort: false,
                row_col_highlight_enabled: false,
                highlighted_row: None,
                highlighted_col: None,
                row_context_menu_enabled: false,
                next_dynamic_col: 1,
                context_menu_enabled: false,
                context_frame: None,
                on_row_context_menu: None,
                on_column_context_menu: None,
                on_sort_changed: None,
                on_row_activated: None,
                row_perm: Vec::new(),
                cells: HashMap::new(),
                provider: None,
                last_selection: None,
                on_sel_change: None,
                last_selection_rows: Vec::new(),
                on_sel_rows_change: None,
                col_hidden: Vec::new(),
                row_labels_enabled: false,
                row_label_width: 40,
                cell_editing_enabled: false,
                focused_cell: None,
                on_focused_cell_changed: None,
                on_cell_edited: None,
                row_height: 24.0,
                cell_editor: None,
                filter_query: String::new(),
                column_filters: HashMap::new(),
                merge_spans: HashMap::new(),
                col_renderers: HashMap::new(),
                grid_table: None,
                frozen_col_count: 0,
                frozen_row_count: 0,
                row_drag_enabled: false,
                on_row_reordered: None,
                link_clicks_enabled: false,
                frozen_stub_handle: stub_handle,
                enabled: true,
                visible: true,
                notify_hub: false,
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
            let dpi = GetDpiForWindow(hwnd);
            let physical_w = dpi.scale(width as u32) as i32;
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
            // Force a full repaint so the new column header + width
            // take effect immediately.
            SendMessageW(hwnd, LVM_REDRAWITEMS, 0, i32::MAX as isize);
        }
        // Remember the requested width/align (logical pixels) so we
        // can re-apply them after a sizer resize.
        let apply_theme = {
            let mut i = self.inner.borrow_mut();
            i.col_titles.push(title.to_string());
            i.col_widths.push(width);
            i.col_aligns.push(align);
            i.col_sort_kinds.push(ColumnSortKind::Auto);
            i.col_hidden.push(false);
            i.col_count += 1;
            #[cfg(target_os = "windows")]
            {
                if i.header_hwnd.is_null() {
                    // SAFETY: report-view ListView owns a header child.
                    i.header_hwnd =
                        unsafe { SendMessageW(i.hwnd, LVM_GETHEADER, 0, 0) as isize as HWND };
                    if i.context_menu_enabled && !i.header_hwnd.is_null() {
                        install_header_subclass(i.header_hwnd, self.inner.clone());
                    }
                }
            }
            i.appearance.system_theme
        };
        #[cfg(target_os = "windows")]
        if apply_theme {
            self.apply_explorer_theme();
        }
        #[cfg(not(target_os = "windows"))]
        let _ = apply_theme;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_append_column(
                self.inner.borrow().stub_handle,
                title,
                width,
                align.as_grid_align(),
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_append_column(
                self.inner.borrow().stub_handle,
                title,
                width,
                align.as_grid_align(),
            );
        }
    }

    /// Return the title of column `col`, if it exists.
    pub fn column_title(&self, col: usize) -> Option<String> {
        self.inner.borrow().col_titles.get(col).cloned()
    }

    /// Remove column `col`. Returns `false` if `col` is out of range
    /// or if it is the last remaining column (at least one column
    /// must stay visible).
    pub fn remove_column(&self, col: usize) -> bool {
        let mut i = self.inner.borrow_mut();
        if col >= i.col_count || i.col_count <= 1 {
            return false;
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI with a live ListView HWND and valid column index.
        unsafe {
            SendMessageW(i.hwnd, LVM_DELETECOLUMN, col, 0);
        }
        i.col_titles.remove(col);
        i.col_widths.remove(col);
        i.col_aligns.remove(col);
        i.col_sort_kinds.remove(col);
        i.col_hidden.remove(col);
        i.col_count -= 1;
        let old_cells = std::mem::take(&mut i.cells);
        for ((r, c), cell) in old_cells {
            if c == col {
                continue;
            }
            let nc = if c > col { c - 1 } else { c };
            i.cells.insert((r, nc), cell);
        }
        let old_styles = std::mem::take(&mut i.cell_styles);
        for ((r, c), style) in old_styles {
            if c == col {
                continue;
            }
            let nc = if c > col { c - 1 } else { c };
            i.cell_styles.insert((r, nc), style);
        }
        if i.context_menu_col >= i.col_count {
            i.context_menu_col = i.col_count.saturating_sub(1);
        }
        drop(i);
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            if !crate::platform::appkit::grid_remove_column(handle, col) {
                return false;
            }
        }
        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            if !crate::platform::gtk::grid_remove_column(handle, col) {
                return false;
            }
        }
        self.refresh();
        true
    }

    /// Append a new column with a generated title (`Colonna N`).
    pub fn append_dynamic_column(&self, width: i32) {
        let title = {
            let mut i = self.inner.borrow_mut();
            let n = i.next_dynamic_col;
            i.next_dynamic_col += 1;
            format!("Colonna {n}")
        };
        self.append_column_with_align(&title, width, ColumnAlign::Left);
    }

    /// Override the colours of a single cell.
    pub fn set_cell_style(&self, row: usize, col: usize, style: GridCellStyle) {
        self.inner
            .borrow_mut()
            .cell_styles
            .insert((row, col), style);
        #[cfg(target_os = "windows")]
        self.invalidate_view();
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::grid_set_cell_style(
                handle,
                row,
                col,
                style.foreground.map(colour_rgba),
                style.background.map(colour_rgba),
            );
        }
    }

    /// Override the colours of an entire row.
    pub fn set_row_style(&self, row: usize, style: GridCellStyle) {
        self.inner.borrow_mut().row_styles.insert(row, style);
        #[cfg(target_os = "windows")]
        self.invalidate_view();
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::grid_set_row_style(
                handle,
                row,
                style.foreground.map(colour_rgba),
                style.background.map(colour_rgba),
            );
        }
    }

    /// Paint even rows with `even` and odd rows with `odd`.
    pub fn set_alternating_row_colors(&self, even: Colour, odd: Colour) {
        self.inner.borrow_mut().alternating_rows = Some((even, odd));
        #[cfg(target_os = "windows")]
        self.invalidate_view();
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_alternating_colors(
                self.inner.borrow().stub_handle,
                colour_rgba(even),
                colour_rgba(odd),
            );
        }
    }

    /// Return the active colour palette.
    pub fn appearance(&self) -> GridAppearance {
        self.inner.borrow().appearance
    }

    /// Apply a full colour palette (stripes, header, selection, text).
    /// Pass `frame` on first use so custom-draw hooks are installed.
    pub fn set_appearance(&self, appearance: GridAppearance, frame: Option<&Frame>) {
        {
            let mut i = self.inner.borrow_mut();
            i.appearance = appearance;
            if appearance.system_theme {
                i.alternating_rows = None;
            } else {
                i.alternating_rows =
                    Some((appearance.alternating_even, appearance.alternating_odd));
            }
        }
        #[cfg(target_os = "windows")]
        {
            if appearance.system_theme {
                self.apply_explorer_theme();
            } else {
                self.clear_explorer_theme();
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(f) = frame {
                self.ensure_visual_hooks(f);
            } else if let Some(f) = self.inner.borrow().visual_frame.clone() {
                self.ensure_visual_hooks(&f);
            }
            self.repaint_after_theme_change();
        }
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::grid_set_appearance(handle, appearance_to_appkit(appearance));
            if appearance.system_theme {
                crate::platform::appkit::grid_clear_alternating_colors(handle);
            } else if let Some((even, odd)) = self.inner.borrow().alternating_rows {
                crate::platform::appkit::grid_set_alternating_colors(
                    handle,
                    colour_rgba(even),
                    colour_rgba(odd),
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = frame;
    }

    /// Apply the native Windows 11 Explorer visual style.
    pub fn apply_win11_theme(&self, frame: &Frame) {
        self.set_appearance(GridAppearance::win11(), Some(frame));
    }

    /// Run `f` once the frame message loop is processing messages.
    ///
    /// Use this instead of a [`crate::Timer`] when you need to call
    /// [`Self::set_checked`] or repaint the grid after `app.run` has
    /// started. Operations issued before the loop runs can deadlock
    /// inside synchronous `SendMessageW` notifications.
    pub fn call_after_message_loop<F>(&self, frame: &Frame, f: F)
    where
        F: FnOnce() + 'static,
    {
        #[cfg(target_os = "windows")]
        {
            static NEXT_DEFER_MSG: AtomicU32 = AtomicU32::new(0x400);
            let msg_id = WM_APP + 0x400 + NEXT_DEFER_MSG.fetch_add(1, Ordering::Relaxed);
            let once = Rc::new(RefCell::new(Some(f)));
            let frame_for_cleanup = frame.clone();
            frame.register_tray_message_handler(
                msg_id,
                Box::new(move |_| {
                    if let Some(cb) = once.borrow_mut().take() {
                        cb();
                    }
                    frame_for_cleanup.unregister_tray_message_handler(msg_id);
                }),
            );
            // SAFETY: `PostMessageW` to a live frame HWND; processed on
            // the first message-loop iteration after `app.run`.
            unsafe {
                PostMessageW(as_hwnd(frame.hwnd()), msg_id, 0, 0);
            }
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (self, frame);
            crate::platform::appkit::run_on_next_event(f);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = (self, frame, f);
        }
    }

    /// Repaint the grid without synchronous `LVM_REDRAWITEMS`.
    pub fn request_repaint(&self) {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        self.invalidate_view();
    }

    #[cfg(target_os = "windows")]
    fn apply_explorer_theme(&self) {
        let (hwnd, header) = {
            let i = self.inner.borrow();
            (i.hwnd, i.header_hwnd)
        };
        set_window_theme(hwnd, Some("Explorer"));
        if !header.is_null() {
            set_window_theme(header, Some("Explorer"));
        }
    }

    #[cfg(target_os = "windows")]
    fn clear_explorer_theme(&self) {
        let (hwnd, header) = {
            let i = self.inner.borrow();
            (i.hwnd, i.header_hwnd)
        };
        set_window_theme(hwnd, None);
        if !header.is_null() {
            set_window_theme(header, None);
        }
    }

    /// Current logical font description (face + point size).
    pub fn font_desc(&self) -> FontDesc {
        self.inner.borrow().font_desc.clone()
    }

    /// Replace the grid font from a [`FontDesc`] (cells + header).
    pub fn set_font_desc(&self, desc: FontDesc, redraw: bool) {
        let font = Font::new(desc);
        self.set_font(&font, redraw);
    }

    /// Row height in pixels. macOS sets `NSTableView` row height directly;
    /// Windows approximates via body font scaling.
    pub fn set_row_height(&self, height: f64) {
        let height = height.clamp(16.0, 96.0);
        self.inner.borrow_mut().row_height = height;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_row_height(self.inner.borrow().stub_handle, height);
        }
        #[cfg(target_os = "windows")]
        {
            win_apply_row_height(self, height);
        }
    }

    /// Configured row height in pixels.
    pub fn row_height(&self) -> f64 {
        self.inner.borrow().row_height
    }

    /// Change only the typeface, keeping size and style.
    pub fn set_font_face(&self, face: &str) {
        let desc = {
            let mut i = self.inner.borrow_mut();
            i.font_desc.face_name = face.to_string();
            i.font_desc.clone()
        };
        self.set_font_desc(desc, true);
    }

    /// Increase or decrease the point size (clamped to 6–48 pt).
    pub fn adjust_font_size(&self, delta: i32) {
        let desc = {
            let mut i = self.inner.borrow_mut();
            i.font_desc.point_size = (i.font_desc.point_size + delta).clamp(6, 48);
            i.font_desc.clone()
        };
        self.set_font_desc(desc, true);
    }

    /// Show the system font picker. Returns `true` if the user confirmed.
    pub fn pick_font(&self, frame: &Frame) -> bool {
        #[cfg(target_os = "windows")]
        {
            let initial = self.font_desc();
            let mut dlg = FontDialog::with_initial(frame, initial);
            if let Some(font) = dlg.show_modal() {
                self.set_font(&font, true);
                return true;
            }
            false
        }
        #[cfg(target_os = "macos")]
        {
            let initial = self.font_desc();
            let mut dlg = FontDialog::with_initial(frame, initial);
            if let Some(font) = dlg.show_modal() {
                self.set_font(&font, true);
                return true;
            }
            false
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = frame;
            false
        }
    }

    /// Schedule a repaint without synchronous redraw. Safe to call
    /// before the frame message loop is running.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn invalidate_view(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            if hwnd.is_null() {
                return;
            }
            // SAFETY: `InvalidateRect` on a live ListView HWND.
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 1);
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_redraw_view(self.inner.borrow().stub_handle);
        }
    }

    /// Install column context menu, header-click sort, row/column
    /// highlight, row context menu and truncated-cell tooltips in one call.
    pub fn enable_interactive_features(&self, frame: &Frame) {
        self.enable_column_context_menu(frame);
        self.enable_header_click_sort(frame);
        self.enable_row_column_highlight(frame);
        self.enable_row_context_menu(frame);
        self.enable_cell_editing(frame);
        self.enable_link_clicks(frame);
        self.enable_keyboard_navigation(frame);
        self.set_label_tips(true);
    }

    /// Arrow keys / Tab move the focused cell (requires focus on the grid).
    pub fn enable_keyboard_navigation(&self, frame: &Frame) {
        #[cfg(target_os = "macos")]
        self.ensure_macos_notify_hub(frame);
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = frame;
    }

    /// Install a right-click menu on the column headers (and on the
    /// grid body) with **Aggiungi colonna** / **Rimuovi colonna**.
    /// Also wires `NM_CUSTOMDRAW` so per-cell / per-row colours
    /// take effect. Safe to call once per grid instance.
    pub fn enable_column_context_menu(&self, frame: &Frame) {
        {
            let mut i = self.inner.borrow_mut();
            if i.context_menu_enabled {
                return;
            }
            i.context_menu_enabled = true;
            i.context_frame = Some(frame.clone());
        }
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            let header = self.inner.borrow().header_hwnd;
            if !header.is_null() {
                install_header_subclass(header, self.inner.clone());
            }
        }
        #[cfg(target_os = "macos")]
        {
            self.ensure_macos_notify_hub(frame);
        }
    }

    /// Alias of [`Self::enable_column_context_menu`] used by examples
    /// that prefer the more explicit `_with_frame` suffix.
    pub fn enable_column_context_menu_with_frame(&self, frame: &Frame) {
        self.enable_column_context_menu(frame)
    }

    /// Enable right-click on a data row to invoke
    /// [`Self::on_row_context_menu`]. The callback receives
    /// `(display_row, column, frame)`.
    pub fn enable_row_context_menu(&self, frame: &Frame) {
        self.inner.borrow_mut().row_context_menu_enabled = true;
        {
            let mut i = self.inner.borrow_mut();
            if i.context_frame.is_none() {
                i.context_frame = Some(frame.clone());
            }
        }
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(target_os = "macos")]
        {
            self.ensure_macos_notify_hub(frame);
        }
    }

    /// Register the handler invoked when the user right-clicks a row
    /// (requires [`Self::enable_row_context_menu`]).
    pub fn on_row_context_menu<F>(&self, f: F)
    where
        F: FnMut(&Frame, usize, usize) + 'static,
    {
        self.inner.borrow_mut().on_row_context_menu = Some(Box::new(f));
    }

    /// Register the handler invoked when the user right-clicks a column
    /// header (requires [`Self::enable_column_context_menu`]).
    /// When unset, the built-in add/remove column menu is shown.
    pub fn on_column_context_menu<F>(&self, f: F)
    where
        F: FnMut(&Frame, usize) + 'static,
    {
        self.inner.borrow_mut().on_column_context_menu = Some(Box::new(f));
    }

    /// Click a column header to sort (toggles ↑/↓ on repeat clicks).
    /// Updates `HDF_SORTUP` / `HDF_SORTDOWN` indicators on the header.
    pub fn enable_header_click_sort(&self, frame: &Frame) {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().header_click_sort = true;
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(target_os = "macos")]
        {
            self.inner.borrow_mut().header_click_sort = true;
            self.ensure_macos_notify_hub(frame);
        }
        #[cfg(target_os = "linux")]
        {
            self.inner.borrow_mut().header_click_sort = true;
            self.ensure_linux_notify_hub(frame);
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = frame;
    }

    /// Click the row reference cell (checkbox column, or column 0) or a
    /// column header to highlight the entire row or column.
    pub fn enable_row_column_highlight(&self, frame: &Frame) {
        self.inner.borrow_mut().row_col_highlight_enabled = true;
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_row_col_highlight_enabled(
                self.inner.borrow().stub_handle,
                true,
            );
            self.ensure_macos_notify_hub(frame);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = frame;
    }

    /// Currently highlighted display row, if any.
    pub fn highlighted_row(&self) -> Option<usize> {
        self.inner.borrow().highlighted_row
    }

    /// Currently highlighted data column, if any.
    pub fn highlighted_column(&self) -> Option<usize> {
        self.inner.borrow().highlighted_col
    }

    /// Clear row and column highlights.
    pub fn clear_row_column_highlight(&self) {
        self.set_row_column_highlight(None, None);
    }

    /// Highlight a display row and/or data column programmatically.
    pub fn set_row_column_highlight(&self, row: Option<usize>, col: Option<usize>) {
        {
            let mut i = self.inner.borrow_mut();
            i.highlighted_row = row;
            i.highlighted_col = col;
        }
        self.push_row_column_highlights_to_native();
    }

    fn push_row_column_highlights_to_native(&self) {
        let (row, col) = {
            let i = self.inner.borrow();
            (i.highlighted_row, i.highlighted_col)
        };
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_row_column_highlights(
                self.inner.borrow().stub_handle,
                row,
                col,
            );
        }
        #[cfg(target_os = "windows")]
        {
            let _ = (row, col);
            self.invalidate_view();
        }
    }

    /// Register a callback invoked when the sort column or direction
    /// changes (via [`Self::enable_header_click_sort`], [`Self::sort_by_column`],
    /// or [`Self::clear_sort`]).
    pub fn on_sort_changed<F>(&self, f: F)
    where
        F: FnMut(Option<usize>, Option<SortOrder>) + 'static,
    {
        self.inner.borrow_mut().on_sort_changed = Some(Box::new(f));
    }

    /// Return the active sort column and direction, if any.
    pub fn sort_state(&self) -> (Option<usize>, Option<SortOrder>) {
        let i = self.inner.borrow();
        (i.sort_col, i.sort_order)
    }

    fn notify_sort_changed(&self) {
        let (col, order) = self.sort_state();
        if let Some(ref mut cb) = self.inner.borrow_mut().on_sort_changed {
            cb(col, order);
        }
    }

    /// Show tooltips for truncated cell text (`LVS_EX_LABELTIP`).
    pub fn set_label_tips(&self, enabled: bool) {
        #[cfg(target_os = "windows")]
        set_listview_ex_style(self, LVS_EX_LABELTIP, enabled);
        #[cfg(target_os = "macos")]
        crate::platform::appkit::grid_set_label_tips(self.inner.borrow().stub_handle, enabled);
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = enabled;
    }

    /// Automatically size all columns to fit their content (`LVS_EX_AUTOSIZECOLUMNS`).
    pub fn set_autosize_columns(&self, enabled: bool) {
        #[cfg(target_os = "windows")]
        set_listview_ex_style(self, LVS_EX_AUTOSIZECOLUMNS, enabled);
        #[cfg(target_os = "macos")]
        crate::platform::appkit::grid_set_autosize_columns(
            self.inner.borrow().stub_handle,
            enabled,
        );
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = enabled;
    }

    /// Resize column `col` to fit header text and cell contents.
    pub fn autosize_column(&self, col: usize) {
        #[cfg(target_os = "windows")]
        {
            if col >= self.col_count() {
                return;
            }
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: valid ListView HWND and column index.
            unsafe {
                SendMessageW(
                    hwnd,
                    LVM_SETCOLUMNWIDTH,
                    col,
                    LVSCW_AUTOSIZE_USEHEADER as isize,
                );
            }
        }
        #[cfg(target_os = "macos")]
        {
            if col >= self.col_count() {
                return;
            }
            let width = crate::platform::appkit::grid_autosize_column(
                self.inner.borrow().stub_handle,
                col,
                true,
            );
            self.inner.borrow_mut().col_widths[col] = width;
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = col;
    }

    /// Resize column `col` to fit cell contents only (ignore header width).
    pub fn autosize_column_to_content(&self, col: usize) {
        #[cfg(target_os = "windows")]
        {
            if col >= self.col_count() {
                return;
            }
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: valid ListView HWND and column index.
            unsafe {
                SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, col, LVSCW_AUTOSIZE as isize);
            }
        }
        #[cfg(target_os = "macos")]
        {
            if col >= self.col_count() {
                return;
            }
            let width = crate::platform::appkit::grid_autosize_column(
                self.inner.borrow().stub_handle,
                col,
                false,
            );
            self.inner.borrow_mut().col_widths[col] = width;
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = col;
    }

    /// Return the visual column order (after header drag-and-drop).
    pub fn column_order(&self) -> Vec<usize> {
        #[cfg(target_os = "windows")]
        {
            let n = self.col_count();
            if n == 0 {
                return Vec::new();
            }
            let mut order = vec![0i32; n];
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: `order` has `n` elements; ListView writes `n` indices.
            let ok = unsafe {
                SendMessageW(
                    hwnd,
                    LVM_GETCOLUMNORDERARRAY,
                    n,
                    order.as_mut_ptr() as isize,
                )
            };
            if ok != 0 {
                order.into_iter().map(|i| i as usize).collect()
            } else {
                (0..n).collect()
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            #[cfg(not(target_os = "macos"))]
            let n = self.col_count();
            #[cfg(target_os = "macos")]
            {
                crate::platform::appkit::grid_column_order(self.inner.borrow().stub_handle)
            }
            #[cfg(not(target_os = "macos"))]
            {
                (0..n).collect()
            }
        }
    }

    /// Restore the visual column order. `order.len()` must equal
    /// [`Self::col_count`].
    pub fn set_column_order(&self, order: &[usize]) -> bool {
        if order.len() != self.col_count() {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            let ints: Vec<i32> = order.iter().map(|&i| i as i32).collect();
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: `ints` matches column count.
            let ok = unsafe {
                SendMessageW(
                    hwnd,
                    LVM_SETCOLUMNORDERARRAY,
                    order.len(),
                    ints.as_ptr() as isize,
                )
            };
            ok != 0
        }
        #[cfg(not(target_os = "windows"))]
        {
            #[cfg(target_os = "macos")]
            {
                return crate::platform::appkit::grid_set_column_order(
                    self.inner.borrow().stub_handle,
                    order,
                );
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = order;
                true
            }
        }
    }

    /// Provide custom tooltip text for `(display_row, col)`. Also
    /// enables [`Self::set_label_tips`] automatically.
    pub fn set_cell_tooltip_provider<F>(&self, frame: &Frame, f: F)
    where
        F: Fn(usize, usize) -> Option<String> + 'static,
    {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().cell_tooltip_provider = Some(Box::new(f));
            self.set_label_tips(true);
            self.ensure_infotip_hooks(frame);
        }
        #[cfg(target_os = "macos")]
        {
            self.set_label_tips(true);
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::grid_set_cell_tooltip_provider(handle, f);
            let _ = frame;
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (frame, f);
    }

    /// Register a callback for double-click / Enter on a row
    /// (`LVN_ITEMACTIVATE`). Arguments: `(display_row, column)`.
    pub fn on_row_activated<F>(&self, frame: &Frame, f: F)
    where
        F: FnMut(usize, usize) + 'static,
    {
        self.inner.borrow_mut().on_row_activated = Some(Box::new(f));
        #[cfg(target_os = "windows")]
        self.ensure_row_activate_hooks(frame);
        #[cfg(target_os = "macos")]
        self.ensure_macos_notify_hub(frame);
        #[cfg(target_os = "linux")]
        self.ensure_linux_notify_hub(frame);
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = frame;
    }

    #[cfg(target_os = "windows")]
    fn ensure_row_activate_hooks(&self, frame: &Frame) {
        let install = {
            let mut i = self.inner.borrow_mut();
            if i.row_activate_hooks_enabled {
                false
            } else {
                i.row_activate_hooks_enabled = true;
                true
            }
        };
        if !install {
            return;
        }
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_lv_item_activate_handler(
            id,
            Box::new(move |lparam| {
                handle_lv_item_activate(&inner, lparam);
            }),
        );
    }

    #[cfg(not(target_os = "windows"))]
    fn ensure_row_activate_hooks(&self, _frame: &Frame) {}

    /// Select one **display** row and scroll it into view.
    pub fn select_row(&self, display_row: usize) {
        if display_row >= self.row_count() {
            return;
        }
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            select_display_row(hwnd, display_row);
            // SAFETY: valid row index on a live ListView.
            unsafe {
                SendMessageW(hwnd, LVM_ENSUREVISIBLE, display_row, 0);
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = display_row;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_select_row(self.inner.borrow().stub_handle, display_row);
        }
        #[cfg(target_os = "linux")]
        {
            // Il widget GTK è la fonte di verità della selezione: la
            // notifica viene soppressa da `grid_select_row`, quindi i
            // callback in-memory vengono scattati qui sotto.
            crate::platform::gtk::grid_select_row(self.inner.borrow().stub_handle, display_row);
            // La notifica GTK è stata soppressa: scattiamo i callback
            // leggendo la selezione appena impostata sul widget.
            fire_selection_callbacks(&self.inner);
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            // Nessun widget nativo: la selezione vive solo nello stato
            // in-memory e i callback vengono scattati qui.
            let (cb_single, cb_multi) = {
                let mut i = self.inner.borrow_mut();
                i.last_selection = Some(display_row);
                i.last_selection_rows = vec![display_row];
                (i.on_sel_change.take(), i.on_sel_rows_change.take())
            };
            if let Some(mut c) = cb_single {
                c(Some(display_row));
                self.inner.borrow_mut().on_sel_change = Some(c);
            }
            if let Some(mut c) = cb_multi {
                let rows = vec![display_row];
                c(&rows);
                self.inner.borrow_mut().on_sel_rows_change = Some(c);
            }
        }
    }
    pub fn ensure_row_visible(&self, display_row: usize) {
        if display_row >= self.row_count() {
            return;
        }
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: valid row index on a live ListView.
            unsafe {
                SendMessageW(hwnd, LVM_ENSUREVISIBLE, display_row, 0);
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = display_row;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_ensure_row_visible(
                self.inner.borrow().stub_handle,
                display_row,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_ensure_row_visible(
                self.inner.borrow().stub_handle,
                display_row,
            );
        }
    }

    /// Logical column width in pixels (DPI-scaled value stored at insert time).
    pub fn column_width(&self, col: usize) -> Option<i32> {
        self.inner.borrow().col_widths.get(col).copied()
    }

    /// Set column width in logical pixels.
    pub fn set_column_width(&self, col: usize, width: i32) {
        if col >= self.col_count() {
            return;
        }
        self.inner.borrow_mut().col_widths[col] = width;
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: valid ListView HWND.
            let physical = unsafe { GetDpiForWindow(hwnd) }.scale(width as u32) as i32;
            // SAFETY: valid ListView HWND and column index.
            unsafe {
                SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, col, physical as isize);
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_column_width(
                self.inner.borrow().stub_handle,
                col,
                width,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_set_column_width(
                self.inner.borrow().stub_handle,
                col,
                width,
            );
        }
    }

    /// Indices of all checked rows (requires [`Self::set_checkboxes`]).
    pub fn checked_rows(&self) -> Vec<usize> {
        let n = self.row_count();
        (0..n).filter(|&r| self.is_checked(r)).collect()
    }

    /// Check or uncheck every row.
    pub fn set_all_checked(&self, checked: bool) {
        let n = self.row_count();
        for r in 0..n {
            self.set_checked(r, checked);
        }
    }

    /// All currently selected **display** row indices.
    pub fn selected_rows(&self) -> Vec<usize> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: standard ListView selection enumeration.
            unsafe {
                let count = SendMessageW(hwnd, LVM_GETSELECTEDCOUNT, 0, 0);
                if count <= 0 {
                    return Vec::new();
                }
                let mut rows = Vec::with_capacity(count as usize);
                let mut idx = -1i32;
                while rows.len() < count as usize {
                    let next = SendMessageW(
                        hwnd,
                        LVM_GETNEXTITEM,
                        idx as isize as usize,
                        LVNI_SELECTED as isize,
                    );
                    if next < 0 {
                        break;
                    }
                    rows.push(next as usize);
                    idx = next as i32;
                }
                rows
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            #[cfg(target_os = "macos")]
            {
                return crate::platform::appkit::grid_get_selected_rows(
                    self.inner.borrow().stub_handle,
                );
            }
            #[cfg(target_os = "linux")]
            {
                return crate::platform::gtk::grid_get_selected_rows(
                    self.inner.borrow().stub_handle,
                );
            }
            #[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
            {
                self.get_selected_row().into_iter().collect()
            }
        }
    }

    /// Tab-separated cell values for one **display** row.
    pub fn row_as_tsv(&self, display_row: usize) -> String {
        if display_row >= self.row_count() {
            return String::new();
        }
        (0..self.col_count())
            .map(|c| self.cell_display_text(display_row, c))
            .collect::<Vec<_>>()
            .join("\t")
    }

    /// Copy one row (TSV) to the system clipboard.
    pub fn copy_row_to_clipboard(&self, display_row: usize) -> bool {
        let text = self.row_as_tsv(display_row);
        if text.is_empty() {
            return false;
        }
        crate::Clipboard::set_text(&text)
    }

    /// Copy all checked rows (header + values) as TSV to the clipboard.
    pub fn copy_checked_rows_to_clipboard(&self) -> bool {
        let checked = self.checked_rows();
        if checked.is_empty() {
            return false;
        }
        let header = (0..self.col_count())
            .filter_map(|c| self.column_title(c))
            .collect::<Vec<_>>()
            .join("\t");
        let body: Vec<String> = checked.iter().map(|&r| self.row_as_tsv(r)).collect();
        let text = format!("{header}\n{}", body.join("\n"));
        crate::Clipboard::set_text(&text)
    }

    /// Tab-separated export of the whole grid (optional header row).
    pub fn as_tsv(&self, include_header: bool) -> String {
        let n_cols = self.col_count();
        let n_rows = self.row_count();
        let mut out = String::new();
        if include_header && n_cols > 0 {
            let header = (0..n_cols)
                .filter_map(|c| self.column_title(c))
                .collect::<Vec<_>>()
                .join("\t");
            out.push_str(&header);
            if n_rows > 0 {
                out.push('\n');
            }
        }
        for r in 0..n_rows {
            if r > 0 {
                out.push('\n');
            }
            out.push_str(&self.row_as_tsv(r));
        }
        out
    }

    /// Copy the entire grid (header + all rows) as TSV.
    pub fn copy_all_to_clipboard(&self) -> bool {
        let text = self.as_tsv(true);
        if text.is_empty() {
            return false;
        }
        crate::Clipboard::set_text(&text)
    }

    /// Write the grid as TSV to a file (includes header row).
    pub fn write_tsv_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.as_tsv(true))
    }

    /// Returns whether an inline editor is active.
    pub fn is_editing(&self) -> bool {
        self.inner
            .borrow()
            .cell_editor
            .as_ref()
            .is_some_and(|e| e.active)
    }

    /// Visible display rows after filtering (equals [`Self::row_count`] when unfiltered).
    pub fn display_row_count(&self) -> usize {
        let i = self.inner.borrow();
        if i.row_perm.is_empty() {
            i.row_count
        } else {
            i.row_perm.len()
        }
    }

    /// Attach a wx-style [`GridTable`] model (installs provider + optional edit-back).
    pub fn set_grid_table(&self, table: Rc<RefCell<dyn GridTable>>) {
        let (rows, cols) = {
            let t = table.borrow();
            (t.row_count(), t.col_count())
        };
        self.inner.borrow_mut().grid_table = Some(table.clone());
        while self.col_count() < cols {
            self.append_column("", 80);
        }
        let table_for_provider = table.clone();
        self.set_value_provider(move |row, col| table_for_provider.borrow().value(row, col));
        if self.inner.borrow().on_cell_edited.is_none() {
            let table2 = table.clone();
            self.on_cell_edited(move |row, col, text| {
                table2
                    .borrow_mut()
                    .set_value(row, col, Cell::Text(text.to_string()))
            });
        }
        self.set_row_count(rows);
    }

    /// Live column renderer for one data column.
    pub fn set_column_renderer(&self, col: usize, renderer: Rc<dyn GridColumnRenderer>) {
        if col < self.col_count() {
            self.inner.borrow_mut().col_renderers.insert(col, renderer);
            self.refresh();
        }
    }

    /// Global case-insensitive text filter (all columns).
    pub fn set_text_filter(&self, query: &str) {
        self.inner.borrow_mut().filter_query = query.to_lowercase();
        self.rebuild_display_perm();
    }

    /// Per-column case-insensitive filter.
    pub fn set_column_filter(&self, col: usize, query: &str) {
        self.inner
            .borrow_mut()
            .column_filters
            .insert(col, query.to_lowercase());
        self.rebuild_display_perm();
    }

    pub fn clear_filters(&self) {
        {
            let mut i = self.inner.borrow_mut();
            i.filter_query.clear();
            i.column_filters.clear();
        }
        self.rebuild_display_perm();
    }

    pub fn text_filter(&self) -> String {
        self.inner.borrow().filter_query.clone()
    }

    /// Merge logical cell range (inclusive corners).
    pub fn merge_cells(&self, range: GridRange) -> bool {
        let ok = register_merge(&mut self.inner.borrow_mut().merge_spans, range);
        if ok {
            self.refresh();
        }
        ok
    }

    pub fn merge_cells_coords(&self, top: usize, left: usize, bottom: usize, right: usize) -> bool {
        self.merge_cells(GridRange::new(
            GridCoords::new(top, left),
            GridCoords::new(bottom, right),
        ))
    }

    pub fn unmerge_cell(&self, logical_row: usize, col: usize) {
        self.inner
            .borrow_mut()
            .merge_spans
            .remove(&(logical_row, col));
        self.refresh();
    }

    pub fn clear_merges(&self) {
        self.inner.borrow_mut().merge_spans.clear();
        self.refresh();
    }

    pub fn enable_frozen_columns(&self, count: usize) {
        let count = count.min(self.col_count());
        self.inner.borrow_mut().frozen_col_count = count;
        #[cfg(target_os = "macos")]
        crate::platform::appkit::grid_set_frozen_columns(self.inner.borrow().stub_handle, count);
        #[cfg(target_os = "windows")]
        win_rebuild_frozen_columns(self);
        self.refresh();
    }

    pub fn enable_frozen_rows(&self, count: usize) {
        self.inner.borrow_mut().frozen_row_count = count;
        #[cfg(target_os = "macos")]
        crate::platform::appkit::grid_set_frozen_rows(self.inner.borrow().stub_handle, count);
    }

    pub fn frozen_column_count(&self) -> usize {
        self.inner.borrow().frozen_col_count
    }

    pub fn frozen_row_count(&self) -> usize {
        self.inner.borrow().frozen_row_count
    }

    pub fn enable_row_drag_reorder(&self, frame: &Frame) {
        self.inner.borrow_mut().row_drag_enabled = true;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_row_drag_enabled(
                self.inner.borrow().stub_handle,
                true,
            );
            self.ensure_macos_notify_hub(frame);
        }
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = frame;
    }

    pub fn on_row_reordered<F>(&self, f: F)
    where
        F: FnMut(usize, usize) + 'static,
    {
        self.inner.borrow_mut().on_row_reordered = Some(Box::new(f));
    }

    pub fn move_display_row(&self, from: usize, to: usize) -> bool {
        let mut perm = self.inner.borrow().row_perm.clone();
        if perm.is_empty() {
            perm = (0..self.row_count()).collect();
        }
        if from >= perm.len() || to >= perm.len() || from == to {
            return false;
        }
        let item = perm.remove(from);
        perm.insert(to, item);
        self.inner.borrow_mut().row_perm = perm;
        self.sync_native_display_rows();
        self.refresh();
        let mut cb = self.inner.borrow_mut().on_row_reordered.take();
        if let Some(ref mut handler) = cb {
            handler(from, to);
        }
        self.inner.borrow_mut().on_row_reordered = cb;
        true
    }

    pub fn export_pdf(&self, path: &std::path::Path) -> std::io::Result<()> {
        let title = self
            .inner
            .borrow()
            .col_titles
            .first()
            .cloned()
            .unwrap_or_else(|| "Grid".to_string());
        std::fs::write(path, tsv_to_pdf_bytes(&title, &self.as_tsv(true)))
    }

    pub fn print(&self) -> bool {
        use crate::printing::{Printer, Printout};
        let pages = (self.display_row_count() / 40).max(1) as u32;
        Printer::new().print(&Printout::new("Grid", pages))
    }

    fn rebuild_display_perm(&self) {
        let (n, filter_query, column_filters, sort_col, sort_order, sort_kinds) = {
            let i = self.inner.borrow();
            (
                i.row_count,
                i.filter_query.clone(),
                i.column_filters.clone(),
                i.sort_col,
                i.sort_order,
                i.col_sort_kinds.clone(),
            )
        };
        let col_count = self.col_count();
        let mut rows: Vec<usize> = (0..n)
            .filter(|&r| {
                row_text_matches_filter(
                    col_count,
                    &|row, col| self.cell_text_logical(row, col),
                    r,
                    &filter_query,
                    &column_filters,
                )
            })
            .collect();
        if rows.is_empty() && n > 0 && filter_query.is_empty() && column_filters.is_empty() {
            rows = (0..n).collect();
        }
        if let (Some(col), Some(order)) = (sort_col, sort_order) {
            if col < col_count {
                let kind = sort_kinds.get(col).copied().unwrap_or(ColumnSortKind::Auto);
                rows.sort_by(|&a, &b| {
                    let cmp = compare_column_cells(
                        &self.cell_at_logical(a, col),
                        &self.cell_at_logical(b, col),
                        kind,
                    );
                    match order {
                        SortOrder::None => cmp,
                        SortOrder::Ascending => cmp,
                        SortOrder::Descending => cmp.reverse(),
                    }
                });
            }
        }
        self.inner.borrow_mut().row_perm = rows;
        self.sync_native_display_rows();
        self.refresh();
    }

    fn sync_native_display_rows(&self) {
        let count = self.display_row_count();
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            let current = crate::platform::appkit::grid_native_row_count(handle);
            if current != count {
                crate::platform::appkit::grid_set_row_count(handle, count);
            }
        }
        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            let current = crate::platform::gtk::grid_native_row_count(handle);
            if current != count {
                crate::platform::gtk::grid_set_row_count(handle, count);
            }
        }
        #[cfg(target_os = "windows")]
        win_sync_listview_row_count(self, count);
    }

    /// Paste TSV from the clipboard starting at `(display_row, col)`.
    /// Returns the number of rows written.
    pub fn paste_from_clipboard(&self, display_row: usize, col: usize) -> usize {
        let Some(text) = crate::Clipboard::get_text() else {
            return 0;
        };
        grid_paste_tsv(self, display_row, col, &text)
    }

    /// Paste TSV from the clipboard at the focused cell, or row 0 / col 0.
    pub fn paste_at_focus(&self) -> usize {
        let (row, col) = self.focused_cell().unwrap_or((0, 0));
        self.paste_from_clipboard(row, col)
    }

    /// Currently focused cell `(display_row, data_column)`, if any.
    pub fn focused_cell(&self) -> Option<(usize, usize)> {
        self.inner.borrow().focused_cell
    }

    /// Move keyboard / edit focus to one cell.
    pub fn set_focused_cell(&self, display_row: usize, col: usize) {
        if display_row >= self.display_row_count() || col >= self.col_count() {
            return;
        }
        {
            let mut i = self.inner.borrow_mut();
            i.focused_cell = Some((display_row, col));
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_focused_cell(
                self.inner.borrow().stub_handle,
                display_row,
                col,
            );
        }
        self.fire_focused_cell_changed(display_row, col);
    }

    /// Callback when the focused cell changes.
    pub fn on_focused_cell_changed<F>(&self, f: F)
    where
        F: FnMut(usize, usize) + 'static,
    {
        self.inner.borrow_mut().on_focused_cell_changed = Some(Box::new(f));
    }

    fn fire_focused_cell_changed(&self, row: usize, col: usize) {
        let cb = self.inner.borrow_mut().on_focused_cell_changed.take();
        if let Some(mut c) = cb {
            c(row, col);
            self.inner.borrow_mut().on_focused_cell_changed = Some(c);
        }
    }

    /// Multi-row selection callback (fires alongside [`Self::on_selection_changed`]).
    pub fn on_selection_rows_changed<F>(&self, frame: &Frame, f: F)
    where
        F: FnMut(&[usize]) + 'static,
    {
        self.inner.borrow_mut().on_sel_rows_change = Some(Box::new(f));
        self.on_selection_changed(frame, |_| {});
    }

    /// Hide a data column (width zero / NSTableColumn hidden).
    pub fn hide_column(&self, col: usize) -> bool {
        if col >= self.col_count() {
            return false;
        }
        self.inner.borrow_mut().col_hidden[col] = true;
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let lv_col = lv_col_for_data_col(&self.inner.borrow(), col);
            // SAFETY: valid ListView column index.
            unsafe {
                SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, lv_col, 0);
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_column_hidden(
                self.inner.borrow().stub_handle,
                col,
                true,
            );
        }
        true
    }

    /// Show a previously hidden column.
    pub fn show_column(&self, col: usize) -> bool {
        if col >= self.col_count() {
            return false;
        }
        let width = self
            .inner
            .borrow()
            .col_widths
            .get(col)
            .copied()
            .unwrap_or(80);
        self.inner.borrow_mut().col_hidden[col] = false;
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let lv_col = lv_col_for_data_col(&self.inner.borrow(), col);
            // SAFETY: valid ListView HWND.
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            let physical_w = dpi.scale(width as u32) as i32;
            // SAFETY: valid ListView column index.
            unsafe {
                SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, lv_col, physical_w as isize);
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_column_hidden(
                self.inner.borrow().stub_handle,
                col,
                false,
            );
            self.set_column_width(col, width);
        }
        true
    }

    /// Whether data column `col` is hidden.
    pub fn is_column_hidden(&self, col: usize) -> bool {
        self.inner
            .borrow()
            .col_hidden
            .get(col)
            .copied()
            .unwrap_or(false)
    }

    /// Insert a leading row-number column (`#`, 1-based).
    pub fn enable_row_labels(&self, enabled: bool) {
        self.enable_row_labels_with_width(enabled, 40)
    }

    /// Insert a leading row-number column with a custom width.
    pub fn enable_row_labels_with_width(&self, enabled: bool, width: i32) {
        {
            let mut i = self.inner.borrow_mut();
            if i.row_labels_enabled == enabled {
                return;
            }
            i.row_labels_enabled = enabled;
            i.row_label_width = width.max(20);
        }
        #[cfg(target_os = "windows")]
        win_sync_row_label_column(self);
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_enable_row_labels(
                self.inner.borrow().stub_handle,
                enabled,
                width,
            );
        }
        self.refresh();
    }

    /// Enable inline editing (F2, double-click on text cells, Cmd/Ctrl+V paste).
    pub fn enable_cell_editing(&self, frame: &Frame) {
        self.inner.borrow_mut().cell_editing_enabled = true;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_cell_editing_enabled(
                self.inner.borrow().stub_handle,
                true,
            );
            self.ensure_macos_notify_hub(frame);
        }
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = frame;
    }

    /// Single-click on [`Cell::Link`] cells opens `url` in the default browser.
    pub fn enable_link_clicks(&self, frame: &Frame) {
        self.inner.borrow_mut().link_clicks_enabled = true;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_link_clicks_enabled(
                self.inner.borrow().stub_handle,
                true,
            );
            self.ensure_macos_notify_hub(frame);
        }
        #[cfg(target_os = "windows")]
        {
            self.ensure_visual_hooks(frame);
            install_list_subclass_if_needed(self);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = frame;
    }

    /// If `(display_row, col)` holds a [`Cell::Link`], open its URL. Returns whether a URL was opened.
    pub fn open_link_at(&self, display_row: usize, col: usize) -> bool {
        if !self.inner.borrow().link_clicks_enabled {
            return false;
        }
        if col >= self.col_count() || display_row >= self.row_count() {
            return false;
        }
        let logical = self.logical_row(display_row);
        let cell = self.raw_cell_at_logical(logical, col);
        if let Cell::Link { url, .. } = cell {
            if !url.is_empty() {
                return crate::core::open_url::open_url_in_browser(&url);
            }
        }
        false
    }

    /// Callback when the user commits an edited cell. Return `false` to reject.
    /// Arguments use **logical** row index and **data** column.
    pub fn on_cell_edited<F>(&self, f: F)
    where
        F: FnMut(usize, usize, &str) -> bool + 'static,
    {
        self.inner.borrow_mut().on_cell_edited = Some(Box::new(f));
    }

    /// Begin editing the focused cell (F2). Returns `false` if editing is disabled.
    pub fn begin_cell_edit(&self) -> bool {
        if !self.inner.borrow().cell_editing_enabled {
            return false;
        }
        let Some((display_row, col)) = self.focused_cell() else {
            return false;
        };
        if self.is_column_hidden(col) {
            return false;
        }
        #[cfg(target_os = "macos")]
        {
            let mut editor = crate::GridCellEditor::new(display_row, col);
            editor.begin_edit(&self.cell_display_text(display_row, col));
            self.inner.borrow_mut().cell_editor = Some(editor);
            crate::platform::appkit::grid_begin_cell_edit(self.inner.borrow().stub_handle)
        }
        #[cfg(target_os = "windows")]
        {
            let mut editor = crate::GridCellEditor::new(display_row, col);
            editor.begin_edit(&self.cell_display_text(display_row, col));
            self.inner.borrow_mut().cell_editor = Some(editor);
            win_begin_cell_edit(self, display_row, col)
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = (display_row, col);
            false
        }
    }

    /// Commit or cancel the active inline editor.
    pub fn end_cell_edit(&self, accept: bool) -> bool {
        if let Some(mut editor) = self.inner.borrow_mut().cell_editor.take() {
            let _ = editor.end_edit(accept);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_end_cell_edit(self.inner.borrow().stub_handle, accept)
        }
        #[cfg(target_os = "windows")]
        {
            win_end_cell_edit(self, accept)
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = accept;
            false
        }
    }

    /// Install column context menu, header-click sort, row/column
    #[cfg(target_os = "windows")]
    fn ensure_visual_hooks(&self, frame: &Frame) {
        let install = {
            let mut i = self.inner.borrow_mut();
            i.visual_frame = Some(frame.clone());
            if i.visual_hooks_enabled {
                false
            } else {
                i.visual_hooks_enabled = true;
                if i.header_hwnd.is_null() && !i.hwnd.is_null() {
                    i.header_hwnd =
                        unsafe { SendMessageW(i.hwnd, LVM_GETHEADER, 0, 0) as isize as HWND };
                }
                true
            }
        };
        if !install {
            return;
        }
        install_list_subclass(self.inner.borrow().hwnd, self.inner.clone());
        if self.inner.borrow().appearance.system_theme {
            self.apply_explorer_theme();
        }
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_lv_custom_draw_handler(
            id,
            Box::new(move |lparam| handle_grid_custom_draw(&inner, lparam)),
        );
    }

    #[cfg(target_os = "macos")]
    fn ensure_visual_hooks(&self, frame: &Frame) {
        crate::platform::appkit::grid_install_macos_visual_hooks(self.inner.borrow().stub_handle);
        self.ensure_macos_notify_hub(frame);
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn ensure_visual_hooks(&self, _frame: &Frame) {}

    #[cfg(target_os = "windows")]
    fn ensure_infotip_hooks(&self, frame: &Frame) {
        let install = {
            let mut i = self.inner.borrow_mut();
            if i.infotip_hooks_enabled {
                false
            } else {
                i.infotip_hooks_enabled = true;
                true
            }
        };
        if !install {
            return;
        }
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_lv_infotip_handler(
            id,
            Box::new(move |lparam| {
                handle_lv_get_infotip(&inner, lparam);
            }),
        );
    }

    #[cfg(not(target_os = "windows"))]
    fn ensure_infotip_hooks(&self, _frame: &Frame) {}

    #[cfg(target_os = "windows")]
    fn repaint_header(&self) {
        let header = self.inner.borrow().header_hwnd;
        if !header.is_null() {
            // SAFETY: `InvalidateRect` on the report-view header child.
            unsafe {
                InvalidateRect(header, std::ptr::null(), 1);
            }
        }
    }

    /// Force ListView + header to repaint after a theme switch.
    #[cfg(target_os = "windows")]
    fn repaint_after_theme_change(&self) {
        let (hwnd, header) = {
            let i = self.inner.borrow();
            (i.hwnd, i.header_hwnd)
        };
        if hwnd.is_null() {
            return;
        }
        // SAFETY: Win32 repaint calls on live ListView / header HWNDs.
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), 1);
            if !header.is_null() {
                InvalidateRect(header, std::ptr::null(), 1);
            }
            SendMessageW(hwnd, LVM_REDRAWITEMS, 0, i32::MAX as isize);
        }
    }

    /// Show the column context menu at the current cursor position.
    /// Normally called automatically by [`Self::enable_column_context_menu`].
    pub fn popup_column_context_menu(&self, frame: &Frame) {
        let col = self.inner.borrow().context_menu_col;
        let can_remove = self.inner.borrow().col_count > 1;
        let mut menu = PopupMenu::new();
        let grid_add = self.clone();
        menu.append("Aggiungi colonna", frame, move || {
            grid_add.append_dynamic_column(80);
        });
        if can_remove {
            let grid_remove = self.clone();
            let label = format!("Rimuovi colonna \"{}\"", {
                grid_remove
                    .column_title(col)
                    .unwrap_or_else(|| format!("#{col}"))
            });
            menu.append(&label, frame, move || {
                grid_remove.remove_column(col);
            });
        } else {
            menu.append_disabled("Rimuovi colonna (ultima)");
        }
        menu.popup(frame);
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
            let current = SendMessageW(hwnd, LVM_GETEXTENDEDLISTVIEWSTYLE, 0, 0);
            let new = if enabled {
                (current as u32) | LVS_EX_CHECKBOXES
            } else {
                (current as u32) & !LVS_EX_CHECKBOXES
            };
            SendMessageW(hwnd, LVM_SETEXTENDEDLISTVIEWSTYLE, 0, new as isize);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_checkboxes(self.inner.borrow().stub_handle, enabled);
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
            let _r = SendMessageW(hwnd, LVM_GETITEM, 0, &mut item as *mut LVITEMW as isize);
            // The state-image index is stored in bits 12..15. Index
            // 2 = checked, anything else (incl. 1 = unchecked) = not
            // checked.
            ((item.state >> 12) & 0x0F) == 2
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_is_row_checked(self.inner.borrow().stub_handle, row)
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_row_checked(
                self.inner.borrow().stub_handle,
                row,
                checked,
            );
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_row_count(self.inner.borrow().stub_handle, n);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_set_row_count(self.inner.borrow().stub_handle, n);
        }
        {
            let mut i = self.inner.borrow_mut();
            i.row_count = n;
            i.row_perm = (0..n).collect();
            i.cells.clear();
            i.row_styles.clear();
            i.cell_styles.clear();
            i.last_selection = None;
        }
        self.rebuild_display_perm();
    }

    /// Set the value of one static cell (logical row index). Ignored
    /// if a value provider has been set (the provider always wins).
    pub fn set_cell(&self, logical_row: usize, col: usize, cell: Cell) {
        self.inner
            .borrow_mut()
            .cells
            .insert((logical_row, col), cell);
        self.refresh_logical_cell(logical_row, col);
    }

    /// Install a closure that is called for every `(logical_row, col)`
    /// to produce the cell value. Once installed, the provider takes
    /// priority over any cells set via [`Self::set_cell`].
    ///
    /// The provider is re-queried automatically by
    /// [`Self::set_row_count`], [`Self::refresh`], and
    /// [`Self::sort_by_column`] (display order changes, logical
    /// indices stay stable).
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
            (Self::display_row_count_from(&i), i.col_count)
        };
        for r in 0..n_rows {
            for c in 0..n_cols {
                self.apply_cell(r, c);
            }
        }
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::grid_redraw_view(handle);
            crate::platform::appkit::grid_maybe_autosize_after_refresh(handle);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_redraw_view(self.inner.borrow().stub_handle);
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
            let dpi = GetDpiForWindow(hwnd);
            for (idx, &w) in i.col_widths.iter().enumerate() {
                let hidden = i.col_hidden.get(idx).copied().unwrap_or(false);
                let lv_col = lv_col_for_data_col(&i, idx);
                let physical_w: i32 = if hidden {
                    0
                } else {
                    dpi.scale(w as u32) as i32
                };
                SendMessageW(hwnd, LVM_SETCOLUMNWIDTH, lv_col, physical_w as isize);
            }
        }
        #[cfg(target_os = "macos")]
        {
            let i = self.inner.borrow();
            let widths: Vec<(usize, i32)> = i
                .col_widths
                .iter()
                .enumerate()
                .map(|(idx, w)| (idx, *w))
                .collect();
            crate::platform::appkit::grid_apply_column_widths(i.stub_handle, &widths);
        }
        #[cfg(target_os = "linux")]
        {
            let i = self.inner.borrow();
            let widths: Vec<(usize, i32)> = i
                .col_widths
                .iter()
                .enumerate()
                .map(|(idx, w)| (idx, *w))
                .collect();
            crate::platform::gtk::grid_apply_column_widths(i.stub_handle, &widths);
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
            // 0..INT_MAX so the entire range is repainted. (Not
            // `i32::MIN as usize`: zero-extension on Win64 turns it
            // into +2147483648, which is past the end of the list.)
            SendMessageW(hwnd, LVM_REDRAWITEMS, 0, i32::MAX as isize);
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_image_list(
                self.inner.borrow().stub_handle,
                list.handle(),
            );
        }
    }

    /// Attach a [`crate::GridIcons`] set and optionally widen the
    /// icon column (display column `0` by default).
    pub fn attach_icons(&self, icons: &crate::GridIcons) {
        self.set_image_list(icons.image_list());
    }

    /// Attach icons and set the width of the icon column.
    pub fn attach_icons_with_column_width(
        &self,
        icons: &crate::GridIcons,
        icon_column: usize,
        width: i32,
    ) {
        self.attach_icons(icons);
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI with a live ListView HWND.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let dpi = GetDpiForWindow(hwnd);
            SendMessageW(
                hwnd,
                LVM_SETCOLUMNWIDTH,
                icon_column,
                dpi.scale(width as u32) as isize,
            );
        }
        if icon_column < self.inner.borrow().col_widths.len() {
            self.inner.borrow_mut().col_widths[icon_column] = width;
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_column_width(
                self.inner.borrow().stub_handle,
                icon_column,
                width,
            );
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
        {
            let mut i = self.inner.borrow_mut();
            i.font_desc = font.desc().clone();
            #[cfg(target_os = "windows")]
            {
                i.font = Some(font.clone());
            }
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI calls with validated arguments. `LVM_GETHEADER`
        // returns a real child `HWND` of the listview in report view, and
        // `WM_SETFONT` accepts any `HFONT` (including one selected by another
        // control — the kernel merely records it on the window).
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let hfont = font.hfont() as usize;
            SendMessageW(hwnd, WM_SETFONT, hfont, redraw as isize);
            // Forward the same HFONT to the header child so the column
            // titles render at the same metrics as the cell text.
            let header_hwnd = SendMessageW(hwnd, LVM_GETHEADER, 0, 0) as isize as HWND;
            if !header_hwnd.is_null() {
                SendMessageW(header_hwnd, WM_SETFONT, hfont, redraw as isize);
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (font, redraw);
        #[cfg(target_os = "macos")]
        {
            let desc = self.inner.borrow().font_desc.clone();
            crate::platform::appkit::grid_set_font(
                self.inner.borrow().stub_handle,
                &desc.face_name,
                desc.point_size as f64,
            );
        }
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_get_selected_row(self.inner.borrow().stub_handle)
        }
        #[cfg(target_os = "linux")]
        {
            // Il widget è la fonte di verità; `last_selection` resta
            // il fallback per gli handle senza widget (stub).
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::grid_get_selected_row(handle)
                .or_else(|| self.inner.borrow().last_selection)
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            // Nessun widget nativo: si legge la selezione mantenuta
            // in-memory (aggiornata da `select_row`).
            self.inner.borrow().last_selection
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

        #[cfg(target_os = "windows")]
        {
            let inner = self.inner.clone();
            let id = self.inner.borrow().id;
            frame.register_notify_handler(
                id,
                Box::new(move |code| {
                    if code != LVN_ITEMCHANGED {
                        return;
                    }
                    fire_selection_callbacks(&inner);
                }),
            );
        }
        #[cfg(target_os = "macos")]
        self.ensure_macos_notify_hub(frame);
        #[cfg(target_os = "linux")]
        self.ensure_linux_notify_hub(frame);
    }

    /// Map a **display** row (ListView index) to the **logical** data
    /// row that supplies its values. Equal to the display index until
    /// [`Self::sort_by_column`] reorders rows.
    pub fn logical_row(&self, display_row: usize) -> usize {
        let i = self.inner.borrow();
        i.row_perm.get(display_row).copied().unwrap_or(display_row)
    }

    /// Text shown in one cell, using **display** row indices.
    pub fn cell_display_text(&self, display_row: usize, col: usize) -> String {
        let logical = self.logical_row(display_row);
        self.cell_text_logical(logical, col)
    }

    /// Text for a **logical** data row (ignores sort permutation).
    pub fn cell_text_logical(&self, logical_row: usize, col: usize) -> String {
        self.cell_at_logical(logical_row, col).text()
    }

    /// Sort comparison mode for column `col` (defaults to [`ColumnSortKind::Auto`]).
    pub fn column_sort_kind(&self, col: usize) -> Option<ColumnSortKind> {
        self.inner.borrow().col_sort_kinds.get(col).copied()
    }

    /// Override how column `col` is sorted (see [`ColumnSortKind`]).
    pub fn set_column_sort_kind(&self, col: usize, kind: ColumnSortKind) {
        if col >= self.col_count() {
            return;
        }
        self.inner.borrow_mut().col_sort_kinds[col] = kind;
    }

    /// Sort rows by the value in column `col`, using each cell's type
    /// (or the column's [`ColumnSortKind`]) rather than raw display text.
    pub fn sort_by_column(&self, col: usize, order: SortOrder) {
        self.apply_sort_column(col, order, true);
    }

    fn apply_sort_column(&self, col: usize, order: SortOrder, update_native_indicator: bool) {
        if col >= self.col_count() {
            return;
        }
        {
            let mut i = self.inner.borrow_mut();
            i.sort_col = Some(col);
            i.sort_order = Some(order);
        }
        self.rebuild_display_perm();
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::grid_update_sort_state(
                handle,
                Some(col),
                Some(matches!(order, SortOrder::Ascending)),
            );
        }
        if update_native_indicator {
            #[cfg(target_os = "macos")]
            {
                crate::platform::appkit::grid_set_sort_indicator(
                    self.inner.borrow().stub_handle,
                    Some(col),
                    Some(matches!(order, SortOrder::Ascending)),
                );
            }
            #[cfg(target_os = "windows")]
            {
                self.invalidate_view();
                update_header_sort_indicators(&self.inner);
            }
        }
        self.notify_sort_changed();
    }

    /// Restore the original row order (`0, 1, 2, …`).
    pub fn clear_sort(&self) {
        {
            let mut i = self.inner.borrow_mut();
            i.sort_col = None;
            i.sort_order = None;
        }
        self.rebuild_display_perm();
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_set_sort_indicator(
                self.inner.borrow().stub_handle,
                None,
                None,
            );
        }
        #[cfg(target_os = "windows")]
        {
            self.invalidate_view();
            update_header_sort_indicators(&self.inner);
        }
        self.notify_sort_changed();
    }

    fn display_row_count_from(i: &GridInner) -> usize {
        if i.row_perm.is_empty() {
            i.row_count
        } else {
            i.row_perm.len()
        }
    }

    /// Append one empty row at the bottom and populate it from the
    /// value provider (if any).
    pub fn append_row(&self) {
        let n = self.row_count();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI with a live ListView HWND.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let item = LVITEMW {
                mask: 0,
                i_item: n as i32,
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
        {
            let mut i = self.inner.borrow_mut();
            i.row_count = n + 1;
            i.row_perm.push(n);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_append_row(self.inner.borrow().stub_handle);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_append_row(self.inner.borrow().stub_handle);
        }
        for c in 0..self.col_count() {
            self.apply_cell(n, c);
        }
    }

    /// Remove the row at **display** index `display_row`.
    pub fn delete_row(&self, display_row: usize) -> bool {
        if display_row >= self.row_count() {
            return false;
        }
        let logical = self.logical_row(display_row);
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI with a live ListView HWND.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            SendMessageW(hwnd, LVM_DELETEITEM, display_row, 0);
        }
        {
            let mut i = self.inner.borrow_mut();
            i.row_perm.remove(display_row);
            for entry in i.row_perm.iter_mut() {
                if *entry > logical {
                    *entry -= 1;
                }
            }
            compact_logical_cells(&mut i, logical);
            shift_display_styles(&mut i, display_row);
            i.row_count = i.row_count.saturating_sub(1);
            if i.last_selection == Some(display_row) {
                i.last_selection = None;
            } else if let Some(sel) = i.last_selection {
                if sel > display_row {
                    i.last_selection = Some(sel - 1);
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_delete_row(self.inner.borrow().stub_handle, display_row);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_delete_row(self.inner.borrow().stub_handle, display_row);
        }
        true
    }

    /// Remove the currently selected row, if any.
    pub fn delete_selected_row(&self) -> bool {
        match self.get_selected_row() {
            Some(r) => self.delete_row(r),
            None => false,
        }
    }

    /// Apply `style` to the currently selected **display** row.
    pub fn highlight_selected_row(&self, style: GridCellStyle) {
        if let Some(r) = self.get_selected_row() {
            self.set_row_style(r, style);
        }
    }

    /// Clear per-row colours for one **display** row.
    pub fn clear_row_style(&self, display_row: usize) {
        self.inner.borrow_mut().row_styles.remove(&display_row);
        #[cfg(target_os = "windows")]
        self.invalidate_view();
        #[cfg(target_os = "macos")]
        crate::platform::appkit::grid_clear_row_style(self.inner.borrow().stub_handle, display_row);
    }

    /// Clear all per-row colour overrides.
    pub fn clear_all_row_styles(&self) {
        self.inner.borrow_mut().row_styles.clear();
        #[cfg(target_os = "windows")]
        self.invalidate_view();
        #[cfg(target_os = "macos")]
        crate::platform::appkit::grid_clear_all_row_styles(self.inner.borrow().stub_handle);
    }

    /// Number of rows currently configured.
    pub fn row_count(&self) -> usize {
        self.inner.borrow().row_count
    }
    /// Number of columns currently configured.
    pub fn col_count(&self) -> usize {
        self.inner.borrow().col_count
    }

    /// Alias sintetico per il numero di righe (pendant di
    /// [`Self::row_count`]), da `PIANO_DEBUG_INSPECTOR.md` §A2.
    pub fn rows(&self) -> usize {
        self.row_count()
    }

    /// Alias sintetico per il numero di colonne (pendant di
    /// [`Self::col_count`]), da `PIANO_DEBUG_INSPECTOR.md` §A2.
    pub fn columns(&self) -> usize {
        self.col_count()
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

    fn cell_at_logical(&self, logical_row: usize, col: usize) -> Cell {
        let cell = self.raw_cell_at_logical(logical_row, col);
        let i = self.inner.borrow();
        if let Some(renderer) = i.col_renderers.get(&col) {
            Cell::Text(renderer.render_cell(&cell))
        } else {
            cell
        }
    }

    fn raw_cell_at_logical(&self, logical_row: usize, col: usize) -> Cell {
        let i = self.inner.borrow();
        if is_merge_covered(&i.merge_spans, logical_row, col) {
            return Cell::Empty;
        }
        if let Some(p) = i.provider.as_ref() {
            p(logical_row, col)
        } else {
            i.cells
                .get(&(logical_row, col))
                .cloned()
                .unwrap_or(Cell::Empty)
        }
    }

    fn refresh_logical_cell(&self, logical_row: usize, col: usize) {
        let displays: Vec<usize> = {
            let i = self.inner.borrow();
            if i.row_perm.is_empty() {
                vec![logical_row]
            } else {
                i.row_perm
                    .iter()
                    .enumerate()
                    .filter(|(_, &l)| l == logical_row)
                    .map(|(d, _)| d)
                    .collect()
            }
        };
        for d in displays {
            self.apply_cell(d, col);
        }
    }

    /// Compute the cell value at display `(row, col)` and push it to
    /// the underlying control. The provider receives the **logical**
    /// row index (see [`Self::logical_row`]).
    fn apply_cell(&self, display_row: usize, col: usize) {
        let logical = {
            let i = self.inner.borrow();
            i.row_perm.get(display_row).copied().unwrap_or(display_row)
        };
        let cell = self.cell_at_logical(logical, col);

        // Step 2: push it to the ListView.
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let text = if cell.has_text() {
                cell.text()
            } else {
                String::new()
            };
            let wide = to_wide(&text);
            let lv_col = lv_col_for_data_col(&self.inner.borrow(), col);
            let mut item = LVITEMW {
                mask: LVIF_TEXT,
                i_item: display_row as i32,
                i_sub_item: lv_col as i32,
                state: 0,
                state_mask: 0,
                psz_text: wide.as_ptr() as *mut u16,
                cch_text_max: 0,
                i_image: -1,
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
            } else if !cell.has_text() {
                // Clear a stale icon left by a previous `Image` /
                // `ImageOnly` value.
                item.mask |= LVIF_IMAGE;
            }
            // SAFETY: `wide` lives until `SendMessageW` returns; the
            // ListView copies the text synchronously.
            unsafe {
                SendMessageW(hwnd, LVM_SETITEM, 0, &item as *const LVITEMW as isize);
            }
            if col == 0 {
                win_update_row_label_cell(&self.inner, display_row);
            }
        }
        #[cfg(target_os = "macos")]
        {
            use crate::platform::appkit::{grid_set_cell, GridNativeKind, GridProgressCell};
            let progress = match &cell {
                Cell::Progress { value, max, label } => Some(GridProgressCell {
                    value: *value,
                    max: (*max).max(1),
                    label: label.clone(),
                    kind: GridNativeKind::Progress,
                }),
                Cell::Bar {
                    value,
                    max,
                    label,
                    style,
                    ..
                } if *style == BarStyle::Solid => Some(GridProgressCell {
                    value: *value,
                    max: (*max).max(1),
                    label: label.clone(),
                    kind: GridNativeKind::Bar,
                }),
                Cell::Priority { kind } => Some(GridProgressCell {
                    value: kind.sort_rank() as u32,
                    max: 4,
                    label: Some(kind.label().to_string()),
                    kind: GridNativeKind::Priority,
                }),
                _ => None,
            };
            let text = if progress.is_some() {
                String::new()
            } else if cell.has_text() {
                cell.display_text()
            } else {
                String::new()
            };
            grid_set_cell(
                self.inner.borrow().stub_handle,
                display_row,
                col,
                &text,
                cell.image(),
                progress,
            );
        }
        #[cfg(target_os = "linux")]
        {
            // GTK non ha barre di progresso native nel TreeView: le
            // varianti testuali vengono renderizzate come testo.
            let text = if cell.has_text() {
                cell.display_text()
            } else {
                String::new()
            };
            crate::platform::gtk::grid_set_cell(
                self.inner.borrow().stub_handle,
                display_row,
                col,
                &text,
            );
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            let _ = cell;
        }
    }

    /// Install a unified GTK notify hub (selection, header sort, row activate).
    #[cfg(target_os = "linux")]
    fn ensure_linux_notify_hub(&self, frame: &Frame) {
        if self.inner.borrow().notify_hub {
            return;
        }
        self.inner.borrow_mut().notify_hub = true;
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_notify_handler(
            id,
            Box::new(move |code| {
                use crate::platform::gtk::{
                    grid_take_activate, grid_take_header_click, GRID_NOTIFY_ACTIVATE,
                    GRID_NOTIFY_HEADER_CLICK, GRID_NOTIFY_ITEM_CHANGED,
                };
                if code == GRID_NOTIFY_ITEM_CHANGED {
                    fire_selection_callbacks(&inner);
                    return;
                }
                if code == GRID_NOTIFY_ACTIVATE {
                    let pending = {
                        let handle = inner.borrow().stub_handle;
                        grid_take_activate(handle)
                    };
                    if let Some((row, col)) = pending {
                        let mut cb = inner.borrow_mut().on_row_activated.take();
                        if let Some(ref mut handler) = cb {
                            handler(row, col);
                        }
                        inner.borrow_mut().on_row_activated = cb;
                    }
                    return;
                }
                if code == GRID_NOTIFY_HEADER_CLICK {
                    let pending = {
                        let handle = inner.borrow().stub_handle;
                        grid_take_header_click(handle)
                    };
                    if let Some(col) = pending {
                        handle_header_item_click(&inner, col);
                    }
                }
            }),
        );
    }

    /// Install a unified AppKit notify hub (selection, header sort, row activate).
    #[cfg(target_os = "macos")]
    fn ensure_macos_notify_hub(&self, frame: &Frame) {
        if self.inner.borrow().notify_hub {
            return;
        }
        self.inner.borrow_mut().notify_hub = true;
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_notify_handler(
            id,
            Box::new(move |code| {
                use crate::platform::appkit::{
                    grid_take_activate_cell, grid_take_cell_edited, grid_take_context_menu_target,
                    grid_take_edit_start, grid_take_header_sort_click, grid_take_highlight_click,
                    grid_take_link_click, grid_take_paste_at, grid_take_row_reordered,
                    GRID_NOTIFY_CELL_EDITED, GRID_NOTIFY_COLUMN_RESIZED, GRID_NOTIFY_CONTEXT_MENU,
                    GRID_NOTIFY_EDIT_START, GRID_NOTIFY_FOCUS_CHANGED, GRID_NOTIFY_HEADER_CLICK,
                    GRID_NOTIFY_HIGHLIGHT_CLICK, GRID_NOTIFY_ITEM_ACTIVATE,
                    GRID_NOTIFY_ITEM_CHANGED, GRID_NOTIFY_LINK_CLICK, GRID_NOTIFY_PASTE_REQUEST,
                    GRID_NOTIFY_ROW_REORDERED,
                };
                if code == GRID_NOTIFY_ITEM_CHANGED {
                    fire_selection_callbacks(&inner);
                    return;
                }
                if code == GRID_NOTIFY_FOCUS_CHANGED {
                    if let Some((row, col)) = {
                        let h = inner.borrow().stub_handle;
                        crate::platform::appkit::grid_get_focused_cell(h)
                    } {
                        let mut i = inner.borrow_mut();
                        i.focused_cell = Some((row, col));
                        let cb = i.on_focused_cell_changed.take();
                        drop(i);
                        if let Some(mut c) = cb {
                            c(row, col);
                            inner.borrow_mut().on_focused_cell_changed = Some(c);
                        }
                    }
                    return;
                }
                if code == GRID_NOTIFY_EDIT_START {
                    if let Some((row, col)) = grid_take_edit_start(inner.borrow().stub_handle) {
                        inner.borrow_mut().focused_cell = Some((row, col));
                        let grid = Grid {
                            inner: inner.clone(),
                        };
                        let _ = grid.begin_cell_edit();
                    }
                    return;
                }
                if code == GRID_NOTIFY_CELL_EDITED {
                    if let Some((display_row, col, text)) =
                        grid_take_cell_edited(inner.borrow().stub_handle)
                    {
                        handle_cell_edited(&inner, display_row, col, &text);
                    }
                    return;
                }
                if code == GRID_NOTIFY_PASTE_REQUEST {
                    let pending = {
                        let handle = inner.borrow().stub_handle;
                        grid_take_paste_at(handle)
                    };
                    if let Some((row, col)) = pending {
                        let grid = Grid {
                            inner: inner.clone(),
                        };
                        let _ = grid.paste_from_clipboard(row, col);
                    }
                    return;
                }
                if code == GRID_NOTIFY_HEADER_CLICK {
                    let pending = {
                        let handle = inner.borrow().stub_handle;
                        grid_take_header_sort_click(handle)
                    };
                    if let Some((col, ascending)) = pending {
                        apply_header_sort(&inner, col, ascending);
                    }
                    return;
                }
                if code == GRID_NOTIFY_HIGHLIGHT_CLICK {
                    if let Some((row, col)) = grid_take_highlight_click(inner.borrow().stub_handle)
                    {
                        apply_reference_highlight(&inner, row, col);
                    }
                    return;
                }
                if code == GRID_NOTIFY_ITEM_ACTIVATE {
                    if let Some((row, col)) = grid_take_activate_cell(inner.borrow().stub_handle) {
                        let mut cb = inner.borrow_mut().on_row_activated.take();
                        if let Some(ref mut handler) = cb {
                            handler(row, col);
                        }
                        inner.borrow_mut().on_row_activated = cb;
                    }
                    return;
                }
                if code == GRID_NOTIFY_COLUMN_RESIZED {
                    let handle = inner.borrow().stub_handle;
                    let widths = crate::platform::appkit::grid_snapshot_column_widths(handle);
                    let mut i = inner.borrow_mut();
                    for (col, width) in widths.into_iter().enumerate() {
                        if col < i.col_widths.len() {
                            i.col_widths[col] = width;
                        }
                    }
                    return;
                }
                if code == GRID_NOTIFY_CONTEXT_MENU {
                    if let Some((in_header, row, col)) =
                        grid_take_context_menu_target(inner.borrow().stub_handle)
                    {
                        handle_macos_context_menu(&inner, in_header, row, col);
                    }
                    return;
                }
                if code == GRID_NOTIFY_ROW_REORDERED {
                    if let Some((from, to)) = grid_take_row_reordered(inner.borrow().stub_handle) {
                        let grid = Grid {
                            inner: inner.clone(),
                        };
                        let _ = grid.move_display_row(from, to);
                    }
                    return;
                }
                if code == GRID_NOTIFY_LINK_CLICK {
                    let pending = {
                        let handle = inner.borrow().stub_handle;
                        grid_take_link_click(handle)
                    };
                    if let Some((row, col)) = pending {
                        let grid = Grid {
                            inner: inner.clone(),
                        };
                        let _ = grid.open_link_at(row, col);
                    }
                }
            }),
        );
    }
}

#[cfg(target_os = "macos")]
fn handle_macos_context_menu(
    inner: &Rc<RefCell<GridInner>>,
    in_header: bool,
    row: usize,
    col: usize,
) {
    let frame = match inner
        .try_borrow()
        .ok()
        .and_then(|i| i.context_frame.clone())
    {
        Some(f) => f,
        None => return,
    };
    if in_header {
        if !inner
            .try_borrow()
            .ok()
            .is_some_and(|i| i.context_menu_enabled)
        {
            return;
        }
        if let Ok(mut i) = inner.try_borrow_mut() {
            i.context_menu_col = col;
        }
        let mut col_cb = inner.borrow_mut().on_column_context_menu.take();
        if let Some(ref mut handler) = col_cb {
            handler(&frame, col);
        } else {
            let grid = Grid {
                inner: inner.clone(),
            };
            grid.popup_column_context_menu(&frame);
        }
        inner.borrow_mut().on_column_context_menu = col_cb;
        return;
    }
    if row != usize::MAX
        && inner
            .try_borrow()
            .ok()
            .is_some_and(|i| i.row_context_menu_enabled)
    {
        crate::platform::appkit::grid_select_row(inner.borrow().stub_handle, row);
        {
            let Ok(mut i) = inner.try_borrow_mut() else {
                return;
            };
            i.context_menu_row = row;
            i.context_menu_row_col = col;
            i.last_selection = Some(row);
        }
        let mut cb = inner.borrow_mut().on_row_context_menu.take();
        if let Some(ref mut handler) = cb {
            handler(&frame, row, col);
        }
        inner.borrow_mut().on_row_context_menu = cb;
        return;
    }
    if inner
        .try_borrow()
        .ok()
        .is_some_and(|i| i.context_menu_enabled)
    {
        if let Ok(mut i) = inner.try_borrow_mut() {
            i.context_menu_col = col;
        }
        let mut col_cb = inner.borrow_mut().on_column_context_menu.take();
        if let Some(ref mut handler) = col_cb {
            handler(&frame, col);
        } else {
            let grid = Grid {
                inner: inner.clone(),
            };
            grid.popup_column_context_menu(&frame);
        }
        inner.borrow_mut().on_column_context_menu = col_cb;
    }
}

#[cfg(target_os = "macos")]
type Rgba = (u8, u8, u8, u8);

#[cfg(target_os = "macos")]
fn colour_rgba(c: Colour) -> Rgba {
    (c.r, c.g, c.b, c.a)
}

#[cfg(target_os = "macos")]
fn appearance_to_appkit(a: GridAppearance) -> crate::platform::appkit::GridAppearanceData {
    crate::platform::appkit::GridAppearanceData {
        system_theme: a.system_theme,
        alternating_even: colour_rgba(a.alternating_even),
        alternating_odd: colour_rgba(a.alternating_odd),
        default_text: colour_rgba(a.default_text),
        header_foreground: colour_rgba(a.header_foreground),
        header_background: colour_rgba(a.header_background),
        selection_foreground: colour_rgba(a.selection_foreground),
        selection_background: colour_rgba(a.selection_background),
    }
}

fn apply_reference_highlight(
    inner: &Rc<RefCell<GridInner>>,
    row: Option<usize>,
    col: Option<usize>,
) {
    if !inner
        .try_borrow()
        .ok()
        .is_some_and(|i| i.row_col_highlight_enabled)
    {
        return;
    }
    let mut changed = false;
    {
        let mut i = match inner.try_borrow_mut() {
            Ok(i) => i,
            Err(_) => return,
        };
        if let Some(r) = row {
            if i.highlighted_row != Some(r) {
                i.highlighted_row = Some(r);
                changed = true;
            }
        }
        if let Some(c) = col {
            if i.highlighted_col != Some(c) {
                i.highlighted_col = Some(c);
                changed = true;
            }
        }
    }
    if changed {
        let grid = Grid {
            inner: inner.clone(),
        };
        grid.push_row_column_highlights_to_native();
    }
}

fn apply_header_sort(inner: &Rc<RefCell<GridInner>>, col: usize, ascending: bool) {
    let enabled = inner.try_borrow().ok().is_some_and(|i| i.header_click_sort);
    let col_count = inner.try_borrow().ok().map(|i| i.col_count).unwrap_or(0);
    if !enabled || col >= col_count {
        return;
    }
    let order = if ascending {
        SortOrder::Ascending
    } else {
        SortOrder::Descending
    };
    let already_sorted = inner
        .try_borrow()
        .ok()
        .is_some_and(|i| i.sort_col == Some(col) && i.sort_order == Some(order));
    if already_sorted {
        return;
    }
    let grid = Grid {
        inner: inner.clone(),
    };
    grid.apply_sort_column(col, order, false);
}

fn handle_header_item_click(inner: &Rc<RefCell<GridInner>>, col: usize) {
    let order = {
        let i = match inner.try_borrow() {
            Ok(i) => i,
            Err(_) => return,
        };
        if i.sort_col == Some(col) {
            match i.sort_order {
                Some(SortOrder::Ascending) => SortOrder::Descending,
                _ => SortOrder::Ascending,
            }
        } else {
            SortOrder::Ascending
        }
    };
    apply_header_sort(inner, col, matches!(order, SortOrder::Ascending));
}

#[cfg(target_os = "windows")]
fn set_window_theme(hwnd: HWND, theme: Option<&str>) {
    // SAFETY: `uxtheme.dll` is a system DLL; `SetWindowTheme` is optional.
    unsafe {
        let dll = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
        if dll.is_null() {
            return;
        }
        type SetWindowThemeFn = unsafe extern "system" fn(HWND, *const u16, *const u16) -> i32;
        let Some(proc) = GetProcAddress(dll, c"SetWindowTheme".as_ptr().cast()) else {
            return;
        };
        let f: SetWindowThemeFn = std::mem::transmute(proc);
        let empty = to_wide("");
        match theme {
            Some(name) => {
                let primary = to_wide(name);
                f(hwnd, primary.as_ptr(), std::ptr::null());
            }
            None => {
                f(hwnd, empty.as_ptr(), empty.as_ptr());
            }
        }
    }
}

fn compare_column_cells(a: &Cell, b: &Cell, kind: ColumnSortKind) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match kind {
        ColumnSortKind::Auto => compare_cells_typed(a, b),
        ColumnSortKind::Text => compare_text_lexical(&a.display_text(), &b.display_text()),
        ColumnSortKind::Number => compare_numeric_strings(&a.display_text(), &b.display_text()),
        ColumnSortKind::Integer => {
            let na = parse_numeric_token(&a.display_text()).map(f64::trunc);
            let nb = parse_numeric_token(&b.display_text()).map(f64::trunc);
            match (na, nb) {
                (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => compare_text_lexical(&a.display_text(), &b.display_text()),
            }
        }
        ColumnSortKind::Date => {
            let ka = cell_date_sort_key(a);
            let kb = cell_date_sort_key(b);
            ka.cmp(&kb)
                .then_with(|| compare_text_lexical(&a.display_text(), &b.display_text()))
        }
    }
}

fn compare_cells_typed(a: &Cell, b: &Cell) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Cell::Empty, Cell::Empty) => Ordering::Equal,
        (Cell::Empty, _) => Ordering::Greater,
        (_, Cell::Empty) => Ordering::Less,
        (Cell::Number { value: va, .. }, Cell::Number { value: vb, .. }) => {
            va.partial_cmp(vb).unwrap_or(Ordering::Equal)
        }
        (Cell::DateTime { iso: ia, .. }, Cell::DateTime { iso: ib, .. }) => {
            iso_sort_key(ia).cmp(&iso_sort_key(ib))
        }
        (
            Cell::Progress {
                value: va, max: ma, ..
            },
            Cell::Progress {
                value: vb, max: mb, ..
            },
        )
        | (
            Cell::Bar {
                value: va, max: ma, ..
            },
            Cell::Bar {
                value: vb, max: mb, ..
            },
        ) => compare_ratio(*va, *ma, *vb, *mb),
        (Cell::Priority { kind: ka }, Cell::Priority { kind: kb }) => {
            ka.sort_rank().cmp(&kb.sort_rank())
        }
        (Cell::Stars { value: va, max: ma }, Cell::Stars { value: vb, max: mb }) => {
            va.cmp(vb).then_with(|| ma.cmp(mb))
        }
        (Cell::Badge { text: ta, kind: ka }, Cell::Badge { text: tb, kind: kb }) => ka
            .sort_rank()
            .cmp(&kb.sort_rank())
            .then_with(|| compare_text_lexical(ta, tb)),
        (
            Cell::Image {
                text: ta, idx: ia, ..
            },
            Cell::Image {
                text: tb, idx: ib, ..
            },
        ) => compare_text_lexical(ta, tb).then_with(|| ia.cmp(ib)),
        (Cell::ImageOnly(ia), Cell::ImageOnly(ib)) => ia.cmp(ib),
        (Cell::Link { text: ta, .. }, Cell::Link { text: tb, .. }) => compare_text_lexical(ta, tb),
        (Cell::Text(ta) | Cell::MultiLine(ta), Cell::Text(tb) | Cell::MultiLine(tb)) => {
            compare_text_smart(ta, tb)
        }
        _ => {
            let sa = cell_sort_scalar(a);
            let sb = cell_sort_scalar(b);
            sa.partial_cmp(&sb)
                .unwrap_or(Ordering::Equal)
                .then_with(|| compare_text_lexical(&a.display_text(), &b.display_text()))
        }
    }
}

fn compare_ratio(va: u32, ma: u32, vb: u32, mb: u32) -> std::cmp::Ordering {
    bar_ratio(va, ma)
        .partial_cmp(&bar_ratio(vb, mb))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| va.cmp(&vb))
}

fn bar_ratio(value: u32, max: u32) -> f64 {
    if max == 0 {
        value as f64
    } else {
        value as f64 / max as f64
    }
}

fn cell_date_sort_key(cell: &Cell) -> i64 {
    match cell {
        Cell::DateTime { iso, .. } => iso_sort_key(iso),
        other => iso_sort_key(&other.display_text()),
    }
}

fn cell_sort_scalar(cell: &Cell) -> f64 {
    match cell {
        Cell::Empty => f64::NAN,
        Cell::Number { value, .. } => *value,
        Cell::DateTime { iso, .. } => iso_sort_key(iso) as f64,
        Cell::Progress { value, max, .. } | Cell::Bar { value, max, .. } => bar_ratio(*value, *max),
        Cell::Priority { kind } => kind.sort_rank() as f64,
        Cell::Stars { value, .. } => *value as f64,
        Cell::Badge { kind, .. } => kind.sort_rank() as f64,
        Cell::ImageOnly(idx) => *idx as f64,
        _ => parse_numeric_token(&cell.display_text()).unwrap_or(f64::NAN),
    }
}

fn iso_sort_key(iso: &str) -> i64 {
    let iso = iso.trim();
    if iso.is_empty() {
        return 0;
    }
    let (date_part, time_part) = match iso.split_once(' ') {
        Some((d, t)) => (d, Some(t)),
        None => (iso, None),
    };
    let mut dp = date_part.split('-');
    let y: i64 = dp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let m: i64 = dp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let day: i64 = dp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let (h, min, sec) = if let Some(t) = time_part {
        let mut tp = t.split(':');
        (
            tp.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            tp.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            tp.next().and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };
    y * 10_000_000_000 + m * 100_000_000 + day * 1_000_000 + h * 10_000 + min * 100 + sec
}

fn compare_text_lexical(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
}

fn compare_text_smart(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (parse_numeric_token(a), parse_numeric_token(b)) {
        (Some(na), Some(nb)) => na.partial_cmp(&nb).unwrap_or(Ordering::Equal),
        _ => compare_text_lexical(a, b),
    }
}

fn compare_numeric_strings(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (parse_numeric_token(a), parse_numeric_token(b)) {
        (Some(na), Some(nb)) => na.partial_cmp(&nb).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => compare_text_lexical(a, b),
    }
}

fn parse_numeric_token(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s == "\u{2014}" || s.eq_ignore_ascii_case("n/a") {
        return None;
    }
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '-')
        .collect();
    if cleaned.is_empty() || cleaned == "-" {
        return None;
    }
    let normalized = if cleaned.contains(',') && cleaned.contains('.') {
        let last_dot = cleaned.rfind('.');
        let last_comma = cleaned.rfind(',');
        match (last_dot, last_comma) {
            (Some(d), Some(c)) if d > c => cleaned.replace(',', ""),
            (Some(_), Some(_)) => cleaned.replace('.', "").replace(',', "."),
            _ => cleaned.replace(',', ""),
        }
    } else if cleaned.contains(',') {
        if let Some(i) = cleaned.rfind(',') {
            let after = &cleaned[i + 1..];
            if after.len() == 3 && after.chars().all(|c| c.is_ascii_digit()) {
                cleaned.replace(',', "")
            } else {
                cleaned.replace(',', ".")
            }
        } else {
            cleaned
        }
    } else {
        cleaned
    };
    normalized.parse().ok()
}

fn compare_cells_for_sort(a: &str, b: &str) -> std::cmp::Ordering {
    compare_numeric_strings(a, b)
}

fn compact_logical_cells(inner: &mut GridInner, removed_logical: usize) {
    let old = std::mem::take(&mut inner.cells);
    for ((r, c), cell) in old {
        if r < removed_logical {
            inner.cells.insert((r, c), cell);
        } else if r > removed_logical {
            inner.cells.insert((r - 1, c), cell);
        }
    }
}

fn shift_display_styles(inner: &mut GridInner, removed_display: usize) {
    let old_rows = std::mem::take(&mut inner.row_styles);
    for (r, s) in old_rows {
        if r < removed_display {
            inner.row_styles.insert(r, s);
        } else if r > removed_display {
            inner.row_styles.insert(r - 1, s);
        }
    }
    let old_cells = std::mem::take(&mut inner.cell_styles);
    for ((r, c), s) in old_cells {
        if r < removed_display {
            inner.cell_styles.insert((r, c), s);
        } else if r > removed_display {
            inner.cell_styles.insert((r - 1, c), s);
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
        #[cfg(target_os = "macos")]
        {
            self.stub_handle
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
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
            let dpi = GetDpiForWindow(self.hwnd);
            for (idx, &cw) in self.col_widths.iter().enumerate() {
                let physical_cw = dpi.scale(cw as u32) as i32;
                SendMessageW(self.hwnd, LVM_SETCOLUMNWIDTH, idx, physical_cw as isize);
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
            crate::platform::appkit::grid_sync_view_geometry(self.stub_handle, w as f64, h as f64);
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_visible(self.stub_handle, visible);
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_enabled(self.stub_handle, enabled);
        }
    }
}

#[cfg(target_os = "windows")]
impl GridInner {
    fn resolve_style(&self, row: usize, col: usize) -> GridCellStyle {
        let appearance = self.appearance;
        if let Some(s) = self.cell_styles.get(&(row, col)) {
            return *s;
        }
        if let Some(s) = self.row_styles.get(&row) {
            return *s;
        }
        if self.row_col_highlight_enabled {
            let in_row = self.highlighted_row == Some(row);
            let in_col = self.highlighted_col == Some(col);
            if in_row || in_col {
                let bg = if in_row && in_col {
                    Colour::new(225, 236, 252, 255)
                } else if in_row {
                    Colour::new(255, 249, 219, 255)
                } else {
                    Colour::new(232, 244, 255, 255)
                };
                return GridCellStyle {
                    foreground: None,
                    background: Some(bg),
                };
            }
        }
        if appearance.system_theme {
            return GridCellStyle::default();
        }
        let mut style = if let Some((even, odd)) = self.alternating_rows {
            let bg = if row.is_multiple_of(2) { even } else { odd };
            GridCellStyle {
                foreground: None,
                background: Some(bg),
            }
        } else {
            GridCellStyle::default()
        };
        if style.foreground.is_none() {
            style.foreground = Some(appearance.default_text);
        }
        style
    }

    fn has_explicit_style(&self, row: usize, col: usize) -> bool {
        self.cell_styles.contains_key(&(row, col))
            || self.row_styles.contains_key(&row)
            || (self.row_col_highlight_enabled
                && (self.highlighted_row == Some(row) || self.highlighted_col == Some(col)))
    }
}

#[cfg(target_os = "windows")]
type GridWndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[cfg(target_os = "windows")]
thread_local! {
    static GRID_LIST_ORIG_PROCS: std::cell::RefCell<HashMap<isize, GridWndProc>> =
        std::cell::RefCell::new(HashMap::new());
    static GRID_HEADER_ORIG_PROCS: std::cell::RefCell<HashMap<isize, GridWndProc>> =
        std::cell::RefCell::new(HashMap::new());
    static GRID_CONTEXT_GRIDS: std::cell::RefCell<HashMap<isize, Rc<RefCell<GridInner>>>> =
        std::cell::RefCell::new(HashMap::new());
}

#[cfg(target_os = "windows")]
fn install_list_subclass_if_needed(grid: &Grid) {
    let (hwnd, enabled) = {
        let i = grid.inner.borrow();
        (
            i.hwnd,
            i.visual_hooks_enabled
                || i.row_context_menu_enabled
                || i.header_click_sort
                || i.row_col_highlight_enabled,
        )
    };
    if !enabled || hwnd.is_null() {
        return;
    }
    let key = hwnd as isize;
    let already = GRID_LIST_ORIG_PROCS.with(|m| m.borrow().contains_key(&key));
    if !already {
        install_list_subclass(hwnd, grid.inner.clone());
    }
}

#[cfg(target_os = "windows")]
fn set_listview_ex_style(grid: &Grid, flag: u32, enabled: bool) {
    let hwnd = grid.inner.borrow().hwnd;
    if hwnd.is_null() {
        return;
    }
    // SAFETY: standard extended-style toggle on a live ListView.
    unsafe {
        let current = SendMessageW(hwnd, LVM_GETEXTENDEDLISTVIEWSTYLE, 0, 0) as u32;
        let new = if enabled {
            current | flag
        } else {
            current & !flag
        };
        SendMessageW(hwnd, LVM_SETEXTENDEDLISTVIEWSTYLE, 0, new as isize);
    }
}

#[cfg(target_os = "windows")]
fn update_header_sort_indicators(inner: &Rc<RefCell<GridInner>>) {
    let (header, col_count, sort_col, sort_order) = match inner.try_borrow() {
        Ok(i) => (i.header_hwnd, i.col_count, i.sort_col, i.sort_order),
        Err(_) => return,
    };
    if header.is_null() || col_count == 0 {
        return;
    }
    // SAFETY: valid header HWND; `HDITEMW` fmt field is read/written per column.
    unsafe {
        for c in 0..col_count {
            let mut item = HDITEMW {
                mask: HDI_FORMAT,
                cxy: 0,
                pszText: std::ptr::null_mut(),
                hbm: std::ptr::null_mut(),
                cchTextMax: 0,
                fmt: 0,
                lParam: 0,
                iImage: 0,
                iOrder: 0,
                r#type: 0,
                pvFilter: std::ptr::null_mut(),
                state: 0,
            };
            SendMessageW(header, HDM_GETITEMW, c, &mut item as *mut HDITEMW as isize);
            item.fmt &= !(HDF_SORTUP | HDF_SORTDOWN);
            if sort_col == Some(c) {
                item.fmt |= match sort_order {
                    Some(SortOrder::Ascending) | Some(SortOrder::None) => HDF_SORTUP,
                    Some(SortOrder::Descending) => HDF_SORTDOWN,
                    None => 0,
                };
            }
            SendMessageW(header, HDM_SETITEMW, c, &mut item as *mut HDITEMW as isize);
        }
        InvalidateRect(header, std::ptr::null(), 1);
    }
}

#[cfg(target_os = "windows")]
fn select_display_row(list: HWND, row: usize) {
    // SAFETY: `LVM_SETITEMSTATE` dereferenzia `lParam` come `LVITEMW*`
    // (legge solo `state`/`stateMask`): la struct è zeroed e resta
    // viva finché `SendMessageW` non ritorna.
    unsafe {
        // Prima deseleziona tutto (indice -1), poi seleziona la riga.
        let mut clear: LVITEMW = std::mem::zeroed();
        clear.i_item = -1;
        clear.state = 0;
        clear.state_mask = LVIS_SELECTED;
        SendMessageW(
            list,
            LVM_SETITEMSTATE,
            usize::MAX,
            &clear as *const LVITEMW as isize,
        );
        let mut set: LVITEMW = std::mem::zeroed();
        set.i_item = row as i32;
        set.state = LVIS_SELECTED | LVIS_FOCUSED;
        set.state_mask = LVIS_SELECTED | LVIS_FOCUSED;
        SendMessageW(list, LVM_SETITEMSTATE, row, &set as *const LVITEMW as isize);
    }
}

#[cfg(target_os = "windows")]
fn hit_test_list_point(list: HWND, screen_x: i32, screen_y: i32) -> (i32, usize) {
    // SAFETY: `list` is non-null; `ScreenToClient` only mutates `pt`.
    unsafe {
        let mut pt = POINT {
            x: screen_x,
            y: screen_y,
        };
        ScreenToClient(list, &mut pt);
        let mut info = LvHitTestInfo {
            pt,
            flags: 0,
            i_item: -1,
            i_sub_item: 0,
        };
        let _ = SendMessageW(
            list,
            LVM_SUBITEMHITTEST,
            0,
            &mut info as *mut LvHitTestInfo as isize,
        );
        let col = if info.i_sub_item >= 0 {
            info.i_sub_item as usize
        } else {
            0
        };
        (info.i_item, col)
    }
}

#[cfg(target_os = "windows")]
fn handle_header_divider_dblclick(inner: &Rc<RefCell<GridInner>>, col: usize) {
    if col >= inner.try_borrow().ok().map(|i| i.col_count).unwrap_or(0) {
        return;
    }
    let grid = Grid {
        inner: inner.clone(),
    };
    grid.autosize_column(col);
}

#[cfg(target_os = "windows")]
fn handle_lv_get_infotip(inner: &Rc<RefCell<GridInner>>, lparam: isize) {
    // SAFETY: `lparam` is `NMLVGETINFOTIPW` from the ListView parent.
    unsafe {
        let p = lparam as *mut NMLVGETINFOTIPW;
        if p.is_null() {
            return;
        }
        let row = (*p).iItem;
        let col = (*p).iSubItem;
        if row < 0 {
            return;
        }
        let tip = {
            let Ok(i) = inner.try_borrow() else {
                return;
            };
            i.cell_tooltip_provider
                .as_ref()
                .and_then(|f| f(row as usize, col as usize))
        };
        let Some(text) = tip else {
            return;
        };
        if (*p).pszText.is_null() || (*p).cchTextMax <= 0 {
            return;
        }
        let wide = to_wide(&text);
        let max = ((*p).cchTextMax as usize).saturating_sub(1);
        let copy_len = wide.len().min(max);
        std::ptr::copy_nonoverlapping(wide.as_ptr(), (*p).pszText, copy_len);
        *(*p).pszText.add(copy_len) = 0;
    }
}

#[cfg(target_os = "windows")]
fn handle_lv_item_activate(inner: &Rc<RefCell<GridInner>>, lparam: isize) {
    // SAFETY: `lparam` is `NMITEMACTIVATE` from the ListView parent.
    unsafe {
        let p = lparam as *const NMITEMACTIVATE;
        if p.is_null() {
            return;
        }
        let row = (*p).iItem;
        let col = (*p).iSubItem;
        if row < 0 {
            return;
        }
        let mut cb = inner.borrow_mut().on_row_activated.take();
        if let Some(ref mut handler) = cb {
            handler(row as usize, col.max(0) as usize);
        }
        inner.borrow_mut().on_row_activated = cb;
    }
}

#[cfg(target_os = "windows")]
fn open_row_context_menu(inner: &Rc<RefCell<GridInner>>, list: HWND, screen_x: i32, screen_y: i32) {
    let enabled = inner
        .try_borrow()
        .ok()
        .is_some_and(|i| i.row_context_menu_enabled);
    if !enabled {
        return;
    }
    let (row, lv_col) = hit_test_list_point(list, screen_x, screen_y);
    if row < 0 {
        return;
    }
    let row = row as usize;
    let col = {
        let i = match inner.try_borrow() {
            Ok(i) => i,
            Err(_) => return,
        };
        data_col_from_lv_col(&i, lv_col).unwrap_or(lv_col)
    };
    let frame = {
        let i = match inner.try_borrow() {
            Ok(i) => i,
            Err(_) => return,
        };
        match i.context_frame.clone() {
            Some(f) => f,
            None => return,
        }
    };
    select_display_row(list, row);
    {
        let Ok(mut i) = inner.try_borrow_mut() else {
            return;
        };
        i.context_menu_row = row;
        i.context_menu_row_col = col;
        i.last_selection = Some(row);
    }
    let mut cb = inner.borrow_mut().on_row_context_menu.take();
    if let Some(ref mut handler) = cb {
        handler(&frame, row, col);
    }
    inner.borrow_mut().on_row_context_menu = cb;
}

#[cfg(target_os = "windows")]
fn open_grid_context_menu(hwnd: HWND, inner: &Rc<RefCell<GridInner>>, x: i32, y: i32) {
    let header_is_target = inner
        .try_borrow()
        .ok()
        .is_some_and(|i| !i.header_hwnd.is_null() && hwnd == i.header_hwnd);
    if header_is_target {
        open_column_context_menu(hwnd, inner, x, y);
        return;
    }
    let (list, row_menu) = match inner.try_borrow() {
        Ok(i) => (i.hwnd, i.row_context_menu_enabled),
        Err(_) => return,
    };
    if hwnd == list && row_menu {
        let (row, _) = hit_test_list_point(list, x, y);
        if row >= 0 {
            open_row_context_menu(inner, list, x, y);
            return;
        }
    }
    open_column_context_menu(hwnd, inner, x, y);
}

#[cfg(target_os = "windows")]
fn install_list_subclass(hwnd: HWND, inner: Rc<RefCell<GridInner>>) {
    // SAFETY: `hwnd` is a live ListView returned by `CreateWindowExW`.
    unsafe {
        let key = hwnd as isize;
        GRID_CONTEXT_GRIDS.with(|m| m.borrow_mut().insert(key, inner));
        let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
        let original_proc: GridWndProc = std::mem::transmute(original);
        GRID_LIST_ORIG_PROCS.with(|m| m.borrow_mut().insert(key, original_proc));
        SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            grid_list_wnd_proc as *const () as usize as isize,
        );
    }
}

#[cfg(target_os = "windows")]
fn install_header_subclass(hwnd: HWND, inner: Rc<RefCell<GridInner>>) {
    // SAFETY: `hwnd` is the report-view header child HWND.
    unsafe {
        let key = hwnd as isize;
        GRID_CONTEXT_GRIDS.with(|m| m.borrow_mut().insert(key, inner));
        let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
        let original_proc: GridWndProc = std::mem::transmute(original);
        GRID_HEADER_ORIG_PROCS.with(|m| m.borrow_mut().insert(key, original_proc));
        SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            grid_header_wnd_proc as *const () as usize as isize,
        );
    }
}

#[cfg(target_os = "windows")]
fn column_at_header_point(header: HWND, screen_x: i32, screen_y: i32) -> usize {
    // SAFETY: `header` is non-null; `ScreenToClient` only mutates `pt`.
    unsafe {
        let mut pt = POINT {
            x: screen_x,
            y: screen_y,
        };
        ScreenToClient(header, &mut pt);
        let mut info = HdHitTestInfo {
            pt,
            flags: 0,
            i_item: -1,
        };
        let idx = SendMessageW(
            header,
            HDM_HITTEST,
            0,
            &mut info as *mut HdHitTestInfo as isize,
        );
        if idx >= 0 {
            idx as usize
        } else {
            0
        }
    }
}

#[cfg(target_os = "windows")]
fn column_at_list_point(list: HWND, screen_x: i32, screen_y: i32) -> usize {
    // SAFETY: `list` is non-null; `ScreenToClient` only mutates `pt`.
    unsafe {
        let mut pt = POINT {
            x: screen_x,
            y: screen_y,
        };
        ScreenToClient(list, &mut pt);
        let mut info = LvHitTestInfo {
            pt,
            flags: 0,
            i_item: -1,
            i_sub_item: 0,
        };
        let _ = SendMessageW(
            list,
            LVM_SUBITEMHITTEST,
            0,
            &mut info as *mut LvHitTestInfo as isize,
        );
        if info.i_sub_item >= 0 {
            info.i_sub_item as usize
        } else {
            0
        }
    }
}

#[cfg(target_os = "windows")]
fn open_column_context_menu(hwnd: HWND, inner: &Rc<RefCell<GridInner>>, x: i32, y: i32) {
    let frame = {
        let i = match inner.try_borrow() {
            Ok(i) => i,
            Err(_) => return,
        };
        match i.context_frame.clone() {
            Some(f) => f,
            None => return,
        }
    };
    let col = {
        let i = match inner.try_borrow() {
            Ok(i) => i,
            Err(_) => return,
        };
        if !i.header_hwnd.is_null() && hwnd == i.header_hwnd {
            column_at_header_point(i.header_hwnd, x, y)
        } else {
            column_at_list_point(i.hwnd, x, y)
        }
    };
    let Ok(mut i) = inner.try_borrow_mut() else {
        return;
    };
    i.context_menu_col = col;
    drop(i);
    let mut col_cb = inner.borrow_mut().on_column_context_menu.take();
    if let Some(ref mut handler) = col_cb {
        handler(&frame, col);
    } else {
        let grid = Grid {
            inner: inner.clone(),
        };
        grid.popup_column_context_menu(&frame);
    }
    inner.borrow_mut().on_column_context_menu = col_cb;
}

#[cfg(target_os = "windows")]
fn handle_header_custom_draw(inner: &Rc<RefCell<GridInner>>, lparam: isize) -> u32 {
    // SAFETY: `lparam` is the `NMCUSTOMDRAW` pointer from the header.
    unsafe {
        let p = lparam as *mut NmCustomDraw;
        if p.is_null() {
            return CDRF_DODEFAULT;
        }
        let (system_theme, appearance) = match inner.try_borrow() {
            Ok(i) => (i.appearance.system_theme, i.appearance),
            Err(_) => return CDRF_DODEFAULT,
        };
        match (*p).dw_draw_stage {
            CDDS_PREPAINT => {
                if system_theme {
                    CDRF_DODEFAULT
                } else {
                    CDRF_NOTIFYITEMDRAW
                }
            }
            CDDS_ITEMPREPAINT => {
                if system_theme {
                    CDRF_DODEFAULT
                } else {
                    let col = (*p).dw_item_spec;
                    let highlighted = inner.try_borrow().ok().and_then(|i| {
                        if i.row_col_highlight_enabled {
                            i.highlighted_col
                        } else {
                            None
                        }
                    });
                    if highlighted == Some(col) {
                        (*p).clr_text = appearance.header_foreground.to_colorref();
                        (*p).clr_text_bk = Colour::new(191, 219, 254, 255).to_colorref();
                        CDRF_NEWFONT
                    } else {
                        (*p).clr_text = appearance.header_foreground.to_colorref();
                        (*p).clr_text_bk = appearance.header_background.to_colorref();
                        CDRF_NEWFONT
                    }
                }
            }
            _ => CDRF_DODEFAULT,
        }
    }
}

#[cfg(target_os = "windows")]
fn handle_grid_custom_draw(inner: &Rc<RefCell<GridInner>>, lparam: isize) -> u32 {
    // SAFETY: `lparam` is the `NMLVCUSTOMDRAW` pointer from `NM_CUSTOMDRAW`.
    unsafe {
        let p = lparam as *mut NmLvCustomDraw;
        if p.is_null() {
            return CDRF_DODEFAULT;
        }
        let stage = (*p).nmcd.dw_draw_stage;
        match stage {
            CDDS_PREPAINT => CDRF_NOTIFYITEMDRAW,
            CDDS_ITEMPREPAINT => CDRF_NOTIFYSUBITEMDRAW,
            s if s == (CDDS_ITEMPREPAINT | CDDS_SUBITEM) => {
                let row = (*p).nmcd.dw_item_spec;
                let col = (*p).i_sub_item;
                if row == usize::MAX {
                    return CDRF_DODEFAULT;
                }
                let draw = match inner.try_borrow() {
                    Ok(i) => {
                        if i.appearance.system_theme {
                            if ((*p).nmcd.u_item_state & CDIS_SELECTED) != 0
                                || !i.has_explicit_style(row, col as usize)
                            {
                                return CDRF_DODEFAULT;
                            }
                            i.resolve_style(row, col as usize)
                        } else {
                            let appearance = i.appearance;
                            if ((*p).nmcd.u_item_state & CDIS_SELECTED) != 0 {
                                (*p).nmcd.clr_text = appearance.selection_foreground.to_colorref();
                                (*p).nmcd.clr_text_bk =
                                    appearance.selection_background.to_colorref();
                                return CDRF_NEWFONT;
                            }
                            i.resolve_style(row, col as usize)
                        }
                    }
                    Err(_) => return CDRF_DODEFAULT,
                };
                if let Some(fg) = draw.foreground {
                    (*p).nmcd.clr_text = fg.to_colorref();
                }
                if let Some(bg) = draw.background {
                    (*p).nmcd.clr_text_bk = bg.to_colorref();
                }
                if draw.foreground.is_some() || draw.background.is_some() {
                    CDRF_NEWFONT
                } else {
                    CDRF_DODEFAULT
                }
            }
            _ => CDRF_DODEFAULT,
        }
    }
}

#[cfg(target_os = "windows")]
fn win_apply_row_height(grid: &Grid, height: f64) {
    let pt = ((height - 8.0) * 0.72).clamp(8.0, 28.0) as i32;
    let desc = {
        let mut i = grid.inner.borrow_mut();
        i.font_desc.point_size = pt;
        i.font_desc.clone()
    };
    grid.set_font_desc(desc, true);
}

#[cfg(target_os = "windows")]
fn win_move_focused_cell(inner: &Rc<RefCell<GridInner>>, row_delta: i32, col_delta: i32) {
    let (nrows, ncols) = {
        let i = inner.borrow();
        (i.row_count, i.col_count)
    };
    if nrows == 0 || ncols == 0 {
        return;
    }
    let (row, col) = inner.borrow().focused_cell.unwrap_or((0, 0));
    let new_row = (row as i64 + row_delta as i64).clamp(0, (nrows - 1) as i64) as usize;
    let mut new_col = (col as i64 + col_delta as i64).clamp(0, (ncols - 1) as i64) as usize;
    if col_delta != 0 {
        let step = if col_delta > 0 { 1i32 } else { -1i32 };
        let mut c = new_col as i64;
        while c >= 0 && (c as usize) < ncols {
            let idx = c as usize;
            if !inner.borrow().col_hidden.get(idx).copied().unwrap_or(false) {
                new_col = idx;
                break;
            }
            c += step as i64;
            if c < 0 || c as usize >= ncols {
                return;
            }
            new_col = c as usize;
        }
    } else if inner
        .borrow()
        .col_hidden
        .get(new_col)
        .copied()
        .unwrap_or(false)
    {
        return;
    }
    inner.borrow_mut().focused_cell = Some((new_row, new_col));
    let hwnd = inner.borrow().hwnd;
    // SAFETY: select and scroll the focused row.
    unsafe {
        select_display_row(hwnd, new_row);
        SendMessageW(hwnd, LVM_ENSUREVISIBLE, new_row, 0);
    }
}

#[cfg(target_os = "windows")]
fn lv_col_for_data_col(i: &GridInner, data_col: usize) -> usize {
    data_col + i.lv_leading_cols
}

#[cfg(target_os = "windows")]
fn data_col_from_lv_col(i: &GridInner, lv_col: usize) -> Option<usize> {
    if lv_col < i.lv_leading_cols {
        None
    } else {
        Some(lv_col - i.lv_leading_cols)
    }
}

fn fire_selection_callbacks(inner: &Rc<RefCell<GridInner>>) {
    let new_rows = {
        #[cfg(target_os = "windows")]
        {
            win_selected_rows(inner)
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::grid_get_selected_rows(inner.borrow().stub_handle)
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::grid_get_selected_rows(inner.borrow().stub_handle)
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            Vec::new()
        }
    };
    let new_sel = new_rows.first().copied();
    let (single_changed, multi_changed) = {
        let mut i = inner.borrow_mut();
        let sc = i.last_selection != new_sel;
        let mc = i.last_selection_rows != new_rows;
        if sc {
            i.last_selection = new_sel;
        }
        if mc {
            i.last_selection_rows = new_rows.clone();
        }
        (sc, mc)
    };
    if single_changed {
        let cb = inner.borrow_mut().on_sel_change.take();
        if let Some(mut c) = cb {
            c(new_sel);
            inner.borrow_mut().on_sel_change = Some(c);
        }
    }
    if multi_changed {
        let cb = inner.borrow_mut().on_sel_rows_change.take();
        if let Some(mut c) = cb {
            c(&new_rows);
            inner.borrow_mut().on_sel_rows_change = Some(c);
        }
    }
}

#[cfg(target_os = "windows")]
fn win_selected_rows(inner: &Rc<RefCell<GridInner>>) -> Vec<usize> {
    let hwnd = inner.borrow().hwnd;
    let mut rows = Vec::new();
    let mut idx = -1i32;
    loop {
        // SAFETY: valid ListView HWND; `-1` starts the search.
        let next = unsafe {
            SendMessageW(
                hwnd,
                LVM_GETNEXTITEM,
                idx as isize as usize,
                LVNI_SELECTED as isize,
            )
        };
        if next < 0 {
            break;
        }
        rows.push(next as usize);
        idx = next as i32;
    }
    rows
}

fn handle_cell_edited(inner: &Rc<RefCell<GridInner>>, display_row: usize, col: usize, text: &str) {
    let logical = inner
        .borrow()
        .row_perm
        .get(display_row)
        .copied()
        .unwrap_or(display_row);
    if !commit_cell_value_inner(inner, logical, col, text) {
        return;
    }
    let grid = Grid {
        inner: inner.clone(),
    };
    grid.refresh_logical_cell(logical, col);
}

fn commit_cell_value_inner(
    inner: &Rc<RefCell<GridInner>>,
    logical_row: usize,
    col: usize,
    text: &str,
) -> bool {
    {
        let mut i = inner.borrow_mut();
        if let Some(ref mut cb) = i.on_cell_edited {
            if !cb(logical_row, col, text) {
                return false;
            }
        }
        i.cells
            .insert((logical_row, col), Cell::Text(text.to_string()));
    }
    true
}

fn grid_paste_tsv(grid: &Grid, start_display_row: usize, start_col: usize, text: &str) -> usize {
    if start_col >= grid.col_count() {
        return 0;
    }
    let inner = grid.inner.clone();
    let mut rows_written = 0usize;
    for (r_off, line) in text.lines().enumerate() {
        let display_row = start_display_row + r_off;
        if display_row >= grid.row_count() {
            break;
        }
        let logical = grid.logical_row(display_row);
        let cells: Vec<&str> = line.split('\t').collect();
        let mut wrote = false;
        for (c_off, cell_text) in cells.iter().enumerate() {
            let col = start_col + c_off;
            if col >= grid.col_count() || grid.is_column_hidden(col) {
                continue;
            }
            if commit_cell_value_inner(&inner, logical, col, cell_text) {
                grid.refresh_logical_cell(logical, col);
                wrote = true;
            }
        }
        if wrote {
            rows_written += 1;
        }
    }
    rows_written
}

#[cfg(target_os = "windows")]
fn win_update_row_label_cell(inner: &Rc<RefCell<GridInner>>, display_row: usize) {
    let (enabled, hwnd) = {
        let i = inner.borrow();
        (i.row_labels_enabled, i.hwnd)
    };
    if !enabled {
        return;
    }
    let label = (display_row + 1).to_string();
    let wide = to_wide(&label);
    let item = LVITEMW {
        mask: LVIF_TEXT,
        i_item: display_row as i32,
        i_sub_item: 0,
        state: 0,
        state_mask: 0,
        psz_text: wide.as_ptr() as *mut u16,
        cch_text_max: 0,
        i_image: -1,
        l_param: 0,
        i_indent: 0,
        i_group_id: 0,
        c_columns: 0,
        pu_columns: std::ptr::null_mut(),
        pi_col_fmt: std::ptr::null_mut(),
        i_group: 0,
    };
    // SAFETY: synchronous ListView text update.
    unsafe {
        SendMessageW(hwnd, LVM_SETITEM, 0, &item as *const LVITEMW as isize);
    }
}

#[cfg(target_os = "windows")]
fn win_sync_row_label_column(grid: &Grid) {
    let (enabled, width, hwnd, had, dpi) = {
        let i = grid.inner.borrow();
        (
            i.row_labels_enabled,
            i.row_label_width,
            i.hwnd,
            i.lv_leading_cols,
            // SAFETY: valid ListView HWND.
            unsafe { GetDpiForWindow(i.hwnd) },
        )
    };
    if enabled && had == 0 {
        let wide = to_wide("#");
        let physical_w = dpi.scale(width.max(20) as u32) as i32;
        let col = LVCOLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
            fmt: ColumnAlign::Center.as_lvfmt() as i32,
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
        // SAFETY: insert leading row-label column at index 0.
        unsafe {
            SendMessageW(hwnd, LVM_INSERTCOLUMN, 0, &col as *const LVCOLUMNW as isize);
        }
        grid.inner.borrow_mut().lv_leading_cols = 1;
    } else if !enabled && had == 1 {
        // SAFETY: remove leading row-label column.
        unsafe {
            SendMessageW(hwnd, LVM_DELETECOLUMN, 0, 0);
        }
        grid.inner.borrow_mut().lv_leading_cols = 0;
    }
}

#[cfg(target_os = "windows")]
fn win_begin_cell_edit(grid: &Grid, display_row: usize, col: usize) -> bool {
    if !grid.inner.borrow().cell_editing_enabled {
        return false;
    }
    win_end_cell_edit(grid, false);
    let (hwnd, text) = {
        let i = grid.inner.borrow();
        let logical = i.row_perm.get(display_row).copied().unwrap_or(display_row);
        let cell = if let Some(p) = i.provider.as_ref() {
            p(logical, col)
        } else {
            i.cells.get(&(logical, col)).cloned().unwrap_or(Cell::Empty)
        };
        if !cell.has_text() && cell.image().is_none() {
            return false;
        }
        (i.hwnd, cell.text())
    };
    let lv_col = lv_col_for_data_col(&grid.inner.borrow(), col);
    // SAFETY: ListView sub-item rect query.
    let rect = unsafe {
        let mut r = RECT {
            left: lv_col as i32,
            top: LVIR_BOUNDS as i32,
            right: 0,
            bottom: 0,
        };
        let ok = SendMessageW(
            hwnd,
            LVM_GETSUBITEMRECT,
            display_row,
            &mut r as *mut RECT as isize,
        );
        if ok != 0 {
            r
        } else {
            let row_h = 24;
            RECT {
                left: 0,
                top: display_row as i32 * row_h,
                right: 200,
                bottom: (display_row as i32 + 1) * row_h,
            }
        }
    };
    // SAFETY: create child EDIT over the cell.
    let editor_hwnd = unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let wide_text = to_wide(&text);
        let class_name = to_wide("EDIT");
        let hwnd_edit = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
            rect.left,
            rect.top,
            (rect.right - rect.left).max(40),
            (rect.bottom - rect.top).max(16),
            hwnd,
            next_control_id() as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );
        SendMessageW(hwnd_edit, EM_SETSEL, 0, -1);
        SetFocus(hwnd_edit);
        hwnd_edit
    };
    if editor_hwnd.is_null() {
        return false;
    }
    grid.inner.borrow_mut().active_editor = Some(WinGridEditor {
        hwnd: editor_hwnd,
        display_row,
        col,
    });
    true
}

#[cfg(target_os = "windows")]
fn win_end_cell_edit(grid: &Grid, accept: bool) -> bool {
    let editor = grid.inner.borrow_mut().active_editor.take();
    let Some(editor) = editor else {
        return false;
    };
    if accept {
        let mut buf = vec![0u16; 512];
        // SAFETY: read committed editor text before destroy.
        let len = unsafe { GetWindowTextW(editor.hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        buf.truncate(len.max(0) as usize);
        let text = String::from_utf16_lossy(&buf);
        handle_cell_edited(&grid.inner, editor.display_row, editor.col, &text);
    }
    // SAFETY: editor is a child window owned by the grid.
    unsafe {
        DestroyWindow(editor.hwnd);
    }
    true
}

#[cfg(target_os = "windows")]
const LVM_GETSUBITEMRECT: u32 = LVM_FIRST + 56;
#[cfg(target_os = "windows")]
const LVIR_BOUNDS: u32 = 0;

#[cfg(target_os = "windows")]
unsafe extern "system" fn grid_list_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_LBUTTONDOWN {
        let inner = GRID_CONTEXT_GRIDS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
        if let Some(inner) = inner {
            let x = (lparam as i32 & 0xFFFF) as i16 as i32;
            let y = ((lparam as u32 >> 16) & 0xFFFF) as i16 as i32;
            // SAFETY: client-area hit test on a live ListView HWND.
            let (row, lv_col) = unsafe {
                let mut info = LvHitTestInfo {
                    pt: POINT { x, y },
                    flags: 0,
                    i_item: -1,
                    i_sub_item: 0,
                };
                let _ = SendMessageW(
                    hwnd,
                    LVM_SUBITEMHITTEST,
                    0,
                    &mut info as *mut LvHitTestInfo as isize,
                );
                let lv_col = if info.i_sub_item >= 0 {
                    info.i_sub_item as usize
                } else {
                    0
                };
                (info.i_item, lv_col)
            };
            if row >= 0 {
                if let Some(data_col) = data_col_from_lv_col(&inner.borrow(), lv_col) {
                    inner.borrow_mut().focused_cell = Some((row as usize, data_col));
                    if inner.borrow().link_clicks_enabled {
                        let grid = Grid {
                            inner: inner.clone(),
                        };
                        let _ = grid.open_link_at(row as usize, data_col);
                    }
                }
            }
            if inner
                .try_borrow()
                .ok()
                .is_some_and(|i| i.row_col_highlight_enabled)
            {
                let col = lv_col;
                if row >= 0 && col == 0 {
                    apply_reference_highlight(&inner, Some(row as usize), None);
                }
            }
        }
    }
    if msg == WM_KEYDOWN {
        let inner = GRID_CONTEXT_GRIDS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
        if let Some(inner) = inner {
            let key = wparam as u32;
            let ctrl = unsafe { (GetAsyncKeyState(VK_CONTROL as i32) as u16) & 0x8000 != 0 };
            if key == VK_RETURN as u32 {
                // Enter activation is handled by LVN_ITEMACTIVATE when hooks are enabled.
            } else if key == VK_F2 as u32 && inner.borrow().cell_editing_enabled {
                let grid = Grid {
                    inner: inner.clone(),
                };
                let _ = grid.begin_cell_edit();
                return 0;
            } else if ctrl && key == VK_V as u32 {
                let grid = Grid {
                    inner: inner.clone(),
                };
                let _ = grid.paste_at_focus();
                return 0;
            } else if key == VK_LEFT as u32 {
                win_move_focused_cell(&inner, 0, -1);
                return 0;
            } else if key == VK_RIGHT as u32 {
                win_move_focused_cell(&inner, 0, 1);
                return 0;
            } else if key == VK_UP as u32 {
                win_move_focused_cell(&inner, -1, 0);
                return 0;
            } else if key == VK_DOWN as u32 {
                win_move_focused_cell(&inner, 1, 0);
                return 0;
            } else if key == VK_TAB as u32 {
                let shift = unsafe { (GetAsyncKeyState(VK_SHIFT as i32) as u16) & 0x8000 != 0 };
                win_move_focused_cell(&inner, 0, if shift { -1 } else { 1 });
                return 0;
            }
        }
    }
    if msg == WM_NOTIFY {
        let inner = GRID_CONTEXT_GRIDS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
        if let Some(inner) = inner {
            let nm = lparam as *const NmHdr;
            if !nm.is_null() {
                let header = inner.try_borrow().ok().map(|i| i.header_hwnd);
                if let Some(header) = header {
                    if !header.is_null() && (*nm).hwnd_from == header {
                        if (*nm).code == NM_CUSTOMDRAW {
                            return handle_header_custom_draw(&inner, lparam) as LRESULT;
                        }
                        if (*nm).code == HDN_ITEMCLICKW {
                            let col = {
                                let p = lparam as *const NMHEADERW;
                                if p.is_null() {
                                    -1
                                } else {
                                    (*p).iItem
                                }
                            };
                            if col >= 0 {
                                handle_header_item_click(&inner, col as usize);
                                apply_reference_highlight(&inner, None, Some(col as usize));
                            }
                            return 0;
                        }
                        if (*nm).code == HDN_DIVIDERDBLCLICKW {
                            let col = {
                                let p = lparam as *const NMHEADERW;
                                if p.is_null() {
                                    -1
                                } else {
                                    (*p).iItem
                                }
                            };
                            if col >= 0 {
                                handle_header_divider_dblclick(&inner, col as usize);
                            }
                            return 0;
                        }
                    }
                }
            }
        }
    }
    if msg == WM_CONTEXTMENU {
        let inner = GRID_CONTEXT_GRIDS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
        if let Some(inner) = inner {
            let (x, y) = if lparam == -1 {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                (pt.x, pt.y)
            } else {
                ((lparam as i32 & 0xFFFF), ((lparam as i32 >> 16) & 0xFFFF))
            };
            open_grid_context_menu(hwnd, &inner, x, y);
            return 0;
        }
    }
    let orig = GRID_LIST_ORIG_PROCS.with(|m| m.borrow().get(&(hwnd as isize)).copied());
    if let Some(proc) = orig {
        return CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn grid_header_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CONTEXTMENU {
        let inner = GRID_CONTEXT_GRIDS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
        if let Some(inner) = inner {
            let (x, y) = if lparam == -1 {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                (pt.x, pt.y)
            } else {
                ((lparam as i32 & 0xFFFF), ((lparam as i32 >> 16) & 0xFFFF))
            };
            open_grid_context_menu(hwnd, &inner, x, y);
            return 0;
        }
    }
    let orig = GRID_HEADER_ORIG_PROCS.with(|m| m.borrow().get(&(hwnd as isize)).copied());
    if let Some(proc) = orig {
        return CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(target_os = "windows")]
fn win_sync_listview_row_count(grid: &Grid, count: usize) {
    let hwnd = grid.inner.borrow().hwnd;
    // SAFETY: Win32 ListView row resize via item insert/delete.
    unsafe {
        let current = SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0) as usize;
        if count > current {
            for i in current..count {
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
        } else if count < current {
            for i in (count..current).rev() {
                SendMessageW(hwnd, LVM_DELETEITEM, i, 0);
            }
        }
        let frozen = grid.inner.borrow().frozen_hwnd;
        if !frozen.is_null() && frozen != hwnd {
            win_sync_listview_row_count_hwnd(frozen, count);
        }
    }
}

#[cfg(target_os = "windows")]
fn win_sync_listview_row_count_hwnd(hwnd: HWND, count: usize) {
    unsafe {
        let current = SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0) as usize;
        if count > current {
            for i in current..count {
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
        } else if count < current {
            for i in (count..current).rev() {
                SendMessageW(hwnd, LVM_DELETEITEM, i, 0);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn win_rebuild_frozen_columns(grid: &Grid) {
    let count = grid.inner.borrow().frozen_col_count;
    let hwnd = grid.inner.borrow().hwnd;
    if count == 0 {
        let frozen = grid.inner.borrow().frozen_hwnd;
        if !frozen.is_null() && frozen != hwnd {
            unsafe {
                DestroyWindow(frozen);
            }
            grid.inner.borrow_mut().frozen_hwnd = hwnd;
        }
        return;
    }
    let mut inner = grid.inner.borrow_mut();
    if inner.frozen_hwnd.is_null() || inner.frozen_hwnd == inner.hwnd {
        let parent = unsafe { GetParent(inner.hwnd) };
        let id = inner.id.wrapping_add(0x4000);
        let frozen = unsafe {
            let wide_class = to_wide("SysListView32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | LVS_REPORT,
                0,
                0,
                120,
                inner.rect.height as i32,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        inner.frozen_hwnd = frozen;
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// A2: `row_count`/`col_count` rispecchiano la configurazione e
    /// `selected_rows` legge la selezione nativa della list-view
    /// (senza inspector).
    #[test]
    fn grid_count_and_selection_getters_read_native_state() {
        let frame = Frame::builder()
            .with_title("grid-getter")
            .with_size(380, 240)
            .build();
        let grid = Grid::new(&frame);
        grid.append_column("Nome", 140);
        grid.append_column("Valore", 120);
        grid.set_row_count(3);
        assert_eq!(grid.col_count(), 2);
        assert_eq!(grid.row_count(), 3);

        grid.select_row(1);
        assert_eq!(grid.selected_rows(), vec![1usize]);
    }

    #[test]
    fn number_format_integer_omits_fraction() {
        assert_eq!(NumberFormat::Integer.render(1234.0), "1,234");
    }

    #[test]
    fn number_format_fixed2_always_shows_two_decimals() {
        assert_eq!(NumberFormat::Fixed2.render(12.3), "12.30");
    }

    #[test]
    fn number_format_percent_suffix() {
        assert_eq!(NumberFormat::Percent.render(12.3), "12.30%");
    }

    #[test]
    fn cell_multiline_converts_newlines_for_listview() {
        let cell = Cell::MultiLine("line1\nline2".to_string());
        assert_eq!(cell.text(), "line1\rline2");
    }

    #[test]
    fn cell_empty_has_no_text() {
        assert!(!Cell::Empty.has_text());
        assert!(Cell::Text("x".into()).has_text());
    }

    #[test]
    fn grid_date_format_iso_renders() {
        assert_eq!(GridDateFormat::Iso.render("2025-11-07"), "2025-11-07");
    }

    #[test]
    fn compare_cells_typed_number_and_date() {
        use std::cmp::Ordering;
        let a = Cell::Number {
            value: 599.99,
            format: NumberFormat::CurrencyEuro,
        };
        let b = Cell::Number {
            value: 49.0,
            format: NumberFormat::Plain,
        };
        assert_eq!(compare_cells_typed(&a, &b), Ordering::Greater);
        let d1 = Cell::DateTime {
            iso: "2024-03-15".into(),
            format: GridDateFormat::Iso,
        };
        let d2 = Cell::DateTime {
            iso: "2023-09-01".into(),
            format: GridDateFormat::Long,
        };
        assert_eq!(compare_cells_typed(&d1, &d2), Ordering::Greater);
        let p1 = Cell::Priority {
            kind: PriorityKind::Critical,
        };
        let p2 = Cell::Priority {
            kind: PriorityKind::Low,
        };
        assert_eq!(compare_cells_typed(&p1, &p2), Ordering::Greater);
    }

    #[test]
    fn compare_cells_for_sort_numeric() {
        use std::cmp::Ordering;
        assert_eq!(compare_cells_for_sort("1,234", "9,999"), Ordering::Less);
        assert_eq!(
            compare_cells_for_sort("€ 10.00", "€ 2.00"),
            Ordering::Greater
        );
    }

    #[test]
    fn grid_appearance_modern_has_blue_header() {
        let m = GridAppearance::modern();
        assert_eq!(m.header_background.r, 30);
        assert_eq!(m.header_background.g, 64);
        assert_eq!(m.header_background.b, 175);
    }

    #[test]
    fn compare_cells_for_sort_lexical() {
        use std::cmp::Ordering;
        assert_eq!(compare_cells_for_sort("Beta", "alpha"), Ordering::Greater);
    }

    #[test]
    fn lvm_wide_message_ids_are_pinned() {
        #[cfg(target_os = "windows")]
        {
            assert_eq!(LVM_INSERTCOLUMN, LVM_FIRST + 97);
            assert_eq!(LVM_INSERTITEM, LVM_FIRST + 77);
            assert_eq!(LVM_SETITEM, LVM_FIRST + 76);
            assert_eq!(LVM_GETITEM, LVM_FIRST + 75);
            assert_eq!(LVN_ITEMCHANGED, 0xFFFFFF9B);
        }
    }
}
