//! WinUI3 Grid backend - native Win32 implementation.
//!
//! Implements the Grid widget on top of `SysListView32` in `LVS_REPORT`
//! mode with `LVS_OWNERDATA` for efficient data binding. Text is
//! supplied on demand through `LVN_GETDISPINFO` notifications.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use once_cell::sync::Lazy;
use std::sync::Mutex;

// ============================================================================
// Re-exports of types from containers/grid
// ============================================================================

pub use crate::containers::grid::{
    BadgeKind, BarStyle, Cell, ColumnAlign, GridAppearance, GridCellStyle, GridDateFormat,
    NumberFormat, PriorityKind, SortOrder,
};

/// Convert a `Cell` value to its text representation for display
/// in the WinUI3 Grid backend.
fn cell_to_string(cell: crate::containers::grid::Cell) -> String {
    use crate::containers::grid::Cell;
    match cell {
        Cell::Empty => String::new(),
        Cell::Text(s) => s,
        Cell::Image { text, .. } => text,
        Cell::ImageOnly(_) => String::new(),
        Cell::Progress { value, max, .. } => format!("{}/{}", value, max),
        Cell::MultiLine(s) => s,
        Cell::Bar {
            value,
            max,
            width: _,
            style: _,
            label,
        } => {
            if let Some(l) = label {
                l
            } else {
                format!("{}/{}", value, max)
            }
        }
        Cell::Badge { text, .. } => text,
        Cell::Number { value, format: _ } => format!("{}", value),
        // Catch-all for any future Cell variants (DateTime, Link,
        // Priority, etc.) — we just render their text representation.
        _ => format!("{:?}", cell),
    }
}
pub use crate::containers::grid_block::GridBlock;
pub use crate::containers::grid_coords::GridCoords;
pub use crate::containers::grid_range::GridRange;
pub use crate::containers::grid_string_table::GridStringTable;
pub use crate::containers::grid_table::{FunctionGridTable, GridTable};

// ============================================================================
// Win32 constants
// ============================================================================

const LVM_INSERTCOLUMNW: u32 = 0x1000 + 97;
const LVM_SETITEMCOUNT: u32 = 0x1000 + 47;
const LVM_INSERTITEM: u32 = 0x1000 + 7;
const LVM_SETITEMTEXT: u32 = 0x1000 + 76;
const LVM_DELETEALLITEMS: u32 = 0x1000 + 9;
const LVM_GETITEMCOUNT: u32 = 0x1000 + 4;
const LVN_GETDISPINFO: u32 = 0xFFFFFF9C_u32 - 77;
const LVIF_TEXT: u32 = 0x0001;
const NMHDR_WM_NOTIFY: u32 = 0x004E;

// ============================================================================
// Native structs
// ============================================================================

