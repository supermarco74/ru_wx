# icon_tray

`IconTray` — wxWidgets-style "task bar icon" (system tray / notification area). Backed by
Win32 `Shell_NotifyIconW` with `NOTIFYICON_VERSION_4` for modern notification codes
(`NIN_SELECT`, `NIN_POPUPOPEN`, `NIN_BALLOONUSERCLICK`).

## When to use

- Background apps that need to live in the notification area.
- Apps that need balloon / toast notifications (`show_balloon`).
- Apps that want a context menu (right-click) attached to a tray icon.

## Public types

```rust
/// Visual style for `show_balloon`.
#[derive(Clone, Copy, Debug)]
pub enum BalloonIcon {
    None,     // no icon
    Info,     // blue
    Warning,  // yellow
    Error,    // red
    User,     // the tray icon itself, large
}

#[derive(Clone)]
pub struct IconTray { /* opaque; see "Lifetime" below */ }
```

## Public API (Windows)

```rust
impl IconTray {
    /// Create a tray icon and add it to the notification area.
    /// `svg_bytes` is rendered at `icon_size × icon_size` and used
    /// as the icon. Returns `None` if SVG rendering fails.
    pub fn new(frame: &Frame, svg_bytes: &[u8], icon_size: u32) -> Option<Self>;

    /// Create an icon that is *not* added to the tray yet — uses a
    /// 1×1 placeholder `HICON`. Configure it (icon / tooltip / menu)
    /// then call `show()` to add it. Useful when you want to defer
    /// the tray insert until after setup is complete.
    pub fn hidden(frame: &Frame) -> Self;

    /// Replace the current icon. Old `HICON` is destroyed. Issues
    /// `NIM_MODIFY` if the tray is currently shown.
    pub fn set_icon_from_svg_bytes(&mut self, svg_bytes: &[u8], icon_size: u32) -> bool;

    /// Tooltip shown on hover. Triggers `NIM_MODIFY` if shown.
    pub fn set_tooltip(&mut self, tooltip: &str);

    /// Attach a context `Menu` shown on right-click / `NIN_POPUPOPEN`.
    /// The menu's items must already have their click handlers
    /// registered (via `Menu::append`).
    pub fn set_menu(&mut self, menu: Menu);

    /// Add the icon to the tray (no-op if already shown).
    pub fn show(&mut self) -> bool;

    /// Remove the icon (it can be re-added with `show()`).
    pub fn hide(&mut self);

    /// Pop a balloon / toast notification. Returns `true` on success.
    /// Requires the tray to be currently shown.
    pub fn show_balloon(&self, title: &str, text: &str, icon: BalloonIcon) -> bool;

    /// Callback for `WM_LBUTTONUP` / `NIN_SELECT`.
    pub fn on_left_click<F: FnMut() + 'static>(&mut self, callback: F);
    /// Callback for `WM_LBUTTONDBLCLK`.
    pub fn on_left_double_click<F: FnMut() + 'static>(&mut self, callback: F);
    /// Callback for `WM_RBUTTONDOWN` / `WM_CONTEXTMENU` / `NIN_POPUPOPEN`.
    /// Fired *after* the context menu is shown (if any).
    pub fn on_right_click<F: FnMut() + 'static>(&mut self, callback: F);
    /// Callback for `NIN_BALLOONUSERCLICK` (user clicked the balloon).
    pub fn on_balloon_click<F: FnMut() + 'static>(&mut self, callback: F);

    /// The current `NOTIFYICONDATAW.uID` (a per-process incrementing id).
    pub fn id(&self) -> u32;
}
```

## Public API (non-Windows stubs)

On non-Windows targets all methods compile and are safe to call but no-ops. `new` returns
`None`. `on_*` accept but discard callbacks. `id` returns `0`. This lets cross-platform code
build everywhere.

## Lifetime

- `IconTray::new` / `hidden` both register a `WM_APP + n` callback handler on the **parent
  Frame**. The handler is keyed by `self.msg` (a process-global monotonic counter starting at
  `0x8001`) and routes the tray events to the four user callbacks.
- `Drop` calls `Shell_NotifyIconW(NIM_DELETE)` (if currently shown), destroys the `HICON`,
  and unregisters the message handler from the frame.
- A tray icon is forever tied to its frame — dropping the frame will leave the icon orphaned
  unless the `IconTray` is dropped first.

## Quick start

A complete, copy-pasteable "background app" example: a tray icon with a
right-click context menu, a left-click toggle for a settings window, and
a balloon notification on startup.

