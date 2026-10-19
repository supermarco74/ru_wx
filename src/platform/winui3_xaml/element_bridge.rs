//! Core bridge: ru_wx imperative widget model → reactor declarative Element tree.
//!
//! ru_wx widgets are created imperatively (`Button::new(&frame, "Click")`)
//! and mutated via setters (`btn.set_label("New")`). Reactor builds an
//! `Element` tree declaratively at render time. This module holds the
//! intermediate representation: widget descriptors collected during the
//! imperative phase, converted to reactor `Element`s when the app runs.
//!
//! # Thread model
//!
//! Reactor requires `Send` closures for `App::render()`. The bridge uses:
//! - `Rc<RefCell<…>>` during the single-threaded builder phase
//! - `Arc<Mutex<…>>` for the `EventRegistry` shared into the render closure
//! - Event handlers are `Box<dyn FnMut(…) + Send>` to satisfy reactor's `Send` bound

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// ── Widget ID ────────────────────────────────────────────────────────────────

/// Unique identifier for each widget created through the bridge.
pub type WidgetId = u64;

/// Generates sequential widget IDs (1, 2, 3, …).
pub(crate) struct IdGenerator {
    next: std::cell::Cell<WidgetId>,
}

impl IdGenerator {
    pub(crate) const fn new() -> Self {
        Self {
            next: std::cell::Cell::new(1),
        }
    }

    pub(crate) fn next(&self) -> WidgetId {
        let id = self.next.get();
        self.next.set(id + 1);
        id
    }
}

thread_local! {
    static ID_GEN: IdGenerator = const { IdGenerator::new() };
}

/// Allocate a fresh widget ID.
pub(crate) fn next_id() -> WidgetId {
    ID_GEN.with(|g| g.next())
}

// ── Event handler storage ────────────────────────────────────────────────────

/// Type-erased event handler. All handlers must be `Send` because reactor
/// requires `Send` closures. In practice all callbacks run on the UI thread,
/// so the `Send` bound is a formality for the type system.
pub(crate) type SimpleHandler = Box<dyn FnMut() + Send>;
pub(crate) type TextHandler = Box<dyn FnMut(String) + Send>;
pub(crate) type IndexHandler = Box<dyn FnMut(usize) + Send>;
pub(crate) type BoolHandler = Box<dyn FnMut(bool) + Send>;
pub(crate) type ResizeHandler = Box<dyn FnMut(i32, i32) + Send>;

/// Central registry mapping widget IDs to their event handlers.
///
/// Shared via `Arc<Mutex>` into the reactor render closure.
pub struct EventRegistry {
    pub(crate) click_handlers: HashMap<WidgetId, SimpleHandler>,
    pub(crate) text_changed_handlers: HashMap<WidgetId, TextHandler>,
    pub(crate) selection_changed_handlers: HashMap<WidgetId, IndexHandler>,
    pub(crate) toggled_handlers: HashMap<WidgetId, BoolHandler>,
    pub(crate) close_handlers: HashMap<WidgetId, SimpleHandler>,
    pub(crate) resize_handlers: HashMap<WidgetId, ResizeHandler>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            click_handlers: HashMap::new(),
            text_changed_handlers: HashMap::new(),
            selection_changed_handlers: HashMap::new(),
            toggled_handlers: HashMap::new(),
            close_handlers: HashMap::new(),
            resize_handlers: HashMap::new(),
        }
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe shared reference to the event registry.
pub type SharedRegistry = Arc<Mutex<EventRegistry>>;

/// Create a new shared registry.
pub fn new_shared_registry() -> SharedRegistry {
    Arc::new(Mutex::new(EventRegistry::new()))
}

// ── Widget descriptors ───────────────────────────────────────────────────────

/// Orientation for sizer-based layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XamlOrientation {
    Vertical,
    Horizontal,
}

