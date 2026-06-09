# `src/scrolled_window.rs` — Scrollable container (`wxScrolledWindow`)

Win32 `STATIC` subclass with **window-attached** scroll bars
(`WS_HSCROLL | WS_VSCROLL` window styles, not separate `SCROLLBAR`
child controls). The constructor installs a custom WndProc via
`SetWindowLongPtrW(GWLP_WNDPROC, ...)` that dispatches
`WM_HSCROLL` / `WM_VSCROLL` to the user callback, then forwards
every other message to the original `STATIC` WndProc via
`CallWindowProcW`.

This is the **opposite approach** to [scroll_bar.md](scroll_bar.md):
where `ScrollBar` is a child control whose notifications bubble up
to the parent frame, `ScrolledWindow` *is* the parent — the scroll
bars are part of the window itself, and the WndProc subclass
handles them locally.

## Public types

### `ScrollEvent`

Same shape as `scroll_bar::ScrollEvent` (one-to-one with the
`SB_*` request codes). Defined locally because the two modules
deliver the events through different paths and the user is
expected to import the one matching the widget they use.

| Variant                       | Win32 code       |
| ----------------------------- | ---------------- |
| `LineUp`                      | `SB_LINEUP`      |
| `LineDown`                    | `SB_LINEDOWN`    |
| `PageUp`                      | `SB_PAGEUP`      |
| `PageDown`                    | `SB_PAGEDOWN`    |
| `ThumbRelease { position }`   | `SB_THUMBPOSITION` |
| `ThumbTrack { position }`     | `SB_THUMBTRACK`  |
| `Top`                         | `SB_TOP`         |
| `Bottom`                      | `SB_BOTTOM`      |
| `EndScroll`                   | `SB_ENDSCROLL`   |

### `ScrolledWindow`

`Clone`able wrapper around `Rc<RefCell<ScrolledWindowInner>>`.
Default geometry is `200×200` pixels with a virtual size of
`(0, 0)` (no scroll bars visible). The widget holds at most one
scroll callback at any time — a second `on_scroll` call replaces
the previous one.

## Construction

```rust
let scroll = ScrolledWindow::new(&frame);
```

Creates a `STATIC` child with `WS_CHILD | WS_VISIBLE |
WS_HSCROLL | WS_VSCROLL`. The constructor:

1. Calls `CreateWindowExW` for a 200×200 `STATIC` child.
2. Captures the original `STATIC` WndProc via `GetWindowLongPtrW`.
3. Installs `scrolled_window_wnd_proc` via `SetWindowLongPtrW`.
4. Stores the original WndProc in the thread-local `ORIGINAL_PROCS`
   map for forwarding.

## Virtual size

```rust
scroll.set_virtual_size(2000, 1500);
let (vw, vh) = scroll.get_virtual_size();
```

The scroll bar range is set to `0..max(0, virtual - view)` for
each axis. Passing `(0, 0)` hides the scroll bars. Internally
uses `SCROLLINFO` with `SIF_RANGE | SIF_PAGE` (the modern API).
Setting `nPage` to the view size makes the thumb proportional to
the view / virtual ratio and hides the bar when the view is at
least as large as the virtual content (because then
`nMax - nMin + 1 <= nPage`).

## View position

```rust
scroll.set_view_position(120, 80);
let (x, y) = scroll.get_view_position();
```

`set_view_position` updates the internal `view_position`, mutates
the scroll-bar thumb with `SIF_POS`, and calls `InvalidateRect` to
trigger a repaint. The WndProc also updates `view_position` on
every `WM_HSCROLL` / `WM_VSCROLL` so the cached value stays
correct even if the user doesn't call `set_view_position` from the
callback.

## Events

```rust
scroll.on_scroll(|ev| match ev {
    ScrollEvent::ThumbRelease { position } => { /* user committed a new view position */ }
    ScrollEvent::ThumbTrack { position }   => { /* continuous while dragging */ }
    _ => {}
});
```

