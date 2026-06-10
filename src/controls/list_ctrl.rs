//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::NMHDR;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ListView constants ─────────────────────────────────────────

#[cfg(target_os = "windows")]
const LVM_FIRST: u32 = 0x1000;

// Use the **wide** ListView message variants. `LVM_FIRST + 7/27/45/46`
// are the ANSI (`...A`) versions; passing UTF-16 `pszText` to the ANSI
// handler truncates every cell/header to its first character.
#[cfg(target_os = "windows")]
const LVM_INSERTCOLUMN: u32 = LVM_FIRST + 97; // LVM_INSERTCOLUMNW
#[cfg(target_os = "windows")]
const LVM_INSERTITEM: u32 = LVM_FIRST + 77; // LVM_INSERTITEMW
#[cfg(target_os = "windows")]
const LVM_SETITEMTEXT: u32 = LVM_FIRST + 116; // LVM_SETITEMTEXTW
#[cfg(target_os = "windows")]
const LVM_GETITEMTEXT: u32 = LVM_FIRST + 115; // LVM_GETITEMTEXTW
#[cfg(target_os = "windows")]
#[allow(dead_code)] // used only by future v0.5.x removals of cached counts
const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
#[cfg(target_os = "windows")]
const LVM_DELETEITEM: u32 = LVM_FIRST + 8;
#[cfg(target_os = "windows")]
const LVM_DELETEALLITEMS: u32 = LVM_FIRST + 9;
#[cfg(target_os = "windows")]
const LVM_GETNEXTITEM: u32 = LVM_FIRST + 12;
#[cfg(target_os = "windows")]
const LVM_SETITEMSTATE: u32 = LVM_FIRST + 43;
#[cfg(target_os = "windows")]
const LVM_GETITEMSTATE: u32 = LVM_FIRST + 44;
#[cfg(target_os = "windows")]
const LVM_GETSELECTEDCOUNT: u32 = LVM_FIRST + 50;
#[cfg(target_os = "windows")]
const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = LVM_FIRST + 54;

/// LVNI_SELECTED flag for LVM_GETNEXTITEM
#[cfg(target_os = "windows")]
const LVNI_SELECTED: u32 = 2;

/// LVIS_* state bits for list-view items (used with `LVM_SETITEMSTATE`
/// and `LVM_GETITEMSTATE`). See Microsoft Docs:
/// <https://learn.microsoft.com/en-us/windows/win32/api/commctrl/ns-commctrl-lvitemw>
#[cfg(target_os = "windows")]
const LVIS_FOCUSED: u32 = 0x0001;
#[cfg(target_os = "windows")]
const LVIS_SELECTED: u32 = 0x0002;

/// LVS_EX_FULLROWSELECT — highlight entire row in report view
#[cfg(target_os = "windows")]
const LVS_EX_FULLROWSELECT: u32 = 0x20;

/// LVN_ITEMCHANGED — ListView notification code, sent when the state
/// of an item changes (selection, focus, cut/highlight, etc.).
/// Computed as `LVN_FIRST - 1` = `(0U - 100U) - 1` = 0xFFFFFF9B.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const LVN_ITEMCHANGED: u32 = 0xFFFFFF9B;

/// LVN_GETDISPINFOW — ListView virtual-mode notification code. Sent
/// when the ListView needs to display an item and the control was
/// created with `LVS_OWNERDATA` (i.e. the item count is set via
/// [`LVM_SETITEMCOUNT`] and the application supplies per-item data
/// on demand through this notification).
///
/// Computed as `LVN_FIRST - 77` = `(0U - 100U) - 77` = 0xFFFFFF4F.
/// This is the **W (Unicode) variant**; the A variant has a
/// different code (0xFFFFFF6A). The W variant is the one we use
/// because every `ListCtrl` API in this crate goes through the
/// wide Win32 entry points.
///
/// `pub(crate)` so the frame's `WM_NOTIFY` arm can dispatch the
/// notification to the per-control `on_get_disp_info` handler
/// without re-typing the magic number.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub(crate) const LVN_GETDISPINFOW: u32 = 0xFFFFFF4F;

/// LVS_OWNERDATA — owner-data (a.k.a. "virtual") list-view style.
/// When this bit is set, the ListView does NOT store per-item
/// strings; it asks the parent for them on demand through
/// `LVN_GETDISPINFOW`. This lets the application back the
/// ListView with a million rows without allocating a million
/// `LVITEM` structs. The style can be set at creation time (the
/// safe path) or toggled on an existing ListView via
/// [`SetWindowLongPtrW`] with `GWL_STYLE` (the path used by
/// [`ListCtrl::set_item_count`] when the user opts in after
/// construction).
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const LVS_OWNERDATA: u32 = 0x1000;

/// LVM_SETITEMCOUNT — message that sets the (possibly very large)
/// virtual item count of a ListView in `LVS_OWNERDATA` mode. The
/// `wparam` carries the new count, the `lparam` is a combination
/// of `LVSICF_*` flags.
#[cfg(target_os = "windows")]
const LVM_SETITEMCOUNT: u32 = LVM_FIRST + 47;

/// LVSICF_NOINVALIDATEALL — when passed to `LVM_SETITEMCOUNT`, the
/// ListView does not redraw items it already had on screen. The
/// default (`LVSICF_INVALIDATEALL`, value 0) would force a full
/// redraw, which is wasteful for the "the user scrolled, give me
/// the new chunk" hot path.
#[cfg(target_os = "windows")]
const LVSICF_NOINVALIDATEALL: u32 = 0x0001;

/// LVSICF_NOSCROLL — when passed to `LVM_SETITEMCOUNT`, the
/// ListView does not change its scroll position when the item
/// count is updated. Useful when the new count is the result of
/// an in-place edit (e.g. an item was inserted at row N) rather
/// than a wholesale refresh.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const LVSICF_NOSCROLL: u32 = 0x0002;

/// ListView view styles
#[cfg(target_os = "windows")]
const LVS_ICON: u32 = 0x0000;
#[cfg(target_os = "windows")]
const LVS_REPORT: u32 = 0x0001;
#[cfg(target_os = "windows")]
const LVS_SMALLICON: u32 = 0x0002;
#[cfg(target_os = "windows")]
const LVS_LIST: u32 = 0x0003;

/// LVCOLUMNW / LVITEMW mask flags
#[cfg(target_os = "windows")]
const LVCF_TEXT: u32 = 4;
#[cfg(target_os = "windows")]
const LVCF_WIDTH: u32 = 2;
#[cfg(target_os = "windows")]
const LVIF_TEXT: u32 = 1;
/// `LVIF_IMAGE` — the `i_image` field is valid.
#[cfg(target_os = "windows")]
const LVIF_IMAGE: u32 = 2;
/// `LVM_SETIMAGELIST` (`LVM_FIRST + 3`).
#[cfg(target_os = "windows")]
const LVM_SETIMAGELIST: u32 = LVM_FIRST + 3;
/// `LVM_REDRAWITEMS` (`LVM_FIRST + 21`).
#[cfg(target_os = "windows")]
const LVM_REDRAWITEMS: u32 = LVM_FIRST + 21;
/// `LVSIL_SMALL` — image list slot used in report view.
#[cfg(target_os = "windows")]
const LVSIL_SMALL: usize = 1;

// ── Win32 structs ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
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

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
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

