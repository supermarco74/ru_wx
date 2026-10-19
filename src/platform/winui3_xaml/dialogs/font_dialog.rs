//! XamlFontDialog — font picker dialog (stub).
//!
//! WinUI3 has no built-in font picker. A custom implementation would need
//! a font list, size selector, and style toggles. This is a placeholder.

use super::super::window::XamlFrame;

/// Font picker dialog (stub — shows a message instead).
pub struct XamlFontDialog;

impl XamlFontDialog {
    /// Show a font picker dialog. Returns `None` (not yet implemented).
    pub fn show(_frame: &XamlFrame) -> Option<String> {
        eprintln!("[STUB] FontDialog — requires custom font picker implementation");
        None
    }
}