```rust,no_run
use ru_wx::prelude::*;

fn install_tray(frame: &Frame) -> Option<IconTray> {
    // 1. Create the tray icon from an SVG. Returns None if SVG rendering fails.
    let svg = include_bytes!("../../assets/icons/star.svg");
    let mut tray = IconTray::new(frame, svg, 16)?;

    // 2. Tooltip on hover.
    tray.set_tooltip("My App — right-click for menu");

    // 3. Build a right-click context menu (re-used across shows).
    let mut menu = Menu::new("&Tray");
    let show_id = menu.append("&Show window", || {
        println!("show window requested");
    });
    let hide_id = menu.append("&Hide window", || {
        println!("hide window requested");
    });
    menu.append_separator();
    let quit_id = menu.append_with_shortcut(
        "&Quit",
        Accelerator::new(KeyCode::Q, Modifiers::CTRL),
        || std::process::exit(0),
    );
    tray.set_menu(menu);

    // 4. Click callbacks. on_right_click fires *after* the context menu is shown.
    tray.on_left_click(|| {
        println!("left click on tray icon");
    });
    tray.on_double_click(|| {
        println!("double click on tray icon");
    });
    tray.on_balloon_click(|| {
        println!("user clicked the balloon");
    });

    // 5. Pop a one-shot balloon notification.
    tray.show_balloon(
        "My App is running",
        "Click the icon for the menu, or close to exit.",
        BalloonIcon::Info,
    );

    Some(tray)
}
```

**Typical workflow**

1. Create the tray with `IconTray::new(frame, svg_bytes, size)`. Use
   `IconTray::hidden(frame)` if you want to defer `show()` until after
   you've configured icon, tooltip, and menu.
2. Configure the icon with `set_icon_from_svg_bytes`, the hover tooltip
   with `set_tooltip`, and (optionally) a right-click context menu with
   `set_menu`. All of these are no-ops until you call `show()`.
3. Call `show()` to add the icon to the notification area. Use `hide()`
   to remove it without dropping the `IconTray` (so you can re-`show()`).
4. Register click callbacks with `on_left_click` / `on_double_click` /
   `on_right_click` / `on_balloon_click`. Right-click fires *after* the
   context menu has been shown.
5. Pop balloon / toast notifications with `show_balloon(title, text, icon)`.
   The icon parameter is the visual style (`Info` / `Warning` / `Error` /
   `User` / `None`).
6. Keep the `IconTray` alive for as long as the icon should be in the
   tray. **Drop the `IconTray` before the parent `Frame`** — `Drop`
   issues `NIM_DELETE` and unregisters the message handler.

**Notes**

- The tray uses **`NOTIFYICON_VERSION_4`**, so you get the modern
  `NIN_SELECT` / `NIN_POPUPOPEN` / `NIN_BALLOONUSERCLICK` codes (not
  the legacy `WM_USER` ones).
- The icon's `id()` is a per-process monotonic `u32` (from
  `NEXT_TRAY_UID`); the message is a per-process monotonic `WM_APP + n`
  (from `NEXT_TRAY_MSG`). Don't hardcode either value.
- The 1×1 placeholder HICON used by `hidden()` is real but transparent;
  the user will see a blank slot until they call `set_icon_from_svg_bytes`
  and `show()`.
- `show_balloon` requires the tray to currently be shown. On Windows 10+
  the OS may also surface the balloon as a system toast.
- Cross-platform: on non-Windows targets the type compiles and all
  methods are safe no-ops (with `new` returning `None` and `id` returning
  `0`). This is the same pattern as `Frame::set_drop_files_callback`.

## Win32 notes

- `NEXT_TRAY_UID` (atomic, starts at 1) and `NEXT_TRAY_MSG` (atomic, starts at `WM_APP + 1`)
  are process-globals. Each new `IconTray` consumes one of each.
- Uses `NOTIFYICON_VERSION_4`: without an explicit `NIM_SETVERSION` after `NIM_ADD`, the
  shell delivers only the legacy `WM_USER` codes.
- `TrayState` is held in `Rc<RefCell<_>>` and shared with the frame's callback closure.
  Callbacks are **take/call/put** to satisfy the borrow checker (re-entrant WM delivery
  pattern).
- `show_balloon` writes `title` / `text` into `NOTIFYICONDATAW.szInfoTitle` / `szInfo` via a
  local `write_u16_array` helper that zero-pads unused slots.
- The 1×1 placeholder icon is created with `CreateIconIndirect` + a 1×1 `CreateBitmap`, and
  the bitmap is deleted (it's only used during the icon creation, not afterwards).
- Right-click handling is a two-phase borrow: phase 1 pops the context menu (immutable borrow,
  released at end of block); phase 2 fires the user's right-click callback (mutable borrow).

## Tests

No unit tests in this module — the Win32 shell is required for the call chain. Manual
end-to-end coverage via `examples/icon_tray_demo.rs`.

## Cross-references

- [frame](../window/frame.md) — every tray icon is owned by a frame. The frame's `wndproc` dispatches
  `WM_APP + n` messages to the registered tray handlers.
- [menu](../window/menu.md) — `Menu` is the context-menu payload for `set_menu`.
- [icon](../dc/icon.md) — `svg_bytes_to_hicon` / `destroy_hicon` are the building blocks for the
  tray `HICON`.
- [timer](../core/timer.md) — for periodic re-pop of tray icons / status updates.
- [prelude](../prelude.md)
