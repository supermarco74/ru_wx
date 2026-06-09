# animation_ctrl.rs

`wxAnimationCtrl` analog — a custom WndProc-based widget that displays an [`Animation`](./animation.md) and advances its frames at the per-frame rate declared by the GIF (or, for static images, at 100 ms per tick).

## Purpose

The **display** half of the animation pair (the other half is [`Animation`](./animation.md)). `AnimationCtrl` is a real child window: you drop one in a sizer, call `set_animation(anim)` to install a decoded animation, then `play()` / `stop()` to control playback.

## Key Types

- **`AnimationCtrl`** — public struct, wraps `Rc<RefCell<AnimationCtrlInner>>`. Cloneable.
- `AnimationCtrlInner` (private) — state: `hwnd`, `animation`, `current_frame`, `playing`, `rect`, `visible`, `enabled`.

## Constants

- `AnimationCtrl::DEFAULT_W: u32 = 32` / `DEFAULT_H: u32 = 32` — used by the size-less constructor.

## Constructors

- `AnimationCtrl::new<W: Window>(parent: &W) -> Self` — 32×32 control.
- `AnimationCtrl::with_size<W: Window>(parent: &W, width: u32, height: u32) -> Self`.

## Key Methods

- `set_animation(&self, anim: Animation)` — install a new animation. Resets the current frame to `0`. If a previous animation was playing, it is stopped.
- `clear_animation(&self)` — drop the current animation. The control paints an empty (background-coloured) client area until a new animation is installed.
- `play(&self)` — start playback. No-op if the animation is empty or already playing.
- `stop(&self)` — stop. The control keeps showing the first frame.
- `is_playing(&self) -> bool`.
- `current_frame(&self) -> usize` — index of the frame currently drawn. Returns `0` when stopped or empty.
- `animation(&self) -> Option<Animation>` — a clone of the installed animation, or `None` if empty.
- `as_widget_ref(&self) -> WidgetRef` — for use with sizers.
- `hwnd(&self) -> HWND` (Windows) / `0` (stub).

## Quick start

```rust,no_run
use std::path::Path;
use ru_wx::prelude::*;

// 1) Decode the source (GIF preserves per-frame delays; PNG/JPG/BMP
//    become a single-frame animation with delay_ms = 0).
let mut anim = Animation::new();
anim.load_file(Path::new("loader.gif"))?;
assert!(anim.is_loaded());
println!("{} frames @ {}x{}",
         anim.frame_count(), anim.size().0, anim.size().1);

// 2) Build the control, sized to match the first frame so the
//    bitblit is 1:1 (no stretching artefacts).
let (w, h) = anim.size();
let ctrl = AnimationCtrl::with_size(&frame, w.max(1), h.max(1));

// 3) Install the animation and start playing.
ctrl.set_animation(anim);
ctrl.play();
assert!(ctrl.is_playing());

// 4) Drive the UI from the control:
//    - current_frame() advances on every WM_TIMER tick
//    - stop() pauses on the first frame
//    - clear_animation() removes the source (repaints an empty
//      client area).
frame.on_close(move || {
    ctrl.stop();
    ctrl.clear_animation();
});
```

Embedding in a sizer — clone the control and hand a `WidgetRef`
to the parent, the same way as every other widget:

```rust,no_run
use ru_wx::prelude::*;
let ctrl = AnimationCtrl::new(&frame);
let sizer = BoxSizer::builder(Orientation::Horizontal).build();
sizer.add(ctrl.as_widget_ref(), 1, SizerFlag::Expand | SizerFlag::All, 8);
frame.set_sizer(sizer);
```

`AnimationCtrl` is a real child window (`class = "RuWxAnimationCtrl"`),
so it participates in tab order, DPI scaling, and the frame's
`DragAcceptFiles` plumbing like any other control. The internal
`WM_TIMER` id `0xC0_1D` is private to this module; do not pass
that value to `Timer::start` from your own code or you will
interleave your timer with the frame advance.

## Usage

```rust,no_run
use ru_wx::prelude::*;

let mut anim = Animation::new();
anim.load_file(std::path::Path::new("loader.gif"))?;

let ctrl = AnimationCtrl::new(&frame);
ctrl.set_animation(anim);
ctrl.play();

// Later:
ctrl.stop();
ctrl.clear_animation();
```

## Win32 Notes

- The control is a child window of class `RuWxAnimationCtrl`, registered at first construction (`CS_HREDRAW | CS_VREDRAW`, `IDC_ARROW` cursor).
- On each `WM_TIMER` tick the frame index is advanced; on each `WM_PAINT` the current frame is converted to a `Bitmap` and blitted to the client area.
- The timer ID `0xC0_1D` is internal — picked to avoid collisions with the user's other timers.
- The timer is re-armed with the new frame's delay, **clamped to a 10 ms minimum** (`MIN_FRAME_DELAY_MS = 10`). 10 ms is a safe floor: any tighter interval would saturate the `WM_TIMER` queue on most machines without giving the user any visible difference.
- For static (single-frame) animations the per-frame delay is `0`, which the control interprets as "use the default 100 ms tick" (`DEFAULT_FRAME_DELAY_MS = 100`, i.e. 10 fps).
- The current frame is blitted aspect-preserving: if its size matches the control's client area it is `BitBlt`ed at 1:1, otherwise it is `StretchBlt`ed.
- Each repaint produces a fresh DIB from the current frame's pixels; the previous one is released at the end of the paint arm.
- `Drop` calls `KillTimer` on the HWND if the control was still playing, so a dropped `AnimationCtrl` does not leak a timer message.
- `WM_ERASEBKGND` is suppressed (`return 1`) because the paint arm fills the client area itself.
- `GWLP_USERDATA` stores the raw `Rc<RefCell<…>>` pointer; the WndProc reconstructs an owned `Rc` for the duration of the arm and releases the extra strong count on drop. This is the same re-entrancy-safe pattern used by [`frame.md`](./frame.md).

## Cross-platform stub

On non-Windows targets the type is constructible (as a width/height record with no real window) and the getters / setters are no-ops, so code can compile cross-platform. `play()` does not start a timer.

## Tests

- `default_size_is_positive` — `DEFAULT_W` and `DEFAULT_H` are non-zero.
- `new_control_is_not_playing` — a freshly-constructed inner has `playing == false`, `current_frame == 0`.

## See Also

- [`animation.md`](./animation.md) — the data source.
- [`image.md`](./image.md) — frame pixels are stored as `Image`.
- [`bitmap.md`](./bitmap.md) — `Image::to_bitmap` is the conversion used in `WM_PAINT`.
- [`widget.md`](./widget.md) — `Window` trait, `as_widget_ref`.
