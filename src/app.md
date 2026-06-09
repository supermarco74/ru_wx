# app.rs

Application bootstrap and event-loop driver.

## Purpose

- `App::new()` performs one-shot platform initialisation. On Windows it calls `InitCommonControlsEx` with `ICC_STANDARD_CLASSES | ICC_TAB_CLASSES | ICC_LISTVIEW_CLASSES` so common controls (tab, list-view, header, …) render with visual styles.
- `App::run(frame)` enters the platform event loop on the supplied top-level frame and blocks until the window is closed. Internally it just calls `frame.show()`, which calls `ShowWindow` and then `GetMessageW` / `DispatchMessageW` until `WM_QUIT`.

## Key types

- **`App`** — zero-sized struct. Construction is where Win32 initialisation lives; the message loop is on `run`.

## Public API

```rust
pub struct App;
impl App {
    pub fn new() -> Self;
    pub fn run(self, frame: Frame);   // never returns
}
impl Default for App { /* delegates to ::new */ }
```

## Quick start

```rust,no_run
use ru_wx::prelude::*;

fn main() {
    // 1. Boot: this is the *only* place platform initialisation happens.
    //    On Windows it calls InitCommonControlsEx with STANDARD | TAB | LISTVIEW.
    let app = App::new();

    // 2. Build a top-level frame.
    let frame = Frame::builder()
        .with_title("My first ru_wx app")
        .with_size(640, 480)
        .build();

    // 3. Populate the frame with controls (omitted here — see Panel, Button, etc.).

    // 4. Show the window and block on the message loop until WM_QUIT.
    //    `run` consumes the Frame and never returns.
    frame.show();
    app.run(frame);
}
```

If you build a custom executable, put the four steps above in `main()`. There is no implicit app context — every `ru_wx` program starts with `App::new()` exactly once.

## Win32 notes

- `InitCommonControlsEx` is called with an `INITCOMMONCONTROLSEX` whose `dwSize` is computed via `size_of::<INITCOMMONCONTROLSEX>() as u32`. The flag set is defensive: `ICC_STANDARD_CLASSES` alone would cover tabs, but the explicit `ICC_TAB_CLASSES` and `ICC_LISTVIEW_CLASSES` belt-and-suspender the call against future `windows-sys` flag reshuffles.
- The actual `GetMessage` / `DispatchMessage` loop lives in [`frame.rs`](./frame.md) (`Frame::show`), not here.

## Usage

```rust
use ru_wx::prelude::*;
let app = App::new();
let frame = Frame::builder().with_title("hi").build();
app.run(frame);
```

## See also

- [`frame.rs`](./frame.md) — `Frame::show` is where the message loop actually runs.
- [`lib.rs`](./lib.md) — `App` is re-exported at the crate root.
