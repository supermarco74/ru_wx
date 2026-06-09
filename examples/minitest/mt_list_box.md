# mt_list_box.rs

Minitest for [`ListBox`](file:///f:/code/ru_wx/ru_wx/src/list_box.rs) and [`CheckListBox`](file:///f:/code/ru_wx/ru_wx/src/check_list_box.rs) — list-style selection controls.

**Run:** `cargo run --example mt_list_box`

## Purpose
Show the two list variants side-by-side:
- `ListBox` — single-click + double-click callbacks
- `CheckListBox` — per-item check toggle with `(idx, checked)` callback

## Top-level flow
1. Frame 460×420 with 1-field `StatusBar` ("Click items / toggle checks.").
2. `ListBox` populated with 5 Greek letters; `on_selection_change` and `on_double_click` both update the status bar.
3. `CheckListBox` populated with 4 task items; `check(0, true)` and `check(2, true)` pre-set; `on_check_toggle(|idx, checked| …)` resolves the label via `get_string(idx)` and reports it.
4. Stack everything vertically; `app.run(frame)`.

## Key APIs exercised
- `ListBox::new(&frame)`, `ListBox::append(&str)`
- `ListBox::on_selection_change(&frame, FnOnce())`
- `ListBox::on_double_click(&frame, FnOnce())`
- `CheckListBox::new(&frame)`, `CheckListBox::append(&str)`
- `CheckListBox::check(idx: usize, checked: bool)`
- `CheckListBox::on_check_toggle(&frame, FnMut(usize, bool))`
- `CheckListBox::get_string(idx) -> Option<String>`

## Patterns worth noting
- **`ListBox` callbacks are zero-arg** — if you need the picked index, the closure must capture a clone and call `get_selection()` itself.
- **`CheckListBox` callback carries the state** — the signature is `FnMut(usize, bool)`, so the toggle reaches the closure directly with no extra look-up.
- **Two separate status messages** are used to disambiguate which list fired (`"ListBox: …"` vs `"CheckListBox: …"`).

## Win32 notes
- `ListBox` → Win32 `LISTBOX` with `LBS_NOTIFY | LBS_STANDARD`.
- `CheckListBox` → Win32 `LISTBOX` with `LBS_OWNERDRAWFIXED` + per-item `WM_DRAWITEM` + mouse handling in the subclass to flip the check bitmap.
- `LBN_DBLCLK` from the OS maps to `on_double_click`; `LBN_SELCHANGE` maps to `on_selection_change`; `CLBN_CHKCHANGE` maps to `on_check_toggle`.

## Cross-references
- [`list_box.md`](file:///f:/code/ru_wx/ru_wx/src/list_box.md)
- [`check_list_box.md`](file:///f:/code/ru_wx/ru_wx/src/check_list_box.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
