//! Chrome widgets: MenuBar, ToolBar, StatusBar.
//!
//! | Module       | Widget          | Reactor equivalent       |
//! |--------------|-----------------|--------------------------|
//! | `menu_bar`   | `XamlMenuBar`   | `reactor::MenuBar`       |
//! | `tool_bar`   | `XamlToolBar`   | `reactor::CommandBar`    |
//! | `status_bar` | `XamlStatusBar` | `reactor::hstack` custom |

pub mod menu_bar;
pub mod status_bar;
pub mod tool_bar;

pub use menu_bar::{XamlMenu, XamlMenuBar};
pub use status_bar::XamlStatusBar;
pub use tool_bar::XamlToolBar;
