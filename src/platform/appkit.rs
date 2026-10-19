//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Native AppKit backend for macOS — real `NSWindow` / `NSView` widgets
//! wired into the unified `Frame` / `Button` / `StaticText` API.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly, Message};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle, NSBitmapFormat,
    NSBitmapImageRep, NSBorderType, NSButton, NSComboBox, NSControl, NSControlStateValueOff,
    NSControlStateValueOn, NSControlTextEditingDelegate, NSEvent, NSFont, NSFontManager, NSImage,
    NSImageScaling, NSImageView, NSMenu, NSMenuItem, NSPopUpButton, NSProgressIndicator, NSScreen,
    NSScrollView, NSSecureTextField, NSSlider, NSStepper, NSTabView, NSTabViewDelegate,
    NSTabViewItem, NSTabViewType, NSTextAlignment, NSTextDelegate, NSTextField,
    NSTextFieldBezelStyle, NSTextFieldDelegate, NSTextView, NSTextViewDelegate, NSView, NSWindow,
    NSWindowDelegate, NSWindowStyleMask,
};
use objc2_app_kit::{
    NSCalibratedRGBColorSpace, NSCellImagePosition, NSColor, NSControlSize, NSMutableParagraphStyle,
};
use objc2_foundation::{
    MainThreadMarker, NSCopying, NSDate, NSDefaultRunLoopMode, NSNotification, NSObject,
    NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize, NSString, NSTimer,
};

use crate::core::geometry::{Colour, Rect};
use crate::platform::stub_backend::alloc_widget_handle;
use crate::platform::NativeHandle;
use crate::window::frame::Frame;

use crate::window::frame::FrameData;
use crate::window::menu::MenuItemKind;

// ── Main-thread view registry (AppKit is main-thread only) ──────────

/// Alias for the deeply nested registry value types used by the
/// `thread_local!` tables below, keeping the declarations readable.
type TabSelectionHandler = Box<dyn FnMut(usize)>;
type PopupSelectionHandler = Box<dyn FnMut()>;
type SpinParts = (
    Retained<NSTextField>,
    Retained<NSStepper>,
    Retained<SpinStepperTarget>,
);
type ListCtrlColumns = RefCell<Vec<(String, i32)>>;
type ListCtrlRows = RefCell<Vec<(Vec<String>, Option<i32>)>>;
type ListCtrlSelected = RefCell<Vec<usize>>;
type ListCtrlVirtualFetch = Box<dyn FnMut(usize, usize) -> String>;
type ListCtrlCellStyles = RefCell<HashMap<(usize, usize), GridStyleData>>;
type GridColumns = RefCell<Vec<(String, i32, i32)>>;
type GridRows = RefCell<Vec<(Vec<String>, Vec<Option<i32>>)>>;
type GridSelected = RefCell<Vec<usize>>;
type GridCellStyles = RefCell<HashMap<(usize, usize), GridStyleData>>;
type GridRowStyles = RefCell<HashMap<usize, GridStyleData>>;
type GridAlternating = RefCell<Option<(Rgba, Rgba)>>;
type GridSortState = RefCell<(Option<usize>, Option<bool>)>;
type GridHighlightClick = (Option<usize>, Option<usize>);
type GridTooltipProvider = Box<dyn Fn(usize, usize) -> Option<String>>;
type GridCellKinds = RefCell<Vec<Vec<Option<GridProgressCell>>>>;

thread_local! {
    static VIEWS: RefCell<HashMap<NativeHandle, Retained<NSView>>> = RefCell::new(HashMap::new());
    static FRAME_CONTENT: RefCell<HashMap<NativeHandle, Retained<NSView>>> =
        RefCell::new(HashMap::new());
    static FRAME_WINDOWS: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
    /// Container views (`Panel`, nested hosts) keyed by their logical handle.
    static CONTAINERS: RefCell<HashMap<NativeHandle, Retained<NSView>>> =
        RefCell::new(HashMap::new());
    static VIEW_PARENT: RefCell<HashMap<NativeHandle, NativeHandle>> = RefCell::new(HashMap::new());
    static FRAME_DATA_WEAK: RefCell<HashMap<NativeHandle, Weak<RefCell<FrameData>>>> =
        RefCell::new(HashMap::new());
    /// `true` when the handle refers to an editable [`NSComboBox`], else [`NSPopUpButton`].
    static POPUP_IS_COMBO: RefCell<HashMap<NativeHandle, bool>> = RefCell::new(HashMap::new());
    /// Ruolo semantico dichiarato dal widget (Livello 2), per view (chiave:
    /// indirizzo della `NSView`). Permette all'inspector di distinguere
    /// checkbox/radio senza `buttonType` (cfr. `PIANO_DEBUG_INSPECTOR.md` §17.1).
    static DECLARED_ROLES: RefCell<HashMap<usize, &'static str>> = RefCell::new(HashMap::new());
    static LISTBOX_ITEMS: RefCell<HashMap<NativeHandle, RefCell<Vec<String>>>> =
        RefCell::new(HashMap::new());
    static LISTBOX_VIEWS: RefCell<HashMap<NativeHandle, Retained<ListBoxView>>> =
        RefCell::new(HashMap::new());
    static LISTBOX_SELECTED: RefCell<HashMap<NativeHandle, RefCell<Vec<usize>>>> =
        RefCell::new(HashMap::new());
    static LISTBOX_MULTI: RefCell<HashMap<NativeHandle, bool>> = RefCell::new(HashMap::new());
    static CHECKLIST_ITEMS: RefCell<HashMap<NativeHandle, RefCell<Vec<String>>>> =
        RefCell::new(HashMap::new());
    static CHECKLIST_CHECKED: RefCell<HashMap<NativeHandle, RefCell<Vec<bool>>>> =
        RefCell::new(HashMap::new());
    static CHECKLIST_SELECTED: RefCell<HashMap<NativeHandle, RefCell<Vec<usize>>>> =
        RefCell::new(HashMap::new());
    static CHECKLIST_LAST_ROW: RefCell<HashMap<NativeHandle, Cell<Option<usize>>>> =
        RefCell::new(HashMap::new());
    static STATUSBAR_LABELS: RefCell<HashMap<NativeHandle, Vec<Retained<NSTextField>>>> =
        RefCell::new(HashMap::new());
    static STATUSBAR_FIELD_COUNT: RefCell<HashMap<NativeHandle, usize>> =
        RefCell::new(HashMap::new());
    static STATUSBAR_PARENT: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    static TAB_VIEWS: RefCell<HashMap<NativeHandle, Retained<NSTabView>>> =
        RefCell::new(HashMap::new());
    static TAB_ITEMS: RefCell<HashMap<NativeHandle, Vec<Retained<NSTabViewItem>>>> =
        RefCell::new(HashMap::new());
    static TAB_PAGE_HANDLES: RefCell<HashMap<NativeHandle, Vec<NativeHandle>>> =
        RefCell::new(HashMap::new());
    static TAB_DELEGATES: RefCell<HashMap<NativeHandle, Retained<TabViewDelegate>>> =
        RefCell::new(HashMap::new());
    static TAB_SELECTION_HANDLERS: RefCell<HashMap<NativeHandle, TabSelectionHandler>> =
        RefCell::new(HashMap::new());
    static POPUP_SELECTION_HANDLERS: RefCell<HashMap<NativeHandle, PopupSelectionHandler>> =
        RefCell::new(HashMap::new());
    static MENU_TARGETS: RefCell<Vec<Retained<ButtonTarget>>> =
        const { RefCell::new(Vec::new()) };
    static SLIDER_PROGRAMMATIC: Cell<u32> = const { Cell::new(0) };
    static SPIN_PARTS: RefCell<HashMap<NativeHandle, SpinParts>> = RefCell::new(HashMap::new());
    static LISTCTRL_COLUMNS: RefCell<HashMap<NativeHandle, ListCtrlColumns>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_ROWS: RefCell<HashMap<NativeHandle, ListCtrlRows>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_SELECTED: RefCell<HashMap<NativeHandle, ListCtrlSelected>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_VIEWS: RefCell<HashMap<NativeHandle, Retained<ListCtrlView>>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_HAS_IMAGES: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static SCROLLED_DOCUMENTS: RefCell<HashMap<NativeHandle, Retained<NSView>>> =
        RefCell::new(HashMap::new());
    static SCROLLED_RESIZE: RefCell<HashMap<NativeHandle, Box<dyn FnMut()>>> =
        RefCell::new(HashMap::new());
    static SCROLLED_RESIZE_PENDING: RefCell<HashSet<NativeHandle>> = RefCell::new(HashSet::new());
    static SCROLLED_RESIZE_SCHEDULED: Cell<bool> = const { Cell::new(false) };
    static IN_DEFERRED_DRAIN: Cell<bool> = const { Cell::new(false) };
    static TIMER_INSTANCES: RefCell<HashMap<u32, Retained<NSTimer>>> =
        RefCell::new(HashMap::new());
    static TIMER_FIRE: RefCell<HashMap<u32, Box<dyn FnMut()>>> = RefCell::new(HashMap::new());
    static TIMER_TARGETS: RefCell<Vec<Retained<TimerTarget>>> =
        const { RefCell::new(Vec::new()) };
    static IMAGE_LISTS: RefCell<HashMap<NativeHandle, ImageListStorage>> =
        RefCell::new(HashMap::new());
    static MENU_ITEM_ICONS: RefCell<HashMap<u16, Retained<NSImage>>> =
        RefCell::new(HashMap::new());
    static MENU_NS_ITEMS: RefCell<HashMap<u16, Retained<NSMenuItem>>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_IMAGE_LISTS: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_VIRTUAL_COUNT: RefCell<HashMap<NativeHandle, u32>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_VIRTUAL_FETCH: RefCell<HashMap<NativeHandle, ListCtrlVirtualFetch>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_EXT_STYLE: RefCell<HashMap<NativeHandle, u32>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_ROW_REORDER_ENABLED: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_ROW_REORDERED: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_ROW_HEIGHTS: RefCell<HashMap<NativeHandle, f64>> = RefCell::new(HashMap::new());
    static LISTCTRL_DROP_HINT: RefCell<HashMap<NativeHandle, Option<usize>>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_LAST_HIT: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_CELL_STYLES: RefCell<HashMap<NativeHandle, ListCtrlCellStyles>> =
        RefCell::new(HashMap::new());
    static LISTCTRL_DEFER_REDRAWS: RefCell<HashSet<NativeHandle>> = RefCell::new(HashSet::new());
    /// Colonna checkbox (selezione multipla) abilitata per ciascun ListCtrl.
    static LISTCTRL_CHECKBOXES_ENABLED: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    /// Indici delle righe marcate col checkbox di selezione multipla.
    static LISTCTRL_CHECKED: RefCell<HashMap<NativeHandle, RefCell<HashSet<usize>>>> =
        RefCell::new(HashMap::new());
    /// Notifiche ListCtrl differite: `(frame, control_id, code, riga)`.
    /// La riga vale `-1` per le notifiche che non ne hanno una (es.
    /// `ITEM_CHANGED`); per `ITEM_DBLCLK` porta l'indice attivato, così
    /// il frame può invocare gli handler di attivazione riga.
    static LISTCTRL_DEFER_NOTIFIES: RefCell<Vec<(NativeHandle, u16, u32, isize)>> =
        const { RefCell::new(Vec::new()) };
    static LISTCTRL_DEFER_TIMER_ARMED: Cell<bool> = const { Cell::new(false) };
    static TAB_IMAGE_LISTS: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    static TAB_PAGE_IMAGE_INDICES: RefCell<HashMap<NativeHandle, Vec<i32>>> =
        RefCell::new(HashMap::new());
    static POPUP_IMAGE_LISTS: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    static TOOLBARS: RefCell<HashMap<NativeHandle, RefCell<ToolbarStorage>>> =
        RefCell::new(HashMap::new());
    static TOOLBARS_BY_FRAME: RefCell<HashMap<NativeHandle, Vec<NativeHandle>>> =
        RefCell::new(HashMap::new());
    static TREE_NODES: RefCell<HashMap<NativeHandle, RefCell<HashMap<usize, TreeNodeData>>>> =
        RefCell::new(HashMap::new());
    static TREE_ROOTS: RefCell<HashMap<NativeHandle, RefCell<Vec<usize>>>> =
        RefCell::new(HashMap::new());
    static TREE_NEXT_ID: RefCell<HashMap<NativeHandle, Cell<usize>>> =
        RefCell::new(HashMap::new());
    static TREE_SELECTED: RefCell<HashMap<NativeHandle, RefCell<Option<usize>>>> =
        RefCell::new(HashMap::new());
    static TREE_IMAGE_LISTS: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    static TREE_VIEWS: RefCell<HashMap<NativeHandle, Retained<TreeView>>> =
        RefCell::new(HashMap::new());
    static TREE_NOTIFY: RefCell<HashMap<NativeHandle, (NativeHandle, u16)>> =
        RefCell::new(HashMap::new());
    static GRID_COLUMNS: RefCell<HashMap<NativeHandle, GridColumns>> =
        RefCell::new(HashMap::new());
    static GRID_ROWS: RefCell<HashMap<NativeHandle, GridRows>> = RefCell::new(HashMap::new());
    static GRID_SELECTED: RefCell<HashMap<NativeHandle, GridSelected>> =
        RefCell::new(HashMap::new());
    static GRID_IMAGE_LISTS: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    static GRID_CHECKBOXES: RefCell<HashMap<NativeHandle, bool>> = RefCell::new(HashMap::new());
    static GRID_ROW_CHECKED: RefCell<HashMap<NativeHandle, RefCell<Vec<bool>>>> =
        RefCell::new(HashMap::new());
    static GRID_APPEARANCE: RefCell<HashMap<NativeHandle, RefCell<GridAppearanceData>>> =
        RefCell::new(HashMap::new());
    static GRID_CELL_STYLES: RefCell<HashMap<NativeHandle, GridCellStyles>> =
        RefCell::new(HashMap::new());
    static GRID_ROW_STYLES: RefCell<HashMap<NativeHandle, GridRowStyles>> =
        RefCell::new(HashMap::new());
    static GRID_ALTERNATING: RefCell<HashMap<NativeHandle, GridAlternating>> =
        RefCell::new(HashMap::new());
    static GRID_FONT: RefCell<HashMap<NativeHandle, RefCell<(String, f64)>>> =
        RefCell::new(HashMap::new());
    static GRID_SORT: RefCell<HashMap<NativeHandle, GridSortState>> =
        RefCell::new(HashMap::new());
    static GRID_HEADER_SORT_CLICK: RefCell<HashMap<NativeHandle, (usize, bool)>> =
        RefCell::new(HashMap::new());
    static GRID_SORT_SYNCING: RefCell<HashMap<NativeHandle, bool>> = RefCell::new(HashMap::new());
    static GRID_HIGHLIGHT_ROW: RefCell<HashMap<NativeHandle, Option<usize>>> =
        RefCell::new(HashMap::new());
    static GRID_HIGHLIGHT_COL: RefCell<HashMap<NativeHandle, Option<usize>>> =
        RefCell::new(HashMap::new());
    static GRID_ROW_COL_HIGHLIGHT: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_HIGHLIGHT_CLICK: RefCell<HashMap<NativeHandle, GridHighlightClick>> =
        RefCell::new(HashMap::new());
    static GRID_ACTIVATE_CELL: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static GRID_SHOW_LINES: RefCell<HashMap<NativeHandle, bool>> = RefCell::new(HashMap::new());
    static GRID_ROW_HEIGHTS: RefCell<HashMap<NativeHandle, f64>> = RefCell::new(HashMap::new());
    static GRID_CONTEXT_MENU: RefCell<HashMap<NativeHandle, (bool, usize, usize)>> =
        RefCell::new(HashMap::new());
    static GRID_COLUMN_RESIZED: RefCell<HashMap<NativeHandle, (usize, i32)>> =
        RefCell::new(HashMap::new());
    static GRID_COLUMN_RESIZE_SYNCING: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_COLUMN_RESIZE_SCHEDULED: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_COLUMN_RESIZE_DIRTY: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_LABEL_TIPS: RefCell<HashMap<NativeHandle, bool>> = RefCell::new(HashMap::new());
    static GRID_AUTOSIZE_COLUMNS: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_TOOLTIP_PROVIDERS: RefCell<HashMap<NativeHandle, GridTooltipProvider>> =
        RefCell::new(HashMap::new());
    static GRID_CELL_KINDS: RefCell<HashMap<NativeHandle, GridCellKinds>> =
        RefCell::new(HashMap::new());
    static GRID_HIDDEN_COLS: RefCell<HashMap<NativeHandle, RefCell<Vec<bool>>>> =
        RefCell::new(HashMap::new());
    static GRID_ROW_LABELS: RefCell<HashMap<NativeHandle, (bool, i32)>> =
        RefCell::new(HashMap::new());
    static GRID_FOCUSED_CELL: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static GRID_CELL_EDITING: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_CELL_EDITED: RefCell<HashMap<NativeHandle, (usize, usize, String)>> =
        RefCell::new(HashMap::new());
    static GRID_PASTE_AT: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static GRID_EDIT_START: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static GRID_LINK_CLICK: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static GRID_LINK_CLICKS: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_FROZEN_COL_COUNT: RefCell<HashMap<NativeHandle, usize>> =
        RefCell::new(HashMap::new());
    static GRID_FROZEN_ROW_COUNT: RefCell<HashMap<NativeHandle, usize>> =
        RefCell::new(HashMap::new());
    static GRID_ROW_DRAG_ENABLED: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
    static GRID_ROW_REORDERED: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    static DEFERRED_MAIN: RefCell<Vec<Box<dyn FnMut()>>> =
        RefCell::new(const { Vec::new() });
}

struct ImageListStorage {
    width: i32,
    height: i32,
    images: Vec<Retained<NSImage>>,
}

type Rgba = (u8, u8, u8, u8);

#[derive(Clone, Copy)]
pub(super) struct GridStyleData {
    foreground: Option<Rgba>,
    background: Option<Rgba>,
}

#[derive(Clone, Copy)]
pub struct GridAppearanceData {
    pub system_theme: bool,
    pub alternating_even: Rgba,
    pub alternating_odd: Rgba,
    pub default_text: Rgba,
    pub header_foreground: Rgba,
    pub header_background: Rgba,
    pub selection_foreground: Rgba,
    pub selection_background: Rgba,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridNativeKind {
    #[default]
    Progress,
    Bar,
    Priority,
}

#[derive(Clone, Debug)]
pub struct GridProgressCell {
    pub value: u32,
    pub max: u32,
    pub label: Option<String>,
    pub kind: GridNativeKind,
}

pub(super) fn grid_init_cell_kinds(grid_handle: NativeHandle) {
    GRID_CELL_KINDS.with(|map| {
        map.borrow_mut()
            .insert(grid_handle, RefCell::new(Vec::new()));
    });
}

pub(super) fn grid_append_column_kind_slots(grid_handle: NativeHandle) {
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            for row in kinds.borrow_mut().iter_mut() {
                row.push(None);
            }
        }
    });
}

fn grid_ensure_kind_row(grid_handle: NativeHandle, row: usize, col_count: usize) {
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            let mut vec = kinds.borrow_mut();
            if row >= vec.len() {
                vec.resize_with(row + 1, || vec![None; col_count]);
            }
            if vec[row].len() < col_count {
                vec[row].resize(col_count, None);
            }
        }
    });
}

pub(super) fn grid_cell_progress_kind(
    handle: NativeHandle,
    row: usize,
    col: usize,
) -> Option<GridProgressCell> {
    GRID_CELL_KINDS.with(|map| {
        map.borrow().get(&handle).and_then(|kinds| {
            kinds
                .borrow()
                .get(row)
                .and_then(|r| r.get(col))
                .and_then(|opt| opt.clone())
        })
    })
}

fn grid_set_cell_progress_kind(
    grid_handle: NativeHandle,
    row: usize,
    col: usize,
    kind: Option<GridProgressCell>,
) {
    let col_count = grid_col_count(grid_handle).max(col + 1);
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            grid_ensure_kind_row(grid_handle, row, col_count);
            let mut vec = kinds.borrow_mut();
            if row < vec.len() && col < vec[row].len() {
                vec[row][col] = kind;
            }
        }
    });
}

impl Default for GridAppearanceData {
    fn default() -> Self {
        Self {
            system_theme: true,
            alternating_even: (255, 255, 255, 255),
            alternating_odd: (255, 255, 255, 255),
            default_text: (0, 0, 0, 255),
            header_foreground: (0, 0, 0, 255),
            header_background: (246, 246, 246, 255),
            selection_foreground: (255, 255, 255, 255),
            selection_background: (0, 120, 215, 255),
        }
    }
}

struct ToolbarStorage {
    /// Outer container sized to the dock edge (or floating window client).
    host: Retained<NSView>,
    /// Inner row holding the gripper and tool buttons.
    strip: Retained<NSView>,
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    image_list_handle: NativeHandle,
    image_w: i32,
    image_h: i32,
    pending: Vec<ToolbarPendingItem>,
    with_gripper: bool,
    gripper_id: u16,
    height: f64,
    floating_window: Cell<NativeHandle>,
    /// Last dock edge when not floating ([`TOOLBAR_DOCK_*`] constants).
    dock_side: Cell<u8>,
    /// Content width of the tool row (gripper + buttons), set by [`toolbar_realize`].
    content_width: Cell<f64>,
    /// When true, top-docked toolbars prefer [`NSToolbar`] on the frame window.
    prefer_native: bool,
    /// When true, [`AuiManager`] owns the toolbar host frame; do not re-dock to `parent_handle`.
    managed_by_aui: Cell<bool>,
}

/// Docked to the top edge of the parent content view.
pub const TOOLBAR_DOCK_TOP: u8 = 0;
/// Docked to the bottom edge.
pub const TOOLBAR_DOCK_BOTTOM: u8 = 1;
/// Docked to the left edge (horizontal strip, full width — matches Win32).
pub const TOOLBAR_DOCK_LEFT: u8 = 2;
/// Docked to the right edge (200 px wide strip at the top-right — matches Win32).
pub const TOOLBAR_DOCK_RIGHT: u8 = 3;

#[derive(Clone)]
enum ToolbarPendingItem {
    Separator,
    Tool {
        id: u16,
        image_index: i32,
        /// Testo del tooltip mostrato all'hover (su macOS: `NSView.setToolTip`).
        tooltip: String,
    },
}

#[derive(Clone)]
struct TreeNodeData {
    text: String,
    image_index: Option<i32>,
    parent: Option<usize>,
    children: Vec<usize>,
    expanded: bool,
}

pub fn register_frame_weak(handle: NativeHandle, frame: &Frame) {
    FRAME_DATA_WEAK.with(|map| {
        map.borrow_mut().insert(handle, Rc::downgrade(&frame.inner));
    });
}

fn record_parent(child: NativeHandle, parent: NativeHandle) {
    VIEW_PARENT.with(|map| {
        map.borrow_mut().insert(child, parent);
    });
}

pub(super) fn resolve_frame_handle(mut handle: NativeHandle) -> NativeHandle {
    for _ in 0..64 {
        if FRAME_CONTENT.with(|map| map.borrow().contains_key(&handle)) {
            return handle;
        }
        if let Some(parent) = VIEW_PARENT.with(|map| map.borrow().get(&handle).copied()) {
            handle = parent;
        } else {
            break;
        }
    }
    handle
}

pub(super) fn dispatch_command(frame_handle: NativeHandle, command_id: u16) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let handler = data.borrow_mut().command_handlers.remove(&command_id);
        if let Some(mut handler) = handler {
            handler();
            data.borrow_mut()
                .command_handlers
                .insert(command_id, handler);
        }
    }
}

fn dispatch_command_notify(frame_handle: NativeHandle, command_id: u16, notify_code: u16) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let key = (command_id, notify_code);
        let handler = data.borrow_mut().command_notify_handlers.remove(&key);
        if let Some(mut handler) = handler {
            handler();
            data.borrow_mut()
                .command_notify_handlers
                .insert(key, handler);
        }
    }
}

fn dispatch_notify(frame_handle: NativeHandle, control_id: u16, code: u32) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let handler = data.borrow_mut().notify_handlers.remove(&control_id);
        if let Some(mut handler) = handler {
            handler(code);
            data.borrow_mut()
                .notify_handlers
                .insert(control_id, handler);
        }
    }
}

pub(super) fn dispatch_scroll(
    frame_handle: NativeHandle,
    scroll_handle: NativeHandle,
    code: u16,
    pos: i32,
) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let handler = data.borrow_mut().scroll_handlers.remove(&scroll_handle);
        if let Some(mut handler) = handler {
            handler(code, pos);
            data.borrow_mut()
                .scroll_handlers
                .insert(scroll_handle, handler);
        }
    }
}

fn dispatch_frame_on_close(frame_handle: NativeHandle) {
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        // Preleva `on_close` in un borrow separato e breve: il
        // temporaneo di una testa `if let` resta vivo per tutto il
        // blocco, quindi un `borrow_mut()` dentro verrebbe rifiutato
        // ("RefCell already borrowed"). Il callback inoltre può a sua
        // volta accedere alla RefCell, quindi va chiamato senza borrow
        // attivo.
        let mut on_close = {
            let mut borrow = data.borrow_mut();
            borrow.host.visible = false;
            borrow.on_close.take()
        };
        if let Some(mut cb) = on_close.take() {
            cb();
            data.borrow_mut().on_close = Some(cb);
        }
    }
}

fn dispatch_frame_resize(frame_handle: NativeHandle, width: u32, height: u32) {
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let mut callbacks = std::mem::take(&mut data.borrow_mut().on_resize);
        for cb in callbacks.iter_mut() {
            cb(width, height);
        }
        data.borrow_mut().on_resize = callbacks;
    }
    TOOLBARS_BY_FRAME.with(|map| {
        if let Some(handles) = map.borrow().get(&frame_handle).cloned() {
            for tb in handles {
                toolbar_relayout_docked(tb);
            }
        }
    });
}

fn dispatch_frame_sizer_layout(frame_handle: NativeHandle, width: u32, height: u32) {
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        data.borrow_mut().layout_client_area(width, height);
    }
    TOOLBARS_BY_FRAME.with(|map| {
        if let Some(handles) = map.borrow().get(&frame_handle).cloned() {
            for tb in handles {
                toolbar_relayout_docked(tb);
            }
        }
    });
}

// ── Window delegate ─────────────────────────────────────────────────

#[derive(Default)]
struct WindowDelegateIvars {
    running: std::rc::Rc<Cell<bool>>,
    frame_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = WindowDelegateIvars]
    struct WindowDelegate;

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            dispatch_frame_on_close(self.ivars().frame_handle.get());
            self.ivars().running.set(false);
        }

        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, notification: &NSNotification) {
            let Some(obj) = notification.object() else {
                return;
            };
            let Some(window) = obj.downcast_ref::<NSWindow>() else {
                return;
            };
            if let Some(content) = window.contentView() {
                let size = content.frame().size;
                let w = size.width.max(0.0) as u32;
                let h = size.height.max(0.0) as u32;
                dispatch_frame_sizer_layout(self.ivars().frame_handle.get(), w, h);
                dispatch_frame_resize(self.ivars().frame_handle.get(), w, h);
            }
        }
    }
);

impl WindowDelegate {
    fn new(
        mtm: MainThreadMarker,
        running: std::rc::Rc<Cell<bool>>,
        frame_handle: NativeHandle,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(WindowDelegateIvars {
            running,
            frame_handle: Cell::new(frame_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ── Button target ───────────────────────────────────────────────────

#[derive(Default)]
pub(super) struct ButtonTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonTargetIvars]
    pub(super) struct ButtonTarget;

    unsafe impl NSObjectProtocol for ButtonTarget {}

    impl ButtonTarget {
        #[unsafe(method(ruWxButtonClicked:))]
        fn button_clicked(&self, sender: Option<&AnyObject>) {
            if SLIDER_PROGRAMMATIC.with(|c| c.get() > 0)
                && sender
                    .and_then(|s| s.downcast_ref::<NSSlider>())
                    .is_some()
                {
                    return;
                }
            // Debug: log button click with coordinates
            if let Some(btn) = sender.and_then(|s| s.downcast_ref::<NSButton>()) {
                let frame = btn.frame();
                let title = btn.title();
                eprintln!("[CLICK] Button clicked: title='{}', frame=({:.1},{:.1},{:.1},{:.1})",
                    title, frame.origin.x, frame.origin.y, frame.size.width, frame.size.height);
            }
            let frame_handle = self.ivars().frame_handle.get();
            let command_id = self.ivars().command_id.get();
            menu_item_will_dispatch(frame_handle, command_id);
            dispatch_command(frame_handle, command_id);
        }
    }
);

impl ButtonTarget {
    fn new(mtm: MainThreadMarker, frame_handle: NativeHandle, command_id: u16) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ButtonTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) fn button_target_new(
    mtm: MainThreadMarker,
    frame_handle: NativeHandle,
    command_id: u16,
) -> Retained<ButtonTarget> {
    ButtonTarget::new(mtm, frame_handle, command_id)
}

// ── Text field change delegate (SearchCtrl / TextCtrl on macOS) ─────

thread_local! {
    static TEXT_FIELD_DELEGATES: RefCell<Vec<Retained<TextFieldChangeTarget>>> =
        const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
struct TextFieldChangeTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TextFieldChangeTargetIvars]
    struct TextFieldChangeTarget;

    unsafe impl NSObjectProtocol for TextFieldChangeTarget {}

    unsafe impl NSControlTextEditingDelegate for TextFieldChangeTarget {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }

    unsafe impl NSTextFieldDelegate for TextFieldChangeTarget {}
);

impl TextFieldChangeTarget {
    fn new(mtm: MainThreadMarker, frame_handle: NativeHandle, command_id: u16) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TextFieldChangeTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

/// Fire [`dispatch_command`] when an editable [`NSTextField`] changes.
pub fn text_field_enable_change_notify(field_handle: NativeHandle, command_id: u16) {
    let Some(parent) = VIEW_PARENT.with(|map| map.borrow().get(&field_handle).copied()) else {
        return;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let frame_handle = resolve_frame_handle(parent);
    let target = TextFieldChangeTarget::new(mtm, frame_handle, command_id);
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&field_handle) {
            if let Some(field) = view.downcast_ref::<NSTextField>() {
                unsafe {
                    field.setDelegate(Some(ProtocolObject::from_ref(&*target)));
                }
            }
        }
    });
    TEXT_FIELD_DELEGATES.with(|delegates| delegates.borrow_mut().push(target));
}

// ── Multiline TextCtrl (NSScrollView + NSTextView) ────────────────

thread_local! {
    static TEXT_VIEW_DELEGATES: RefCell<Vec<Retained<TextViewChangeTarget>>> =
        const { RefCell::new(Vec::new()) };
    static MULTILINE_TEXT_CTRL: RefCell<HashSet<NativeHandle>> =
        RefCell::new(HashSet::new());
    static RICH_TEXT_CTRL: RefCell<HashSet<NativeHandle>> =
        RefCell::new(HashSet::new());
}

#[derive(Default)]
struct TextViewChangeTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TextViewChangeTargetIvars]
    struct TextViewChangeTarget;

    unsafe impl NSObjectProtocol for TextViewChangeTarget {}

    unsafe impl NSTextDelegate for TextViewChangeTarget {
        #[unsafe(method(textDidChange:))]
        fn text_did_change(&self, _notification: &NSNotification) {
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }

    unsafe impl NSTextViewDelegate for TextViewChangeTarget {}
);

impl TextViewChangeTarget {
    fn new(mtm: MainThreadMarker, frame_handle: NativeHandle, command_id: u16) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TextViewChangeTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) fn text_view_in_scroll_handle(handle: NativeHandle) -> Option<Retained<NSTextView>> {
    VIEWS.with(|map| {
        let views = map.borrow();
        let view = views.get(&handle)?;
        let scroll = view.downcast_ref::<NSScrollView>()?;
        let doc = scroll.documentView()?;
        doc.downcast::<NSTextView>().ok()
    })
}

fn text_view_enable_change_notify(scroll_handle: NativeHandle, command_id: u16) {
    let Some(parent) = VIEW_PARENT.with(|map| map.borrow().get(&scroll_handle).copied()) else {
        return;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(text_view) = text_view_in_scroll_handle(scroll_handle) else {
        return;
    };
    let frame_handle = resolve_frame_handle(parent);
    let target = TextViewChangeTarget::new(mtm, frame_handle, command_id);
    text_view.setDelegate(Some(ProtocolObject::from_ref(&*target)));
    TEXT_VIEW_DELEGATES.with(|delegates| delegates.borrow_mut().push(target));
}

// ── Spin stepper target (sync field + dispatch command) ───────────

#[derive(Default)]
struct SpinStepperTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
    field: Cell<Option<Retained<NSTextField>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SpinStepperTargetIvars]
    struct SpinStepperTarget;

    unsafe impl NSObjectProtocol for SpinStepperTarget {}

    impl SpinStepperTarget {
        #[unsafe(method(ruWxSpinChanged:))]
        fn spin_changed(&self, sender: Option<&AnyObject>) {
            if let Some(stepper) = sender.and_then(|s| s.downcast_ref::<NSStepper>()) {
                let v = stepper.doubleValue().round() as i32;
                if let Some(field) = self.ivars().field.take() {
                    field.setStringValue(&NSString::from_str(&v.to_string()));
                    self.ivars().field.set(Some(field));
                }
            }
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }
);

