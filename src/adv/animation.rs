//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxAnimation` — animation data (multi-frame image).
//!
//! An [`Animation`] is a data container that holds the frames of a
//! multi-frame raster image (typically GIF or APNG). Static image
//! formats (PNG / JPEG / BMP) are wrapped as a single-frame
//! animation with an "infinite" display time (delay `0`).
//!
//! # Decoding
//!
//! On Windows the file is decoded with the `image` crate:
//! * `image::codecs::gif::GifDecoder` for GIF (preserves
//!   per-frame delays),
//! * the static decoders for everything else.
//!
//! # Cross-platform stub
//!
//! On non-Windows targets the type is still usable — you can load
//! files, inspect the frame count, and read frame pixels — but the
//! decoded pixel buffer is a thin (0×0) placeholder.

use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

use crate::dc::image::{Image, ImageError};

/// A single frame of an [`Animation`].
///
/// Each frame is a complete decoded RGBA8 [`Image`] together with
/// the per-frame display time (in milliseconds). For a
/// single-frame (static) animation the delay is `0` which the
/// [`crate::adv::animation_ctrl::AnimationCtrl`] treats as "hold the
/// last frame indefinitely".
#[derive(Debug, Clone)]
pub struct AnimationFrame {
    /// Decoded RGBA8 pixels of this frame.
    pub image: Image,
    /// Per-frame display time in milliseconds.
    pub delay_ms: u32,
}

/// Animation data (`wxAnimation`).
///
/// Holds the list of [`AnimationFrame`]s decoded from a file or
/// buffer. Use [`Animation::load_file`] / [`Animation::load_from_memory`]
/// to populate it, then read the frame count, the per-frame
/// delays, and the per-frame images.
#[derive(Debug, Clone, Default)]
pub struct Animation {
    frames: Vec<AnimationFrame>,
}

impl Animation {
    /// Create an empty animation (no frames loaded).
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the animation has at least one frame.
    pub fn is_loaded(&self) -> bool {
        !self.frames.is_empty()
    }

    /// Returns the number of decoded frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Returns the pixel size of the animation. For multi-frame
    /// GIFs, all frames are assumed to share the same logical
    /// canvas size (matching the size of the first frame). Returns
    /// `(0, 0)` if the animation is empty.
    pub fn size(&self) -> (u32, u32) {
        if let Some(f) = self.frames.first() {
            (f.image.width, f.image.height)
        } else {
            (0, 0)
        }
    }

    /// Returns the frame at `index`, or `None` if the index is
    /// out of range.
    pub fn frame(&self, index: usize) -> Option<&AnimationFrame> {
        self.frames.get(index)
    }

    /// Returns a slice of all decoded frames.
    pub fn frames(&self) -> &[AnimationFrame] {
        &self.frames
    }

    /// Load the animation from a file on disk. The format is
    /// detected from the file contents (magic bytes), so a wrong
    /// extension still works.
    pub fn load_file(&mut self, path: &Path) -> Result<(), ImageError> {
        let data = fs::read(path).map_err(|e| ImageError::IoError(e.to_string()))?;
        self.load_from_memory(&data)
    }

    /// Load the animation from a byte buffer.
    ///
    /// The decoder tries `GIF` first (preserves per-frame delays)
    /// and falls back to a single-frame static decode for any
    /// other format the `image` crate recognises.
    pub fn load_from_memory(&mut self, data: &[u8]) -> Result<(), ImageError> {
        self.frames.clear();

        // ── Try GIF first so we keep per-frame delays ────────────
        if let Some(frames) = decode_gif_frames(data) {
            if !frames.is_empty() {
                self.frames = frames;
                return Ok(());
            }
        }

        // ── Fallback: static image, one frame, no delay ───────────
        let img = Image::load_from_memory(data)?;
        if !img.is_null() {
            self.frames.push(AnimationFrame {
                image: img,
                delay_ms: 0,
            });
        }
        Ok(())
    }