The closure is stored in the thread-local `HANDLERS` map keyed by
HWND. The WndProc looks it up on every `WM_HSCROLL` / `WM_VSCROLL`
and invokes it with the decoded `(code, pos)` payload.

### Replacement semantics

A second `on_scroll` call **replaces** the previous callback (the
old `Box<dyn FnMut>` is dropped). Matches the "one owner" model
used elsewhere in the crate (e.g. `set_drop_files_callback`).

## Sizer integration

```rust
sizer.add(&scroll.as_widget_ref(), 1, SizerFlag::Expand);
```

`as_widget_ref()` returns a `WidgetRef` for sizer use.

## `Widget` implementation

The `Widget` trait methods (`set_position`, `set_size`, `set_visible`,
`set_enabled`) are implemented on `ScrolledWindowInner`. `set_size`
also re-computes the scroll-bar range so the new view size is
reflected: range = `0..max(0, virtual - view)`. If the new view
exceeds the virtual size, the range collapses to `0` and the
scroll bar disappears.

## Subclass WndProc

```rust
unsafe extern "system" fn scrolled_window_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT
```

Dispatch table:

| `msg`            | Action                                                        |
| ---------------- | ------------------------------------------------------------- |
| `WM_HSCROLL` / `WM_VSCROLL` | Decode `(code, pos)`, look up `HANDLERS[hwnd]`, invoke. |
| `WM_NCDESTROY`   | Forward to original WndProc, then drop `HANDLERS[hwnd]` and `ORIGINAL_PROCS[hwnd]`. |
| anything else    | Forward to original WndProc (or `DefWindowProcW` as fallback). |

The thread-local tables are the right scope because Win32 windows
are owned by the GUI thread, and the WndProc, the constructor, and
the `on_scroll` setter all run there.

### Cleanup ordering on `WM_NCDESTROY`

The forward to the original WndProc is done **before** the
`HANDLERS` / `ORIGINAL_PROCS` removal — the original WndProc may
legitimately issue another `GetWindowLongPtrW(hwnd, GWLP_WNDPROC)`
and we want it to see the value we stored, not the one Win32 resets
on destroy.

## Quick start

A scrollable container that owns its scroll bars (no separate
`ScrollBar` child controls). Pair it with one or more child
widgets — the typical pattern is a sizer with a large body, then
call `set_virtual_size` so the scroll range matches the content.

```rust,no_run
use ru_wx::prelude::*;

let app = App::new();
let frame = Frame::builder()
    .with_title("ScrolledWindow demo")
    .with_size(400, 300)
    .build();

// 1) Create the scrolled window. It is a STATIC child with
//    WS_HSCROLL | WS_VSCROLL — the bars are part of the window
//    itself, not separate child controls.
let scroll = ScrolledWindow::new(&frame);

// 2) Tell the control how big the *virtual* content is. The
//    scroll-bar range is computed as 0..max(0, virtual - view).
//    Passing (0, 0) hides both bars.
scroll.set_virtual_size(2000, 1500);

// 3) Optionally pre-scroll to a known offset (e.g. restore a
//    saved position). Also invalidates so the change is painted.
scroll.set_view_position(120, 80);

// 4) React to scroll events. The closure receives a typed
//    ScrollEvent that mirrors the SB_* request codes.
scroll.on_scroll(|ev| match ev {
    ScrollEvent::ThumbRelease { position } => {
        // User committed a new view position — persist it.
        println!("committed to {}", position);
    }
    ScrollEvent::ThumbTrack { position } => {
        // Continuous while dragging — keep this cheap (no I/O).
        println!("tracking {}", position);
    }
    _ => {}
});

// 5) Drop the control into a sizer. set_size is also a
//    Widget trait method and re-computes the scroll range.
let sizer = BoxSizer::builder(Orientation::Vertical).build();
sizer.add(&scroll.as_widget_ref(), 1, SizerFlag::Expand);
frame.set_sizer(sizer);

frame.show();
app.run(frame);
```