impl SpinStepperTarget {
    fn new(
        mtm: MainThreadMarker,
        frame_handle: NativeHandle,
        command_id: u16,
        field: Retained<NSTextField>,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SpinStepperTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
            field: Cell::new(Some(field)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ── Timer target ────────────────────────────────────────────────────

#[derive(Default)]
struct TimerTargetIvars {
    timer_id: Cell<u32>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TimerTargetIvars]
    struct TimerTarget;

    unsafe impl NSObjectProtocol for TimerTarget {}

    impl TimerTarget {
        #[unsafe(method(ruWxTimerFired:))]
        fn timer_fired(&self, _timer: Option<&NSTimer>) {
            dispatch_timer(self.ivars().timer_id.get());
        }
    }
);

impl TimerTarget {
    fn new(mtm: MainThreadMarker, timer_id: u32) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TimerTargetIvars {
            timer_id: Cell::new(timer_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn dispatch_timer(timer_id: u32) {
    TIMER_FIRE.with(|map| {
        let handler = map.borrow_mut().remove(&timer_id);
        if let Some(mut handler) = handler {
            // Come `deliver_raw_message`: un panic non deve far
            // perdere la callback del timer né propagarsi nel
            // run loop nativo.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler();
            }));
            map.borrow_mut().insert(timer_id, handler);
            if let Err(payload) = result {
                crate::core::log::log_message(
                    crate::core::log::LogLevel::Error,
                    "ui/dispatch",
                    format!(
                        "panic in timer handler (id={}): {}",
                        timer_id,
                        crate::core::log::payload_to_string(payload.as_ref())
                    ),
                );
            }
        }
    });
}

/// Register the callback invoked when an AppKit timer fires.
pub fn timer_set_fire(timer_id: u32, handler: Box<dyn FnMut()>) {
    TIMER_FIRE.with(|map| map.borrow_mut().insert(timer_id, handler));
}

/// Remove a timer callback and invalidate any running timer.
pub fn timer_unregister(timer_id: u32) {
    timer_stop(timer_id);
    TIMER_FIRE.with(|map| map.borrow_mut().remove(&timer_id));
}

/// Start (or restart) an [`NSTimer`] for `timer_id`.
pub fn timer_start(timer_id: u32, interval_ms: u32, repeat: bool) {
    timer_stop(timer_id);
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let target = TimerTarget::new(mtm, timer_id);
    TIMER_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            interval_ms as f64 / 1000.0,
            &target,
            sel!(ruWxTimerFired:),
            None,
            repeat,
        )
    };
    TIMER_INSTANCES.with(|map| map.borrow_mut().insert(timer_id, timer));
}

/// Stop and invalidate the AppKit timer for `timer_id`.
pub fn timer_stop(timer_id: u32) {
    TIMER_INSTANCES.with(|map| {
        if let Some(timer) = map.borrow_mut().remove(&timer_id) {
            timer.invalidate();
        }
    });
}

// ── ListBox custom view ─────────────────────────────────────────────

const LISTBOX_ROW_HEIGHT: f64 = 20.0;

#[derive(Clone, Copy)]
struct ListBoxAppearance {
    row_height: f64,
    text: Rgba,
    background: Rgba,
    alt_background: Rgba,
    selection_bg: Rgba,
    selection_fg: Rgba,
}

impl Default for ListBoxAppearance {
    fn default() -> Self {
        Self {
            row_height: LISTBOX_ROW_HEIGHT,
            text: (0, 0, 0, 255),
            background: (255, 255, 255, 255),
            alt_background: (255, 255, 255, 255),
            selection_bg: (0, 120, 215, 255),
            selection_fg: (255, 255, 255, 255),
        }
    }
}

thread_local! {
    static LISTBOX_APPEARANCE: RefCell<HashMap<NativeHandle, ListBoxAppearance>> =
        RefCell::new(HashMap::new());
}

fn listbox_appearance(list_handle: NativeHandle) -> ListBoxAppearance {
    LISTBOX_APPEARANCE.with(|map| map.borrow().get(&list_handle).copied().unwrap_or_default())
}

/// Customise list row colours and height (macOS).
pub fn listbox_set_appearance(
    list_handle: NativeHandle,
    row_height: f64,
    text: Colour,
    background: Colour,
    alt_background: Colour,
    selection_bg: Colour,
    selection_fg: Colour,
) {
    let to_rgba = |c: Colour| (c.r, c.g, c.b, c.a);
    LISTBOX_APPEARANCE.with(|map| {
        map.borrow_mut().insert(
            list_handle,
            ListBoxAppearance {
                row_height: row_height.max(18.0),
                text: to_rgba(text),
                background: to_rgba(background),
                alt_background: to_rgba(alt_background),
                selection_bg: to_rgba(selection_bg),
                selection_fg: to_rgba(selection_fg),
            },
        );
    });
    listbox_reload(list_handle);
    VIEWS.with(|map| {
        if let Some(scroll) = map.borrow().get(&list_handle) {
            if let Some(scroll_view) = scroll.downcast_ref::<NSScrollView>() {
                let theme = listbox_appearance(list_handle);
                scroll_view.setBackgroundColor(&ns_color_from_rgba(theme.background));
                scroll_view.setDrawsBackground(true);
                scroll_view.setBorderType(NSBorderType::NoBorder);
            }
        }
    });
}

pub(super) fn listbox_init_storage(list_handle: NativeHandle, multi_select: bool) {
    LISTBOX_ITEMS.with(|map| {
        map.borrow_mut()
            .insert(list_handle, RefCell::new(Vec::new()));
    });
    LISTBOX_SELECTED.with(|map| {
        map.borrow_mut()
            .insert(list_handle, RefCell::new(Vec::new()));
    });
    LISTBOX_MULTI.with(|map| {
        map.borrow_mut().insert(list_handle, multi_select);
    });
}

pub(super) fn listbox_item_count(list_handle: NativeHandle) -> usize {
    LISTBOX_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|items| items.borrow().len())
            .unwrap_or(0)
    })
}

pub(super) fn listbox_item_text(list_handle: NativeHandle, index: usize) -> String {
    LISTBOX_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|items| items.borrow().get(index).cloned())
            .unwrap_or_default()
    })
}

pub(super) fn listbox_is_multi_select(list_handle: NativeHandle) -> bool {
    LISTBOX_MULTI.with(|map| map.borrow().get(&list_handle).copied().unwrap_or(false))
}

pub(super) fn listbox_read_selection(list_handle: NativeHandle) -> Vec<usize> {
    LISTBOX_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|sel| sel.borrow().clone())
            .unwrap_or_default()
    })
}

pub(super) fn listbox_write_selection(list_handle: NativeHandle, rows: Vec<usize>) {
    LISTBOX_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            *sel.borrow_mut() = rows;
        }
    });
}

#[derive(Default)]
struct ListBoxViewIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ListBoxViewIvars]
    struct ListBoxView;

    unsafe impl NSObjectProtocol for ListBoxView {}

    impl ListBoxView {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().list_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let count = LISTBOX_ITEMS.with(|map| {
                map.borrow()
                    .get(&handle)
                    .map(|items| items.borrow().len())
                    .unwrap_or(0)
            });
            if count == 0 {
                return;
            }
            let row_h = listbox_appearance(handle).row_height;
            let total_h = count as f64 * row_h;
            let row = ((total_h - point.y) / row_h).floor() as isize;
            if row < 0 || row as usize >= count {
                return;
            }

            let multi = LISTBOX_MULTI.with(|map| map.borrow().get(&handle).copied().unwrap_or(false));
            LISTBOX_SELECTED.with(|map| {
                if let Some(sel) = map.borrow().get(&handle) {
                    let mut selected = sel.borrow_mut();
                    if multi {
                        let idx = row as usize;
                        if selected.contains(&idx) {
                            selected.retain(|&i| i != idx);
                        } else {
                            selected.push(idx);
                        }
                    } else {
                        selected.clear();
                        selected.push(row as usize);
                    }
                }
            });

            listbox_redraw(handle);

            if event.clickCount() >= 2 {
                dispatch_command_notify(
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                    2,
                );
            } else {
                dispatch_command(
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                );
            }
        }
    }
);

impl ListBoxView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ListBoxViewIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }
}

fn listbox_redraw(list_handle: NativeHandle) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    LISTBOX_VIEWS.with(|views| {
        let view = views.borrow().get(&list_handle).cloned();
        let Some(view) = view else {
            return;
        };
        while let Some(sub) = view.subviews().firstObject() {
            sub.removeFromSuperview();
        }

        let data = LISTBOX_ITEMS.with(|map| {
            map.borrow().get(&list_handle).map(|items| {
                let borrowed = items.borrow();
                let selected = LISTBOX_SELECTED.with(|sel| {
                    sel.borrow()
                        .get(&list_handle)
                        .map(|s| s.borrow().clone())
                        .unwrap_or_default()
                });
                (borrowed.clone(), selected, view.frame().size.width)
            })
        });
        let Some((items, selected, width)) = data else {
            return;
        };

        let theme = listbox_appearance(list_handle);
        let row_h = theme.row_height;
        let total_h = items.len() as f64 * row_h;
        view.setFrameSize(NSSize::new(width, total_h.max(row_h)));

        for (i, text) in items.iter().enumerate() {
            let y = total_h - (i as f64 + 1.0) * row_h;
            let row_frame = NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, row_h));
            let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
            label.setFrame(row_frame);
            label.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(13.0)));
            label.setTextColor(Some(&ns_color_from_rgba(theme.text)));
            if selected.contains(&i) {
                label.setBackgroundColor(Some(&ns_color_from_rgba(theme.selection_bg)));
                label.setTextColor(Some(&ns_color_from_rgba(theme.selection_fg)));
                label.setDrawsBackground(true);
            } else {
                let bg = if i % 2 == 0 {
                    theme.background
                } else {
                    theme.alt_background
                };
                label.setBackgroundColor(Some(&ns_color_from_rgba(bg)));
                label.setDrawsBackground(true);
            }
            view.addSubview(&label);
        }
    });
}

fn listbox_reload(list_handle: NativeHandle) {
    if appkit_listbox_table::listbox_is_native_table(list_handle) {
        appkit_listbox_table::listbox_reload(list_handle);
    } else {
        listbox_redraw(list_handle);
    }
}

// ── Image lists and raster icons (NSImage) ───────────────────────────

fn bgra_premul_to_rgba(bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
    let px_count = width * height;
    let mut rgba = vec![0u8; px_count * 4];
    for i in 0..px_count {
        let b = bgra[i * 4] as f32;
        let g = bgra[i * 4 + 1] as f32;
        let r = bgra[i * 4 + 2] as f32;
        let a = bgra[i * 4 + 3] as f32 / 255.0;
        if a > 0.0 {
            rgba[i * 4] = (r / a).min(255.0) as u8;
            rgba[i * 4 + 1] = (g / a).min(255.0) as u8;
            rgba[i * 4 + 2] = (b / a).min(255.0) as u8;
        }
        rgba[i * 4 + 3] = bgra[i * 4 + 3];
    }
    rgba
}

pub(in crate::platform) fn nsimage_from_bgra(
    _mtm: MainThreadMarker,
    bgra: &[u8],
    width: i32,
    height: i32,
) -> Retained<NSImage> {
    let w = width as usize;
    let h = height as usize;
    let mut rgba = bgra_premul_to_rgba(bgra, w, h);
    let mut planes: [*mut u8; 1] = [rgba.as_mut_ptr()];
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            planes.as_mut_ptr(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSCalibratedRGBColorSpace,
            NSBitmapFormat::AlphaNonpremultiplied,
            (w * 4) as isize,
            32,
        )
    }
    .expect("NSBitmapImageRep");
    std::mem::forget(rgba);

    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(width as f64, height as f64));
    image.addRepresentation(&rep);
    image
}

/// Create NSImage from raw RGBA pixel data (width × height × 4 bytes).
pub(in crate::platform) fn nsimage_from_rgba(
    _mtm: MainThreadMarker,
    rgba: &[u8],
    width: i32,
    height: i32,
) -> Retained<NSImage> {
    let w = width as usize;
    let _h = height as usize;
    let mut rgba_copy = rgba.to_vec();
    let mut planes: [*mut u8; 1] = [rgba_copy.as_mut_ptr()];
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            planes.as_mut_ptr(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSCalibratedRGBColorSpace,
            NSBitmapFormat::AlphaNonpremultiplied,
            (w * 4) as isize,
            32,
        )
    }
    .expect("NSBitmapImageRep");
    std::mem::forget(rgba_copy);

    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(width as f64, height as f64));
    image.addRepresentation(&rep);
    image
}

pub fn image_list_create(width: i32, height: i32) -> NativeHandle {
    let handle = alloc_widget_handle();
    IMAGE_LISTS.with(|map| {
        map.borrow_mut().insert(
            handle,
            ImageListStorage {
                width,
                height,
                images: Vec::new(),
            },
        );
    });
    handle
}

pub fn image_list_destroy(handle: NativeHandle) {
    IMAGE_LISTS.with(|map| {
        map.borrow_mut().remove(&handle);
    });
}

pub fn image_list_count(handle: NativeHandle) -> i32 {
    IMAGE_LISTS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|s| s.images.len() as i32)
            .unwrap_or(0)
    })
}

fn nsimage_sized(image: &NSImage, side: f64) -> Retained<NSImage> {
    let sized = NSCopying::copy(image);
    sized.setSize(NSSize::new(side, side));
    sized
}

/// Coloured toolbar/tab glyph (never template — template icons disappear on many backgrounds).
pub(super) fn image_for_display(image: &NSImage, side: f64) -> Retained<NSImage> {
    let sized = nsimage_sized(image, side);
    sized.setTemplate(false);
    sized
}

fn toolbar_icon_side(bar_h: f64, img_w: f64, img_h: f64) -> f64 {
    let cap = (bar_h - 8.0).max(20.0);
    img_w.max(img_h).min(cap)
}

fn image_list_push_image(handle: NativeHandle, image: Retained<NSImage>) -> Option<i32> {
    IMAGE_LISTS.with(|map| {
        let mut map = map.borrow_mut();
        let storage = map.get_mut(&handle)?;
        image.setTemplate(false);
        let idx = storage.images.len() as i32;
        storage.images.push(image);
        Some(idx)
    })
}

pub fn image_list_add_svg(handle: NativeHandle, svg: &[u8]) -> Option<i32> {
    let mtm = MainThreadMarker::new()?;
    let (width, height) =
        IMAGE_LISTS.with(|map| map.borrow().get(&handle).map(|s| (s.width, s.height)))?;
    let bgra = crate::dc::icon::svg_bytes_to_bgra(svg, width as u32, height as u32)?;
    let image = nsimage_from_bgra(mtm, &bgra, width, height);
    image_list_push_image(handle, image)
}

pub fn image_list_add_image_bytes(handle: NativeHandle, bytes: &[u8]) -> Option<i32> {
    let mtm = MainThreadMarker::new()?;
    let (width, height) =
        IMAGE_LISTS.with(|map| map.borrow().get(&handle).map(|s| (s.width, s.height)))?;
    let bgra = crate::dc::icon::image_bytes_to_bgra(bytes, width as u32, height as u32)?;
    let image = nsimage_from_bgra(mtm, &bgra, width, height);
    image_list_push_image(handle, image)
}

/// Add a macOS bitmap handle ([`mac_bitmap_image_at`]) into an image list.
pub fn image_list_add_mac_bitmap(
    list_handle: NativeHandle,
    bitmap_handle: NativeHandle,
) -> Option<i32> {
    if bitmap_handle == 0 {
        return None;
    }
    let image = mac_bitmap_image_at(bitmap_handle)?;
    image_list_push_image(list_handle, image)
}

pub fn image_list_image_at(handle: NativeHandle, index: i32) -> Option<Retained<NSImage>> {
    if index < 0 {
        return None;
    }
    IMAGE_LISTS.with(|map| {
        map.borrow()
            .get(&handle)
            .and_then(|s| s.images.get(index as usize).cloned())
    })
}

pub fn menu_register_icon(item_id: u16, image: Retained<NSImage>) {
    MENU_ITEM_ICONS.with(|map| {
        map.borrow_mut().insert(item_id, image);
    });
}

pub fn menu_release_icons(item_ids: &[u16]) {
    MENU_ITEM_ICONS.with(|map| {
        let mut m = map.borrow_mut();
        for id in item_ids {
            m.remove(id);
        }
    });
}

pub fn menu_icon_from_svg(svg_bytes: &[u8], icon_size: u32) -> Option<Retained<NSImage>> {
    let mtm = MainThreadMarker::new()?;
    let bgra = crate::dc::icon::svg_bytes_to_bgra(svg_bytes, icon_size, icon_size)?;
    Some(nsimage_from_bgra(
        mtm,
        &bgra,
        icon_size as i32,
        icon_size as i32,
    ))
}

pub fn menu_icon_from_png(png_bytes: &[u8], icon_size: u32) -> Option<Retained<NSImage>> {
    let mtm = MainThreadMarker::new()?;
    let bgra = crate::dc::icon::image_bytes_to_bgra(png_bytes, icon_size, icon_size)?;
    Some(nsimage_from_bgra(
        mtm,
        &bgra,
        icon_size as i32,
        icon_size as i32,
    ))
}

pub fn menu_icon_from_colour(r: u8, g: u8, b: u8, a: u8, size: u32) -> Retained<NSImage> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let bgra = crate::dc::icon::solid_colour_bgra(size, size, r, g, b, a);
    nsimage_from_bgra(mtm, &bgra, size as i32, size as i32)
}

fn menu_item_icon(item_id: u16) -> Option<Retained<NSImage>> {
    MENU_ITEM_ICONS.with(|map| map.borrow().get(&item_id).cloned())
}

pub fn button_set_image(view_handle: NativeHandle, image: &NSImage, image_only: bool) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(btn) = view.downcast_ref::<NSButton>() {
                btn.setImage(Some(image));
                btn.setImagePosition(if image_only {
                    NSCellImagePosition::ImageOnly
                } else {
                    NSCellImagePosition::ImageLeft
                });
                btn.setImageScaling(NSImageScaling::ScaleProportionallyDown);
            }
        }
    });
}

/// Icon-only transport/toolbar button: flat bezel and a pre-sized bitmap.
// `NSBezelStyleShadowlessSquare` (raw value 6) has no non-deprecated equivalent in
// objc2-app-kit 0.3; the bezel style is required to keep the flat icon-only look.
#[allow(deprecated)]
pub fn button_set_bitmap_icon(view_handle: NativeHandle, bitmap_handle: NativeHandle, side: u32) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(btn) = view.downcast_ref::<NSButton>() {
                btn.setBezelStyle(NSBezelStyle::ShadowlessSquare);
                btn.setBordered(false);
                btn.setTitle(&NSString::from_str(""));
                btn.setImagePosition(NSCellImagePosition::ImageOnly);
                btn.setEnabled(true);
            }
        }
    });
    if bitmap_handle == 0 {
        return;
    }
    if let Some(image) = mac_bitmap_image_at(bitmap_handle) {
        let icon_side = (side as f64).max(16.0);
        let sized = nsimage_sized(&image, icon_side);
        VIEWS.with(|map| {
            if let Some(view) = map.borrow().get(&view_handle) {
                if let Some(btn) = view.downcast_ref::<NSButton>() {
                    btn.setImage(Some(&sized));
                    btn.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                }
            }
        });
    }
}

/// Like [`button_set_bitmap_icon`] and re-attach the click target.
pub fn button_set_bitmap_icon_wired(
    parent: NativeHandle,
    view_handle: NativeHandle,
    command_id: u16,
    bitmap_handle: NativeHandle,
    side: u32,
) {
    button_set_bitmap_icon(view_handle, bitmap_handle, side);
    button_rewire_click_target(view_handle, parent, command_id);
}

pub fn button_set_svg_image(
    view_handle: NativeHandle,
    svg: &[u8],
    size: u32,
    image_only: bool,
) -> bool {
    if let Some(image) = menu_icon_from_svg(svg, size) {
        button_set_image(view_handle, &image, image_only);
        true
    } else {
        false
    }
}

// The RGBA components mirror the platform-independent `Colour` API; grouping
// them in a struct would change the exported signature.
#[allow(clippy::too_many_arguments)]
pub fn button_set_colour_image(
    view_handle: NativeHandle,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    width: u32,
    height: u32,
    image_only: bool,
) {
    let image = menu_icon_from_colour(r, g, b, a, width.min(height));
    button_set_image(view_handle, &image, image_only);
}

// ── ListCtrl custom view ────────────────────────────────────────────

const LISTCTRL_HEADER_HEIGHT: f64 = 22.0;
const LISTCTRL_ROW_HEIGHT: f64 = 20.0;
const LISTCTRL_ICON_WIDTH: f64 = 18.0;

pub(super) fn listctrl_row_height(list_handle: NativeHandle) -> f64 {
    LISTCTRL_ROW_HEIGHTS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .copied()
            .unwrap_or(LISTCTRL_ROW_HEIGHT)
            .max(18.0)
    })
}

fn listctrl_header_height(list_handle: NativeHandle) -> f64 {
    (listctrl_row_height(list_handle) + 4.0).max(LISTCTRL_HEADER_HEIGHT)
}

pub fn listctrl_set_row_height(list_handle: NativeHandle, height: f64) {
    LISTCTRL_ROW_HEIGHTS.with(|map| {
        map.borrow_mut().insert(list_handle, height.max(18.0));
    });
    listctrl_redraw(list_handle);
}

/// Imposta la riga-altezza nella mappa condivisa dei listctrl nativi.
/// Usato da [`appkit_list_table::listctrl_set_font_size_native`] che
/// non può accedere direttamente al `thread_local!` privato di `appkit`.
pub(super) fn listctrl_set_row_height_inner(list_handle: NativeHandle, height: f64) {
    LISTCTRL_ROW_HEIGHTS.with(|map| {
        map.borrow_mut().insert(list_handle, height);
    });
}

/// Imposta la dimensione del font (in punti) per la ListCtrl nativa.
/// Usato dalle preferenze utente (`Strumenti → Opzioni…`).
pub fn listctrl_set_font_size(list_handle: NativeHandle, point_size: f64) {
    if let Some(mtm) = MainThreadMarker::new() {
        let _ = mtm;
    }
    appkit_list_table::listctrl_set_font_size_native(list_handle, point_size);
}
/// Matches Win32 `LVN_ITEMCHANGED` — used by [`ListCtrl::on_item_selected`].
pub const LISTCTRL_NOTIFY_ITEM_CHANGED: u32 = 0xFFFFFF9B;
/// Row drag-and-drop reorder — used by [`ListCtrl::on_row_reordered`].
pub const LISTCTRL_NOTIFY_ROW_REORDERED: u32 = 0xFFFFFF9A;
/// Double-click on a row — used by [`ListCtrl::on_item_double_click`].
pub const LISTCTRL_NOTIFY_ITEM_DBLCLK: u32 = 0xFFFFFF99;
/// Click on a column header — the macOS form of Win32 `LVN_COLUMNCLICK`;
/// the deferred payload carries the clicked column index.
pub const LISTCTRL_NOTIFY_COLUMN_CLICK: u32 = 0xFFFFFF98;

const LISTCTRL_DRAG_THRESHOLD: f64 = 4.0;

#[derive(Default)]
struct ListCtrlViewIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
    drag_source_row: Cell<isize>,
    drag_active: Cell<bool>,
    drag_start_x: Cell<f64>,
    drag_start_y: Cell<f64>,
}

#[derive(Default)]
struct ListCtrlMouseViewIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

fn listctrl_with_view<F, R>(handle: NativeHandle, f: F) -> Option<R>
where
    F: FnOnce(&ListCtrlView) -> R,
{
    LISTCTRL_VIEWS.with(|views| views.borrow().get(&handle).map(|view| f(view)))
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ListCtrlViewIvars]
    struct ListCtrlView;

    unsafe impl NSObjectProtocol for ListCtrlView {}
);

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ListCtrlMouseViewIvars]
    struct ListCtrlMouseView;

    unsafe impl NSObjectProtocol for ListCtrlMouseView {}

    impl ListCtrlMouseView {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().list_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            if let Some(view) = listctrl_with_view(handle, |v| v.retain()) {
                view.ivars().drag_active.set(false);
                view.ivars().drag_source_row.set(-1);
                view.ivars().drag_start_x.set(point.x);
                view.ivars().drag_start_y.set(point.y);
            }
            LISTCTRL_DROP_HINT.with(|map| {
                map.borrow_mut().insert(handle, None);
            });

            let Some(row) = listctrl_row_at_y(handle, point.y) else {
                return;
            };

            if let Some(view) = listctrl_with_view(handle, |v| v.retain()) {
                view.ivars().drag_source_row.set(row as isize);
            }

            LISTCTRL_SELECTED.with(|map| {
                if let Some(sel) = map.borrow().get(&handle) {
                    *sel.borrow_mut() = vec![row];
                }
            });

            let col = listctrl_column_at_x(handle, point.x);
            LISTCTRL_LAST_HIT.with(|map| {
                map.borrow_mut().insert(handle, (row, col));
            });

            let frame_handle = self.ivars().frame_handle.get();
            let command_id = self.ivars().command_id.get();
            listctrl_defer_redraw(handle);

            if event.clickCount() >= 2 {
                listctrl_defer_notify_row(
                    frame_handle,
                    command_id,
                    LISTCTRL_NOTIFY_ITEM_DBLCLK,
                    row as isize,
                );
                return;
            }

            listctrl_defer_notify(frame_handle, command_id, LISTCTRL_NOTIFY_ITEM_CHANGED);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let handle = self.ivars().list_handle.get();
            if !listctrl_row_reorder_enabled(handle) {
                return;
            }
            let (source, drag_active, start_x, start_y) = listctrl_with_view(handle, |v| {
                (
                    v.ivars().drag_source_row.get(),
                    v.ivars().drag_active.get(),
                    v.ivars().drag_start_x.get(),
                    v.ivars().drag_start_y.get(),
                )
            })
            .unwrap_or((-1, false, 0.0, 0.0));
            if source < 0 {
                return;
            }
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let dx = point.x - start_x;
            let dy = point.y - start_y;
            if !drag_active {
                if (dx * dx + dy * dy).sqrt() < LISTCTRL_DRAG_THRESHOLD {
                    return;
                }
                if let Some(view) = listctrl_with_view(handle, |v| v.retain()) {
                    view.ivars().drag_active.set(true);
                }
            }
            let hint = listctrl_row_at_y(handle, point.y);
            LISTCTRL_DROP_HINT.with(|map| {
                map.borrow_mut().insert(handle, hint);
            });
            listctrl_defer_redraw(handle);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let handle = self.ivars().list_handle.get();
            let (source, drag_active) = listctrl_with_view(handle, |v| {
                (
                    v.ivars().drag_source_row.get(),
                    v.ivars().drag_active.get(),
                )
            })
            .unwrap_or((-1, false));
            if let Some(view) = listctrl_with_view(handle, |v| v.retain()) {
                view.ivars().drag_source_row.set(-1);
                view.ivars().drag_active.set(false);
            }
            LISTCTRL_DROP_HINT.with(|map| {
                map.borrow_mut().insert(handle, None);
            });

            if !listctrl_row_reorder_enabled(handle) || !drag_active || source < 0 {
                return;
            }

            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let Some(to) = listctrl_row_at_y(handle, point.y) else {
                listctrl_defer_redraw(handle);
                return;
            };
            let from = source as usize;
            let frame_handle = self.ivars().frame_handle.get();
            let command_id = self.ivars().command_id.get();
            if from != to {
                listctrl_record_row_reordered(handle, from, to);
                listctrl_defer_notify(frame_handle, command_id, LISTCTRL_NOTIFY_ROW_REORDERED);
            } else {
                listctrl_defer_redraw(handle);
            }
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().list_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            if let Some(row) = listctrl_row_at_y(handle, point.y) {
                LISTCTRL_SELECTED.with(|map| {
                    if let Some(sel) = map.borrow().get(&handle) {
                        *sel.borrow_mut() = vec![row];
                    }
                });
                let col = listctrl_column_at_x(handle, point.x);
                LISTCTRL_LAST_HIT.with(|map| {
                    map.borrow_mut().insert(handle, (row, col));
                });
                listctrl_defer_redraw(handle);
            }
            appkit_accelerator::dispatch_frame_context_menu(
                self.ivars().frame_handle.get(),
                event,
                self,
            );
        }
    }
);

impl ListCtrlView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ListCtrlViewIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
            drag_source_row: Cell::new(-1),
            drag_active: Cell::new(false),
            drag_start_x: Cell::new(0.0),
            drag_start_y: Cell::new(0.0),
        });
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }
}

impl ListCtrlMouseView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ListCtrlMouseViewIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }
}

fn listctrl_row_count(handle: NativeHandle) -> usize {
    LISTCTRL_ROWS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|rows| rows.borrow().len())
            .unwrap_or(0)
    })
}

fn listctrl_content_height(list_handle: NativeHandle, row_count: usize) -> f64 {
    listctrl_header_height(list_handle) + row_count as f64 * listctrl_row_height(list_handle)
}

/// When the document is shorter than the scroll clip, pin rows under the header at the top.
fn listctrl_document_layout(list_handle: NativeHandle, content_h: f64) -> (f64, f64) {
    let clip_h = VIEWS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|view| view.downcast_ref::<NSScrollView>())
            .map(|scroll| scroll.contentView().bounds().size.height)
            .unwrap_or(content_h)
    });
    let doc_h = content_h.max(clip_h);
    let y_base = doc_h - content_h;
    (doc_h, y_base)
}

fn listctrl_row_at_y(handle: NativeHandle, y: f64) -> Option<usize> {
    let row_count = listctrl_row_count(handle);
    if row_count == 0 {
        return None;
    }
    let content_h = listctrl_content_height(handle, row_count);
    let (_, y_base) = listctrl_document_layout(handle, content_h);
    let row_h = listctrl_row_height(handle);
    let header_h = listctrl_header_height(handle);
    if y >= y_base + content_h - header_h {
        return None;
    }
    if y < y_base {
        return None;
    }
    let row = ((y_base + content_h - header_h - y) / row_h).floor() as isize;
    if row < 0 || row as usize >= row_count {
        None
    } else {
        Some(row as usize)
    }
}

pub(super) fn listctrl_row_reorder_enabled(handle: NativeHandle) -> bool {
    LISTCTRL_ROW_REORDER_ENABLED.with(|map| map.borrow().get(&handle).copied().unwrap_or(false))
}

pub fn listctrl_set_row_reorder_enabled(list_handle: NativeHandle, enabled: bool) {
    LISTCTRL_ROW_REORDER_ENABLED.with(|map| {
        map.borrow_mut().insert(list_handle, enabled);
    });
    if appkit_list_table::listctrl_is_native_table(list_handle) {
        appkit_list_table::listctrl_apply_row_reorder(list_handle);
    }
}

pub fn listctrl_take_row_reordered(list_handle: NativeHandle) -> Option<(usize, usize)> {
    LISTCTRL_ROW_REORDERED.with(|map| map.borrow_mut().remove(&list_handle))
}

pub(super) fn listctrl_record_row_reordered(list_handle: NativeHandle, from: usize, to: usize) {
    LISTCTRL_ROW_REORDERED.with(|map| {
        map.borrow_mut().insert(list_handle, (from, to));
    });
}

pub fn listctrl_move_row(list_handle: NativeHandle, from: usize, to: usize) -> bool {
    if from == to {
        return false;
    }
    let moved = LISTCTRL_ROWS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(rows) = map.get_mut(&list_handle) else {
            return false;
        };
        let mut vec = rows.borrow_mut();
        if from >= vec.len() {
            return false;
        }
        let item = vec.remove(from);
        let insert_at = to.min(vec.len());
        vec.insert(insert_at, item);
        true
    });
    if moved {
        let count = listctrl_row_count(list_handle);
        let new_sel = to.min(count.saturating_sub(1));
        LISTCTRL_SELECTED.with(|map| {
            if let Some(sel) = map.borrow().get(&list_handle) {
                *sel.borrow_mut() = vec![new_sel];
            }
        });
        LISTCTRL_DROP_HINT.with(|map| {
            map.borrow_mut().insert(list_handle, None);
        });
        listctrl_redraw(list_handle);
    }
    moved
}

fn listctrl_column_layout(handle: NativeHandle, view_width: f64) -> Vec<(f64, f64)> {
    LISTCTRL_COLUMNS.with(|map| {
        let cols = map
            .borrow()
            .get(&handle)
            .map(|c| c.borrow().clone())
            .unwrap_or_default();
        if cols.is_empty() {
            return vec![(0.0, view_width)];
        }
        let has_icons =
            LISTCTRL_HAS_IMAGES.with(|m| m.borrow().get(&handle).copied().unwrap_or(false));
        let mut x = if has_icons { LISTCTRL_ICON_WIDTH } else { 0.0 };
        let col_count = cols.len();
        let mut layout = Vec::with_capacity(col_count);
        for (_, width) in &cols {
            let w = if *width > 0 {
                *width as f64
            } else {
                (view_width - x) / (col_count - layout.len()).max(1) as f64
            };
            layout.push((x, w.max(20.0)));
            x += w.max(20.0);
        }
        layout
    })
}

fn listctrl_column_at_x(handle: NativeHandle, x: f64) -> usize {
    let width = LISTCTRL_VIEWS.with(|views| {
        views
            .borrow()
            .get(&handle)
            .map(|v| v.frame().size.width)
            .unwrap_or(0.0)
    });
    let layout = listctrl_column_layout(handle, width);
    for (i, &(cx, cw)) in layout.iter().enumerate() {
        if x >= cx && x < cx + cw {
            return i;
        }
    }
    layout.len().saturating_sub(1)
}

/// Last row/column clicked in the list (for reading cell values).
pub fn listctrl_last_hit_cell(list_handle: NativeHandle) -> Option<(usize, usize)> {
    LISTCTRL_LAST_HIT.with(|map| map.borrow().get(&list_handle).copied())
}

