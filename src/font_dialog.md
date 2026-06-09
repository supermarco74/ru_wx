# font_dialog.rs

Modal font-chooser dialog mapped to the Win32 common dialog `ChooseFontW` (comdlg32.dll).

## Purpose
Lets the user pick a face name, point size, weight (bold), style (italic / underline / strike-out) and (optionally) a colour. On confirm the dialog returns a live [`Font`] built from a [`FontDesc`]; on cancel it returns `None`.

## Key Types
- `FontDialog` — builder; holds `parent_hwnd: HWND`, `initial: FontDesc`, `initial_colour: u32` (COLORREF `0x00BBGGRR`), `show_effects: bool`, `title: String`.

## Key Functions/Methods
- `FontDialog::new(frame: &Frame)` — pre-populated with the system default font (Segoe UI 9pt).
- `FontDialog::with_initial(frame, desc)` — start from a custom `FontDesc`.
- `FontDialog::set_initial_font(desc)` — replace the initial font description.
- `FontDialog::set_show_effects(bool)` — toggle the colour / strike-out / underline box (`CF_EFFECTS`); default `true`.
- `FontDialog::set_initial_colour(colorref: u32)` — pre-select a colour in the "Effects" picker.
- `FontDialog::set_title(&str)` — stored for cross-platform wrappers; the Windows common dialog ignores the title.
- `FontDialog::show_modal() -> Option<Font>` — runs `ChooseFontW` and converts the resulting `LOGFONTW` back into a `Font` (with a freshly created `HFONT`).

## Win32 Notes
- `CHOOSEFONTW` is built with `lStructSize`, `hwndOwner`, `lpLogFont`, `iPointSize` (1/10 pt), `Flags`, `rgbColors`. Other fields are zeroed.
- Flags used: `CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST`, plus `CF_EFFECTS` when `show_effects` is `true`.
- The `LOGFONTW.lfHeight` is computed as `-(point_size * dpi / 72)` (negative = character height). The same `MulDiv` is inverted when reading the result back, with a `.max(1)` floor.
- `lfFaceName` is copied into the 32-element fixed-size `u16` array (NUL-terminated by `std::mem::zeroed`).
- The face name is recovered as `String::from_utf16_lossy` up to the first `0` `u16`.
- Weight `>= 700` is treated as bold; `lfItalic`, `lfUnderline`, `lfStrikeOut` are read as Rust `bool`s.
- `CF_BOTH` (printer + screen) is intentionally **not** used because the printer list is empty in apps without a printer DC.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let dlg = FontDialog::new(&frame);
if let Some(font) = dlg.show_modal() {
    // font is a live Font backed by a freshly created HFONT.
    let _face = font.face_name();
    let _size = font.point_size();
    // e.g. apply it to a StaticText: label.set_font(&font);
}

// Or pre-populate the dialog with a custom initial font.
let dlg = FontDialog::with_initial(
    &frame,
    FontDesc { face: "Consolas".into(), point_size: 11, bold: true, ..Default::default() },
);
let _ = dlg.show_modal();
```

`set_show_effects(false)` hides the colour / strike-out / underline box.
`set_initial_colour(colorref)` takes a packed `0x00BBGGRR` value (use
[`Colour::to_colorref`](./geometry.md)).

## See Also
- [`font.rs`](./font.md) — `Font`, `FontDesc`, `Font::new(desc)`.
- [`color_dialog.rs`](./color_dialog.md) — colour-only picker that shares the same `set_title` cross-platform workaround.
- [`colour_picker_ctrl.rs`](./colour_picker_ctrl.md) — inline colour picker control.
- [`frame.rs`](./frame.md) — `frame.hwnd()` used as the parent.
- [`platform/win32.rs`](./platform/win32.md) — `to_wide`.