/// Describes a widget to be created inside the reactor element tree.
///
/// All fields are `Send`-safe (String, Vec, f64, bool) so the tree can be
/// cloned and moved into the reactor render closure.
#[derive(Debug, Clone)]
pub enum WidgetDesc {
    /// Simple text label.
    TextBlock {
        id: WidgetId,
        text: String,
        font_size: f64,
        bold: bool,
    },
    /// Push button.
    Button {
        id: WidgetId,
        label: String,
        enabled: bool,
        /// Larghezza fissa in px logici (0.0 = automatica).
        width: f64,
    },
    /// Single-line text input.
    TextBox {
        id: WidgetId,
        text: String,
        placeholder: String,
        read_only: bool,
    },
    /// Multi-line text input.
    TextArea {
        id: WidgetId,
        text: String,
        read_only: bool,
    },
    /// Checkbox with label.
    CheckBox {
        id: WidgetId,
        label: String,
        checked: bool,
    },
    /// Radio button with label and group.
    RadioButton {
        id: WidgetId,
        label: String,
        group: String,
        checked: bool,
    },
    /// Slider (min, max, current).
    Slider {
        id: WidgetId,
        min: f64,
        max: f64,
        value: f64,
    },
    /// Progress bar (0..max).
    ProgressBar {
        id: WidgetId,
        max: f64,
        value: f64,
        indeterminate: bool,
    },
    /// Dropdown / combo box.
    ComboBox {
        id: WidgetId,
        items: Vec<String>,
        selected: Option<usize>,
        editable: bool,
        /// Larghezza fissa in px logici (0.0 = automatica).
        width: f64,
    },
    /// Vertical stack panel.
    VStack {
        id: WidgetId,
        children: Vec<WidgetDesc>,
        spacing: f64,
        /// `true` se costruito da un sizer dichiarato (es. `XamlBoxSizer`):
        /// nell'inspector produce il ruolo `group` (cfr.
        /// `element_tree::ROLE_MARKER_PREFIX`).
        group: bool,
    },
    /// Horizontal stack panel.
    HStack {
        id: WidgetId,
        children: Vec<WidgetDesc>,
        spacing: f64,
        /// Come [`Self::VStack::group`].
        group: bool,
    },
    /// Simple container panel (no layout logic, children are stacked).
    Panel {
        id: WidgetId,
        children: Vec<WidgetDesc>,
    },
    /// Tab view with multiple pages.
    TabView {
        id: WidgetId,
        tabs: Vec<TabDesc>,
        selected: i32,
    },
    /// Split view (collapsible pane + content).
    SplitView {
        id: WidgetId,
        pane: Box<WidgetDesc>,
        content: Box<WidgetDesc>,
        pane_open: bool,
        pane_length: f64,
    },
    /// Scrollable container.
    ScrollViewer {
        id: WidgetId,
        child: Box<WidgetDesc>,
        horizontal_scroll: bool,
        vertical_scroll: bool,
    },
    /// Grid layout with rows and columns.
    GridLayout {
        id: WidgetId,
        rows: Vec<GridLengthDesc>,
        columns: Vec<GridLengthDesc>,
        children: Vec<WidgetDesc>,
        row_spacing: f64,
        column_spacing: f64,
    },
    /// Menu bar (top-level menus with dropdown items).
    MenuBar { id: WidgetId, menus: Vec<MenuDesc> },
    /// Toolbar (command bar with buttons).
    ToolBar {
        id: WidgetId,
        items: Vec<ToolBarItemDesc>,
    },
    /// Status bar (bottom bar with text fields).
    StatusBar {
        id: WidgetId,
        fields: Vec<StatusFieldDesc>,
    },
    /// Modal dialog (ContentDialog).
    ContentDialog {
        id: WidgetId,
        title: String,
        content: String,
        primary_text: String,
        secondary_text: String,
        close_text: String,
        is_open: bool,
    },
    /// Color picker dialog. `is_open = false` → non occupa spazio nel
    /// layout (come il `ContentDialog`): altrimenti il picker "entra" nella
    /// finestra appena creato e spinge gli altri widget fuori schermo.
    ColorPicker {
        id: WidgetId,
        color: (u8, u8, u8, u8),
        is_open: bool,
    },
    /// Image widget.
    ///
    /// `width`/`height` opzionali: senza size esplicita un'`Image` con
    /// `Stretch::Uniform` dentro uno `StackPanel` desidera tutta la larghezza
    /// disponibile scalata (es. 468x468) e schiaccia i widget successivi
    /// fuori dalla finestra.
    Image {
        id: WidgetId,
        uri: String,
        stretch: ImageStretch,
        width: Option<f64>,
        height: Option<f64>,
    },
    /// Tree view (hierarchical list).
    TreeView {
        id: WidgetId,
        nodes: Vec<TreeNodeDesc>,
    },
    /// List box (flat list).
    ListBox {
        id: WidgetId,
        items: Vec<String>,
        selected: i32,
    },
    /// List control (table with columns).
    ListCtrl {
        id: WidgetId,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// Data grid (full-featured table with sorting, styling, selection).
    Grid {
        id: WidgetId,
        columns: Vec<GridColumnDesc>,
        rows: Vec<Vec<GridCellDesc>>,
        sort_column: Option<usize>,
        sort_ascending: bool,
        selected_row: Option<usize>,
        alternating_colors: bool,
        even_color: (u8, u8, u8),
        odd_color: (u8, u8, u8),
    },
    /// Check list box: lista piatta con una check-box per riga
    /// (omologa XAML della `CheckListBox` di AppKit).
    CheckListBox {
        id: WidgetId,
        items: Vec<String>,
        checked: Vec<bool>,
    },
    /// Hyperlink testuale con URL di navigazione
    /// (omologo XAML dell'`HyperlinkCtrl` di AppKit, `HyperlinkButton`).
    Hyperlink {
        id: WidgetId,
        label: String,
        url: String,
    },
    /// Spacer / empty element.
    Spacer,
}

/// Grid column descriptor.
#[derive(Debug, Clone)]
pub struct GridColumnDesc {
    pub title: String,
    pub width: f64,
    pub align: GridColumnAlign,
}

/// Grid column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridColumnAlign {
    Left,
    Center,
    Right,
}