fn listctrl_flush_deferred() {
    LISTCTRL_DEFER_TIMER_ARMED.with(|armed| armed.set(false));
    let redraws: Vec<NativeHandle> =
        LISTCTRL_DEFER_REDRAWS.with(|set| set.borrow_mut().drain().collect());
    for handle in redraws {
        listctrl_redraw_now(handle);
    }
    let notifies: Vec<(NativeHandle, u16, u32, isize)> =
        LISTCTRL_DEFER_NOTIFIES.with(|q| q.borrow_mut().drain(..).collect());
    for (frame_handle, control_id, code, row) in notifies {
        // `ITEM_DBLCLK` è la forma macOS di `LVN_ITEMACTIVATE`: oltre alla
        // notifica generica, va instradato verso gli handler di
        // attivazione riga registrati da `register_lv_item_activate_handler`
        // (su Windows lo fa il WndProc leggendo `NMITEMACTIVATE`).
        if code == LISTCTRL_NOTIFY_ITEM_DBLCLK && row >= 0 {
            dispatch_lv_item_activate(frame_handle, control_id, row);
        }
        // `COLUMN_CLICK` è la forma macOS di `LVN_COLUMNCLICK`: qui `row`
        // trasporta l'indice della colonna cliccata, da instradare verso
        // gli handler registrati con `register_lv_column_click_handler`
        // (su Windows lo fa il WndProc leggendo `NMLISTVIEW.iSubItem`).
        if code == LISTCTRL_NOTIFY_COLUMN_CLICK && row >= 0 {
            dispatch_lv_column_click(frame_handle, control_id, row);
        }
        dispatch_notify(frame_handle, control_id, code);
    }
}

/// Invoca l'handler di attivazione riga registrato per `control_id`,
/// passando l'indice di riga codificato direttamente in `lparam`
/// (vedi [`crate::platform::shell::list_activate_item_index`]).
fn dispatch_lv_item_activate(frame_handle: NativeHandle, control_id: u16, row: isize) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let handler = data
            .borrow_mut()
            .lv_item_activate_handlers
            .remove(&control_id);
        if let Some(mut handler) = handler {
            handler(row);
            data.borrow_mut()
                .lv_item_activate_handlers
                .insert(control_id, handler);
        }
    }
}

/// Invoca l'handler di click su header colonna registrato per
/// `control_id`, passando l'indice di colonna codificato direttamente in
/// `lparam` (vedi [`crate::platform::shell::list_column_click_index`]).
fn dispatch_lv_column_click(frame_handle: NativeHandle, control_id: u16, col: isize) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let inner =
        FRAME_DATA_WEAK.with(|map| map.borrow().get(&frame_handle).and_then(|w| w.upgrade()));
    if let Some(data) = inner {
        let handler = data
            .borrow_mut()
            .lv_column_click_handlers
            .remove(&control_id);
        if let Some(mut handler) = handler {
            handler(col);
            data.borrow_mut()
                .lv_column_click_handlers
                .insert(control_id, handler);
        }
    }
}

fn listctrl_defer_redraw(list_handle: NativeHandle) {
    LISTCTRL_DEFER_REDRAWS.with(|set| {
        set.borrow_mut().insert(list_handle);
    });
    listctrl_arm_defer_timer();
}

fn listctrl_defer_notify(frame_handle: NativeHandle, control_id: u16, code: u32) {
    listctrl_defer_notify_row(frame_handle, control_id, code, -1);
}

/// Come [`listctrl_defer_notify`], ma con l'indice di riga della
/// notifica (usato da `ITEM_DBLCLK` per l'attivazione riga).
fn listctrl_defer_notify_row(frame_handle: NativeHandle, control_id: u16, code: u32, row: isize) {
    LISTCTRL_DEFER_NOTIFIES.with(|q| {
        q.borrow_mut().push((frame_handle, control_id, code, row));
    });
    listctrl_arm_defer_timer();
}

fn listctrl_arm_defer_timer() {
    if LISTCTRL_DEFER_TIMER_ARMED.with(|armed| armed.get()) {
        return;
    }
    LISTCTRL_DEFER_TIMER_ARMED.with(|armed| armed.set(true));
    // Defer until after the current AppKit event returns — a zero-interval
    // NSTimer can fire between the two clicks of a double-click and destroy
    // the mouse overlay view while it is still handling input.
    run_on_next_event(listctrl_flush_deferred);
}

fn listctrl_redraw(list_handle: NativeHandle) {
    listctrl_defer_redraw(list_handle);
}

fn listctrl_redraw_now(list_handle: NativeHandle) {
    if appkit_list_table::listctrl_is_native_table(list_handle) {
        appkit_list_table::listctrl_reload(list_handle);
        return;
    }
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    LISTCTRL_VIEWS.with(|views| {
        let view = views.borrow().get(&list_handle).cloned();
        let Some(view) = view else {
            return;
        };
        while let Some(sub) = view.subviews().firstObject() {
            sub.removeFromSuperview();
        }

        let width = view.frame().size.width;
        let columns = LISTCTRL_COLUMNS.with(|map| {
            map.borrow()
                .get(&list_handle)
                .map(|c| c.borrow().clone())
                .unwrap_or_default()
        });
        let rows = LISTCTRL_ROWS.with(|map| {
            map.borrow()
                .get(&list_handle)
                .map(|r| r.borrow().clone())
                .unwrap_or_default()
        });
        let virtual_count =
            LISTCTRL_VIRTUAL_COUNT.with(|map| map.borrow().get(&list_handle).copied());
        let selected = LISTCTRL_SELECTED.with(|map| {
            map.borrow()
                .get(&list_handle)
                .map(|s| s.borrow().clone())
                .unwrap_or_default()
        });
        let ext_style =
            LISTCTRL_EXT_STYLE.with(|map| map.borrow().get(&list_handle).copied().unwrap_or(0));
        const LVS_EX_FULLROWSELECT: u32 = 0x20;
        let full_row_select = ext_style & LVS_EX_FULLROWSELECT != 0;
        let drop_hint =
            LISTCTRL_DROP_HINT.with(|map| map.borrow().get(&list_handle).copied().flatten());
        let col_layout = listctrl_column_layout(list_handle, width);
        let row_count = virtual_count
            .map(|n| n as usize)
            .unwrap_or_else(|| rows.len());
        let row_h = listctrl_row_height(list_handle);
        let header_h = listctrl_header_height(list_handle);
        let cell_font_size = (row_h * 0.52).clamp(11.0, 15.0);
        let content_h = listctrl_content_height(list_handle, row_count);
        let (doc_h, y_base) = listctrl_document_layout(list_handle, content_h);
        view.setFrameSize(NSSize::new(width, doc_h.max(header_h + row_h)));

        let header_y = y_base + content_h - header_h;
        for (i, (title, _)) in columns.iter().enumerate() {
            let (x, w) = col_layout.get(i).copied().unwrap_or((0.0, width));
            let label = NSTextField::labelWithString(&NSString::from_str(title), mtm);
            label.setFrame(NSRect::new(
                NSPoint::new(x, header_y),
                NSSize::new(w, header_h),
            ));
            label.setDrawsBackground(true);
            label.setBackgroundColor(Some(&objc2_app_kit::NSColor::controlBackgroundColor()));
            let font = objc2_app_kit::NSFont::boldSystemFontOfSize(cell_font_size);
            label.setFont(Some(&font));
            view.addSubview(&label);
        }

        if let Some(hint_row) = drop_hint {
            if hint_row < row_count {
                let row_y = y_base + content_h - header_h - (hint_row as f64 + 1.0) * row_h;
                let marker = NSTextField::labelWithString(&NSString::from_str(""), mtm);
                marker.setFrame(NSRect::new(
                    NSPoint::new(0.0, row_y),
                    NSSize::new(width.max(1.0), 2.0),
                ));
                marker.setDrawsBackground(true);
                marker.setBackgroundColor(Some(&objc2_app_kit::NSColor::systemBlueColor()));
                marker.setEditable(false);
                marker.setSelectable(false);
                view.addSubview(&marker);
            }
        }

        for row_idx in 0..row_count {
            let (cells, image_index) = if virtual_count.is_some() {
                let col_count = columns.len().max(1);
                let mut fetched = Vec::with_capacity(col_count);
                for col_idx in 0..col_count {
                    fetched.push(listctrl_virtual_cell_text(list_handle, row_idx, col_idx));
                }
                (fetched, None)
            } else {
                rows.get(row_idx)
                    .cloned()
                    .unwrap_or_else(|| (Vec::new(), None))
            };
            let row_y = y_base + content_h - header_h - (row_idx as f64 + 1.0) * row_h;
            let selected_row = selected.contains(&row_idx);

            if selected_row && full_row_select {
                let row_x = col_layout.first().map(|(x, _)| *x).unwrap_or(0.0);
                let row_w = col_layout.iter().map(|(x, w)| x + w).fold(row_x, f64::max) - row_x;
                let highlight = NSTextField::labelWithString(&NSString::from_str(""), mtm);
                highlight.setFrame(NSRect::new(
                    NSPoint::new(row_x, row_y),
                    NSSize::new(row_w.max(1.0), row_h),
                ));
                highlight.setDrawsBackground(true);
                highlight.setBackgroundColor(Some(
                    &objc2_app_kit::NSColor::selectedTextBackgroundColor(),
                ));
                highlight.setEditable(false);
                highlight.setSelectable(false);
                view.addSubview(&highlight);
            }

            if let Some(img_idx) = image_index.as_ref().copied().filter(|idx| *idx >= 0) {
                let image_list_handle =
                    LISTCTRL_IMAGE_LISTS.with(|map| map.borrow().get(&list_handle).copied());
                if let Some(list_h) = image_list_handle {
                    if let Some(img) = image_list_image_at(list_h, img_idx) {
                        let icon_h = row_h - 2.0;
                        let icon = NSImageView::initWithFrame(
                            NSImageView::alloc(mtm),
                            NSRect::new(
                                NSPoint::new(1.0, row_y + 1.0),
                                NSSize::new(LISTCTRL_ICON_WIDTH - 2.0, icon_h),
                            ),
                        );
                        icon.setImage(Some(&img));
                        icon.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                        view.addSubview(&icon);
                    }
                }
            }

            for (col_idx, (x, w)) in col_layout.iter().enumerate() {
                let text = cells.get(col_idx).map(String::as_str).unwrap_or("");
                let cell = NSTextField::labelWithString(&NSString::from_str(text), mtm);
                cell.setFrame(NSRect::new(NSPoint::new(*x, row_y), NSSize::new(*w, row_h)));
                let font = objc2_app_kit::NSFont::systemFontOfSize(cell_font_size);
                cell.setFont(Some(&font));
                if selected_row && !full_row_select {
                    cell.setDrawsBackground(true);
                    cell.setBackgroundColor(Some(
                        &objc2_app_kit::NSColor::selectedTextBackgroundColor(),
                    ));
                } else if selected_row && full_row_select {
                    cell.setTextColor(Some(&objc2_app_kit::NSColor::selectedTextColor()));
                }
                view.addSubview(&cell);
            }
        }

        let notify_meta = LISTCTRL_VIEWS.with(|views| {
            views
                .borrow()
                .get(&list_handle)
                .map(|v| (v.ivars().frame_handle.get(), v.ivars().command_id.get()))
        });
        if let Some((frame_handle, command_id)) = notify_meta {
            let mouse_plane = ListCtrlMouseView::new(
                mtm,
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, doc_h)),
                list_handle,
                frame_handle,
                command_id,
            );
            view.addSubview(&mouse_plane);
        }
    });
}

pub(super) fn listctrl_virtual_cell_text(
    list_handle: NativeHandle,
    row: usize,
    col: usize,
) -> String {
    LISTCTRL_VIRTUAL_FETCH.with(|map| {
        map.borrow_mut()
            .get_mut(&list_handle)
            .map(|fetch| fetch(row, col))
            .unwrap_or_default()
    })
}

pub fn listctrl_set_virtual_count(list_handle: NativeHandle, count: u32) {
    LISTCTRL_VIRTUAL_COUNT.with(|map| {
        map.borrow_mut().insert(list_handle, count);
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_set_virtual_fetch(
    list_handle: NativeHandle,
    fetch: Box<dyn FnMut(usize, usize) -> String>,
) {
    LISTCTRL_VIRTUAL_FETCH.with(|map| {
        map.borrow_mut().insert(list_handle, fetch);
    });
}

pub fn listctrl_set_extended_style(list_handle: NativeHandle, style: u32) {
    LISTCTRL_EXT_STYLE.with(|map| {
        map.borrow_mut().insert(list_handle, style);
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_set_item_state(list_handle: NativeHandle, index: usize, state: u32, mask: u32) {
    const LVIS_SELECTED: u32 = 0x0002;
    const LVIS_FOCUSED: u32 = 0x0001;
    if mask & LVIS_SELECTED != 0 {
        if state & LVIS_SELECTED != 0 {
            listctrl_select(list_handle, index);
        } else {
            listctrl_deselect(list_handle, index);
        }
    }
    let _ = LVIS_FOCUSED;
}

pub fn listctrl_get_item_state(list_handle: NativeHandle, index: usize, mask: u32) -> u32 {
    const LVIS_SELECTED: u32 = 0x0002;
    let mut state = 0u32;
    if mask & LVIS_SELECTED != 0 && listctrl_is_selected(list_handle, index) {
        state |= LVIS_SELECTED;
    }
    state
}

/// Whole-window pixel size (including chrome) for [`WindowDC`].
pub fn mac_window_frame_size(handle: NativeHandle) -> (u32, u32) {
    FRAME_WINDOWS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|w| {
                let f = w.frame();
                (f.size.width.max(1.0) as u32, f.size.height.max(1.0) as u32)
            })
            .unwrap_or((1, 1))
    })
}

/// Screen-space rect (top-left origin) of a top-level window.
///
/// AppKit reports window frames with a bottom-left origin relative to
/// the primary screen; this helper normalises them to the top-left
/// origin shared with Win32's `GetWindowRect` (see the inspector's
/// coordinate rules). Must be called on the main thread.
pub fn mac_window_frame_rect(handle: NativeHandle) -> Rect {
    FRAME_WINDOWS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|w| ns_rect_to_top_left(w.frame()))
            .unwrap_or_else(|| Rect::new(0, 0, 1, 1))
    })
}

/// Screen-space rect (top-left origin) of a registered child view.
///
/// Returns an all-zero rect when the handle is unknown or the view is
/// not attached to a window (e.g. a detached popup). Must be called on
/// the main thread.
pub fn mac_view_frame_rect(handle: NativeHandle) -> Rect {
    let Some(view) = child_view_for_handle(handle) else {
        return Rect::new(0, 0, 0, 0);
    };
    let Some(window) = view.window() else {
        return Rect::new(0, 0, 0, 0);
    };
    // `convertRect_toView(_, None)` maps from the view's own coordinate
    // system into window coordinates; `convertRectToScreen` then maps
    // window coordinates into the global screen space.
    let in_window = view.convertRect_toView(view.frame(), None);
    ns_rect_to_top_left(window.convertRectToScreen(in_window))
}

/// Convert an AppKit screen rect (bottom-left origin, points) into the
/// top-left-origin convention used across the library.
fn ns_rect_to_top_left(r: NSRect) -> Rect {
    let width = r.size.width.max(0.0).round() as u32;
    let height = r.size.height.max(0.0).round() as u32;
    let x = r.origin.x.round() as i32;
    let y = (primary_screen_height_points() - r.origin.y - r.size.height).round() as i32;
    Rect::new(x, y, width, height)
}

/// Height in points of the primary display — the reference line for the
/// bottom-left → top-left conversion.
fn primary_screen_height_points() -> f64 {
    MainThreadMarker::new()
        .and_then(NSScreen::mainScreen)
        .map(|s| s.frame().size.height)
        .unwrap_or(0.0)
}

// ── Debug inspector — accessor di Livello 3 (cfr. PIANO §9.1) ────────
//
// L'inspector (Livello 1) non legge mai `FRAME_WINDOWS` / `VIEWS`: passa
// da `platform::inspector_macos`, che usa questi accessor. Tutti gli
// accessor vanno chiamati sul main thread.

/// Content view (radice dell'albero dei widget) di una frame.
pub fn inspector_content_view(frame: NativeHandle) -> Option<Retained<NSView>> {
    if let Some(view) = FRAME_CONTENT.with(|map| map.borrow().get(&frame).cloned()) {
        return Some(view);
    }
    inspector_window(frame).and_then(|w| w.contentView())
}

/// `NSWindow` associata a una frame.
pub fn inspector_window(frame: NativeHandle) -> Option<Retained<NSWindow>> {
    FRAME_WINDOWS.with(|map| map.borrow().get(&frame).cloned())
}

/// Handle logico di una view registrata (identità per puntatore),
/// oppure `None` se la view non è nel registro.
pub fn inspector_handle_for_view(view: &NSView) -> Option<NativeHandle> {
    let ptr = view as *const NSView as usize;
    let find = |map: &RefCell<HashMap<NativeHandle, Retained<NSView>>>| -> Option<NativeHandle> {
        map.borrow()
            .iter()
            .find(|(_, v)| &***v as *const NSView as usize == ptr)
            .map(|(h, _)| *h)
    };
    VIEWS
        .with(|m| find(m))
        .or_else(|| CONTAINERS.with(|m| find(m)))
        .or_else(|| FRAME_CONTENT.with(|m| find(m)))
}

/// Rect su schermo (origine alto-sinistra, punti) del frame di una view.
pub fn inspector_view_screen_rect(view: &NSView) -> Rect {
    let Some(window) = view.window() else {
        return Rect::new(0, 0, 0, 0);
    };
    let in_window = view.convertRect_toView(view.frame(), None);
    ns_rect_to_top_left(window.convertRectToScreen(in_window))
}

/// Nome della classe nativa di una view (es. `"NSButton"`).
pub fn inspector_class_name(view: &NSView) -> String {
    let obj: &objc2::runtime::AnyObject = view;
    obj.class().name().to_string_lossy().into_owned()
}

/// Rect su schermo di una view espressa in punti, con origine
/// alto-sinistra. Alias esplicito di [`inspector_view_screen_rect`].
pub fn inspector_view_rect(view: &NSView) -> Rect {
    inspector_view_screen_rect(view)
}

/// `true` se la view non è visibile (o ha un antenato nascosto).
pub fn inspector_view_hidden(view: &NSView) -> bool {
    view.isHiddenOrHasHiddenAncestor()
}

/// Registra il ruolo semantico dichiarato dal widget (Livello 2) per una view.
fn mark_declared_role(view: &NSView, role: &'static str) {
    let ptr = view as *const NSView as usize;
    DECLARED_ROLES.with(|m| {
        m.borrow_mut().insert(ptr, role);
    });
}

/// Ruolo semantico dichiarato dal widget (Livello 2) per la view, se presente.
///
/// È la sorgente più affidabile per distinguere checkbox e radio su macOS,
/// dove `NSButton.buttonType` / `accessibilityRole` non sono utilizzabili
/// (cfr. `PIANO_DEBUG_INSPECTOR.md` §17.1).
pub fn inspector_declared_role(view: &NSView) -> Option<&'static str> {
    let ptr = view as *const NSView as usize;
    DECLARED_ROLES.with(|m| m.borrow().get(&ptr).copied())
}

/// Primary display pixel size for [`ScreenDC`].
pub fn mac_screen_size() -> (u32, u32) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return (1920, 1080),
    };
    NSScreen::mainScreen(mtm)
        .map(|s| {
            let f = s.frame();
            (f.size.width.max(1.0) as u32, f.size.height.max(1.0) as u32)
        })
        .unwrap_or((1920, 1080))
}

pub(super) fn listctrl_init_storage(handle: NativeHandle) {
    LISTCTRL_COLUMNS.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    LISTCTRL_ROWS.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    LISTCTRL_SELECTED.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    LISTCTRL_CHECKED.with(|map| {
        map.borrow_mut()
            .insert(handle, RefCell::new(HashSet::new()));
    });
}

pub(super) fn listctrl_effective_row_count(handle: NativeHandle) -> usize {
    LISTCTRL_VIRTUAL_COUNT.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|n| *n as usize)
            .unwrap_or_else(|| listctrl_row_count(handle))
    })
}

pub(super) fn listctrl_cell_text(list_handle: NativeHandle, row: usize, col: usize) -> String {
    if LISTCTRL_VIRTUAL_COUNT.with(|map| map.borrow().contains_key(&list_handle)) {
        return listctrl_virtual_cell_text(list_handle, row, col);
    }
    LISTCTRL_ROWS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|rows| {
                rows.borrow()
                    .get(row)
                    .and_then(|(cells, _)| cells.get(col).cloned())
            })
            .unwrap_or_default()
    })
}

pub(super) fn listctrl_full_row_select(handle: NativeHandle) -> bool {
    const LVS_EX_FULLROWSELECT: u32 = 0x20;
    LISTCTRL_EXT_STYLE
        .with(|map| map.borrow().get(&handle).copied().unwrap_or(0) & LVS_EX_FULLROWSELECT != 0)
}

pub(super) fn listctrl_row_image_index(list_handle: NativeHandle, row: usize) -> Option<i32> {
    if LISTCTRL_VIRTUAL_COUNT.with(|map| map.borrow().contains_key(&list_handle)) {
        return None;
    }
    LISTCTRL_ROWS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|rows| rows.borrow().get(row).and_then(|(_, img)| *img))
    })
}

pub(super) fn listctrl_image_for_row(
    list_handle: NativeHandle,
    img_idx: i32,
) -> Option<Retained<NSImage>> {
    LISTCTRL_IMAGE_LISTS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .copied()
            .and_then(|list_h| image_list_image_at(list_h, img_idx))
    })
}

pub(super) fn listctrl_set_last_hit(handle: NativeHandle, row: usize, col: usize) {
    LISTCTRL_LAST_HIT.with(|map| {
        map.borrow_mut().insert(handle, (row, col));
    });
}

pub(super) fn listctrl_select_model(list_handle: NativeHandle, index: usize) {
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            *sel.borrow_mut() = vec![index];
        }
    });
}

pub(super) fn listctrl_read_columns(list_handle: NativeHandle) -> Vec<(String, i32)> {
    LISTCTRL_COLUMNS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|c| c.borrow().clone())
            .unwrap_or_default()
    })
}

pub(super) fn listctrl_read_selected(list_handle: NativeHandle) -> Vec<usize> {
    LISTCTRL_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|sel| sel.borrow().clone())
            .unwrap_or_default()
    })
}

pub(super) fn listctrl_write_selected(list_handle: NativeHandle, selected: Vec<usize>) {
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            *sel.borrow_mut() = selected;
        }
    });
}

/// Create a multi-column list control (`NSScrollView` + native [`NSTableView`]).
pub fn create_list_ctrl(parent: NativeHandle, command_id: u16, initial_rect: Rect) -> NativeHandle {
    appkit_list_table::create_list_ctrl_native(parent, command_id, initial_rect)
}

/// Mostra/nasconde la freccia di sort sull'header della colonna `col`
/// (equivalente macOS dell'helper Win32 `HDF_SORTUP`/`HDF_SORTDOWN`).
pub fn listctrl_set_header_sort_indicator(
    list_handle: NativeHandle,
    col: i32,
    ascending: bool,
    show: bool,
) {
    appkit_list_table::listctrl_set_header_sort_indicator(list_handle, col, ascending, show);
}

pub fn listctrl_insert_column(list_handle: NativeHandle, index: u32, title: &str, width: i32) {
    LISTCTRL_COLUMNS.with(|map| {
        if let Some(cols) = map.borrow().get(&list_handle) {
            let mut vec = cols.borrow_mut();
            let idx = (index as usize).min(vec.len());
            vec.insert(idx, (title.to_string(), width));
        }
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_insert_item(
    list_handle: NativeHandle,
    index: usize,
    text: &str,
    image_index: Option<i32>,
) -> usize {
    let inserted = LISTCTRL_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&list_handle) {
            let mut vec = rows.borrow_mut();
            let col_count = LISTCTRL_COLUMNS.with(|cmap| {
                cmap.borrow()
                    .get(&list_handle)
                    .map(|c| c.borrow().len())
                    .unwrap_or(1)
                    .max(1)
            });
            let mut cells = vec![String::new(); col_count];
            cells[0] = text.to_string();
            let idx = index.min(vec.len());
            vec.insert(idx, (cells, image_index));
            idx
        } else {
            0
        }
    });
    // Sposta gli indici marcati a partire dalla posizione di inserimento.
    LISTCTRL_CHECKED.with(|map| {
        if let Some(chk) = map.borrow().get(&list_handle) {
            let mut checked = chk.borrow_mut();
            let shifted: HashSet<usize> = checked
                .iter()
                .map(|&i| if i >= inserted { i + 1 } else { i })
                .collect();
            *checked = shifted;
        }
    });
    listctrl_redraw(list_handle);
    inserted
}

pub fn listctrl_set_item_text(
    list_handle: NativeHandle,
    item_index: usize,
    col_index: usize,
    text: &str,
) {
    LISTCTRL_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&list_handle) {
            let mut vec = rows.borrow_mut();
            if let Some((cells, _)) = vec.get_mut(item_index) {
                if col_index >= cells.len() {
                    cells.resize(col_index + 1, String::new());
                }
                cells[col_index] = text.to_string();
            }
        }
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_get_item_text(
    list_handle: NativeHandle,
    item_index: usize,
    col_index: usize,
) -> String {
    LISTCTRL_ROWS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|rows| {
                rows.borrow()
                    .get(item_index)
                    .and_then(|(cells, _)| cells.get(col_index).cloned())
            })
            .unwrap_or_default()
    })
}

pub fn listctrl_get_item_count(list_handle: NativeHandle) -> usize {
    listctrl_row_count(list_handle)
}

pub fn listctrl_delete_item(list_handle: NativeHandle, index: usize) {
    LISTCTRL_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&list_handle) {
            let mut vec = rows.borrow_mut();
            if index < vec.len() {
                vec.remove(index);
            }
        }
    });
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            let mut selected = sel.borrow_mut();
            selected.retain(|&i| i != index);
            for s in selected.iter_mut() {
                if *s > index {
                    *s -= 1;
                }
            }
        }
    });
    LISTCTRL_CHECKED.with(|map| {
        if let Some(chk) = map.borrow().get(&list_handle) {
            let mut checked = chk.borrow_mut();
            let shifted: HashSet<usize> = checked
                .iter()
                .filter(|&&i| i != index)
                .map(|&i| if i > index { i - 1 } else { i })
                .collect();
            *checked = shifted;
        }
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_delete_all_items(list_handle: NativeHandle) {
    LISTCTRL_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&list_handle) {
            rows.borrow_mut().clear();
        }
    });
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            sel.borrow_mut().clear();
        }
    });
    LISTCTRL_CHECKED.with(|map| {
        if let Some(chk) = map.borrow().get(&list_handle) {
            chk.borrow_mut().clear();
        }
    });
    listctrl_redraw(list_handle);
}

/// Abilita/disabilita la colonna dei checkbox (selezione multipla).
pub fn listctrl_set_checkboxes(list_handle: NativeHandle, enabled: bool) {
    LISTCTRL_CHECKBOXES_ENABLED.with(|map| {
        map.borrow_mut().insert(list_handle, enabled);
    });
    listctrl_redraw(list_handle);
}

/// `true` se la colonna dei checkbox è abilitata.
pub fn listctrl_checkboxes_enabled(list_handle: NativeHandle) -> bool {
    LISTCTRL_CHECKBOXES_ENABLED.with(|map| map.borrow().get(&list_handle).copied().unwrap_or(false))
}

/// Marca/smarca il checkbox della riga `index`.
pub fn listctrl_set_checked(list_handle: NativeHandle, index: usize, checked: bool) {
    LISTCTRL_CHECKED.with(|map| {
        if let Some(chk) = map.borrow().get(&list_handle) {
            let mut set = chk.borrow_mut();
            if checked {
                set.insert(index);
            } else {
                set.remove(&index);
            }
        }
    });
    listctrl_redraw(list_handle);
}

/// `true` se il checkbox della riga `index` è marcato.
pub fn listctrl_is_checked(list_handle: NativeHandle, index: usize) -> bool {
    LISTCTRL_CHECKED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|chk| chk.borrow().contains(&index))
            .unwrap_or(false)
    })
}

/// Indici delle righe col checkbox marcato, in ordine crescente.
pub fn listctrl_checked_items(list_handle: NativeHandle) -> Vec<usize> {
    LISTCTRL_CHECKED.with(|map| {
        let mut v: Vec<usize> = map
            .borrow()
            .get(&list_handle)
            .map(|chk| chk.borrow().iter().copied().collect())
            .unwrap_or_default();
        v.sort_unstable();
        v
    })
}

/// Marca/smarca tutti i checkbox in un colpo solo (un redraw alla fine).
pub fn listctrl_set_all_checked(list_handle: NativeHandle, checked: bool) {
    LISTCTRL_CHECKED.with(|map| {
        if let Some(chk) = map.borrow().get(&list_handle) {
            let mut set = chk.borrow_mut();
            set.clear();
            if checked {
                let count = listctrl_effective_row_count(list_handle);
                set.extend(0..count);
            }
        }
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_get_selected_item(list_handle: NativeHandle) -> Option<usize> {
    LISTCTRL_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|sel| sel.borrow().first().copied())
    })
}

pub fn listctrl_get_selected_items(list_handle: NativeHandle) -> Vec<usize> {
    LISTCTRL_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|sel| sel.borrow().clone())
            .unwrap_or_default()
    })
}

pub fn listctrl_select(list_handle: NativeHandle, index: usize) {
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            *sel.borrow_mut() = vec![index];
        }
    });
    if appkit_list_table::listctrl_is_native_table(list_handle) {
        appkit_list_table::listctrl_apply_selection(list_handle, index);
    } else {
        listctrl_redraw(list_handle);
    }
}

pub fn listctrl_deselect(list_handle: NativeHandle, index: usize) {
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            sel.borrow_mut().retain(|&i| i != index);
        }
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_clear_selection(list_handle: NativeHandle) {
    LISTCTRL_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            sel.borrow_mut().clear();
        }
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_is_selected(list_handle: NativeHandle, index: usize) -> bool {
    LISTCTRL_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|sel| sel.borrow().contains(&index))
            .unwrap_or(false)
    })
}

pub fn listctrl_set_image_list(list_handle: NativeHandle, image_list_handle: NativeHandle) {
    LISTCTRL_HAS_IMAGES.with(|map| {
        map.borrow_mut().insert(list_handle, true);
    });
    LISTCTRL_IMAGE_LISTS.with(|map| {
        map.borrow_mut().insert(list_handle, image_list_handle);
    });
    listctrl_redraw(list_handle);
}

pub fn listctrl_redraw_items(list_handle: NativeHandle) {
    listctrl_redraw(list_handle);
}

pub fn listctrl_set_cell_colours(
    list_handle: NativeHandle,
    row: usize,
    col: usize,
    foreground: Option<crate::core::geometry::Colour>,
    background: Option<crate::core::geometry::Colour>,
) {
    let fg = foreground.map(|c| (c.r, c.g, c.b, c.a));
    let bg = background.map(|c| (c.r, c.g, c.b, c.a));
    LISTCTRL_CELL_STYLES.with(|map| {
        map.borrow_mut()
            .entry(list_handle)
            .or_insert_with(|| RefCell::new(HashMap::new()))
            .borrow_mut()
            .insert(
                (row, col),
                GridStyleData {
                    foreground: fg,
                    background: bg,
                },
            );
    });
    listctrl_redraw(list_handle);
}

pub(super) fn listctrl_cell_style(
    list_handle: NativeHandle,
    row: usize,
    col: usize,
) -> Option<GridStyleData> {
    LISTCTRL_CELL_STYLES.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|styles| styles.borrow().get(&(row, col)).copied())
    })
}

pub fn embed_native_view(parent: NativeHandle, child: NativeHandle, rect: Rect) {
    let Some(child_view) = child_view_for_handle(child) else {
        return;
    };
    if let Some(parent_view) = parent_view_for(parent) {
        child_view.removeFromSuperview();
        parent_view.addSubview(&child_view);
        set_view_frame(parent, child, rect);
        record_parent(child, parent);
    }
}

/// Move `child` under `parent` keeping its current frame (Win32 `SetParent`
/// equivalent: the view keeps its geometry instead of being re-laid out).
pub fn reparent_view_keeping_frame(child: NativeHandle, parent: NativeHandle) {
    let Some(child_view) = child_view_for_handle(child) else {
        return;
    };
    if let Some(parent_view) = parent_view_for(parent) {
        let frame = child_view.frame();
        child_view.removeFromSuperview();
        parent_view.addSubview(&child_view);
        child_view.setFrame(frame);
        record_parent(child, parent);
    }
}

// ── Grid (native NSTableView) ───────────────────────────────────────

pub(super) const GRID_ROW_HEIGHT_DEFAULT: f64 = 24.0;

pub(super) fn grid_row_height(handle: NativeHandle) -> f64 {
    GRID_ROW_HEIGHTS.with(|map| {
        map.borrow()
            .get(&handle)
            .copied()
            .unwrap_or(GRID_ROW_HEIGHT_DEFAULT)
    })
}

