//! WinUI3 Backend - Pure Rust Win32 Native Controls
//!
//! This backend provides a wxWidgets-like API using Win32 native controls
//! (`CreateWindowExW`, `ToolbarWindow32`, `SysTreeView32`, `msctls_progress32`,
//! …) with a modern Rust interface based on closures for handlers.
//!
//! # Module structure
//!
//! | File                  | Contents                                          |
//! |-----------------------|---------------------------------------------------|
//! | `app`                 | `App`, `Frame`, `Button`, `StaticText`, `TextCtrl`, `CheckBox`, `RadioButton`, `RadioBox`, `Panel`, `Choice`, `ListBox`, `ListView`, `ListBoxUWP`, `Slider`, `Gauge`, `ComboBox`, `StatusBar`, `Tab` |
//! | `layout`              | `BoxSizer`, `Orientation`, `SizerFlag`            |
//! | `menu`                | `Menu`, `MenuBar`, `PopupMenu`, `MenuItem`        |
//! | `grid`                | `Grid` (SysListView32-based report grid)          |
//! | `tree`                | `TreeCtrl` (SysTreeView32-based tree)             |
//! | `widgets`             | `Hyperlink`, `StaticLine`, `StaticBox`, `BitmapButton`, `ProgressBar`, `SpinCtrl`, `Splitter`, `SearchCtrl` |
//! | `toolbar`             | `ToolBar`, `ToolBarItem`                          |
//! | `events`              | shared callback dispatch + per-window handler map |
//! | `timer`               | `Timer` (SetTimer / KillTimer wrapper)            |
//! | `accelerator`         | `AcceleratorKey`, `AcceleratorTable`              |
//! | `dialogs`             | `message_box`, `file_pick_open`, `pick_color`, …  |
//! | `dpi`                 | Per-Monitor V2 helpers                            |
//! | `icon`                | SVG → HICON, frame icon, `IconTray`               |
//! | `window_trait`        | `crate::core::widget::Window` impl for `Frame`    |
//! | `window`              | Modern Window API (windows-rs style: `WindowBuilder`, `run`, `run_with`, `pump`) |

pub mod app;
pub mod grid;
pub mod layout;
pub mod menu;

pub mod accelerator;
pub mod common;
pub mod dialogs;
pub mod dpi;
pub mod events;
pub mod icon;
pub mod palette;
pub mod subclass;
pub mod timer;
pub mod toolbar;
pub mod tree;
pub mod widgets;
pub mod window;
pub mod window_trait;

pub use accelerator::{AcceleratorKey, AcceleratorTable};
pub use app::*;
pub use common::WidgetCommon;
pub use dialogs::{
    file_pick_open, file_pick_save, message_box, pick_color, Color as DialogColor,
    MessageBoxButton, MessageBoxIcon, MessageBoxResponse,
};
pub use dpi::{query as query_dpi, scale as scale_for_dpi, scale_size, DEFAULT_DPI};
pub use events::{Handler as EventHandler, Handlers, MouseButton, WidgetEvent};
pub use grid::*;
pub use icon::{set_frame_icon, svg_to_hicon, IconTray};
pub use layout::*;
pub use menu::*;
pub use palette::{
    clear_brush_cache as clear_color_cache, color_ref, is_background_set as has_background, preset,
    rgb, set_background_color, set_foreground_color,
};
pub use subclass::{
    clear_foreground as clear_widget_foreground, set_foreground as set_widget_foreground,
    HYPERLINK_BLUE,
};
pub use timer::Timer;
pub use toolbar::ToolBar;
pub use tree::{TreeCtrl, TreeItemId};
pub use widgets::{
    BitmapButton, Hyperlink, ProgressBar, SearchCtrl, SpinCtrl, Splitter, StaticBox, StaticLine,
};
// Modern Window API (windows-rs style)
pub use window::{
    pump, pump_with_timeout, quit, run, run_with, run_with_simple, wm, ws, Result as WindowResult,
    Win32Error, Window, WindowBuilder,
};

// Re-export the `Window` trait impl from `window_trait` so that
// downstream code can call `Frame::hwnd()` in a cross-platform way.
pub use crate::core::widget::Window as CoreWindow;

/// Module version metadata — useful for `Cargo` build scripts that
/// need to know whether the winui3 backend bumped between commits.
pub const WINUI3_VERSION: &str = env!("CARGO_PKG_VERSION");