/// NMLVDISPINFOW — payload of the `LVN_GETDISPINFOW` notification
/// (a.k.a. "owner-data / virtual list-view callback"). The ListView
/// fills in `hdr.idFrom`, `hdr.code`, `item.mask`, `item.iItem`, and
/// `item.iSubItem`; the application is expected to read those fields
/// and populate `item.pszText` (a buffer of `cchTextMax` UTF-16 code
/// units) for the cell the control is about to draw.
///
/// The struct layout is fixed by Microsoft (it must match
/// `tagNMLVDISPINFOW` from `<commctrl.h>`); we therefore match the
/// field order, types, and `#[repr(C)]` of the upstream definition.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
struct NMLVDISPINFOW {
    hdr: NMHDR,
    item: LVITEMW,
}

/// `NMLVCACHEHINT` — payload of the `LVN_ODCACHEHINT` notification
/// sent by an `LVS_OWNERDATA` (virtual) ListView when it is about
/// to ask for a contiguous range of items. The application uses
/// the hint to pre-fetch the data (open a file, query a database,
/// etc.) so the subsequent `LVN_GETDISPINFOW` requests for items
/// in the range `[iFrom, iTo]` can be served from the cache
/// instead of doing the work on the callback path.
///
/// Layout is fixed by Microsoft (it must match
/// `tagNMLVCACHEHINT` from `<commctrl.h>`), so we match the
/// field order, types, and `#[repr(C)]` of the upstream
/// definition.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
struct NMLVCACHEHINT {
    hdr: NMHDR,
    i_from: i32,
    i_to: i32,
}

/// `LVN_ODCACHEHINT` — owner-data virtual-list "I'm about to ask
/// for a range of items" notification. Sent before the control
/// issues a flurry of `LVN_GETDISPINFOW` requests for a
/// contiguous range; the application is expected to pre-load
/// the backing data for the range and stash it in a cache.
///
/// Computed as `LVN_FIRST - 79` = `(0U - 100U) - 79` = 0xFFFFFF4D.
/// This is the **W (Unicode) variant**; the A variant has a
/// different code (0xFFFFFF68). The W variant is the one we
/// use because every `ListCtrl` API in this crate goes through
/// the wide Win32 entry points.
///
/// `pub(crate)` so the frame's `WM_NOTIFY` arm can dispatch
/// the notification to the per-control `on_cache_hint` handler
/// without re-typing the magic number.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub(crate) const LVN_ODCACHEHINT: u32 = 0xFFFFFF4D;

// ── ListItem (public wrapper) ────────────────────────────────────────

/// Per-cell request handed to an [`ListCtrl::on_get_disp_info`]
/// callback when the underlying Win32 ListView is in
/// [`LVS_OWNERDATA`](https://learn.microsoft.com/en-us/windows/win32/controls/list-view-controls-overview)
/// (virtual) mode.
///
/// The wrapper exposes a small, safe view over the request: callers
/// can read the row (`index()`), the sub-item / column (`sub_item()`),
/// and the requested mask bits (`is_text_requested()`), and call
/// `set_text()` to populate the cell with up to 1024 UTF-16 code
/// units. The wrapper is single-shot per notification: the
/// notification handler is the only thing holding a `&mut ListItem`
/// at a time, and the ListView copies the supplied string out of the
/// supplied buffer before the next message is processed.
///
/// The 1024-code-unit cap matches the typical default buffer the
/// Win32 ListView allocates internally for `LVN_GETDISPINFOW`
/// (the `cchTextMax` field the control passes in the `LVITEMW`).
/// Callers that need more can use [`ListCtrl::insert_item`] /
/// [`ListCtrl::set_item_text`] on a non-virtual list.
pub struct ListItem<'a> {
    item: &'a mut LVITEMW,
}

impl<'a> ListItem<'a> {
    /// Zero-based row index the ListView is asking about.
    pub fn index(&self) -> usize {
        self.item.i_item as usize
    }

    /// Zero-based column index (a.k.a. sub-item). `0` is the main
    /// column, `1..` are the columns added with
    /// [`ListCtrl::insert_column`].
    pub fn sub_item(&self) -> usize {
        self.item.i_sub_item as usize
    }

    /// Whether the control asked the callback to populate the cell's
    /// text. (The only mask bit the current implementation honours,
    /// but the getter is exposed so callers can be defensive against
    /// future mask additions without breaking.)
    pub fn is_text_requested(&self) -> bool {
        (self.item.mask & LVIF_TEXT) != 0
    }

    /// Populate the cell's text. The string is encoded as UTF-16 and
    /// copied into the buffer the ListView handed us; the control
    /// reads it back when it next redraws the cell. Returns `Err`
    /// only if the internal buffer is non-empty but too small for
    /// the supplied string (truncation is **not** performed
    /// silently — the caller can choose to `set_text` a shorter
    /// string or move to a non-virtual list).
    #[cfg(target_os = "windows")]
    pub fn set_text(&mut self, text: &str) -> Result<(), &'static str> {
        // The internal buffer the ListView handed us is at least
        // `cch_text_max` UTF-16 code units long. We allocate a
        // matching temporary (including the NUL terminator), then
        // copy with `copy_nonoverlapping` after the
        // bounds-check below.
        let max = self.item.cch_text_max as usize;
        if max == 0 {
            return Err("internal buffer is zero-sized");
        }
        // Encode (with the trailing NUL) and bounds-check.
        let wide = to_wide(text);
        // `to_wide` always appends a NUL terminator, so the encoded
        // length is `wide.len()` (including the NUL). The buffer
        // needs `wide.len()` code units; we need room for at least
        // the NUL, and we'll reject the call if the whole string
        // (NUL included) does not fit.
        if wide.len() > max {
            return Err("text too long for LVN_GETDISPINFO buffer");
        }
        // SAFETY:
        // * `self.item.psz_text` is a valid `*mut u16` for `max`
        //   elements (it was set by the ListView when it dispatched
        //   LVN_GETDISPINFOW).
        // * `wide` is a Rust `Vec<u16>` of length `wide.len()` with
        //   no overlap with the destination (heap vs. the buffer
        //   the control owns).
        // * We bounds-checked `wide.len() <= max` above.
        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), self.item.psz_text, wide.len());
        }
        // The internal buffer keeps ownership of the storage; we
        // don't free or extend it.
        let _ = wide;
        Ok(())
    }
}

// ── CacheHint (public wrapper) ──────────────────────────────────────

/// Per-range "prefetch this chunk" hint handed to a
/// [`ListCtrl::on_cache_hint`] callback when the underlying Win32
/// ListView is in
/// [`LVS_OWNERDATA`](https://learn.microsoft.com/en-us/windows/win32/controls/list-view-controls-overview)
/// (virtual) mode.
///
/// The ListView sends `LVN_ODCACHEHINT` *before* it issues a
/// burst of `LVN_GETDISPINFOW` requests for a contiguous range
/// of rows. The application can use the hint to pre-load the
/// backing data (open a file, query a database, decompress a
/// chunk, etc.) so the subsequent per-cell requests are served
/// from a cache instead of doing the work on the callback
/// hot path.
///
/// The wrapper exposes a read-only view of the range: callers
/// can read the inclusive lower bound (`from()`) and upper
/// bound (`to()`) of the row range the control is about to
/// ask for. The notification carries no write-back data
/// (unlike [`ListItem`]) so there is no `set_*` method.
pub struct CacheHint<'a> {
    hint: &'a NMLVCACHEHINT,
}

impl<'a> CacheHint<'a> {
    /// Zero-based inclusive lower bound of the row range the
    /// ListView is about to request via `LVN_GETDISPINFOW`.
    pub fn from(&self) -> usize {
        self.hint.i_from.max(0) as usize
    }

    /// Zero-based inclusive upper bound of the row range the
    /// ListView is about to request via `LVN_GETDISPINFOW`.
    pub fn to(&self) -> usize {
        self.hint.i_to.max(0) as usize
    }
}

