# checkbox.rs

Check-box control mapped to Win32 `BUTTON` with style `BS_AUTOCHECKBOX`.

## Purpose
Native two-state check-box with a label. State is queried/updated via `BM_GETCHECK` / `BM_SETCHECK` so the underlying control is the single source of truth.

## Key Types
- `CheckBox` — `Clone`, wraps `Rc<RefCell<CheckBoxInner>>`. `CheckBoxInner` has `hwnd`, `id`, `label`, `rect`, `enabled`, `visible`.

## Key Functions/Methods
- `CheckBox::new<W: Window>(parent, label)` — creates a 120×24 px auto-check-box.
- `CheckBox::is_checked(&self) -> bool` — sends `BM_GETCHECK` (`0x00F0`), returns `result == 1` (`BST_CHECKED`).
- `CheckBox::set_checked(&self, checked: bool)` — `BM_SETCHECK` with `BST_CHECKED=1` or `BST_UNCHECKED=0`.
- `CheckBox::on_toggle<F: FnMut() + 'static>(&self, frame: &Frame, cb)` — registers a handler invoked when the user toggles the box.
- `CheckBox::set_label` / `CheckBox::get_label` — text via `SetWindowTextW` / `GetWindowTextW`.
- `CheckBox::id(&self) -> u16` — control id.
- `CheckBox::as_widget_ref(&self) -> WidgetRef` — for sizers.

## Win32 Notes
- Class `BUTTON`, style `BS_AUTOCHECKBOX = 0x0003` (auto-toggles on click).
- `BM_GETCHECK` and `BM_SETCHECK` are `u32` (0x00F0 / 0x00F1) message codes.
- Non-Windows stub: `is_checked()` always returns `false`; `set_checked` is a no-op.

## Quick start

```rust
use ru_wx::prelude::*;

let cb = CheckBox::new(&frame, "Enable sound");
cb.on_toggle(&frame, || {
    println!("checked: {}", cb.is_checked());
});

// Read / set state at any time:
println!("current = {}", cb.is_checked());
cb.set_checked(true);
```

## See Also
- [`button.rs`](./button.md) — same scaffolding (BUTTON class, Widget impl).
- [`radio_button.rs`](./radio_button.md) — related but mutually exclusive.
- [`frame.rs`](./frame.md) — `register_command_handler`.
- [`widget.rs`](./widget.md) — `Widget` trait.
