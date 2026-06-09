# `src/drop_target.rs` — Shell-level file drop (`WM_DROPFILES`)

## Purpose
Receives files dragged from Windows Explorer onto a top-level frame. Uses
the **Shell-level** `WM_DROPFILES` / `DragAcceptFiles` / `DragQueryFileW`
protocol, which is simpler than the OLE COM `IDropTarget` and is sufficient
for the common "user drags N files onto my app" workflow. Does **not** need
`OleInitialize` / `RegisterDragDrop` / COM.

For arbitrary data formats (text, in-app objects) and drag-over feedback,
see [`ole_dnd.rs`](ole_dnd.md) — the two protocols coexist and are not
mutually exclusive.

## Public types
- `DroppedFiles`:
  - `pub fn len(&self) -> usize`
  - `pub fn is_empty(&self) -> bool`
  - `pub fn paths(&self) -> &[PathBuf]`
  - `pub fn into_paths(self) -> Vec<PathBuf>`
  - `Debug` impl prints `DroppedFiles { count: N }`.
  - Constructed by the library from the `HDROP` the Shell hands to the
    frame's wndproc; passed by value to the callback registered via
    `Frame::set_drop_files_callback`.

## Internal API (Windows only)
- `pub(crate) fn extract_paths_from_hdrop(hdrop: HDROP) -> Vec<PathBuf>` —
  the canonical two-call `DragQueryFileW` pattern: first call with a
  null buffer returns the required `TCHAR` count; second call fills the
  buffer. Iterates `0..count` to produce the final `Vec<PathBuf>`.
- `pub(crate) fn finish_drop(hdrop: HDROP)` — wrapper around
  `DragFinish`. Called by the wndproc **after** the user callback returns,
  to release the Shell's internal storage backing the `HDROP`.

## Quick start

```rust,no_run
use ru_wx::prelude::*;
use ru_wx::DroppedFiles;

// 1. The standard "drop files from Explorer onto my window" workflow.
//    Call this *before* `frame.show()`; the bar just sets a single
//    `WM_DROPFILES` handler on the frame's HWND.
let mut frame = Frame::builder()
    .with_title("Drop files here")
    .with_size(600, 400)
    .build();

// 2. Register the callback. The callback receives a `DroppedFiles`
//    by value; you can iterate, query the count, or consume the inner Vec.
frame.set_drop_files_callback(|files: DroppedFiles| {
    println!("Dropped {} file(s):", files.len());
    for path in files.paths() {
        println!("  - {}", path.display());
    }
    // Or consume the paths if you don't need to keep `files` around:
    // let v: Vec<PathBuf> = files.into_paths();
});

// 3. If you build a non-Windows host, the callback is silently never
//    invoked but the rest of the type system still compiles, so cross-
//    platform code does not need `#[cfg(target_os = "windows")]` guards.

// 4. For arbitrary data formats (text, in-app objects) or drag-over
//    feedback, use the OLE COM `IDropTarget` machinery in [`ole_dnd`](./ole_dnd.md).
//    The two protocols coexist and are not mutually exclusive.

// 5. Reading the paths after the fact (rare):
let sample = DroppedFiles::from_paths(vec![]);
assert!(sample.is_empty());
assert_eq!(sample.len(), 0);
```

Behind the scenes, `Frame::build` calls `DragAcceptFiles(hwnd, TRUE)` once and on every `WM_DROPFILES` the wndproc runs `extract_paths_from_hdrop` (the canonical two-call `DragQueryFileW` pattern), hands the result to the user callback, then calls `DragFinish` to release the Shell's internal storage.

## Public API on `Frame`
```rust
impl Frame {
    pub fn set_drop_files_callback<F: FnMut(DroppedFiles) + 'static>(&mut self, cb: F);
    // also: drop_files_handler(), drop_files_handler_or_global()
    // — see frame.rs for the storage accessors.
}
```

## Win32 / platform notes
- The Shell sends `WM_DROPFILES` with `wParam = HDROP` and `lParam = {pt}`.
  The frame's wndproc:
  1. Calls `DragAcceptFiles(hwnd, TRUE)` once on creation (already done
     in `Frame::build` when a drop callback is registered).
  2. On `WM_DROPFILES`, calls `extract_paths_from_hdrop(hdrop)`, hands
     the result to the user callback, then `DragFinish(hdrop)`.
- `0xFFFFFFFF` is the documented "give me the count" sentinel for
  `DragQueryFileW`.
- `len_tchars` from `DragQueryFileW` excludes the trailing NUL, so the
  buffer is allocated as `len_tchars + 1` slots and read with
  `String::from_utf16_lossy(&buf[..copied as usize])`. The `lossy`
  decoder is safe because Explorer always produces well-formed UTF-16
  file paths.
- HDROP is **not** thread-safe; the library never calls
  `DragQueryFileW` from a worker thread.
- **Non-Windows hosts**: the type is still reachable (and the registration
  method is exposed), but the callback is never invoked.

## Tests (5)
- `from_paths_then_paths_round_trips` — `DroppedFiles::from_paths(v)` then
  `paths()` returns the original `v` slice.
- `len_reports_the_underlying_vec_length` — covers 0 / 1 / 3 paths.
- `into_paths_returns_the_inner_vec_and_consumes_self` — `into_paths`
  yields the original `Vec<PathBuf>`.
- `paths_survive_non_ascii_unicode` — Latin-1 (`è`) and CJK (`文`) characters
  in a file name exercise the `String::from_utf16_lossy` path.
- `debug_does_not_panic_for_empty` / `debug_does_not_panic_for_many` —
  panic-freeness of the public `Debug` impl for 0 and 3 paths.

The actual `WM_DROPFILES` dispatch path is **not** unit-tested (it would
need a real `HWND` from the Shell); it is covered end-to-end by the
`examples/showcase_all` binary, which has a "drop files here" zone and
prints the resulting paths.

## Cross-references
- `frame.rs` — `WM_DROPFILES` wndproc, `set_drop_files_callback` storage.
- `ole_dnd.rs` — the COM-level complement for non-file data and for
  drag-over feedback.
- `lib.rs` / `prelude.rs` — re-export of `DroppedFiles`.

## Example
```rust,no_run
use ru_wx::prelude::*;
use ru_wx::DroppedFiles;

let frame = Frame::builder().with_title("D&D demo").build();
frame.set_drop_files_callback(|files: DroppedFiles| {
    println!("Dropped {} file(s):", files.len());
    for path in files.paths() {
        println!("  - {}", path.display());
    }
});
```
