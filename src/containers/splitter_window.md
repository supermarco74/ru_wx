# `src/splitter_window.rs` — Two-pane resizable container (`wxSplitterWindow`)

Win32 `STATIC` subclass that owns two child pane `HWND`s and draws
/ tracks a single draggable sash between them. The constructor
installs a custom WndProc via `SetWindowLongPtrW(GWLP_WNDPROC, ...)`
that:

1. **Draws the sash line** on `WM_PAINT`.
2. **Starts a sash drag** on `WM_LBUTTONDOWN` when the click is
   inside the ±`SASH_GRAB` (4 px) strip around the sash, and
   updates the sash position in real time on `WM_MOUSEMOVE`.
3. **Changes the cursor** to the appropriate size cursor
   (`IDC_SIZENS` for horizontal, `IDC_SIZEWE` for vertical) on
   `WM_SETCURSOR` when the mouse is over the sash.
4. **Re-positions the two owned pane `HWND`s** on every size
   change (and after every drag) so the user's child widgets
   stay flush with the splitter's geometry.
5. **Cleans up the thread-local state** on `WM_NCDESTROY`.

The widget is a **controller** — it does not own the pane contents
itself, it just lays out the two pane `HWND`s that the user passed
to `split_horizontally` or `split_vertically`. The user may
either let the splitter reposition those `HWND`s automatically
(the default) or set them up to react to `SashEvent` callbacks
and reposition their own children by hand.

## Public types

### `SplitterOrientation`

```rust
pub enum SplitterOrientation {
    Horizontal, // sash is a horizontal line; first pane above, second below; cursor = IDC_SIZENS
    Vertical,   // sash is a vertical line; first pane left, second right; cursor = IDC_SIZEWE
}
```

### `SashEvent`

One phase of a sash drag, delivered to callbacks registered with
`on_sash_drag`:

| Variant                       | Trigger                                              |
| ----------------------------- | ---------------------------------------------------- |
| `DragStart`                   | Left mouse button pressed over the sash.             |
| `DragMove { position }`       | Mouse moving while dragging; `position` is live.     |
| `DragEnd { position }`        | Left button released; `position` is the final value. |

`position` is in client-area pixels — `x` for vertical splitters,
`y` for horizontal.

### `SplitterWindow`

`Clone`able wrapper around `Rc<RefCell<SplitterWindowInner>>`.
Default geometry is `200×200` with a vertical sash at `x = 100`.
The widget holds at most one sash callback at any time.

## Construction

```rust
let splitter = SplitterWindow::new(&frame);
```

Creates a 200×200 `STATIC` child, captures the original WndProc,
and installs `splitter_window_wnd_proc`. Pane `HWND`s are
`null` until `split_horizontally` / `split_vertically` is called.

## Pane attachment

```rust
let pane1 = ...; // HWND of a child widget parented to the splitter
let pane2 = ...;
splitter.split_vertically(pane1, pane2);
// or
splitter.split_horizontally(pane1, pane2);
```

| Method                          | Effect                                              |
| ------------------------------- | --------------------------------------------------- |
| `split_horizontally(p1, p2)`    | Orientation = Horizontal. Panes auto-laid out.      |
| `split_vertically(p1, p2)`      | Orientation = Vertical. Panes auto-laid out.        |
| `set_orientation(orientation)`  | Re-issue the most recent `split_*` with new orient. |
| `orientation()`                 | Returns the current orientation.                    |

The `HWND`s are expected to be children of the splitter itself
(Win32 parenting rules); in practice the user creates them with
the splitter as their parent. If either `HWND` is `null`, the
corresponding pane is left empty.

## Sash position

```rust
splitter.set_sash_position(150);
let pos = splitter.get_sash_position();
```

| Method                       | Effect                                                |
| ---------------------------- | ----------------------------------------------------- |
| `set_sash_position(pos)`     | Clamped to `SASH_GRAB..dim - SASH_GRAB` (4 px margins). |
| `get_sash_position()`        | Cached value.                                         |

After every `set_sash_position`, `layout_panes` is invoked to
re-position the two pane `HWND`s, and the splitter is invalidated
to repaint the sash.

## Events

```rust
splitter.on_sash_drag(|ev| match ev {
    SashEvent::DragStart => { /* user grabbed the sash */ }
    SashEvent::DragMove { position } => { /* live update */ }
    SashEvent::DragEnd { position } => { /* user committed */ }
});
```

