# tree_ctrl

`TreeCtrl` — wxWidgets-style hierarchical list. Backed by Win32 `SysTreeView32` (the standard
"tree view" common control).

## When to use

- Displaying a hierarchy (file system, org chart, outline, settings tree, …).
- You need add/remove/expand/collapse and a selection-change callback.

## Public types

```rust
/// A handle to a single tree item (wraps an `HTREEITEM` = `isize`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeItem(pub isize);

#[derive(Clone)]
pub struct TreeCtrl { /* Rc<RefCell<TreeCtrlInner>> */ }
```

`TreeItem` is a transparent newtype around `isize` so it can be safely `Copy` and stored in
user data structures.

## Public API

```rust
impl TreeCtrl {
    /// Create a new tree view as a child of `frame`.
    /// Initial rect: 200×200 at (0, 0). Resize via sizer.
    pub fn new(frame: &Frame) -> Self;

    /// Add a root-level item, return its `TreeItem` handle.
    pub fn add_root(&self, text: &str) -> TreeItem;

    /// Append `text` as a child of `parent`, return its `TreeItem`.
    pub fn append_item(&self, parent: TreeItem, text: &str) -> TreeItem;

    /// Delete `item` and all its descendants.
    pub fn delete_item(&self, item: TreeItem);

    /// Delete every item in the tree.
    pub fn delete_all_items(&self);

    /// Current selection, or `None` if nothing is selected.
    pub fn get_selection(&self) -> Option<TreeItem>;

    /// Change the text of an existing item.
    pub fn set_item_text(&self, item: TreeItem, text: &str);

    /// Expand (show children) of an item.
    pub fn expand(&self, item: TreeItem);

    /// Collapse (hide children) of an item.
    pub fn collapse(&self, item: TreeItem);

    /// Register a callback that fires when the user picks a
    /// different item. Receives the new selection, or `None` if
    /// the selection was cleared.
    ///
    /// Internally registers a `WM_NOTIFY` handler on the parent
    /// `Frame` that filters for `TVN_SELCHANGED`, then re-queries
    /// the tree for the current selection (`TVM_GETNEXTITEM` /
    /// `TVGN_CARET`).
    pub fn on_selection_change<F: FnMut(Option<TreeItem>) + 'static>(
        &self,
        frame: &Frame,
        callback: F,
    );

    pub fn id(&self) -> u16;
    pub fn as_widget_ref(&self) -> WidgetRef;
}
```

## Quick start

A complete, copy-pasteable "file system outline" example that puts a tree
in a sizer and reacts to selection changes.

```rust,no_run
use ru_wx::prelude::*;

fn build_tree(frame: &Frame) -> TreeCtrl {
    let tree = TreeCtrl::new(frame);

    // Roots (top-level entries).
    let src    = tree.add_root("Source");
    let docs   = tree.add_root("Docs");
    let readme = tree.add_root("README.md");

    // Children of "Source".
    let main_rs = tree.append_item(src, "main.rs");
    let lib_rs  = tree.append_item(src, "lib.rs");
    tree.append_item(src, "Cargo.toml");

    // Children of "Docs".
    tree.append_item(docs, "guide.md");
    tree.append_item(docs, "api.md");

    // A grandchild.
    tree.append_item(main_rs, "// todo: clean up");

    // React to selection changes. The handler receives the new
    // selection, or `None` if the user cleared it.
    tree.on_selection_change(frame, |selected| {
        if let Some(item) = selected {
            println!("now selected: {:?}", item);
        } else {
            println!("selection cleared");
        }
    });

    // Programmatic operations.
    tree.expand(src);                          // show children of "Source"
    tree.set_item_text(lib_rs, "lib_v2.rs");   // rename
    if let Some(current) = tree.get_selection() {
        println!("current selection: {:?}", current);
    }
    // tree.delete_all_items();                // wipe the tree
    tree
}
```

**Typical workflow**

1. Create the tree with `TreeCtrl::new(frame)`. It is a 200×200 child at
   `(0, 0)` — resize it through a sizer, not by direct `MoveWindow`.
