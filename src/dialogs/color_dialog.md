# color_dialog.rs

Colour picker dialog (`wxColourDialog`).

## Purpose
Modal dialog that lets the user pick an RGB colour. On Windows it wraps the standard `ChooseColorW` Win32 common dialog.

## Key Types
- `ColorDialog` — re-exported as `ColourDialog` from `lib.rs`.

## Key Functions/Methods
- `ColorDialog::new<W: Window>(parent: &W)` — creates the dialog with the current default colour (black).
- `ColorDialog::with_colour<W: Window>(parent: &W, initial: Colour)` — pre-selects a colour.
- `ColorDialog::show_modal() -> Option<Colour>` — runs the dialog. Returns `Some(colour)` on OK, `None` on cancel.

## Win32 Notes
- Built on top of the `comdlg32!ChooseColorW` API.
- Uses a `CHOOSECOLORW` struct on the stack and 16 custom-colour slots; the struct is the dialog's working memory.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

if let Some(colour) = ColorDialog::new(&frame).show_modal() {
    println!("Picked RGB({}, {}, {})", colour.r, colour.g, colour.b);
}

// Pre-select a starting colour:
if let Some(c) = ColorDialog::with_colour(&frame, Colour::from_rgb(32, 64, 128))
    .show_modal() { /* … */ }
```

## See Also
- [`colour_picker_ctrl.rs`](../controls/colour_picker_ctrl.md) — in-place colour picker control (non-modal).
- [`geometry.rs`](../core/geometry.md) — `Colour` type used here.
- [`message_dialog.rs`](message_dialog.md) — sibling modal dialog pattern.
