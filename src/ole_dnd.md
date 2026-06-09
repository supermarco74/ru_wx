# `src/ole_dnd.rs` — OLE COM `IDropTarget` (destination side)

## Purpose
The OLE COM half of Win32 drag-and-drop. Registers an `IDropTarget` with the
shell so the application can receive **any** `IDataObject` payload (text,
files, custom formats) with live drag-over feedback. Complements
[`drop_target.rs`](drop_target.md), which is the simpler Shell-level
`WM_DROPFILES` path. The two coexist on the same frame; the Shell picks
the one that matches the data source.

This module is **destination-side only** — it does not yet expose a
`DoDragDrop` wrapper for an in-app widget acting as a source.

## Key types

### `OleDropEffect(pub u32)` — newtype around `DROPEFFECT` bits
- Associated constants: `NONE(0)`, `COPY(1)`, `MOVE(2)`, `LINK(4)`,
  `SCROLL(0x8000_0000)` — the five canonical Win32 values.
- `bits()`, `from_bits_truncate(bits)`, `is_none()`, `contains(other)`,
  `remove(other)`, `intersect(other)`.
- `BitOr`, `BitOrAssign`, `BitAnd`, `BitAndAssign` for composition:
  `OleDropEffect::COPY | OleDropEffect::MOVE` = "either, source's choice".
- `Copy + Clone + Default + Hash + Eq`. The `Debug` impl pretty-prints
  the known bits and adds `UNKNOWN` for any remainder.

### `OleDroppedData` (enum)
- `Files(Vec<PathBuf>)` — `CF_HDROP` decoded.
- `Text(String)` — `CF_UNICODETEXT` decoded.
- `Other` — format not extracted; matches the "unknown format" case.

### `OleDropPosition { pub x: i32, pub y: i32 }`
- Drop location in **client coordinates** of the receiving window.
- `new(x, y)`, `Default`, `Copy + Hash`.

### `OleDropError`
- `RegisterFailed(i32)` — `RegisterDragDrop` returned non-zero `HRESULT`.
- `Display` prints `"OLE RegisterDragDrop failed with HRESULT 0xhhhhhhhh"`.
- Implements `std::error::Error`.

### `OleDropTarget` (Windows handle)
- Owns the reference-counted COM object (refcount=1 at construction).
- `register(hwnd: HWND) -> Result<(), OleDropError>` — calls
  `RegisterDragDrop`; on success caches the `HWND`.
- `hwnd() -> Option<HWND>` — the window it's registered with, if any.
- `Drop` calls `RevokeDragDrop(hwnd)` (if registered) and then releases
  the IUnknown refcount, freeing the COM object + payload.
- `unsafe impl Send` (not `Sync`) — the user callback is `FnMut` and
  lives behind a `RefCell`.

## Internal: `mod win` (Windows-only)
The COM vtable plumbing:
- `IUnknownVtbl`, `IDropTargetVtbl` — `#[repr(C)]` layout, field names
  PascalCase per the COM ABI.
- `OleDropTargetComObject { vtable, payload }` — what `RegisterDragDrop`
  receives.
- `OleDropTargetPayload { refcount: AtomicU32, callback: RefCell<...>,
  last_data_object: AtomicI32 }` — per-instance state, heap-allocated.
- vtable functions: `query_interface`, `add_ref`, `release`,
  `drag_enter`, `drag_over`, `drag_leave`, `drop_vtable` (named
  `drop_vtable` to avoid shadowing `std::mem::drop`).
- `IDataObject::GetData` is dispatched at vtable slot 3
  (3 IUnknown methods, then GetData) via `core::mem::transmute`.
- `read_hdrop` / `read_unicode_text` — extract the two supported formats
  using `FORMATETC` + `STGMEDIUM`; cleanup with `ReleaseStgMedium`.
- `ensure_ole_initialized` — `OleInitialize` once per process, gated by
  `std::sync::Once`. **Not** paired with `OleUninitialize` (the process
  is expected to live for the rest of its lifetime after this point).
