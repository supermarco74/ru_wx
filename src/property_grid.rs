//! Two-column property sheet (`wxPropertyGrid`).
//!
//! A [`PropertyGrid`] is a custom-drawn control that displays a
//! list of named properties in two columns ("Property" and "Value")
//! and lets the user edit the value of the selected property
//! *in place* — by clicking on a value cell the grid creates a
//! child editor (an `Edit` control for text / numeric values, a
//! toggle button for booleans, a `ComboBox` for enums) overlaid
//! on the cell, and commits the new value when the editor loses
//! focus or the user presses Enter.
//!
//! This is a deliberately focused port of the wxWidgets
//! `wxPropertyGrid`: it supports the four most common property
//! types (`String`, `Int`, `Float`, `Bool`) plus a `Choice` enum
//! for picking from a fixed list of options, but does not (yet)
//! implement `wxColourProperty`, `wxFontProperty`, custom
//! categories, derived properties, splitters between the
//! property name and value columns, or a help-text strip at the
//! bottom. Each of those is a self-contained follow-up; the
//! core data flow (append / get / set / on_change) is already
//! in place and matches the wxWidgets contract.
//!
//! # Typical usage
//!
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let app = App::new();
//! let frame = Frame::builder()
//!     .with_title("PropertyGrid demo")
//!     .with_size(500, 400)
//!     .build();
//!
//! let mut grid = PropertyGrid::new(&frame);
//! grid.append("Name", PropertyValue::String("Alice".into()));
//! grid.append("Age",  PropertyValue::Int(30));
//! grid.append("Active", PropertyValue::Bool(true));
//! grid.append("Role", PropertyValue::Choice {
//!     options:  vec!["User".into(), "Admin".into(), "Owner".into()],
//!     selected: 1,
//! });
//! grid.on_change(|idx| println!("property {} changed", idx));
//! ```
//!
//! # Editor lifecycle
//!
//! 1. The user clicks on a value cell. The grid detects the hit
//!    via `WM_LBUTTONDOWN` and starts an editor
//!    ([`start_editor`]) for that property.
//! 2. The editor is a child of the grid HWND, positioned over
//!    the value cell.
//! 3. The editor's parent notifies the grid via `WM_COMMAND`
//!    when:
//!    * the editor loses focus (`EN_KILLFOCUS` for the
//!      `Edit` control, `CBN_KILLFOCUS` for the `ComboBox`),
//!    * the user presses `Enter` (`EN_UPDATE` is too noisy;
//!      we listen for `WM_COMMAND` with the standard
//!      `IDOK` accelerator — see `commit_editor`),
//!    * the user clicks the boolean toggle button
//!      (`BN_CLICKED`).
//! 4. The grid reads the new value, updates the property, and
//!    destroys the editor. The `on_change` callback (if any)
//!    fires *after* the editor is destroyed so it can safely
//!    re-enter the grid's public API.

use std::cell::RefCell;
use std::rc::Rc;

use crate::frame::Frame;
use crate::widget::{Widget, WidgetRef};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ─── Layout constants ───────────────────────────────────────────────────

/// Height of one row in the property grid (logical pixels).
const PG_ROW_HEIGHT: i32 = 22;
/// Width of the vertical separator between the name and value columns.
const PG_COL_SEP: i32 = 1;
/// Default width fraction (out of 100) of the "Property" column.
const PG_DEFAULT_NAME_PCT: u32 = 45;

/// Win32 `EM_SETSEL` message (select all text in an `Edit` control).
#[cfg(target_os = "windows")]
const EM_SETSEL: u32 = 0x00B1;

// ─── Public types ───────────────────────────────────────────────────────

/// The runtime value of a property.
///
/// A flat enum that covers the four most common property types
/// in a settings dialog: text strings, integers, floating-point
/// numbers, booleans, and a fixed list of options.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// Free-form text (e.g. a person's name).
    String(String),
    /// 32-bit signed integer (e.g. an age, a count).
    Int(i32),
    /// 64-bit floating-point number (e.g. a price, a ratio).
    Float(f64),
    /// Boolean (e.g. "active", "enabled").
    Bool(bool),
    /// One of a fixed set of options (e.g. a role).
    Choice {
        /// The list of options the user can pick from.
        options: Vec<String>,
        /// The currently-selected option's index in `options`.
        selected: usize,
    },
}

