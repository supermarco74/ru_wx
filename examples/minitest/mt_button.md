# mt_button.rs

Minitest that exhaustively exercises the [`Button`](file:///f:/code/ru_wx/ru_wx/src/button.rs) widget — six different forms in a single window.

**Run:** `cargo run --example mt_button`

## Purpose
Walk through every public way to construct a `Button`:
1. Plain text (`Button::new`)
2. Coloured bitmap (`Button::new_with_bitmap`)
3. Inline-SVG icon (`Button::new_with_svg_bytes`)
4. Asset-file SVG icon (`include_bytes!` + `new_with_svg_bytes`)
5. Disabled (`borrow_mut().set_enabled(false)`)
6. Self-updating label (closure captures `btn.clone()` and `Rc<Cell<u32>>`)

## Embedded assets
| Const | Source | Purpose |
|---|---|---|
| `STAR_SVG` | `assets/icons/star.svg` | Bootstrap-icons star (file) |
| `INFO_SVG` | inline `br#"…"#` | Info-circle glyph (literal) |

Both are decoded into HBITMAPs internally by `new_with_svg_bytes`.

## Top-level flow
1. Build `App` + 420×380 `Frame`.
2. Create a shared `StaticText` for status messages.
3. For each button form, clone `status` and install an `on_click` callback that writes a distinct label.
4. Stack all 7 widgets (status + 6 buttons) into a `BoxSizer::vertical()`.
5. `frame.set_sizer(sizer)` then `app.run(frame)`.

## Key APIs exercised
- [`Button::new(&frame, label)`](file:///f:/code/ru_wx/ru_wx/src/button.rs)
- [`Button::new_with_bitmap(&frame, label, Colour, w, h)`](file:///f:/code/ru_wx/ru_wx/src/button.rs)
- [`Button::new_with_svg_bytes(&frame, &[u8], size)`](file:///f:/code/ru_wx/ru_wx/src/button.rs)
- `Button::on_click(&frame, FnOnce())`
- `Button::set_label(&str)`
- `Button::as_widget_ref() -> WidgetRef` (via the widget handle)
- `Widget::set_enabled(bool)` reached through `borrow_mut()`
- `StaticText::set_label` for the shared status line
- `BoxSizer::vertical()` + `sizer.add(widget_ref)`

## Patterns worth noting
- **Disabled button** — `btn_disabled.as_widget_ref().borrow_mut().set_enabled(false)` is the only Win32 way; the click handler never fires after that.
- **Self-referential closure** — `let btn_self_clone = btn_self.clone(); … let counter = Rc::new(Cell::new(0u32));` then the closure mutates the same button that owns it. No `RefCell` is needed because we use `Cell<u32>` (copy type).
- **Status sharing** — `StaticText` is `Clone`; clones are passed into each closure so the original stays alive in the sizer.

## Win32 notes
- Native `BUTTON` control (`BS_PUSHBUTTON` for text, `BS_OWNERDRAW` for bitmap variants).
- Bitmap form sends `BM_SETIMAGE` and stores the HBITMAP in the widget's `userdata`.
- SVG bytes go through `resvg` + `tiny_skia` and end up as a 32-bit DIB section (BGRA).

## Cross-references
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md) — public API of `Button`
- [`colour.md`](file:///f:/code/ru_wx/ru_wx/src/colour.md) — `Colour::new(r, g, b, a)`
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md) — `StaticText`
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md) — `BoxSizer::vertical`
- [`assets/icons/star.svg`](file:///f:/code/ru_wx/ru_wx/assets/icons/star.svg) — SVG fixture