pub fn grid_set_row_height(grid_handle: NativeHandle, height: f64) {
    let height = height.clamp(16.0, 96.0);
    GRID_ROW_HEIGHTS.with(|map| {
        map.borrow_mut().insert(grid_handle, height);
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub(super) fn grid_image_list_icon_width(grid_handle: NativeHandle) -> Option<f64> {
    GRID_IMAGE_LISTS.with(|map| {
        let list_h = map.borrow().get(&grid_handle).copied()?;
        IMAGE_LISTS.with(|lists| lists.borrow().get(&list_h).map(|s| s.width as f64))
    })
}
pub(super) const GRID_MIN_COLUMN_WIDTH: i32 = 24;

#[path = "appkit_background.rs"]
mod appkit_background;
#[path = "appkit_checklist_table.rs"]
mod appkit_checklist_table;
#[path = "appkit_color_well.rs"]
mod appkit_color_well;
#[path = "appkit_dialogs.rs"]
mod appkit_dialogs;
#[path = "appkit_grid_table.rs"]
mod appkit_grid_table;
#[path = "appkit_hyperlink.rs"]
mod appkit_hyperlink;
#[path = "appkit_list_table.rs"]
mod appkit_list_table;
#[path = "appkit_listbox_table.rs"]
mod appkit_listbox_table;
#[path = "appkit_scroll_bar.rs"]
mod appkit_scroll_bar;
#[path = "appkit_splitter.rs"]
mod appkit_splitter;
#[path = "appkit_static_text.rs"]
mod appkit_static_text;
#[cfg(target_os = "macos")]
#[path = "appkit_toolbar_native.rs"]
mod appkit_toolbar_native;
#[path = "appkit_tree_outline.rs"]
mod appkit_tree_outline;
pub use appkit_dialogs::{
    run_color_panel, run_date_picker_dialog, run_dir_panel, run_file_panel, run_file_panel_multi,
    run_single_choice_alert, run_text_entry_alert, FilePanelOptions,
};
#[path = "appkit_dialog_host.rs"]
mod appkit_dialog_host;
#[path = "appkit_dnd.rs"]
mod appkit_dnd;
pub use appkit_dnd::{enable_frame_file_drop, enable_frame_ole_drop};
#[path = "appkit_drag_source.rs"]
mod appkit_drag_source;
pub use appkit_drag_source::mac_do_drag_drop;
#[path = "appkit_accelerator.rs"]
mod appkit_accelerator;
pub use appkit_accelerator::{
    apply_accelerator_to_menu_item, dispatch_frame_context_menu, replace_window_content_view,
};
#[path = "appkit_menu.rs"]
mod appkit_menu;
pub use appkit_menu::{menu_item_will_dispatch, menu_sync_item_checked, MenuInstallRow};
#[path = "appkit_webview.rs"]
mod appkit_webview;
pub use appkit_webview::{
    create_web_view, webview_load_html, webview_load_url, webview_reload,
    webview_set_event_handler, WebViewNavKind, WEBVIEW_NAV_COMPLETE, WEBVIEW_NAV_ERROR,
    WEBVIEW_NAV_REQUESTED, WEBVIEW_TITLE_CHANGED,
};
#[path = "appkit_print.rs"]
mod appkit_print;
pub use appkit_print::{mac_show_page_setup, mac_show_print_dialog, mac_show_print_preview};
#[path = "appkit_notifications.rs"]
mod appkit_notifications;
pub use appkit_notifications::{
    bundle_supports_user_notifications, notification_set_click_handler, post_user_notification,
};
#[path = "appkit_bitmap.rs"]
mod appkit_bitmap;
pub use appkit_bitmap::{
    mac_bitmap_attach_bgra, mac_bitmap_bgra_dims, mac_bitmap_from_svg, mac_bitmap_image_at,
    mac_bitmap_release, mac_bitmap_store_bgra, mac_bitmap_sync_image, static_bitmap_apply_cached,
};
#[path = "appkit_font.rs"]
mod appkit_font;
pub use appkit_font::{mac_font_from_desc, mac_font_handle};
#[path = "appkit_frame_extras.rs"]
mod appkit_frame_extras;
pub use appkit_frame_extras::{
    mini_frame_create, mini_frame_destroy, mini_frame_set_title, mini_frame_show, splash_close,
    splash_create, splash_set_auto_close, splash_show, tip_close, tip_create, tip_set_text,
    tip_show,
};
#[path = "appkit_frame.rs"]
mod appkit_frame;
pub use appkit_frame::{
    mac_apply_modern_style, mac_dark_title_bar, mac_dpi_for_frame, mac_dpi_for_point,
    mac_enumerate_displays, mac_frame_center, mac_frame_deminiaturize, mac_frame_get_title,
    mac_frame_hide, mac_frame_iconize, mac_frame_is_full_screen, mac_frame_is_iconized,
    mac_frame_is_maximized, mac_frame_maximize, mac_frame_request_attention, mac_frame_restore,
    mac_frame_set_icon, mac_frame_set_icon_from_bitmap, mac_frame_set_max_size,
    mac_frame_set_min_size, mac_frame_set_size, mac_frame_show_full_screen, mac_screen_dpi,
    mac_set_dark_title_bar,
};
#[path = "appkit_dc.rs"]
mod appkit_dc;
pub use appkit_dc::{mac_cg_dc_for_bitmap, mac_cg_dc_new, MacCgDc};
#[path = "appkit_media.rs"]
mod appkit_media;
pub use appkit_media::{
    media_ctrl_close, media_ctrl_create, media_ctrl_destroy, media_ctrl_length_ms, media_ctrl_load,
    media_ctrl_pause, media_ctrl_play, media_ctrl_position_ms, media_ctrl_seek_ms,
    media_ctrl_state, media_ctrl_stop,
};
#[path = "appkit_status.rs"]
mod appkit_status;
pub use appkit_status::{
    status_item_create, status_item_remove, status_item_set_image, status_item_set_left_click,
    status_item_set_menu, status_item_set_tooltip,
};
#[path = "appkit_controls.rs"]
mod appkit_controls;
pub use appkit_controls::{
    create_bitmap_button, create_calendar_ctrl, create_command_link_button, create_date_picker,
    create_group_box, create_radio_box, create_search_field, create_spin_button,
    create_spin_ctrl_double, create_static_line, create_time_picker, create_toggle_button,
    date_picker_get_value, date_picker_set_value, macos_widget_set_enabled, macos_widget_set_frame,
    macos_widget_set_visible, set_group_box_title, spin_button_get_value, spin_button_set_range,
    spin_button_set_value, spin_double_get_value, spin_double_set_digits,
    spin_double_set_increment, spin_double_set_range, spin_double_set_value, time_picker_get_value,
    time_picker_set_value, toggle_button_is_pressed, toggle_button_set_pressed,
};
#[path = "appkit_gl.rs"]
mod appkit_gl;
pub use appkit_gl::{
    create_gl_canvas, gl_canvas_destroy, gl_canvas_has_context, gl_canvas_is_current,
    gl_canvas_make_inactive, gl_canvas_set_current, gl_canvas_swap_buffers,
};
#[path = "appkit_property_grid.rs"]
mod appkit_property_grid;
pub use appkit_dialog_host::{
    create_dialog_window, create_progress_window, dialog_close, dialog_content_view,
    dialog_present, dialog_run_until, dialog_show_modal, progress_close, progress_is_cancelled,
    progress_show, progress_update,
};
pub use appkit_property_grid::{
    create_property_grid, property_grid_close_editor, property_grid_redraw,
    property_grid_set_callbacks, property_grid_set_name_col_w, PropertyGridCallbacks,
    PropertyGridValueKind,
};
#[path = "appkit_header.rs"]
mod appkit_header;
pub use appkit_header::{
    create_header_ctrl, header_insert_column, header_set_column_width, header_set_sort_column,
    header_sync_frame,
};
#[path = "appkit_aui.rs"]
mod appkit_aui;
#[path = "appkit_mdi.rs"]
mod appkit_mdi;
#[path = "appkit_ribbon_panel.rs"]
mod appkit_ribbon_panel;
#[path = "appkit_ribbon_strip.rs"]
mod appkit_ribbon_strip;
pub use appkit_aui::{
    aui_side_caption_width, create_borderless_popup, create_floating_frame, dock_aui_pane_host,
    ensure_pane_host as ensure_aui_pane_host, float_aui_pane_host, floating_frame_close,
    floating_frame_host_child, floating_frame_show, layout_pane_host as layout_aui_pane_host,
    pane_button_commands_for_frame, pane_host_handle as aui_pane_host_handle, popup_close,
    popup_set_opacity, position_pane_host as position_aui_pane_host,
    register_aui_pane_action_target, sync_aui_pane_auto_hide, sync_aui_pane_host_pin,
    sync_aui_sashes, view_backing_scale_factor,
};
pub use appkit_grid_table::{
    create_grid_ctrl, grid_append_column, grid_apply_row_column_highlights,
    grid_autosize_all_columns, grid_autosize_column, grid_column_order, grid_ensure_row_visible,
    grid_rebuild_columns, grid_redraw_view, grid_select_row, grid_set_checkboxes,
    grid_set_column_order, grid_set_column_width, grid_set_sort_indicator, grid_sync_view_geometry,
    GRID_NOTIFY_CELL_EDITED, GRID_NOTIFY_COLUMN_RESIZED, GRID_NOTIFY_CONTEXT_MENU,
    GRID_NOTIFY_EDIT_START, GRID_NOTIFY_FOCUS_CHANGED, GRID_NOTIFY_HEADER_CLICK,
    GRID_NOTIFY_HIGHLIGHT_CLICK, GRID_NOTIFY_ITEM_ACTIVATE, GRID_NOTIFY_ITEM_CHANGED,
    GRID_NOTIFY_LINK_CLICK, GRID_NOTIFY_PASTE_REQUEST, GRID_NOTIFY_ROW_REORDERED,
};
pub use appkit_mdi::{
    mdi_cascade, mdi_child_activate, mdi_child_close, mdi_child_create, mdi_child_maximize,
    mdi_child_restore, mdi_close_all_children, mdi_parent_client_handle, mdi_parent_create,
    mdi_parent_destroy, mdi_parent_show, mdi_tile,
};
pub use appkit_ribbon_panel::{
    create_ribbon_button, create_ribbon_dropdown, create_ribbon_panel, ribbon_button_set_frame,
    ribbon_dropdown_add_item, ribbon_panel_set_frame, ribbon_panel_set_label, set_button_handler,
    start_hover_timer, update_hover_state,
};
pub use appkit_ribbon_strip::{
    create_ribbon_tab_strip, ribbon_strip_add_page, ribbon_strip_page_count,
    ribbon_strip_set_frame, ribbon_strip_set_selection, ribbon_strip_set_selection_handler,
};
pub use appkit_splitter::{
    create_splitter, splitter_resize, splitter_set_enabled, splitter_set_orientation,
    splitter_set_panes, splitter_set_position, splitter_set_visible,
    SplitterOrientation as MacSplitterOrientation,
};

pub fn grid_maybe_autosize_after_refresh(grid_handle: NativeHandle) {
    if grid_autosize_columns_enabled(grid_handle) {
        appkit_grid_table::grid_autosize_all_columns(grid_handle, true);
    }
}

pub fn ns_window_for_frame_handle(handle: NativeHandle) -> Option<Retained<NSWindow>> {
    FRAME_WINDOWS.with(|m| m.borrow().get(&handle).cloned())
}

/// Block until the next AppKit event or run-loop source (e.g. [`NSTimer`]) fires.
///
/// Se esistono handler per messaggi custom la attesa e' limitata a
/// [`RAW_POLL_INTERVAL`], altrimenti un messaggio postato da un worker
/// resterebbe in coda finche' non arriva un evento AppKit.
fn wait_for_next_event() {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let until = if HAS_RAW_HANDLERS.load(Ordering::Relaxed) {
        NSDate::dateWithTimeIntervalSinceNow(RAW_POLL_INTERVAL)
    } else {
        NSDate::distantFuture()
    };
    if let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
        objc2_app_kit::NSEventMask::Any,
        Some(&until),
        unsafe { NSDefaultRunLoopMode },
        true,
    ) {
        app.sendEvent(&event);
    }
}

/// Process pending AppKit events (for progress dialogs and similar).
pub fn pump_pending_events() {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let app = NSApplication::sharedApplication(mtm);
    while let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
        objc2_app_kit::NSEventMask::Any,
        None,
        unsafe { NSDefaultRunLoopMode },
        true,
    ) {
        app.sendEvent(&event);
    }
}

pub fn create_static_bitmap(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let content_h = parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let view = NSImageView::initWithFrame(NSImageView::alloc(mtm), ns_rect);
    view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&view);
    }
    store_ns_view(handle, view.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn static_bitmap_set_image(handle: NativeHandle, image: &NSImage) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&handle) {
            if let Some(iv) = view.downcast_ref::<NSImageView>() {
                iv.setImage(Some(image));
            }
        }
    });
}

/// Rimuove l'immagine dal `NSImageView` (equivalente macOS di
/// `STM_SETIMAGE` con handle nullo usato da `StaticBitmap::clear`
/// su Windows).
pub fn static_bitmap_clear_image(handle: NativeHandle) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&handle) {
            if let Some(iv) = view.downcast_ref::<NSImageView>() {
                iv.setImage(None);
            }
        }
    });
}

pub fn static_bitmap_set_svg_bytes(handle: NativeHandle, svg: &[u8], width: u32, height: u32) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(bgra) = crate::dc::icon::svg_bytes_to_bgra(svg, width, height) else {
        return;
    };
    let image = nsimage_from_bgra(mtm, &bgra, width as i32, height as i32);
    static_bitmap_set_image(handle, &image);
}

pub fn create_rich_text_ctrl(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let content_h = parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(false);
    scroll.setAutohidesScrollers(true);
    let doc_bounds = scroll.contentView().bounds();
    let text = NSTextView::initWithFrame(NSTextView::alloc(mtm), doc_bounds);
    text.setEditable(true);
    text.setRichText(true);
    scroll.setDocumentView(Some(&text));
    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&scroll);
    }
    RICH_TEXT_CTRL.with(|set| set.borrow_mut().insert(handle));
    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn rich_text_set_value(handle: NativeHandle, value: &str) {
    VIEWS.with(|map| {
        let views = map.borrow();
        let Some(view) = views.get(&handle) else {
            return;
        };
        let Some(scroll) = view.downcast_ref::<NSScrollView>() else {
            return;
        };
        let Some(doc) = scroll.documentView() else {
            return;
        };
        if let Some(text) = doc.downcast_ref::<NSTextView>() {
            text.setString(&NSString::from_str(value));
        }
    });
}

pub fn rich_text_get_value(handle: NativeHandle) -> String {
    VIEWS.with(|map| {
        let views = map.borrow();
        let Some(view) = views.get(&handle) else {
            return String::new();
        };
        let Some(scroll) = view.downcast_ref::<NSScrollView>() else {
            return String::new();
        };
        let Some(doc) = scroll.documentView() else {
            return String::new();
        };
        doc.downcast_ref::<NSTextView>()
            .map(|t| t.string().to_string())
            .unwrap_or_default()
    })
}

fn rich_text_view(handle: NativeHandle) -> Option<Retained<NSTextView>> {
    text_view_in_scroll_handle(handle)
}

pub fn rich_text_set_bold(handle: NativeHandle, bold: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let font = text
        .font()
        .unwrap_or_else(|| NSFont::systemFontOfSize(13.0));
    let manager = NSFontManager::sharedFontManager(mtm);
    let new_font = if bold {
        manager.convertFont_toHaveTrait(&font, objc2_app_kit::NSFontTraitMask::BoldFontMask)
    } else {
        manager.convertFont_toNotHaveTrait(&font, objc2_app_kit::NSFontTraitMask::BoldFontMask)
    };
    text.setFont(Some(&new_font));
}

pub fn rich_text_set_italic(handle: NativeHandle, italic: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let font = text
        .font()
        .unwrap_or_else(|| NSFont::systemFontOfSize(13.0));
    let manager = NSFontManager::sharedFontManager(mtm);
    let new_font = if italic {
        manager.convertFont_toHaveTrait(&font, objc2_app_kit::NSFontTraitMask::ItalicFontMask)
    } else {
        manager.convertFont_toNotHaveTrait(&font, objc2_app_kit::NSFontTraitMask::ItalicFontMask)
    };
    text.setFont(Some(&new_font));
}

pub fn rich_text_set_text_colour(handle: NativeHandle, colour: crate::core::geometry::Colour) {
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    let ns = NSColor::colorWithCalibratedRed_green_blue_alpha(
        colour.r as f64 / 255.0,
        colour.g as f64 / 255.0,
        colour.b as f64 / 255.0,
        colour.a as f64 / 255.0,
    );
    if range.length == 0 {
        text.setTextColor(Some(&ns));
        return;
    }
    unsafe {
        if let Some(storage) = text.textStorage() {
            storage.addAttribute_value_range(
                objc2_app_kit::NSForegroundColorAttributeName,
                &ns,
                range,
            );
        }
    }
}

pub fn rich_text_set_background_colour(
    handle: NativeHandle,
    colour: crate::core::geometry::Colour,
) {
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    if range.length == 0 {
        return;
    }
    let ns = NSColor::colorWithCalibratedRed_green_blue_alpha(
        colour.r as f64 / 255.0,
        colour.g as f64 / 255.0,
        colour.b as f64 / 255.0,
        colour.a as f64 / 255.0,
    );
    unsafe {
        if let Some(storage) = text.textStorage() {
            storage.addAttribute_value_range(
                objc2_app_kit::NSBackgroundColorAttributeName,
                &ns,
                range,
            );
        }
    }
}

pub fn rich_text_set_underline(handle: NativeHandle, underline: bool) {
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    if range.length == 0 {
        return;
    }
    unsafe {
        let Some(storage) = text.textStorage() else {
            return;
        };
        if underline {
            let val = objc2_foundation::NSNumber::numberWithInteger(
                objc2_app_kit::NSUnderlineStyle::Single.0,
            );
            storage.addAttribute_value_range(
                objc2_app_kit::NSUnderlineStyleAttributeName,
                &val,
                range,
            );
        } else {
            storage.removeAttribute_range(objc2_app_kit::NSUnderlineStyleAttributeName, range);
        }
    }
}

pub fn rich_text_set_font_size(handle: NativeHandle, size: f64) {
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    let font = NSFont::systemFontOfSize(size.max(6.0));
    if range.length == 0 {
        text.setFont(Some(&font));
        return;
    }
    unsafe {
        if let Some(storage) = text.textStorage() {
            storage.addAttribute_value_range(objc2_app_kit::NSFontAttributeName, &font, range);
        }
    }
}

/// Register `textDidChange:` on a rich-text scroll view (`create_rich_text_ctrl` handle).
pub fn rich_text_enable_change_notify(handle: NativeHandle, command_id: u16) {
    text_view_enable_change_notify(handle, command_id);
}

pub fn rich_text_set_strikethrough(handle: NativeHandle, strike: bool) {
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    if range.length == 0 {
        return;
    }
    unsafe {
        let Some(storage) = text.textStorage() else {
            return;
        };
        if strike {
            let val = objc2_foundation::NSNumber::numberWithInteger(
                objc2_app_kit::NSUnderlineStyle::Single.0,
            );
            storage.addAttribute_value_range(
                objc2_app_kit::NSStrikethroughStyleAttributeName,
                &val,
                range,
            );
        } else {
            storage.removeAttribute_range(objc2_app_kit::NSStrikethroughStyleAttributeName, range);
        }
    }
}

/// `align`: 0 = left, 1 = center, 2 = right, 3 = justified.
pub fn rich_text_set_alignment(handle: NativeHandle, align: u8) {
    let Some(_mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    let ns_align = match align {
        1 => NSTextAlignment::Center,
        2 => NSTextAlignment::Right,
        3 => NSTextAlignment::Justified,
        _ => NSTextAlignment::Left,
    };
    let mut_style = NSMutableParagraphStyle::new();
    mut_style.setAlignment(ns_align);
    let apply_range = if range.length == 0 {
        NSRange::new(0, text.string().len())
    } else {
        range
    };
    unsafe {
        if let Some(storage) = text.textStorage() {
            storage.addAttribute_value_range(
                objc2_app_kit::NSParagraphStyleAttributeName,
                &mut_style,
                apply_range,
            );
        }
    }
}

pub fn rich_text_set_link(handle: NativeHandle, url: &str) {
    let Some(text) = rich_text_view(handle) else {
        return;
    };
    let range = text.selectedRange();
    if range.length == 0 {
        return;
    }
    unsafe {
        if let Some(storage) = text.textStorage() {
            storage.addAttribute_value_range(
                objc2_app_kit::NSLinkAttributeName,
                &NSString::from_str(url),
                range,
            );
        }
    }
}

pub fn rich_text_can_undo(handle: NativeHandle) -> bool {
    rich_text_view(handle)
        .and_then(|text| text.undoManager())
        .is_some_and(|mgr| mgr.canUndo())
}

pub fn rich_text_can_redo(handle: NativeHandle) -> bool {
    rich_text_view(handle)
        .and_then(|text| text.undoManager())
        .is_some_and(|mgr| mgr.canRedo())
}

pub fn rich_text_undo(handle: NativeHandle) {
    if let Some(mgr) = rich_text_view(handle).and_then(|text| text.undoManager()) {
        mgr.undo();
    }
}

pub fn rich_text_redo(handle: NativeHandle) {
    if let Some(mgr) = rich_text_view(handle).and_then(|text| text.undoManager()) {
        mgr.redo();
    }
}

pub fn rich_text_set_html(handle: NativeHandle, html: &str) -> bool {
    let mut plain = html.to_string();
    for (from, to) in [
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("<br>", "\n"),
        ("</p>", "\n"),
        ("</div>", "\n"),
        ("</li>", "\n"),
    ] {
        plain = plain.replace(from, to);
    }
    for tag in [
        "<p>", "<div>", "<li>", "<html>", "<body>", "</html>", "</body>",
    ] {
        plain = plain.replace(tag, "");
    }
    while let Some(start) = plain.find('<') {
        let Some(rel_end) = plain[start..].find('>') else {
            break;
        };
        plain.replace_range(start..start + rel_end + 1, "");
    }
    rich_text_set_value(handle, plain.trim());
    true
}

pub fn rich_text_get_html(handle: NativeHandle) -> String {
    let Some(text) = rich_text_view(handle) else {
        return String::new();
    };
    let plain = text.string().to_string();
    if plain.is_empty() {
        return String::new();
    }
    let escaped = plain
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!("<html><body><p>{escaped}</p></body></html>")
}

fn install_frame_content_host(
    mtm: MainThreadMarker,
    window: &NSWindow,
    frame_handle: NativeHandle,
) -> Retained<NSView> {
    appkit_accelerator::replace_window_content_view(mtm, window, frame_handle)
}

pub fn create_scroll_bar(parent: NativeHandle, initial_rect: Rect, vertical: bool) -> NativeHandle {
    appkit_scroll_bar::create_scroll_bar_native(parent, initial_rect, vertical)
}

pub fn scroll_bar_set_range(handle: NativeHandle, min: i32, max: i32, page: i32, value: i32) {
    appkit_scroll_bar::scroll_bar_set_range_native(handle, min, max, page, value);
}

pub fn scroll_bar_get_position(handle: NativeHandle) -> i32 {
    appkit_scroll_bar::scroll_bar_get_position_native(handle)
}

pub fn scroll_bar_set_enabled(handle: NativeHandle, enabled: bool) {
    appkit_scroll_bar::scroll_bar_set_enabled_native(handle, enabled);
}

pub fn create_colour_picker(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    colour: Colour,
) -> NativeHandle {
    appkit_color_well::create_colour_picker_native(parent, command_id, initial_rect, colour)
}

pub fn colour_picker_set_colour(handle: NativeHandle, colour: Colour) {
    appkit_color_well::colour_picker_set_colour(handle, colour);
}

pub fn colour_picker_get_colour(handle: NativeHandle) -> Colour {
    appkit_color_well::colour_picker_get_colour(handle)
}

pub fn colour_picker_activate(handle: NativeHandle) {
    appkit_color_well::colour_picker_activate(handle);
}

pub fn create_hyperlink(
    parent: NativeHandle,
    label: &str,
    url: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    appkit_hyperlink::create_hyperlink_native(parent, label, url, command_id, initial_rect)
}

pub fn hyperlink_update(handle: NativeHandle, label: &str, url: &str) {
    appkit_hyperlink::hyperlink_update(handle, label, url);
}

fn grid_row_count(handle: NativeHandle) -> usize {
    GRID_ROWS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|rows| rows.borrow().len())
            .unwrap_or(0)
    })
}

pub fn grid_update_sort_state(
    grid_handle: NativeHandle,
    sort_col: Option<usize>,
    ascending: Option<bool>,
) {
    GRID_SORT.with(|map| {
        if let Some(sort) = map.borrow().get(&grid_handle) {
            *sort.borrow_mut() = (sort_col, ascending);
        }
    });
}

pub fn grid_native_row_count(handle: NativeHandle) -> usize {
    grid_row_count(handle)
}

pub(super) fn grid_col_count(handle: NativeHandle) -> usize {
    GRID_COLUMNS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|c| c.borrow().len())
            .unwrap_or(0)
    })
}

pub(super) fn ns_color_from_rgba(rgba: Rgba) -> Retained<objc2_app_kit::NSColor> {
    let (r, g, b, a) = rgba;
    objc2_app_kit::NSColor::colorWithCalibratedRed_green_blue_alpha(
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        a as f64 / 255.0,
    )
}

pub(super) fn grid_appearance(handle: NativeHandle) -> GridAppearanceData {
    GRID_APPEARANCE.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|a| *a.borrow())
            .unwrap_or_default()
    })
}

pub(super) fn grid_body_font(
    _mtm: MainThreadMarker,
    handle: NativeHandle,
) -> Retained<objc2_app_kit::NSFont> {
    let (name, size) = GRID_FONT.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|f| f.borrow().clone())
            .unwrap_or_else(|| (String::new(), 11.0))
    });
    if name.is_empty() {
        objc2_app_kit::NSFont::systemFontOfSize(size)
    } else if let Some(font) =
        objc2_app_kit::NSFont::fontWithName_size(&NSString::from_str(&name), size)
    {
        font
    } else {
        objc2_app_kit::NSFont::systemFontOfSize(size)
    }
}

pub(super) fn grid_set_field_alignment(field: &NSTextField, align: i32) {
    let alignment = match align {
        1 => NSTextAlignment::Right,
        2 => NSTextAlignment::Center,
        _ => NSTextAlignment::Left,
    };
    field.setAlignment(alignment);
}

pub(super) fn grid_resolve_style(
    handle: NativeHandle,
    row: usize,
    col: usize,
    selected: bool,
) -> (Rgba, Rgba) {
    let appearance = grid_appearance(handle);
    if selected {
        return (
            appearance.selection_foreground,
            appearance.selection_background,
        );
    }
    if let Some(style) = GRID_CELL_STYLES.with(|map| {
        map.borrow()
            .get(&handle)
            .and_then(|m| m.borrow().get(&(row, col)).copied())
    }) {
        let fg = style.foreground.unwrap_or(appearance.default_text);
        let bg = style
            .background
            .unwrap_or_else(|| grid_alternating_bg(handle, row, &appearance));
        return (fg, bg);
    }
    if let Some(style) = GRID_ROW_STYLES.with(|map| {
        map.borrow()
            .get(&handle)
            .and_then(|m| m.borrow().get(&row).copied())
    }) {
        let fg = style.foreground.unwrap_or(appearance.default_text);
        let bg = style
            .background
            .unwrap_or_else(|| grid_alternating_bg(handle, row, &appearance));
        return (fg, bg);
    }
    let highlight_row = GRID_HIGHLIGHT_ROW.with(|map| map.borrow().get(&handle).and_then(|r| *r));
    let highlight_col = GRID_HIGHLIGHT_COL.with(|map| map.borrow().get(&handle).and_then(|c| *c));
    let in_row = highlight_row == Some(row);
    let in_col = highlight_col == Some(col);
    if in_row || in_col {
        let _base = if appearance.system_theme {
            (255, 255, 255, 255)
        } else {
            grid_alternating_bg(handle, row, &appearance)
        };
        let bg = if in_row && in_col {
            HIGHLIGHT_BOTH_BG
        } else if in_row {
            HIGHLIGHT_ROW_BG
        } else {
            HIGHLIGHT_COL_BG
        };
        return (appearance.default_text, bg);
    }
    if appearance.system_theme {
        return (appearance.default_text, (255, 255, 255, 255));
    }
    (
        appearance.default_text,
        grid_alternating_bg(handle, row, &appearance),
    )
}

pub(super) fn grid_alternating_bg(
    handle: NativeHandle,
    row: usize,
    appearance: &GridAppearanceData,
) -> Rgba {
    if let Some((even, odd)) =
        GRID_ALTERNATING.with(|map| map.borrow().get(&handle).and_then(|a| *a.borrow()))
    {
        if row.is_multiple_of(2) {
            even
        } else {
            odd
        }
    } else if appearance.system_theme {
        (255, 255, 255, 255)
    } else if row.is_multiple_of(2) {
        appearance.alternating_even
    } else {
        appearance.alternating_odd
    }
}

pub(super) fn grid_header_title(_handle: NativeHandle, _col: usize, title: &str) -> String {
    title.to_string()
}

pub(super) fn grid_col_sort_key(col: usize) -> String {
    format!("ru_wx_col_{col}")
}

pub(super) fn grid_col_from_sort_key(key: &str) -> Option<usize> {
    key.strip_prefix("ru_wx_col_")?.parse().ok()
}

const HIGHLIGHT_ROW_BG: Rgba = (255, 249, 219, 255);
const HIGHLIGHT_COL_BG: Rgba = (232, 244, 255, 255);
const HIGHLIGHT_BOTH_BG: Rgba = (225, 236, 252, 255);

pub(super) fn grid_row_col_highlight_enabled(handle: NativeHandle) -> bool {
    GRID_ROW_COL_HIGHLIGHT.with(|map| map.borrow().get(&handle).copied().unwrap_or(false))
}

pub fn grid_set_row_col_highlight_enabled(grid_handle: NativeHandle, enabled: bool) {
    GRID_ROW_COL_HIGHLIGHT.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
    if !enabled {
        grid_set_row_column_highlights(grid_handle, None, None);
    }
}

pub fn grid_set_row_column_highlights(
    grid_handle: NativeHandle,
    row: Option<usize>,
    col: Option<usize>,
) {
    GRID_HIGHLIGHT_ROW.with(|map| {
        map.borrow_mut().insert(grid_handle, row);
    });
    GRID_HIGHLIGHT_COL.with(|map| {
        map.borrow_mut().insert(grid_handle, col);
    });
    appkit_grid_table::grid_apply_row_column_highlights(grid_handle);
}

pub fn grid_take_highlight_click(
    grid_handle: NativeHandle,
) -> Option<(Option<usize>, Option<usize>)> {
    GRID_HIGHLIGHT_CLICK.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_set_row_count(grid_handle: NativeHandle, count: usize) {
    let col_count = grid_col_count(grid_handle).max(1);
    GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&grid_handle) {
            let mut vec = rows.borrow_mut();
            vec.resize_with(count, || {
                (vec![String::new(); col_count], vec![None; col_count])
            });
            vec.truncate(count);
        }
    });
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            let mut vec = kinds.borrow_mut();
            vec.resize_with(count, || vec![None; col_count]);
            vec.truncate(count);
        }
    });
    GRID_ROW_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&grid_handle) {
            let mut vec = checked.borrow_mut();
            vec.resize(count, false);
            vec.truncate(count);
        }
    });
    GRID_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&grid_handle) {
            sel.borrow_mut().clear();
        }
    });
}

pub fn grid_set_cell(
    grid_handle: NativeHandle,
    row: usize,
    col: usize,
    text: &str,
    image_index: Option<i32>,
    progress: Option<GridProgressCell>,
) {
    GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&grid_handle) {
            let mut vec = rows.borrow_mut();
            if row >= vec.len() {
                return;
            }
            let col_count = grid_col_count(grid_handle).max(col + 1);
            let (texts, images) = &mut vec[row];
            if texts.len() < col_count {
                texts.resize(col_count, String::new());
            }
            if images.len() < col_count {
                images.resize(col_count, None);
            }
            if col < texts.len() {
                texts[col] = text.to_string();
            }
            if col < images.len() {
                images[col] = image_index.filter(|idx| *idx >= 0);
            }
        }
    });
    grid_set_cell_progress_kind(grid_handle, row, col, progress);
}

