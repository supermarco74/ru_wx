//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Convenient re-exports of the most commonly used `ru_wx` items.
//!
//! `ru_wx` exposes a large surface (≈ 60 modules grouped into ten
//! domain folders: `core`, `window`, `controls`, `containers`,
//! `chrome`, `dialogs`, `dc`, `adv`, `dnd`, `platform`); most user
//! code only needs a handful of types. This module gathers the
//! typical "import-and-go" set so a single line brings the whole
//! working set into scope:
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
pub use crate::core::widget::Window;
pub use crate::core::widget::{Widget, WidgetRef};

// --- Application & top-level windows ----------------------------------
pub use crate::core::app::App;
pub use crate::dialogs::color_dialog::ColorDialog as ColourDialog;
pub use crate::window::dialog::Dialog;
pub use crate::dialogs::dir_dialog::DirDialog;
pub use crate::dialogs::file_dialog::{FileDialog, FileDialogStyle};
pub use crate::dialogs::find_replace_dialog::{FindReplaceDialog, FindReplaceEvent};
pub use crate::dialogs::font_dialog::FontDialog;
pub use crate::window::frame::{Frame, FrameBuilder};
pub use crate::dialogs::message_box::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxStyle};
pub use crate::dialogs::message_dialog::{MessageDialog, MessageDialogIcon, MessageDialogStyle};
pub use crate::window::panel::Panel;
pub use crate::dialogs::progress_dialog::ProgressDialog;
pub use crate::adv::property_grid::{Property, PropertyGrid, PropertyValue};
pub use crate::dialogs::property_sheet_dialog::{PropertySheetDialog, PropertySheetDialogResult};
pub use crate::dialogs::single_choice_dialog::{ChoiceResult, MultiChoiceDialog, SingleChoiceDialog};
pub use crate::dialogs::symbol_picker_dialog::SymbolPickerDialog;
pub use crate::dialogs::text_entry_dialog::{NumberEntryDialog, PasswordEntryDialog, TextEntryDialog};
pub use crate::window::top_level_window::{
    CentreDirection, FullScreenStyle, TopLevelWindow, UserAttentionFlags, WindowCornerPreference,
};
pub use crate::adv::wizard::{Wizard, WizardPage, WizardResult};

// --- Common containers ------------------------------------------------
pub use crate::chrome::aui_tool_bar::{AuiDockSide, AuiToolBar};
pub use crate::core::busy_info::BusyInfo;
pub use crate::window::menu::{Menu, MenuBar, MenuItem, MenuItemKind};
pub use crate::window::popup_menu::PopupMenu;
pub use crate::containers::scrolled_window::ScrolledWindow;
pub use crate::containers::scroll_bar::{ScrollBar, ScrollBarOrientation};
pub use crate::containers::splitter_window::{SashEvent, SplitterOrientation, SplitterWindow};
pub use crate::chrome::status_bar::StatusBar;
pub use crate::containers::tab::Tab;
pub use crate::chrome::tool_bar::ToolBar;

// --- Input controls ---------------------------------------------------
pub use crate::controls::bitmap_button::BitmapButton;
pub use crate::controls::button::Button;
pub use crate::controls::check_list_box::CheckListBox;
pub use crate::controls::checkbox::CheckBox;
pub use crate::controls::choice::Choice;
pub use crate::controls::colour_picker_ctrl::ColourPickerCtrl;
pub use crate::controls::combo_box::BitmapComboBox;
pub use crate::controls::combo_box::ComboBox;
pub use crate::controls::date_picker_ctrl::{Date, DateFormat, DatePickerCtrl};
pub use crate::dialogs::date_picker_dialog::{DateDialogFormat, DatePickerDialog};
pub use crate::dc::dc::{BackgroundMode, Dc, MemoryDC, PaintDC, WindowDC};
pub use crate::controls::gauge::Gauge;
pub use crate::controls::list_box::ListBox;
pub use crate::controls::list_ctrl::{CacheHint, ListCtrl, ListCtrlStyle, ListItem};
pub use crate::controls::radio_box::RadioBox;
pub use crate::controls::radio_button::RadioButton;
pub use crate::controls::slider::Slider;
pub use crate::controls::spin_button::SpinButton;
pub use crate::controls::spin_ctrl::SpinCtrl;
pub use crate::controls::spin_ctrl_double::SpinCtrlDouble;
pub use crate::controls::static_bitmap::StaticBitmap;
pub use crate::controls::static_box::StaticBox;
pub use crate::controls::static_line::{StaticLine, StaticLineOrientation};
pub use crate::controls::static_text::StaticText;
pub use crate::controls::text_ctrl::TextCtrl;
pub use crate::controls::toggle_button::ToggleButton;
pub use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};

// --- Geometry & layout ------------------------------------------------
pub use crate::core::geometry::{Colour, Rect};
pub use crate::containers::grid::{Cell, Grid};
pub use crate::containers::grid_sizer::{FlexGridSizer, GridSizer};
pub use crate::containers::sizer::{BoxSizer, Orientation};

// --- Image / icon helpers --------------------------------------------
pub use crate::adv::animation::{Animation, AnimationFrame};
pub use crate::adv::animation_ctrl::AnimationCtrl;
pub use crate::dc::bitmap::Bitmap;
pub use crate::dc::bitmap_bundle::{BitmapBundle, RawBitmap};
pub use crate::dc::brush::{Brush, BrushStyle};
pub use crate::dc::gl_canvas::GLCanvas;
pub use crate::chrome::icon_tray::{BalloonIcon, IconTray};
pub use crate::dc::image::{Image, ImageError, Rgba};
pub use crate::dc::image_list::ImageList;
pub use crate::adv::media_ctrl::{MediaCtrl, MediaState};
pub use crate::dc::pen::{Pen, PenStyle};

// --- Misc helpers -----------------------------------------------------
pub use crate::core::accelerator::{Accelerator, Modifiers, VirtualKey};
pub use crate::dc::art_provider::{ArtClient, ArtId, ArtProvider};
pub use crate::core::dpi::{
    get_dpi_for_point, get_dpi_for_window, get_system_dpi, Dpi, DpiAwareness, SYSTEM_DPI,
};
pub use crate::dnd::drop_target::DroppedFiles;
pub use crate::dnd::ole_dnd::{
    DragContinueResult, OleDragData, OleDragError, OleDragSourceCallbacks,
    OleDropEffect, OleDropError, OleDroppedData, OleDropPosition,
};
#[cfg(target_os = "windows")]
pub use crate::dnd::ole_dnd::{OleDragSource, OleDropTarget};
pub use crate::core::font::{Font, FontDesc};
pub use crate::core::timer::Timer;
pub use crate::core::tooltip::ToolTip;
