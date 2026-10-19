//! XamlDirDialog — directory picker dialog (stub).
//!
//! WinRT `FolderPicker` is an async API requiring `IAsyncOperation` bridging.
//! This is a placeholder.

use super::super::window::XamlFrame;

/// Directory picker dialog (stub — shows a message instead).
pub struct XamlDirDialog;

impl XamlDirDialog {
    /// Show a directory picker dialog. Returns `None` (not yet implemented).
    pub fn show(_frame: &XamlFrame) -> Option<String> {
        eprintln!("[STUB] DirDialog — requires WinRT FolderPicker async bridging");
        None
    }
}
