//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Device contexts and the drawing primitives they draw with.
//!
//! This is the "I want to draw on a window" layer:
//! * [`art_provider`] — themed stock icons (folder, save, …).
//! * [`bitmap`] / [`bitmap_bundle`] / [`image`] / [`image_list`] —
//!   raster image types.
//! * [`brush`] / [`pen`] — solid fills and strokes.
//! * [`dc`] — `Dc` plus the per-context flavours (`PaintDC`,
//!   `MemoryDC`, `WindowDC`, `ClientDC`).
//! * [`gl_canvas`] — OpenGL rendering surface.
//! * [`icon`] — vector (SVG) icon helpers.

pub mod art_provider;
pub mod auto_buffered_paint_dc;
pub mod buffered_dc;
pub mod buffered_paint_dc;
pub mod bitmap;
pub mod bitmap_bundle;
pub mod brush;
#[allow(clippy::module_inception)] // `dc::dc` keeps the wxDC name parity
pub mod dc;
pub mod gl_canvas;
pub mod graphics_context;
pub mod icon;
pub mod image;
pub mod image_list;
pub mod image_handler;
pub mod bitmap_handler;
pub mod mirror_dc;
pub mod palette;
pub mod pen;
pub mod region;
pub mod svg_bitmap;