pub fn grid_set_image_list(grid_handle: NativeHandle, image_list_handle: NativeHandle) {
    GRID_IMAGE_LISTS.with(|map| {
        map.borrow_mut().insert(grid_handle, image_list_handle);
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_get_selected_row(grid_handle: NativeHandle) -> Option<usize> {
    grid_get_selected_rows(grid_handle).into_iter().next()
}

pub fn grid_get_selected_rows(grid_handle: NativeHandle) -> Vec<usize> {
    GRID_SELECTED.with(|map| {
        map.borrow()
            .get(&grid_handle)
            .map(|sel| sel.borrow().clone())
            .unwrap_or_default()
    })
}

pub fn grid_set_label_tips(grid_handle: NativeHandle, enabled: bool) {
    GRID_LABEL_TIPS.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_set_autosize_columns(grid_handle: NativeHandle, enabled: bool) {
    GRID_AUTOSIZE_COLUMNS.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
    if enabled {
        appkit_grid_table::grid_autosize_all_columns(grid_handle, true);
    }
}

pub fn grid_set_cell_tooltip_provider<F>(grid_handle: NativeHandle, f: F)
where
    F: Fn(usize, usize) -> Option<String> + 'static,
{
    GRID_TOOLTIP_PROVIDERS.with(|map| {
        map.borrow_mut().insert(grid_handle, Box::new(f));
    });
    grid_set_label_tips(grid_handle, true);
}

pub(super) fn grid_label_tips_enabled(handle: NativeHandle) -> bool {
    GRID_LABEL_TIPS.with(|map| map.borrow().get(&handle).copied().unwrap_or(false))
}

pub(super) fn grid_autosize_columns_enabled(handle: NativeHandle) -> bool {
    GRID_AUTOSIZE_COLUMNS.with(|map| map.borrow().get(&handle).copied().unwrap_or(false))
}

pub(super) fn grid_resolve_cell_tooltip(
    handle: NativeHandle,
    row: usize,
    col: usize,
    display_text: &str,
    visible_text_width: f64,
) -> Option<String> {
    if let Some(tip) =
        GRID_TOOLTIP_PROVIDERS.with(|map| map.borrow().get(&handle).and_then(|f| f(row, col)))
    {
        return Some(tip);
    }
    if !grid_label_tips_enabled(handle) || display_text.is_empty() {
        return None;
    }
    let mtm = MainThreadMarker::new()?;
    let needed = appkit_grid_table::grid_measure_text_width(mtm, handle, display_text);
    if needed > visible_text_width + 2.0 {
        Some(display_text.to_string())
    } else {
        None
    }
}

pub fn grid_is_row_checked(grid_handle: NativeHandle, row: usize) -> bool {
    GRID_ROW_CHECKED.with(|map| {
        map.borrow()
            .get(&grid_handle)
            .and_then(|checked| checked.borrow().get(row).copied())
            .unwrap_or(false)
    })
}

pub fn grid_set_row_checked(grid_handle: NativeHandle, row: usize, checked: bool) {
    GRID_ROW_CHECKED.with(|map| {
        if let Some(vec) = map.borrow().get(&grid_handle) {
            let mut checked_rows = vec.borrow_mut();
            if row >= checked_rows.len() {
                checked_rows.resize(row + 1, false);
            }
            checked_rows[row] = checked;
        }
    });
    appkit_grid_table::grid_reload_display_rows(grid_handle, &[row]);
}

pub fn grid_apply_column_widths(grid_handle: NativeHandle, widths: &[(usize, i32)]) {
    GRID_COLUMNS.with(|map| {
        if let Some(cols) = map.borrow().get(&grid_handle) {
            let mut vec = cols.borrow_mut();
            for (idx, width) in widths {
                if let Some(entry) = vec.get_mut(*idx) {
                    entry.1 = *width;
                }
            }
        }
    });
    GRID_COLUMN_RESIZE_SYNCING.with(|m| m.borrow_mut().insert(grid_handle, true));
    for (idx, width) in widths {
        appkit_grid_table::grid_set_column_width(grid_handle, *idx, *width);
    }
    GRID_COLUMN_RESIZE_SYNCING.with(|m| m.borrow_mut().remove(&grid_handle));
}

fn style_from_options(fg: Option<Rgba>, bg: Option<Rgba>) -> GridStyleData {
    GridStyleData {
        foreground: fg,
        background: bg,
    }
}

pub fn grid_set_appearance(grid_handle: NativeHandle, appearance: GridAppearanceData) {
    GRID_APPEARANCE.with(|map| {
        if let Some(entry) = map.borrow().get(&grid_handle) {
            *entry.borrow_mut() = appearance;
        }
    });
    appkit_grid_table::grid_apply_header_appearance(grid_handle);
    appkit_grid_table::grid_reload(grid_handle);
}

/// Wire macOS header theming (called from [`Grid::ensure_visual_hooks`]).
pub fn grid_install_macos_visual_hooks(grid_handle: NativeHandle) {
    appkit_grid_table::grid_apply_header_appearance(grid_handle);
    appkit_grid_table::grid_install_frozen_scroll_sync(grid_handle);
}

pub fn grid_set_alternating_colors(grid_handle: NativeHandle, even: Rgba, odd: Rgba) {
    GRID_ALTERNATING.with(|map| {
        if let Some(entry) = map.borrow().get(&grid_handle) {
            *entry.borrow_mut() = Some((even, odd));
        }
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_clear_alternating_colors(grid_handle: NativeHandle) {
    GRID_ALTERNATING.with(|map| {
        if let Some(entry) = map.borrow().get(&grid_handle) {
            *entry.borrow_mut() = None;
        }
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_set_cell_style(
    grid_handle: NativeHandle,
    row: usize,
    col: usize,
    foreground: Option<Rgba>,
    background: Option<Rgba>,
) {
    GRID_CELL_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            styles
                .borrow_mut()
                .insert((row, col), style_from_options(foreground, background));
        }
    });
    appkit_grid_table::grid_reload_display_rows(grid_handle, &[row]);
}

pub fn grid_set_row_style(
    grid_handle: NativeHandle,
    row: usize,
    foreground: Option<Rgba>,
    background: Option<Rgba>,
) {
    GRID_ROW_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            styles
                .borrow_mut()
                .insert(row, style_from_options(foreground, background));
        }
    });
    appkit_grid_table::grid_reload_display_rows(grid_handle, &[row]);
}

pub fn grid_clear_row_style(grid_handle: NativeHandle, row: usize) {
    GRID_ROW_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            styles.borrow_mut().remove(&row);
        }
    });
    appkit_grid_table::grid_reload_display_rows(grid_handle, &[row]);
}

pub fn grid_clear_all_row_styles(grid_handle: NativeHandle) {
    GRID_ROW_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            styles.borrow_mut().clear();
        }
    });
    GRID_CELL_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            styles.borrow_mut().clear();
        }
    });
    appkit_grid_table::grid_reload(grid_handle);
}

fn grid_shift_styles_after_row_delete(grid_handle: NativeHandle, deleted_row: usize) {
    GRID_ROW_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            let mut m = styles.borrow_mut();
            m.remove(&deleted_row);
            let keys: Vec<usize> = m.keys().copied().filter(|r| *r > deleted_row).collect();
            for r in keys {
                if let Some(s) = m.remove(&r) {
                    m.insert(r - 1, s);
                }
            }
        }
    });
    GRID_CELL_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            let mut m = styles.borrow_mut();
            m.retain(|(r, _), _| *r != deleted_row);
            let keys: Vec<(usize, usize)> = m
                .keys()
                .copied()
                .filter(|(r, _)| *r > deleted_row)
                .collect();
            for (r, c) in keys {
                if let Some(s) = m.remove(&(r, c)) {
                    m.insert((r - 1, c), s);
                }
            }
        }
    });
}

fn grid_shift_cell_styles_after_col_remove(grid_handle: NativeHandle, removed_col: usize) {
    GRID_CELL_STYLES.with(|map| {
        if let Some(styles) = map.borrow().get(&grid_handle) {
            let mut m = styles.borrow_mut();
            m.retain(|(_, c), _| *c != removed_col);
            let keys: Vec<(usize, usize)> = m
                .keys()
                .copied()
                .filter(|(_, c)| *c > removed_col)
                .collect();
            for (r, c) in keys {
                if let Some(s) = m.remove(&(r, c)) {
                    m.insert((r, c - 1), s);
                }
            }
        }
    });
}

pub fn grid_append_row(grid_handle: NativeHandle) {
    let col_count = grid_col_count(grid_handle).max(1);
    GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&grid_handle) {
            rows.borrow_mut()
                .push((vec![String::new(); col_count], vec![None; col_count]));
        }
    });
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            kinds.borrow_mut().push(vec![None; col_count]);
        }
    });
    GRID_ROW_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&grid_handle) {
            checked.borrow_mut().push(false);
        }
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_delete_row(grid_handle: NativeHandle, display_row: usize) -> bool {
    let ok = GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&grid_handle) {
            let mut vec = rows.borrow_mut();
            if display_row >= vec.len() {
                return false;
            }
            vec.remove(display_row);
            true
        } else {
            false
        }
    });
    if !ok {
        return false;
    }
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            let mut vec = kinds.borrow_mut();
            if display_row < vec.len() {
                vec.remove(display_row);
            }
        }
    });
    GRID_ROW_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&grid_handle) {
            let mut vec = checked.borrow_mut();
            if display_row < vec.len() {
                vec.remove(display_row);
            }
        }
    });
    GRID_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&grid_handle) {
            sel.borrow_mut().clear();
        }
    });
    grid_shift_styles_after_row_delete(grid_handle, display_row);
    appkit_grid_table::grid_reload(grid_handle);
    true
}

pub fn grid_remove_column(grid_handle: NativeHandle, col: usize) -> bool {
    let col_count = grid_col_count(grid_handle);
    if col >= col_count || col_count <= 1 {
        return false;
    }
    GRID_COLUMNS.with(|map| {
        if let Some(cols) = map.borrow().get(&grid_handle) {
            cols.borrow_mut().remove(col);
        }
    });
    GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&grid_handle) {
            let mut vec = rows.borrow_mut();
            for (texts, images) in vec.iter_mut() {
                if col < texts.len() {
                    texts.remove(col);
                }
                if col < images.len() {
                    images.remove(col);
                }
            }
        }
    });
    GRID_CELL_KINDS.with(|map| {
        if let Some(kinds) = map.borrow().get(&grid_handle) {
            let mut vec = kinds.borrow_mut();
            for row in vec.iter_mut() {
                if col < row.len() {
                    row.remove(col);
                }
            }
        }
    });
    grid_shift_cell_styles_after_col_remove(grid_handle, col);
    appkit_grid_table::grid_rebuild_columns(grid_handle);
    appkit_grid_table::grid_reload(grid_handle);
    true
}

pub fn grid_set_font(grid_handle: NativeHandle, family: &str, size: f64) {
    GRID_FONT.with(|map| {
        if let Some(font) = map.borrow().get(&grid_handle) {
            *font.borrow_mut() = (family.to_string(), size);
        }
    });
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_take_header_sort_click(grid_handle: NativeHandle) -> Option<(usize, bool)> {
    GRID_HEADER_SORT_CLICK.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_take_header_click_col(grid_handle: NativeHandle) -> Option<usize> {
    grid_take_header_sort_click(grid_handle).map(|(col, _)| col)
}

pub fn grid_take_activate_cell(grid_handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_ACTIVATE_CELL.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_take_context_menu_target(grid_handle: NativeHandle) -> Option<(bool, usize, usize)> {
    GRID_CONTEXT_MENU.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_take_row_reordered(grid_handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_ROW_REORDERED.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_take_column_resized(grid_handle: NativeHandle) -> Option<(usize, i32)> {
    GRID_COLUMN_RESIZED.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub(super) fn grid_column_resize_syncing(handle: NativeHandle) -> bool {
    GRID_COLUMN_RESIZE_SYNCING.with(|m| m.borrow().get(&handle).copied().unwrap_or(false))
}

pub(super) fn grid_begin_column_resize_sync(handle: NativeHandle) {
    GRID_COLUMN_RESIZE_SYNCING.with(|m| {
        m.borrow_mut().insert(handle, true);
    });
}

pub(super) fn grid_end_column_resize_sync(handle: NativeHandle) {
    GRID_COLUMN_RESIZE_SYNCING.with(|m| {
        m.borrow_mut().remove(&handle);
    });
}

/// Widths stored in the AppKit column model (parallel to data columns).
pub fn grid_snapshot_column_widths(grid_handle: NativeHandle) -> Vec<i32> {
    GRID_COLUMNS.with(|m| {
        m.borrow()
            .get(&grid_handle)
            .map(|c| c.borrow().iter().map(|(_, w, _)| *w).collect())
            .unwrap_or_default()
    })
}

pub(super) fn grid_schedule_column_resize_notify(
    grid_handle: NativeHandle,
    frame_handle: NativeHandle,
    command_id: u16,
) {
    GRID_COLUMN_RESIZE_DIRTY.with(|m| {
        m.borrow_mut().insert(grid_handle, true);
    });
    let already = GRID_COLUMN_RESIZE_SCHEDULED.with(|m| {
        if m.borrow().get(&grid_handle).copied().unwrap_or(false) {
            return true;
        }
        m.borrow_mut().insert(grid_handle, true);
        false
    });
    if already {
        return;
    }
    run_on_next_event(move || {
        GRID_COLUMN_RESIZE_SCHEDULED.with(|m| m.borrow_mut().remove(&grid_handle));
        let dirty = GRID_COLUMN_RESIZE_DIRTY
            .with(|m| m.borrow_mut().remove(&grid_handle))
            .is_some();
        if dirty {
            appkit_grid_table::grid_note_row_heights_changed(grid_handle);
            dispatch_notify(
                frame_handle,
                command_id,
                appkit_grid_table::GRID_NOTIFY_COLUMN_RESIZED,
            );
        }
    });
}

pub fn grid_set_column_hidden(grid_handle: NativeHandle, col: usize, hidden: bool) {
    GRID_HIDDEN_COLS.with(|map| {
        let mut borrowed = map.borrow_mut();
        let entry = borrowed
            .entry(grid_handle)
            .or_insert_with(|| RefCell::new(Vec::new()));
        let mut cols = entry.borrow_mut();
        if col >= cols.len() {
            cols.resize(col + 1, false);
        }
        cols[col] = hidden;
    });
    appkit_grid_table::grid_apply_column_hidden(grid_handle, col, hidden);
}

pub fn grid_is_column_hidden(grid_handle: NativeHandle, col: usize) -> bool {
    GRID_HIDDEN_COLS.with(|map| {
        map.borrow()
            .get(&grid_handle)
            .and_then(|c| c.borrow().get(col).copied())
            .unwrap_or(false)
    })
}

pub(super) fn grid_row_labels_enabled(handle: NativeHandle) -> bool {
    GRID_ROW_LABELS.with(|m| m.borrow().get(&handle).map(|(e, _)| *e).unwrap_or(false))
}

pub(super) fn grid_row_label_width(handle: NativeHandle) -> i32 {
    GRID_ROW_LABELS.with(|m| m.borrow().get(&handle).map(|(_, w)| *w).unwrap_or(40))
}

pub fn grid_enable_row_labels(grid_handle: NativeHandle, enabled: bool, width: i32) {
    GRID_ROW_LABELS.with(|map| {
        map.borrow_mut()
            .insert(grid_handle, (enabled, width.max(20)));
    });
    appkit_grid_table::grid_rebuild_columns(grid_handle);
    appkit_grid_table::grid_reload(grid_handle);
}

pub fn grid_set_frozen_columns(grid_handle: NativeHandle, count: usize) {
    GRID_FROZEN_COL_COUNT.with(|map| {
        map.borrow_mut().insert(grid_handle, count);
    });
    appkit_grid_table::grid_sync_frozen_panes(grid_handle);
}

pub fn grid_set_frozen_rows(grid_handle: NativeHandle, count: usize) {
    GRID_FROZEN_ROW_COUNT.with(|map| {
        map.borrow_mut().insert(grid_handle, count);
    });
    appkit_grid_table::grid_sync_frozen_panes(grid_handle);
}

pub fn grid_set_row_drag_enabled(grid_handle: NativeHandle, enabled: bool) {
    GRID_ROW_DRAG_ENABLED.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
    appkit_grid_table::grid_apply_row_drag(grid_handle);
}

pub(super) fn grid_record_row_reordered(grid_handle: NativeHandle, from: usize, to: usize) {
    GRID_ROW_REORDERED.with(|map| {
        map.borrow_mut().insert(grid_handle, (from, to));
    });
}

pub(super) fn grid_frozen_col_count(handle: NativeHandle) -> usize {
    GRID_FROZEN_COL_COUNT.with(|map| map.borrow().get(&handle).copied().unwrap_or(0))
}

pub(super) fn grid_frozen_row_count(handle: NativeHandle) -> usize {
    GRID_FROZEN_ROW_COUNT.with(|map| map.borrow().get(&handle).copied().unwrap_or(0))
}

pub(super) fn grid_row_drag_enabled(handle: NativeHandle) -> bool {
    GRID_ROW_DRAG_ENABLED.with(|map| map.borrow().get(&handle).copied().unwrap_or(false))
}

pub(super) fn grid_replace_root_view(handle: NativeHandle, view: Retained<NSView>) {
    store_ns_view(handle, view);
}

pub(super) fn grid_clone_root_view(handle: NativeHandle) -> Option<Retained<NSView>> {
    VIEWS.with(|map| map.borrow().get(&handle).cloned())
}

pub fn grid_set_focused_cell(grid_handle: NativeHandle, row: usize, col: usize) {
    GRID_FOCUSED_CELL.with(|map| {
        map.borrow_mut().insert(grid_handle, (row, col));
    });
}

pub fn grid_get_focused_cell(grid_handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_FOCUSED_CELL.with(|map| map.borrow().get(&grid_handle).copied())
}

pub fn grid_take_cell_edited(grid_handle: NativeHandle) -> Option<(usize, usize, String)> {
    GRID_CELL_EDITED.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_take_paste_at(grid_handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_PASTE_AT.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_take_edit_start(grid_handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_EDIT_START.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_queue_link_click(grid_handle: NativeHandle, row: usize, col: usize) {
    GRID_LINK_CLICK.with(|map| {
        map.borrow_mut().insert(grid_handle, (row, col));
    });
}

pub fn grid_take_link_click(grid_handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_LINK_CLICK.with(|map| map.borrow_mut().remove(&grid_handle))
}

pub fn grid_set_link_clicks_enabled(grid_handle: NativeHandle, enabled: bool) {
    GRID_LINK_CLICKS.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
}

pub(super) fn grid_link_clicks_enabled(handle: NativeHandle) -> bool {
    GRID_LINK_CLICKS.with(|m| m.borrow().get(&handle).copied().unwrap_or(false))
}

pub fn grid_set_cell_editing_enabled(grid_handle: NativeHandle, enabled: bool) {
    GRID_CELL_EDITING.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
}

pub(super) fn grid_cell_editing_enabled(handle: NativeHandle) -> bool {
    GRID_CELL_EDITING.with(|m| m.borrow().get(&handle).copied().unwrap_or(false))
}

pub fn grid_begin_cell_edit(grid_handle: NativeHandle) -> bool {
    appkit_grid_table::grid_begin_inline_edit(grid_handle)
}

pub fn grid_end_cell_edit(grid_handle: NativeHandle, accept: bool) -> bool {
    appkit_grid_table::grid_end_inline_edit(grid_handle, accept)
}

pub fn grid_queue_paste_at(grid_handle: NativeHandle, row: usize, col: usize) {
    GRID_PASTE_AT.with(|map| {
        map.borrow_mut().insert(grid_handle, (row, col));
    });
}

pub fn grid_queue_edit_start(grid_handle: NativeHandle, row: usize, col: usize) {
    GRID_EDIT_START.with(|map| {
        map.borrow_mut().insert(grid_handle, (row, col));
    });
}

pub fn grid_record_cell_edited(grid_handle: NativeHandle, row: usize, col: usize, text: String) {
    GRID_CELL_EDITED.with(|map| {
        map.borrow_mut().insert(grid_handle, (row, col, text));
    });
}

// ── Scrolled window (NSScrollView + document NSView) ────────────────

/// Create a scrollable container (`NSScrollView` + document [`NSView`]).
pub fn create_scrolled_window(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);

    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);

    let doc = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(ns_rect.size.width, ns_rect.size.height),
        ),
    );
    scroll.setDocumentView(Some(&doc));
    content.addSubview(&scroll);

    SCROLLED_DOCUMENTS.with(|map| map.borrow_mut().insert(handle, doc));
    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn scrolled_set_virtual_size(scroll_handle: NativeHandle, w: i32, h: i32) {
    let w = w.max(0) as f64;
    let h = h.max(0) as f64;
    SCROLLED_DOCUMENTS.with(|map| {
        if let Some(doc) = map.borrow().get(&scroll_handle) {
            doc.setFrameSize(NSSize::new(w.max(1.0), h.max(1.0)));
        }
    });
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&scroll_handle) {
            if let Some(scroll) = view.downcast_ref::<NSScrollView>() {
                let clip = scroll.contentView();
                scroll.reflectScrolledClipView(&clip);
            }
        }
    });
}

pub fn scrolled_set_view_position(scroll_handle: NativeHandle, x: i32, y: i32) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&scroll_handle) {
            if let Some(scroll) = view.downcast_ref::<NSScrollView>() {
                let clip = scroll.contentView();
                let doc_h = clip
                    .documentView()
                    .map(|dv| dv.frame().size.height)
                    .unwrap_or(0.0);
                let clip_h = clip.bounds().size.height;
                let flipped_y = (doc_h - clip_h - y.max(0) as f64).max(0.0);
                clip.scrollToPoint(NSPoint::new(x.max(0) as f64, flipped_y));
                scroll.reflectScrolledClipView(&clip);
            }
        }
    });
}

pub fn scrolled_set_frame(parent: NativeHandle, scroll_handle: NativeHandle, rect: Rect) {
    let content_h = parent_height_for(parent);
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&scroll_handle) {
            view.setFrame(ru_rect_to_ns(content_h, &rect));
        }
    });
    schedule_scrolled_resize(scroll_handle);
}

fn schedule_scrolled_resize(scroll_handle: NativeHandle) {
    let has_handler = SCROLLED_RESIZE.with(|map| map.borrow().contains_key(&scroll_handle));
    if !has_handler {
        return;
    }
    SCROLLED_RESIZE_PENDING.with(|pending| {
        pending.borrow_mut().insert(scroll_handle);
    });
    if IN_DEFERRED_DRAIN.get() {
        return;
    }
    if SCROLLED_RESIZE_SCHEDULED.get() {
        return;
    }
    SCROLLED_RESIZE_SCHEDULED.set(true);
    run_on_next_event(|| {
        SCROLLED_RESIZE_SCHEDULED.set(false);
        flush_scrolled_resize_pending();
    });
}

pub fn flush_scrolled_resize_pending() {
    loop {
        let handles: Vec<_> = SCROLLED_RESIZE_PENDING.with(|pending| {
            if pending.borrow().is_empty() {
                return Vec::new();
            }
            pending.borrow_mut().drain().collect()
        });
        if handles.is_empty() {
            break;
        }
        for handle in handles {
            SCROLLED_RESIZE.with(|map| {
                let taken = map.borrow_mut().remove(&handle);
                if let Some(mut handler) = taken {
                    handler();
                    map.borrow_mut().insert(handle, handler);
                }
            });
        }
    }
}

pub fn scrolled_set_visible(scroll_handle: NativeHandle, visible: bool) {
    set_view_visible(scroll_handle, visible);
}

pub fn scrolled_set_enabled(scroll_handle: NativeHandle, enabled: bool) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&scroll_handle) {
            if let Some(scroll) = view.downcast_ref::<NSScrollView>() {
                // Enable/disable the horizontal and vertical scrollers.
                // NSScroller inherits from NSControl which provides setEnabled:.
                unsafe {
                    let h_scroller: Option<Retained<NSControl>> =
                        msg_send![scroll, horizontalScroller];
                    if let Some(ref s) = h_scroller {
                        s.setEnabled(enabled);
                    }
                    let v_scroller: Option<Retained<NSControl>> =
                        msg_send![scroll, verticalScroller];
                    if let Some(ref s) = v_scroller {
                        s.setEnabled(enabled);
                    }
                }
            }
        }
    });
}

pub fn scrolled_on_resize(scroll_handle: NativeHandle, handler: Box<dyn FnMut()>) {
    SCROLLED_RESIZE.with(|map| map.borrow_mut().insert(scroll_handle, handler));
}

// ── Tab view delegate ───────────────────────────────────────────────

#[derive(Default)]
struct TabViewDelegateIvars {
    tab_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TabViewDelegateIvars]
    struct TabViewDelegate;

    unsafe impl NSObjectProtocol for TabViewDelegate {}

    unsafe impl NSTabViewDelegate for TabViewDelegate {
        #[unsafe(method(tabView:didSelectTabViewItem:))]
        fn tab_view_did_select(&self, tab_view: &NSTabView, tab_view_item: Option<&NSTabViewItem>) {
            let Some(item) = tab_view_item else {
                return;
            };
            let index = tab_view.indexOfTabViewItem(item);
            if index < 0 {
                return;
            }
            let tab_handle = self.ivars().tab_handle.get();
            if let Some(panel_handle) = TAB_PAGE_HANDLES.with(|map| {
                map.borrow()
                    .get(&tab_handle)
                    .and_then(|pages| pages.get(index as usize).copied())
            }) {
                tab_layout_page(tab_handle, panel_handle);
            }
            TAB_SELECTION_HANDLERS.with(|map| {
                if let Some(handler) = map.borrow_mut().get_mut(&tab_handle) {
                    handler(index as usize);
                }
            });
        }
    }
);

impl TabViewDelegate {
    fn new(mtm: MainThreadMarker, tab_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TabViewDelegateIvars {
            tab_handle: Cell::new(tab_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ── Frame host ──────────────────────────────────────────────────────

/// Native top-level window on macOS.
pub struct FrameHost {
    pub handle: NativeHandle,
    pub title: String,
    pub rect: Rect,
    pub visible: bool,
    window: std::cell::RefCell<Option<Retained<NSWindow>>>,
    content_view: std::cell::RefCell<Option<Retained<NSView>>>,
    pub running: std::rc::Rc<Cell<bool>>,
    delegate: std::cell::RefCell<Option<Retained<WindowDelegate>>>,
}

impl FrameHost {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        Self {
            handle: alloc_widget_handle(),
            title: title.to_string(),
            rect: Rect::new(0, 0, width, height),
            visible: false,
            window: std::cell::RefCell::new(None),
            content_view: std::cell::RefCell::new(None),
            running: std::rc::Rc::new(Cell::new(true)),
            delegate: std::cell::RefCell::new(None),
        }
    }

    /// Create the native window (hidden) so child controls can be added.
    pub fn prepare_window(&mut self) {
        if self.window.borrow().is_some() {
            return;
        }

        let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
        init_application();

        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable;

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(
                    NSPoint::new(100.0, 100.0),
                    NSSize::new(self.rect.width as f64, self.rect.height as f64),
                ),
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(&self.title));

        let _content = window
            .contentView()
            .expect("NSWindow must have a content view");

        let delegate = WindowDelegate::new(mtm, std::rc::Rc::clone(&self.running), self.handle);
        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        *self.delegate.borrow_mut() = Some(delegate);

        window.center();
        window.orderOut(None);

        let content = appkit_accelerator::replace_window_content_view(mtm, &window, self.handle);

        FRAME_WINDOWS.with(|map| {
            map.borrow_mut().insert(self.handle, window.clone());
        });

        *self.window.borrow_mut() = Some(window);
        *self.content_view.borrow_mut() = Some(content);
    }

    /// Show the window on screen.
    pub fn present(&mut self) {
        if self.window.borrow().is_none() {
            self.prepare_window();
        }
        if let Some(window) = self.window.borrow().as_ref() {
            let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
            let app = NSApplication::sharedApplication(mtm);
            window.makeKeyAndOrderFront(None);
            // Keep the window itself as first responder so editable text fields
            // (e.g. the address bar) don't start with all text selected.
            window.makeFirstResponder(None);
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
            self.visible = true;
        }
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        if let Some(window) = self.window.borrow().as_ref() {
            window.setTitle(&NSString::from_str(title));
        }
    }

    pub fn request_close(&mut self) {
        self.running.set(false);
        if let Some(window) = self.window.borrow().as_ref() {
            window.close();
        }
        self.visible = false;
    }

    pub fn content_height(&self) -> f64 {
        self.content_view
            .borrow()
            .as_ref()
            .map(|v| v.frame().size.height)
            .unwrap_or(self.rect.height as f64)
    }
}

impl std::fmt::Debug for FrameHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameHost")
            .field("handle", &self.handle)
            .field("title", &self.title)
            .field("rect", &self.rect)
            .field("visible", &self.visible)
            .finish()
    }
}

// ── Application bootstrap ───────────────────────────────────────────

static APP_INIT: OnceLock<()> = OnceLock::new();

pub fn init_application() {
    APP_INIT.get_or_init(|| {
        let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
        let app = NSApplication::sharedApplication(mtm);

        let menu = NSMenu::new(mtm);
        let app_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str("ru_wx"),
                None,
                &NSString::from_str(""),
            )
        };
        let quit_menu = NSMenu::new(mtm);
        let quit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str("Quit"),
                Some(sel!(terminate:)),
                &NSString::from_str("q"),
            )
        };
        quit_menu.addItem(&quit_item);
        app_item.setSubmenu(Some(&quit_menu));
        menu.addItem(&app_item);
        app.setMainMenu(Some(&menu));

        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    });
}

// ── Messaggi custom (equivalente macOS di `PostMessageW`) ───────────
//
// Su Windows i worker thread notificano il thread UI con
// `PostMessageW` e i messaggi vengono recapitati dalla message pump
// agli handler registrati con `Frame::register_raw_message_handler`.
// Su macOS non esiste un equivalente diretto, quindi il messaggio
// viene impacchettato e accodato al main queue di libdispatch: il
// recapito avviene cosi' sul thread UI, dove vivono gli handler
// (che sono `!Send`).

/// Raw window-procedure handler: receives `(wparam, lparam)`.
type RawHandler = Box<dyn FnMut(isize, isize)>;

thread_local! {
    /// Handler registrati per handle della frame e codice messaggio.
    static RAW_HANDLERS: RefCell<HashMap<NativeHandle, HashMap<u32, Vec<RawHandler>>>> =
        RefCell::new(HashMap::new());
}

/// Registra un handler per un messaggio custom di una frame.
pub fn register_raw_handler(frame: NativeHandle, msg: u32, handler: Box<dyn FnMut(isize, isize)>) {
    RAW_HANDLERS.with(|map| {
        map.borrow_mut()
            .entry(frame)
            .or_default()
            .entry(msg)
            .or_default()
            .push(handler);
    });
    HAS_RAW_HANDLERS.store(true, Ordering::Relaxed);
}

/// Recapita `(w, l)` agli handler registrati per `(frame, msg)`.
/// Gira sul thread UI: gli handler sono `!Send`.
fn deliver_raw_message(frame: NativeHandle, msg: u32, w: isize, l: isize) {
    // Pattern preleva → chiama → reinserisci: l'handler puo' a sua
    // volta registrare handler o postare messaggi, quindi il `RefCell`
    // non va tenuto occupato durante la chiamata.
    let handler = RAW_HANDLERS.with(|map| {
        map.borrow_mut()
            .get_mut(&frame)
            .and_then(|by_msg| by_msg.get_mut(&msg))
            .and_then(|handlers| {
                if handlers.is_empty() {
                    None
                } else {
                    Some(handlers.remove(0))
                }
            })
    });
    if let Some(mut handler) = handler {
        // Un panic nell'handler NON deve far perdere l'handler:
        // altrimenti i messaggi successivi verrebbero drenati e
        // scartati in silenzio, lasciando la UI sorda (es. nessun
        // aggiornamento al cambio cartella dopo un errore di
        // navigazione). Catturiamo il panic, logghiamolo e
        // reinseriamo comunque l'handler.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handler(w, l);
        }));
        RAW_HANDLERS.with(|map| {
            if let Some(handlers) = map
                .borrow_mut()
                .get_mut(&frame)
                .and_then(|by_msg| by_msg.get_mut(&msg))
            {
                handlers.push(handler);
            }
        });
        if let Err(payload) = result {
            crate::core::log::log_message(
                crate::core::log::LogLevel::Error,
                "ui/dispatch",
                format!(
                    "panic in raw message handler (msg=0x{:04X}): {}",
                    msg,
                    crate::core::log::payload_to_string(payload.as_ref())
                ),
            );
        }
    }
}

/// Messaggio custom in transito verso il thread UI.
struct RawMessage {
    frame: NativeHandle,
    msg: u32,
    w: isize,
    l: isize,
}

/// Coda condivisa dei messaggi postati dai worker thread.
static RAW_QUEUE: Mutex<Vec<RawMessage>> = Mutex::new(Vec::new());

/// Vero se almeno una frame ha handler per messaggi custom. In quel
/// caso il run loop non puo' bloccarsi a tempo indefinito: nessun
/// evento AppKit segnalerebbe l'arrivo di un messaggio da un worker.
static HAS_RAW_HANDLERS: AtomicBool = AtomicBool::new(false);

/// Intervallo di polling del run loop quando ci sono handler custom.
const RAW_POLL_INTERVAL: f64 = 0.03;

/// Accoda un messaggio custom al thread UI; chiamabile da qualunque
/// thread (i worker lo usano al posto di `PostMessageW`).
pub fn post_raw_message(frame: NativeHandle, msg: u32, w: isize, l: isize) {
    // Anche se il mutex risulta avvelenato (un thread e' morto mentre
    // teneva il lock) la coda resta utilizzabile: dentro il lock si
    // fanno solo push/take, nessuno stato puo' restare corrotto.
    // Perdere silenziosamente il messaggio lascerebbe la UI sorda.
    let mut queue = RAW_QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    queue.push(RawMessage { frame, msg, w, l });
}

/// Recapita al thread UI i messaggi accodati dai worker.
fn drain_raw_messages() {
    if !HAS_RAW_HANDLERS.load(Ordering::Relaxed) {
        return;
    }
    let pending: Vec<RawMessage> = {
        let mut queue = RAW_QUEUE.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *queue)
    };
    for message in pending {
        deliver_raw_message(message.frame, message.msg, message.w, message.l);
    }
}

pub fn run_event_loop(running: std::rc::Rc<Cell<bool>>) {
    while running.get() {
        drain_deferred_callbacks();
        drain_raw_messages();
        wait_for_next_event();
        drain_raw_messages();
    }
}

/// Queue `f` to run on the main thread before the next AppKit event is processed.
pub fn run_on_next_event(f: impl FnOnce() + 'static) {
    let slot = RefCell::new(Some(f));
    DEFERRED_MAIN.with(|q| {
        q.borrow_mut().push(Box::new(move || {
            if let Some(callback) = slot.borrow_mut().take() {
                callback();
            }
        }));
    });
}

fn drain_deferred_callbacks() {
    IN_DEFERRED_DRAIN.set(true);
    DEFERRED_MAIN.with(|q| {
        let pending: Vec<_> = std::mem::take(&mut *q.borrow_mut());
        for mut f in pending {
            f();
        }
    });
    flush_scrolled_resize_pending();
    IN_DEFERRED_DRAIN.set(false);
}

// ── Control factories ───────────────────────────────────────────────

pub fn mac_view_client_size(handle: NativeHandle) -> (u32, u32) {
    child_view_for_handle(handle)
        .map(|v| {
            let f = v.frame();
            (f.size.width.max(1.0) as u32, f.size.height.max(1.0) as u32)
        })
        .unwrap_or((1, 1))
}

pub fn mac_view_redraw(handle: NativeHandle) {
    if let Some(view) = child_view_for_handle(handle) {
        view.setNeedsDisplay(true);
    }
}