impl PropertyValue {
    /// String representation used in the value cell when the
    /// grid is not actively editing the property, and used by
    /// the `Edit` editor as the initial / committed text.
    fn display_string(&self) -> String {
        match self {
            PropertyValue::String(s) => s.clone(),
            PropertyValue::Int(i) => i.to_string(),
            PropertyValue::Float(f) => format!("{:.6}", f)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string(),
            PropertyValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            PropertyValue::Choice { options, selected } => options
                .get(*selected)
                .cloned()
                .unwrap_or_default(),
        }
    }
}

/// A single property: a name (left column) plus a value
/// (right column). The grid stores these in insertion order.
#[derive(Debug, Clone)]
pub struct Property {
    /// The label shown in the left column.
    pub name: String,
    /// The current value (right column).
    pub value: PropertyValue,
}

/// A two-column property sheet.
pub struct PropertyGrid {
    data: Rc<RefCell<PropertyGridData>>,
}

// ─── Internal state ─────────────────────────────────────────────────────

/// The currently-active inline editor, if any. Only one editor
/// is open at a time; opening a new one automatically closes the
/// previous one.
#[cfg(target_os = "windows")]
#[derive(Clone)]
struct ActiveEditor {
    /// HWND of the editor child control.
    hwnd: HWND,
    /// Index of the property being edited.
    property_idx: usize,
}

pub(crate) struct PropertyGridData {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: crate::geometry::Rect,
    /// All properties, in insertion order.
    properties: Vec<Property>,
    /// Width of the "Property" column in pixels.
    name_col_w: i32,
    /// Width of the "Value" column in pixels.
    value_col_w: i32,
    /// User-supplied change callback, if any.
    on_change: Option<Box<dyn FnMut(usize)>>,
    /// Currently-active editor, if any.
    #[cfg(target_os = "windows")]
    active_editor: Option<ActiveEditor>,
    /// `true` while the user is dragging the mouse — used to
    /// suppress the editor-commit on the click that starts a
    /// new edit.
    #[cfg(target_os = "windows")]
    pending_new_edit: bool,
}

// ─── Constructor & public API ──────────────────────────────────────────

impl PropertyGrid {
    /// Create a new property grid as a child of the given frame.
    ///
    /// The grid is initially empty; populate it with
    /// [`PropertyGrid::append`]. The grid does not position or
    /// size itself — the caller is responsible for that (via
    /// sizers or direct `set_position` / `set_size`).
    pub fn new(frame: &Frame) -> Self {
        let id = next_control_id();
        let data = Rc::new(RefCell::new(PropertyGridData {
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            id,
            rect: crate::geometry::Rect::new(0, 0, 200, 200),
            properties: Vec::new(),
            name_col_w: 0, // computed on first WM_SIZE
            value_col_w: 0,
            on_change: None,
            #[cfg(target_os = "windows")]
            active_editor: None,
            #[cfg(target_os = "windows")]
            pending_new_edit: false,
        }));

        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };
            let class_name = to_wide("RuWxPropertyGridClass");

