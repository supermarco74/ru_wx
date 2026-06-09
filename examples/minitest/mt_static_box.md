# mt_static_box.rs

Minitest for [`StaticBox`](file:///f:/code/ru_wx/ru_wx/src/static_box.rs) — labelled box container.

**Run:** `cargo run --example mt_static_box`

## Purpose
Demonstrate the three `StaticBox` constructors (`new`, `new_empty`, `with_size`) plus the `set_label` / `get_label` round-trip. Children can be reparented to a `StaticBox` by passing it as the parent in their own constructor; here we reparent a `StaticText` for the visual grouping effect.

## Top-level flow
1. Frame 420×280.
2. **(1)** `let box1 = StaticBox::new(&frame, "Group A");` — `assert_eq!(box1.get_label(), "Group A");` then `StaticText::new(&box1, "(child of Group A)")` reparented to the box.
3. **(2)** `let box2 = StaticBox::new_empty(&frame);` — `assert_eq!(box2.get_label(), "");` → `box2.set_label("Group B (renamed)");` → `assert_eq!(box2.get_label(), "Group B (renamed)");` then a reparented child.
4. **(3)** `let box3 = StaticBox::with_size(&frame, "Group C (sized)", 200, 80);` — `assert_eq!(box3.get_label(), "Group C (sized)");` (no reparented child, just the box itself).
5. **(4)** Reuse a label after the fact: `let box4 = StaticBox::new(&frame, "Original");` → `box4.set_label("Updated");` → `assert_eq!(box4.get_label(), "Updated");`.
6. Stack all 4 boxes in a vertical sizer; the reparented child texts ride along via the box's HWND parent.
7. `app.run(frame)`.

## Key APIs exercised
- [`StaticBox::new(&frame, label)`](file:///f:/code/ru_wx/ru_wx/src/static_box.rs)
- `StaticBox::new_empty(&frame)`
- `StaticBox::with_size(&frame, label, w, h)`
- `StaticBox::set_label(&str)`
- `StaticBox::get_label() -> String`

## Patterns worth noting
- **`assert_eq!` at construction time** is the canonical way to lock in the round-trip behaviour in minitests — the assertions run before `app.run`, so they verify the value before the user can interact with the control.
- **Reparenting via parent argument** — the reparented child is created with `&box1` as its parent, so it lives inside `box1`'s HWND. No `set_parent` call is needed.
- **`new_empty` is useful for "create now, label later"** — the label is mutable via `set_label`; `new(label)` is sugar that calls `set_label` for you.

## Win32 notes
- `StaticBox` is a native `BUTTON` control with `BS_GROUPBOX` style; the OS draws the etched border and the label.
- The label is stored in the control's window text and re-issued on each `set_label` via `SetWindowTextW`.
- Children created with the box as parent are re-parented via the standard `CreateWindowExW` `hwndParent` argument, so they clip to the box's client area.

## Cross-references
- [`static_box.md`](file:///f:/code/ru_wx/ru_wx/src/static_box.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md) — the reparented child
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md) — `BoxSizer::vertical` stack
