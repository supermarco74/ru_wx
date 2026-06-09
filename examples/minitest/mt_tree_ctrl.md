# mt_tree_ctrl.rs

Minitest for [`TreeCtrl`](file:///f:/code/ru_wx/ru_wx/src/tree_ctrl.rs) — a hierarchical item tree populated programmatically, with selection reporting to the status bar.

**Run:** `cargo run --example mt_tree_ctrl`

## Purpose
1. Build a tree with a root and several levels of children using `add_root` / `append_item`
2. Expand specific items programmatically with `tree.expand`
3. React to selection changes with `on_selection_change`
4. Document the `TreeItem` handle that the selection callback receives

## Top-level flow
1. Frame 420×460.
2. `StaticText` hint: `"Click an item to see its label in the status bar."`
3. 1-field `StatusBar` with default text `"Select a node…"`.
4. `TreeCtrl::new(&frame)`.
5. **Build the tree**
   - `let root = tree.add_root("Project");`
   - `let src = tree.append_item(root, "src");`
     - `tree.append_item(src, "main.rs");`
     - `tree.append_item(src, "lib.rs");`
     - `let modules = tree.append_item(src, "modules");`
       - `tree.append_item(modules, "auth.rs");`
       - `tree.append_item(modules, "db.rs");`
       - `tree.append_item(modules, "ui.rs");`
   - `let assets = tree.append_item(root, "assets");`
     - `tree.append_item(assets, "logo.png");`
     - `tree.append_item(assets, "styles.css");`
   - `let docs = tree.append_item(root, "docs");`
     - `tree.append_item(docs, "README.md");`
     - `tree.append_item(docs, "CHANGELOG.md");`
6. **Expand** the root and the `src` branch so the user sees the project structure on launch.
7. **Selection callback** — `tree.on_selection_change(&frame, move |item: Option<TreeItem>| match item { Some(it) => set_status_text(&format!("Selected item handle: {}", it.0), 0), None => set_status_text("(no selection)", 0) })`. The callback fires whenever the user clicks (or keyboard-navigates to) a new item; `None` means "no current selection".
8. Vertical `BoxSizer` containing only the tree; `app.run(frame)`.

> The callback **prints the raw handle value** (e.g. `Selected item handle: 12345678`), not the textual label. `TreeCtrl` does **not** expose `get_item_text`; if you need the label on click, store it yourself when you build the tree (e.g. in a `HashMap<TreeItem, String>`).

## Key APIs exercised
- [`TreeCtrl::new(&frame)`](file:///f:/code/ru_wx/ru_wx/src/tree_ctrl.rs)
- `TreeCtrl::add_root(&str) -> TreeItem`
- `TreeCtrl::append_item(TreeItem parent, &str label) -> TreeItem`
- `TreeCtrl::expand(TreeItem)`
- `TreeCtrl::on_selection_change(&frame, FnMut(Option<TreeItem>))`
- The `TreeItem` newtype (currently a `pub struct TreeItem(pub HTREEITEM)`; the `.0` field exposes the raw Win32 handle)

## Patterns worth noting
- **Build the tree imperatively** — there is no `insert` API; you `add_root` once, then `append_item` for every node. The shape of the tree is whatever order you call `append_item` in. A depth-first, post-order traversal (build child subtrees before adding siblings) keeps the code linear.
- **`TreeItem` is the universal handle** — every mutating or query method takes a `TreeItem`, and the selection callback hands you one. Treat it as opaque (or, for the brave, read `.0` for the raw Win32 `HTREEITEM`).
- **No `get_item_text` is provided** — see the workaround in the callout above. If your selection handler needs the label, build a side-index `HashMap<TreeItem, String>` while you build the tree.
- **Expand on launch** — `tree.expand(root)` is what makes the tree useful on first paint. By default a freshly built `TreeCtrl` shows only the root collapsed.
- **`Option<TreeItem>` is the selection type** — `None` means "the user has deselected everything" (e.g. clicked the client area outside any item). Handle both arms explicitly, like this example does.

## Win32 notes
- `TreeCtrl` is a native `SysTreeView32` (`WC_TREEVIEW`) with `TVS_HASLINES | TVS_LINESATROOT | TVS_HASBUTTONS | TVS_SHOWSELALWAYS` for the standard file-explorer look.
- `add_root` issues `TVM_INSERTITEMW` with `TVI_ROOT` and a `TVINSERTSTRUCTW` whose `item.pszText` is the label.
- `append_item` is `TVM_INSERTITEMW` with a `hParent` of the supplied `TreeItem` (`HTREEITEM`).
- `expand` issues `TVM_EXPAND` with `TVE_EXPAND` (partial expand) or `TVE_EXPAND | TVE_EXPANDPARTIAL` (recursive, depending on `ru_wx`'s mapping).
- `on_selection_change` registers a `WM_NOTIFY` / `TVN_SELCHANGEDW` filter on the parent frame and dispatches the new `HTREEITEM` (wrapped in `TreeItem`, or `None`) to the closure.

## Cross-references
- [`tree_ctrl.md`](file:///f:/code/ru_wx/ru_wx/src/tree_ctrl.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md)
- [`frame.md`](file:///f:/code/ru_wx/ru_wx/src/frame.md)