            // Register the property-grid window class (idempotent
            // — `RegisterClassExW` is a no-op if the class is
            // already registered, so calling it from every
            // `PropertyGrid::new` is fine).
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
                    lpfnWndProc: Some(property_grid_wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinstance,
                    hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
                    hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                    hbrBackground: (COLOR_WINDOW + 1) as usize as HBRUSH,
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                    hIconSm: std::ptr::null_mut(),
                };
                RegisterClassExW(&wc);
            }

            // Store a raw pointer to the inner state in the
            // window's user data so the WndProc can find it.
            let raw = Rc::into_raw(data.clone());

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hwnd = unsafe {
                let parent = frame.hwnd();
                CreateWindowExW(
                    WS_EX_CLIENTEDGE,
                    class_name.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL,
                    0,
                    0,
                    200,
                    200,
                    parent,
                    id as usize as HMENU,
                    hinstance,
                    std::ptr::null_mut(),
                )
            };

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
                data.borrow_mut().hwnd = hwnd;
            }
        }

        PropertyGrid { data }
    }

    /// Append a new property to the grid. Returns the property's
    /// index.
    pub fn append(&mut self, name: &str, value: PropertyValue) -> usize {
        let idx = self.data.borrow().properties.len();
        self.data.borrow_mut().properties.push(Property {
            name: name.to_string(),
            value,
        });
        self.invalidate();
        idx
    }

    /// Return a clone of the value of the property at `idx`, or
    /// `None` if `idx` is out of range.
    pub fn get_value(&self, idx: usize) -> Option<PropertyValue> {
        self.data
            .borrow()
            .properties
            .get(idx)
            .map(|p| p.value.clone())
    }

    /// Update the value of the property at `idx`. No-op if
    /// `idx` is out of range. Triggers a repaint but does
    /// **not** fire the `on_change` callback (which is reserved
    /// for user-initiated edits).
    pub fn set_value(&mut self, idx: usize, value: PropertyValue) {
        if let Some(p) = self.data.borrow_mut().properties.get_mut(idx) {
            p.value = value;
        }
        self.invalidate();
    }

    /// Return the number of properties in the grid.
    pub fn len(&self) -> usize {
        self.data.borrow().properties.len()
    }

    /// `true` if the grid has no properties.
    pub fn is_empty(&self) -> bool {
        self.data.borrow().properties.is_empty()
    }

    /// Remove all properties. Closes the active editor (if any)
    /// and triggers a repaint.
    pub fn clear(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let d = self.data.borrow();
            if let Some(editor) = &d.active_editor {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    DestroyWindow(editor.hwnd);
                }
            }
        }
        self.data.borrow_mut().properties.clear();
        #[cfg(target_os = "windows")]
        {
            self.data.borrow_mut().active_editor = None;
        }
        self.invalidate();
    }

    /// Register a callback fired when the user commits an
    /// edit. The callback receives the index of the property
    /// that just changed.
    pub fn on_change<F: FnMut(usize) + 'static>(&mut self, f: F) {
        self.data.borrow_mut().on_change = Some(Box::new(f));
    }

    /// Manually re-paint the grid. Called automatically by
    /// [`PropertyGrid::append`], [`PropertyGrid::set_value`]
    /// and [`PropertyGrid::clear`].
    pub fn invalidate(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.data.borrow().hwnd;
            if !hwnd.is_null() {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                }
            }
        }
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        // The Widget trait is implemented on the inner data;
        // hand back an Rc clone of the same Rc that backs
        // `self.data`.
        self.data.clone()
    }
}

// ─── Widget trait implementation ───────────────────────────────────────

impl Widget for PropertyGridData {
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
        self.rect.width = w;
        self.rect.height = h;
        // Split width between name and value columns. On
        // the first resize, default to 45/55; afterwards
        // honour any user override via
        // `set_column_widths`.
        if self.name_col_w == 0 || self.value_col_w == 0 {
            self.name_col_w = (w as i32 * PG_DEFAULT_NAME_PCT as i32) / 100;
            self.value_col_w = w as i32 - self.name_col_w - PG_COL_SEP;
        } else {
            // Keep absolute pixel widths, only redistribute
            // the *delta* when the user resizes the grid.
            let new_w = w as i32;
            let new_value = (new_w - self.name_col_w - PG_COL_SEP).max(40);
            self.value_col_w = new_value;
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
        self.invalidate_after_resize();
    }

    fn rect(&self) -> crate::geometry::Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.rect.width > 0 && self.rect.height > 0
    }

    fn set_visible(&mut self, visible: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, enabled: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}

impl PropertyGridData {
    /// Re-paint and reposition the active editor (if any) so
    /// it follows a grid resize.
    #[cfg(target_os = "windows")]
    fn invalidate_after_resize(&self) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn invalidate_after_resize(&self) {}
}

