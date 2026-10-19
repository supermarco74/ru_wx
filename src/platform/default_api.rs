//! Default API Layer for Windows
//!
//! This module exposes the WinUI3 backend's type aliases so that user
//! code can opt into the WinUI3 backend by writing
//! `use ru_wx::platform::winui3::*`.
//!
//! **Important**: the `winui3` cargo feature does NOT change the
//! types re-exported at the crate root (`ru_wx::Frame`,
//! `ru_wx::Button`, …). The crate root always resolves to the
//! production-quality `window::frame::Frame` and `controls::button::Button`
//! implementation, so existing examples keep compiling regardless
//! of the feature flag.
//!
//! The WinUI3 backend in `platform::winui3` is a feature-gated backend
//! that, over time, has grown a much wider surface — including new
//! widgets (`TreeCtrl`, `Hyperlink`, `ProgressBar`, `StaticBox`, …),
//! a proper event-dispatch layer (`events::register_command`), and
//! timers / accelerators / dialogs. Reach for it explicitly via
//! `ru_wx::platform::winui3::*` (or `ru_wx::platform::default_api::*`)
//! when you want to exercise them; do NOT enable the `winui3` feature
//! and then write `use ru_wx::Button`, because the two `Button` types
//! are distinct.
//!
//! For the Win32 classic API used by the rest of the library, the
//! implementation lives in `platform::win32` and is automatically
//! pulled in by the controls / windows it backs.

// Re-export the WinUI3 backend types so that downstream code can
// `use ru_wx::platform::winui3::*` (or `use ru_wx::platform::default_api::*`)
// to opt into the experimental backend.
#[cfg(target_os = "windows")]
pub use super::winui3::App;
#[cfg(target_os = "windows")]
pub use super::winui3::BoxSizer;
#[cfg(target_os = "windows")]
pub use super::winui3::Button;
#[cfg(target_os = "windows")]
pub use super::winui3::CheckBox;
#[cfg(target_os = "windows")]
pub use super::winui3::Choice;
#[cfg(target_os = "windows")]
pub use super::winui3::ComboBox;
#[cfg(target_os = "windows")]
pub use super::winui3::Frame;
#[cfg(target_os = "windows")]
pub use super::winui3::FrameBuilder;
#[cfg(target_os = "windows")]
pub use super::winui3::Gauge;
#[cfg(target_os = "windows")]
pub use super::winui3::ListBox;
#[cfg(target_os = "windows")]
pub use super::winui3::ListBoxUWP;
#[cfg(target_os = "windows")]
pub use super::winui3::ListView;
#[cfg(target_os = "windows")]
pub use super::winui3::Menu;
#[cfg(target_os = "windows")]
pub use super::winui3::MenuBar;
#[cfg(target_os = "windows")]
pub use super::winui3::Orientation;
#[cfg(target_os = "windows")]
pub use super::winui3::Panel;
#[cfg(target_os = "windows")]
pub use super::winui3::PopupMenu;
#[cfg(target_os = "windows")]
pub use super::winui3::RadioBox;
#[cfg(target_os = "windows")]
pub use super::winui3::RadioButton;
#[cfg(target_os = "windows")]
pub use super::winui3::Slider;
#[cfg(target_os = "windows")]
pub use super::winui3::StaticText;
#[cfg(target_os = "windows")]
pub use super::winui3::StatusBar;
#[cfg(target_os = "windows")]
pub use super::winui3::Tab;
#[cfg(target_os = "windows")]
pub use super::winui3::TextCtrl;
// New surface since the previous "placeholder" label.
#[cfg(target_os = "windows")]
pub use super::winui3::dialogs::{
    file_pick_open as pick_open_file, file_pick_save as pick_save_file,
    message_box as show_message_box, pick_color as pick_colour, MessageBoxButton, MessageBoxIcon,
    MessageBoxResponse,
};
#[cfg(target_os = "windows")]
pub use super::winui3::AcceleratorKey;
#[cfg(target_os = "windows")]
pub use super::winui3::AcceleratorTable;
#[cfg(target_os = "windows")]
pub use super::winui3::BitmapButton;
#[cfg(target_os = "windows")]
pub use super::winui3::Hyperlink;
#[cfg(target_os = "windows")]
pub use super::winui3::IconTray;
#[cfg(target_os = "windows")]
pub use super::winui3::ProgressBar;
#[cfg(target_os = "windows")]
pub use super::winui3::SearchCtrl;
#[cfg(target_os = "windows")]
pub use super::winui3::SpinCtrl;
#[cfg(target_os = "windows")]
pub use super::winui3::Splitter;
#[cfg(target_os = "windows")]
pub use super::winui3::StaticBox;
#[cfg(target_os = "windows")]
pub use super::winui3::StaticLine;
#[cfg(target_os = "windows")]
pub use super::winui3::Timer;
#[cfg(target_os = "windows")]
pub use super::winui3::ToolBar;
#[cfg(target_os = "windows")]
pub use super::winui3::TreeCtrl;
#[cfg(target_os = "windows")]
pub use super::winui3::TreeItemId;
#[cfg(target_os = "windows")]
pub use super::winui3::{query_dpi, scale_for_dpi, scale_size as dpi_scale_size, DEFAULT_DPI};

// Native Grid widget (SysListView32 + custom draw + ImageList).
#[cfg(target_os = "windows")]
pub use super::winui3::grid::{
    CellCoord as GridCellCoord, FunctionGridTable as GridFunctionGridTable, Grid as GridWidget,
    GridCellValue as GridCell, GridIcons, GridStringTable, GridTable,
};
