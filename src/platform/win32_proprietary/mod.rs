//! Win32 Proprietary Backend — native Win32 wrappers for the advanced
//! widgets, mirroring the `appkit.rs` model on macOS.
//!
//! Each subsystem in `ru_wx::adv::*` is split into two implementations:
//! * a public, cross-platform Rust API in `src/adv/<widget>.rs`
//! * a platform-specific proprietary wrapper that drives the native
//!   Win32 control (WebView2, WIC, Media Foundation, RichEdit, etc.)
//!
//! On Windows the adv module calls the wrappers in this folder; on
//! macOS the corresponding calls go to `platform::appkit::*`. Both
//! backends expose the same function names so the cfg-gated call sites
//! in `src/adv` stay minimal.

pub mod animation;
pub mod html_window;
pub mod media;
pub mod property_grid;
pub mod webview;
pub mod wizard;

pub use animation::*;
pub use html_window::*;
pub use media::*;
pub use property_grid::*;
pub use webview::*;
pub use wizard::*;