The closure is stored in the thread-local `HANDLERS` map keyed by
HWND. A second `on_sash_drag` call replaces the previous callback.

## Sizer integration

```rust
sizer.add(&splitter.as_widget_ref(), 1, SizerFlag::Expand);
```

`as_widget_ref()` returns a `WidgetRef` for sizer use.

## Pane layout

The free function `layout_panes` (private to the module) is the
single source of truth for pane geometry. Called from
`set_sash_position`, `set_orientation`, `split_horizontally` /
`split_vertically`, and the `WM_SIZE` arm of the subclass WndProc.

| Orientation  | `pane1`                       | `pane2`                            |
| ------------ | ----------------------------- | ---------------------------------- |
| Vertical     | `(0, 0, split, h)`            | `(split+1, 0, w-split-1, h)`       |
| Horizontal   | `(0, 0, w, split)`            | `(0, split+1, w, h-split-1)`       |

The 1-pixel gap at `split+1` is the visible sash line.

## Subclass WndProc

```rust
unsafe extern "system" fn splitter_window_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT
```

Dispatch table:

| `msg`                          | Action                                              |
| ------------------------------ | --------------------------------------------------- |
| `WM_PAINT`                     | Draw the sash line via `MoveToEx` / `LineTo`.       |
| `WM_LBUTTONDOWN`               | If click within ±4 px of the sash, `SetCapture` and fire `DragStart`. |
| `WM_MOUSEMOVE`                 | If dragging, update `SASH_POS`, fire `DragMove`, invalidate. |
| `WM_LBUTTONUP` / `WM_CAPTURECHANGED` | End the drag, `ReleaseCapture`, fire `DragEnd`. |
| `WM_SETCURSOR`                 | If mouse over sash, set `IDC_SIZENS` / `IDC_SIZEWE` cursor. |
| `WM_SIZE`                      | Re-clamp `SASH_POS` to new client area.             |
| `WM_NCDESTROY`                 | Forward to original WndProc, then drop all thread-local state. |
| anything else                  | Forward to original WndProc (or `DefWindowProcW` as fallback). |

The WndProc holds the per-HWND drag state in three extra
thread-local maps (in addition to `HANDLERS` / `ORIGINAL_PROCS`
shared with `scrolled_window.rs`):

| Map         | Type             | Purpose                                                |
| ----------- | ---------------- | ------------------------------------------------------ |
| `DRAGGING`  | `HashMap<HWND, bool>` | True while a drag is in progress.                 |
| `SASH_POS`  | `HashMap<HWND, i32>`  | Most recent sash position (kept in sync).         |
| `ORIENT`    | `HashMap<HWND, u8>`   | Orientation as `0 = Vertical, 1 = Horizontal`.   |

These are written by `sync_splitter_state` (the public-in-crate
helper) and read by the WndProc on every event.

## Quick start

A two-pane resizable container with a single draggable sash. The
splitter is a *controller*: it does not own the pane contents,
it just lays out the two pane `HWND`s that the user passes to
`split_horizontally` or `split_vertically`. By default the
splitter repositions those `HWND`s automatically on every size
change and drag, so the user only needs to construct the panes
and parent them to the splitter.

```rust,no_run
use ru_wx::prelude::*;

let app = App::new();
let frame = Frame::builder()
    .with_title("SplitterWindow demo")
    .with_size(600, 400)
    .build();

// 1) Create the splitter. It is a STATIC subclass that owns
//    the sash. Pane HWNDs are null until split_* is called.
let splitter = SplitterWindow::new(&frame);

// 2) Build the two panes. They must be children of the
//    splitter (Win32 parenting rules). For this example we
//    use two simple panels as placeholders.
let left = Panel::new(&splitter); // any child widget works
let right = Panel::new(&splitter);

// 3) Lay them out with a vertical sash at x = 200.
//    The splitter auto-positions the two pane HWNDs on every
//    WM_SIZE and after every drag.
let (left_hwnd, right_hwnd) = (left.hwnd(), right.hwnd());
splitter.split_vertically(left_hwnd, right_hwnd);
splitter.set_sash_position(200);

// 4) React to sash drags. The closure receives a typed
//    SashEvent describing the current phase.
splitter.on_sash_drag(|ev| match ev {
    SashEvent::DragStart => println!("grabbed the sash"),
    SashEvent::DragMove { position } => println!("dragging at {}", position),
    SashEvent::DragEnd { position } => println!("released at {}", position),
});

// 5) Drop the splitter into a sizer so it fills the frame.
let sizer = BoxSizer::builder(Orientation::Vertical).build();
sizer.add(&splitter.as_widget_ref(), 1, SizerFlag::Expand);
frame.set_sizer(sizer);

frame.show();
app.run(frame);
```