2. Populate it with `add_root` (one per top-level entry) then
   `append_item(parent, text)` to attach children. Items are returned as
   `TreeItem` handles you can keep around to mutate them later.
3. Register a selection callback with `on_selection_change(frame, |item| ...)`.
   The callback fires on `TVN_SELCHANGED`; re-entrancy-safe (take/call/put).
4. Drive the tree programmatically with `expand` / `collapse` /
   `set_item_text` / `delete_item` / `delete_all_items` / `get_selection`.
5. Pass the tree to a sizer via `as_widget_ref()` and let the sizer own
   the rectangle.

**Notes**

- `TreeItem` is a transparent `Copy` newtype around `isize` (an
  `HTREEITEM`). Treat it as opaque — never invent values; only use handles
  that came back from `add_root` / `append_item` / `get_selection`.
- Items are stored in the order they were inserted. Renaming via
  `set_item_text` does not change the handle, only the visible text.
- `delete_item` removes the item **and all its descendants**. Use
  `delete_all_items` to clear the whole tree in one call.
- The tree is rooted at the frame, not at a `Panel` — even when you put it
  inside a panel sizer, the `HWND` is a direct child of the frame.

## Win32 notes

- Window class: `SysTreeView32`. Styles: `WS_CHILD | WS_VISIBLE | WS_BORDER | TVS_HASLINES |
  TVS_LINESATROOT | TVS_HASBUTTONS` (classic "Explorer" look).
- Two local FFI structs — `TVINSERTSTRUCTW` and `TVITEMW` — are declared `#[repr(C)]` to
  match `<commctrl.h>` exactly. This guarantees the same memory layout regardless of the
  version of `windows-sys` in use.
- Local Win32 message constants:
  - `TVM_INSERTITEMW = 0x1132`
  - `TVM_DELETEITEM = 0x1101`
  - `TVM_EXPAND = 0x1102`
  - `TVM_GETNEXTITEM = 0x110A`
  - `TVM_SETITEMW = 0x113F`
  - `TVGN_CARET = 9`, `TVE_EXPAND = 2`, `TVE_COLLAPSE = 1`
  - `TVN_SELCHANGED = 0xFFFFFE6E`
  - `TVI_ROOT = 0xFFFF0000`, `TVI_LAST = 0xFFFF0002`
  - `TVS_HASLINES = 2`, `TVS_LINESATROOT = 4`, `TVS_HASBUTTONS = 1`
  - `TVIF_TEXT = 1`
- `add_root` and `append_item` build a `TVINSERTSTRUCTW { h_parent, h_insert_after = TVI_LAST,
  item: TVITEMW { mask = TVIF_TEXT, psz_text = <wide ptr>, ... } }` and pass it to
  `SendMessageW(TVM_INSERTITEMW)`. The return value is the new `HTREEITEM` (an `isize`).
- `set_item_text` uses `TVM_SETITEMW` with a single-field `TVITEMW` (`mask = TVIF_TEXT`,
  `h_item = item.0`, `psz_text = <wide ptr>`).
- `get_selection` uses `TVM_GETNEXTITEM(TVGN_CARET)`. Returns 0 when nothing is selected.
- `on_selection_change` registers a `WM_NOTIFY` handler on the parent `Frame`. The handler:
  1. Filters for `TVN_SELCHANGED`.
  2. Re-queries the tree (`TVM_GETNEXTITEM` / `TVGN_CARET`).
  3. Take/call/put fires the user callback with the new selection (or `None`).
- `next_control_id()` allocates a unique control id (process-global `AtomicU16` counter
  starting at 100).

## Tests

No unit tests in this module. Manual coverage via `examples/minitest/mt_tree_ctrl.rs`.

## Cross-references

- [frame](frame.md) — `TreeCtrl::new` takes a `Frame`; `on_selection_change` registers a
  notify handler on the frame.
- [widget](widget.md) — `as_widget_ref()` for use with sizers.
- [sizer](sizer.md) — typical layout is "tree on the left, content on the right" inside a
  horizontal `BoxSizer` or a `FlexGridSizer` with a growable right column.
- [list_ctrl](list_ctrl.rs) — flat counterpart (no hierarchy).
- [prelude](prelude.md)