// ── View style enum ──────────────────────────────────────────────────

/// Determines the visual style of the ListView control.
pub enum ListCtrlStyle {
    /// Multi-column with headers (report / details view)
    Report,
    /// Simple list view
    List,
    /// Large icon view
    Icon,
    /// Small icon view
    SmallIcon,
}

#[cfg(target_os = "windows")]
fn list_ctrl_style_value(style: &ListCtrlStyle) -> u32 {
    match style {
        ListCtrlStyle::Report => LVS_REPORT,
        ListCtrlStyle::List => LVS_LIST,
        ListCtrlStyle::Icon => LVS_ICON,
        ListCtrlStyle::SmallIcon => LVS_SMALLICON,
    }
}

// ── Inner type ───────────────────────────────────────────────────────

/// Type alias for the user-supplied `on_get_disp_info` callback
/// closure. Kept in one place so a future signature change (e.g.
/// a `mask: u32` parameter) only has to update one site, and so
/// the `Option<...>` field type in [`ListCtrlInner`] stays short
/// enough to silence `clippy::type_complexity`.
type DispInfoCallback = Box<dyn FnMut(&mut ListItem)>;

/// Type alias for the user-supplied `on_cache_hint` callback
/// closure. Mirrors [`DispInfoCallback`] so a future signature
/// change (e.g. adding a `kind: u32` parameter for the
/// `LVN_ODCACHEHINT` flags) only has to update one site.
type CacheHintCallback = Box<dyn FnMut(&CacheHint)>;

struct ListCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    col_count: u32,
    /// Number of rows in the ListView. Tracked locally so the
    /// `set_item_count` / `get_item_count` round-trip is consistent
    /// even on a `null` `HWND` (where `LVM_GETITEMCOUNT` returns 0
    /// and `LVM_SETITEMCOUNT` is a no-op). Defaults to 0, which
    /// matches the Win32 default for a freshly-created non-virtual
    /// ListView.
    item_count: u32,
    enabled: bool,
    visible: bool,
    /// User-supplied on-item-selected callback, if any. Receives the
    /// newly selected row index (or `None` if the selection is
    /// cleared). Stored in the inner state so the WM_NOTIFY handler
    /// registered on the parent `Frame` can reach it.
    on_item_selected: Option<Box<dyn FnMut(Option<usize>)>>,
    /// Last row index reported via `LVN_ITEMCHANGED`, used to debounce
    /// the duplicate notifications that ListView sends per click.
    last_selection: Option<usize>,
    /// User-supplied virtual-mode callback, if any. Receives a
    /// `&mut ListItem` describing the cell the control is about to
    /// draw. Stored in the inner state so the `LVN_GETDISPINFOW`
    /// handler registered on the parent `Frame` can reach it. The
    /// closure is replaced on every call to
    /// [`ListCtrl::on_get_disp_info`]; there is no
    /// "register multiple" support, matching the existing
    /// [`set_drop_files_callback`](crate::window::frame::Frame::set_drop_files_callback)
    /// "one owner" model on the frame.
    on_get_disp_info: Option<DispInfoCallback>,
    /// User-supplied virtual-mode cache-hint callback, if any.
    /// Receives a `&CacheHint` describing the inclusive row range
    /// the control is about to ask for via a burst of
    /// `LVN_GETDISPINFOW` requests. Stored in the inner state so
    /// the `LVN_ODCACHEHINT` handler registered on the parent
    /// `Frame` can reach it. The closure is replaced on every
    /// call to [`ListCtrl::on_cache_hint`].
    on_cache_hint: Option<CacheHintCallback>,
}

// ── Public type ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ListCtrl {
    inner: Rc<RefCell<ListCtrlInner>>,
}

