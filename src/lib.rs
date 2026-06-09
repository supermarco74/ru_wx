//! ru_wx - A cross-platform native GUI library for Rust
//!
//! This library provides a wxWidgets-like API using native platform controls:
//! - Windows: Win32 API (HWND-based controls)
//! - macOS: AppKit (planned)
//! - Linux: GTK (planned)
//!
//! # Example
//! ```no_run
//! use ru_wx::*;
//!
//! let app = App::new();
//! let frame = Frame::builder()
//!     .with_title("Hello")
//!     .with_size(400, 300)
//!     .build();
//!
//! let button = Button::new(&frame, "Click me!");
//! button.on_click(&frame, || {
//!     println!("Clicked!");
//! });
//!
//! app.run(frame);
//! ```
//!
//! # Internal lint policy
//!
//! The crate intentionally allows the
//! `clippy::missing_docs_in_private_items` lint at the crate
//! root. Every public item in the public API is documented
//! (see the module-level rustdoc on each `src/*.rs` file and
//! the `MIGRATION_STATUS.md` index), but the `pub(crate)` and
//! private items (e.g. the fields of every `*Inner` struct,
//! the Win32 message constants, the helper closures used by
//! `WM_NOTIFY` dispatch) are not, by design. Documenting them
//! would create documentation that is not reachable from the
//! public rustdoc output and would be a maintenance burden for
//! no user-facing benefit. The lint is therefore explicitly
//! silenced at the crate root; individual modules that contain
//! user-facing surface (the `log` submodules, `tooltip::imp`,
//! etc.) carry their own `//!` or `///` rustdoc regardless of
//! the lint suppression.

#![allow(clippy::missing_docs_in_private_items)]

pub mod accelerator;
pub mod animation;
pub mod animation_ctrl;
pub mod app;
pub mod art_provider;
pub mod aui_tool_bar;
pub mod bitmap;
pub mod bitmap_bundle;
pub mod bitmap_button;
pub mod book;
pub mod brush;
pub mod busy_info;
pub mod button;
pub mod check_list_box;
pub mod checkbox;
pub mod choice;
pub mod color_dialog;
pub mod colour_picker_ctrl;
pub mod combo_box;
pub mod date_picker_ctrl;
pub mod date_picker_dialog;
pub mod dc;
pub mod dialog;
pub mod dir_dialog;
pub mod dpi;
pub mod drop_target;
pub mod ole_dnd;
pub mod file_dialog;
pub mod find_replace_dialog;
pub mod font;
pub mod font_dialog;
pub mod frame;
pub mod frame_extras;
pub mod gauge;
pub mod geometry;
pub mod gl_canvas;
pub mod grid;
pub mod grid_sizer;
pub mod icon;
pub mod icon_tray;
pub mod image;
pub mod image_list;
pub mod list_box;
pub mod list_ctrl;
pub mod log;
pub mod media_ctrl;
pub mod menu;
pub mod mdi;
pub use mdi::{MDIChildFrame, MDIParentFrame};
pub mod message_box;
pub mod message_dialog;
pub mod panel;
pub mod pen;
pub mod platform;
pub mod popup_menu;
pub mod progress_dialog;
pub mod property_grid;
pub mod property_sheet_dialog;
pub mod radio_box;
pub mod radio_button;
pub mod scrolled_window;
pub mod scroll_bar;
pub mod single_choice_dialog;
pub mod sizer;
pub mod slider;
pub mod spin_button;
pub mod spin_ctrl;
pub mod spin_ctrl_double;
pub mod splitter_window;
pub mod static_bitmap;
pub mod static_box;
pub mod static_line;
pub mod static_text;
pub mod status_bar;
pub mod symbol_picker_dialog;
pub mod tab;
pub mod text_ctrl;
pub mod text_entry_dialog;
pub mod timer;
pub mod toggle_button;
pub mod tool_bar;
pub mod tooltip;
pub mod top_level_window;
pub mod tree_ctrl;
pub mod widget;
pub mod wizard;