#[repr(C)]
#[allow(non_snake_case)]
pub struct LVCOLUMNW {
    pub mask: u32,
    pub fmt: i32,
    pub cx: i32,
    pub pszText: *mut u16,
    pub cchTextMax: i32,
    pub iSubItem: i32,
    pub iImage: i32,
    pub iOrder: i32,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct LVITEM {
    pub mask: u32,
    pub iItem: i32,
    pub iSubItem: i32,
    pub state: u32,
    pub stateMask: u32,
    pub pszText: *mut u16,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: isize,
    pub iIndent: i32,
    pub iGroupId: i32,
    pub cColumns: u32,
    pub puColumns: *mut u32,
}

#[repr(C)]
pub struct NMLVDISPINFO {
    pub hdr: NMHDR,
    pub item: LVITEM,
}

#[repr(C)]
#[allow(non_snake_case)] // field names are pinned to the Windows NMHDR ABI
pub struct NMHDR {
    pub hwndFrom: HWND,
    pub idFrom: usize,
    pub code: u32,
}

// ============================================================================
// Subclass state (one entry per Grid widget)
// ============================================================================

type HwndKey = usize;
struct SubclassEntry {
    state: *mut RefCell<GridState>,
    prev_proc: isize,
}
unsafe impl Send for SubclassEntry {}

static SUBCLASS_TABLE: Lazy<Mutex<HashMap<HwndKey, SubclassEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

unsafe fn register_subclass(hwnd: HWND, state: *mut RefCell<GridState>, prev_proc: isize) {
    SUBCLASS_TABLE
        .lock()
        .unwrap()
        .insert(hwnd as HwndKey, SubclassEntry { state, prev_proc });
}

unsafe fn unregister_subclass(hwnd: HWND) -> Option<SubclassEntry> {
    SUBCLASS_TABLE.lock().unwrap().remove(&(hwnd as HwndKey))
}

// ============================================================================
// GridState - per-widget data
// ============================================================================

type GridValueProvider = Rc<RefCell<Box<dyn FnMut(usize, usize) -> String>>>;

struct GridState {
    columns: Vec<GridColumn>,
    data: Vec<Vec<String>>,
    selected_rows: HashSet<usize>,
    focused_cell: Option<CellCoord>,
    appearance: GridAppearance,
    value_provider: Option<GridValueProvider>,
    row_count: usize,
}

#[derive(Debug, Clone)]
pub struct GridColumn {
    pub header: String,
    pub width: i32,
    pub align: ColumnAlign,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CellCoord {
    pub row: usize,
    pub col: usize,
}

impl CellCoord {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

// ============================================================================
// Grid - the public widget
// ============================================================================

pub struct Grid {
    hwnd: HWND,
    state: Rc<RefCell<GridState>>,
}

impl Grid {
    pub fn new<W: Window>(parent: &W) -> Self {
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide("SysListView32");
            let hwnd = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                class_name.as_ptr(),
                std::ptr::null_mut(),
                WS_CHILD
                    | WS_VISIBLE
                    | LVS_REPORT
                    | LVS_SINGLESEL
                    | LVS_SHOWSELALWAYS
                    | LVS_OWNERDATA,
                0,
                0,
                600,
                400,
                parent.hwnd() as HWND,
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null(),
            );
            let state = Rc::new(RefCell::new(GridState {
                columns: Vec::new(),
                data: Vec::new(),
                selected_rows: HashSet::new(),
                focused_cell: None,
                appearance: GridAppearance::win11(),
                value_provider: None,
                row_count: 0,
            }));
            let state_ptr: *mut RefCell<GridState> =
                Rc::into_raw(state.clone()) as *mut RefCell<GridState>;
            // Win32 subclassing requires the wndproc address as `isize`.
            // This is the canonical SetWindowLongPtrW(GWLP_WNDPROC, ...) pattern.
            let new_proc = grid_wnd_proc as *const () as usize as isize;
            let prev_proc = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, new_proc);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            register_subclass(hwnd, state_ptr, prev_proc);
            SendMessageW(
                hwnd,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                0,
                (LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES | LVS_EX_DOUBLEBUFFER) as isize,
            );
            Self { hwnd, state }
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
    pub fn row_count(&self) -> usize {
        self.state.borrow().row_count
    }
    pub fn col_count(&self) -> usize {
        self.state.borrow().columns.len()
    }
    pub fn column_title(&self, col: usize) -> String {
        self.state
            .borrow()
            .columns
            .get(col)
            .map(|c| c.header.clone())
            .unwrap_or_default()
    }

    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        struct W(HWND);
        impl crate::core::widget::Widget for W {
            fn native_handle(&self) -> isize {
                self.0 as isize
            }
            fn set_position(&mut self, x: i32, y: i32) {
                unsafe {
                    SetWindowPos(self.0, std::ptr::null_mut(), x, y, 0, 0, 0x0010 | 0x0040);
                }
            }
            fn set_size(&mut self, w: u32, h: u32) {
                unsafe {
                    SetWindowPos(
                        self.0,
                        std::ptr::null_mut(),
                        0,
                        0,
                        w as i32,
                        h as i32,
                        0x0002 | 0x0040,
                    );
                }
            }
            fn rect(&self) -> crate::core::geometry::Rect {
                unsafe {
                    let mut r: RECT = std::mem::zeroed();
                    GetClientRect(self.0, &mut r);
                    crate::core::geometry::Rect {
                        x: r.left,
                        y: r.top,
                        width: (r.right - r.left) as u32,
                        height: (r.bottom - r.top) as u32,
                    }
                }
            }
            fn is_visible(&self) -> bool {
                true
            }
            fn set_visible(&mut self, _v: bool) {}
            fn is_enabled(&self) -> bool {
                true
            }
            fn set_enabled(&mut self, _e: bool) {}
        }
        Rc::new(RefCell::new(W(self.hwnd)))
    }