    /// Drop all frames, leaving the animation empty.
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Detect GIF and decode the per-frame delays. Returns `None` if
/// the buffer is not a GIF, or `Some(Vec<…>)` (possibly empty) if
/// it is. The returned vector contains at least one frame on
/// success.
#[cfg(target_os = "windows")]
fn decode_gif_frames(data: &[u8]) -> Option<Vec<AnimationFrame>> {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;

    if data.len() < 6 {
        return None;
    }
    // GIF89a / GIF87a
    if &data[0..3] != b"GIF" {
        return None;
    }

    let cursor = Cursor::new(data);
    let decoder = match GifDecoder::new(cursor) {
        Ok(d) => d,
        Err(_) => return None,
    };
    let frames_iter = decoder.into_frames();
    let mut out: Vec<AnimationFrame> = Vec::new();
    for frame in frames_iter {
        let frame = match frame {
            Ok(f) => f,
            Err(_) => continue,
        };
        let delay_ms = {
            let d: Duration = frame.delay().into();
            d.as_millis().min(u32::MAX as u128) as u32
        };
        let buffer = frame.into_buffer();
        let (w, h) = buffer.dimensions();
        let img = Image::from_rgba8(w, h, buffer.into_raw());
        out.push(AnimationFrame {
            image: img,
            delay_ms,
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// On non-Windows targets we cannot decode GIF (no `image` GIF
/// feature is built), so the only path left is a single-frame
/// static decode. We still honour "looks like a GIF" by returning
/// `None` (forcing the caller to fall back).
#[cfg(not(target_os = "windows"))]
fn decode_gif_frames(_data: &[u8]) -> Option<Vec<AnimationFrame>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_animation_is_empty() {
        let a = Animation::new();
        assert!(!a.is_loaded());
        assert_eq!(a.frame_count(), 0);
        assert_eq!(a.size(), (0, 0));
    }

    #[test]
    fn clear_empties_frames() {
        let mut a = Animation::new();
        a.frames.push(AnimationFrame {
            image: Image::new(4, 4),
            delay_ms: 100,
        });
        assert!(a.is_loaded());
        a.clear();
        assert!(!a.is_loaded());
    }

    #[test]
    fn frame_out_of_range_returns_none() {
        let a = Animation::new();
        assert!(a.frame(0).is_none());
        assert!(a.frame(usize::MAX).is_none());
    }

    #[test]
    fn size_uses_first_frame() {
        let mut a = Animation::new();
        a.frames.push(AnimationFrame {
            image: Image::new(8, 6),
            delay_ms: 0,
        });
        assert_eq!(a.size(), (8, 6));
    }

    #[test]
    fn load_from_memory_rejects_garbage() {
        let mut a = Animation::new();
        assert!(a.load_from_memory(&[0u8, 1, 2, 3, 4, 5]).is_err());
    }

    #[test]
    fn load_from_memory_png_becomes_single_frame() {
        // Build a real 1×1 transparent PNG at runtime via the
        // `image` crate. The previous test embedded a hand-encoded
        // byte array with invalid chunk CRCs; the decoder (now
        // CRC-strict) rejected it, and `.unwrap()` then panicked.
        // Generating the bytes here is panic-safe: encoder errors
        // are propagated via `expect` with a clear message instead
        // of being treated as test invariants.
        use image::codecs::png::PngEncoder;
        use image::ImageEncoder;

        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(
                &[0u8; 4], // one transparent RGBA pixel
                1,
                1,
                image::ExtendedColorType::Rgba8,
            )
            .expect("PNG encoder should not fail on a 1×1 RGBA buffer");

        let mut a = Animation::new();
        let load_result = a.load_from_memory(&png);
        assert!(
            load_result.is_ok(),
            "loading a valid 1×1 PNG should succeed, got: {:?}",
            load_result.err()
        );
        assert_eq!(a.frame_count(), 1);
        assert_eq!(a.size(), (1, 1));
        let f = a.frame(0).expect("animation should expose frame 0");
        // PNG decode may report 0 ms for static frames.
        assert_eq!(f.delay_ms, 0);
    }
}