fn parent_view_for(parent: NativeHandle) -> Option<Retained<NSView>> {
    SCROLLED_DOCUMENTS
        .with(|map| map.borrow().get(&parent).cloned())
        .or_else(|| FRAME_CONTENT.with(|map| map.borrow().get(&parent).cloned()))
        .or_else(|| CONTAINERS.with(|map| map.borrow().get(&parent).cloned()))
}

fn parent_height_for(parent: NativeHandle) -> f64 {
    parent_view_for(parent)
        .map(|v| v.frame().size.height)
        .unwrap_or(600.0)
}

fn ru_rect_to_ns(content_height: f64, r: &Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(r.x as f64, content_height - r.y as f64 - r.height as f64),
        NSSize::new(r.width as f64, r.height as f64),
    )
}

pub(super) fn rect_to_ns_in_parent(parent: NativeHandle, r: &Rect) -> NSRect {
    ru_rect_to_ns(parent_height_for(parent), r)
}

fn store_ns_view(handle: NativeHandle, view: Retained<NSView>) {
    VIEWS.with(|map| {
        map.borrow_mut().insert(handle, view);
    });
}

pub(super) fn remove_stored_view(handle: NativeHandle) {
    VIEWS.with(|map| {
        map.borrow_mut().remove(&handle);
    });
}

pub(super) fn child_view_for_handle(handle: NativeHandle) -> Option<Retained<NSView>> {
    VIEWS
        .with(|m| m.borrow().get(&handle).cloned())
        .or_else(|| CONTAINERS.with(|m| m.borrow().get(&handle).cloned()))
}

fn store_container(handle: NativeHandle, view: Retained<NSView>) {
    CONTAINERS.with(|map| {
        map.borrow_mut().insert(handle, view);
    });
}

pub(super) fn remove_container(handle: NativeHandle) {
    CONTAINERS.with(|map| {
        map.borrow_mut().remove(&handle);
    });
}

/// Create a native container `NSView` for [`crate::window::panel::Panel`].
pub fn create_panel(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(parent_view) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = parent_view.frame().size.height;
    let view = NSView::initWithFrame(NSView::alloc(mtm), ru_rect_to_ns(content_h, &initial_rect));
    view.setClipsToBounds(true);
    parent_view.addSubview(&view);
    store_container(handle, view);
    record_parent(handle, parent);
    handle
}

pub fn set_container_frame(parent: NativeHandle, container: NativeHandle, rect: Rect) {
    let content_h = parent_height_for(parent);
    CONTAINERS.with(|map| {
        if let Some(view) = map.borrow().get(&container) {
            view.setFrame(ru_rect_to_ns(content_h, &rect));
        }
    });
}

pub fn set_container_visible(container: NativeHandle, visible: bool) {
    CONTAINERS.with(|map| {
        if let Some(view) = map.borrow().get(&container) {
            view.setHidden(!visible);
        }
    });
}

pub fn create_button(
    parent: NativeHandle,
    title: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let frame_handle = resolve_frame_handle(parent);
    let target = ButtonTarget::new(mtm, frame_handle, command_id);
    MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(&target),
            Some(sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    button.setFrame(ru_rect_to_ns(content_h, &initial_rect));
    button.setBezelStyle(NSBezelStyle::Push);
    content.addSubview(&button);
    store_ns_view(handle, button.into_super().into_super());
    record_parent(handle, parent);

    handle
}

/// Re-attach the AppKit target/action after styling an [`NSButton`] (some
/// `setButtonType` / bezel changes clear the target on macOS).
pub fn button_rewire_click_target(
    view_handle: NativeHandle,
    parent: NativeHandle,
    command_id: u16,
) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let frame_handle = resolve_frame_handle(parent);
    let target = ButtonTarget::new(mtm, frame_handle, command_id);
    MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(btn) = view.downcast_ref::<NSButton>() {
                unsafe {
                    btn.setTarget(Some(&target));
                    btn.setAction(Some(sel!(ruWxButtonClicked:)));
                }
                btn.setEnabled(true);
            }
        }
    });
}

pub fn create_checkbox(
    parent: NativeHandle,
    title: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let frame_handle = resolve_frame_handle(parent);
    let target = ButtonTarget::new(mtm, frame_handle, command_id);
    MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
    let checkbox = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str(title),
            Some(&target),
            Some(sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    checkbox.setFrame(ru_rect_to_ns(content_h, &initial_rect));
    content.addSubview(&checkbox);
    mark_declared_role(&checkbox, "check");
    store_ns_view(handle, checkbox.into_super().into_super());
    record_parent(handle, parent);

    handle
}

pub fn checkbox_is_checked(view_handle: NativeHandle) -> bool {
    VIEWS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .and_then(|view| view.downcast_ref::<NSButton>())
            .map(|cb| cb.state() == NSControlStateValueOn)
            .unwrap_or(false)
    })
}

pub fn checkbox_set_checked(view_handle: NativeHandle, checked: bool) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(cb) = view.downcast_ref::<NSButton>() {
                cb.setState(if checked {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
        }
    });
}

pub fn create_radio_button(
    parent: NativeHandle,
    title: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let frame_handle = resolve_frame_handle(parent);
    let target = ButtonTarget::new(mtm, frame_handle, command_id);
    MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
    let radio = unsafe {
        NSButton::radioButtonWithTitle_target_action(
            &NSString::from_str(title),
            Some(&target),
            Some(sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    radio.setFrame(ru_rect_to_ns(content_h, &initial_rect));
    content.addSubview(&radio);
    mark_declared_role(&radio, "radio");
    store_ns_view(handle, radio.into_super().into_super());
    record_parent(handle, parent);

    handle
}

pub fn radio_is_selected(view_handle: NativeHandle) -> bool {
    checkbox_is_checked(view_handle)
}

pub fn radio_set_selected(view_handle: NativeHandle, selected: bool) {
    checkbox_set_checked(view_handle, selected);
}

fn popup_is_combo(view_handle: NativeHandle) -> bool {
    POPUP_IS_COMBO.with(|map| map.borrow().get(&view_handle).copied().unwrap_or(false))
}

fn mark_popup_kind(view_handle: NativeHandle, editable: bool) {
    POPUP_IS_COMBO.with(|map| {
        map.borrow_mut().insert(view_handle, editable);
    });
}

/// Create a drop-down list (`NSPopUpButton`) or editable combo (`NSComboBox`).
pub fn create_popup(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    editable: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let frame_handle = resolve_frame_handle(parent);
    let target = ButtonTarget::new(mtm, frame_handle, command_id);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);

    if editable {
        let combo = NSComboBox::initWithFrame(NSComboBox::alloc(mtm), ns_rect);
        combo.setCompletes(false);
        unsafe {
            combo.setTarget(Some(&target));
            combo.setAction(Some(sel!(ruWxButtonClicked:)));
        }
        content.addSubview(&combo);
        store_ns_view(handle, combo.into_super().into_super().into_super());
    } else {
        let popup =
            NSPopUpButton::initWithFrame_pullsDown(NSPopUpButton::alloc(mtm), ns_rect, false);
        unsafe {
            popup.setTarget(Some(&target));
            popup.setAction(Some(sel!(ruWxButtonClicked:)));
        }
        content.addSubview(&popup);
        store_ns_view(handle, popup.into_super().into_super().into_super());
    }

    mark_popup_kind(handle, editable);
    record_parent(handle, parent);
    handle
}

pub fn popup_append_item(view_handle: NativeHandle, item: &str) {
    let title = NSString::from_str(item);
    VIEWS.with(|map| {
        let view = map.borrow().get(&view_handle).cloned();
        if let Some(view) = view {
            if popup_is_combo(view_handle) {
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    unsafe { combo.addItemWithObjectValue(&title) };
                }
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.addItemWithTitle(&title);
            }
        }
    });
}

pub fn popup_insert_item(view_handle: NativeHandle, index: usize, item: &str) {
    let title = NSString::from_str(item);
    let idx = index as isize;
    VIEWS.with(|map| {
        let view = map.borrow().get(&view_handle).cloned();
        if let Some(view) = view {
            if popup_is_combo(view_handle) {
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    unsafe { combo.insertItemWithObjectValue_atIndex(&title, idx) };
                }
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.insertItemWithTitle_atIndex(&title, idx);
            }
        }
    });
}

pub fn popup_remove_item(view_handle: NativeHandle, index: usize) {
    let idx = index as isize;
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if popup_is_combo(view_handle) {
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    combo.removeItemAtIndex(idx);
                }
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.removeItemAtIndex(idx);
            }
        }
    });
}

pub fn popup_clear(view_handle: NativeHandle) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if popup_is_combo(view_handle) {
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    combo.removeAllItems();
                }
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.removeAllItems();
            }
        }
    });
}

pub fn popup_get_count(view_handle: NativeHandle) -> usize {
    VIEWS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .map(|view| {
                if popup_is_combo(view_handle) {
                    view.downcast_ref::<NSComboBox>()
                        .map(|c| c.numberOfItems().max(0) as usize)
                        .unwrap_or(0)
                } else {
                    view.downcast_ref::<NSPopUpButton>()
                        .map(|p| p.numberOfItems().max(0) as usize)
                        .unwrap_or(0)
                }
            })
            .unwrap_or(0)
    })
}

pub fn popup_get_selection(view_handle: NativeHandle) -> Option<usize> {
    VIEWS.with(|map| {
        map.borrow().get(&view_handle).and_then(|view| {
            let idx = if popup_is_combo(view_handle) {
                view.downcast_ref::<NSComboBox>()
                    .map(|c| c.indexOfSelectedItem())
            } else {
                view.downcast_ref::<NSPopUpButton>()
                    .map(|p| p.indexOfSelectedItem())
            }?;
            if idx < 0 {
                None
            } else {
                Some(idx as usize)
            }
        })
    })
}

pub fn popup_set_selection(view_handle: NativeHandle, index: usize) {
    let idx = index as isize;
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if popup_is_combo(view_handle) {
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    combo.selectItemAtIndex(idx);
                }
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.selectItemAtIndex(idx);
            }
        }
    });
}

pub fn popup_get_string(view_handle: NativeHandle, index: usize) -> Option<String> {
    let idx = index as isize;
    VIEWS.with(|map| {
        map.borrow().get(&view_handle).and_then(|view| {
            if popup_is_combo(view_handle) {
                view.downcast_ref::<NSComboBox>().and_then(|combo| {
                    if idx < 0 || idx >= combo.numberOfItems() {
                        return None;
                    }
                    let obj = combo.itemObjectValueAtIndex(idx);
                    obj.downcast::<NSString>().ok().map(|s| s.to_string())
                })
            } else {
                view.downcast_ref::<NSPopUpButton>().and_then(|popup| {
                    if idx < 0 || idx >= popup.numberOfItems() {
                        return None;
                    }
                    Some(popup.itemTitleAtIndex(idx).to_string())
                })
            }
        })
    })
}

pub fn popup_get_value(view_handle: NativeHandle) -> String {
    VIEWS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .map(|view| {
                if popup_is_combo(view_handle) {
                    view.downcast_ref::<NSComboBox>()
                        .map(|c| c.stringValue().to_string())
                        .unwrap_or_default()
                } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                    let idx = popup.indexOfSelectedItem();
                    if idx >= 0 {
                        popup.itemTitleAtIndex(idx).to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            })
            .unwrap_or_default()
    })
}

pub fn popup_set_value(view_handle: NativeHandle, text: &str) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if popup_is_combo(view_handle) {
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    combo.setStringValue(&NSString::from_str(text));
                }
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.selectItemWithTitle(&NSString::from_str(text));
            }
        }
    });
}

/// Create a list box (`NSScrollView` + native [`NSTableView`]).
pub fn create_list_box(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    multi_select: bool,
) -> NativeHandle {
    appkit_listbox_table::create_list_box_native(parent, command_id, initial_rect, multi_select)
}

#[allow(dead_code)]
fn create_list_box_legacy(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    multi_select: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    listbox_init_storage(handle, multi_select);

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let frame_handle = resolve_frame_handle(parent);

    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);

    let list_view = ListBoxView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
        handle,
        frame_handle,
        command_id,
    );
    scroll.setDocumentView(Some(&list_view));
    content.addSubview(&scroll);

    LISTBOX_VIEWS.with(|map| map.borrow_mut().insert(handle, list_view));
    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn listbox_append_item(list_handle: NativeHandle, item: &str) {
    LISTBOX_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            items.borrow_mut().push(item.to_string());
        }
    });
    listbox_reload(list_handle);
}

pub fn listbox_insert_item(list_handle: NativeHandle, index: usize, item: &str) {
    LISTBOX_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            let mut vec = items.borrow_mut();
            let idx = index.min(vec.len());
            vec.insert(idx, item.to_string());
        }
    });
    listbox_reload(list_handle);
}

pub fn listbox_remove_item(list_handle: NativeHandle, index: usize) {
    LISTBOX_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            let mut vec = items.borrow_mut();
            if index < vec.len() {
                vec.remove(index);
            }
        }
    });
    listbox_reload(list_handle);
}

pub fn listbox_clear(list_handle: NativeHandle) {
    LISTBOX_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            items.borrow_mut().clear();
        }
    });
    listbox_reload(list_handle);
}

pub fn listbox_get_count(list_handle: NativeHandle) -> usize {
    LISTBOX_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|items| items.borrow().len())
            .unwrap_or(0)
    })
}

pub fn listbox_get_string(list_handle: NativeHandle, index: usize) -> Option<String> {
    LISTBOX_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|items| items.borrow().get(index).cloned())
    })
}

pub fn listbox_get_selection(list_handle: NativeHandle) -> Option<usize> {
    LISTBOX_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|sel| sel.borrow().first().copied())
    })
}

pub fn listbox_get_selections(list_handle: NativeHandle) -> Vec<usize> {
    LISTBOX_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|sel| sel.borrow().clone())
            .unwrap_or_default()
    })
}

pub fn listbox_set_selection(list_handle: NativeHandle, index: usize) {
    listbox_write_selection(list_handle, vec![index]);
    if appkit_listbox_table::listbox_is_native_table(list_handle) {
        appkit_listbox_table::listbox_apply_selection(list_handle);
    } else {
        listbox_redraw(list_handle);
    }
}

// ── CheckListBox (native NSTableView + checkbox cells) ───────────────

pub fn create_check_list_box(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    appkit_checklist_table::create_check_list_box_native(parent, command_id, initial_rect)
}

pub(super) fn checklist_init_storage(list_handle: NativeHandle) {
    CHECKLIST_ITEMS.with(|map| {
        map.borrow_mut()
            .insert(list_handle, RefCell::new(Vec::new()));
    });
    CHECKLIST_CHECKED.with(|map| {
        map.borrow_mut()
            .insert(list_handle, RefCell::new(Vec::new()));
    });
    CHECKLIST_SELECTED.with(|map| {
        map.borrow_mut()
            .insert(list_handle, RefCell::new(Vec::new()));
    });
    CHECKLIST_LAST_ROW.with(|map| {
        map.borrow_mut().insert(list_handle, Cell::new(None));
    });
}

pub(super) fn checklist_item_count(list_handle: NativeHandle) -> usize {
    CHECKLIST_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|items| items.borrow().len())
            .unwrap_or(0)
    })
}

pub(super) fn checklist_item_text(list_handle: NativeHandle, index: usize) -> String {
    CHECKLIST_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|items| items.borrow().get(index).cloned())
            .unwrap_or_default()
    })
}

pub fn checklist_is_checked(list_handle: NativeHandle, index: usize) -> bool {
    CHECKLIST_CHECKED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|checked| checked.borrow().get(index).copied())
            .unwrap_or(false)
    })
}

pub fn checklist_set_checked(list_handle: NativeHandle, index: usize, checked: bool) {
    CHECKLIST_CHECKED.with(|map| {
        if let Some(flags) = map.borrow().get(&list_handle) {
            if let Some(flag) = flags.borrow_mut().get_mut(index) {
                *flag = checked;
            }
        }
    });
    checklist_reload(list_handle);
}

pub(super) fn checklist_toggle_checked(list_handle: NativeHandle, index: usize) -> bool {
    let new_state = !checklist_is_checked(list_handle, index);
    checklist_set_checked(list_handle, index, new_state);
    new_state
}

pub(super) fn checklist_set_last_row(list_handle: NativeHandle, row: usize) {
    CHECKLIST_LAST_ROW.with(|map| {
        if let Some(cell) = map.borrow().get(&list_handle) {
            cell.set(Some(row));
        }
    });
}

pub fn checklist_last_row(list_handle: NativeHandle) -> Option<usize> {
    CHECKLIST_LAST_ROW.with(|map| map.borrow().get(&list_handle).and_then(|cell| cell.get()))
}

pub(super) fn checklist_read_selection(list_handle: NativeHandle) -> Vec<usize> {
    CHECKLIST_SELECTED.with(|map| {
        map.borrow()
            .get(&list_handle)
            .map(|sel| sel.borrow().clone())
            .unwrap_or_default()
    })
}

pub(super) fn checklist_write_selection(list_handle: NativeHandle, rows: Vec<usize>) {
    CHECKLIST_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&list_handle) {
            *sel.borrow_mut() = rows;
        }
    });
}

fn checklist_reload(list_handle: NativeHandle) {
    appkit_checklist_table::checklist_reload(list_handle);
}

pub fn checklist_append_item(list_handle: NativeHandle, item: &str) {
    CHECKLIST_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            items.borrow_mut().push(item.to_string());
        }
    });
    CHECKLIST_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&list_handle) {
            checked.borrow_mut().push(false);
        }
    });
    checklist_reload(list_handle);
}

pub fn checklist_insert_item(list_handle: NativeHandle, index: usize, item: &str) {
    CHECKLIST_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            let mut vec = items.borrow_mut();
            let idx = index.min(vec.len());
            vec.insert(idx, item.to_string());
        }
    });
    CHECKLIST_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&list_handle) {
            let mut vec = checked.borrow_mut();
            let idx = index.min(vec.len());
            vec.insert(idx, false);
        }
    });
    checklist_reload(list_handle);
}

pub fn checklist_remove_item(list_handle: NativeHandle, index: usize) {
    CHECKLIST_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            let mut vec = items.borrow_mut();
            if index < vec.len() {
                vec.remove(index);
            }
        }
    });
    CHECKLIST_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&list_handle) {
            let mut vec = checked.borrow_mut();
            if index < vec.len() {
                vec.remove(index);
            }
        }
    });
    checklist_reload(list_handle);
}

pub fn checklist_clear(list_handle: NativeHandle) {
    CHECKLIST_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&list_handle) {
            items.borrow_mut().clear();
        }
    });
    CHECKLIST_CHECKED.with(|map| {
        if let Some(checked) = map.borrow().get(&list_handle) {
            checked.borrow_mut().clear();
        }
    });
    checklist_reload(list_handle);
}

pub fn checklist_get_count(list_handle: NativeHandle) -> usize {
    checklist_item_count(list_handle)
}

pub fn checklist_get_string(list_handle: NativeHandle, index: usize) -> Option<String> {
    CHECKLIST_ITEMS.with(|map| {
        map.borrow()
            .get(&list_handle)
            .and_then(|items| items.borrow().get(index).cloned())
    })
}

pub fn checklist_get_selection(list_handle: NativeHandle) -> Option<usize> {
    checklist_read_selection(list_handle).first().copied()
}

pub fn checklist_get_selections(list_handle: NativeHandle) -> Vec<usize> {
    checklist_read_selection(list_handle)
}

pub fn checklist_set_selection(list_handle: NativeHandle, index: usize) {
    checklist_write_selection(list_handle, vec![index]);
    appkit_checklist_table::checklist_apply_selection(list_handle);
}

pub fn create_static_text(parent: NativeHandle, text: &str, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFrame(ru_rect_to_ns(content_h, &initial_rect));
    content.addSubview(&label);
    store_ns_view(handle, label.into_super().into_super());
    record_parent(handle, parent);

    handle
}

pub fn create_text_field(parent: NativeHandle, text: &str, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let field = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    field.setStringValue(&NSString::from_str(text));
    field.setEditable(true);
    field.setBezeled(true);
    field.setBezelStyle(NSTextFieldBezelStyle::SquareBezel);
    field.setDrawsBackground(true);
    // The native bezel is nearly invisible on modern macOS over a light
    // background, so draw an explicit layer border around the field.
    field.setWantsLayer(true);
    if let Some(layer) = field.layer() {
        let border = ns_color_from_rgba((178, 178, 182, 255));
        layer.setBorderColor(Some(&border.CGColor()));
        layer.setBorderWidth(1.0);
        layer.setCornerRadius(4.0);
    }
    content.addSubview(&field);
    store_ns_view(handle, field.into_super().into_super());
    record_parent(handle, parent);

    handle
}

pub fn create_secure_text_field(
    parent: NativeHandle,
    text: &str,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let field = NSSecureTextField::initWithFrame(
        NSSecureTextField::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    field.setStringValue(&NSString::from_str(text));
    field.setEditable(true);
    field.setBezeled(true);
    field.setBezelStyle(NSTextFieldBezelStyle::SquareBezel);
    field.setDrawsBackground(true);
    content.addSubview(&field);
    store_ns_view(handle, field.into_super().into_super().into_super());
    record_parent(handle, parent);

    handle
}

/// Like [`create_secure_text_field`] but dispatches `command_id` on every edit.
pub fn create_secure_text_field_notify(
    parent: NativeHandle,
    text: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    let handle = create_secure_text_field(parent, text, initial_rect);
    text_field_enable_change_notify(handle, command_id);
    handle
}

/// Like [`create_text_field`] but dispatches `command_id` on every edit.
pub fn create_text_field_notify(
    parent: NativeHandle,
    text: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    let handle = create_text_field(parent, text, initial_rect);
    text_field_enable_change_notify(handle, command_id);
    handle
}

/// Multi-line plain-text editor (`NSScrollView` + [`NSTextView`]) with change notify.
pub fn create_multiline_text_ctrl_notify(
    parent: NativeHandle,
    text: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);

    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(false);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);

    let doc_bounds = scroll.contentView().bounds();
    let text_view = NSTextView::initWithFrame(NSTextView::alloc(mtm), doc_bounds);
    text_view.setEditable(true);
    text_view.setRichText(false);
    text_view.setHorizontallyResizable(false);
    text_view.setVerticallyResizable(true);
    text_view.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    if let Some(container) = unsafe { text_view.textContainer() } {
        container.setWidthTracksTextView(true);
    }
    text_view.setString(&NSString::from_str(text));
    scroll.setDocumentView(Some(&text_view));
    content.addSubview(&scroll);

    MULTILINE_TEXT_CTRL.with(|set| set.borrow_mut().insert(handle));
    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    text_view_enable_change_notify(handle, command_id);
    handle
}

/// Apply read-only mode to a single-line [`NSTextField`] or multiline [`NSTextView`].
pub fn text_ctrl_set_readonly(view_handle: NativeHandle, readonly: bool) {
    if MULTILINE_TEXT_CTRL.with(|set| set.borrow().contains(&view_handle)) {
        if let Some(text) = text_view_in_scroll_handle(view_handle) {
            text.setEditable(!readonly);
            text.setSelectable(true);
        }
        return;
    }
    with_text_field(view_handle, |field| {
        field.setEditable(!readonly);
    });
}

/// Move `view_handle` to the top of its superview's z-order (mouse hits).
pub fn mac_view_raise_to_front(view_handle: NativeHandle) {
    if let Some(view) = child_view_for_handle(view_handle) {
        unsafe {
            if let Some(superview) = view.superview() {
                superview.addSubview(&view);
            }
        }
    } else if let Some(view) = CONTAINERS.with(|map| map.borrow().get(&view_handle).cloned()) {
        unsafe {
            if let Some(superview) = view.superview() {
                superview.addSubview(&view);
            }
        }
    }
}

pub fn set_view_frame(parent: NativeHandle, view_handle: NativeHandle, rect: Rect) {
    let content_h = parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &rect);
    if VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            view.setFrame(ns_rect);
            true
        } else {
            false
        }
    }) {
        appkit_static_text::static_text_refresh_wrap(view_handle);
        return;
    }
    CONTAINERS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            view.setFrame(ns_rect);
        }
    });
}

pub fn frame_client_size(frame_handle: NativeHandle) -> (u32, u32) {
    FRAME_CONTENT.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .map(|v| {
                let s = v.frame().size;
                (s.width.max(0.0) as u32, s.height.max(0.0) as u32)
            })
            .unwrap_or((0, 0))
    })
}

pub fn set_view_visible(view_handle: NativeHandle, visible: bool) {
    let hidden = !visible;
    if VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            view.setHidden(hidden);
            true
        } else {
            false
        }
    }) {
        return;
    }
    CONTAINERS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            view.setHidden(hidden);
        }
    });
}

pub(super) fn native_view_size(view_handle: NativeHandle) -> Option<(f64, f64)> {
    VIEWS.with(|map| {
        map.borrow().get(&view_handle).map(|view| {
            let size = view.frame().size;
            (size.width, size.height)
        })
    })
}

pub fn set_static_text(view_handle: NativeHandle, text: &str) {
    with_text_field(view_handle, |field| {
        field.setStringValue(&NSString::from_str(text));
    });
}

fn with_text_field(view_handle: NativeHandle, f: impl FnOnce(&NSTextField)) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(field) = view.downcast_ref::<NSTextField>() {
                f(field);
            }
        }
    });
}

/// Apply an [`NSFont`] to a static label or text field view.
pub fn apply_font_to_view(view_handle: NativeHandle, font: &Retained<NSFont>) {
    with_text_field(view_handle, |field| field.setFont(Some(font)));
}

/// Set the foreground colour of a label / text field.
pub fn set_view_text_color(view_handle: NativeHandle, colour: Colour) {
    let rgba = (colour.r, colour.g, colour.b, colour.a);
    with_text_field(view_handle, |field| {
        field.setTextColor(Some(&ns_color_from_rgba(rgba)));
    });
}

/// Enable word wrapping (`max_lines`: 0 = unlimited, 1 = single line).
pub fn static_label_set_max_lines(view_handle: NativeHandle, max_lines: i32) {
    appkit_static_text::static_text_set_max_lines(view_handle, max_lines);
}

/// Fill a container [`NSView`] (panel) with a solid colour.
pub fn set_container_background(container: NativeHandle, colour: Colour) {
    appkit_background::set_container_background(container, colour);
}

/// Fill the frame client area with a solid colour.
pub fn frame_set_content_background(frame_handle: NativeHandle, colour: Colour) {
    appkit_background::frame_set_content_background(frame_handle, colour);
}

pub fn get_text_field_value(view_handle: NativeHandle) -> String {
    if let Some(text) = text_view_in_scroll_handle(view_handle) {
        return text.string().to_string();
    }
    VIEWS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .and_then(|view| {
                view.downcast_ref::<NSTextField>()
                    .map(|f| f.stringValue().to_string())
            })
            .unwrap_or_default()
    })
}

pub fn set_text_field_placeholder(view_handle: NativeHandle, placeholder: &str) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(field) = view.downcast_ref::<NSTextField>() {
                field.setPlaceholderString(Some(&NSString::from_str(placeholder)));
            }
        }
    });
}

pub fn present_native_window(frame_handle: NativeHandle) {
    if let Some(window) = ns_window_for_frame_handle(frame_handle) {
        window.makeKeyAndOrderFront(None);
    }
}

pub fn set_text_field_value(view_handle: NativeHandle, text: &str) {
    if let Some(tv) = text_view_in_scroll_handle(view_handle) {
        tv.setString(&NSString::from_str(text));
        return;
    }
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(field) = view.downcast_ref::<NSTextField>() {
                field.setStringValue(&NSString::from_str(text));
            }
        }
    });
}

pub fn set_button_title(view_handle: NativeHandle, title: &str) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(button) = view.downcast_ref::<NSButton>() {
                button.setTitle(&NSString::from_str(title));
            }
        }
    });
}

pub fn set_view_enabled(view_handle: NativeHandle, enabled: bool) {
    SPIN_PARTS.with(|map| {
        if let Some((field, stepper, _)) = map.borrow().get(&view_handle) {
            field.setEnabled(enabled);
            stepper.setEnabled(enabled);
        }
    });
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(control) = view.downcast_ref::<NSControl>() {
                control.setEnabled(enabled);
            }
        }
    });
}

/// Set or clear the AppKit tooltip string on a registered view handle.
pub fn view_set_tooltip(view_handle: NativeHandle, text: Option<&str>) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            match text {
                Some(t) => view.setToolTip(Some(&NSString::from_str(t))),
                None => view.setToolTip(None),
            }
        }
    });
}

const STATUS_BAR_HEIGHT: f64 = 22.0;

/// Create a multi-field status bar strip at the bottom of a frame.
pub fn create_status_bar(parent: NativeHandle, fields: usize) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let fields = fields.max(1);

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let width = content.frame().size.width;
    let bar = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width, STATUS_BAR_HEIGHT),
        ),
    );

    let field_w = width / fields as f64;
    let mut labels = Vec::with_capacity(fields);
    for i in 0..fields {
        let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        label.setFrame(NSRect::new(
            NSPoint::new(i as f64 * field_w, 0.0),
            NSSize::new(field_w, STATUS_BAR_HEIGHT),
        ));
        label.setDrawsBackground(true);
        bar.addSubview(&label);
        labels.push(label);
    }

    content.addSubview(&bar);
    STATUSBAR_LABELS.with(|map| map.borrow_mut().insert(handle, labels));
    STATUSBAR_FIELD_COUNT.with(|map| map.borrow_mut().insert(handle, fields));
    STATUSBAR_PARENT.with(|map| map.borrow_mut().insert(handle, parent));
    store_ns_view(handle, bar);
    record_parent(handle, parent);
    handle
}

pub fn status_bar_layout(bar_handle: NativeHandle, width: u32, height: u32) {
    let _ = height;
    VIEWS.with(|map| {
        let Some(bar) = map.borrow().get(&bar_handle).cloned() else {
            return;
        };
        bar.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width as f64, STATUS_BAR_HEIGHT),
        ));
        let fields =
            STATUSBAR_FIELD_COUNT.with(|map| map.borrow().get(&bar_handle).copied().unwrap_or(1));
        let field_w = width as f64 / fields as f64;
        STATUSBAR_LABELS.with(|map| {
            if let Some(labels) = map.borrow().get(&bar_handle) {
                for (i, label) in labels.iter().enumerate() {
                    label.setFrame(NSRect::new(
                        NSPoint::new(i as f64 * field_w, 0.0),
                        NSSize::new(field_w, STATUS_BAR_HEIGHT),
                    ));
                }
            }
        });
    });
}

pub fn status_bar_set_field_text(bar_handle: NativeHandle, field: usize, text: &str) {
    STATUSBAR_LABELS.with(|map| {
        if let Some(labels) = map.borrow().get(&bar_handle) {
            if let Some(label) = labels.get(field) {
                label.setStringValue(&NSString::from_str(text));
            }
        }
    });
}

pub fn status_bar_get_field_text(bar_handle: NativeHandle, field: usize) -> String {
    STATUSBAR_LABELS.with(|map| {
        map.borrow()
            .get(&bar_handle)
            .and_then(|labels| labels.get(field))
            .map(|label| label.stringValue().to_string())
            .unwrap_or_default()
    })
}

/// Create an [`NSTabView`] tab control.
pub fn create_tab(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let tab = NSTabView::initWithFrame(
        NSTabView::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    // NSTabView nasce con autoresizing `WidthSizable | HeightSizable`,
    // che la fa espandere al content-view ad ogni resize del frame e la
    // fa sovrapporre agli altri widget. Il sizer di rust_explorer
    // posiziona la tab strip con dimensione fissa, quindi disattiviamo
    // l'autoresizing: la posizione/area è gestita da `set_view_frame`
    // chiamato da `TabInner::set_position` / `set_size`.
    tab.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewNotSizable);
    let delegate = TabViewDelegate::new(mtm, handle);
    tab.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    content.addSubview(&tab);
    TAB_VIEWS.with(|map| map.borrow_mut().insert(handle, tab.clone()));
    TAB_ITEMS.with(|map| map.borrow_mut().insert(handle, Vec::new()));
    TAB_DELEGATES.with(|map| map.borrow_mut().insert(handle, delegate));
    store_ns_view(handle, tab.into_super());
    record_parent(handle, parent);
    handle
}

pub fn set_tab_selection_handler(tab_handle: NativeHandle, handler: Box<dyn FnMut(usize)>) {
    TAB_SELECTION_HANDLERS.with(|map| {
        map.borrow_mut().insert(tab_handle, handler);
    });
}