### Typical workflow

1. **Construct** with `ScrolledWindow::new(&parent)`. The default
   size is 200×200 and the virtual size is (0, 0) (no bars visible).
2. **Set the virtual size** with `set_virtual_size(w, h)`. This is
   the only call that re-computes the scroll range; later changes
   to the *view* size (via `Widget::set_size`) re-compute it
   automatically.
3. **Optionally pre-scroll** with `set_view_position(x, y)`. The
   control remembers the position in an internal field, paints
   itself, and the scroll bar thumb follows.
4. **Register a callback** with `on_scroll(closure)`. A second call
   replaces the previous one (one-owner model).
5. **Add to a sizer** with `scroll.as_widget_ref()`. The view size
   is taken from the sizer slot, and the scroll range updates
   automatically.

### Notes

- **Different from `ScrollBar`**: `ScrollBar` is a separate
  `SCROLLBAR` child control that posts notifications to its
  parent. `ScrolledWindow` *is* the parent — the bars are part
  of the window styles (`WS_HSCROLL | WS_VSCROLL`), and the
  subclass WndProc handles the messages locally. Use
  `ScrolledWindow` for "the body of my dialog scrolls" and
  `ScrollBar` for "I want a dedicated scroll control".
- **`ScrollEvent` is local**: this module's `ScrollEvent` enum is
  intentionally distinct from `scroll_bar::ScrollEvent` (same
  variants, different module path). Import the one matching the
  widget you used.
- **Replacement semantics**: a second `on_scroll` call **replaces**
  the previous callback; the old `Box<dyn FnMut>` is dropped.
- **No auto-managed content**: the user is responsible for
  drawing the scrolled contents (overlap with `on_paint`, layer
  children, etc.). The widget just owns the scroll range and the
  thumb position.
- **Cleanup**: the subclass WndProc removes its thread-local
  `HANDLERS` / `ORIGINAL_PROCS` entries on `WM_NCDESTROY` so the
  widget is safe to drop via `Clone` + `Drop` from any scope.

## Win32 internals

- **Control class**: `STATIC` subclassed at runtime.
- **Range / position API**: full `SCROLLINFO` with
  `SIF_RANGE | SIF_PAGE` and `SIF_POS` (modern API; the legacy
  `SetScrollPos` / `SetScrollRange` pair is not exposed by
  `windows-sys 0.59`).
- **`CallWindowProcW`**: takes `Option<WNDPROC>` in
  `windows-sys 0.59` (so the same signature can be used for both
  "real" and "def" subclasses), so the original pointer is
  wrapped in `Some`.
- **Constants defined locally**: `WS_HSCROLL`, `WS_VSCROLL`,
  `WM_NCDESTROY`, `SB_HORZ`, `SB_VERT`. The `SCROLLBAR_CONSTANTS`
  enum is a type alias for `i32`, so the constants are plain
  integer literals.

## Tests

Three unit tests in the `tests` module cover the platform-agnostic
surface:

| Test                                | Verifies                                         |
| ----------------------------------- | ------------------------------------------------ |
| `scroll_event_variants_are_distinct` | Every variant compares unequal.                  |
| `scroll_event_copy_semantics`       | The enum is `Copy` (carries at most one `i32`). |
| `virtual_size_defaults_to_zero`     | The default virtual size is `(0, 0)` (no bars). |

The Win32 WndProc dispatch path is exercised in the integration
test (a real `HWND` and a real parent frame are needed).

## Cross-references

- [`scroll_bar.md`](scroll_bar.md) — sibling module with separate
  `SCROLLBAR` child controls; the two `ScrollEvent` enums are
  intentionally identical but live in different modules.
- [`splitter_window.md`](splitter_window.md) — another subclassed
  container (sash-based, not scroll-based).
- [`widget.md`](widget.md) — `Widget` / `WidgetRef` / `Window`
  traits and the geometry methods.
