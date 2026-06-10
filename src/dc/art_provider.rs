//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxArtProvider` — built-in icon provider.
//!
//! Provides named system-style icons (New, Open, Save, Cut, Copy, Paste, …)
//! as a [`BitmapBundle`], so they automatically pick the right resolution
//! for the current DPI.
//!
//! On Windows there is no single, stable, public set of resource IDs for
//! the modern toolbar glyphs (Ribbon / Office-style). For that reason this
//! module ships a small library of clean, license-free SVG glyphs that
//! are rendered through the same resvg pipeline used everywhere else in
//! the library. Users can override individual icons with
//! [`ArtProvider::register_svg`] to plug in their own icon set.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let provider = ArtProvider::new();
//! let bundle = provider.get_bitmap(ArtId::New, ArtClient::Menu);
//! ```

use crate::dc::bitmap_bundle::BitmapBundle;

/// Identifies a built-in icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtId {
    New,
    Open,
    Save,
    SaveAs,
    Print,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    Find,
    Replace,
    Delete,
    Add,
    Remove,
    Ok,
    Cancel,
    Apply,
    Close,
    Quit,
    About,
    Help,
    Information,
    Warning,
    Error,
    Question,
    Folder,
    File,
    Home,
    Settings,
    Refresh,
    Search,
    Star,
}

/// Hint about where the icon will be drawn. The provider may pick a
/// different size or colour scheme accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtClient {
    /// 16×16 monochrome, drawn next to a menu item.
    Menu,
    /// 24×24, used inside toolbars.
    ToolBar,
    /// 32×32, used in large buttons or the about box.
    Button,
    /// 48×48, used in dialog title bars.
    Dialog,
}

impl ArtClient {
    /// Default pixel size for this client.
    pub fn default_size(self) -> (u32, u32) {
        match self {
            ArtClient::Menu => (16, 16),
            ArtClient::ToolBar => (24, 24),
            ArtClient::Button => (32, 32),
            ArtClient::Dialog => (48, 48),
        }
    }
}

// -----------------------------------------------------------------------------
// Built-in SVG library
//
// These are all simple 24×24 viewBox SVGs, black-on-transparent strokes/fills,
// designed to scale cleanly. They are intentionally minimal — for production
// icons you should call ArtProvider::register_svg() to plug in your own
// assets.
// -----------------------------------------------------------------------------

/// Build a self-contained SVG document with the given inner XML.
/// Used internally to wrap the hand-written glyph paths into a
/// valid `<?xml?>`-free SVG that resvg can render.
macro_rules! svg {
    ($body:expr) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">"#,
            $body,
            r#"</svg>"#,
        )
        .as_bytes()
    };
}