pub fn tab_add_page(tab_handle: NativeHandle, title: &str, panel_handle: NativeHandle) -> i32 {
    let _mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let item = NSTabViewItem::new();
    item.setLabel(&NSString::from_str(title));

    if let Some(page_view) = child_view_for_handle(panel_handle) {
        page_view.setAutoresizingMask(
            objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        item.setView(Some(&page_view));
        record_parent(panel_handle, tab_handle);
    }

    let index = TAB_VIEWS.with(|map| {
        map.borrow()
            .get(&tab_handle)
            .map(|tab| {
                tab.addTabViewItem(&item);
                tab.numberOfTabViewItems() - 1
            })
            .unwrap_or(-1)
    });

    if index >= 0 {
        TAB_ITEMS.with(|map| {
            map.borrow_mut().entry(tab_handle).or_default().push(item);
        });
        TAB_PAGE_HANDLES.with(|map| {
            map.borrow_mut()
                .entry(tab_handle)
                .or_default()
                .push(panel_handle);
        });
        let page_index = index as usize;
        TAB_PAGE_IMAGE_INDICES.with(|map| {
            if let Some(indices) = map.borrow().get(&tab_handle) {
                if let Some(&image_index) = indices.get(page_index) {
                    if image_index >= 0 {
                        tab_apply_page_image(tab_handle, page_index, image_index);
                    }
                }
            }
        });
    }
    index as i32
}

pub fn tab_set_image_list(tab_handle: NativeHandle, image_list_handle: NativeHandle) {
    TAB_IMAGE_LISTS.with(|map| {
        map.borrow_mut().insert(tab_handle, image_list_handle);
    });
    TAB_PAGE_IMAGE_INDICES.with(|map| {
        if let Some(indices) = map.borrow().get(&tab_handle).cloned() {
            for (page_index, image_index) in indices.iter().enumerate() {
                if *image_index >= 0 {
                    tab_apply_page_image(tab_handle, page_index, *image_index);
                }
            }
        }
    });
}

fn tab_icon_side(tab_handle: NativeHandle) -> f64 {
    TAB_IMAGE_LISTS.with(|lists| {
        lists
            .borrow()
            .get(&tab_handle)
            .and_then(|list_h| {
                IMAGE_LISTS.with(|map| {
                    map.borrow()
                        .get(list_h)
                        .map(|s| s.width.max(s.height) as f64)
                })
            })
            .unwrap_or(16.0)
    })
}

fn tab_apply_page_image(tab_handle: NativeHandle, page_index: usize, image_index: i32) {
    if image_index < 0 {
        TAB_ITEMS.with(|map| {
            if let Some(items) = map.borrow().get(&tab_handle) {
                if let Some(item) = items.get(page_index) {
                    item.setImage(None);
                }
            }
        });
        return;
    }
    let image_list_handle = TAB_IMAGE_LISTS.with(|map| map.borrow().get(&tab_handle).copied());
    let Some(list_h) = image_list_handle else {
        return;
    };
    let Some(img) = image_list_image_at(list_h, image_index) else {
        return;
    };
    let side = tab_icon_side(tab_handle);
    let sized = image_for_display(&img, side);
    TAB_VIEWS.with(|views| {
        let apply = |item: &NSTabViewItem| {
            item.setImage(Some(&sized));
        };
        if let Some(tab) = views.borrow().get(&tab_handle) {
            let idx = page_index as isize;
            if idx >= 0 && idx < tab.numberOfTabViewItems() {
                apply(&tab.tabViewItemAtIndex(idx));
                return;
            }
        }
        TAB_ITEMS.with(|map| {
            if let Some(items) = map.borrow().get(&tab_handle) {
                if let Some(item) = items.get(page_index) {
                    apply(item);
                }
            }
        });
    });
}

pub fn tab_set_page_image(tab_handle: NativeHandle, page_index: usize, image_index: i32) {
    TAB_PAGE_IMAGE_INDICES.with(|map| {
        let mut map_mut = map.borrow_mut();
        let indices = map_mut.entry(tab_handle).or_default();
        if page_index >= indices.len() {
            indices.resize(page_index + 1, -1);
        }
        indices[page_index] = image_index;
    });
    tab_apply_page_image(tab_handle, page_index, image_index);
}

pub fn tab_get_page_count(tab_handle: NativeHandle) -> usize {
    TAB_VIEWS.with(|map| {
        map.borrow()
            .get(&tab_handle)
            .map(|tab| tab.numberOfTabViewItems().max(0) as usize)
            .unwrap_or(0)
    })
}

pub fn tab_set_page_text(tab_handle: NativeHandle, index: usize, title: &str) -> bool {
    TAB_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&tab_handle) {
            if let Some(item) = items.get(index) {
                item.setLabel(&NSString::from_str(title));
                return true;
            }
        }
        false
    })
}

/// Imposta il tooltip al passaggio del mouse su una linguetta della
/// strip (mostrato quando il mouse si ferma sul titolo della tab).
/// Esegue il mirroring di `NSTabViewItem.toolTip`.
pub fn tab_set_page_tooltip(tab_handle: NativeHandle, index: usize, text: &str) -> bool {
    TAB_ITEMS.with(|map| {
        if let Some(items) = map.borrow().get(&tab_handle) {
            if let Some(item) = items.get(index) {
                item.setToolTip(Some(&NSString::from_str(text)));
                return true;
            }
        }
        false
    })
}

pub fn tab_get_page_text(tab_handle: NativeHandle, index: usize) -> Option<String> {
    TAB_ITEMS.with(|map| {
        map.borrow()
            .get(&tab_handle)
            .and_then(|items| items.get(index))
            .map(|item| item.label().to_string())
    })
}

pub fn tab_remove_page(tab_handle: NativeHandle, index: usize) -> bool {
    // Drop the page from the thread-local bookkeeping BEFORE calling
    // `removeTabViewItem`: the removal can synchronously fire
    // `tabView:didSelectTabViewItem:` for the newly-selected tab, and
    // that delegate reads these maps with post-removal indexing.
    let item = TAB_ITEMS.with(|items| {
        items
            .borrow_mut()
            .get_mut(&tab_handle)
            .and_then(|list| (index < list.len()).then(|| list.remove(index)))
    });
    let Some(item) = item else {
        return false;
    };
    TAB_PAGE_HANDLES.with(|map| {
        if let Some(list) = map.borrow_mut().get_mut(&tab_handle) {
            if index < list.len() {
                list.remove(index);
            }
        }
    });
    TAB_PAGE_IMAGE_INDICES.with(|map| {
        if let Some(list) = map.borrow_mut().get_mut(&tab_handle) {
            if index < list.len() {
                list.remove(index);
            }
        }
    });
    TAB_VIEWS.with(|map| {
        if let Some(tab) = map.borrow().get(&tab_handle) {
            tab.removeTabViewItem(&item);
        }
    });
    true
}

pub fn tab_select_index(tab_handle: NativeHandle, index: usize) {
    TAB_VIEWS.with(|map| {
        if let Some(tab) = map.borrow().get(&tab_handle) {
            tab.selectTabViewItemAtIndex(index as isize);
        }
    });
}

pub fn tab_reapply_all_page_images(tab_handle: NativeHandle) {
    TAB_PAGE_IMAGE_INDICES.with(|map| {
        if let Some(indices) = map.borrow().get(&tab_handle).cloned() {
            for (page_index, image_index) in indices.iter().enumerate() {
                tab_apply_page_image(tab_handle, page_index, *image_index);
            }
        }
    });
}

pub fn tab_set_selection(tab_handle: NativeHandle, index: usize) {
    tab_select_index(tab_handle, index);
    if let Some(panel_handle) = TAB_PAGE_HANDLES.with(|map| {
        map.borrow()
            .get(&tab_handle)
            .and_then(|pages| pages.get(index).copied())
    }) {
        tab_layout_page(tab_handle, panel_handle);
    }
}

pub fn tab_layout_page(tab_handle: NativeHandle, panel_handle: NativeHandle) {
    let content_rect =
        TAB_VIEWS.with(|map| map.borrow().get(&tab_handle).map(|tab| tab.contentRect()));
    let Some(rect) = content_rect else {
        return;
    };
    let w = rect.size.width.max(0.0) as u32;
    let h = rect.size.height.max(0.0) as u32;
    CONTAINERS.with(|map| {
        if let Some(view) = map.borrow().get(&panel_handle) {
            view.setFrame(rect);
        }
    });
    crate::window::panel::relayout_tab_page(panel_handle, tab_handle, w, h);
}

/// Tab strip layout / chrome (maps to [`NSTabViewType`] on macOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabViewChrome {
    TopTabsBezel,
    LeftTabs,
    BottomTabs,
    RightTabs,
    NoTabsBezel,
    NoTabsLine,
    NoTabsNoBorder,
}

impl TabViewChrome {
    fn to_ns(self) -> NSTabViewType {
        match self {
            Self::TopTabsBezel => NSTabViewType::TopTabsBezelBorder,
            Self::LeftTabs => NSTabViewType::LeftTabsBezelBorder,
            Self::BottomTabs => NSTabViewType::BottomTabsBezelBorder,
            Self::RightTabs => NSTabViewType::RightTabsBezelBorder,
            Self::NoTabsBezel => NSTabViewType::NoTabsBezelBorder,
            Self::NoTabsLine => NSTabViewType::NoTabsLineBorder,
            Self::NoTabsNoBorder => NSTabViewType::NoTabsNoBorder,
        }
    }
}

/// Tab control size (maps to [`NSControlSize`] on macOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabControlSize {
    Regular,
    Small,
    Mini,
    Large,
}

impl TabControlSize {
    fn to_ns(self) -> NSControlSize {
        match self {
            Self::Regular => NSControlSize::Regular,
            Self::Small => NSControlSize::Small,
            Self::Mini => NSControlSize::Mini,
            Self::Large => NSControlSize::Large,
        }
    }
}

pub fn tab_set_view_chrome(tab_handle: NativeHandle, chrome: TabViewChrome) {
    TAB_VIEWS.with(|map| {
        if let Some(tab) = map.borrow().get(&tab_handle) {
            tab.setTabViewType(chrome.to_ns());
        }
    });
}

pub fn tab_set_control_size(tab_handle: NativeHandle, size: TabControlSize) {
    TAB_VIEWS.with(|map| {
        if let Some(tab) = map.borrow().get(&tab_handle) {
            tab.setControlSize(size.to_ns());
        }
    });
}

pub fn tab_set_draws_background(tab_handle: NativeHandle, draws: bool) {
    TAB_VIEWS.with(|map| {
        if let Some(tab) = map.borrow().get(&tab_handle) {
            tab.setDrawsBackground(draws);
        }
    });
}

pub fn tab_relayout_all_pages(tab_handle: NativeHandle) {
    TAB_PAGE_HANDLES.with(|map| {
        if let Some(pages) = map.borrow().get(&tab_handle).cloned() {
            for panel_handle in pages {
                tab_layout_page(tab_handle, panel_handle);
            }
        }
    });
    tab_reapply_all_page_images(tab_handle);
}

/// Install a menu bar built from top-level menus and their items.
pub fn install_menu_bar(frame_handle: NativeHandle, menus: &[(String, Vec<MenuInstallRow>)]) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    init_application();
    appkit_menu::clear_menu_registry();
    MENU_NS_ITEMS.with(|m| m.borrow_mut().clear());
    let bar = NSMenu::new(mtm);

    for (title, items) in menus {
        let submenu_id = appkit_menu::begin_submenu_registration();
        let submenu = NSMenu::new(mtm);
        for (id, label, enabled, kind, checked, shortcut) in items {
            if *kind == MenuItemKind::Separator {
                appkit_menu::register_submenu_separator(submenu_id);
                submenu.addItem(&NSMenuItem::separatorItem(mtm));
                continue;
            }
            appkit_menu::register_menu_item(*id, *kind, submenu_id, *checked);
            let target = ButtonTarget::new(mtm, frame_handle, *id);
            MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(&appkit_menu_label(label)),
                    Some(sel!(ruWxButtonClicked:)),
                    &NSString::from_str(""),
                )
            };
            apply_accelerator_to_menu_item(&item, shortcut.as_ref());
            unsafe {
                item.setTarget(Some(&target));
            }
            item.setEnabled(*enabled);
            if *kind == MenuItemKind::Check || *kind == MenuItemKind::Radio {
                item.setState(if *checked {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
            if let Some(img) = menu_item_icon(*id) {
                item.setImage(Some(&img));
            }
            submenu.addItem(&item);
            MENU_NS_ITEMS.with(|m| m.borrow_mut().insert(*id, item));
        }
        let top = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&appkit_menu_label(title)),
                None,
                &NSString::from_str(""),
            )
        };
        top.setSubmenu(Some(&submenu));
        bar.addItem(&top);
    }

    let app = NSApplication::sharedApplication(mtm);
    app.setMainMenu(Some(&bar));
}

/// Update the keyboard shortcut shown on a menu-bar item.
pub fn menu_set_item_shortcut(id: u16, shortcut: Option<crate::core::accelerator::Accelerator>) {
    MENU_NS_ITEMS.with(|map| {
        if let Some(item) = map.borrow().get(&id) {
            apply_accelerator_to_menu_item(item, shortcut.as_ref());
        }
    });
}

/// Update the checked state of a menu-bar item previously installed via
/// [`install_menu_bar`].
pub fn menu_set_item_checked(id: u16, checked: bool) {
    MENU_NS_ITEMS.with(|map| {
        if let Some(item) = map.borrow().get(&id) {
            item.setState(if checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    });
}

/// Converte una label in stile Win32 nel testo mostrato da AppKit.
///
/// Le label arrivano dall'app con i mnemonici `&` (`"&File"`, `"E&xit"`)
/// e con lo shortcut testuale dopo un TAB (`"Exit\tAlt+F4"`): su Windows
/// la sottolineatura e il testo a destra sono corretti, su macOS il `&`
/// comparirebbe letteralmente e lo shortcut va mostrato da AppKit come
/// key equivalent nativo (es. "⌘S"), quindi entrambi vengono rimossi.
fn appkit_menu_label(label: &str) -> String {
    let text = label.split('\t').next().unwrap_or(label);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            // `&&` e' un `&` letterale, `&X` e' il mnemonico di X.
            if chars.peek() == Some(&'&') {
                chars.next();
                out.push('&');
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Show a popup context menu for a frame at screen coordinates.
///
/// Pass `None` for both coordinates to use the current mouse location.
pub fn show_popup_menu(
    frame_handle: NativeHandle,
    items: &[MenuInstallRow],
    screen_x: Option<f64>,
    screen_y: Option<f64>,
) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let menu = NSMenu::new(mtm);
    let submenu_id = appkit_menu::begin_submenu_registration();

    for (id, label, enabled, kind, checked, shortcut) in items {
        if *kind == MenuItemKind::Separator {
            appkit_menu::register_submenu_separator(submenu_id);
            menu.addItem(&NSMenuItem::separatorItem(mtm));
            continue;
        }
        appkit_menu::register_menu_item(*id, *kind, submenu_id, *checked);
        let target = ButtonTarget::new(mtm, frame_handle, *id);
        MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&appkit_menu_label(label)),
                Some(sel!(ruWxButtonClicked:)),
                &NSString::from_str(""),
            )
        };
        apply_accelerator_to_menu_item(&item, shortcut.as_ref());
        unsafe {
            item.setTarget(Some(&target));
        }
        item.setEnabled(*enabled);
        if *kind == MenuItemKind::Check || *kind == MenuItemKind::Radio {
            item.setState(if *checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
        if let Some(img) = menu_item_icon(*id) {
            item.setImage(Some(&img));
        }
        menu.addItem(&item);
    }

    let view = FRAME_CONTENT.with(|map| map.borrow().get(&frame_handle).cloned());
    let Some(view) = view else {
        return;
    };

    let screen_point = match (screen_x, screen_y) {
        (Some(x), Some(y)) => {
            let screen_h = NSScreen::mainScreen(mtm)
                .map(|s| s.frame().size.height)
                .unwrap_or(900.0);
            // Win32 `popup_at` uses top-left screen origin; AppKit uses bottom-left.
            NSPoint::new(x, screen_h - y)
        }
        _ => NSEvent::mouseLocation(),
    };

    let view_point = if let Some(window) = view.window() {
        window.convertPointFromScreen(screen_point)
    } else {
        screen_point
    };

    menu.popUpMenuPositioningItem_atLocation_inView(None, view_point, Some(&view));
}

fn layout_spin_parts(handle: NativeHandle, width: f64, height: f64) {
    const STEPPER_WIDTH: f64 = 22.0;
    SPIN_PARTS.with(|map| {
        if let Some((field, stepper, _)) = map.borrow().get(&handle) {
            field.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new((width - STEPPER_WIDTH).max(0.0), height),
            ));
            stepper.setFrame(NSRect::new(
                NSPoint::new((width - STEPPER_WIDTH).max(0.0), 0.0),
                NSSize::new(STEPPER_WIDTH.min(width), height),
            ));
        }
    });
}

/// Create an [`NSSlider`].
pub fn create_slider(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    min: i32,
    max: i32,
    initial: i32,
    vertical: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let slider = NSSlider::initWithFrame(
        NSSlider::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    slider.setMinValue(min as f64);
    slider.setMaxValue(max as f64);
    slider.setDoubleValue(initial.clamp(min, max) as f64);
    slider.setVertical(vertical);
    slider.setContinuous(true);

    let frame_handle = resolve_frame_handle(parent);
    let target = ButtonTarget::new(mtm, frame_handle, command_id);
    MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));
    unsafe {
        slider.setTarget(Some(&target));
        slider.setAction(Some(sel!(ruWxButtonClicked:)));
    }

    content.addSubview(&slider);
    store_ns_view(handle, slider.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn slider_set_range(view_handle: NativeHandle, min: i32, max: i32) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(slider) = view.downcast_ref::<NSSlider>() {
                slider.setMinValue(min as f64);
                slider.setMaxValue(max as f64);
            }
        }
    });
}

pub fn slider_set_value(view_handle: NativeHandle, value: i32) {
    SLIDER_PROGRAMMATIC.with(|c| c.set(c.get() + 1));
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(slider) = view.downcast_ref::<NSSlider>() {
                let v = value as f64;
                let clamped = v.max(slider.minValue()).min(slider.maxValue());
                slider.setDoubleValue(clamped);
            }
        }
    });
    SLIDER_PROGRAMMATIC.with(|c| c.set(c.get() - 1));
}

pub fn slider_get_value(view_handle: NativeHandle) -> i32 {
    VIEWS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .and_then(|view| {
                view.downcast_ref::<NSSlider>()
                    .map(|s| s.doubleValue().round() as i32)
            })
            .unwrap_or(0)
    })
}

pub fn slider_set_tick_freq(view_handle: NativeHandle, freq: i32) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(slider) = view.downcast_ref::<NSSlider>() {
                slider.setAltIncrementValue(freq as f64);
            }
        }
    });
}

pub fn slider_set_page_size(view_handle: NativeHandle, page: i32) {
    let _ = (view_handle, page);
}

pub fn slider_set_line_size(view_handle: NativeHandle, line: i32) {
    let _ = (view_handle, line);
}

/// Create an [`NSProgressIndicator`] gauge.
pub fn create_gauge(
    parent: NativeHandle,
    initial_rect: Rect,
    range: i32,
    indeterminate: bool,
    vertical: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let gauge = NSProgressIndicator::initWithFrame(
        NSProgressIndicator::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    gauge.setMinValue(0.0);
    gauge.setMaxValue(range as f64);
    gauge.setDoubleValue(0.0);
    gauge.setIndeterminate(indeterminate);
    let _ = vertical;
    if indeterminate {
        unsafe {
            gauge.startAnimation(None);
        }
    }

    content.addSubview(&gauge);
    store_ns_view(handle, gauge.into_super());
    record_parent(handle, parent);
    handle
}

pub fn gauge_set_range(view_handle: NativeHandle, range: i32) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(gauge) = view.downcast_ref::<NSProgressIndicator>() {
                gauge.setMaxValue(range as f64);
            }
        }
    });
}

pub fn gauge_set_value(view_handle: NativeHandle, value: i32) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(gauge) = view.downcast_ref::<NSProgressIndicator>() {
                if gauge.isIndeterminate() {
                    return;
                }
                let clamped = (value as f64).max(gauge.minValue()).min(gauge.maxValue());
                gauge.setDoubleValue(clamped);
            }
        }
    });
}

pub fn gauge_get_value(view_handle: NativeHandle) -> i32 {
    VIEWS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .and_then(|view| {
                view.downcast_ref::<NSProgressIndicator>()
                    .map(|g| g.doubleValue().round() as i32)
            })
            .unwrap_or(0)
    })
}

pub fn gauge_pulse(view_handle: NativeHandle) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(gauge) = view.downcast_ref::<NSProgressIndicator>() {
                gauge.setIndeterminate(true);
                unsafe {
                    gauge.startAnimation(None);
                }
            }
        }
    });
}

pub fn gauge_stop_pulse(view_handle: NativeHandle) {
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(gauge) = view.downcast_ref::<NSProgressIndicator>() {
                unsafe {
                    gauge.stopAnimation(None);
                }
                gauge.setIndeterminate(false);
            }
        }
    });
}

/// Create a numeric stepper (`NSTextField` + [`NSStepper`] in a container [`NSView`]).
pub fn create_spin_ctrl(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    min: i32,
    max: i32,
    initial: i32,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let container = NSView::initWithFrame(NSView::alloc(mtm), ns_rect);

    let field = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    field.setEditable(true);
    field.setBezeled(true);
    field.setDrawsBackground(true);
    field.setStringValue(&NSString::from_str(&initial.clamp(min, max).to_string()));

    let stepper = NSStepper::initWithFrame(
        NSStepper::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(22.0, ns_rect.size.height),
        ),
    );
    stepper.setMinValue(min as f64);
    stepper.setMaxValue(max as f64);
    stepper.setDoubleValue(initial.clamp(min, max) as f64);
    stepper.setIncrement(1.0);

    let frame_handle = resolve_frame_handle(parent);
    let target = SpinStepperTarget::new(mtm, frame_handle, command_id, field.clone());
    unsafe {
        stepper.setTarget(Some(&target));
        stepper.setAction(Some(sel!(ruWxSpinChanged:)));
    }

    container.addSubview(&field);
    container.addSubview(&stepper);
    content.addSubview(&container);

    SPIN_PARTS.with(|map| {
        map.borrow_mut().insert(handle, (field, stepper, target));
    });
    layout_spin_parts(handle, ns_rect.size.width, ns_rect.size.height);
    store_ns_view(handle, container);
    record_parent(handle, parent);
    handle
}

pub fn spin_set_range(view_handle: NativeHandle, min: i32, max: i32) {
    SPIN_PARTS.with(|map| {
        if let Some((field, stepper, _)) = map.borrow().get(&view_handle) {
            stepper.setMinValue(min as f64);
            stepper.setMaxValue(max as f64);
            let v = stepper.doubleValue().round().clamp(min as f64, max as f64) as i32;
            stepper.setDoubleValue(v as f64);
            field.setStringValue(&NSString::from_str(&v.to_string()));
        }
    });
}

pub fn spin_set_value(view_handle: NativeHandle, value: i32) {
    SPIN_PARTS.with(|map| {
        if let Some((field, stepper, _)) = map.borrow().get(&view_handle) {
            let v = value as f64;
            let clamped = v.max(stepper.minValue()).min(stepper.maxValue());
            stepper.setDoubleValue(clamped);
            field.setStringValue(&NSString::from_str(&(clamped.round() as i32).to_string()));
        }
    });
}

pub fn spin_get_value(view_handle: NativeHandle) -> i32 {
    SPIN_PARTS.with(|map| {
        map.borrow()
            .get(&view_handle)
            .map(|(_, stepper, _)| stepper.doubleValue().round() as i32)
            .unwrap_or(0)
    })
}

pub fn spin_set_frame(parent: NativeHandle, view_handle: NativeHandle, rect: Rect) {
    let content_h = parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &rect);
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            view.setFrame(ns_rect);
        }
    });
    layout_spin_parts(view_handle, ns_rect.size.width, ns_rect.size.height);
}

// ── Toolbar (ToolBar / AuiToolBar) ──────────────────────────────────

const TOOLBAR_GRIPPER_WIDTH: f64 = 24.0;
const TOOLBAR_SEP_WIDTH: f64 = 8.0;

pub fn create_toolbar_strip(
    parent: NativeHandle,
    frame_handle: NativeHandle,
    with_gripper: bool,
    gripper_id: u16,
    height: i32,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let h = height.max(24) as f64;

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_w = content.frame().size.width;
    let content_h = content.frame().size.height;
    let host = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, content_h - h), NSSize::new(content_w, h)),
    );
    let strip = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, h)),
    );
    host.addSubview(&strip);
    content.addSubview(&host);

    TOOLBARS.with(|map| {
        map.borrow_mut().insert(
            handle,
            RefCell::new(ToolbarStorage {
                host: host.clone(),
                strip: strip.clone(),
                frame_handle,
                parent_handle: parent,
                image_list_handle: 0,
                image_w: 16,
                image_h: 16,
                pending: Vec::new(),
                with_gripper,
                gripper_id,
                height: h,
                floating_window: Cell::new(0),
                dock_side: Cell::new(TOOLBAR_DOCK_TOP),
                content_width: Cell::new(0.0),
                prefer_native: true,
                managed_by_aui: Cell::new(false),
            }),
        );
    });
    TOOLBARS_BY_FRAME.with(|map| {
        map.borrow_mut()
            .entry(frame_handle)
            .or_default()
            .push(handle);
    });
    store_ns_view(handle, host);
    record_parent(handle, parent);
    handle
}

pub fn toolbar_set_image_list(
    toolbar_handle: NativeHandle,
    image_list_handle: NativeHandle,
    width: i32,
    height: i32,
) {
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            let mut s = tb.borrow_mut();
            s.image_list_handle = image_list_handle;
            s.image_w = width;
            s.image_h = height;
        }
    });
}

pub fn toolbar_add_tool(toolbar_handle: NativeHandle, id: u16, image_index: i32) {
    toolbar_add_tool_with_tooltip(toolbar_handle, id, image_index, "");
}

/// Come [`toolbar_add_tool`] ma permette di specificare anche il
/// testo del tooltip mostrato all'hover del mouse (su macOS è
/// `NSView.setToolTip`, ignoto su Windows dove il tooltip arriva via
/// `TTN_GETDISPINFO` dal frame).
pub fn toolbar_add_tool_with_tooltip(
    toolbar_handle: NativeHandle,
    id: u16,
    image_index: i32,
    tooltip: &str,
) {
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            tb.borrow_mut().pending.push(ToolbarPendingItem::Tool {
                id,
                image_index,
                tooltip: tooltip.to_string(),
            });
        }
    });
}

pub fn toolbar_add_separator(toolbar_handle: NativeHandle) {
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            tb.borrow_mut().pending.push(ToolbarPendingItem::Separator);
        }
    });
}

/// When `false`, the toolbar is drawn in-client (required for AUI pane hosts).
pub fn toolbar_set_prefer_native(toolbar_handle: NativeHandle, prefer: bool) {
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            tb.borrow_mut().prefer_native = prefer;
        }
    });
}

pub fn toolbar_realize(toolbar_handle: NativeHandle) {
    #[cfg(target_os = "macos")]
    {
        if appkit_toolbar_native::should_use_native(toolbar_handle)
            && appkit_toolbar_native::toolbar_realize_native(toolbar_handle)
        {
            return;
        }
        appkit_toolbar_native::toolbar_detach_native(toolbar_handle);
    }
    toolbar_realize_legacy(toolbar_handle);
}

// `NSBezelStyleShadowlessSquare` (raw value 6) has no non-deprecated equivalent in
// objc2-app-kit 0.3; it is required for the borderless toolbar item look.
#[allow(deprecated)]
fn toolbar_realize_legacy(toolbar_handle: NativeHandle) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    TOOLBARS.with(|map| {
        let map_ref = map.borrow();
        let Some(tb) = map_ref.get(&toolbar_handle) else {
            return;
        };
        let storage = tb.borrow_mut();
        while let Some(sub) = storage.strip.subviews().firstObject() {
            sub.removeFromSuperview();
        }

        let pending = storage.pending.clone();
        let frame_handle = storage.frame_handle;
        let list_h = storage.image_list_handle;
        let img_w = storage.image_w as f64;
        let img_h = storage.image_h as f64;
        let bar_h = storage.height;
        let icon_side = toolbar_icon_side(bar_h, img_w, img_h);
        let with_gripper = storage.with_gripper;
        let gripper_id = storage.gripper_id;

        let mut x = 0.0;
        if with_gripper {
            let target = ButtonTarget::new(mtm, frame_handle, gripper_id);
            MENU_TARGETS.with(|m| m.borrow_mut().push(target.clone()));
            let gripper = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str("≡"),
                    Some(&target),
                    Some(sel!(ruWxButtonClicked:)),
                    mtm,
                )
            };
            gripper.setBezelStyle(NSBezelStyle::SmallSquare);
            gripper.setFrame(NSRect::new(
                NSPoint::new(x, 0.0),
                NSSize::new(TOOLBAR_GRIPPER_WIDTH, bar_h),
            ));
            storage.strip.addSubview(&gripper);
            x += TOOLBAR_GRIPPER_WIDTH;
        }

        for spec in &pending {
            match spec {
                ToolbarPendingItem::Separator => {
                    x += TOOLBAR_SEP_WIDTH;
                }
                ToolbarPendingItem::Tool {
                    id,
                    image_index,
                    tooltip,
                } => {
                    let target = ButtonTarget::new(mtm, frame_handle, *id);
                    MENU_TARGETS.with(|m| m.borrow_mut().push(target.clone()));
                    let btn = unsafe {
                        NSButton::buttonWithTitle_target_action(
                            &NSString::from_str(""),
                            Some(&target),
                            Some(sel!(ruWxButtonClicked:)),
                            mtm,
                        )
                    };
                    btn.setBezelStyle(NSBezelStyle::ShadowlessSquare);
                    btn.setBordered(false);
                    if !tooltip.is_empty() {
                        btn.setToolTip(Some(&NSString::from_str(tooltip)));
                    }
                    if *image_index >= 0 {
                        if let Some(img) = image_list_image_at(list_h, *image_index) {
                            let sized = image_for_display(&img, icon_side);
                            btn.setImage(Some(&sized));
                            btn.setImagePosition(NSCellImagePosition::ImageOnly);
                            btn.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                        }
                    }
                    let btn_w = icon_side + 12.0;
                    let btn_h = icon_side + 8.0;
                    btn.setFrame(NSRect::new(
                        NSPoint::new(x, (bar_h - btn_h).max(0.0) / 2.0),
                        NSSize::new(btn_w, btn_h),
                    ));
                    storage.strip.addSubview(&btn);
                    x += btn_w + 2.0;
                }
            }
        }
        storage.strip.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(x.max(1.0), bar_h),
        ));
        storage.content_width.set(x.max(1.0));
    });
    toolbar_relayout_docked(toolbar_handle);
}

/// `true` when the toolbar is backed by a window-attached [`NSToolbar`].
pub fn toolbar_is_native(toolbar_handle: NativeHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        appkit_toolbar_native::toolbar_is_native(toolbar_handle)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = toolbar_handle;
        false
    }
}

/// Update the native toolbar strip height (pixels).
pub fn toolbar_set_height(toolbar_handle: NativeHandle, height: i32) {
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            tb.borrow_mut().height = height.max(24) as f64;
        }
    });
}

pub fn toolbar_bring_to_front(toolbar_handle: NativeHandle) {
    if toolbar_is_native(toolbar_handle) {
        return;
    }
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            let host = tb.borrow().host.clone();
            unsafe {
                if let Some(superview) = host.superview() {
                    superview.addSubview(&host);
                }
            }
        }
    });
}

/// `true` when the toolbar strip lives in a floating window.
pub fn toolbar_is_floating(toolbar_handle: NativeHandle) -> bool {
    TOOLBARS.with(|map| {
        map.borrow()
            .get(&toolbar_handle)
            .map(|tb| tb.borrow().floating_window.get() != 0)
            .unwrap_or(false)
    })
}

/// When `true`, the toolbar host frame is positioned by [`AuiManager`] (in-client strip).
pub fn toolbar_set_managed_by_aui(toolbar_handle: NativeHandle, managed: bool) {
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            tb.borrow().managed_by_aui.set(managed);
        }
    });
}

fn sync_toolbar_strip_to_host(storage: &ToolbarStorage) {
    if storage.managed_by_aui.get() {
        unsafe {
            if let Some(superview) = storage.host.superview() {
                let bounds = superview.bounds();
                storage.host.setFrame(NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(bounds.size.width.max(1.0), bounds.size.height.max(1.0)),
                ));
            }
        }
    }
    let host_bounds = storage.host.bounds();
    let w = host_bounds.size.width.max(1.0);
    let h = host_bounds.size.height.max(1.0);
    storage
        .strip
        .setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h)));
    let bar_h = h;
    let mut x = 0.0;
    for sub in storage.strip.subviews().iter() {
        if let Some(btn) = sub.downcast_ref::<NSButton>() {
            let frame = btn.frame();
            let btn_h = frame.size.height;
            btn.setFrame(NSRect::new(
                NSPoint::new(x, (bar_h - btn_h).max(0.0) / 2.0),
                NSSize::new(frame.size.width, btn_h),
            ));
            x += frame.size.width + 2.0;
        }
    }
    if storage.managed_by_aui.get() && storage.image_list_handle != 0 {
        toolbar_refresh_tool_button_images(storage);
    }
}

/// Re-apply image-list glyphs to existing tool buttons (after AUI reparent/resize).
fn toolbar_refresh_tool_button_images(storage: &ToolbarStorage) {
    let list_h = storage.image_list_handle;
    if list_h == 0 {
        return;
    }
    let icon_side = toolbar_icon_side(
        storage.height,
        storage.image_w as f64,
        storage.image_h as f64,
    );
    let mut specs = storage.pending.iter();
    for sub in storage.strip.subviews().iter() {
        let Some(btn) = sub.downcast_ref::<NSButton>() else {
            continue;
        };
        if btn.title().to_string() == "≡" {
            continue;
        }
        loop {
            match specs.next() {
                Some(ToolbarPendingItem::Separator) => continue,
                Some(ToolbarPendingItem::Tool { image_index, .. }) => {
                    if *image_index >= 0 {
                        if let Some(img) = image_list_image_at(list_h, *image_index) {
                            let sized = image_for_display(&img, icon_side);
                            btn.setImage(Some(&sized));
                            btn.setImagePosition(NSCellImagePosition::ImageOnly);
                            btn.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                        }
                    }
                    break;
                }
                None => break,
            }
        }
    }
}

/// Resize the tool row to the current host bounds (after AUI layout).
pub fn toolbar_sync_host_layout(toolbar_handle: NativeHandle) {
    let needs_realize = TOOLBARS.with(|map| {
        let map_b = map.borrow();
        let Some(tb) = map_b.get(&toolbar_handle) else {
            return false;
        };
        let storage = tb.borrow();
        sync_toolbar_strip_to_host(&storage);
        storage.managed_by_aui.get()
            && !storage.prefer_native
            && storage.strip.subviews().len() <= 1
    });
    if needs_realize {
        toolbar_realize_legacy(toolbar_handle);
    }
}

