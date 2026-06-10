# `src/scroll_bar.rs` — Standalone scroll bar widget (`wxScrollBar`)

Win32 `SCROLLBAR` class wrapped as a regular `Widget`. Reports
scroll events through the parent frame's `WM_HSCROLL` / `WM_VSCROLL`
dispatch — see [Frame::register_scroll_handler](../window/frame.md) for the
mechanism. Range and position use the legacy 16-bit-friendly
`SBM_SETRANGEREDRAW` / `SBM_SETPOS` API (no full `SCROLLINFO`
support).

## Public types

### `ScrollBarOrientation`

```rust
pub enum ScrollBarOrientation {
    Horizontal, // SBS_HORZ, drawn left-to-right
    Vertical,   // SBS_VERT, drawn top-to-bottom
}
```

### `ScrollEvent`

Strongly-typed scroll event delivered to user callbacks. Variant
names mirror the Win32 `SB_*` request codes; the position payload
(where applicable) is wrapped in `ThumbTrack { position }` /
`ThumbRelease { position }` to make the type-safe signature
self-documenting.

| Variant                              | Win32 code      | Notes                                  |
| ------------------------------------ | --------------- | -------------------------------------- |
| `LineUp`                             | `SB_LINEUP`     | Up / left arrow                        |
| `LineDown`                           | `SB_LINEDOWN`   | Down / right arrow                     |
| `PageUp`                             | `SB_PAGEUP`     | Page-up / page-left area               |
| `PageDown`                           | `SB_PAGEDOWN`   | Page-down / page-right area            |
| `ThumbRelease { position }`          | `SB_THUMBPOSITION` | User released the thumb              |
| `ThumbTrack { position }`            | `SB_THUMBTRACK` | Fires repeatedly while dragging        |
| `Top`                                | `SB_TOP`        | `Ctrl+Home`                            |
| `Bottom`                             | `SB_BOTTOM`     | `Ctrl+End`                             |
| `EndScroll`                          | `SB_ENDSCROLL`  | Last event in a scroll sequence        |

The same four "up/down/page" variants cover both horizontal and
vertical — `SB_LINELEFT` / `SB_LINERIGHT` / `SB_PAGELEFT` /
`SB_PAGERIGHT` / `SB_LEFT` / `SB_RIGHT` collapse into the same
variants on the user side.

### `ScrollBar`

`Clone`able wrapper around `Rc<RefCell<ScrollBarInner>>`. Owns no
event handlers — the frame's WndProc dispatches.

## Construction

```rust
let bar = ScrollBar::new(&frame, ScrollBarOrientation::Vertical);
// or
let bar = ScrollBar::new_full(&frame, orientation, 0, 100, 10);
```

| Method                  | Effect                                                |
| ----------------------- | ----------------------------------------------------- |
| `ScrollBar::new`        | Range `0..100`, page size `10`.                       |
| `ScrollBar::new_full`   | Explicit `min`, `max`, `page_size`.                   |

The HWND is created with `WS_CHILD | WS_VISIBLE` plus
`SBS_HORZ` / `SBS_VERT` based on the orientation. The thumb starts
at `min`; use `set_position` to move it. Default geometry is
`200×16` for horizontal, `16×200` for vertical.

## Range / position

| Method                    | Notes                                                |
| ------------------------- | ---------------------------------------------------- |
| `set_range(min, max)`     | `SBM_SETRANGEREDRAW`; thumb clamped to new range.    |
| `get_range()`             | Returns cached `(min, max)`.                         |
| `set_position(pos)`       | `SBM_SETPOS` with `wparam = 1` (redraw on).          |
| `get_position()`          | Live `SBM_GETPOS`, cached in `ScrollBarInner`.       |
| `set_page_size(size)`     | `SBM_SETPAGESIZE` (the "large step").                |
| `get_page_size()`         | Cached value.                                        |
| `orientation()`           | Returns the construction-time orientation.           |
| `id()`                    | The control id (from `next_control_id`).             |

### Method-name collision with `Widget::set_position`