- `register` / `unregister` — raw `RegisterDragDrop` / `RevokeDragDrop`
  wrappers, marked `unsafe`.

## Public API on `Frame`
```rust
impl Frame {
    pub fn set_ole_drop_callback<F>(&mut self, cb: F)
    where F: FnMut(OleDroppedData, OleDropPosition) + 'static;
}
```

## Win32 / platform notes
- The implementation accepts **every** drop (`DROPEFFECT_COPY` returned
  unconditionally in `DragEnter` and `DragOver`). Per-effect / per-key
  refinement is a future cycle.
- Format preference in `drop_vtable`: `CF_HDROP` (15) first, then
  `CF_UNICODETEXT` (13). Both are hard-coded (`windows-sys 0.59` doesn't
  export them) because they belong to the clipboard format registry, not
  the Win32 API.
- `CF_UNICODETEXT` layout per the spec: a `u32` byte-length (not char
  count) followed by a UTF-16LE string. We read the byte length, divide
  by 2, subtract 1 to drop the NUL, then `String::from_utf16_lossy`.
- `query_interface` returns `E_NOINTERFACE` for anything other than
  `IID_IUnknown` / `IID_IDropTarget` (the two IIDs are spelled out as
  raw byte arrays; windows-sys 0.59 doesn't expose them either).
- Allocations: COM object + payload are both `Box`-allocated. The
  `release` vtable function is the sole owner of both on the final
  `Release`.
- **Non-Windows hosts**: `OleDropTarget` is a no-op placeholder
  (`register` returns `Ok(())` without doing anything); the types are
  still reachable on every platform.

## Tests (10)
All platform-agnostic; they test the public data types and do not
require a real `HWND`:
- `ole_drop_effect_standard_bits_match_win32` — `COPY == 1`, `MOVE == 2`,
  `LINK == 4`, `SCROLL == 0x8000_0000`.
- `ole_drop_effect_is_none_round_trips` — `is_none()` is the
  `bits() == 0` test.
- `ole_drop_effect_from_bits_truncate_never_panics` — including 0xFFFF_FFFF.
- `ole_drop_effect_bitor_composes_bits` — `COPY | MOVE == 3`; `|=` works.
- `ole_drop_effect_is_copy_and_hash` — the type is `Copy` and `Hash`.
- `ole_drop_position_new_stores_xy` — `new(11, 22)` stores the pair;
  `default()` is the origin.
- `ole_drop_position_is_copy` — value move.
- `ole_dropped_data_variants_match` — `Files` / `Text` / `Other` match.
- `ole_drop_error_is_copy_and_eq` — `Copy + PartialEq` over two HRESULTs.
- `ole_drop_error_display_includes_hresult` — `Display` includes the
  lower-case hex `0x80040100` for `E_NOTIMPL`-style failures.
- `ole_drop_error_is_std_error` — `Box<dyn Error>` round-trips and
  `{:?}` mentions the variant.

The integration test for the vtable dispatch is `examples/showcase_all.rs`,
which performs a programmatic `IDataObject` drop and checks the callback
fires.

## Cross-references
- `drop_target.rs` — Shell-level `WM_DROPFILES` (files only, no drag-over
  feedback). The two protocols coexist; `Frame::set_drop_files_callback`
  and `Frame::set_ole_drop_callback` are independent.
- `frame.rs` — owns the `OleDropTarget` storage and the wndproc that
  dispatches the COM callbacks to user code.
- `lib.rs` / `prelude.rs` — re-exports `OleDropEffect`, `OleDroppedData`,
  `OleDropPosition`, `OleDropError`, `OleDropTarget`.

## Quick start

A complete, copy-pasteable "drop receiver" example: a frame that accepts
both file drops (`CF_HDROP`) and text drops (`CF_UNICODETEXT`), logging
the payload and the cursor position.

```rust,no_run
use ru_wx::prelude::*;

fn install_ole_drop(frame: &mut Frame) {
    // 1. Register the drop callback. The closure runs on every drop
    //    that the shell delivers to this frame.
    frame.set_ole_drop_callback(|data, pos| {
        match data {
            // Explorer / Finder-style file drops.
            OleDroppedData::Files(paths) => {
                println!("{} file(s) at ({}, {}):", paths.len(), pos.x, pos.y);
                for p in paths {
                    println!("  {}", p.display());
                }
            }
            // Any source that produces Unicode text.
            OleDroppedData::Text(s) => {
                println!("text drop at ({}, {}): {:?}", pos.x, pos.y, s);
            }
            // The source used a format we don't decode.
            OleDroppedData::Other => {
                println!("unknown format at ({}, {})", pos.x, pos.y);
            }
        }
    });
}

// (Optional) Build an OleDropTarget explicitly for advanced uses — e.g.
// to introspect the registered HWND, or to drop the registration
// independently of the frame.
#[cfg(target_os = "windows")]
fn build_explicit_target(frame: &Frame) -> Result<OleDropTarget, OleDropError> {
    let target = OleDropTarget::register(frame.hwnd())?;
    println!("OLE drop target attached to {:?}", target.hwnd());
    Ok(target)
    // target's Drop impl calls RevokeDragDrop(hwnd) and releases
    // the COM object, so just letting it go out of scope is fine.
}
```

**Typical workflow**

1. Build the frame with `Frame::builder().with_title(...).build()`.
2. Call `frame.set_ole_drop_callback(|data, pos| ...)` with a closure
   that matches on `OleDroppedData`. The closure runs for every drop
   the shell delivers; both files and text are decoded automatically.
3. (Optional, advanced) build an `OleDropTarget` directly with
   `OleDropTarget::register(hwnd)` to introspect the HWND or to control
   the lifetime independently. `Drop` calls `RevokeDragDrop(hwnd)` and
   releases the COM refcount.
4. (Optional) combine with `Frame::set_drop_files_callback` — the
   Shell-level `WM_DROPFILES` path and the COM `IDropTarget` path
   coexist; the Shell picks the one matching the data source.

**Notes**

- This is the **destination side** only. There's no `DoDragDrop` wrapper
  for an in-app widget acting as a *source* yet.
- The crate currently accepts **every** drop (returns
  `DROPEFFECT_COPY` unconditionally). Per-effect / per-key refinement
  (e.g. only accept when `Shift` is held) is a future cycle.
- Format preference in the drop decoder: `CF_HDROP` (file list) first,
  then `CF_UNICODETEXT`. Both are decoded; if neither matches,
  `OleDroppedData::Other` is delivered so the callback always fires.
- The `OleDropEffect` bit pattern matches Win32 `DROPEFFECT_*`
  (`COPY=1`, `MOVE=2`, `LINK=4`, `SCROLL=0x8000_0000`). Use `|`, `&`,
  `Copy` like any bitflags type.
- `OleDropPosition` is in **client coordinates** of the receiving
  window — `(0, 0)` is the upper-left of the frame's client area, not
  the screen.
- Cross-platform: the public types and `set_ole_drop_callback` compile
  on every platform. On non-Windows hosts, `OleDropTarget::register`
  is a no-op that returns `Ok(())`, and the callback never fires.

## Example
```rust,no_run
use ru_wx::prelude::*;

let frame = Frame::builder().with_title("OLE d&d").build();
frame.set_ole_drop_callback(|data, pos| {
    match data {
        OleDroppedData::Files(paths) =>
            println!("{} file(s) at ({}, {})", paths.len(), pos.x, pos.y),
        OleDroppedData::Text(s) =>
            println!("text ({}, {}): {}", pos.x, pos.y, s),
        OleDroppedData::Other =>
            println!("unknown format at ({}, {})", pos.x, pos.y),
    }
});
```
