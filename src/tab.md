# `tab.rs` — `Tab` (SysTabControl32 notebook wrapper)

A wrapper around the Win32 **`SysTabControl32`** common control. It exposes
a "notebook of pages", each backed by a [`Panel`](panel.md).

## Purpose

- The `Tab` control owns the *tab strip* (the row of tabs at the top).
- Each page is a `Panel` provided by the caller. The `Tab` sizes every
  page to the tab control's inner *display area* (the rectangle below the
  tab strip, inside the border) and shows only the selected page.

## Page parenting

The page `Panel`s are **direct children of the frame, not of the tab
control**. This is deliberate: it lets the existing frame-level command
dispatch (`register_command_handler`) keep working for controls inside a
page without routing the messages through the tab control.

## Public type

```rust
#[derive(Clone)]
pub struct Tab { /* Rc<RefCell<TabInner>> */ }
```

## Public API

| Method | Purpose |
|---|---|
| `new(frame) -> Self` | Create a 200×200 tab control as a child of `frame`. |
| `set_image_list(&self, image_list)` | Attach an `ImageList` for icons on tabs. |
| `add_page(&self, title, panel) -> i32` | Append a page; returns its index, or `-1` on failure. |
| `add_page_with_image(&self, title, panel, image_index) -> i32` | Append a page with an icon. |
| `get_page_count(&self) -> usize` | Number of pages (from `TCM_GETITEMCOUNT`). |
| `get_selection(&self) -> Option<usize>` | Currently-selected index, or `None` if no pages. |
| `set_selection(&self, index)` | Programmatically select a page; shows it, hides the others, fires the callback. |
| `on_selection_change(&self, frame, F)` | Register a `FnMut(usize)` callback for tab clicks. |
| `id() -> u16` | The control's Win32 id (allocated by `next_control_id()`). |
| `as_widget_ref(&self) -> WidgetRef` | For sizer interop. |

The first page added is selected by default; all others are hidden until
the user clicks their tab.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

let frame = Frame::new("Notebook", 800, 600);

// 1. Create a Tab control as a direct child of the frame.
let tab = Tab::new(&frame);

// 2. (Optional) attach an image list for tab icons.
//    let image_list = ImageList::new(16, 16);
//    image_list.add_svg(include_bytes!("../assets/icons/file-new.svg"))?;
//    tab.set_image_list(&image_list);

// 3. Build a Panel for each page. Pages are direct children of the
//    frame, NOT of the tab control — this is intentional and lets
//    the frame's command dispatch work for controls inside a page.
let page1 = Panel::new(&frame);
page1.set_background(Colour::WHITE);
let page2 = Panel::new(&frame);
page2.set_background(Colour::LIGHT_GREY);

tab.add_page("General", page1.clone());
tab.add_page("Advanced", page2.clone());
// tab.add_page_with_image("Settings", page3.clone(), 0); // if you set an image list

// 4. Listen for selection changes (receives the new page index).
tab.on_selection_change(&frame, |index| {
    println!("Tab changed to page {}", index);
});

// 5. Initial selection: page 0 (the first one added) is selected by
//    default. You can override with:
tab.set_selection(1);

// 6. Re-parent the tab into your sizer. The tab owns its tab strip;
//    the page Panels remain siblings of the tab inside the frame.
// let sizer = BoxSizer::vertical();
// sizer.add(tab.as_widget_ref());
// sizer.add_stretch(1);
// frame.set_sizer(sizer);
```

**Typical workflow**

1. `Tab::new(frame)` — creates a `SysTabControl32` child of the frame.
2. Optionally `tab.set_image_list(&image_list)` for tab icons.
3. For every page: `let panel = Panel::new(&frame);` and then
   `tab.add_page("title", panel.clone());` (or
   `tab.add_page_with_image("title", panel, image_index)`).
4. `tab.on_selection_change(&frame, |idx| { … })` for click handling.
5. `tab.set_selection(i)` to drive the page programmatically (e.g. open
   the "Errors" tab when validation fails). The new page is shown and
   the others are hidden; the callback also fires.
6. Add `tab.as_widget_ref()` to a sizer for normal layout-driven sizing.

**Insertion order matters**: `add_page` appends (uses
`page_panels.len()` as the insertion index), so the *i-th* `add_page`
call produces the *i-th* tab from the left. The selection callback
receives the index in the same order.

## Win32 notes

- Local **`TCITEMW`** struct (the layout-stable variant is defined here
  rather than relying on `windows-sys 0.59`, where the struct's layout
  has shifted across versions).
- Local constants:
  - `TCM_FIRST = 0x1300` (Tab Control Messages base).
  - `TCM_GETITEMCOUNT = TCM_FIRST + 4`.
  - `TCM_SETIMAGELIST = TCM_FIRST + 3`.
  - `TCM_INSERTITEM = TCM_FIRST + 7`.
  - `TCM_GETITEM = TCM_FIRST + 5`.
  - `TCM_GETCURSEL = TCM_FIRST + 11`.
  - `TCM_SETCURSEL = TCM_FIRST + 12`.
  - `TCM_ADJUSTRECT = TCM_FIRST + 40` (converts outer-rect ↔ inner-rect).
  - `TCIF_TEXT = 0x0001`, `TCIF_IMAGE = 0x0002`.
- Window class **`SysTabControl32`**, styles `WS_CHILD | WS_VISIBLE |
  WS_CLIPSIBLINGS | WS_TABSTOP` (clip-siblings so pages don't bleed into
  each other; tab-stop so the keyboard can land on the control).
- **Unicode mode is forced** with `TCM_SETUNICODEFORMAT` (wParam = TRUE).
  Even with a Common Controls v6 manifest, some hosts initialise the
  control as ANSI, which truncates a UTF-16 `TCITEMW.pszText` to its
  first code unit. The explicit format switch is the documented fix.

## Inserting at the right position

`add_page` and `add_page_with_image` pass `insert_at = page_panels.len()`
as the `wParam` of `TCM_INSERTITEM`. This *appends* in insertion order
matching the call order; passing `0` would put every new tab at the front,
which made a previously-debugged bug put the tabs in reverse order *and*
made every page appear "selected" because all four were positioned at the
same place.

## Laying out pages

`TabInner::layout_page` builds a `RECT` covering the full tab control and
sends it to `TCM_ADJUSTRECT` with `wParam = 0`. The call returns the
**inner display rect** (the area below the strip, inside the border); the
panel is then `MoveWindow`'d to that rect *and* `set_size`'d so its
sizer re-lays out.

The layout helper is called on every `set_position` and `set_size` so the
pages track the tab control when the frame is resized.

## Selection change notification

`on_selection_change` registers a `WM_NOTIFY` handler on the frame for
the tab control's id. When the user clicks a tab:

1. The handler queries `TCM_GETCURSEL` for the new index.
2. Updates `inner.selected`, shows the new page, hides the others
   (`ShowWindow(SW_SHOW/SW_HIDE)`).
3. **Takes** the user's callback out of the `RefCell`, calls it with the
   new index, then **puts it back** — the same take/call/put pattern
   used everywhere else in the frame to avoid re-entrancy under a
   `RefCell` borrow.

## Cross-references

- [`panel.md`](panel.md) — page content container; the `Tab` keeps a
  cloned `Panel` per page so it can re-layout on resize.
- [`frame.md`](frame.md) — supplies the parent HWND and the
  `register_notify_handler` mechanism.
- [`image_list.md`](image_list.md) — optional source of tab icons.
