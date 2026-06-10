//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Drag-and-drop support.
//!
//! * [`drop_target`] — Shell-style file drop (paths only).
//! * [`ole_dnd`] — full OLE COM drag-and-drop (text, URLs, files,
//!   virtual files).

pub mod drag_image;
pub mod drop_target;
pub mod ole_dnd;
