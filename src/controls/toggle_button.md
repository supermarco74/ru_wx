# toggle_button.rs

Toggle-button control mapped to Win32 `BUTTON` class with style `BS_PUSHBUTTON`. The "stays pressed" visual is achieved by sending `BM_SETCHECK` (`BST_CHECKED` / `BST_UNCHECKED`) after the user clicks the button; the logical state is tracked in Rust.

## Purpose
Wraps a native Win32 push-button that "sticks" when clicked, mirroring `wxToggleButton`. Use [`ToggleButton::get_value`] / [`ToggleButton::set_value`] to query / set the state, and [`ToggleButton::on_toggle`] to register a state-change callback.

## Key Types
- `ToggleButton` — `Clone`, holds `Rc<RefCell<ToggleButtonInner>>`. `ToggleButtonInner` stores `hwnd: HWND`, `id: u16`, `label`, `rect`, `enabled`, `visible`, and `checked: bool`.

## Key Functions/Methods
- `ToggleButton::new<W: Window>(parent, label)` — creates a toggle button (100×30 px) starting in the unchecked state.
- `ToggleButton::with_value<W: Window>(parent, label, checked: bool)` — same as `new` but with a non-default initial state.
- `ToggleButton::get_value(&self) -> bool` — return the current state.
- `ToggleButton::is_checked(&self) -> bool` — convenience alias for `get_value`.
- `ToggleButton::set_value(&self, checked: bool)` — set the state; sends `BM_SETCHECK` to update the visual.
- `ToggleButton::toggle(&self) -> bool` — flip the state and return the new value.
- `ToggleButton::set_label(&self, label) / get_label(&self) -> String` — `SetWindowTextW` / `GetWindowTextW`.
- `ToggleButton::on_click<F: FnMut()>(&self, frame, cb)` — click-only callback.
- `ToggleButton::on_toggle<F: FnMut(bool)>(&self, frame, cb)` — state-change callback (gets the new state).
- `ToggleButton::id(&self) -> u16` — returns the control id used for `WM_COMMAND` dispatch.
- `ToggleButton::as_widget_ref(&self) -> WidgetRef` — for use with sizers.

## Win32 Notes
- `BUTTON` class, `BS_PUSHBUTTON` (default) style.
- `BM_GETCHECK` (`0x00F0`) / `BM_SETCHECK` (`0x00F1`) for state queries / updates.
- `BST_UNCHECKED` (`0x0000`), `BST_CHECKED` (`0x0001`), `BST_INDETERMINATE` (`0x0002`) for the three check states.
- The cached `checked: bool` in `ToggleButtonInner` is the source of truth; the live control is kept in sync via `BM_SETCHECK` on every `set_value` / `toggle` and on every click notification.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let tb = ToggleButton::new(&frame, "Mute");

// Toggle programmatically and read the result.
tb.toggle();                              // flip and get new value
if tb.is_checked() { /* muted */ }

// React to user clicks (state-change callback).
let tb_for_cb = tb.clone();
tb.on_toggle(&frame, move |checked| {
    println!("Mute = {checked}");
});

// Start in the checked state.
let pre_on = ToggleButton::with_value(&frame, "Pre-on", true);
```

The Rust-side `checked: bool` is the source of truth; the control is
kept in sync via `BM_SETCHECK` on every state change. Use `on_click`
for "fires on every press" semantics, or `on_toggle` for "fires only
when the state actually changed".

## See Also
- [`button.rs`](button.md) — the standard non-toggle push-button.
- [`frame.rs`](../window/frame.md) — `Frame::register_command_handler` used by `on_click` / `on_toggle`.
- [`widget.rs`](../core/widget.md) — `Widget` trait, `Window` trait, `WidgetRef`.
- [`platform/win32.rs`](../platform/win32.md) — `next_control_id`, `to_wide`.
