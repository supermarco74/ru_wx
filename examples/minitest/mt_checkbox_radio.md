# mt_checkbox_radio.rs

Minitest for the three selection-state widgets: [`CheckBox`](file:///f:/code/ru_wx/ru_wx/src/checkbox.rs), [`RadioBox`](file:///f:/code/ru_wx/ru_wx/src/radio_box.rs) and [`RadioButton`](file:///f:/code/ru_wx/ru_wx/src/radio_button.rs).

**Run:** `cargo run --example mt_checkbox_radio`

## Purpose
Show the three different "selected / unselected" idioms side-by-side:
- `CheckBox` — independent toggle (`on_toggle`, `is_checked`)
- `RadioBox` — pre-grouped radio set; selection by **index** (`on_select(|idx| …)`)
- `RadioButton` — manual radio group, with the first member flagged `is_group_start = true`; selection is fire-and-forget per-button (`on_select(|| …)`)

All three write their current state into a shared 1-field `StatusBar` so the visual output is uniform.

## Top-level flow
1. Build frame, 1-field `StatusBar` (label "Toggle any control.").
2. Build `CheckBox("Enable feature X")` with `set_checked(true)`; on toggle, write `feature X = <bool>` to the status bar.
3. Build `RadioBox("Priority", &["Low","Normal","High","Urgent"])` with `set_selection(1)`; on select, write the label of the chosen index.
4. Build three `RadioButton`s — first one with `is_group_start = true`; each writes its own label.
5. Stack all 8 widgets in a vertical sizer; `app.run(frame)`.

## Key APIs exercised
| Type | Call | Notes |
|---|---|---|
| `CheckBox` | `new`, `set_checked`, `is_checked`, `on_toggle` | independent |
| `RadioBox` | `new(label, &str[])`, `set_selection(idx)`, `on_select(\|idx\|)` | enum-style, pre-grouped |
| `RadioButton` | `new(label, is_group_start)`, `on_select` | manual group, first member starts the group |
| `StatusBar` | `new(&frame, n)`, `set_status_text(&str, field_idx)` | for shared state |
| `StaticText` | `new(&frame, "label:")` | per-section header |

## Patterns worth noting
- **`RadioBox` is index-based** — the callback receives `usize` and the test resolves the label itself with an array `.get(idx)`.
- **`RadioButton` is per-button** — the `is_group_start` flag tells the OS where the chain begins; siblings without the flag extend the current group.
- **Status bar as console** — every callback writes a different message, so the bar is effectively `eprintln!` for the GUI.

## Win32 notes
- `CheckBox` → `BUTTON` with `BS_AUTOCHECKBOX` style
- `RadioBox` → `BUTTON` with `BS_AUTORADIOBUTTON` style for each option, all child of the same group
- `RadioButton` → `BUTTON` with `BS_AUTORADIOBUTTON` + `WS_GROUP` for the first member
- All three send `WM_COMMAND` with `BN_CLICKED`; ru_wx de-multiplexes by control id

## Cross-references
- [`checkbox.md`](file:///f:/code/ru_wx/ru_wx/src/checkbox.md)
- [`radio_box.md`](file:///f:/code/ru_wx/ru_wx/src/radio_box.md)
- [`radio_button.md`](file:///f:/code/ru_wx/ru_wx/src/radio_button.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
