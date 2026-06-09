# image_list

`ImageList` — a thin Win32 `HIMAGELIST` wrapper for tiny icons (16×16, 32×32, etc.). Consumed by
`Grid`, `Tab`, `ToolBar`, and any other control that draws an icon next to a row/column/page.

## When to use

- You need icons in a `Grid` (`set_image_list`), `Tab` (`set_image_list` with `add_page_with_image`),
  `ToolBar` (toolbar buttons), or any common control backed by `SysListView32` /
  `SysTreeView32` / `TabControl`.
- You want per-cell images without managing the underlying `HIMAGELIST` Win32 handle yourself.

## Public types

```rust
/// Opaque Win32 `HIMAGELIST` handle exposed as `isize` (matches
/// `windows-sys`'s raw handle convention).
pub type ImageListHandle = isize;

/// Owns a Win32 HIMAGELIST. Drop calls `ImageList_Destroy`.
pub struct ImageList {
    handle: ImageListHandle,  // not pub; use `handle()`
    width: i32,               // not pub; use `width()`
    height: i32,              // not pub; use `height()`
}
```

## Public API

```rust
impl ImageList {
    /// Create a new (empty) image list. `width` / `height` are pixel
    /// dimensions shared by every bitmap added via `add_bitmap`.
    pub fn new(width: i32, height: i32) -> Self;

    /// Append a pre-loaded `HBITMAP` (32-bit DIB section with alpha)
    /// to the list. Returns the assigned index, or `None` on failure.
    pub fn add_bitmap(&self, hbitmap: HBITMAP) -> Option<i32>;

    /// The raw `HIMAGELIST` (as `isize`) — hand this to Win32
    /// `SendMessageW(LVM_SETIMAGELIST, ...)` style APIs.
    pub fn handle(&self) -> ImageListHandle;

    /// Width in pixels.
    pub fn width(&self) -> i32;

    /// Height in pixels.
    pub fn height(&self) -> i32;
}
```

There is no public `Drop` for raw `HBITMAP`s — the caller still owns the bitmap passed to
`add_bitmap`. `ImageList_Destroy` is called for the list itself on `Drop`.

## Win32 notes

- Backed by `ImageList_Create` / `ImageList_Add` / `ImageList_Destroy` from `comctl32`.
- Created with `ILC_COLOR32` (0x20). No `ILC_MASK` — modern bitmaps carry their own alpha.
- The `HIMAGELIST` handle is shared with the underlying control, so the list typically lives
  as long as the consuming widget does. Cloning the `ImageList` is **not** supported; create
  one per control.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Create the list with a single shared icon size.
let mut icons = ImageList::new(16, 16);

// 2. Add HBITMAPs (32-bit DIBs with alpha). The caller still owns the
//    HBITMAP; the list stores a reference into ImageList_Add's table.
for svg_bytes in &[include_bytes!("../assets/icons/star.svg")] {
    if let Some(hbmp) = icon::svg_bytes_to_hbitmap(svg_bytes, 16, 16) {
        let idx = icons.add_bitmap(hbmp).expect("add bitmap");
        // `idx` is the position in the list (0, 1, 2, …).
        println!("added icon at index {idx}");
    }
}

// 3. Hand the list to a consumer:
let grid = Grid::new(&frame);
grid.set_image_list(&icons);

let tab = Tab::new(&frame);
tab.set_image_list(&icons);
tab.add_page_with_image("Overview", &panel_for_overview, /*image_index=*/ 0);

// 4. Or feed a toolbar:
//    toolbar.set_image_list(&icons);
//    toolbar.add_tool(1001, "Star", 0);

// 5. Low-level: read the raw HIMAGELIST handle for raw Win32 calls.
let himl = icons.handle();   // isize
// SendMessageW(grid.hwnd(), LVM_SETIMAGELIST, LVSIL_SMALL, himl as isize);
```

`ImageList` does not implement `Clone` — create one per consumer. The owning widget is responsible for keeping the list alive as long as it is in use.

## Cross-references

- [grid](grid.md) — primary consumer (small-icon slot via `LVM_SETIMAGELIST` / `LVSIL_SMALL`).
- [tab](tab.md) — uses images for tab icons (`add_page_with_image`).
- [icon](icon.md) — `svg_bytes_to_hbitmap` is the standard way to get an `HBITMAP` for `add_bitmap`.
- [prelude](prelude.md)