    pub fn append_column_with_align(&self, header: &str, width: i32, align: ColumnAlign) {
        let col = GridColumn {
            header: header.to_string(),
            width,
            align,
        };
        self.state.borrow_mut().columns.push(col);
        unsafe {
            let cols = self.state.borrow().columns.len();
            let wide: Vec<u16> = header.encode_utf16().chain(std::iter::once(0)).collect();
            let mut lvc = std::mem::zeroed::<LVCOLUMNW>();
            lvc.mask = 0x0001 | 0x0002 | 0x0004 | 0x0008;
            lvc.fmt = match align {
                ColumnAlign::Left => 0,
                ColumnAlign::Center => 2,
                ColumnAlign::Right => 1,
            };
            lvc.cx = width;
            lvc.iSubItem = (cols - 1) as i32;
            lvc.pszText = wide.as_ptr() as *mut _;
            lvc.cchTextMax = wide.len() as i32;
            SendMessageW(
                self.hwnd,
                LVM_INSERTCOLUMNW,
                cols - 1,
                &lvc as *const _ as isize,
            );
        }
    }

    pub fn set_row_count(&self, count: usize) {
        let cols = self.state.borrow().columns.len();
        {
            let mut state = self.state.borrow_mut();
            let cur = state.data.len();
            if count > cur {
                let mut new_rows_data: Vec<Vec<String>> = Vec::new();
                for row in cur..count {
                    let mut row_data: Vec<String> = Vec::with_capacity(cols);
                    if let Some(provider) = state.value_provider.as_ref() {
                        let mut p = provider.borrow_mut();
                        for col in 0..cols {
                            row_data.push((p)(row, col));
                        }
                    } else {
                        for _ in 0..cols {
                            row_data.push(String::new());
                        }
                    }
                    new_rows_data.push(row_data);
                }
                state.data.extend(new_rows_data);
            } else {
                state.data.truncate(count);
            }
            state.row_count = count;
        }
        unsafe {
            SendMessageW(self.hwnd, LVM_SETITEMCOUNT, count, 0);
        }
    }

    /// Set a value provider that returns `String` values.
    pub fn set_value_provider_string<F: FnMut(usize, usize) -> String + 'static>(&self, f: F) {
        let cell: Rc<RefCell<F>> = Rc::new(RefCell::new(f));
        let cell_for_closure = cell.clone();
        self.state.borrow_mut().value_provider =
            Some(Rc::new(RefCell::new(Box::new(move |row, col| {
                (cell_for_closure.borrow_mut())(row, col)
            }))));
    }

