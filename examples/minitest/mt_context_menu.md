# mt_context_menu.rs

Minitest for [`PopupMenu`](file:///f:/code/ru_wx/ru_wx/src/popup_menu.rs) — context menus shown from a button click.

**Run:** `cargo run --example mt_context_menu`

## Purpose
Show the two ways to launch a popup:
1. **At cursor** — `popup.popup(&frame)` — `TrackPopupMenu` with `TPM_LEFTALIGN | TPM_TOPALIGN` and current cursor pos
2. **At fixed screen coords** — `popup.popup_at(&frame, x, y)` — same call but with explicit `x, y`

Also exercises every flavour of popup item: plain, separator, colour-icon, checkable and disabled.

## Top-level flow
1. Frame 460×280 with a hint `StaticText` + 1-field `StatusBar`.
2. **Button 1** — "Open menu at cursor":
   - Build a `PopupMenu`
   - Append "Cut" / "Copy" / "Paste" (plain)
   - `append_separator()`
   - `append_with_colour_icon("Mark in red", Colour::new(220,60,60,255), …)` — coloured bitmap on the left
   - `append_check_item("Pin to top", …)` — checkable toggle
   - `append_separator()`
   - `append_disabled("Disabled item")`
   - `popup.popup(&frame)`
3. **Button 2** — "Open menu at (100, 100)": 2-item popup launched with `popup_at(&frame, 100, 100)`.
4. Both buttons in a vertical sizer; `app.run(frame)`.

## Key APIs exercised
- [`PopupMenu::new()`](file:///f:/code/ru_wx/ru_wx/src/popup_menu.rs)
- `popup.append(&str, &frame, FnOnce())`
- `popup.append_separator()`
- `popup.append_with_colour_icon(&str, Colour, &frame, FnOnce())`
- `popup.append_check_item(&str, &frame, FnOnce())`
- `popup.append_disabled(&str)`
- `popup.popup(&frame)` — at cursor
- `popup.popup_at(&frame, x, y)` — at fixed coords

## Patterns worth noting
- **Per-item closure clone** — the `status.clone()` inside each `append` call is required because the closure outlives the original `status` binding.
- **`append_disabled` needs no closure** — the item never fires, so there is nothing to register.
- **All callbacks are fire-and-forget** — popups don't carry "the user picked X" state. If you need that, the closure must capture mutable state (e.g. `Rc<RefCell<Option<usize>>>`) and stash the index.

## Win32 notes
- `CreatePopupMenu` → `HMENU`; items added with `AppendMenuW` / `InsertMenuItemW`.
- `TrackPopupMenuEx` with `TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RIGHTBUTTON`.
- `popup_at` sets the menu's origin to the supplied `(x, y)` in **client coordinates** of the supplied frame; ru_wx converts to screen coords via `ClientToScreen` before the call.
- Coloured-icon item uses `HBMMENU_CALLBACK` — Win32 asks the menu's owner-draw proc to paint the bitmap; ru_wx caches a per-menu HBITMAP list keyed by id.

## Cross-references
- [`popup_menu.md`](file:///f:/code/ru_wx/ru_wx/src/popup_menu.md)
- [`colour.md`](file:///f:/code/ru_wx/ru_wx/src/colour.md) — `Colour::new`
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
