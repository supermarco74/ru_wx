# `window_with_button.rs` — simplest end-to-end demo

## Purpose
A **minimal smoke test** of the library. Builds a single window, embeds
a label, a button, a menu bar with a File menu, and runs the event loop.
Useful as a starting template: copy, rename, and start adding widgets.

## Run
```bash
cargo run --example window_with_button
```

## What it shows
- `App::new()` — application object / message-loop driver
- `Frame::builder().with_title(...).with_size(...).build()` — top-level window
- `StaticText` — read-only label
- `Button::new_with_svg_bytes(parent, svg_bytes, px)` — button with an embedded SVG icon
- `Menu::new(label)`, `menu.append_with_svg_icon(label, svg, px, parent, cb)`, `Menu::append_disabled(label)`
- `MenuBar::new()` + `menubar.append(menu)` + `frame.set_menu_bar(menubar)`
- `frame.show()` / `app.run(frame)` — event loop

## Embedded assets
Uses `include_bytes!` to embed 4 SVG icons at compile time from
`assets/icons/`:

| Constant     | Path                            | Used for              |
|--------------|---------------------------------|-----------------------|
| `STAR_SVG`   | `assets/icons/star.svg`         | the button's icon     |
| `FILE_NEW`   | `assets/icons/file-new.svg`     | "New" menu item       |
| `FOLDER_OPEN`| `assets/icons/folder-open.svg`  | "Open" menu item      |
| `EXIT`       | `assets/icons/exit.svg`         | "Exit" menu item      |

## Top-level flow
1. `App::new()` — initialise the Win32 subsystem.
2. Build a 480×320 frame.
3. Create a vertical `BoxSizer`.
4. Create a `StaticText` and a `Button`; add both to the sizer; apply the
   sizer to the frame.
5. Build a File menu with three SVG-icon items + one disabled item;
   wrap in a `MenuBar`; attach to the frame.
6. Wire the "Exit" menu item to call `frame.close()`.
7. `frame.show()` (implicit via `app.run`) and enter the message loop.

## Key APIs exercised
- `Frame::builder()` — fluent constructor
- `StaticText::new(parent, text)` + `as_widget_ref()` for sizer insertion
- `Button::new_with_svg_bytes(parent, svg, size_px)` — SVG → HBITMAP → button
- `Menu::append_with_svg_icon(...)` — menu item with icon
- `Menu::append_disabled(...)` — greyed-out item
- `MenuBar::new()` + `menubar.append(menu)`
- `app.run(frame)` — blocks until the window is closed

## Win32 / platform notes
- `#![windows_subsystem = "windows"]` — hides the console window in release.
- All widgets are real native Win32 controls (no GDI user-draw).
- The button uses `BS_ICON` style with an `HICON` derived from the SVG.

## Cross-references
- See `src/app.rs` for `App::new` / `app.run`
- See `src/frame.rs` for the builder
- See `src/button.rs` for `new_with_svg_bytes`
- See `src/menu.rs` for menu construction
- See `src/static_text.rs` for the label