    /// Set a value provider that returns `Cell` values (matches Win32 Grid API).
    /// Cells are converted to their text representation internally.
    pub fn set_value_provider<F>(&self, mut f: F)
    where
        F: FnMut(usize, usize) -> crate::containers::grid::Cell + 'static,
    {
        self.set_value_provider_string(move |row, col| cell_to_string(f(row, col)));
    }
    pub fn force_refresh(&self) {
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
        }
    }

    pub fn request_repaint(&self) {
        self.force_refresh();
    }

    /// Clone the Grid (shared Rc semantics, not a deep copy).
    #[allow(clippy::should_implement_trait)] // inherent handle-clone by design; not the Clone trait
    pub fn clone(&self) -> Self {
        Self {
            hwnd: self.hwnd,
            state: Rc::clone(&self.state),
        }
    }

    /// Register a handler for selection changes (matches Win32 API).
    pub fn on_selection_changed<F: FnMut(Option<usize>) + 'static>(
        &self,
        _frame: &impl crate::core::widget::Window,
        _f: F,
    ) {
        // Stub — requires WM_NOTIFY/LVN_ITEMCHANGED routing.
    }

    /// Attach icons with explicit column width (stub).
    pub fn attach_icons_with_column_width(
        &self,
        _icons: &crate::GridIcons,
        _col: usize,
        _width: i32,
    ) {
    }

    /// Call a closure after the message loop starts (matches Win32 API).
    pub fn call_after_message_loop<F: FnOnce() + 'static>(
        &self,
        _frame: &impl crate::core::widget::Window,
        _f: F,
    ) {
    }

    /// Refresh the grid display (stub for force_refresh).
    pub fn refresh(&self) {
        self.force_refresh();
    }

    /// Sort rows by the given column index (stub).
    pub fn sort_by_column(&self, _col: usize, _order: crate::containers::grid::SortOrder) {}

    /// Set the grid appearance (matches Win32 API).
    pub fn set_appearance(
        &self,
        _app: crate::containers::grid::GridAppearance,
        _frame: Option<&impl crate::core::widget::Window>,
    ) {
    }

    /// Set checked state for a checkbox row (matches Win32 API).
    pub fn set_checked(&self, _row: usize, _checked: bool) {}

    /// Set value at a cell (stub).
    pub fn set_cell_value(&self, _row: usize, _col: usize, _value: &str) {}

    /// Get value at a cell (stub).
    pub fn get_cell_value(&self, _row: usize, _col: usize) -> String {
        String::new()
    }
    /// Enable or disable grid editing (stub).
    pub fn enable_editing(&self, _enabled: bool) {}

    /// Set column alignment for a specific column (stub).
    pub fn set_column_align(&self, _col: usize, _align: crate::containers::grid::ColumnAlign) {}

    // ---- Stubs for methods present in the Win32 Grid but not yet
    //      implemented in the WinUI3 Grid backend.

    /// Win11 visual style theme (stub).
    pub fn apply_win11_theme(&self, _frame: &impl crate::core::widget::Window) {}

    /// Set a custom font description (stub).
    pub fn set_font_desc(&self, _desc: crate::core::font::FontDesc, _redraw: bool) {}

    /// Adjust font size by delta points (stub).
    pub fn adjust_font_size(&self, _delta: i32) {}

    /// Set cell style (stub).
    pub fn set_cell_style(
        &self,
        _row: usize,
        _col: usize,
        _style: crate::containers::grid::GridCellStyle,
    ) {
    }

    /// Set row style (stub).
    pub fn set_row_style(&self, _row: usize, _style: crate::containers::grid::GridCellStyle) {}

    /// Enable interactive features like column resizing (stub).
    pub fn enable_interactive_features(&self, _frame: &impl crate::core::widget::Window) {}

    /// Enable column header context menu (stub).
    pub fn enable_column_context_menu(&self, _frame: &impl crate::core::widget::Window) {}
    pub fn enable_column_context_menu_with_frame(&self, _frame: &impl crate::core::widget::Window) {
    }

    /// Enable checkbox column rendering (stub).
    pub fn set_checkboxes(&self, _enabled: bool) {}

    /// Set an image list for row icons (stub).
    pub fn set_image_list(&self, _list: &crate::ImageList) {}

    /// Check/uncheck all rows (stub).
    pub fn set_all_checked(&self, _checked: bool) {}

    /// Get checked state of a row (stub).
    pub fn is_checked(&self, _row: usize) -> bool {
        false
    }

    /// Get the current grid appearance (stub).
    pub fn appearance(&self) -> crate::containers::grid::GridAppearance {
        crate::containers::grid::GridAppearance::default()
    }

    /// Set alternating row colors (matches Win32 API — 2 args).
    pub fn set_alternating_row_colors(
        &self,
        _even: crate::core::geometry::Colour,
        _odd: crate::core::geometry::Colour,
    ) {
    }

    /// Set grid line color (stub).
    pub fn set_grid_line_color(&self, _color: crate::core::geometry::Colour) {}

    /// Set header background color (stub).
    pub fn set_header_bg_color(&self, _color: crate::core::geometry::Colour) {}

    /// Set header foreground color (stub).
    pub fn set_header_fg_color(&self, _color: crate::core::geometry::Colour) {}

    /// Pick a font for the grid (stub, matches Win32 API).
    pub fn pick_font(&self, _frame: &impl crate::core::widget::Window) -> bool {
        false
    }

    /// Get the logical (model) row index for a display row (stub).
    pub fn logical_row(&self, display_row: usize) -> usize {
        display_row
    }

    /// Highlight the selected row (matches Win32 API).
    pub fn highlight_selected_row(&self, _style: crate::containers::grid::GridCellStyle) {}

    /// Get the currently selected row index (stub).
    pub fn get_selected_row(&self) -> Option<usize> {
        None
    }

    /// Set column header height (stub).
    pub fn set_header_height(&self, _height: i32) {}

    /// Set row height (stub).
    pub fn set_row_height(&self, _height: i32) {}

    /// Auto-size columns (stub).
    pub fn auto_size_columns(&self) {}

    /// Set column order (stub).
    pub fn set_column_order(&self, _order: &[usize]) {}

    /// Get column order (stub).
    pub fn column_order(&self) -> Vec<usize> {
        Vec::new()
    }

    /// Delete the currently selected row (stub).
    pub fn delete_selected_row(&self) -> bool {
        false
    }

    /// Clear sort state (stub).
    pub fn clear_sort(&self) {}

    /// Clear style for a single row (stub).
    pub fn clear_row_style(&self, _display_row: usize) {}

    /// Clear all row styles (stub).
    pub fn clear_all_row_styles(&self) {}

    /// Append a row with default values (stub).
    pub fn append_row_default(&mut self) {}

    /// Set cell editor for a specific cell (stub).
    pub fn set_cell_editor(
        &self,
        _row: usize,
        _col: usize,
        _editor: crate::containers::grid_cell_editor::GridCellEditor,
    ) {
    }

    /// Begin cell editing (stub).
    pub fn begin_edit(&self, _row: usize, _col: usize) {}

    /// End cell editing (stub).
    pub fn end_edit(&self, _accept: bool) -> Option<String> {
        None
    }

    /// Get the cell value as a Cell type (stub).
    pub fn get_cell(&self, _row: usize, _col: usize) -> crate::containers::grid::Cell {
        crate::containers::grid::Cell::Empty
    }

    /// Set the cell value as a Cell type (stub).
    pub fn set_cell(&self, _row: usize, _col: usize, _cell: crate::containers::grid::Cell) {}

    /// Append a column with default left alignment.
    pub fn append_column(&self, header: &str, width: i32) {
        self.append_column_with_align(header, width, crate::containers::grid::ColumnAlign::Left);
    }
}

