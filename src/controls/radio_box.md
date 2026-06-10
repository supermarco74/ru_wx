# radio_box.rs

Composite radio-box (`wxRadioBox`): a labelled `BS_GROUPBOX` frame containing N mutually-exclusive `BS_AUTORADIOBUTTON` children.

## Purpose
A higher-level alternative to laying out [`RadioButton`](radio_button.md) widgets manually. Owns:
- One outer `BS_GROUPBOX` `BUTTON` (the visual frame carrying the title).
- N child `BS_AUTORADIOBUTTON` controls. The first carries `WS_GROUP` so the OS treats them as a single mutually-exclusive group.

Sizers position the whole composite via the groupbox `HWND`; the radio children move with it.

## Key Types
- `RadioBox` — `Clone`, wraps `Rc<RefCell<RadioBoxInner>>`. `Inner` holds `box_hwnd`, `radio_hwnds: Vec<HWND>`, `id`, `rect`, `enabled`, `visible`.

## Key Functions/Methods
- `RadioBox::new<W: Window>(parent, title, labels: &[&str])` — convenience, defaults initial selection to 0.
- `RadioBox::with_selection<W: Window>(parent, title, labels, initial_selection)` — full constructor; `initial_selection` is `min(requested, len-1)`.
  - Computes box size: `box_width = 200`, `box_height = 18 + labels.len() * 22 + 8`. Row height is 22.
  - Spawns the `BS_GROUPBOX` frame plus N `BS_AUTORADIOBUTTON` children, each offset by `(box_padding_x=10, box_padding_top=18 + i*row_height)`.
  - Each child gets its own `next_control_id()` so it can carry an independent `WM_COMMAND` id.
- `RadioBox::get_selection(&self) -> Option<usize>` — iterates `radio_hwnds`, returns first index whose `BM_GETCHECK` returns `BST_CHECKED`.
- `RadioBox::set_selection(&self, index: usize)` — `BM_SETCHECK = 1` for the chosen index, `0` for all others.
- `RadioBox::len(&self) -> usize`, `RadioBox::is_empty(&self) -> bool`.
- `RadioBox::on_select<F: FnMut(usize) + 'static>(&self, frame: &Frame, cb)` — installs the same handler on every radio id (wrapped in `Rc<RefCell<>>` to share state). On fire, scans for the checked index and calls `cb(index)`.
- `RadioBox::id(&self) -> u16` — id of the outer groupbox.
- `RadioBox::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Two distinct control classes under the hood, both `BUTTON`:
  - `BS_GROUPBOX = 0x0007` for the labelled frame.
  - `BS_AUTORADIOBUTTON = 0x0009` for the children.
- `WS_GROUP = 0x0002_0000` is added only to the first child.
- The child ids are recovered after the fact via `GetDlgCtrlID(hwnd)` (helper `get_radio_id`).
- `set_position` and `set_size` use `GetWindowRect` + `ScreenToClient(GetParent)` to move children by the same delta as the groupbox; on resize the children share the new width evenly (`(w - 20) / n`).
- `set_visible` / `set_enabled` apply to the groupbox **and** all radios (so the whole composite can be hidden/disabled atomically).

## Known Limitations
- `on_select` installs a separate `WM_COMMAND` handler per radio id. If you also install handlers on those ids elsewhere, the last registration wins.
- The `set_size` re-layout is best-effort: children keep their original height and split the new width evenly, which is fine for a vertical stack but may look odd in other sizer arrangements.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let labels = ["Red", "Green", "Blue"];
let rb = RadioBox::new(&frame, "Colour", &labels);
// First item is selected by default.

// Or pre-select something.
let rb2 = RadioBox::with_selection(&frame, "Size", &["S", "M", "L", "XL"], 2);

let label = StaticText::new(&frame, "Red");
let label_for_cb = label.clone();
rb.on_select(&frame, move |idx| {
    label_for_cb.set_label(labels.get(idx).copied().unwrap_or(""));
});

// Read / write programmatically.
let current = rb.get_selection();    // Option<usize>
rb.set_selection(1);
```

The whole composite is sizer-positioned by the outer `BS_GROUPBOX`
HWND — children move with it. The first radio child carries `WS_GROUP`
so the OS treats them as one mutually-exclusive group.

## See Also
- [`radio_button.rs`](radio_button.md) — single radio primitive.
- [`button.rs`](button.md), [`checkbox.rs`](checkbox.md) — sibling `BUTTON`-class controls.
- [`frame.rs`](../window/frame.md) — `register_command_handler` used by `on_select`.
- [`widget.rs`](../core/widget.md) — `Widget` trait.