fn toolbar_layout_docked(storage: &ToolbarStorage) {
    if storage.managed_by_aui.get() {
        sync_toolbar_strip_to_host(storage);
        return;
    }
    let Some(content) = parent_view_for(storage.parent_handle) else {
        return;
    };
    let host = storage.host.clone();
    let bar_h = storage.height;
    let content_w = content.frame().size.width;
    let content_h = content.frame().size.height;
    let strip_w = storage.content_width.get().max(200.0);
    let (x, y, w) = match storage.dock_side.get() {
        TOOLBAR_DOCK_BOTTOM => (0.0, 0.0, content_w),
        TOOLBAR_DOCK_LEFT => (0.0, content_h - bar_h, content_w),
        TOOLBAR_DOCK_RIGHT => {
            let w = 200.0_f64.min(content_w);
            (content_w - w, content_h - bar_h, w)
        }
        _ => (0.0, content_h - bar_h, content_w),
    };
    host.removeFromSuperview();
    host.setFrame(NSRect::new(
        NSPoint::new(x, y),
        NSSize::new(w.max(1.0), bar_h),
    ));
    content.addSubview(&host);
    storage.strip.setFrame(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(strip_w.min(w.max(1.0)), bar_h),
    ));
}

/// Re-apply dock geometry after frame resize or toolbar rebuild.
pub fn toolbar_relayout_docked(toolbar_handle: NativeHandle) {
    if toolbar_is_native(toolbar_handle) {
        return;
    }
    if toolbar_is_floating(toolbar_handle) {
        return;
    }
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            toolbar_layout_docked(&tb.borrow());
        }
    });
    toolbar_bring_to_front(toolbar_handle);
}

/// Re-dock a toolbar to `side` ([`TOOLBAR_DOCK_*`] constants).
pub fn toolbar_dock(toolbar_handle: NativeHandle, side: u8) {
    if side != TOOLBAR_DOCK_TOP {
        #[cfg(target_os = "macos")]
        {
            appkit_toolbar_native::toolbar_detach_native(toolbar_handle);
            toolbar_realize_legacy(toolbar_handle);
        }
    }
    let was_floating = toolbar_is_floating(toolbar_handle);
    if was_floating {
        TOOLBARS.with(|map| {
            if let Some(tb) = map.borrow().get(&toolbar_handle) {
                tb.borrow().dock_side.set(side);
            }
        });
        toolbar_toggle_float(toolbar_handle, 0, 0);
        return;
    }
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            let storage = tb.borrow();
            storage.dock_side.set(side);
            toolbar_layout_docked(&storage);
        }
    });
    toolbar_bring_to_front(toolbar_handle);
    #[cfg(target_os = "macos")]
    if side == TOOLBAR_DOCK_TOP && !toolbar_is_floating(toolbar_handle) {
        toolbar_realize(toolbar_handle);
    }
}

/// Toggle float/dock. Returns the new floating state (`true` = floating).
pub fn toolbar_toggle_float(toolbar_handle: NativeHandle, x: i32, y: i32) -> bool {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");

    // Phase 1 — undock: release all RefCell borrows before closing the popup
    // (window close can synchronously fire resize on the parent frame).
    let undock_popup = TOOLBARS.with(|map| {
        let map_borrow = map.borrow();
        let tb = map_borrow.get(&toolbar_handle)?;
        let storage = tb.borrow_mut();
        if storage.floating_window.get() == 0 {
            return None;
        }
        let popup = storage.floating_window.get();
        storage.floating_window.set(0);
        Some(popup)
    });
    if let Some(popup) = undock_popup {
        appkit_aui::popup_close(popup);
        TOOLBARS.with(|map| {
            if let Some(tb) = map.borrow().get(&toolbar_handle) {
                toolbar_layout_docked(&tb.borrow());
            }
        });
        toolbar_bring_to_front(toolbar_handle);
        #[cfg(target_os = "macos")]
        {
            let dock_top = TOOLBARS.with(|map| {
                map.borrow()
                    .get(&toolbar_handle)
                    .map(|tb| tb.borrow().dock_side.get() == TOOLBAR_DOCK_TOP)
                    .unwrap_or(false)
            });
            if dock_top {
                toolbar_realize(toolbar_handle);
            }
        }
        return false;
    }

    #[cfg(target_os = "macos")]
    if toolbar_is_native(toolbar_handle) {
        appkit_toolbar_native::toolbar_detach_native(toolbar_handle);
        toolbar_realize_legacy(toolbar_handle);
    }

    // Phase 2 — float: read layout data, then create the window without holding TOOLBARS.
    let float_plan = TOOLBARS.with(|map| {
        let map_borrow = map.borrow();
        let tb = map_borrow.get(&toolbar_handle)?;
        let s = tb.borrow();
        Some((s.host.clone(), s.content_width.get().max(200.0), s.height))
    });
    let Some((host, strip_w, bar_h)) = float_plan else {
        return false;
    };

    let (fx, fy) = if x == 0 && y == 0 {
        appkit_aui::cursor_win32_screen_point(mtm)
    } else {
        (x, y)
    };
    const FLOAT_TITLE_CHROME: f64 = 28.0;
    let rect = Rect::new(
        fx,
        fy,
        strip_w as u32,
        (bar_h + FLOAT_TITLE_CHROME).max(52.0) as u32,
    );
    let popup = appkit_aui::create_floating_frame("Toolbar", rect);
    appkit_aui::floating_frame_show(popup);
    if let Some(content) = child_view_for_handle(popup) {
        host.removeFromSuperview();
        let size = content.bounds().size;
        host.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(size.width, bar_h),
        ));
        TOOLBARS.with(|map| {
            if let Some(tb) = map.borrow().get(&toolbar_handle) {
                tb.borrow().strip.setFrame(NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(strip_w.min(size.width), bar_h),
                ));
            }
        });
        content.addSubview(&host);
    }
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            tb.borrow().floating_window.set(popup);
        }
    });
    true
}

/// Float at screen coordinates (does nothing if already floating).
pub fn toolbar_float_at(toolbar_handle: NativeHandle, x: i32, y: i32) {
    if !toolbar_is_floating(toolbar_handle) {
        toolbar_toggle_float(toolbar_handle, x, y);
    }
}

/// Re-dock a floating toolbar to the top of the parent content view.
pub fn toolbar_dock_top(toolbar_handle: NativeHandle) {
    toolbar_dock(toolbar_handle, TOOLBAR_DOCK_TOP);
}

// ── Popup with per-row icons (BitmapComboBox) ───────────────────────

pub fn popup_set_image_list(view_handle: NativeHandle, image_list_handle: NativeHandle) {
    POPUP_IMAGE_LISTS.with(|map| {
        map.borrow_mut().insert(view_handle, image_list_handle);
    });
}

pub fn popup_append_item_with_image(view_handle: NativeHandle, item: &str, image_index: i32) {
    popup_append_item(view_handle, item);
    if image_index < 0 {
        return;
    }
    let list_h = POPUP_IMAGE_LISTS.with(|map| map.borrow().get(&view_handle).copied().unwrap_or(0));
    if list_h == 0 {
        return;
    }
    let Some(img) = image_list_image_at(list_h, image_index) else {
        return;
    };
    VIEWS.with(|map| {
        if let Some(view) = map.borrow().get(&view_handle) {
            if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                let idx = popup.numberOfItems() - 1;
                if idx >= 0 {
                    if let Some(menu_item) = popup.itemAtIndex(idx) {
                        menu_item.setImage(Some(&img));
                    }
                }
            }
        }
    });
}

pub fn popup_insert_item_with_image(
    view_handle: NativeHandle,
    index: usize,
    item: &str,
    image_index: i32,
) -> i32 {
    popup_insert_item(view_handle, index, item);
    if image_index >= 0 {
        let list_h =
            POPUP_IMAGE_LISTS.with(|map| map.borrow().get(&view_handle).copied().unwrap_or(0));
        if list_h != 0 {
            if let Some(img) = image_list_image_at(list_h, image_index) {
                VIEWS.with(|map| {
                    if let Some(view) = map.borrow().get(&view_handle) {
                        if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                            let idx = index as isize;
                            if let Some(menu_item) = popup.itemAtIndex(idx) {
                                menu_item.setImage(Some(&img));
                            }
                        }
                    }
                });
            }
        }
    }
    index as i32
}

/// Wire a selection-change callback for a popup/combobox.
/// On macOS this stores the callback in thread-local storage.
/// The callback is invoked when the popup selection changes.
pub fn popup_wire_selection_callback(view_handle: NativeHandle, callback: Box<dyn FnMut()>) {
    POPUP_SELECTION_HANDLERS.with(|map| {
        map.borrow_mut().insert(view_handle, callback);
    });
}

// ── TreeCtrl custom view ────────────────────────────────────────────

pub const TREE_NOTIFY_SELCHANGED: u32 = 0xFFFFFE6E;

const TREE_ROW_HEIGHT: f64 = 20.0;
const TREE_INDENT: f64 = 16.0;
const TREE_ICON_WIDTH: f64 = 18.0;

#[derive(Default)]
struct TreeViewIvars {
    tree_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    notify_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TreeViewIvars]
    struct TreeView;

    unsafe impl NSObjectProtocol for TreeView {}

    impl TreeView {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().tree_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let rows = tree_visible_rows(handle);
            if rows.is_empty() {
                return;
            }
            let total_h = rows.len() as f64 * TREE_ROW_HEIGHT;
            let row_idx = ((total_h - point.y) / TREE_ROW_HEIGHT).floor() as isize;
            if row_idx < 0 || row_idx as usize >= rows.len() {
                return;
            }
            let node_id = rows[row_idx as usize];

            if event.clickCount() >= 2 {
                TREE_NODES.with(|map| {
                    if let Some(nodes) = map.borrow().get(&handle) {
                        if let Some(node) = nodes.borrow_mut().get_mut(&node_id) {
                            if !node.children.is_empty() {
                                node.expanded = !node.expanded;
                            }
                        }
                    }
                });
                tree_redraw(handle);
            }

            TREE_SELECTED.with(|map| {
                if let Some(sel) = map.borrow().get(&handle) {
                    *sel.borrow_mut() = Some(node_id);
                }
            });
            tree_redraw(handle);

            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().notify_id.get(),
                TREE_NOTIFY_SELCHANGED,
            );
        }
    }
);

impl TreeView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        tree_handle: NativeHandle,
        frame_handle: NativeHandle,
        notify_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TreeViewIvars {
            tree_handle: Cell::new(tree_handle),
            frame_handle: Cell::new(frame_handle),
            notify_id: Cell::new(notify_id),
        });
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }
}

pub(super) fn tree_init_storage(tree_handle: NativeHandle) {
    TREE_NODES.with(|m| {
        m.borrow_mut()
            .insert(tree_handle, RefCell::new(HashMap::new()));
    });
    TREE_ROOTS.with(|m| {
        m.borrow_mut().insert(tree_handle, RefCell::new(Vec::new()));
    });
    TREE_NEXT_ID.with(|m| {
        m.borrow_mut().insert(tree_handle, Cell::new(1));
    });
    TREE_SELECTED.with(|m| {
        m.borrow_mut().insert(tree_handle, RefCell::new(None));
    });
}

fn tree_alloc_node(
    tree_handle: NativeHandle,
    parent: Option<usize>,
    text: &str,
    image_index: Option<i32>,
) -> usize {
    let id = TREE_NEXT_ID.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .map(|cell| {
                let id = cell.get();
                cell.set(id + 1);
                id
            })
            .unwrap_or(1)
    });

    TREE_NODES.with(|map| {
        if let Some(nodes) = map.borrow().get(&tree_handle) {
            nodes.borrow_mut().insert(
                id,
                TreeNodeData {
                    text: text.to_string(),
                    image_index,
                    parent,
                    children: Vec::new(),
                    expanded: false,
                },
            );
        }
    });

    if let Some(p) = parent {
        TREE_NODES.with(|map| {
            if let Some(nodes) = map.borrow().get(&tree_handle) {
                if let Some(node) = nodes.borrow_mut().get_mut(&p) {
                    node.children.push(id);
                }
            }
        });
    } else {
        TREE_ROOTS.with(|map| {
            if let Some(roots) = map.borrow().get(&tree_handle) {
                roots.borrow_mut().push(id);
            }
        });
    }
    id
}

pub(super) fn tree_children_count_for_item(
    tree_handle: NativeHandle,
    parent: Option<usize>,
) -> usize {
    match parent {
        None => TREE_ROOTS.with(|map| {
            map.borrow()
                .get(&tree_handle)
                .map(|r| r.borrow().len())
                .unwrap_or(0)
        }),
        Some(node_id) => TREE_NODES.with(|map| {
            map.borrow()
                .get(&tree_handle)
                .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.children.len()))
                .unwrap_or(0)
        }),
    }
}

pub(super) fn tree_child_at_for_item(
    tree_handle: NativeHandle,
    parent: Option<usize>,
    index: usize,
) -> Option<usize> {
    match parent {
        None => TREE_ROOTS.with(|map| {
            map.borrow()
                .get(&tree_handle)
                .and_then(|r| r.borrow().get(index).copied())
        }),
        Some(node_id) => TREE_NODES.with(|map| {
            map.borrow().get(&tree_handle).and_then(|nodes| {
                nodes
                    .borrow()
                    .get(&node_id)
                    .and_then(|n| n.children.get(index).copied())
            })
        }),
    }
}

pub(super) fn tree_node_has_children(tree_handle: NativeHandle, node_id: usize) -> bool {
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| !n.children.is_empty()))
            .unwrap_or(false)
    })
}

pub(super) fn tree_node_text(tree_handle: NativeHandle, node_id: usize) -> String {
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.text.clone()))
            .unwrap_or_default()
    })
}

pub(super) fn tree_node_image_index(tree_handle: NativeHandle, node_id: usize) -> Option<i32> {
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).and_then(|n| n.image_index))
    })
}

pub(super) fn tree_image_list_for(tree_handle: NativeHandle) -> NativeHandle {
    TREE_IMAGE_LISTS.with(|map| map.borrow().get(&tree_handle).copied().unwrap_or(0))
}

pub(super) fn tree_read_selected(tree_handle: NativeHandle) -> Option<usize> {
    TREE_SELECTED.with(|map| map.borrow().get(&tree_handle).and_then(|s| *s.borrow()))
}

pub(super) fn tree_write_selected(tree_handle: NativeHandle, node_id: Option<usize>) {
    TREE_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&tree_handle) {
            *sel.borrow_mut() = node_id;
        }
    });
}

pub(super) fn tree_set_expanded(tree_handle: NativeHandle, node_id: usize, expanded: bool) {
    TREE_NODES.with(|map| {
        if let Some(nodes) = map.borrow().get(&tree_handle) {
            if let Some(node) = nodes.borrow_mut().get_mut(&node_id) {
                node.expanded = expanded;
            }
        }
    });
}

pub(super) fn tree_is_expanded(tree_handle: NativeHandle, node_id: usize) -> bool {
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.expanded))
            .unwrap_or(false)
    })
}

pub(super) fn tree_collect_expanded_nodes(tree_handle: NativeHandle) -> Vec<usize> {
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .map(|nodes| {
                nodes
                    .borrow()
                    .iter()
                    .filter_map(|(id, node)| node.expanded.then_some(*id))
                    .collect()
            })
            .unwrap_or_default()
    })
}

fn tree_collect_visible(tree_handle: NativeHandle, node_id: usize, out: &mut Vec<usize>) {
    out.push(node_id);
    let expanded = TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.expanded))
            .unwrap_or(false)
    });
    if !expanded {
        return;
    }
    let children: Vec<usize> = TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.children.clone()))
            .unwrap_or_default()
    });
    for child in children {
        tree_collect_visible(tree_handle, child, out);
    }
}

fn tree_visible_rows(tree_handle: NativeHandle) -> Vec<usize> {
    let roots = TREE_ROOTS.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .map(|r| r.borrow().clone())
            .unwrap_or_default()
    });
    let mut rows = Vec::new();
    for root in roots {
        tree_collect_visible(tree_handle, root, &mut rows);
    }
    rows
}

fn tree_node_depth(tree_handle: NativeHandle, node_id: usize) -> usize {
    let mut depth = 0;
    let mut current = Some(node_id);
    while let Some(id) = current {
        current = TREE_NODES.with(|map| {
            map.borrow()
                .get(&tree_handle)
                .and_then(|nodes| nodes.borrow().get(&id).and_then(|n| n.parent))
        });
        if current.is_some() {
            depth += 1;
        }
    }
    depth
}

fn tree_redraw(tree_handle: NativeHandle) {
    if appkit_tree_outline::tree_is_native_outline(tree_handle) {
        appkit_tree_outline::tree_reload(tree_handle);
        return;
    }
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    TREE_VIEWS.with(|views| {
        let view = views.borrow().get(&tree_handle).cloned();
        let Some(view) = view else {
            return;
        };
        while let Some(sub) = view.subviews().firstObject() {
            sub.removeFromSuperview();
        }

        let rows = tree_visible_rows(tree_handle);
        let selected =
            TREE_SELECTED.with(|map| map.borrow().get(&tree_handle).and_then(|s| *s.borrow()));
        let list_h =
            TREE_IMAGE_LISTS.with(|map| map.borrow().get(&tree_handle).copied().unwrap_or(0));
        let width = view.frame().size.width;
        let total_h = rows.len() as f64 * TREE_ROW_HEIGHT;
        view.setFrameSize(NSSize::new(width, total_h.max(TREE_ROW_HEIGHT)));

        for (row_idx, node_id) in rows.iter().enumerate() {
            let row_y = total_h - (row_idx as f64 + 1.0) * TREE_ROW_HEIGHT;
            let depth = tree_node_depth(tree_handle, *node_id);
            let (text, image_index, has_children, expanded) = TREE_NODES.with(|map| {
                map.borrow()
                    .get(&tree_handle)
                    .and_then(|nodes| nodes.borrow().get(node_id).cloned())
                    .map(|n| {
                        (
                            n.text.clone(),
                            n.image_index,
                            !n.children.is_empty(),
                            n.expanded,
                        )
                    })
                    .unwrap_or_default()
            });

            let mut x = depth as f64 * TREE_INDENT;
            if has_children {
                let marker = if expanded { "▼" } else { "▶" };
                let disclosure = NSTextField::labelWithString(&NSString::from_str(marker), mtm);
                disclosure.setFrame(NSRect::new(
                    NSPoint::new(x, row_y),
                    NSSize::new(12.0, TREE_ROW_HEIGHT),
                ));
                view.addSubview(&disclosure);
                x += 12.0;
            }

            if let Some(img_idx) = image_index.filter(|i| *i >= 0) {
                if let Some(img) = image_list_image_at(list_h, img_idx) {
                    let icon = NSImageView::initWithFrame(
                        NSImageView::alloc(mtm),
                        NSRect::new(
                            NSPoint::new(x, row_y + 1.0),
                            NSSize::new(TREE_ICON_WIDTH - 2.0, TREE_ROW_HEIGHT - 2.0),
                        ),
                    );
                    icon.setImage(Some(&img));
                    icon.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                    view.addSubview(&icon);
                }
                x += TREE_ICON_WIDTH;
            }

            let label = NSTextField::labelWithString(&NSString::from_str(&text), mtm);
            label.setFrame(NSRect::new(
                NSPoint::new(x, row_y),
                NSSize::new((width - x).max(20.0), TREE_ROW_HEIGHT),
            ));
            if selected == Some(*node_id) {
                label.setDrawsBackground(true);
                label.setBackgroundColor(Some(
                    &objc2_app_kit::NSColor::selectedTextBackgroundColor(),
                ));
            }
            view.addSubview(&label);
        }
    });
}

pub fn create_tree_ctrl(parent: NativeHandle, notify_id: u16, initial_rect: Rect) -> NativeHandle {
    appkit_tree_outline::create_tree_ctrl_native(parent, notify_id, initial_rect)
}

pub fn tree_set_image_list(tree_handle: NativeHandle, image_list_handle: NativeHandle) {
    TREE_IMAGE_LISTS.with(|map| {
        map.borrow_mut().insert(tree_handle, image_list_handle);
    });
    tree_redraw(tree_handle);
}

pub fn tree_add_root_with_image(tree_handle: NativeHandle, text: &str, image_index: i32) -> isize {
    let img = if image_index >= 0 {
        Some(image_index)
    } else {
        None
    };
    let id = tree_alloc_node(tree_handle, None, text, img);
    tree_redraw(tree_handle);
    id as isize
}

pub fn tree_append_item_with_image(
    tree_handle: NativeHandle,
    parent: isize,
    text: &str,
    image_index: i32,
) -> isize {
    if parent == 0 {
        return 0;
    }
    let img = if image_index >= 0 {
        Some(image_index)
    } else {
        None
    };
    let id = tree_alloc_node(tree_handle, Some(parent as usize), text, img);
    tree_redraw(tree_handle);
    id as isize
}

pub fn tree_get_item_text(tree_handle: NativeHandle, item: isize) -> Option<String> {
    if item == 0 {
        return None;
    }
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&(item as usize)).map(|n| n.text.clone()))
    })
}

pub fn tree_get_selection(tree_handle: NativeHandle) -> Option<isize> {
    TREE_SELECTED.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|s| *s.borrow())
            .map(|id| id as isize)
    })
}

pub fn tree_select_item(tree_handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    if appkit_tree_outline::tree_is_native_outline(tree_handle) {
        appkit_tree_outline::tree_select_item_native(tree_handle, item as usize);
        return;
    }
    TREE_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&tree_handle) {
            *sel.borrow_mut() = Some(item as usize);
        }
    });
    tree_redraw(tree_handle);
}

pub fn tree_expand(tree_handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    if appkit_tree_outline::tree_is_native_outline(tree_handle) {
        appkit_tree_outline::tree_expand_item(tree_handle, item as usize);
    }
}

/// Imposta la dimensione del font (in punti) per la NSOutlineView.
/// Usato dalle preferenze utente (`Strumenti → Opzioni…`).
pub fn tree_set_font_size(tree_handle: NativeHandle, point_size: f64) {
    if appkit_tree_outline::tree_is_native_outline(tree_handle) {
        appkit_tree_outline::tree_set_font_size_native(tree_handle, point_size);
    }
}

pub fn tree_collapse(tree_handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    if appkit_tree_outline::tree_is_native_outline(tree_handle) {
        appkit_tree_outline::tree_collapse_item(tree_handle, item as usize);
        return;
    }
    TREE_NODES.with(|map| {
        if let Some(nodes) = map.borrow().get(&tree_handle) {
            if let Some(node) = nodes.borrow_mut().get_mut(&(item as usize)) {
                node.expanded = false;
            }
        }
    });
    tree_redraw(tree_handle);
}

pub fn tree_get_root_item(tree_handle: NativeHandle) -> Option<isize> {
    TREE_ROOTS.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|roots| roots.borrow().first().copied())
            .map(|id| id as isize)
    })
}

pub fn tree_get_first_child(tree_handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| {
                nodes
                    .borrow()
                    .get(&(item as usize))
                    .and_then(|n| n.children.first().copied())
            })
            .map(|id| id as isize)
    })
}

pub fn tree_get_next_sibling(tree_handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    let parent = TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&(item as usize)).and_then(|n| n.parent))
    });
    match parent {
        Some(p) => TREE_NODES.with(|map| {
            map.borrow()
                .get(&tree_handle)
                .and_then(|nodes| {
                    let nodes = nodes.borrow();
                    let parent_node = nodes.get(&p)?;
                    let pos = parent_node
                        .children
                        .iter()
                        .position(|&c| c == item as usize)?;
                    parent_node.children.get(pos + 1).copied()
                })
                .map(|id| id as isize)
        }),
        None => TREE_ROOTS.with(|map| {
            map.borrow()
                .get(&tree_handle)
                .and_then(|roots| {
                    let roots = roots.borrow();
                    let pos = roots.iter().position(|&c| c == item as usize)?;
                    roots.get(pos + 1).copied()
                })
                .map(|id| id as isize)
        }),
    }
}

pub fn tree_get_item_parent(tree_handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&(item as usize)).and_then(|n| n.parent))
            .map(|id| id as isize)
    })
}

fn tree_delete_subtree(tree_handle: NativeHandle, node_id: usize) {
    let children: Vec<usize> = TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.children.clone()))
            .unwrap_or_default()
    });
    for child in children {
        tree_delete_subtree(tree_handle, child);
    }
    TREE_NODES.with(|map| {
        if let Some(nodes) = map.borrow().get(&tree_handle) {
            nodes.borrow_mut().remove(&node_id);
        }
    });
}

pub fn tree_delete_item(tree_handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    let node_id = item as usize;
    let parent = TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).and_then(|n| n.parent))
    });
    if let Some(p) = parent {
        TREE_NODES.with(|map| {
            if let Some(nodes) = map.borrow().get(&tree_handle) {
                if let Some(node) = nodes.borrow_mut().get_mut(&p) {
                    node.children.retain(|&c| c != node_id);
                }
            }
        });
    } else {
        TREE_ROOTS.with(|map| {
            if let Some(roots) = map.borrow().get(&tree_handle) {
                roots.borrow_mut().retain(|&c| c != node_id);
            }
        });
    }
    TREE_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&tree_handle) {
            if *sel.borrow() == Some(node_id) {
                *sel.borrow_mut() = None;
            }
        }
    });
    tree_delete_subtree(tree_handle, node_id);
    tree_redraw(tree_handle);
}

pub fn tree_delete_all_items(tree_handle: NativeHandle) {
    TREE_NODES.with(|map| {
        if let Some(nodes) = map.borrow().get(&tree_handle) {
            nodes.borrow_mut().clear();
        }
    });
    TREE_ROOTS.with(|map| {
        if let Some(roots) = map.borrow().get(&tree_handle) {
            roots.borrow_mut().clear();
        }
    });
    TREE_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&tree_handle) {
            *sel.borrow_mut() = None;
        }
    });
    TREE_NEXT_ID.with(|map| {
        if let Some(counter) = map.borrow().get(&tree_handle) {
            counter.set(1);
        }
    });
    tree_redraw(tree_handle);
}

pub fn tree_get_count(tree_handle: NativeHandle) -> usize {
    TREE_NODES.with(|map| {
        map.borrow()
            .get(&tree_handle)
            .map(|nodes| nodes.borrow().len())
            .unwrap_or(0)
    })
}

pub fn tree_redraw_items(tree_handle: NativeHandle) {
    tree_redraw(tree_handle);
}

#[cfg(test)]
mod list_activate_route_tests {
    //! Verifica che il doppio clic di una riga della lista
    //! (`ITEM_DBLCLK`) venga instradato verso l'handler registrato con
    //! [`crate::window::frame::Frame::register_lv_item_activate_handler`],
    //! la stessa via usata dal WndProc su Windows per
    //! `LVN_ITEMACTIVATE`.
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn list_activate_item_index_decodes_row_from_lparam() {
        assert_eq!(crate::platform::shell::list_activate_item_index(0), Some(0));
        assert_eq!(crate::platform::shell::list_activate_item_index(7), Some(7));
        assert_eq!(crate::platform::shell::list_activate_item_index(-1), None);
    }

    #[test]
    fn dblclk_routes_row_to_activate_handler() {
        let frame = Frame::for_testing();
        let handle: NativeHandle = 0x5EED_DB1C;
        register_frame_weak(handle, &frame);

        let seen = Rc::new(Cell::new(-1isize));
        {
            let seen = Rc::clone(&seen);
            frame.register_lv_item_activate_handler(11, Box::new(move |lp| seen.set(lp)));
        }

        listctrl_defer_notify_row(handle, 11, LISTCTRL_NOTIFY_ITEM_DBLCLK, 3);
        listctrl_flush_deferred();

        assert_eq!(seen.get(), 3);
    }

    #[test]
    fn item_changed_does_not_trigger_activate_handler() {
        let frame = Frame::for_testing();
        let handle: NativeHandle = 0x5EED_DB1D;
        register_frame_weak(handle, &frame);

        let called = Rc::new(Cell::new(false));
        {
            let called = Rc::clone(&called);
            frame.register_lv_item_activate_handler(12, Box::new(move |_| called.set(true)));
        }

        listctrl_defer_notify_row(handle, 12, LISTCTRL_NOTIFY_ITEM_CHANGED, 1);
        listctrl_flush_deferred();

        assert!(!called.get());
    }
}

#[cfg(test)]
mod raw_dispatch_resilience_tests {
    //! Regressione per il bug "dopo un errore i cambi cartella non
    //! vengono piu' letti": un panic in un handler di messaggi
    //! worker→UI non deve far perdere l'handler ne' bloccare gli
    //! handler registrati dopo di esso.
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Handle fittizi: i thread-local sono isolati per thread di test.
    const FRAME: NativeHandle = 0x5EED_AA01;
    const MSG: u32 = 0x0410;

    #[test]
    fn panicking_handler_survives_and_keeps_receiving() {
        let calls = Rc::new(Cell::new(0));
        {
            let calls = Rc::clone(&calls);
            register_raw_handler(
                FRAME,
                MSG,
                Box::new(move |_, _| {
                    calls.set(calls.get() + 1);
                    panic!("boom simulato");
                }),
            );
        }

        deliver_raw_message(FRAME, MSG, 1, 2);
        deliver_raw_message(FRAME, MSG, 3, 4);

        // Senza catch_unwind + reinserimento la seconda consegna
        // avrebbe trovato la lista vuota: calls resterebbe a 1.
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn panic_in_first_handler_does_not_starve_the_next() {
        let first = Rc::new(Cell::new(0));
        let second = Rc::new(Cell::new(0));
        {
            let first = Rc::clone(&first);
            register_raw_handler(
                FRAME,
                MSG,
                Box::new(move |_, _| {
                    first.set(first.get() + 1);
                    panic!("boom simulato");
                }),
            );
        }
        {
            let second = Rc::clone(&second);
            register_raw_handler(
                FRAME,
                MSG,
                Box::new(move |_, _| second.set(second.get() + 1)),
            );
        }

        // Gli handler sono serviti in rotazione: la prima consegna
        // colpisce il primo (che panic-ca), la seconda il successivo.
        deliver_raw_message(FRAME, MSG, 0, 0);
        deliver_raw_message(FRAME, MSG, 0, 0);
        deliver_raw_message(FRAME, MSG, 0, 0);

        assert_eq!(first.get(), 2);
        assert_eq!(second.get(), 1);
    }

    #[test]
    fn posted_messages_are_delivered_by_drain() {
        let seen = Rc::new(Cell::new(0isize));
        {
            let seen = Rc::clone(&seen);
            register_raw_handler(FRAME, MSG, Box::new(move |w, _| seen.set(seen.get() + w)));
        }

        post_raw_message(FRAME, MSG, 5, 0);
        post_raw_message(FRAME, MSG, 7, 0);
        drain_raw_messages();

        assert_eq!(seen.get(), 12);
    }
}

#[cfg(test)]
mod list_checkbox_tests {
    //! Regressione per la selezione multipla su macOS: i metodi
    //! checkbox del `ListCtrl` erano stub no-op fuori da Windows,
    //! quindi la colonna non appariva mai e lo stato andava perso.
    //! Qui si verificano flag di abilitazione, stato marcato e
    //! riallineamento degli indici su insert/delete/delete_all.
    use super::*;

    /// Handle fittizio: i thread-local sono isolati per thread di test.
    const LIST: NativeHandle = 0x5EED_CC01;

    fn fresh_list(handle: NativeHandle, rows: usize) {
        listctrl_init_storage(handle);
        for i in 0..rows {
            listctrl_insert_item(handle, i, &format!("riga {i}"), None);
        }
    }

    #[test]
    fn checkboxes_flag_roundtrip() {
        fresh_list(LIST, 0);
        assert!(!listctrl_checkboxes_enabled(LIST));
        listctrl_set_checkboxes(LIST, true);
        assert!(listctrl_checkboxes_enabled(LIST));
        listctrl_set_checkboxes(LIST, false);
        assert!(!listctrl_checkboxes_enabled(LIST));
    }

    #[test]
    fn set_checked_and_checked_items_are_sorted() {
        fresh_list(LIST, 4);
        assert!(!listctrl_is_checked(LIST, 1));
        listctrl_set_checked(LIST, 3, true);
        listctrl_set_checked(LIST, 1, true);
        assert!(listctrl_is_checked(LIST, 1));
        assert!(!listctrl_is_checked(LIST, 2));
        assert_eq!(listctrl_checked_items(LIST), vec![1, 3]);
        listctrl_set_checked(LIST, 1, false);
        assert_eq!(listctrl_checked_items(LIST), vec![3]);
    }

    #[test]
    fn delete_item_shifts_checked_indices() {
        fresh_list(LIST, 5);
        listctrl_set_checked(LIST, 1, true);
        listctrl_set_checked(LIST, 3, true);
        listctrl_delete_item(LIST, 1);
        // 1 rimosso; 3 diventa 2.
        assert_eq!(listctrl_checked_items(LIST), vec![2]);
    }

    #[test]
    fn insert_item_shifts_checked_indices_up() {
        fresh_list(LIST, 3);
        listctrl_set_checked(LIST, 2, true);
        listctrl_insert_item(LIST, 0, "nuova", None);
        assert_eq!(listctrl_checked_items(LIST), vec![3]);
    }

    #[test]
    fn delete_all_clears_checked() {
        fresh_list(LIST, 3);
        listctrl_set_all_checked(LIST, true);
        assert_eq!(listctrl_checked_items(LIST), vec![0, 1, 2]);
        listctrl_delete_all_items(LIST);
        assert!(listctrl_checked_items(LIST).is_empty());
    }

    #[test]
    fn set_all_checked_toggles_everything() {
        fresh_list(LIST, 3);
        listctrl_set_all_checked(LIST, true);
        assert_eq!(listctrl_checked_items(LIST), vec![0, 1, 2]);
        listctrl_set_all_checked(LIST, false);
        assert!(listctrl_checked_items(LIST).is_empty());
    }
}