`ScrollBar::set_position` is a *separate* method (not the
`Widget::set_position(&mut self, x, y)` trait method) — it sets
the **thumb position**, not the window position. The widget
position is set via the sizer / `MoveWindow` path. If you need
to call the trait method through a `WidgetRef`, qualify the call:
`Widget::set_position(&mut *hbar.as_widget_ref().borrow_mut(), 20, 60)`.

## Events

```rust
bar.on_scroll(&frame, |ev| match ev {
    ScrollEvent::ThumbRelease { position } => { /* user picked a new value */ }
    ScrollEvent::ThumbTrack { position }   => { /* fires every drag tick — debounce if heavy */ }
    _ => {}
});
```

Internally, `on_scroll` registers a `FnMut(u16, i32)` callback
through `Frame::register_scroll_handler` (keyed by HWND). The
wrapper translates the raw `(code, position)` payload into a typed
`ScrollEvent` and re-syncs the cached `position` from `SBM_GETPOS`
on every event (so the cache stays correct across line / page /
thumb events).

The same callback can be registered multiple times for the same
scroll bar — the underlying map uses the HWND as the key, so the
most recent registration wins. To chain multiple callbacks, wrap
them in a single closure.

## Sizer integration

```rust
sizer.add(&bar.as_widget_ref(), 0, SizerFlag::None);
```

`as_widget_ref()` returns a `WidgetRef` (`Rc<RefCell<dyn Widget>>`)
for sizer use.

## Quick start

A complete, copy-pasteable example: a horizontal `ScrollBar` paired with
a label, wired to react to thumb-release events and to update the label.

```rust,no_run
use ru_wx::prelude::*;

fn build_scrollbar(frame: &Frame) -> ScrollBar {
    // 1. Create a horizontal scroll bar with explicit range and page size.
    let bar = ScrollBar::new_full(
        frame,
        ScrollBarOrientation::Horizontal,
        /* min */    0,
        /* max */  100,
        /* page */  10,
    );

    // 2. Initial position.
    bar.set_position(0);

    // 3. React to scroll events. ThumbRelease fires once per drag end;
    //    ThumbTrack fires continuously while dragging (debounce if heavy).
    bar.on_scroll(frame, |ev| match ev {
        ScrollEvent::ThumbRelease { position } => {
            println!("committed to position {}", position);
        }
        ScrollEvent::ThumbTrack { position } => {
            // Heavy: debounce in your own code.
            println!("tracking {}", position);
        }
        ScrollEvent::LineUp        => println!("line up"),
        ScrollEvent::LineDown      => println!("line down"),
        ScrollEvent::PageUp        => println!("page up"),
        ScrollEvent::PageDown      => println!("page down"),
        ScrollEvent::Top           => println!("top"),
        ScrollEvent::Bottom        => println!("bottom"),
        ScrollEvent::EndScroll     => println!("end scroll"),
    });

    // 4. Read back the current state at any time.
    let (min, max) = bar.get_range();
    println!("range: {}..{}, position: {}", min, max, bar.get_position());

    // 5. (Optional) Change the range at runtime — the thumb clamps to it.
    // bar.set_range(0, 1000);
    // bar.set_page_size(50);

    // 6. Add to a sizer. `as_widget_ref()` returns a `WidgetRef`.
    // sizer.add(bar.as_widget_ref(), 0, SizerFlag::None);

    bar
}

// Other constructor you can swap in:

#[allow(dead_code)]
fn quick_vertical(frame: &Frame) -> ScrollBar {
    // Default range 0..100, page 10, vertical orientation.
    ScrollBar::new(frame, ScrollBarOrientation::Vertical)
}
```

**Typical workflow**

1. Create the bar with `ScrollBar::new(frame, orientation)` for the
   common case (range 0..100, page 10) or `ScrollBar::new_full(frame,
   orientation, min, max, page_size)` for full control.
2. Set the initial thumb position with `set_position(pos)`.
3. Register a scroll callback with `on_scroll(frame, |ev| ...)` and
   match on the `ScrollEvent` variants. The frame's WndProc dispatches
   `WM_HSCROLL` / `WM_VSCROLL` to the right HWND-keyed handler.
