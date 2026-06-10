# static_box.rs

Labelled box border container (`wxStaticBox` analog). Renders a rectangular etched group with a text label; used to group related controls visually.

## Purpose
A decorative group frame. Children placed inside the box (via a sizer) appear visually clustered. The box itself is non-interactive — it neither accepts focus nor generates events.

## Key Types
- `StaticBox` — public struct.
- `StaticBoxInner` (private) — Win32 `HWND` for the underlying `BUTTON` control used in `BS_GROUPBOX` style.

## Key Methods
- `StaticBox::new(label: &str) -> Self` — 200×100 group box with label.
- `StaticBox::new_empty() -> Self` — 200×100 group box with no label.
- `StaticBox::with_size(label: &str, w: u32, h: u32) -> Self` — explicit dimensions.
- `set_label(&self, label: &str)` — Updates label text via `SetWindowTextW`.
- `get_label(&self) -> String` — Live query via `GetWindowTextW`.

## Win32 Notes
- Window class: built-in `BUTTON` with style `BS_GROUPBOX` (`0x0007`). This is the canonical Win32 trick for drawing a labelled frame: a button control with the groupbox style renders as a hollow rectangle with a top-left label.
- Style flag: `WS_CHILD | WS_VISIBLE | BS_GROUPBOX`. Always child; never top-level.
- Constants: `DEFAULT_W = 200`, `DEFAULT_H = 100`.
- The `BUTTON` window class normally intercepts keyboard/mouse to draw the focused state; in groupbox style it does not. The label can be changed at any time with `set_label`.

## Tests
- `default_dimensions_match_constants` — Verifies the default size matches the `DEFAULT_W`/`DEFAULT_H` constants.
- `get_label_returns_initial_value` — Verifies the label is read back correctly after construction.

## Quick start

```rust
use ru_wx::prelude::*;

// A StaticBox is a decorative frame. Children go *inside* it via a
// sizer, but the box itself does not host the sizer — the parent
// panel / frame does.

// group is added to the same sizer as its children so it visually
// surrounds them.
let group   = StaticBox::new("Options");
let sizer   = BoxSizer::new(Orientation::Vertical);
sizer.add(&group, 0, SizerFlag::Expand, 0);

// Child controls (e.g. CheckBox) are also children of the parent
// and laid out inside the group via a nested sizer.
let inner   = BoxSizer::new(Orientation::Vertical);
inner.add(&CheckBox::new(&frame, "Option A"), 0, SizerFlag::Expand, 0);
inner.add(&CheckBox::new(&frame, "Option B"), 0, SizerFlag::Expand, 0);
// In real code: frame.set_sizer_for(group, inner);
```

`StaticBox` is **non-interactive** — for a true container that can host
its own sizer, background, and forwarded events, use a
[`Panel`](../window/panel.md) instead.

## See Also
- [`panel.rs`](../window/panel.md) — true container with custom WndProc; use for interactive grouped content
- [`static_text.rs`](static_text.md) — sibling static control without border
- [`sizer.rs`](../containers/sizer.md) — required to lay out children relative to a `StaticBox`
