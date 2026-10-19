//! XamlFileDialog — file open/save dialog (stub).
//!
//! WinRT `FileOpenPicker` / `FileSavePicker` are async APIs that require
//! `IAsyncOperation` bridging. This is a placeholder that shows a message
//! dialog instead. Full implementation requires WinRT async bridging.

use super::super::window::XamlFrame;

/// File open/save dialog (stub — shows a message instead).
pub struct XamlFileDialog;

impl XamlFileDialog {
    /// Show an open file dialog. Returns `None` (not yet implemented).
    pub fn open(_frame: &XamlFrame, _title: &str, _filter: &str) -> Option<String> {
        eprintln!("[STUB] FileDialog::open — requires WinRT FileOpenPicker async bridging");
        None
    }

    /// Show a save file dialog. Returns `None` (not yet implemented).
    pub fn save(_frame: &XamlFrame, _title: &str, _filter: &str) -> Option<String> {
        eprintln!("[STUB] FileDialog::save — requires WinRT FileSavePicker async bridging");
        None
    }
}
