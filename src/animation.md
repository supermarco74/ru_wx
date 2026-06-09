# animation.rs

Multi-frame raster animation data (`wxAnimation`). A pure-data container — pair it with [`AnimationCtrl`](./animation_ctrl.md) to actually display the frames on screen.

## Purpose

`Animation` holds a list of decoded frames (image + per-frame delay) loaded from a file or byte buffer. It is the **data** half of the pair; [`AnimationCtrl`] is the **display** half.

* On Windows the file is decoded with the `image` crate:
  * `image::codecs::gif::GifDecoder` for GIF (preserves per-frame delays),
  * the static decoders for everything else.
* On non-Windows targets the type is still usable — load, inspect frame count, read frame pixels — but the decoded pixel buffer is a thin (0×0) placeholder.

## Key Types

- **`Animation`** — public struct wrapping `Vec<AnimationFrame>`. Cloneable, default-constructible.
- **`AnimationFrame`** — public struct:
  - `image: Image` — decoded RGBA8 frame.
  - `delay_ms: u32` — per-frame display time in ms. `0` = "hold this frame indefinitely" (used for static single-frame animations).

## Key Methods

- `Animation::new() -> Self` — empty animation, no frames.
- `Animation::is_loaded(&self) -> bool` — `true` if at least one frame has been decoded.
- `Animation::frame_count(&self) -> usize`.
- `Animation::size(&self) -> (u32, u32)` — pixel size of the first frame, or `(0, 0)` when empty.
- `Animation::frame(&self, index: usize) -> Option<&AnimationFrame>`.
- `Animation::frames(&self) -> &[AnimationFrame]` — slice of all decoded frames.
- `Animation::load_file(&mut self, path: &Path) -> Result<(), ImageError>` — read the file from disk and decode.
- `Animation::load_from_memory(&mut self, data: &[u8]) -> Result<(), ImageError>` — decode from a byte buffer. Tries GIF first (preserves per-frame delays), falls back to a single-frame static decode for any other format the `image` crate recognises.
- `Animation::clear(&mut self)` — drop all frames.

## Quick start

```rust,no_run
use std::path::Path;
use ru_wx::prelude::*;

// Load a GIF (per-frame delays preserved) or any other format
// (single frame, delay_ms = 0).
let mut anim = Animation::new();
anim.load_file(Path::new("loading.gif"))?;
assert!(anim.is_loaded());

let (w, h) = anim.size();
println!("logical canvas: {w}x{h}");
println!("frames: {}", anim.frame_count());
for (i, f) in anim.frames().iter().enumerate() {
    println!("  frame {i}: {}x{}, hold {} ms",
             f.image.width, f.image.height, f.delay_ms);
}

// Read a single frame by index. Returns None when out of range.
if let Some(f0) = anim.frame(0) {
    let _pixels: &[u8] = f0.image.as_slice();
}

// Load from an in-memory byte buffer (network / embedded asset).
let mut from_mem = Animation::new();
from_mem.load_from_memory(include_bytes!("../assets/spinner.gif"))?;
assert_eq!(from_mem.frame_count(), 8);

// Drop everything when the source is no longer needed — the
// decoded pixel buffers are released immediately.
anim.clear();
assert!(!anim.is_loaded());
```

The data side has no Win32 handle of its own; it is plain
`Vec<AnimationFrame>`. Once a frame is rendered it is converted
to a 32-bit DIB section by `Image::to_bitmap` and blitted /
stretched into the control's HDC inside the `AnimationCtrl`
window procedure — see [`animation_ctrl.md`](./animation_ctrl.md).
That is the only place the data meets the display surface.

## Usage

```rust,no_run
use ru_wx::prelude::*;

let mut anim = Animation::new();
anim.load_file(std::path::Path::new("loading.gif"))?;
assert!(anim.is_loaded());
let f0 = anim.frame(0).unwrap();
println!("frame 0 is {}x{}, hold {} ms", f0.image.width, f0.image.height, f0.delay_ms);
```

To play the animation in a window:

```rust,no_run
use ru_wx::prelude::*;
let mut anim = Animation::new();
anim.load_file(std::path::Path::new("loading.gif"))?;
let ctrl = AnimationCtrl::new(&frame);
ctrl.set_animation(anim);
ctrl.play();
```

## Cross-platform behaviour

- The struct and all getters are available on every target.
- `load_file` / `load_from_memory` use the `image` crate with the `gif` feature. On Windows the GIF decoder is wired up; on other targets a "looks like a GIF" check returns the single-frame fallback (which itself may fail for GIF bytes without a decoder).

## Win32 Notes

- Decoded frames are stored as RGBA8 in an [`Image`](./image.md); the same buffer is reused on every blit. Each repaint converts the current frame to a 32-bit DIB and `BitBlt`s / `StretchBlt`s it into the control's HDC.
- Frame delays in a GIF are in centiseconds; this module multiplies by 10 to express them as milliseconds. The minimum delay enforced by the display side (`AnimationCtrl`) is 10 ms — see [`animation_ctrl.md`](./animation_ctrl.md).

## Tests

- `new_animation_is_empty` — `frame_count == 0`, `size == (0, 0)`, `is_loaded == false`.
- `clear_empties_frames` — `clear` drops all frames.
- `frame_out_of_range_returns_none` — indexing past the end yields `None`.
- `size_uses_first_frame` — `size` reflects the first frame's dimensions.
- `load_from_memory_rejects_garbage` — 6-byte buffer that is not a recognised format returns `Err`.
- `load_from_memory_png_becomes_single_frame` — a hand-rolled 1×1 PNG decodes to one frame with `delay_ms == 0`.

## See Also

- [`animation_ctrl.md`](./animation_ctrl.md) — display control that renders an `Animation` at its declared rate.
- [`image.md`](./image.md) — `Image` is the per-frame pixel buffer.
- [`bitmap.md`](./bitmap.md) — `Image::to_bitmap` is the conversion used during `WM_PAINT`.
