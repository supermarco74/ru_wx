# `icon_tray_demo.rs` — system-tray icon demo

## Purpose
Showcases the **`IconTray`** widget — the `wxTaskBarIcon` port. A small
icon lives in the Windows system tray (notification area). The demo
registers left/right-click, double-click, and balloon-click callbacks.

## Run
```bash
cargo run --example icon_tray_demo
```

## What it shows
- `IconTray::new(parent, svg_bytes, size_px)` — create from an embedded SVG
- `set_tooltip(text)` — hover hint
- `set_menu(menu)` — right-click context menu
- `show()` / `hide()` — visibility toggling
- `show_balloon(title, text, icon)` — Windows 10+ toast-style balloon
- Four callback hookups:
  - `on_left_click`
  - `on_left_double_click`
  - `on_right_click`
  - `on_balloon_click`

## Embedded assets
- `STAR_SVG` from `assets/icons/star.svg` (16×16)

## Top-level flow
1. Build a hidden 320×180 frame (the window is just a host — it can be
   hidden via `frame.show(false)` but must exist for the tray to attach).
2. Build a File menu with one "Exit" item.
3. Create `IconTray::new(&frame, STAR_SVG, 16)`.
4. Wire all four callbacks; each prints / sets a status string.
5. Build a small "Show / Hide / Balloon" button row that mutates the
   tray via the shared `Rc<RefCell<IconTray>>`.
6. `app.run(frame)`.

## Key APIs exercised
- `IconTray::new(&frame, STAR_SVG, 16)` — returns an `IconTray` that owns
  its `NOTIFYICONDATAW` shell registration.
- `set_tooltip(&str)` — sets `NOTIFYICONDATAW.szTip` (TCHAR limit: 128).
- `set_menu(Menu)` — assigns the context menu; rendered via `WM_CONTEXTMENU`.
- `show_balloon(&str, &str, BalloonIcon) -> bool` — Win10+; returns
  `false` on older Windows where the balloon API is unavailable.
- `BalloonIcon::Info | Warning | Error | None` — enum for the icon.
- `Rc<RefCell<IconTray>>` — shared ownership pattern: one reference held
  by the button row, one by the tray-internal WM_COMMAND dispatcher.

## Win32 / platform notes
- Implements `Shell_NotifyIconW` with `NIF_MESSAGE | NIF_ICON | NIF_TIP`.
- Uses `RegisterWindowMessageW("TaskbarCreated")` to re-register the icon
  when Explorer.exe restarts.
- Balloon API requires Windows 10 build 19041+ (the 21H1 feature floor);
  the wrapper returns `Ok(false)` on older releases.
- The tray icon must outlive the callbacks — the demo uses
  `let _tray_ref = tray;` in `main`'s scope to keep it alive.

## Cross-references
- See `src/icon_tray.rs` for the `IconTray` struct
- See `src/icon.rs` for SVG-to-HICON conversion
- See `src/menu.rs` for the right-click context menu
