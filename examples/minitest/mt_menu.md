# mt_menu.rs

Minitest for [`MenuBar`](file:///f:/code/ru_wx/ru_wx/src/menu.rs) — File / Edit / Help with every common item kind.

**Run:** `cargo run --example mt_menu`

## Purpose
Demonstrate every flavour of `Menu` item in a single menubar:
- Plain menu item
- Disabled item
- Item with SVG icon
- Item with keyboard shortcut ([`Accelerator`](file:///f:/code/ru_wx/ru_wx/src/accelerator.rs))
- Checkable item
- Radio item group
- Separators

## Embedded assets
| Const | Source |
|---|---|
| `FILE_NEW_SVG` | `assets/icons/file-new.svg` |
| `FOLDER_OPEN_SVG` | `assets/icons/folder-open.svg` |
| `EXIT_SVG` | `assets/icons/exit.svg` |
| `INFO_SVG` | `assets/icons/info.svg` |
| `STAR_SVG` | `assets/icons/star.svg` |

## Top-level flow
1. Frame 520×320, hint `StaticText`, 1-field `StatusBar`.
2. **File menu** — `Menu::new("&File")`:
   - `append_with_svg_icon("&New", FILE_NEW_SVG, 16, …)`
   - `append_with_shortcut("&Open…", Accelerator::parse("Ctrl+O").unwrap(), …)`
   - `append_with_svg_icon("Open &recent", FOLDER_OPEN_SVG, 16, …)`
   - `append_with_svg_icon("Star (favourite)", STAR_SVG, 16, …)`
   - `append_separator()`
   - `append_disabled("Save (disabled)")`
   - `append_with_svg_icon("E&xit", EXIT_SVG, 16, …)` — closure calls `frame_for_exit.close()`
3. **Edit menu** — `Menu::new("&Edit")`:
   - `append_check_item("Word &wrap", …)`
   - `append_separator()`
   - Three `append_radio_item` calls for "Zoom &50%" / "&100%" / "&200%"
4. **Help menu** — `Menu::new("&Help")`:
   - `append_with_svg_icon("&About…", INFO_SVG, 16, …)`
5. Build a `MenuBar::new()`, `menubar.append(file_menu)`, …, `menubar.append(help_menu)`, then `frame.set_menu_bar(menubar)`.
6. `app.run(frame)`.

## Key APIs exercised
- `Menu::new(label)`, `Menu::append`, `Menu::append_with_svg_icon`, `Menu::append_with_shortcut`
- `Menu::append_separator`, `Menu::append_disabled`
- `Menu::append_check_item`, `Menu::append_radio_item`
- [`Accelerator::parse(&str) -> Result<Accelerator, _>`](file:///f:/code/ru_wx/ru_wx/src/accelerator.rs) — accepts "Ctrl+Shift+S" etc.
- `MenuBar::new()`, `MenuBar::append(menu)`
- `Frame::set_menu_bar(menubar)`

## Patterns worth noting
- **`&` in the label** marks the Windows-style accelerator key (e.g. `"&File"` highlights `F`).
- **`append_with_shortcut` is purely descriptive** — it does not install the accelerator. To make `Ctrl+O` actually fire, the menu item must be on a menubar attached to a frame that pumps the message loop; ru_wx picks up `HACCEL` from the menu and translates it before the WM_COMMAND.
- **`append_radio_item` groups by adjacency** — calling it three times in a row makes a single Win32 radio group; a separator or non-radio item ends the group.
- **Frame is `Clone`** — the `frame_for_exit` capture is required so the closure can call `frame.close()`.

## Win32 notes
- Native Win32 menus built from `CreateMenu` + `AppendMenuW` / `InsertMenuItemW`.
- SVG icons are rendered to HBITMAPs and added to the menu with `MENUITEMINFOW` + `HBMMENU_CALLBACK`.
- `WM_COMMAND` for menu items carries the item id; ru_wx dispatches to the registered closure via a per-frame id → closure map.
- `WM_MENUSELECT` provides hot-tracking info; the menubar is repainted on hover.

## Cross-references
- [`menu.md`](file:///f:/code/ru_wx/ru_wx/src/menu.md)
- [`accelerator.md`](file:///f:/code/ru_wx/ru_wx/src/accelerator.md)
- [`frame.md`](file:///f:/code/ru_wx/ru_wx/src/frame.md) — `set_menu_bar`
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
- [`assets/icons/`](file:///f:/code/ru_wx/ru_wx/assets/icons) — SVG fixtures
