# mt_text_ctrl.rs

Minitest for [`TextCtrl`](file:///f:/code/ru_wx/ru_wx/src/text_ctrl.rs) — single-line, multiline, password, and read-only fields, all in one frame.

**Run:** `cargo run --example mt_text_ctrl`

## Purpose
1. `TextCtrl::new` — single-line field with a default value
2. `TextCtrl::multiline` — multi-line field pre-filled with three lines
3. `TextCtrl::password` — masked entry (echoes `*`)
4. `set_readonly(true)` — turn an editable control into a non-editable one
5. `TextCtrl::append_text(&str)` — append a line to a multiline control
6. `on_change` — react live to typing in a single-line field

## Top-level flow
1. Frame 560×460.
2. 1-field `StatusBar` with hint `"Type into the fields."`.
3. **Field 1 — Single-line**
   - `StaticText` label "Single-line:"
   - `TextCtrl::new(&frame, "Hello world")`
   - `on_change` closure formats `{:?}` of `get_value()` and writes to the status bar so the user sees the live content
4. **Field 2 — Multiline**
   - `StaticText` label "Multiline (3+ lines):"
   - `TextCtrl::multiline(&frame, "First line\nSecond line\nThird line — type freely.")` — `multiline` is a separate constructor, not a flag on `new`
5. **Field 3 — Password**
   - `StaticText` label "Password:"
   - `TextCtrl::password(&frame, "")` — empty default; the field echoes bullets
6. **Field 4 — Read-only**
   - `StaticText` label "Read-only:"
   - `TextCtrl::new(&frame, "you cannot edit me")` followed by `ro.set_readonly(true)` — same `new` constructor as field 1, then locked
7. **Button — Append line**
   - `Button::new(&frame, "Append line to multiline")`
   - Backed by `Rc<Cell<u32>>` so the click counter survives across the move-closure
   - On click: `counter.set(counter.get() + 1); multi.append_text(&format!("\nappended #{}", counter.get()));`
8. **Button — Show password value**
   - `Button::new(&frame, "Show password value")`
   - On click: `set_status_text(&format!("Password = {:?}", pwd.get_value()), 0)` — confirms the unmasked value is reachable from `get_value` even when the field is masked on screen
9. Vertical `BoxSizer` with all labels/fields/buttons stacked in order; `app.run(frame)`.

## Key APIs exercised
- [`TextCtrl::new(&frame, &str)`](file:///f:/code/ru_wx/ru_wx/src/text_ctrl.rs) — single-line, default value
- `TextCtrl::multiline(&frame, &str)` — multi-line constructor
- `TextCtrl::password(&frame, &str)` — masked constructor
- `TextCtrl::get_value() -> String`
- `TextCtrl::set_readonly(bool)`
- `TextCtrl::append_text(&str)`
- `TextCtrl::on_change(&frame, FnClosure)` — fires on every keystroke
- [`StaticText::new`](file:///f:/code/ru_wx/ru_wx/src/static_text.rs)
- [`Button::new`](file:///f:/code/ru_wx/ru_wx/src/button.rs) + `on_click`
- [`StatusBar::new` / `set_status_text`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs)
- [`BoxSizer::vertical()` / `add`](file:///f:/code/ru_wx/ru_wx/src/sizer.rs)
- `Rc<Cell<u32>>` for the click counter

## Patterns worth noting
- **Three constructors, one trait** — `new`, `multiline`, `password` all return the same `TextCtrl` type. There is no enum / flag to switch styles later; you pick the style at construction and it is sticky.
- **`on_change` fires on every keystroke** — for the single-line field, every character the user types calls the closure and rewrites the status bar. For long buffers, throttle upstream.
- **Read-only is a one-liner** — `set_readonly(true)` on an existing field toggles the `ES_READONLY` window style. The field is otherwise identical to a single-line one; you can mix and match by constructing normal fields and locking the ones you want immutable.
- **Append, don't replace, for multiline** — `append_text` adds to the existing buffer; it does **not** clear first. Use a fresh `multiline` constructor (or a hypothetical `set_text`) for full replacement.
- **`get_value` returns the underlying text, password mask or not** — the password field's `get_value` is the actual password the user typed, not the bullet sequence. Use it for validation, never for display.
- **`Rc<Cell<u32>>` for a click counter** — `Cell` is enough for `u32` because there's no `&u32` borrow to share; using `RefCell` would only add runtime borrow-check overhead.

## Win32 notes
- `TextCtrl::new` creates a `WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL` `EDIT` control.
- `multiline` adds `ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN` and omits the default `ES_AUTOHSCROLL`.
- `password` adds `ES_PASSWORD` (echoes the system password character, usually `*`).
- `set_readonly` adds or removes `ES_READONLY` via `SetWindowLongPtrW` + `GWL_STYLE`.
- `append_text` issues `EM_SETSEL` to position the caret at the end, then `EM_REPLACESEL` with the new text.
- `on_change` registers a `WM_COMMAND` / `EN_CHANGE` filter on the parent frame.
- `get_value` issues `WM_GETTEXT` with a buffer that grows until the wide-string fits, then converts to a Rust `String`.

## Cross-references
- [`text_ctrl.md`](file:///f:/code/ru_wx/ru_wx/src/text_ctrl.md)
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md)
- [`mt_status_bar_input.md`](file:///f:/code/ru_wx/ru_wx/examples/minitest/mt_status_bar_input.md) — companion: feeds `TextCtrl::get_value` into a `StatusBar`
