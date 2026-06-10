//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Top-level window types.
//!
//! These are the *window shells* — the `HWND`-backed containers that
//! host other widgets. Controls live in the [`crate::controls`]
//! module; layout helpers in [`crate::containers`].
//!
//! Contents:
//! * [`dialog`] — generic modal / modeless `Dialog`.
//! * [`frame`] — top-level `Frame` and its `FrameBuilder`.
//! * [`frame_extras`] — `MiniFrame`, `SplashScreen`, `TipWindow`.
//! * [`mdi`] — MDI parent / child frames.
//! * [`menu`] — `Menu`, `MenuBar`, `MenuItem`.
//! * [`panel`] — generic child panel with its own WndProc.
//! * [`popup_menu`] — right-click `PopupMenu`.
//! * [`top_level_window`] — `TopLevelWindow` trait and helpers.

pub mod banner_window;
pub mod dialog;
pub mod dwm_style;
pub mod frame;
pub mod frame_extras;
pub mod layer_window;
pub mod mdi;
pub mod menu;
pub mod native_window;
pub mod panel;
pub mod popup_menu;
pub mod file_history;
pub mod popup_transient_window;
pub mod popup_window;
pub mod rich_tooltip;
pub mod top_level_window;