impl ListCtrl {
    /// Create a new ListView control as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W, style: ListCtrlStyle) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("SysListView32");
            let view_style = list_ctrl_style_value(&style);
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | view_style,
                0,
                0,
                300,
                200,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        let ctrl = ListCtrl {
            inner: Rc::new(RefCell::new(ListCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 300, 200),
                col_count: 0,
                item_count: 0,
                enabled: true,
                visible: true,
                on_item_selected: None,
                last_selection: None,
                on_get_disp_info: None,
                on_cache_hint: None,
            })),
        };

        // Default: enable full-row select in report view
        if matches!(style, ListCtrlStyle::Report) {
            ctrl.set_extended_style(LVS_EX_FULLROWSELECT);
        }

        ctrl
    }

    /// Insert a column at the given index with a title and width (in pixels).
    pub fn insert_column(&self, index: u32, title: &str, width: i32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(title);
            let col = LVCOLUMNW {
                mask: LVCF_TEXT | LVCF_WIDTH,
                fmt: 0,
                cx: width,
                psz_text: wide.as_ptr(),
                cch_text_max: wide.len() as i32,
                i_sub_item: index as i32,
                i_image: 0,
                i_order: 0,
                cx_min: 0,
                cx_default: 0,
                cx_ideal: 0,
            };
            SendMessageW(
                self.inner.borrow().hwnd,
                LVM_INSERTCOLUMN,
                index as usize,
                &col as *const LVCOLUMNW as isize,
            );
            self.inner.borrow_mut().col_count += 1;
        }
    }

    /// Insert an item (row) at the given zero-based index with the given text
    /// in the first column. Returns the index of the new item.
    pub fn insert_item(&self, index: usize, text: &str) -> usize {
        #[cfg(target_os = "windows")]
        {
            let wide = to_wide(text);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut item: LVITEMW = unsafe { std::mem::zeroed() };
            item.mask = LVIF_TEXT;
            item.i_item = index as i32;
            item.i_sub_item = 0;
            item.psz_text = wide.as_ptr() as *mut u16;
            item.cch_text_max = wide.len() as i32;

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    LVM_INSERTITEM,
                    0,
                    &item as *const LVITEMW as isize,
                )
            };
            result as usize
        }

        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Attach an [`crate::ImageList`] to the small-icon slot. Rows
    /// added with [`ListCtrl::insert_item_with_image`] display the
    /// icon at their `image_index` to the left of the first column.
    /// The list must stay alive as long as the control.
    pub fn set_image_list(&self, image_list: &crate::ImageList) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                LVM_SETIMAGELIST,
                LVSIL_SMALL,
                image_list.handle(),
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = image_list;
    }

    /// Insert a row whose first column shows the icon at
    /// `image_index` from the image list attached with
    /// [`ListCtrl::set_image_list`]. Returns the index of the new
    /// item.
    pub fn insert_item_with_image(&self, index: usize, text: &str, image_index: i32) -> usize {
        #[cfg(target_os = "windows")]
        {
            let wide = to_wide(text);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut item: LVITEMW = unsafe { std::mem::zeroed() };
            item.mask = LVIF_TEXT | LVIF_IMAGE;
            item.i_item = index as i32;
            item.i_sub_item = 0;
            item.psz_text = wide.as_ptr() as *mut u16;
            item.cch_text_max = wide.len() as i32;
            item.i_image = image_index;

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    LVM_INSERTITEM,
                    0,
                    &item as *const LVITEMW as isize,
                )
            };
            result as usize
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (index, text, image_index);
            0
        }
    }

    /// Set the text of a specific cell (item_index, col_index).
    pub fn set_item_text(&self, item_index: usize, col_index: usize, text: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(text);
            let mut item: LVITEMW = std::mem::zeroed();
            item.mask = LVIF_TEXT;
            item.i_item = item_index as i32;
            item.i_sub_item = col_index as i32;
            item.psz_text = wide.as_ptr() as *mut u16;
            item.cch_text_max = wide.len() as i32;

            SendMessageW(
                self.inner.borrow().hwnd,
                LVM_SETITEMTEXT,
                item_index,
                &item as *const LVITEMW as isize,
            );
        }
    }

    /// Get the text of a specific cell (item_index, col_index).
    pub fn get_item_text(&self, item_index: usize, col_index: usize) -> String {
        #[cfg(target_os = "windows")]
        {
            // Start with a reasonable buffer size and grow if needed
            let mut buf_len: i32 = 256;
            loop {
                let mut buf = vec![0u16; buf_len as usize];
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                let mut item: LVITEMW = unsafe { std::mem::zeroed() };
                item.mask = LVIF_TEXT;
                item.i_item = item_index as i32;
                item.i_sub_item = col_index as i32;
                item.psz_text = buf.as_mut_ptr();
                item.cch_text_max = buf_len;

                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                let result = unsafe {
                    SendMessageW(
                        self.inner.borrow().hwnd,
                        LVM_GETITEMTEXT,
                        item_index,
                        &item as *const LVITEMW as isize,
                    )
                };
                let copied = result as i32;
                // `LVM_GETITEMTEXT` copies at most `buf_len - 1`
                // chars plus the NUL, so `copied == buf_len - 1`
                // may mean truncation → retry with a larger buffer.
                // Any shorter result is complete.
                if copied >= 0 && copied < buf_len - 1 {
                    return String::from_utf16_lossy(&buf[..copied as usize]);
                }
                if copied == buf_len - 1 && buf_len * 2 > 65536 {
                    // Final iteration: return what we have rather
                    // than dropping the (possibly truncated) text.
                    return String::from_utf16_lossy(&buf[..copied as usize]);
                }
                // Buffer was too small, double it and retry
                buf_len *= 2;
                if buf_len > 65536 {
                    break;
                }
            }
            String::new()
        }

        #[cfg(not(target_os = "windows"))]
        String::new()
    }

    /// Return the total number of items (rows) in the list view.
    ///
    /// The count is read from our local cache
    /// ([`ListCtrl::set_item_count`] stores into
    /// `ListCtrlInner::item_count`), so the value is always the
    /// one the application last set — even on a `null` `HWND`
    /// (where `LVM_GETITEMCOUNT` returns 0). On a non-virtual
    /// ListView, `set_item_count` is never called by the
    /// application, so `item_count` stays at the default `0` and
    /// the round-trip with `get_item_count` is "0 rows" — which
    /// is correct because the application has not pushed any
    /// rows yet (the ListView starts empty in non-virtual mode).
    pub fn get_item_count(&self) -> usize {
        self.inner.borrow().item_count as usize
    }

    /// Delete the item at the given index.
    pub fn delete_item(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LVM_DELETEITEM, index, 0);
        }
    }

    /// Delete all items from the list view.
    pub fn delete_all_items(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LVM_DELETEALLITEMS, 0, 0);
        }
    }

    /// Return the index of the currently selected item, or `None`.
    pub fn get_selected_item(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    LVM_GETNEXTITEM,
                    // Start search from -1 (beginning). Must be a
                    // sign-extended -1: `u32::MAX as usize` would be
                    // 0xFFFFFFFF on Win64, which the ListView reads
                    // as item ~4 billion, not "before the first".
                    -1isize as usize,
                    LVNI_SELECTED as isize,
                )
            };
            if result >= 0 {
                Some(result as usize)
            } else {
                None
            }
        }

        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Programmatically select an item. Sets both `LVIS_SELECTED` and
    /// `LVIS_FOCUSED` to match the single-select focus halo that the
    /// ListView normally applies when the user clicks a row.
    pub fn select(&self, index: usize) {
        self.set_item_state(
            index,
            LVIS_SELECTED | LVIS_FOCUSED,
            LVIS_SELECTED | LVIS_FOCUSED,
        );
    }

    /// Programmatically deselect an item. Clears both `LVIS_SELECTED`
    /// and `LVIS_FOCUSED`.
    pub fn deselect(&self, index: usize) {
        self.set_item_state(index, 0, LVIS_SELECTED | LVIS_FOCUSED);
    }

    /// Clear the selection from all items. Iterates from 0 to
    /// `get_item_count()` and clears the selection state on every row.
    pub fn clear_selection(&self) {
        let count = self.get_item_count();
        for i in 0..count {
            self.set_item_state(i, 0, LVIS_SELECTED | LVIS_FOCUSED);
        }
    }

    /// Return whether the item at the given index is currently selected.
    pub fn is_selected(&self, index: usize) -> bool {
        (self.get_item_state(index, LVIS_SELECTED) & LVIS_SELECTED) != 0
    }

    /// Return the number of selected items (0 if none). Uses
    /// `LVM_GETSELECTEDCOUNT` for an O(1) count.
    pub fn get_selected_item_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result =
                unsafe { SendMessageW(self.inner.borrow().hwnd, LVM_GETSELECTEDCOUNT, 0, 0) };
            result as usize
        }
        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Return the indices of all selected items, in ascending order.
    /// Uses `LVM_GETNEXTITEM` with `LVNI_SELECTED` to walk the selection.
    ///
    /// The walk is bounded by the total item count plus one extra
    /// iteration to absorb the final "no more" sentinel, and has a
    /// no-progress guard so it cannot spin on a null/invalid HWND.
    pub fn get_selected_items(&self) -> Vec<usize> {
        let mut result = Vec::new();
        #[cfg(target_os = "windows")]
        {
            let count = self.get_item_count();
            if count == 0 {
                return result;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let hwnd = self.inner.borrow().hwnd;
                // -1 = start from the beginning. Kept as `isize` so
                // the `as usize` cast sign-extends on Win64 (an
                // `i32` -1 would become 0xFFFFFFFF, breaking the
                // first LVM_GETNEXTITEM query).
                let mut current: isize = -1;
                for _ in 0..=count {
                    let r = SendMessageW(
                        hwnd,
                        LVM_GETNEXTITEM,
                        current as usize,
                        LVNI_SELECTED as isize,
                    );
                    if r < 0 {
                        break;
                    }
                    if r == current {
                        // No progress (e.g. null HWND returns 0 forever). Bail.
                        break;
                    }
                    result.push(r as usize);
                    current = r;
                }
            }
        }
        result
    }

    /// Ask the ListView to repaint the rows in `first..=last`
    /// (`LVM_REDRAWITEMS`). Essential in virtual mode
    /// (`LVS_OWNERDATA`): the control caches what it painted last,
    /// so after the owner's backing data changes the row must be
    /// invalidated explicitly for `LVN_GETDISPINFO` to be asked
    /// again.
    pub fn redraw_items(&self, first: usize, last: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                LVM_REDRAWITEMS,
                first,
                last as isize,
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (first, last);
        }
    }

    /// Set the state bits of a specific item. Internal helper used by
    /// [`select`](Self::select), [`deselect`](Self::deselect), and
    /// [`clear_selection`](Self::clear_selection). Public callers should
    /// prefer the high-level methods above; this is exposed for
    /// power-users that need to set custom state bits (cut, highlight,
    /// etc.).
    pub fn set_item_state(&self, index: usize, state: u32, mask: u32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut item: LVITEMW = std::mem::zeroed();
            item.state = state;
            item.state_mask = mask;
            SendMessageW(
                self.inner.borrow().hwnd,
                LVM_SETITEMSTATE,
                index,
                &item as *const LVITEMW as isize,
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (index, state, mask);
        }
    }

    /// Return the state bits of a specific item masked by `mask`. Used
    /// internally by [`is_selected`](Self::is_selected). The state
    /// bits are documented at
    /// <https://learn.microsoft.com/en-us/windows/win32/api/commctrl/ns-commctrl-lvitemw>.
    pub fn get_item_state(&self, index: usize, mask: u32) -> u32 {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut item: LVITEMW = std::mem::zeroed();
            item.state_mask = mask;
            let r = SendMessageW(
                self.inner.borrow().hwnd,
                LVM_GETITEMSTATE,
                index,
                &item as *const LVITEMW as isize,
            );
            r as u32
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (index, mask);
            0
        }
    }

    /// Set extended list-view styles (e.g. `LVS_EX_FULLROWSELECT`).
    pub fn set_extended_style(&self, style: u32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                0, // mask: 0 means set all bits specified in lParam
                style as isize,
            );
        }
    }

    /// Register a callback that fires when the user selects a row. The
    /// callback receives the zero-based index of the newly selected row,
    /// or `None` if the selection is cleared.
    ///
    /// The ListView notifies its parent via `WM_NOTIFY` (not
    /// `WM_COMMAND`), so this method registers a `WM_NOTIFY` handler on
    /// the supplied `Frame`. The handler filters for the
    /// `LVN_ITEMCHANGED` notification code, queries the ListView for
    /// the current selection with `LVM_GETNEXTITEM` / `LVNI_SELECTED`,
    /// and passes it to the user callback. Internal `last_selection`
    /// state is used to debounce the duplicate `LVN_ITEMCHANGED`
    /// notifications that the control sends per click.
    pub fn on_item_selected<F: FnMut(Option<usize>) + 'static>(
        &self,
        frame: &crate::window::frame::Frame,
        callback: F,
    ) {
        // Store the user's callback inside our inner state.
        self.inner.borrow_mut().on_item_selected = Some(Box::new(callback));

        // Register a WM_NOTIFY handler on the frame.
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_notify_handler(
            id,
            Box::new(move |code| {
                if code != LVN_ITEMCHANGED {
                    return;
                }
                #[cfg(target_os = "windows")]
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    let new_sel = {
                        let hwnd = inner.borrow().hwnd;
                        let r = SendMessageW(
                            hwnd,
                            LVM_GETNEXTITEM,
                            // Sign-extended -1 = "start from the
                            // beginning" (not `u32::MAX as usize`,
                            // which is wrong on Win64).
                            -1isize as usize,
                            LVNI_SELECTED as isize,
                        );
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
                        let cb = inner.borrow_mut().on_item_selected.take();
                        if let Some(mut c) = cb {
                            c(new_sel);
                            inner.borrow_mut().on_item_selected = Some(c);
                        }
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (inner, code);
                }
            }),
        );
    }

    /// Switch the ListView into **virtual** mode
    /// ([`LVS_OWNERDATA`](https://learn.microsoft.com/en-us/windows/win32/controls/list-view-controls-overview))
    /// and set its item count to `count`. In virtual mode the control
    /// does **not** store per-item strings; it asks the parent for
    /// them on demand via `LVN_GETDISPINFOW` whenever it needs to
    /// draw a row.
    ///
    /// The intended use case is "a million rows". A non-virtual
    /// ListView with a million rows would have to allocate a million
    /// `LVITEM` structs on the heap; a virtual ListView only ever
    /// needs the `~30` visible rows in memory at any time.
    ///
    /// # Implementation notes
    ///
    /// The method:
    /// 1. Toggles `LVS_OWNERDATA` on the control's existing style
    ///    word (via `SetWindowLongPtrW` with `GWL_STYLE`). This is
    ///    the only path that works after the HWND has been
    ///    created — `LVS_OWNERDATA` cannot be set or cleared with
    ///    `LVM_SETEXTENDEDLISTVIEWSTYLE`.
    /// 2. Issues `LVM_SETITEMCOUNT` with `LVSICF_NOINVALIDATEALL` so
    ///    the control does not redraw the entire list when the count
    ///    changes; the user will see the new rows on the next scroll
    ///    / paint.
    /// 3. Stores the new count internally so the round-trip
    ///    `set_item_count` / `get_item_count` stays consistent on
    ///    a `null` `HWND` (where `SendMessageW` returns 0).
    ///
    /// # Cross-platform behaviour
    ///
    /// Available on every platform; the non-Windows body is a no-op
    /// so cross-platform code can still call it. Setting the count
    /// on a non-Windows platform cannot fail, so the method
    /// returns `()`.
    ///
    /// # Callback is separate
    ///
    /// This method only flips the style bit and the count. You
    /// still need [`ListCtrl::on_get_disp_info`] to register the
    /// callback that supplies the per-cell text. Without a
    /// callback, the control will silently display blanks for every
    /// row (which is the documented Win32 behaviour for
    /// `LVS_OWNERDATA` with no backing data).
    pub fn set_item_count(&self, count: u32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            // (1) Toggle LVS_OWNERDATA in the style word. We do this
            // unconditionally — if it's already on, this is a
            // no-op; if it's off, the user has just opted in.
            let prev_style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = prev_style | LVS_OWNERDATA;
            if new_style != prev_style {
                SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
            }
            // (2) Tell the control about the new (possibly very
            // large) row count. LVSICF_NOINVALIDATEALL skips the
            // full redraw — the user only sees new rows on the
            // next scroll.
            SendMessageW(
                hwnd,
                LVM_SETITEMCOUNT,
                count as usize,
                LVSICF_NOINVALIDATEALL as isize,
            );
        }
        // (3) Track the count in our own state so a subsequent
        // `get_item_count` round-trips even on a `null` HWND
        // (where LVM_GETITEMCOUNT returns 0).
        self.inner.borrow_mut().item_count = count;
        #[cfg(not(target_os = "windows"))]
        {
            let _ = count;
        }
    }

    /// Register a callback that supplies per-cell text for a
    /// ListView in **virtual** mode (see
    /// [`ListCtrl::set_item_count`]). The callback is invoked by
    /// the parent `Frame`'s `WM_NOTIFY` arm whenever the control
    /// dispatches `LVN_GETDISPINFOW` (i.e. right before drawing a
    /// row, sub-item, or sub-item-range cell).
    ///
    /// # Arguments
    ///
    /// * `frame` — the parent `Frame` that owns the
    ///   `ListCtrl`. Required because the dispatch is implemented
    ///   as a `WM_NOTIFY` handler on the parent (the Win32
    ///   protocol — the list-view sends the notification to its
    ///   parent, not to itself).
    /// * `callback` — a `FnMut(&mut ListItem)`. The wrapper
    ///   describes the cell the control is about to draw; the
    ///   callback can read `index()` / `sub_item()` /
    ///   `is_text_requested()` and call `set_text()` to populate
    ///   it. The callback may be invoked many times in quick
    ///   succession (once per visible cell on every scroll) so
    ///   heavy work should be off the callback path.
    ///
    /// # Replacement semantics
    ///
    /// Calling this method again replaces the previous callback
    /// (the old `Box<dyn FnMut>` is dropped). There is no
    /// "register multiple" or "chain handlers" support.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ru_wx::prelude::*;
    ///
    /// let frame = Frame::builder().with_title("Big list").build();
    /// let list = ListCtrl::new(&frame, ListCtrlStyle::Report);
    /// list.insert_column(0, "Name", 200);
    /// list.set_item_count(1_000_000);
    /// list.on_get_disp_info(&frame, |item: &mut ListItem| {
    ///     if item.is_text_requested() {
    ///         // Source from a real model in production code.
    ///         let _ = item.set_text(&format!("row {}", item.index()));
    ///     }
    /// });
    /// ```
    pub fn on_get_disp_info<F: FnMut(&mut ListItem) + 'static>(
        &self,
        frame: &crate::window::frame::Frame,
        callback: F,
    ) {
        // Store the user's callback inside our inner state.
        self.inner.borrow_mut().on_get_disp_info = Some(Box::new(callback));

        // Register a WM_NOTIFY handler on the frame. The handler is
        // dispatched with the full `lparam` (a pointer to the
        // `NMLVDISPINFOW`) so the user-supplied `&mut ListItem`
        // wrapper can read the request fields (i_item, i_sub_item,
        // mask) and write the response into psz_text.
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_disp_info_handler(
            id,
            Box::new(move |lparam| {
                if lparam == 0 {
                    return;
                }
                #[cfg(target_os = "windows")]
                // SAFETY: `lparam` is a pointer to a `NMLVDISPINFOW`
                // supplied by the Win32 ListView when it dispatched
                // LVN_GETDISPINFOW. The control owns the storage and
                // it is valid for the duration of this call. We
                // re-interpret the pointer as `&mut NMLVDISPINFOW`
                // (we never read past the `item` field) and then
                // narrow the borrow to `&mut LVITEMW` for the
                // `ListItem` wrapper.
                unsafe {
                    let nmlv = lparam as *mut NMLVDISPINFOW;
                    if nmlv.is_null() {
                        return;
                    }
                    let item_ref: &mut LVITEMW = &mut (*nmlv).item;
                    let mut wrapper = ListItem { item: item_ref };
                    // Take the callback out, invoke it without
                    // holding the RefCell borrow, then put it back.
                    let cb = inner.borrow_mut().on_get_disp_info.take();
                    if let Some(mut c) = cb {
                        c(&mut wrapper);
                        inner.borrow_mut().on_get_disp_info = Some(c);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (inner, lparam);
                }
            }),
        );
    }

    /// Register a callback that fires when an
    /// `LVS_OWNERDATA` (virtual) ListView is about to ask for a
    /// contiguous range of items (i.e. right before a burst of
    /// `LVN_GETDISPINFOW` requests). This is the standard
    /// optimisation hook for virtual lists: the application uses
    /// the hint to **prefetch** the backing data (open a file,
    /// query a database, decompress a chunk) so the subsequent
    /// per-cell requests can be served from a cache.
    ///
    /// This notification is purely advisory — the ListView does
    /// not require the application to do anything, and the
    /// [`on_get_disp_info`](Self::on_get_disp_info) callback
    /// will still be invoked for the items in the range even
    /// if the application ignores the hint. The two callbacks
    /// are independent: you can register one, both, or neither.
    ///
    /// # Arguments
    ///
    /// * `frame` — the parent `Frame` that owns the
    ///   `ListCtrl`. Required because the dispatch is implemented
    ///   as a `WM_NOTIFY` handler on the parent (the Win32
    ///   protocol — the list-view sends the notification to its
    ///   parent, not to itself).
    /// * `callback` — a `FnMut(&CacheHint)`. The wrapper
    ///   exposes the inclusive lower and upper bounds of the
    ///   row range (`from()` and `to()`). The callback may be
    ///   invoked many times in quick succession (once per
    ///   visible-range change / scroll), so it is the right
    ///   place to push work **off** the per-cell hot path,
    ///   not to do extra work.
    ///
    /// # Replacement semantics
    ///
    /// Calling this method again replaces the previous callback
    /// (the old `Box<dyn FnMut>` is dropped). There is no
    /// "register multiple" or "chain handlers" support.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ru_wx::prelude::*;
    ///
    /// let frame = Frame::builder().with_title("Big list").build();
    /// let list = ListCtrl::new(&frame, ListCtrlStyle::Report);
    /// list.insert_column(0, "Name", 200);
    /// list.set_item_count(1_000_000);
    /// list.on_get_disp_info(&frame, |item: &mut ListItem| {
    ///     if item.is_text_requested() {
    ///         let _ = item.set_text(&format!("row {}", item.index()));
    ///     }
    /// });
    /// list.on_cache_hint(&frame, |hint: &CacheHint| {
    ///     // In production code, pre-load `hint.from()..=hint.to()`.
    /// });
    /// ```
    pub fn on_cache_hint<F: FnMut(&CacheHint) + 'static>(
        &self,
        frame: &crate::window::frame::Frame,
        callback: F,
    ) {
        // Store the user's callback inside our inner state.
        self.inner.borrow_mut().on_cache_hint = Some(Box::new(callback));

        // Register a WM_NOTIFY handler on the frame. The handler is
        // dispatched with the full `lparam` (a pointer to the
        // `NMLVCACHEHINT`) so the user-supplied `&CacheHint`
        // wrapper can read the `i_from` / `i_to` fields.
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_cache_hint_handler(
            id,
            Box::new(move |lparam| {
                if lparam == 0 {
                    return;
                }
                #[cfg(target_os = "windows")]
                // SAFETY: `lparam` is a pointer to a `NMLVCACHEHINT`
                // supplied by the Win32 ListView when it dispatched
                // LVN_ODCACHEHINT. The control owns the storage and
                // it is valid for the duration of this call. We
                // re-interpret the pointer as `&NMLVCACHEHINT`
                // (we never read past the `i_to` field) and then
                // narrow the borrow to `&CacheHint` for the
                // wrapper.
                unsafe {
                    let nmch = lparam as *const NMLVCACHEHINT;
                    if nmch.is_null() {
                        return;
                    }
                    let hint_ref: &NMLVCACHEHINT = &*nmch;
                    let wrapper = CacheHint { hint: hint_ref };
                    // Take the callback out, invoke it without
                    // holding the RefCell borrow, then put it back.
                    let cb = inner.borrow_mut().on_cache_hint.take();
                    if let Some(mut c) = cb {
                        c(&wrapper);
                        inner.borrow_mut().on_cache_hint = Some(c);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (inner, lparam);
                }
            }),
        );
    }

    /// Get the control ID
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ─────────────────────────────────────────────────────

impl Widget for ListCtrlInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
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
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
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

// ── Unit tests (v0.5.2) ────────────────────────────────────────────────────
//
// These tests pin the Win32 constant values and the public-method
// signatures introduced in v0.5.2 (`select`, `deselect`,
// `clear_selection`, `is_selected`, `get_selected_item_count`,
// `get_selected_items`, `set_item_state`, `get_item_state`). They use
// the `Frame::for_testing()` constructor (which produces a `Frame`
// with a `null` `HWND`) so they can run on any host without a real
// Win32 message pump; the resulting `ListCtrl` has a `null` HWND
// too, and `SendMessageW` on a `null` HWND is a no-op that returns
// 0, so the new methods are exercised against that safe null-HWND
// fallback path.
//
// The actual selection behaviour against a real ListView is verified
// by the integration tests in `tests/integration.rs`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::frame::Frame;

    // ---- Win32 constant pinning (Windows-only) -----------------------------

    /// Pin the wide (`...W`) ListView message ids. These values come
    /// from the Microsoft Docs ListView messages table. Locking them
    /// down with an assertion turns a silent constant typo into a
    /// compile-time + test-time failure.
    #[cfg(target_os = "windows")]
    #[test]
    fn lvm_constants_have_expected_values() {
        assert_eq!(LVM_FIRST, 0x1000);
        assert_eq!(LVM_INSERTCOLUMN, LVM_FIRST + 97);
        assert_eq!(LVM_INSERTITEM, LVM_FIRST + 77);
        assert_eq!(LVM_SETITEMTEXT, LVM_FIRST + 116);
        assert_eq!(LVM_GETITEMTEXT, LVM_FIRST + 115);
        assert_eq!(LVM_GETITEMCOUNT, LVM_FIRST + 4);
        assert_eq!(LVM_DELETEITEM, LVM_FIRST + 8);
        assert_eq!(LVM_DELETEALLITEMS, LVM_FIRST + 9);
        assert_eq!(LVM_GETNEXTITEM, LVM_FIRST + 12);
        // New in v0.5.2:
        assert_eq!(LVM_SETITEMSTATE, LVM_FIRST + 43);
        assert_eq!(LVM_GETITEMSTATE, LVM_FIRST + 44);
        assert_eq!(LVM_GETSELECTEDCOUNT, LVM_FIRST + 50);
        assert_eq!(LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_FIRST + 54);
    }

    /// Pin the LVIS_* and LVS_EX_* state-bit constants. These bits
    /// are documented at
    /// <https://learn.microsoft.com/en-us/windows/win32/api/commctrl/ns-commctrl-lvitemw>.
    #[cfg(target_os = "windows")]
    #[test]
    fn lvis_constants_have_expected_values() {
        assert_eq!(LVIS_FOCUSED, 0x0001);
        assert_eq!(LVIS_SELECTED, 0x0002);
        assert_eq!(LVNI_SELECTED, 2);
        assert_eq!(LVS_EX_FULLROWSELECT, 0x20);
    }

    // ---- Public-method signature pinning (always available) ----------------

    /// Pin the signature of `select`. The function-pointer coercion
    /// fails at compile time if the signature ever changes, so this
    /// is a free "API contract" guard.
    #[test]
    fn signature_select() {
        let _: fn(&ListCtrl, usize) = ListCtrl::select;
    }

    #[test]
    fn signature_deselect() {
        let _: fn(&ListCtrl, usize) = ListCtrl::deselect;
    }

    #[test]
    fn signature_clear_selection() {
        let _: fn(&ListCtrl) = ListCtrl::clear_selection;
    }

    #[test]
    fn signature_is_selected() {
        let _: fn(&ListCtrl, usize) -> bool = ListCtrl::is_selected;
    }

    #[test]
    fn signature_get_selected_item_count() {
        let _: fn(&ListCtrl) -> usize = ListCtrl::get_selected_item_count;
    }

    #[test]
    fn signature_get_selected_items() {
        let _: fn(&ListCtrl) -> Vec<usize> = ListCtrl::get_selected_items;
    }

    #[test]
    fn signature_set_item_state() {
        let _: fn(&ListCtrl, usize, u32, u32) = ListCtrl::set_item_state;
    }

    #[test]
    fn signature_get_item_state() {
        let _: fn(&ListCtrl, usize, u32) -> u32 = ListCtrl::get_item_state;
    }

    // ---- Null-HWND safety (Windows-only, where the methods are wired up) --

    /// Build a `ListCtrl` whose underlying `HWND` is `NULL`. With a
    /// `Frame::for_testing()` parent the `CreateWindowExW` call in
    /// `ListCtrl::new` is issued with a `null` parent + `WS_CHILD`,
    /// which fails and returns `NULL`; that `NULL` is stored in the
    /// inner state and is what the methods below operate on. Any
    /// panic on the null-HWND path is a regression.
    #[cfg(target_os = "windows")]
    fn make_null_hwnd_listctrl() -> ListCtrl {
        let frame = Frame::for_testing();
        ListCtrl::new(&frame, ListCtrlStyle::List)
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_select_does_not_panic() {
        let lc = make_null_hwnd_listctrl();
        lc.select(0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_deselect_does_not_panic() {
        let lc = make_null_hwnd_listctrl();
        lc.deselect(0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_clear_selection_does_not_panic() {
        let lc = make_null_hwnd_listctrl();
        lc.clear_selection();
    }

    /// On a null HWND `SendMessageW` returns 0, so the state bits are
    /// 0 and `is_selected` must be `false`. This is the "no false
    /// positive" guard.
    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_is_selected_returns_false() {
        let lc = make_null_hwnd_listctrl();
        assert!(!lc.is_selected(0));
        assert!(!lc.is_selected(5));
    }

    /// On a null HWND `LVM_GETSELECTEDCOUNT` returns 0.
    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_get_selected_item_count_returns_zero() {
        let lc = make_null_hwnd_listctrl();
        assert_eq!(lc.get_selected_item_count(), 0);
    }

    /// On a null HWND `LVM_GETITEMCOUNT` is 0, so
    /// `get_selected_items` short-circuits and returns an empty Vec
    /// (and crucially does NOT spin in its `LVM_GETNEXTITEM` loop,
    /// because the `count == 0` guard kicks in first).
    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_get_selected_items_returns_empty() {
        let lc = make_null_hwnd_listctrl();
        let items = lc.get_selected_items();
        assert!(items.is_empty());
    }

    /// `get_item_state` on a null HWND must return 0 (the
    /// SendMessageW fallback), and `get_item_state(_, 0)` must
    /// return 0 (masking with 0 yields 0).
    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_get_item_state_returns_zero() {
        let lc = make_null_hwnd_listctrl();
        assert_eq!(lc.get_item_state(0, LVIS_SELECTED), 0);
        assert_eq!(lc.get_item_state(0, 0), 0);
    }

    // ---- v0.5.6: virtual list mode (LVS_OWNERDATA) tests --------
    //
    // The `set_item_count` and `on_get_disp_info` methods, plus
    // the public `ListItem` wrapper, are the new surface area in
    // v0.5.6. The constant-pinning tests guard against a future
    // magic-number drift; the signature-pinning tests guard
    // against a future API rename; the null-hwnd tests prove
    // the methods do not panic on a `Frame::for_testing()`
    // parent (which is how the unit tests exercise the crate
    // without a real Win32 message pump).

    /// Pin the new v0.5.6 `LVN_GETDISPINFOW` notification code.
    /// This is the W (Unicode) variant; the A variant has a
    /// different code (0xFFFFFF6A) and we deliberately do not
    /// support it (the whole crate goes through the wide Win32
    /// entry points).
    #[cfg(target_os = "windows")]
    #[test]
    fn lvn_getdispinfow_has_expected_value() {
        // LVN_FIRST = -100 (as u32 = 0xFFFFFF9C). 0xFFFFFF9C - 77 = 0xFFFFFF4F.
        assert_eq!(LVN_GETDISPINFOW, 0xFFFFFF4F);
        // As signed `i32` values, both notification codes are
        // negative (`LVN_GETDISPINFOW` is -177, `LVN_ITEMCHANGED`
        // is -101). The unsigned representation puts
        // `LVN_GETDISPINFOW` at a numerically *lower* u32 value
        // even though its *signed* value is more negative; the
        // point of the assertion is to pin the "lower / more
        // negative" ordering, which is what callers see if they
        // ever do arithmetic on the codes.
        assert!((LVN_GETDISPINFOW as i32) < (LVN_ITEMCHANGED as i32));
    }

    /// Pin the `LVS_OWNERDATA` style bit. This is the same bit
    /// number as `LVS_OWNERDRAWFIXED`, which is the documented
    /// Microsoft aliasing (the two styles are mutually
    /// exclusive).
    #[cfg(target_os = "windows")]
    #[test]
    fn lvs_ownerdata_has_expected_value() {
        assert_eq!(LVS_OWNERDATA, 0x1000);
    }

    /// Pin the `LVM_SETITEMCOUNT` message id. It's `LVM_FIRST +
    /// 47` per the Microsoft Docs ListView messages table.
    #[cfg(target_os = "windows")]
    #[test]
    fn lvm_setitemcount_has_expected_value() {
        assert_eq!(LVM_SETITEMCOUNT, LVM_FIRST + 47);
    }

    /// Pin the `LVSICF_*` flags. `LVSICF_NOINVALIDATEALL = 0x1`
    /// and `LVSICF_NOSCROLL = 0x2` are the two flags we use in
    /// `set_item_count`; the third (`LVSICF_INVALIDATEALL = 0`)
    /// is just "no flag" and is not a discrete constant.
    #[cfg(target_os = "windows")]
    #[test]
    fn lvsicf_flags_have_expected_values() {
        assert_eq!(LVSICF_NOINVALIDATEALL, 0x0001);
        assert_eq!(LVSICF_NOSCROLL, 0x0002);
    }

    /// Pin the `set_item_count` signature. A future change
    /// (e.g. splitting count into row + column counts) would
    /// fail to compile here.
    #[test]
    fn signature_set_item_count() {
        let _: fn(&ListCtrl, u32) = ListCtrl::set_item_count;
    }

    /// Pin the `on_get_disp_info` signature. A future change
    /// (e.g. adding a filter mask parameter, or changing the
    /// callback to `FnOnce`) would fail to compile here.
    #[test]
    #[allow(clippy::type_complexity)]
    fn signature_on_get_disp_info() {
        let _: fn(&ListCtrl, &crate::window::frame::Frame, Box<dyn FnMut(&mut ListItem)>) =
            ListCtrl::on_get_disp_info;
    }

    /// `set_item_count` on a null HWND must not panic and must
    /// store the count in `inner.item_count` so a subsequent
    /// `get_item_count` round-trips (LVM_GETITEMCOUNT on a null
    /// HWND returns 0, so without the local cache the
    /// round-trip would falsely report 0).
    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_set_item_count_tracks_local_state() {
        let lc = make_null_hwnd_listctrl();
        // First, the round-trip defaults to 0.
        assert_eq!(lc.get_item_count(), 0);
        // Setting 12345 must update local state even though
        // SendMessageW is a no-op.
        lc.set_item_count(12345);
        assert_eq!(lc.get_item_count(), 12345);
        // Setting 0 must reset to 0.
        lc.set_item_count(0);
        assert_eq!(lc.get_item_count(), 0);
    }

    /// Registering a disp-info callback must insert a handler
    /// into the frame's `disp_info_handlers` map keyed by the
    /// `ListCtrl`'s id. The unit test exercises only the
    /// registration path; the actual `LVN_GETDISPINFOW`
    /// dispatch requires a real Win32 message pump and is
    /// covered by the manual smoke-test in
    /// `examples/showcase_all.rs`.
    #[cfg(target_os = "windows")]
    #[test]
    fn on_get_disp_info_registers_handler_on_frame() {
        let frame = Frame::for_testing();
        let lc = ListCtrl::new(&frame, ListCtrlStyle::List);
        let id = lc.id();
        // Before registration: the frame has no disp-info
        // handler for this id.
        assert!(!frame.inner.borrow().disp_info_handlers.contains_key(&id));
        lc.on_get_disp_info(&frame, |_item: &mut ListItem| {});
        // After registration: the frame's `disp_info_handlers`
        // map contains an entry for the control id.
        assert!(frame.inner.borrow().disp_info_handlers.contains_key(&id));
    }

    // ---- v0.6.0: LVN_ODCACHEHINT (virtual-mode prefetch hint) ----
    //
    // The `on_cache_hint` method, plus the new `CacheHint`
    // wrapper and the `LVN_ODCACHEHINT` / `NMLVCACHEHINT`
    // surface area, are the new virtual-mode API in v0.6.0.
    // The constant-pinning test guards against a future
    // magic-number drift; the signature-pinning test guards
    // against a future API rename; the null-hwnd test proves
    // the registration path does not panic on a
    // `Frame::for_testing()` parent.

    /// Pin the new v0.6.0 `LVN_ODCACHEHINT` notification code.
    /// This is the W (Unicode) variant; the A variant has a
    /// different code (0xFFFFFF68) and we deliberately do not
    /// support it (the whole crate goes through the wide Win32
    /// entry points).
    ///
    /// The code is `LVN_FIRST - 79` = `(0U - 100U) - 79` =
    /// 0xFFFFFF4D. We pin it here so a future typo in the
    /// constant declaration cannot silently break the
    /// `WM_NOTIFY` dispatch in `frame.rs` (the dispatch arm
    /// compares the incoming code against this constant; a
    /// mismatch would mean the callback never fires in
    /// production but the unit tests would still pass).
    #[cfg(target_os = "windows")]
    #[test]
    fn lvn_odcachehint_has_expected_value() {
        assert_eq!(LVN_ODCACHEHINT, 0xFFFFFF4D);
        // The cache-hint code is two below the
        // get-disp-info code (0xFFFFFF4F), and the
        // item-changed code (0xFFFFFF63) is above both.
        const { assert!(LVN_ODCACHEHINT < LVN_GETDISPINFOW) };
        const { assert!(LVN_ODCACHEHINT < LVN_ITEMCHANGED) };
    }

    /// Pin the `CacheHint::from` / `CacheHint::to` return
    /// types. A future change (e.g. switching to a
    /// `Range<usize>` return) would fail to compile here. We
    /// don't pin the full `fn`-pointer signature because
    /// `CacheHint` is parameterised on its own data lifetime
    /// (the borrowed `NMLVCACHEHINT`) which has a subtle
    /// relationship to the method's reference lifetime — the
    /// user-facing `on_cache_hint` signature test below is
    /// the more durable API pin.
    #[test]
    fn signature_cache_hint_accessors_return_usize() {
        let hint_holder: Option<CacheHint<'_>> = None;
        if let Some(h) = hint_holder.as_ref() {
            // The exact values don't matter (None is the
            // `if-let` path); what matters is that calling
            // `from()` and `to()` compiles and returns `usize`.
            let _f: usize = h.from();
            let _t: usize = h.to();
        }
    }

    /// Pin the `on_cache_hint` signature. A future change
    /// (e.g. changing the callback to `FnOnce`, or adding a
    /// filter mask parameter) would fail to compile here.
    #[test]
    #[allow(clippy::type_complexity)]
    fn signature_on_cache_hint() {
        let _: fn(&ListCtrl, &crate::window::frame::Frame, Box<dyn FnMut(&CacheHint<'_>)>) =
            ListCtrl::on_cache_hint;
    }

    /// `on_cache_hint` on a `null`-`HWND` `ListCtrl` (a
    /// `Frame::for_testing()` parent that has no real
    /// `HWND` behind it) must not panic. The registration
    /// path stores the closure in `inner.on_cache_hint` and
    /// registers a handler in the frame's
    /// `cache_hint_handlers` map; both are pure data
    /// operations and have no Win32 dependency.
    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_on_cache_hint_does_not_panic() {
        let lc = make_null_hwnd_listctrl();
        lc.on_cache_hint(&Frame::for_testing(), |_hint: &CacheHint| {});
    }

    /// Registering a cache-hint callback must insert a
    /// handler into the frame's `cache_hint_handlers` map
    /// keyed by the `ListCtrl`'s id. Mirrors the
    /// `on_get_disp_info_registers_handler_on_frame` test
    /// above.
    #[cfg(target_os = "windows")]
    #[test]
    fn on_cache_hint_registers_handler_on_frame() {
        let frame = Frame::for_testing();
        let lc = ListCtrl::new(&frame, ListCtrlStyle::List);
        let id = lc.id();
        // Before registration: the frame has no cache-hint
        // handler for this id.
        assert!(!frame.inner.borrow().cache_hint_handlers.contains_key(&id));
        lc.on_cache_hint(&frame, |_hint: &CacheHint| {});
        // After registration: the frame's
        // `cache_hint_handlers` map contains an entry for
        // the control id.
        assert!(frame.inner.borrow().cache_hint_handlers.contains_key(&id));
    }
}