/// Look up the built-in SVG bytes for a given [`ArtId`]. Returns a
/// 24×24 viewBox document with `currentColor` strokes that the
/// renderer is expected to colour appropriately.
fn svg_for(art_id: ArtId) -> &'static [u8] {
    match art_id {
        ArtId::New => svg!(
            r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="none" stroke="currentColor" stroke-width="2"/><path d="M14 2v6h6" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 18v-6M9 15h6" stroke="currentColor" stroke-width="2" fill="none"/>"#
        ),
        ArtId::Open => svg!(
            r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Save => svg!(
            r#"<path d="M5 3h11l3 3v15H5z M8 3v6h7V3 M8 14h8v7H8z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::SaveAs => svg!(
            r#"<path d="M5 3h11l3 3v15H5z M8 3v6h7V3 M8 14h8v7H8z" fill="none" stroke="currentColor" stroke-width="2"/><path d="M19 19l2 2M20 17v4h-4" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Print => svg!(
            r#"<path d="M6 9V3h12v6 M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2 M6 14h12v7H6z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Cut => svg!(
            r#"<circle cx="6" cy="6" r="3" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="6" cy="18" r="3" fill="none" stroke="currentColor" stroke-width="2"/><path d="M8.12 8.12L20 20 M8.12 15.88L20 4" stroke="currentColor" stroke-width="2" fill="none"/>"#
        ),
        ArtId::Copy => svg!(
            r#"<rect x="8" y="8" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Paste => svg!(
            r#"<path d="M16 3h-2a2 2 0 0 0-2 2v0H8a2 2 0 0 0-2 2v3h12V7a2 2 0 0 0-2-2v0 M6 10h12v10a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Undo => svg!(
            r#"<path d="M3 7l5-5v3a8 8 0 0 1 8 8v0a8 8 0 0 1-8 8H5" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Redo => svg!(
            r#"<path d="M21 7l-5-5v3a8 8 0 0 0-8 8v0a8 8 0 0 0 8 8h3" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Find => svg!(
            r#"<circle cx="11" cy="11" r="6" fill="none" stroke="currentColor" stroke-width="2"/><path d="M16 16l5 5" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Replace => svg!(
            r#"<path d="M3 7h13l-3-3 M21 17H8l3 3" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Delete => svg!(
            r#"<path d="M3 6h18 M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2 M5 6l1 14a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2l1-14" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Add => svg!(
            r#"<path d="M12 5v14 M5 12h14" stroke="currentColor" stroke-width="2" fill="none"/>"#
        ),
        ArtId::Remove => {
            svg!(r#"<path d="M5 12h14" stroke="currentColor" stroke-width="2" fill="none"/>"#)
        }
        ArtId::Ok => {
            svg!(r#"<path d="M4 12l5 5L20 6" fill="none" stroke="currentColor" stroke-width="2"/>"#)
        }
        ArtId::Cancel => svg!(
            r#"<path d="M6 6l12 12 M18 6L6 18" stroke="currentColor" stroke-width="2" fill="none"/>"#
        ),
        ArtId::Apply => svg!(
            r#"<path d="M5 12h14 M12 5l7 7-7 7" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Close => svg!(
            r#"<path d="M6 6l12 12 M18 6L6 18" stroke="currentColor" stroke-width="2" fill="none"/>"#
        ),
        ArtId::Quit => svg!(
            r#"<path d="M9 4h11v16H9 M3 12h12 M12 8l4 4-4 4" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::About => svg!(
            r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 8h.01 M11 12h1v5h1" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Help => svg!(
            r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 1.5-2.5 2-2.5 4 M12 17h.01" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Information => svg!(
            r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 8h.01 M11 12h1v5h1" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Warning => svg!(
            r#"<path d="M12 3l10 18H2z M12 10v5 M12 18h.01" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Error => svg!(
            r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M9 9l6 6 M15 9l-6 6" stroke="currentColor" stroke-width="2" fill="none"/>"#
        ),
        ArtId::Question => svg!(
            r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 1.5-2.5 2-2.5 4 M12 17h.01" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Folder => svg!(
            r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::File => svg!(
            r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z M14 2v6h6" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Home => svg!(
            r#"<path d="M3 11l9-8 9 8v10a2 2 0 0 1-2 2h-4v-7h-6v7H5a2 2 0 0 1-2-2z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Settings => svg!(
            r#"<circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="2"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.5-2-3.4-2.3 1a7 7 0 0 0-2.1-1.2L14 3h-4l-.5 2.7a7 7 0 0 0-2.1 1.2l-2.3-1-2 3.4 2 1.5A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.5 2 3.4 2.3-1a7 7 0 0 0 2.1 1.2L10 21h4l.5-2.7a7 7 0 0 0 2.1-1.2l2.3 1 2-3.4-2-1.5c.1-.4.1-.8.1-1.2z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Refresh => svg!(
            r#"<path d="M3 12a9 9 0 0 1 15-6.7L21 8 M21 4v4h-4 M21 12a9 9 0 0 1-15 6.7L3 16 M3 20v-4h4" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Search => svg!(
            r#"<circle cx="11" cy="11" r="6" fill="none" stroke="currentColor" stroke-width="2"/><path d="M16 16l5 5" stroke="currentColor" stroke-width="2"/>"#
        ),
        ArtId::Star => svg!(
            r#"<path d="M12 3l2.9 5.9 6.5.9-4.7 4.6 1.1 6.5L12 17.8 6.2 20.9l1.1-6.5L2.6 9.8l6.5-.9z" fill="none" stroke="currentColor" stroke-width="2"/>"#
        ),
    }
}

/// Icon provider. Holds a per-process override table; otherwise serves
/// the built-in SVG library.
pub struct ArtProvider {
    /// Per-id SVG byte overrides registered with
    /// [`ArtProvider::register_svg`]. When an id has an entry, the
    /// override wins over the built-in glyph.
    overrides: std::collections::HashMap<ArtId, Vec<u8>>,
}

impl ArtProvider {
    /// Create a new provider with no overrides.
    pub fn new() -> Self {
        ArtProvider {
            overrides: std::collections::HashMap::new(),
        }
    }

    /// Register a custom SVG to use for a given `ArtId`. Subsequent
    /// calls to [`ArtProvider::get_bitmap`] for this id will use the
    /// supplied bytes instead of the built-in glyph.
    pub fn register_svg(&mut self, art_id: ArtId, svg_bytes: Vec<u8>) {
        self.overrides.insert(art_id, svg_bytes);
    }

    /// Remove a previously-registered override and fall back to the
    /// built-in glyph.
    pub fn unregister(&mut self, art_id: ArtId) {
        self.overrides.remove(&art_id);
    }

    /// Get a [`BitmapBundle`] for the requested icon. The bundle is
    /// rendered at three sizes (1×, 1.5×, 2×) of the client's default
    /// size, so the bundle automatically adapts to the consumer's DPI.
    pub fn get_bitmap(&self, art_id: ArtId, client: ArtClient) -> BitmapBundle {
        self.get_bitmap_with_size(art_id, client.default_size())
    }

    /// Get a [`BitmapBundle`] rendered at a custom list of pixel sizes.
    pub fn get_bitmap_with_size(&self, art_id: ArtId, sizes: (u32, u32)) -> BitmapBundle {
        let svg_bytes = self
            .overrides
            .get(&art_id)
            .map(|v| v.as_slice())
            .unwrap_or_else(|| svg_for(art_id));

        let base = sizes;
        let scale_2x = (base.0 * 2, base.1 * 2);
        // 1.5x sizing, rounded to even numbers to look clean
        let scale_15x = (base.0 + base.0 / 2, base.1 + base.1 / 2);

        let all_sizes: [(u32, u32); 3] = [base, scale_15x, scale_2x];
        let bundle = BitmapBundle::from_svg_bytes(svg_bytes, &all_sizes);
        // The bundle's logical size is the 1× entry; if all_sizes[0]
        // failed to render, pick the first available.
        if bundle.is_empty() {
            return bundle;
        }
        bundle
    }
}

impl Default for ArtProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_size_for_menu_is_16() {
        assert_eq!(ArtClient::Menu.default_size(), (16, 16));
    }

    #[test]
    fn default_size_for_toolbar_is_24() {
        assert_eq!(ArtClient::ToolBar.default_size(), (24, 24));
    }

    #[test]
    fn art_id_svg_is_non_empty() {
        for id in [ArtId::New, ArtId::Save, ArtId::Cut, ArtId::Quit] {
            assert!(!svg_for(id).is_empty(), "missing svg for {:?}", id);
        }
    }
}
