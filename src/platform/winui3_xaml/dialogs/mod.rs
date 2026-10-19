//! Dialog widgets for the WinUI3/XAML bridge.
//!
//! | Module           | Widget              | Reactor equivalent          |
//! |------------------|---------------------|-----------------------------|
//! | `message_dialog` | `XamlMessageDialog` | `reactor::ContentDialog`    |
//! | `color_dialog`   | `XamlColorDialog`   | `reactor::ColorPicker`      |
//! | `file_dialog`    | `XamlFileDialog`    | WinRT FileOpenPicker (stub) |
//! | `font_dialog`    | `XamlFontDialog`    | Custom (stub)               |
//! | `dir_dialog`     | `XamlDirDialog`     | WinRT FolderPicker (stub)   |

pub mod color_dialog;
pub mod dir_dialog;
pub mod file_dialog;
pub mod font_dialog;
pub mod message_dialog;

pub use color_dialog::XamlColorDialog;
pub use dir_dialog::XamlDirDialog;
pub use file_dialog::XamlFileDialog;
pub use font_dialog::XamlFontDialog;
pub use message_dialog::XamlMessageDialog;