/// Grid cell descriptor.
#[derive(Debug, Clone)]
pub enum GridCellDesc {
    Text(String),
    Empty,
    Progress { value: u32, max: u32 },
    Badge { text: String, kind: GridBadgeKind },
    Bool(bool),
}

/// Badge kind for grid cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridBadgeKind {
    Good,
    Warn,
    Bad,
    Info,
}

/// Tree node descriptor.
#[derive(Debug, Clone)]
pub struct TreeNodeDesc {
    pub text: String,
    pub expanded: bool,
    pub children: Vec<TreeNodeDesc>,
}

/// How an image is stretched to fill its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageStretch {
    None,
    Fill,
    Uniform,
    UniformToFill,
}

/// Menu descriptor (top-level menu with dropdown items).
#[derive(Debug, Clone)]
pub struct MenuDesc {
    pub title: String,
    pub items: Vec<MenuItemDesc>,
}

/// Menu item descriptor.
#[derive(Debug, Clone)]
pub enum MenuItemDesc {
    Item {
        text: String,
        enabled: bool,
    },
    Separator,
    SubMenu {
        text: String,
        children: Vec<MenuItemDesc>,
    },
}

/// Toolbar item descriptor.
#[derive(Debug, Clone)]
pub enum ToolBarItemDesc {
    Button { label: String },
    Toggle { label: String },
    Separator,
}

/// Status bar field descriptor.
#[derive(Debug, Clone)]
pub struct StatusFieldDesc {
    pub text: String,
    pub width: Option<f64>,
}

/// Tab page descriptor.
#[derive(Debug, Clone)]
pub struct TabDesc {
    pub header: String,
    pub content: WidgetDesc,
    pub closable: bool,
}

/// Grid length for layout rows/columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridLengthDesc {
    /// Absolute size in DIPs.
    Absolute(f64),
    /// Proportional (star) size.
    Star(f64),
    /// Automatic (content-sized).
    Auto,
}

impl WidgetDesc {
    /// Get the widget ID, if this descriptor has one.
    pub fn id(&self) -> Option<WidgetId> {
        match self {
            Self::TextBlock { id, .. }
            | Self::Button { id, .. }
            | Self::TextBox { id, .. }
            | Self::TextArea { id, .. }
            | Self::CheckBox { id, .. }
            | Self::RadioButton { id, .. }
            | Self::Slider { id, .. }
            | Self::ProgressBar { id, .. }
            | Self::ComboBox { id, .. }
            | Self::VStack { id, .. }
            | Self::HStack { id, .. }
            | Self::Panel { id, .. }
            | Self::TabView { id, .. }
            | Self::SplitView { id, .. }
            | Self::ScrollViewer { id, .. }
            | Self::GridLayout { id, .. }
            | Self::MenuBar { id, .. }
            | Self::ToolBar { id, .. }
            | Self::StatusBar { id, .. }
            | Self::ContentDialog { id, .. }
            | Self::ColorPicker { id, .. }
            | Self::Image { id, .. }
            | Self::TreeView { id, .. }
            | Self::ListBox { id, .. }
            | Self::ListCtrl { id, .. }
            | Self::Grid { id, .. }
            | Self::CheckListBox { id, .. }
            | Self::Hyperlink { id, .. } => Some(*id),
            Self::Spacer => None,
        }
    }
}

// ── Frame descriptor ─────────────────────────────────────────────────────────

/// Describes the top-level window and its widget tree.
///
/// Built imperatively via [`XamlFrameBuilder`](crate::platform::winui3_xaml::XamlFrameBuilder), then consumed by
/// [`XamlApp::run`](crate::platform::winui3_xaml::XamlApp::run) to produce the reactor element tree.
pub struct FrameDesc {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub children: Vec<WidgetDesc>,
    pub registry: SharedRegistry,
    pub centered: bool,
    pub visible: bool,
    /// Padding del container radice (tutti i lati, in px logici).
    pub padding: f64,
    /// Spaziatura verticale tra i figli diretti del frame (px logici).
    pub spacing: f64,
}

impl FrameDesc {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            width: 800.0,
            height: 600.0,
            children: Vec::new(),
            registry: new_shared_registry(),
            centered: false,
            visible: true,
            padding: 0.0,
            spacing: 0.0,
        }
    }
}

impl Default for FrameDesc {
    fn default() -> Self {
        Self::new()
    }
}

// ── Shared frame reference ───────────────────────────────────────────────────

/// Shared, mutable reference to a frame's descriptor.
///
/// Passed to widget constructors so they can append themselves
/// to the frame's child list. Uses `Rc<RefCell>` because the builder
/// phase is single-threaded; `XamlApp::run()` extracts `Send`-safe
/// clones before entering the reactor render closure.
pub type FrameRef = Rc<RefCell<FrameDesc>>;

/// Create a new shared frame descriptor.
pub fn new_frame_ref() -> FrameRef {
    Rc::new(RefCell::new(FrameDesc::new()))
}