4. Read state at any time with `get_range()` / `get_position()` /
   `get_page_size()`. The position cache is kept in sync by every
   scroll event, so it stays correct even if the user doesn't call
   `set_position` from the callback.
5. Pass to a sizer via `as_widget_ref()` (a `WidgetRef`). The bar
   occupies its preferred size (`200×16` horizontal, `16×200` vertical).

**Notes**

- `ScrollBar::set_position` is a **separate method** from the
  `Widget::set_position` trait method — it sets the *thumb position*,
  not the *window position*. To move the window, use the sizer or
  `MoveWindow`. If you need the trait method through a `WidgetRef`,
  qualify the call: `Widget::set_position(&mut *bar.borrow_mut(), 20, 60)`.
- Multiple `on_scroll` registrations on the same scroll bar **clobber**
  each other (the underlying map is keyed by HWND). To chain multiple
  callbacks, wrap them in a single closure.
- The same `ScrollEvent` enum covers both horizontal and vertical
  orientations — `SB_LINELEFT` / `SB_LINERIGHT` etc. collapse into
  the same `LineUp` / `LineDown` variants.
- This module uses the **legacy 16-bit-friendly `SBM_SETRANGEREDRAW` /
  `SBM_SETPOS` API**. For 32-bit-clean ranges you need the full
  `SCROLLINFO` API; that's what [scrolled_window](scrolled_window.md)
  uses (different mechanism, window-attached scroll bars).
- Cross-platform: the type and methods compile on non-Windows hosts
  but the bar does not render. Use it on Windows only.

## Win32 internals

- **Control class**: `SCROLLBAR` (the system scroll bar class, not
  a custom-drawn one).
- **Message dispatch**: scroll notifications bubble up to the
  parent frame's `WndProc`; the frame looks up the registered
  handler by the scroll bar's `HWND`.
- **Range API**: `SBM_SETRANGEREDRAW` packs `(min, max)` into the
  32-bit `lparam` with `min` in the low word and `max` in the
  high word. This is fine for the common case of 16-bit-friendly
  ranges (0..65535). For 32-bit-clean ranges the full
  `SCROLLINFO` API is needed (not currently exposed by this
  module).
- **Constants defined locally**: `SBM_SETPOS`, `SBM_GETPOS`,
  `SBM_SETRANGE`, `SBM_GETRANGE`, `SBM_ENABLE_ARROWS`,
  `SBM_SETPAGESIZE`, `SBM_SETRANGEREDRAW`, the `SB_*` request
  codes, and `SBS_HORZ` / `SBS_VERT` are all defined locally
  because `windows-sys 0.59` does not surface them in
  `Win32_UI_WindowsAndMessaging`.

## Tests

Four unit tests in the `tests` module cover the platform-agnostic
surface (no Win32 window creation required):

| Test                                    | Verifies                                      |
| --------------------------------------- | --------------------------------------------- |
| `line_up_is_a_distinct_variant`         | Variants compare unequal.                     |
| `thumb_release_carries_position`        | `ThumbRelease` payload field works.          |
| `thumb_track_carries_position`          | `ThumbTrack` accepts negative `position`.     |
| `orientation_distinct_variants`         | `as u32` discriminants are 0 and 1.           |
| `end_scroll_is_a_distinct_variant`      | `EndScroll` is its own variant.               |

## Cross-references

- [`frame.md`](../window/frame.md) — `Frame::register_scroll_handler` and
  the `frame_wnd_proc` dispatch.
- [`scrolled_window.md`](scrolled_window.md) — alternative
  container with **window-attached** scroll bars (different
  mechanism: subclass WndProc + `SCROLLINFO`).
- [`splitter_window.md`](splitter_window.md) — a different kind
  of resizable container (sash, not scroll bars).
- [`widget.md`](../core/widget.md) — `Widget` / `WidgetRef` / `Window`
  traits and the geometry methods.
