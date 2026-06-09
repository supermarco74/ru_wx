# top_level_window.rs

`wxTopLevelWindow` analog. A composition wrapper around `Frame` that exposes the higher-level "window operations" surface (iconize, maximize, full-screen, user attention, screen-centring, min/max/default sizes).

## Purpose
Many of the methods on a top-level window are not really "frame-only" — they apply equally to a frame, a dialog, or any future MDI parent. This module centralises those operations as a struct that owns a `Frame` so callers can use a single uniform API regardless of the underlying window class. It also covers the screen / monitor helpers (primary monitor size, work area).

## Key Types
- `TopLevelWindow` — public struct wrapping a `Frame` by value.
- `CentreDirection` — `Screen`, `Horizontal`, `Vertical`, `Both`. Used by `centre`.
- `FullScreenStyle` — currently `Default` (mirrors the wxWidgets bitflag surface; only the default is meaningful in this build).
- `UserAttentionFlags` — `Default` (one-shot flash) and `Continuous` (flash until the window is foregrounded).

## Key Methods
- `TopLevelWindow::new(title: &str, w: u32, h: u32) -> Self` — Build a new top-level window backed by a freshly-constructed `Frame`.
- `from_builder(builder: FrameBuilder) -> Self` — Wrap a pre-configured builder.
- `into_frame(self) -> Frame` — Unwrap back to a raw `Frame`.
- `frame(&self) -> &Frame` — Borrow the underlying frame.
- `hwnd(&self) -> HWND` — Windows-only.
- `get_title(&self) -> String` / `set_title(&self, title: &str)`.
- `set_icon(&self, hicon: HICON)` / `get_icon(&self) -> HICON` — `WM_SETICON` / `WM_GETICON` with `ICON_BIG=1` and `ICON_SMALL=0`.
- `show(&self)`, `hide(&self)`, `close(&self)`.
- `iconize(&self)` / `is_iconized(&self) -> bool` — `ShowWindow(SW_MINIMIZE)` / `IsIconic`.
- `maximize(&self)` / `is_maximized(&self) -> bool` — `ShowWindow(SW_MAXIMIZE)` / `IsZoomed`.
- `restore(&self)` — `ShowWindow(SW_RESTORE)`.
- `show_full_screen(&self, show: bool, style: FullScreenStyle)`, `is_full_screen(&self) -> bool`.
- `set_min_size(&self, w: u32, h: u32)`, `set_max_size(&self, w: u32, h: u32)`, `set_default_size(&self, w: u32, h: u32)`.
- `centre(&self, direction: CentreDirection)` — Centred on screen / horizontally / vertically.
- `request_user_attention(&self, flags: UserAttentionFlags)` — `FlashWindowEx` with `FLASHW_TRAY | FLASHW_TIMERNOFG` (`0x00000002 | 0x0000000C`).
- `get_primary_monitor_size() -> (i32, i32)` — `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`. Static.
- `get_work_area() -> RECT` — `SystemParametersInfoW(SPI_GETWORKAREA=0x0030, ...)`. Static.

## Win32 Notes
- Constants used: `ICON_BIG = 1`, `ICON_SMALL = 0`, `SWP_FRAMECHANGED = 0x0020`, `FLASHW_TRAY = 0x00000002`, `FLASHW_TIMERNOFG = 0x0000000C`, `SPI_GETWORKAREA = 0x0030`.
- `request_user_attention` is the platform equivalent of `wxTopLevelWindow::RequestUserAttention`. The "continuous" mode keeps flashing the taskbar button until the window becomes foreground.
- The composition-over-Frame pattern means the inner `Frame` is **owned**, not borrowed. `into_frame` consumes `self` to release ownership back to a plain `Frame` if needed (e.g. before calling `Frame::show()` to enter the Win32 message loop).
- `show_full_screen` currently uses the `FullScreenStyle::Default` placeholder; the bitflag surface exists so future styles (no-caption, no-border, etc.) can be added without an API break.

## Quick start

```rust
use ru_wx::prelude::*;

// Build a top-level window — it owns a Frame internally.
let w = TopLevelWindow::new("My App", 800, 600);
w.set_min_size(400, 300);
w.set_max_size(1600, 1200);
w.centre(CentreDirection::Both);

// Window-state changes.
w.maximize();
assert!(w.is_maximized());
w.iconize();
w.restore();

// Flash the taskbar to ask for attention (e.g. on a long background op).
w.request_user_attention(UserAttentionFlags::Continuous);

// Screen helpers.
let (sw, sh) = TopLevelWindow::get_primary_monitor_size();
let work = TopLevelWindow::get_work_area();

// When you're ready to enter the message loop, unwrap the Frame.
let frame = w.into_frame();
let app  = App::new();
app.run(frame);
```

`TopLevelWindow` is a **wrapper** — it owns the underlying `Frame`. Call
`into_frame` to recover the raw `Frame` (consumes `self`) when you
need to hand it to `App::run`.

## See Also
- [`frame.rs`](./frame.md) — the underlying window this composes
- [`dialog.rs`](./dialog.md) — sibling top-level window; uses its own message loop
- [`dpi.rs`](./dpi.md) — used by `Frame::dpi` / `Frame::scale_factor`; not directly wrapped here but commonly paired
