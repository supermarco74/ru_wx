//! Convenient re-exports of the most commonly used `ru_wx` items.
//!
//! `ru_wx` exposes a large surface (≈ 45 modules); most user code only
//! needs a handful of types. This module gathers the typical
//! "import-and-go" set so a single line brings the whole working set
//! into scope:
//!
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let app = App::new();
//! let frame = Frame::builder().with_title("Hello").with_size(400, 300).build();
//! let button = Button::new(&frame, "Click me!");
//! button.on_click(&frame, || println!("Clicked!"));
//! app.run(frame);
//! ```
//!
//! Only items that are part of the documented public API and that are
//! useful in the typical "build a window, add some controls, run the
//! loop" path are re-exported here. Lower-level items (the `log` and
//! `platform` modules, raw `BitmapBundle` / `RawBitmap`, `ArtProvider`,
//! the `Window` trait on Windows, etc.) are still reachable at
//! `ru_wx::module_name` for users who need them.

#[cfg(target_os = "windows")]
pub use crate::widget::Window;
pub use crate::widget::{Widget, WidgetRef};

// --- Application & top-level windows ----------------------------------
pub use crate::app::App;
pub use crate::color_dialog::ColorDialog as ColourDialog;
pub use crate::dialog::Dialog;
pub use crate::dir_dialog::DirDialog;
pub use crate::file_dialog::{FileDialog, FileDialogStyle};
pub use crate::find_replace_dialog::{FindReplaceDialog, FindReplaceEvent};
pub use crate::font_dialog::FontDialog;
pub use crate::frame::{Frame, FrameBuilder};
pub use crate::message_box::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxStyle};
pub use crate::message_dialog::{MessageDialog, MessageDialogIcon, MessageDialogStyle};
pub use crate::panel::Panel;
pub use crate::progress_dialog::ProgressDialog;
pub use crate::property_grid::{Property, PropertyGrid, PropertyValue};
pub use crate::property_sheet_dialog::{PropertySheetDialog, PropertySheetDialogResult};
pub use crate::single_choice_dialog::{ChoiceResult, MultiChoiceDialog, SingleChoiceDialog};
pub use crate::symbol_picker_dialog::SymbolPickerDialog;
pub use crate::text_entry_dialog::{NumberEntryDialog, PasswordEntryDialog, TextEntryDialog};
pub use crate::top_level_window::{
    CentreDirection, FullScreenStyle, TopLevelWindow, UserAttentionFlags, WindowCornerPreference,
};
pub use crate::wizard::{Wizard, WizardPage, WizardResult};

// --- Common containers ------------------------------------------------
pub use crate::aui_tool_bar::{AuiDockSide, AuiToolBar};
pub use crate::busy_info::BusyInfo;
pub use crate::menu::{Menu, MenuBar, MenuItem, MenuItemKind};
pub use crate::popup_menu::PopupMenu;
pub use crate::scrolled_window::ScrolledWindow;
pub use crate::scroll_bar::{ScrollBar, ScrollBarOrientation};
pub use crate::splitter_window::{SashEvent, SplitterOrientation, SplitterWindow};
pub use crate::status_bar::StatusBar;
pub use crate::tab::Tab;
pub use crate::tool_bar::ToolBar;

// --- Input controls ---------------------------------------------------
pub use crate::bitmap_button::BitmapButton;
pub use crate::button::Button;
pub use crate::check_list_box::CheckListBox;
pub use crate::checkbox::CheckBox;
pub use crate::choice::Choice;
pub use crate::colour_picker_ctrl::ColourPickerCtrl;
pub use crate::combo_box::BitmapComboBox;
pub use crate::combo_box::ComboBox;
pub use crate::date_picker_ctrl::{Date, DateFormat, DatePickerCtrl};
pub use crate::date_picker_dialog::{DateDialogFormat, DatePickerDialog};
pub use crate::dc::{BackgroundMode, Dc, MemoryDC, PaintDC, WindowDC};
pub use crate::gauge::Gauge;
pub use crate::list_box::ListBox;
pub use crate::list_ctrl::{CacheHint, ListCtrl, ListCtrlStyle, ListItem};
pub use crate::radio_box::RadioBox;
pub use crate::radio_button::RadioButton;
pub use crate::slider::Slider;
pub use crate::spin_button::SpinButton;
pub use crate::spin_ctrl::SpinCtrl;
pub use crate::spin_ctrl_double::SpinCtrlDouble;
pub use crate::static_bitmap::StaticBitmap;
pub use crate::static_box::StaticBox;
pub use crate::static_line::{StaticLine, StaticLineOrientation};
pub use crate::static_text::StaticText;
pub use crate::text_ctrl::TextCtrl;
pub use crate::toggle_button::ToggleButton;
pub use crate::tree_ctrl::{TreeCtrl, TreeItem};

// --- Geometry & layout ------------------------------------------------
pub use crate::geometry::{Colour, Rect};
pub use crate::grid::{Cell, Grid};
pub use crate::grid_sizer::{FlexGridSizer, GridSizer};
pub use crate::sizer::{BoxSizer, Orientation};

// --- Image / icon helpers --------------------------------------------
pub use crate::animation::{Animation, AnimationFrame};
pub use crate::animation_ctrl::AnimationCtrl;
pub use crate::bitmap::Bitmap;
pub use crate::bitmap_bundle::{BitmapBundle, RawBitmap};
pub use crate::brush::{Brush, BrushStyle};
pub use crate::gl_canvas::GLCanvas;
pub use crate::icon_tray::{BalloonIcon, IconTray};
pub use crate::image::{Image, ImageError, Rgba};
pub use crate::image_list::ImageList;
pub use crate::media_ctrl::{MediaCtrl, MediaState};
pub use crate::pen::{Pen, PenStyle};

// --- Misc helpers -----------------------------------------------------
pub use crate::accelerator::{Accelerator, Modifiers, VirtualKey};
pub use crate::art_provider::{ArtClient, ArtId, ArtProvider};
pub use crate::dpi::{
    get_dpi_for_point, get_dpi_for_window, get_system_dpi, Dpi, DpiAwareness, SYSTEM_DPI,
};
pub use crate::drop_target::DroppedFiles;
pub use crate::ole_dnd::{
    DragContinueResult, OleDragData, OleDragError, OleDragSourceCallbacks,
    OleDropEffect, OleDropError, OleDroppedData, OleDropPosition,
};
#[cfg(target_os = "windows")]
pub use crate::ole_dnd::{OleDragSource, OleDropTarget};
pub use crate::font::{Font, FontDesc};
pub use crate::timer::Timer;
pub use crate::tooltip::ToolTip;
