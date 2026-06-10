# widget.rs

Core cross-platform widget traits and a type-erased reference used by sizers.

## Purpose

Defines the minimal API every concrete control implements so that the layout, container, and event-dispatch code can work generically.

## Key types

- **`Widget` (trait)** — platform-independent operations every control exposes:
  - `native_handle(&self) -> isize` — raw pointer-sized handle (`HWND` on Windows).
  - `set_position(&mut self, x: i32, y: i32)` / `set_size(&mut self, w: u32, h: u32)`.
  - `rect(&self) -> Rect` — current position and size.
  - `is_visible` / `set_visible(bool)`, `is_enabled` / `set_enabled(bool)`.
- **`WidgetRef`** — `type WidgetRef = Rc<RefCell<dyn Widget>>;`. Used by sizers and containers to hold heterogeneous children without generics.
- **`Window` (trait, `#[cfg(target_os = "windows")] only)`** — small extension trait: `fn hwnd(&self) -> HWND;`. Any concrete `Frame` / `Panel` / control implements it. Most widgets are generic over `W: Window` so they can be parented by either a `Frame` or a `Panel`.

## Conventions

- The `Window` trait is **Windows-only**; on non-Windows targets it does not exist and code that names it must also be `cfg`-gated.
- `Widget` is total — every method has a default Win32 implementation in the concrete type's `*Inner`.
- `native_handle()` returns `isize` (not `HWND`) so the trait compiles on macOS / Linux without dragging `windows-sys` in.

## Quick start

```rust
use ru_wx::prelude::*;

// Every concrete control implements Widget:
fn describe(w: &dyn Widget) {
    let h = w.native_handle();   // isize — HWND on Windows
    let r = w.rect();            // geometry::Rect
    println!("handle={h} rect={:?} visible={} enabled={}", r, w.is_visible(), w.is_enabled());
}

// Sizers hold heterogeneous children as WidgetRef (= Rc<RefCell<dyn Widget>>):
let mut s = BoxSizer::vertical();
s.add(button.as_widget_ref());
s.add(label.as_widget_ref());

// On Windows, the Window trait gives you the raw HWND:
#[cfg(target_os = "windows")]
fn paint_target(w: &dyn Window) -> HWND { w.hwnd() }
```

## See also

- [`geometry.rs`](geometry.md) — `Rect`, the return type of `Widget::rect`.
- [`app.rs`](app.md) — uses `Frame` (which implements `Window`).
- [`frame.rs`](../window/frame.md) — concrete `Frame` is a `Widget` and a `Window`.
