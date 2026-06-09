# choice.rs

Read-only drop-down list (`wxChoice`). On Windows it is realised as a Win32 `COMBOBOX` with style `CBS_DROPDOWNLIST | CBS_HASSTRINGS`.

## Purpose
A non-editable pick-list: the user can choose an item from the drop-down but cannot type into the text field. Notifications fire as `CBN_SELCHANGE` delivered through `WM_COMMAND`.

## Key Types
- `Choice` — `Clone`, wraps `Rc<RefCell<ChoiceInner>>`. `ChoiceInner` holds `hwnd`, `id`, `rect`, `enabled`, `visible`.

## Key Functions/Methods
- `Choice::new<W: Window>(parent)` — creates a 150×200 (drop-down height) `CBS_DROPDOWNLIST | CBS_HASSTRINGS` combo.
- `Choice::append(&self, item)` / `Choice::insert(&self, index, item)` / `Choice::remove(&self, index)` / `Choice::clear(&self)` — manage the item list via `CB_ADDSTRING` (0x0143), `CB_INSERTSTRING` (0x014A), `CB_DELETESTRING` (0x0144), `CB_RESETCONTENT` (0x014B).
- `Choice::get_count(&self) -> usize` — `CB_GETCOUNT` (0x0146).
- `Choice::get_selection(&self) -> Option<usize>` — `CB_GETCURSEL` (0x0147), `None` if `CB_ERR` (-1).
- `Choice::set_selection(&self, index)` — `CB_SETCURSEL` (0x014E).
- `Choice::get_string(&self, index) -> Option<String>` — `CB_GETLBTEXTLEN` (0x0149) + `CB_GETLBTEXT` (0x0148), decodes UTF-16 with `String::from_utf16_lossy`.
- `Choice::on_selection_change<F: FnMut() + 'static>(&self, frame, cb)` — registers a handler on the frame's command-handler map keyed by control id.
- `Choice::id(&self) -> u16`, `Choice::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Class `COMBOBOX`, style `CBS_DROPDOWNLIST = 0x0003` (no edit field) **plus** `CBS_HASSTRINGS = 0x0200` so the combo stores strings and emits `CBN_SELCHANGE`.
- The 200-px height in `CreateWindowExW` is the **drop-down list** height, not the visible control height; the actual collapsed box is ~24 px.
- `CB_ERR = -1` is the universal "not found / no selection" sentinel for the CB_* messages.
- Non-Windows stub: `get_count` returns 0, `get_selection` / `get_string` return `None`.

## Quick start

```rust
use ru_wx::prelude::*;

let pick = Choice::new(&frame);
pick.append("Apple");
pick.append("Banana");
pick.append("Cherry");
pick.set_selection(0);

pick.on_selection_change(&frame, || {
    if let Some(i) = pick.get_selection() {
        println!("Picked: {}", pick.get_string(i).unwrap_or_default());
    }
});
```

## See Also
- [`combo_box.rs`](./combo_box.md) — same class, but offers an editable `CBS_DROPDOWN` variant.
- [`list_box.rs`](./list_box.md) — non-drop-down always-visible list.
- [`frame.rs`](./frame.md) — `register_command_handler` used by `on_selection_change`.
- [`widget.rs`](./widget.md) — `Widget` trait.
