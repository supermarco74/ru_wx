//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Window chrome — toolbars, status bars, and tray icons.
//!
//! The "decorations" that surround the client area of a frame:
//! * [`aui_tool_bar`] — docking toolbar (AUI).
//! * [`icon_tray`] — system-tray icon and balloon notifications.
//! * [`status_bar`] — multi-field status bar at the bottom.
//! * [`tool_bar`] — standard toolbar of icons.

pub mod aui_manager;
pub mod aui_tool_bar;
pub mod aui_toolbar_event;
pub mod header_ctrl;
pub mod info_bar;
pub mod info_bar_event;
pub mod ribbon_bar_event;
pub mod ribbon_bar;
pub mod ribbon_page;
pub mod ribbon_panel;
pub mod ribbon_button_bar;
pub mod ribbon_gallery;
pub mod ribbon_art_provider;
pub mod icon_tray;
pub mod notification_message;
pub mod status_bar;
pub mod taskbar_icon_event;
pub mod tool_bar;