pub use accelerator::{Accelerator, Modifiers, ParseError, VirtualKey};
pub use animation::{Animation, AnimationFrame};
pub use animation_ctrl::AnimationCtrl;
pub use app::App;
pub use art_provider::{ArtClient, ArtId, ArtProvider};
pub use aui_tool_bar::{AuiDockSide, AuiToolBar};
pub use bitmap::Bitmap;
pub use bitmap_bundle::{BitmapBundle, RawBitmap};
pub use bitmap_button::BitmapButton;
pub use book::{Choicebook, Listbook, Toolbook, Treebook};
pub use brush::{Brush, BrushStyle};
pub use busy_info::BusyInfo;
pub use button::Button;
pub use check_list_box::CheckListBox;
pub use checkbox::CheckBox;
pub use choice::Choice;
pub use color_dialog::ColorDialog as ColourDialog;
pub use colour_picker_ctrl::ColourPickerCtrl;
pub use combo_box::ComboBox;
pub use combo_box::BitmapComboBox;
pub use date_picker_ctrl::{Date, DateFormat, DatePickerCtrl};
pub use date_picker_dialog::{DateDialogFormat, DatePickerDialog};
pub use dc::{BackgroundMode, ClientDC, Dc, MemoryDC, PaintDC, WindowDC};
pub use dialog::Dialog;
pub use dir_dialog::DirDialog;
pub use dpi::{
    get_dpi_for_point, get_dpi_for_window, get_process_dpi_awareness, get_system_dpi,
    set_process_dpi_awareness, Dpi, DpiAwareness, SYSTEM_DPI,
};
pub use drop_target::DroppedFiles;
pub use ole_dnd::{OleDropEffect, OleDropError, OleDroppedData, OleDropPosition};
#[cfg(target_os = "windows")]
pub use ole_dnd::OleDropTarget;
pub use file_dialog::{FileDialog, FileDialogStyle};
pub use find_replace_dialog::{FindReplaceDialog, FindReplaceEvent};
pub use pen::{Pen, PenStyle};
pub use font::{Font, FontDesc};
pub use font_dialog::FontDialog;
pub use frame::{Frame, FrameBuilder};
pub use frame_extras::{MiniFrame, SplashScreen, TipWindow};
pub use gauge::Gauge;
pub use geometry::{Colour, Rect};
#[cfg(target_os = "windows")]
pub use gl_canvas::gl11;
pub use gl_canvas::GLCanvas;
pub use grid::{BadgeKind, BarStyle, Cell, ColumnAlign, Grid, GridDateFormat, NumberFormat, PriorityKind};
pub use grid_sizer::{FlexGridSizer, GridSizer};
pub use icon_tray::{BalloonIcon, IconTray};
#[cfg(target_os = "windows")]
pub use icon::svg_bytes_to_hicon;
pub use image::{Image, ImageError, Rgba};
pub use image_list::ImageList;
pub use list_box::ListBox;
pub use list_ctrl::{ListCtrl, ListCtrlStyle, ListItem};
pub use media_ctrl::{MediaCtrl, MediaState};
pub use menu::{Menu, MenuBar, MenuItem, MenuItemKind};
pub use message_box::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxStyle};
pub use message_dialog::{MessageDialog, MessageDialogIcon, MessageDialogStyle};
pub use panel::Panel;
pub use popup_menu::PopupMenu;
pub use progress_dialog::ProgressDialog;
pub use property_grid::{Property, PropertyGrid, PropertyValue};
pub use property_sheet_dialog::{PropertySheetDialog, PropertySheetDialogResult};
pub use radio_box::RadioBox;
pub use radio_button::RadioButton;
pub use sizer::{BoxSizer, Orientation};
pub use scrolled_window::ScrolledWindow;
pub use scroll_bar::{ScrollBar, ScrollBarOrientation, ScrollEvent as ScrollBarEvent};
pub use scrolled_window::ScrollEvent as ScrolledWindowScrollEvent;
pub use single_choice_dialog::{ChoiceResult, MultiChoiceDialog, SingleChoiceDialog};
pub use slider::Slider;
pub use spin_button::SpinButton;
pub use spin_ctrl::SpinCtrl;
pub use spin_ctrl_double::SpinCtrlDouble;
pub use splitter_window::{SashEvent, SplitterOrientation, SplitterWindow};
pub use static_bitmap::StaticBitmap;
pub use static_box::StaticBox;
pub use static_line::{StaticLine, StaticLineOrientation};
pub use static_text::StaticText;
pub use status_bar::StatusBar;
pub use symbol_picker_dialog::SymbolPickerDialog;
pub use tab::Tab;
pub use text_ctrl::TextCtrl;
pub use text_entry_dialog::{NumberEntryDialog, PasswordEntryDialog, TextEntryDialog};
pub use timer::Timer;
pub use toggle_button::ToggleButton;
pub use tool_bar::ToolBar;
pub use tooltip::ToolTip;
pub use top_level_window::{CentreDirection, FullScreenStyle, TopLevelWindow, UserAttentionFlags, WindowCornerPreference};
pub use tree_ctrl::{TreeCtrl, TreeItem};
#[cfg(target_os = "windows")]
pub use widget::Window;
pub use widget::{Widget, WidgetRef};
pub use wizard::{Wizard, WizardPage, WizardResult};

/// Convenient re-exports of the most commonly used items.
///
/// See [`prelude`](self) for details.
pub mod prelude;