### Typical workflow

1. **Construct** the splitter with `SplitterWindow::new(&parent)`.
   Default size is 200×200, default orientation is Vertical, sash
   at x = 100.
2. **Create the two panes** as ordinary child widgets parented
   to the splitter. Their `HWND`s will be passed to the split
   method.
3. **Call** `split_horizontally(p1_hwnd, p2_hwnd)` or
   `split_vertically(p1_hwnd, p2_hwnd)`. Both set the orientation
   and trigger an initial `layout_panes`. Pass `null` to leave a
   pane empty.
4. **Adjust the sash** with `set_sash_position(pos)`. The value
   is clamped to `SASH_GRAB..dim - SASH_GRAB` (4 px margins), the
   panes are re-laid out, and the control is invalidated so the
   new sash line paints.
5. **Optionally subscribe** to drag phases with `on_sash_drag(closure)`.
   A second call replaces the previous one. The drag state is
   managed by the WndProc — `SetCapture` / `ReleaseCapture` keep
   the drag alive even when the mouse leaves the client area.

### Notes

- **Controller, not container**: the splitter does not own the
  pane widgets themselves — only their `HWND`s. Destroying the
  splitter does not destroy its panes; the user is responsible
  for keeping them alive (e.g. via `Rc<RefCell<…>>` / `Clone`).
- **Auto-layout vs. manual**: the default behaviour is to
  reposition the two pane `HWND`s on every size change. To
  manage layout manually, subscribe to `SashEvent::DragMove` /
  `DragEnd` and resize your own children.
- **`SashEvent::position` units**: for vertical splitters the
  value is the x coordinate of the sash; for horizontal
  splitters it is the y coordinate. Both are in client-area
  pixels.
- **`set_orientation` re-issues the last `split_*`**: it
  re-applies the most recent pane pair with the new orientation,
  so you do not need to remember which pair you passed.
- **Replacement semantics**: a second `on_sash_drag` call
  **replaces** the previous callback; the old `Box<dyn FnMut>` is
  dropped.
- **Cursor is automatic**: `WM_SETCURSOR` sets `IDC_SIZENS` /
  `IDC_SIZEWE` automatically when the mouse is over the ±4 px
  sash strip, so the user does not need to set a cursor
  manually.

## Win32 internals

- **Control class**: `STATIC` subclassed at runtime.
- **Sash geometry**: visible line is 1 device pixel; the wider
  ±4 px strip (`SASH_GRAB`) is the mouse-target zone.
- **Capture**: `SetCapture` / `ReleaseCapture` is used during a
  drag so the sash keeps tracking even when the mouse leaves the
  splitter's client area.
- **Constants defined locally**: `WM_NCDESTROY` and `SASH_GRAB`
  are not exposed by `windows-sys 0.59`.

## Tests

Three unit tests in the `tests` module cover the platform-agnostic
surface:

| Test                          | Verifies                                              |
| ----------------------------- | ----------------------------------------------------- |
| `orientation_variants_are_distinct` | `Horizontal` and `Vertical` are distinct enums. |
| `sash_event_variants_distinct` | `DragStart` / `DragMove` / `DragEnd` are distinct.  |
| `lparam_pos_decoding`         | `(x, y)` extraction from `WM_LBUTTONDOWN` `LPARAM` (handles negative coordinates). |

The Win32 WndProc dispatch path is exercised in the integration
test (a real parent frame and two real pane windows are needed).

## Cross-references

- [`scroll_bar.md`](scroll_bar.md) — separate child controls
  vs. window-attached scroll bars (different mechanism).
- [`scrolled_window.md`](scrolled_window.md) — sibling subclassed
  container (scroll bars, not sash).
- [`widget.md`](../core/widget.md) — `Widget` / `WidgetRef` / `Window`
  traits and the geometry methods.
