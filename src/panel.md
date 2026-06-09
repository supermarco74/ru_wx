# panel.rs

Generic child window container (`wxPanel` analog). A panel is a child window that can host its own sizer-driven layout, custom background colour, and forwarded child events. Panels can be nested (`Panel::new_in_panel`).

## Purpose
The default container widget. Use a panel as the immediate child of a `Frame` to provide a managed client area; nested panels partition that area into sub-regions. A panel's WndProc:
- Erases its own background (via `CreateSolidBrush` + `FillRect` on `WM_ERASEBKGND`).
- Forwards child `WM_COMMAND` / `WM_NOTIFY` notifications to its parent via `SendMessageW` so the frame-level dispatch tables can handle them.
- Cleans up the `Rc<RefCell<PanelData>>` stored in `GWLP_USERDATA` on `WM_DESTROY`.

## Key Types
- `Panel` — public struct.
- `PanelData` (private) — `HWND`, `sizer`, `background_colour`, child widgets, `RefCell` borrow state.
- `PanelInner` (private) — Win32 `HWND`.

## Key Methods
- `Panel::new<W: Window>(parent: &W) -> Self` — Creates a panel as a child of any `Window` (typically a `Frame`).
- `Panel::new_in_panel(parent_panel: &Panel) -> Self` — Nested panel construction.
- `add_widget(&self, widget: WidgetRef)` — Registers a child for sizer/layout tracking.
- `set_sizer(&self, sizer: BoxSizer)` — Installs a `BoxSizer`; child positions/sizes are recomputed on parent resize.
- `set_background_colour(&self, colour: Colour)` — Updates the stored colour and calls `InvalidateRect` to trigger a repaint.
- `set_position(&self, x: i32, y: i32)` / `set_size(&self, w: u32, h: u32)` — Both carefully release the `RefCell` borrow **before** calling `MoveWindow` to avoid a re-entrancy panic (see `panel_wnd_proc` discussion below).

## Win32 Notes
- Window class: `"RuWxPanel"`, registered by `register_panel_class()` at first panel creation. Class styles `CS_HREDRAW | CS_VREDRAW`; cursor `LoadCursorW(IDC_ARROW)`.
- `panel_wnd_proc` (unsafe `extern "system"`) handles:
  - `WM_ERASEBKGND` — `CreateSolidBrush` + `FillRect` with the stored colour; returns `1` to suppress default erase.
  - `WM_DESTROY` — `SetWindowLongPtrW(GWLP_USERDATA, 0)` and drops the stored `Rc`.
  - `WM_COMMAND | WM_NOTIFY` — `SendMessageW(GetParent(hwnd), msg, wparam, lparam)` to forward child notifications to the frame so the frame's command/notify tables handle them.
  - Default — `DefWindowProcW`.
- **Critical forwarding carve-out**: the wndproc does **not** forward `WM_UPDATEUISTATE`, `WM_SETCURSOR`, or `WM_NCHITTEST`. Forwarding any of these would cause Win32 to recurse into the panel from inside the very same `SendMessageW` and crash.
- `set_position` / `set_size` take a brief immutable borrow to read `hwnd`, drop it, then call `MoveWindow`. This avoids the synchronous `WM_SIZE` / `WM_ERASEBKGND` re-entrancy that would otherwise panic the `RefCell`.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let panel = Panel::new(&frame);

// Attach a sizer for automatic child placement.
let sizer = BoxSizer::new(Orientation::Vertical);
sizer.add(&Button::new(&panel, "Top"),    0, SizerFlag::Expand, 0);
sizer.add(&Button::new(&panel, "Bottom"), 0, SizerFlag::Expand, 0);
panel.set_sizer(sizer);

panel.set_background_colour(Colour::LIGHT_GREY);
```

A panel can host any child widget (buttons, text, even other panels).
Use `set_background_colour` to override the system default; the change
triggers a repaint via `InvalidateRect`. For purely decorative grouping
with no sizer, use a [`StaticBox`](./static_box.md).

## See Also
- [`frame.rs`](./frame.md) — typical parent of a top-level `Panel`; owns the dispatch tables
- [`sizer.rs`](./sizer.md) — `BoxSizer` is the standard layout engine inside a panel
- [`dialog.rs`](./dialog.md) — modal/modeless dialog uses a similar WndProc pattern
- [`static_box.rs`](./static_box.md) — purely decorative alternative; does not host a sizer