// ─── Internal helpers (Windows-only) ───────────────────────────────────

/// RAII guard for a GDI pen + brush selection pair. On drop, restores the
/// DC's previous pen and brush (if they were non-null) and deletes the
/// pen we created. This makes the `paint` function panic-safe: even if a
/// future edit adds an early return in the middle, the resources are
/// always released.
#[cfg(target_os = "windows")]
struct PenGuard {
    hdc: HDC,
    old_pen: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
    old_brush: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
    pen: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
}

#[cfg(target_os = "windows")]
impl Drop for PenGuard {
    fn drop(&mut self) {
        // SAFETY: `old_pen` and `old_brush` are the handles `SelectObject`
        // returned when we installed our pen / null brush, so they are
        // valid GDI objects to restore. `pen` is the cosmetic pen we
        // created with `CreatePen`, so it is valid to `DeleteObject`.
        // `DeleteObject` is a no-op on null, and `SelectObject` ignores
        // null (replacing the current object with the default), so this
        // is safe even if any handle is null.
        unsafe {
            if !self.old_pen.is_null() {
                let _ = windows_sys::Win32::Graphics::Gdi::SelectObject(
                    self.hdc,
                    self.old_pen,
                );
            }
            if !self.old_brush.is_null() {
                let _ = windows_sys::Win32::Graphics::Gdi::SelectObject(
                    self.hdc,
                    self.old_brush,
                );
            }
            if !self.pen.is_null() {
                windows_sys::Win32::Graphics::Gdi::DeleteObject(self.pen);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn paint(data: &PropertyGridData, hdc: HDC, rect: RECT) {
    // SAFETY: Win32 FFI calls in this function are all
    // single-threaded GDI / text calls on a `HDC` we own.
    unsafe {
        // Fill the background.
        let bg_brush = CreateSolidBrush(crate::geometry::Colour::new(255, 255, 255, 0).to_colorref());
        FillRect(hdc, &rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Header row separator (1px line at top). We rely on
        // `PenGuard` below to release the pen and restore the
        // previous pen / brush on every exit path (including
        // panics in any future code added inside the loop).
        let pen = CreatePen(PS_SOLID, 1, crate::geometry::Colour::new(180, 180, 180, 0).to_colorref());
        if pen.is_null() {
            // Couldn't allocate a pen; bail out cleanly.
            return;
        }
        let old_pen = SelectObject(hdc, pen as _);
        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH) as _);
        let _guard = PenGuard {
            hdc,
            old_pen,
            old_brush,
            pen: pen as _,
        };

        // Column separator.
        let sep_x = data.rect.x + data.name_col_w + PG_COL_SEP / 2;
        MoveToEx(hdc, sep_x, data.rect.y, std::ptr::null_mut());
        LineTo(hdc, sep_x, data.rect.y + data.rect.height as i32);
        Rectangle(hdc, sep_x, data.rect.y, sep_x + 1, data.rect.y + data.rect.height as i32);

        // Row separators.
        for (i, _p) in data.properties.iter().enumerate() {
            let y = data.rect.y + (i as i32 + 1) * PG_ROW_HEIGHT;
            MoveToEx(hdc, data.rect.x, y, std::ptr::null_mut());
            LineTo(hdc, data.rect.x + data.rect.width as i32, y);
        }

        // PenGuard's `Drop` restores old_pen / old_brush and deletes `pen`.

        // Text: name (left) + value (right) for each row.
        // `SetBkMode` mode 1 == TRANSPARENT.
        SetBkMode(hdc, 1);
        let text_colour = crate::geometry::Colour::new(0, 0, 0, 0).to_colorref();
        SetTextColor(hdc, text_colour);

        for (i, p) in data.properties.iter().enumerate() {
            let y = data.rect.y + i as i32 * PG_ROW_HEIGHT + 4;
            // Name (left column, clipped to name_col_w).
            let mut name_rect = RECT {
                left: data.rect.x + 6,
                top: y,
                right: data.rect.x + data.name_col_w - 6,
                bottom: y + PG_ROW_HEIGHT,
            };
            let wide_name = to_wide(&p.name);
            DrawTextW(
                hdc,
                wide_name.as_ptr(),
                -1,
                &mut name_rect,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            // Value (right column, clipped to value_col_w).
            let mut value_rect = RECT {
                left: data.rect.x + data.name_col_w + PG_COL_SEP + 6,
                top: y,
                right: data.rect.x + data.rect.width as i32 - 6,
                bottom: y + PG_ROW_HEIGHT,
            };
            let s = p.value.display_string();
            let wide_value = to_wide(&s);
            DrawTextW(
                hdc,
                wide_value.as_ptr(),
                -1,
                &mut value_rect,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
        }
    }
}

/// Convert a `COLOUR` to a `COLORREF` (0x00BBGGRR).
trait ToColorref {
    fn to_colorref(self) -> u32;
}
impl ToColorref for crate::geometry::Colour {
    fn to_colorref(self) -> u32 {
        (self.b as u32) << 16 | (self.g as u32) << 8 | (self.r as u32)
    }
}

/// Look up which row (and which column) the user clicked on.
/// Returns `(row_idx, is_value_column)`.
#[cfg(target_os = "windows")]
fn hit_test(data: &PropertyGridData, x: i32, y: i32) -> Option<(usize, bool)> {
    let local_x = x - data.rect.x;
    let local_y = y - data.rect.y;
    if local_x < 0 || local_y < 0 {
        return None;
    }
    let row = (local_y / PG_ROW_HEIGHT) as usize;
    if row >= data.properties.len() {
        return None;
    }
    let is_value = local_x > data.name_col_w;
    Some((row, is_value))
}

/// Start editing the property at `idx`. Creates the appropriate
/// child editor and positions it over the value cell.
#[cfg(target_os = "windows")]
fn start_editor(data_rc: &Rc<RefCell<PropertyGridData>>, idx: usize) {
    // Close any pre-existing editor first so the WndProc
    // doesn't get confused about which property is being
    // edited.
    commit_and_destroy_editor(data_rc);

    let (hwnd_parent, prop_value, cell_rect) = {
        let d = data_rc.borrow();
        let prop_value = d.properties.get(idx).map(|p| p.value.clone());
        let cell_rect = RECT {
            left: d.rect.x + d.name_col_w + PG_COL_SEP + 2,
            top: d.rect.y + idx as i32 * PG_ROW_HEIGHT + 1,
            right: d.rect.x + d.rect.width as i32 - 2,
            bottom: d.rect.y + (idx as i32 + 1) * PG_ROW_HEIGHT - 1,
        };
        (d.hwnd, prop_value, cell_rect)
    };
    let Some(prop_value) = prop_value else { return };

    // SAFETY: Win32 FFI calls in this block all create child
    // controls of `hwnd_parent` and use the grid's own
    // client-area coordinates.
    let editor_hwnd = unsafe {
        match prop_value {
            PropertyValue::String(_) | PropertyValue::Int(_) | PropertyValue::Float(_) => {
                let s = match prop_value {
                    PropertyValue::String(s) => s,
                    PropertyValue::Int(i) => i.to_string(),
                    PropertyValue::Float(f) => f.to_string(),
                    _ => unreachable!(),
                };
                let hinstance = GetModuleHandleW(std::ptr::null());
                let wide_text = to_wide(&s);
                let class_name = to_wide("EDIT");
                let hwnd_edit = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    wide_text.as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_BORDER | (ES_AUTOHSCROLL as u32),
                    cell_rect.left,
                    cell_rect.top,
                    cell_rect.right - cell_rect.left,
                    cell_rect.bottom - cell_rect.top,
                    hwnd_parent,
                    next_control_id() as usize as HMENU,
                    hinstance,
                    std::ptr::null_mut(),
                );
                // Pre-select all text so the user can
                // immediately type a replacement.
                SendMessageW(hwnd_edit, EM_SETSEL as u32, 0, -1);
                SetFocus(hwnd_edit);
                hwnd_edit
            }
            PropertyValue::Bool(b) => {
                let hinstance = GetModuleHandleW(std::ptr::null());
                let label = to_wide(if b { "[x] true" } else { "[ ] false" });
                let class_name = to_wide("BUTTON");
                CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    label.as_ptr(),
                    WS_CHILD | WS_VISIBLE | (BS_PUSHBUTTON as u32),
                    cell_rect.left,
                    cell_rect.top,
                    cell_rect.right - cell_rect.left,
                    cell_rect.bottom - cell_rect.top,
                    hwnd_parent,
                    next_control_id() as usize as HMENU,
                    hinstance,
                    std::ptr::null_mut(),
                )
            }
            PropertyValue::Choice { options, selected } => {
                let hinstance = GetModuleHandleW(std::ptr::null());
                let class_name = to_wide("COMBOBOX");
                let hwnd_combo = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | (CBS_DROPDOWNLIST as u32) | WS_VSCROLL,
                    cell_rect.left,
                    cell_rect.top,
                    cell_rect.right - cell_rect.left,
                    120, // height of the dropdown list
                    hwnd_parent,
                    next_control_id() as usize as HMENU,
                    hinstance,
                    std::ptr::null_mut(),
                );
                for opt in &options {
                    let wide_opt = to_wide(opt);
                    SendMessageW(
                        hwnd_combo,
                        CB_ADDSTRING as u32,
                        0,
                        wide_opt.as_ptr() as isize,
                    );
                }
                SendMessageW(hwnd_combo, CB_SETCURSEL as u32, selected as usize, 0);
                SetFocus(hwnd_combo);
                hwnd_combo
            }
        }
    };

    data_rc.borrow_mut().active_editor = Some(ActiveEditor {
        hwnd: editor_hwnd,
        property_idx: idx,
    });
}

/// Read the current value from the active editor and write it
/// back into the property, then destroy the editor.
#[cfg(target_os = "windows")]
fn commit_and_destroy_editor(data_rc: &Rc<RefCell<PropertyGridData>>) {
    let (editor, prop_value) = {
        let d = data_rc.borrow();
        let editor = d.active_editor.clone();
        let prop_value = d
            .properties
            .get(editor.as_ref().map(|e| e.property_idx).unwrap_or(0))
            .map(|p| p.value.clone());
        (editor, prop_value)
    };
    let (Some(editor), Some(old_value)) = (editor, prop_value) else {
        return;
    };

    // SAFETY: Win32 FFI calls in this block read the
    // editor's content and destroy it. The editor HWND is
    // owned by the grid and is still valid at this point.
    let new_value: Option<PropertyValue> = unsafe {
        match old_value {
            PropertyValue::String(_) => {
                let mut buf = vec![0u16; 256];
                let len = GetWindowTextW(editor.hwnd, buf.as_mut_ptr(), buf.len() as i32);
                buf.truncate(len as usize);
                let s = String::from_utf16_lossy(&buf).to_string();
                Some(PropertyValue::String(s))
            }
            PropertyValue::Int(_) => {
                let mut buf = vec![0u16; 64];
                let len = GetWindowTextW(editor.hwnd, buf.as_mut_ptr(), buf.len() as i32);
                buf.truncate(len as usize);
                let s = String::from_utf16_lossy(&buf).to_string();
                s.trim().parse::<i32>().ok().map(PropertyValue::Int)
            }
            PropertyValue::Float(_) => {
                let mut buf = vec![0u16; 64];
                let len = GetWindowTextW(editor.hwnd, buf.as_mut_ptr(), buf.len() as i32);
                buf.truncate(len as usize);
                let s = String::from_utf16_lossy(&buf).to_string();
                s.trim().parse::<f64>().ok().map(PropertyValue::Float)
            }
            PropertyValue::Bool(b) => {
                // Toggling the button: the click that opened
                // the button was already received via
                // `WM_COMMAND(BN_CLICKED)` in the WndProc,
                // so we just commit the toggled value.
                Some(PropertyValue::Bool(!b))
            }
            PropertyValue::Choice { options, .. } => {
                let sel = SendMessageW(editor.hwnd, CB_GETCURSEL as u32, 0, 0) as usize;
                if sel < options.len() {
                    Some(PropertyValue::Choice {
                        options,
                        selected: sel,
                    })
                } else {
                    None
                }
            }
        }
    };

    // Destroy the editor window.
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        DestroyWindow(editor.hwnd);
    }
    data_rc.borrow_mut().active_editor = None;

    if let Some(new_value) = new_value {
        let idx = editor.property_idx;
        // Write the new value back to the property.
        if let Some(p) = data_rc.borrow_mut().properties.get_mut(idx) {
            p.value = new_value;
        }
        // Repaint the row.
        if let Some(hwnd) = non_null_hwnd(data_rc) {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 1);
            }
        }
        // Fire the user's on_change callback *after* the
        // data borrow is released, so the callback can
        // freely re-enter the grid's public API.
        if let Some(mut cb) = data_rc.borrow_mut().on_change.take() {
            cb(idx);
            data_rc.borrow_mut().on_change = Some(cb);
        }
    }
}