pub use crate::core::widget::Window;

// ============================================================================
// WndProc subclass
// ============================================================================

unsafe extern "system" fn grid_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RefCell<GridState>;
    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    if msg == NMHDR_WM_NOTIFY {
        let hdr = lparam as *const NMHDR;
        if !hdr.is_null() && (*hdr).code == LVN_GETDISPINFO {
            let info = hdr as *const NMLVDISPINFO;
            if !info.is_null() {
                let row = (*info).item.iItem as usize;
                let col = (*info).item.iSubItem as usize;
                if (*info).item.mask & LVIF_TEXT != 0 {
                    let text = {
                        let s = unsafe { (*state_ptr).borrow() };
                        if let Some(p) = s.value_provider.as_ref() {
                            let mut pp = p.borrow_mut();
                            pp(row, col)
                        } else {
                            s.data
                                .get(row)
                                .and_then(|r| r.get(col).cloned())
                                .unwrap_or_default()
                        }
                    };
                    let max = (*info).item.cchTextMax as usize;
                    if max > 0 && !(*info).item.pszText.is_null() {
                        let wide: Vec<u16> =
                            text.encode_utf16().chain(std::iter::once(0)).collect();
                        for (i, &ch) in wide.iter().take(max - 1).enumerate() {
                            *(*info).item.pszText.add(i) = ch;
                        }
                        *(*info).item.pszText.add((max - 1).min(wide.len())) = 0;
                    }
                }
            }
            return 0;
        }
    }
    if msg == WM_NCDESTROY {
        // Reclaim the Rc we created with Rc::into_raw so the
        // GridState is actually dropped (not leaked).
        let entry = unsafe { unregister_subclass(hwnd) };
        if let Some(e) = entry {
            unsafe {
                drop(Rc::from_raw(e.state));
            }
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, e.prev_proc);
            }
            // SAFETY: e.prev_proc was the WndProc that was in place
            // before we subclassed it via SetWindowLongPtrW, so it
            // is a valid `WNDPROC` for this window.
            let prev: WNDPROC = unsafe { std::mem::transmute(e.prev_proc) };
            return unsafe { CallWindowProcW(prev, hwnd, msg, wparam, lparam) };
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================================
// Compatibility types (renderer / editor / icon / cell value / event)
// ============================================================================

