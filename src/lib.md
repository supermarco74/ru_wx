# lib.rs

Crate root for `ru_wx`, a pure-Rust GUI library that mirrors the wxWidgets API on top of native platform controls.

## Purpose

- Declares the `ru_wx` library and re-exports the public API from all submodules.
- Sets the global lint policy (`#![allow(clippy::missing_docs_in_private_items)]`) — every public item is documented, internal `*Inner` fields and Win32 message constants intentionally are not.
- Exposes the `prelude` module for one-line imports.

## Public API shape

- 45+ modules, each exporting one or more widget types (`Button`, `Frame`, `Grid`, …).
- Re-exported at the crate root via `pub use module::Type;` — every public type is reachable as `ru_wx::Type`.
- Standard prelude: `use ru_wx::prelude::*;` brings in the "build a window + controls + run loop" subset.

## Conventions

- The crate targets Win32 only today (macOS AppKit and Linux GTK are planned but stubbed).
- All Win32-specific code is gated `#[cfg(target_os = "windows")]`.
- Layout / coordinate types (`Rect`, `Colour`) live in [`geometry`], DPI helpers in [`dpi`], logging in [`log`], Win32 utilities in [`platform`].

## Quick start (from the lib-level doctest)

```rust,no_run
use ru_wx::*;
let app = App::new();
let frame = Frame::builder()
    .with_title("Hello")
    .with_size(400, 300)
    .build();
let button = Button::new(&frame, "Click me!");
button.on_click(&frame, || println!("Clicked!"));
app.run(frame);
```

## See also

- [`prelude.rs`](prelude.md) — one-import convenience re-exports.
- [`widget.rs`](core/widget.md) — the `Widget` and `Window` traits every concrete type implements.
- [`app.rs`](core/app.md) — the `App` entry point and event loop driver.
