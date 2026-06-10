# radio_button.rs

Single radio-button control mapped to Win32 `BUTTON` with style `BS_AUTORADIOBUTTON`.

## Purpose
One individual radio button. To create a **mutually-exclusive group** on Windows, the first button in the group must carry `WS_GROUP`; subsequent buttons in the same group omit it until the next group start.

For a higher-level group API see [`radio_box.rs`](radio_box.md), which manages a groupbox frame + N radios for you.

## Key Types
- `RadioButton` — `Clone`, wraps `Rc<RefCell<RadioButtonInner>>`. Fields: `hwnd`, `id`, `label`, `rect`, `enabled`, `visible`.

## Key Functions/Methods
- `RadioButton::new<W: Window>(parent, label, is_group_start: bool)` — creates a 120×24 px radio. If `is_group_start == true`, adds `WS_GROUP = 0x0002_0000` so the OS treats this as the start of a new group.
- `RadioButton::is_selected(&self) -> bool` — `BM_GETCHECK` returns `BST_CHECKED (1)`.
- `RadioButton::set_selected(&self, selected: bool)` — `BM_SETCHECK` with 0/1.
- `RadioButton::on_select<F: FnMut() + 'static>(&self, frame: &Frame, cb)` — registers selection handler.
- `RadioButton::id(&self) -> u16`, `RadioButton::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Class `BUTTON`, style `BS_AUTORADIOBUTTON = 0x0009`.
- `WS_GROUP` (0x0002_0000) is the marker that ends the previous group and starts a new one; without it, all consecutive radios form one group.
- The Win32 radio-group behaviour is driven by tab order, not by any single control property, so when manually placing radios into a group make sure `is_group_start` is set on the first widget of the group.
- No `set_label` / `get_label` here (kept minimal — use the in-memory `label` field via `id` if needed).

## Quick start

```rust
use ru_wx::prelude::*;

// Low-level: set is_group_start = true on the FIRST button of each group.
let r1 = RadioButton::new(&frame, "Option A", true);   // start of group
let r2 = RadioButton::new(&frame, "Option B", false);
let r3 = RadioButton::new(&frame, "Option C", false);

r1.on_select(&frame, || println!("A selected"));
```

For a higher-level group API (frame + N radios managed for you), see `RadioBox`.

## See Also
- [`radio_box.rs`](radio_box.md) — composite that builds a `BS_GROUPBOX` frame plus N radios.
- [`checkbox.rs`](checkbox.md) — similar API but non-exclusive.
- [`button.rs`](button.md) — same internal `*Inner` layout.
- [`frame.rs`](../window/frame.md) — `register_command_handler`.
- [`widget.rs`](../core/widget.md) — `Widget` trait.