pub type GridCellValue = Cell;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CellRendererKind {
    String,
    Number,
    Date,
    Bool,
    Choice,
    Float,
    Bar,
    Badge,
    Priority,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CellEditorKind {
    Text,
    Number,
    Float,
    Bool,
    Choice,
    Date,
}

#[derive(Debug, Clone, Default)]
pub struct GridCellAttr {
    pub background: Option<crate::core::geometry::Colour>,
    pub foreground: Option<crate::core::geometry::Colour>,
    pub alignment: Option<ColumnAlign>,
    pub font_bold: bool,
    pub font_italic: bool,
}

impl GridCellAttr {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_background(mut self, c: crate::core::geometry::Colour) -> Self {
        self.background = Some(c);
        self
    }
    pub fn set_foreground(mut self, c: crate::core::geometry::Colour) -> Self {
        self.foreground = Some(c);
        self
    }
    pub fn set_font_bold(mut self, v: bool) -> Self {
        self.font_bold = v;
        self
    }
    pub fn set_font_italic(mut self, v: bool) -> Self {
        self.font_italic = v;
        self
    }
}

// Re-export the cross-platform GridIcons so that callers get a
// consistent API regardless of whether the `winui3` feature is on.
// The winui3 Grid backend doesn't implement its own icon pipeline
// (it relies on the containers GridIcons), so the alias keeps
// `crate::GridIcons::image_list() -> &ImageList` working for
// `containers::grid::Grid::attach_icons`.
pub use crate::containers::grid_icons::GridIcons;

pub struct GridCellStringRenderer;
impl GridCellStringRenderer {
    pub fn new() -> Self {
        Self
    }
    pub fn render_string(&self, v: &str) -> String {
        v.to_string()
    }
}
impl Default for GridCellStringRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GridCellNumberRenderer;
impl GridCellNumberRenderer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellNumberRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GridCellBoolRenderer;
impl GridCellBoolRenderer {
    pub fn new() -> Self {
        Self
    }
    pub fn render_bool(&self, v: &str) -> String {
        v.to_string()
    }
    pub fn render_bool_bool(&self, v: bool) -> String {
        v.to_string()
    }
}
impl Default for GridCellBoolRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GridCellBarRenderer;
impl Default for GridCellBarRenderer {
    fn default() -> Self {
        Self
    }
}
pub struct GridCellBadgeRenderer;
impl Default for GridCellBadgeRenderer {
    fn default() -> Self {
        Self
    }
}
pub struct GridCellDateRenderer;
impl Default for GridCellDateRenderer {
    fn default() -> Self {
        Self
    }
}
pub struct GridCellPriorityRenderer;
impl Default for GridCellPriorityRenderer {
    fn default() -> Self {
        Self
    }
}

/// Trait for grid cell editors (matches Win32 API).
pub trait GridCellEditorTrait {
    fn kind(&self) -> CellEditorKind;
}

pub struct GridCellTextEditor;
impl GridCellTextEditor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellTextEditor {
    fn default() -> Self {
        Self::new()
    }
}
impl GridCellEditorTrait for GridCellTextEditor {
    fn kind(&self) -> CellEditorKind {
        CellEditorKind::Text
    }
}

pub struct GridCellNumberEditor;
impl GridCellNumberEditor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellNumberEditor {
    fn default() -> Self {
        Self::new()
    }
}
impl GridCellEditorTrait for GridCellNumberEditor {
    fn kind(&self) -> CellEditorKind {
        CellEditorKind::Number
    }
}

pub struct GridCellBoolEditor;
impl GridCellBoolEditor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellBoolEditor {
    fn default() -> Self {
        Self::new()
    }
}
pub struct GridCellFloatEditor;
impl GridCellFloatEditor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellFloatEditor {
    fn default() -> Self {
        Self::new()
    }
}
pub struct GridCellChoiceEditor;
impl GridCellChoiceEditor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellChoiceEditor {
    fn default() -> Self {
        Self::new()
    }
}
pub struct GridCellDateEditor;
impl GridCellDateEditor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GridCellDateEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Cell-edit session (mirrors `containers::grid_cell_editor`).
#[derive(Debug, Clone, Default)]
pub struct GridCellEditor {
    pub row: usize,
    pub col: usize,
    pub text: String,
    pub active: bool,
}
impl GridCellEditor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            text: String::new(),
            active: false,
        }
    }
    pub fn begin_edit(&mut self, initial: &str) {
        self.text = initial.to_string();
        self.active = true;
    }
    pub fn end_edit(&mut self, accept: bool) -> Option<String> {
        self.active = false;
        if accept {
            Some(self.text.clone())
        } else {
            None
        }
    }
}

/// Grid cell renderer struct (matches Win32 API surface).
///
/// In the WinUI3 backend, rendering is handled by the SysListView32
/// control itself — the renderer struct exists for API compatibility
/// and provides text/bool rendering helpers.
pub struct GridCellRenderer;
impl Default for GridCellRenderer {
    fn default() -> Self {
        Self
    }
}
impl GridCellRenderer {
    pub fn new() -> Self {
        Self
    }
    /// Render a `Cell` to its text representation.
    pub fn render_text(&self, cell: &Cell, _rect: crate::core::geometry::Rect) -> String {
        cell_to_string(cell.clone())
    }
    /// Render a boolean value to "Yes"/"No".
    pub fn render_bool(&self, v: bool) -> String {
        if v {
            "Yes".to_string()
        } else {
            "No".to_string()
        }
    }
}

// Re-export GridEvent from core so `cust_table_grid` and other
// examples that import `crate::GridEvent` get a real type.
pub use crate::core::container_events::GridEvent;
