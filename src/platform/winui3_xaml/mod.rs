//! WinUI3/XAML Backend — real WinRT via windows-reactor bridge
//!
//! This backend provides a native WinUI3/XAML implementation using the
//! `windows-reactor` crate for WinRT bindings and element composition.
//!
//! # Architecture
//!
//! ```text
//! ru_wx unified API (imperative)
//!   ↓
//! element_bridge.rs  — widget descriptors (WidgetDesc, FrameDesc)
//!   ↓
//! app.rs             — XamlApp converts descriptors → reactor Element tree
//!   ↓
//! windows-reactor    — declarative Element → WinRT/XAML controls
//!   ↓
//! Microsoft.UI.Xaml  — native WinUI3 rendering
//! ```
//!
//! # Module layout
//!
//! | Module             | Role                                              |
//! |--------------------|---------------------------------------------------|
//! | `bootstrap`        | WinAppSDK `MddBootstrapInitialize2`               |
//! | `element_bridge`   | Widget descriptors + event registry               |
//! | `events`           | Event dispatch (reactor callback → user closure)  |
//! | `window`           | `XamlFrame` builder + window methods              |
//! | `app`              | `XamlApp` entry point + element tree builder      |
//!
//! # Usage
//!
//! ```ignore
//! use ru_wx::platform::winui3_xaml::*;
//!
//! let app = XamlApp::new();
//! let frame = XamlFrame::builder()
//!     .with_title("My App")
//!     .with_size(600, 400)
//!     .build();
//! app.run(frame)?;
//! ```

pub mod app;
pub mod bootstrap;
pub mod chrome;
pub mod controls;
pub mod dialogs;
pub mod element_bridge;
pub mod element_tree;
pub mod events;
pub mod layout;
pub mod window;

// Re-export the main types users interact with.
pub use app::XamlApp;
pub use chrome::*;
pub use controls::*;
pub use dialogs::*;
pub use layout::*;
pub use window::{XamlFrame, XamlFrameBuilder};

// Re-export bridge types for widget constructors (crate-internal).
#[allow(unused_imports)] // used by widget modules added in later phases
pub(crate) use element_bridge::{
    new_frame_ref, new_shared_registry, next_id, FrameDesc, FrameRef, SharedRegistry, WidgetDesc,
    WidgetId,
};