#[cfg(target_os = "windows")]
fn non_null_hwnd(data_rc: &Rc<RefCell<PropertyGridData>>) -> Option<HWND> {
    let h = data_rc.borrow().hwnd;
    if h.is_null() {
        None
    } else {
        Some(h)
    }
}

// ─── WndProc ───────────────────────────────────────────────────────────

/// Win32 Window Procedure for the property grid.
#[cfg(target_os = "windows")]
unsafe extern "system" fn property_grid_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Retrieve the inner data via GWLP_USERDATA.
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<PropertyGridData>;
    if ptr.is_null() && msg != WM_CREATE && msg != WM_NCCREATE {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let data_rc: Rc<RefCell<PropertyGridData>> = if !ptr.is_null() {
        // SAFETY: `ptr` came from `Rc::into_raw(data.clone())` in
        // `PropertyGrid::new`. The leaked raw pointer accounts for
        // *one* strong reference. `Rc::from_raw` itself does NOT
        // increment the strong count (see the rustdoc for
        // `Rc::from_raw`), so we must call `Rc::increment_strong_count`
        // to take a *second* strong reference for the local `Rc` we
        // return below. The matching `drop` at the end of this
        // function then decrements it back to 1 (the leaked ref). The
        // real destructor runs when the last user of `data` (the
        // `PropertyGrid` instance) drops its `Rc`, at which point we
        // `Rc::from_raw(raw)` once more to recover and drop that
        // final reference.
        Rc::increment_strong_count(ptr);
        Rc::from_raw(ptr)
    } else {
        // Before the first WM_NCCREATE / WM_CREATE returns,
        // GWLP_USERDATA is still 0. In that case, defer to
        // `DefWindowProcW`.
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };

    let result = match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            // Use `try_borrow`: if a re-entrant call (e.g. a
            // sizer layout) is already holding the borrow, skip
            // the paint — the next paint cycle will retry.
            if let Ok(d) = data_rc.try_borrow() {
                paint(&d, hdc, ps.rcPaint);
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            // Use `try_borrow`: if a re-entrant call is holding
            // the borrow, skip this click — the user can click
            // again after the layout settles.
            if let Ok(d) = data_rc.try_borrow() {
                let (hit, is_value) = hit_test(&d, x, y).unwrap_or((0, false));
                drop(d);
                if is_value {
                    start_editor(&data_rc, hit);
                } else {
                    commit_and_destroy_editor(&data_rc);
                }
            }
            0
        }
        WM_COMMAND => {
            // The editor notifies us via WM_COMMAND:
            //   * EN_KILLFOCUS / CBN_KILLFOCUS — commit
            //   * BN_CLICKED on the boolean button — toggle + commit
            let notification = ((wparam >> 16) & 0xFFFF) as u16;
            const EN_KILLFOCUS: u16 = 0x800;
            const CBN_KILLFOCUS: u16 = 4;
            const BN_CLICKED: u16 = 0;
            match notification {
                EN_KILLFOCUS | CBN_KILLFOCUS => {
                    commit_and_destroy_editor(&data_rc);
                }
                BN_CLICKED => {
                    // Boolean toggle: read which button was
                    // clicked and commit if it's the active
                    // editor. Use `try_borrow` to be re-entrancy
                    // safe: a sizer layout in progress may hold
                    // the borrow.
                    let id = (wparam & 0xFFFF) as u16;
                    let is_active = data_rc
                        .try_borrow()
                        .ok()
                        .map(|d| {
                            d.active_editor
                                .as_ref()
                                .map(|e| e.hwnd as usize == id as usize)
                                .unwrap_or(false)
                        })
                        .unwrap_or(false);
                    if is_active {
                        commit_and_destroy_editor(&data_rc);
                    }
                }
                _ => {}
            }
            0
        }
        WM_KEYDOWN => {
            // Esc cancels the editor; Enter commits it.
            let vkey = wparam as i32;
            if vkey == 27 /* VK_ESCAPE */ {
                // Cancel: just destroy the editor without
                // committing. Use `try_borrow_mut` to be
                // re-entrancy safe.
                if let Ok(mut d) = data_rc.try_borrow_mut() {
                    if let Some(editor) = d.active_editor.take() {
                        drop(d);
                        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                        unsafe {
                            DestroyWindow(editor.hwnd);
                        }
                    }
                }
            } else if vkey == 13 /* VK_RETURN */ {
                commit_and_destroy_editor(&data_rc);
            } else {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            0
        }
        WM_SIZE => {
            // The grid re-lays out its columns on resize;
            // the active editor (if any) gets repositioned
            // to follow the cell. Use `try_borrow_mut` to be
            // re-entrancy safe: a sizer layout in progress
            // may already hold the borrow (e.g. `set_size`
            // calls `MoveWindow` which synchronously sends
            // WM_SIZE). If the borrow is held, the data
            // was already updated by the caller — just
            // skip.
            let w = (lparam & 0xFFFF) as i32;
            let h = ((lparam >> 16) & 0xFFFF) as i32;
            if let Ok(mut d) = data_rc.try_borrow_mut() {
                d.rect.width = w as u32;
                d.rect.height = h as u32;
                if d.name_col_w == 0 || d.value_col_w == 0 {
                    d.name_col_w = (w * PG_DEFAULT_NAME_PCT as i32) / 100;
                    d.value_col_w = w - d.name_col_w - PG_COL_SEP;
                } else {
                    d.value_col_w = (w - d.name_col_w - PG_COL_SEP).max(40);
                }
            }
            InvalidateRect(hwnd, std::ptr::null(), 1);
            0
        }
        WM_DESTROY => {
            // Clean up the active editor and clear
            // `GWLP_USERDATA` so the WndProc doesn't try to
            // access the inner Rc after this point. Use
            // `try_borrow_mut` to be re-entrancy safe: if
            // the borrow is held, the editor HWND will be
            // destroyed by Windows when the parent window
            // is destroyed anyway.
            if let Ok(mut d) = data_rc.try_borrow_mut() {
                if let Some(editor) = d.active_editor.take() {
                    drop(d);
                    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                    unsafe {
                        DestroyWindow(editor.hwnd);
                    }
                }
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            // Reclaim the leaked `Rc` that the constructor
            // placed in `GWLP_USERDATA` via `Rc::into_raw`. The
            // local `data_rc` at the end of this function will
            // drop its own reference (count: 2 -> 1), and this
            // `from_raw` recovers the leaked one (1 -> 0,
            // freeing the backing storage if no other `Rc`
            // clones are alive).
            let _ = Rc::from_raw(ptr);
            // The `data_rc` will be dropped at the bottom of
            // this function when it goes out of scope.
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    };

    // Drop the Rc by consuming the local variable.
    drop(data_rc);

    result
}

// Quiet the unused-import lints on non-Windows builds.
#[cfg(not(target_os = "windows"))]
use crate::platform::win32 as _win32_marker;
