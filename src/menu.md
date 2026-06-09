# `menu.rs` — Menu, MenuBar, MenuItem

Pure-Rust wrapper around Win32 native menus (`HMENU`). All public types in this
module are re-exported from the crate root by `prelude.rs` (`Menu`, `MenuBar`,
`MenuItem`, `MenuItemKind`).

## Purpose

- **`Menu`** is a single drop-down / popup menu (its native handle is always a
  *popup* HMENU; the bar wrapping happens in `MenuBar`).
- **`MenuBar`** is the menu strip attached to a `Frame` (its native handle is
  a plain `CreateMenu` HMENU).
- **`MenuItem`** is a single row inside a `Menu`. The same struct stores the
  metadata for both kinds of items: `Normal`, `Check`, `Radio`, plus an
  optional `Accelerator` shortcut.

## Public types

```rust
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: u16,                    // command id (allocated via next_menu_id())
    pub label: String,              // includes the "&" mnemonic marker
    pub enabled: bool,
    pub kind: MenuItemKind,         // Normal | Check | Radio
    pub shortcut: Option<Accelerator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItemKind { Normal, Check, Radio }

#[derive(Clone)]
pub struct Menu { /* Rc<RefCell<MenuInner>> */ }

pub struct MenuBar { /* Vec<Menu> + HMENU */ }
```

## `Menu` — public API

| Method | Purpose |
|---|---|
| `new(title)` | Create an empty popup menu; `title` is the caption used when this menu is appended to a `MenuBar`. |
| `append(label, frame, F)` | Append a normal item. Returns the command id. |
| `append_disabled(label)` | Append a greyed-out item. |
| `append_with_colour_icon(label, fg, bg, F)` | Item with a colour filled-rect icon. |
| `append_with_svg_icon(label, svg, F)` | Item with a rendered SVG icon. |
| `append_check_item(label, frame, F)` | Checkable item. |
| `append_radio_item(label, frame, F)` | Radio item. |
| `append_with_shortcut(label, acc, frame, F)` | Normal item with a right-aligned shortcut hint. |
| `append_disabled_with_shortcut(label, acc)` | Disabled item with shortcut. |
| `append_check_item_with_shortcut(label, acc, frame, F)` | Check item with shortcut. |
| `append_radio_item_with_shortcut(label, acc, frame, F)` | Radio item with shortcut. |
| `append_separator()` | Append a `MF_SEPARATOR` line. |
| `check_item(id, check) -> bool` | Toggle the check state of a check/radio item. |
| `is_item_checked(id) -> Option<bool>` | Read the current check state. |
| `item(id) -> Option<&MenuItem>` | Lookup a stored item by id. |
| `item_by_id_mut(id) -> Option<&mut MenuItem>` | Mutable lookup (used by the Frame's accelerator-replace path). |
| `update_item_shortcut(id, new_acc) -> bool` | Update an item's shortcut and rebuild its label. |
| `items() -> &[MenuItem]` | All items in insertion order. |
| `hmenu() -> HMENU` | Raw Win32 handle (Windows only). |
| `popup_at_cursor(hwnd)` | Show the menu at the current cursor position relative to `hwnd`. |
| `title() -> &str` | The menu's caption. |

`Drop` deletes all `HBITMAP` handles owned by the menu (icons).

## `MenuBar` — public API

| Method | Purpose |
|---|---|
| `new()` | Create an empty menu bar. |
| `append(menu)` | Attach a `Menu` as a drop-down. Calls `AppendMenuW(MF_POPUP, ...)`. |
| `update_item_shortcut(id, new_acc) -> bool` | Walks every sub-menu in insertion order, first match wins. |
| `hmenu()` (pub(crate)) | Raw HMENU handle. |

## Right-aligned shortcut labels

`menu_label(label, shortcut)` formats the visible text as
`"{label}\t{acc}"`. The `\t` is the Win32 marker for right-aligned-tabbed text
inside a menu item, so the accelerator hint (`Ctrl+S`, `F5`, ...) shows up
aligned to the right edge of the menu.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Create the frame that will own the menu bar.
let frame = Frame::new("Menu demo", 800, 600);

// 2. Build a "File" menu with normal items, a check item, and a
//    separator. The closures receive the command id of the item.
let file_menu = Menu::new("&File");
file_menu.append("&New", &frame, |_id| {
    println!("New");
});
file_menu.append("&Open...", &frame, |_id| {
    println!("Open");
});
file_menu.append_separator();
let _auto_save_id = file_menu.append_check_item("Auto-save", &frame, |_id| {
    // toggled via file_menu.check_item(id, true) below
});
file_menu.append("E&xit", &frame, |_id| {
    frame.close();
});

// 3. Add a keyboard shortcut to an item with `append_with_shortcut`.
//    The accelerator hint is right-aligned in the menu (`{label}\t{acc}`).
let edit_menu = Menu::new("&Edit");
edit_menu.append_with_shortcut(
    "&Find...",
    Accelerator::new(KeyCode::F, Modifiers::CTRL),
    &frame,
    |_id| println!("Find"),
);
edit_menu.append_with_shortcut(
    "&Save",
    Accelerator::new(KeyCode::S, Modifiers::CTRL),
    &frame,
    |_id| println!("Save"),
);

// 4. Build the menu bar and append the menus in the order they should
//    appear on the strip.
let menu_bar = MenuBar::new();
menu_bar.append(file_menu);
menu_bar.append(edit_menu);
frame.set_menu_bar(menu_bar);

// 5. Toggle a check item programmatically.
let _ = file_menu.check_item(_auto_save_id, true);
let checked: Option<bool> = file_menu.is_item_checked(_auto_save_id);

// 6. Show a menu as a popup (right-click context menu) at the cursor.
let context_menu = Menu::new("Context");
context_menu.append("Cut", &frame, |_| println!("Cut"));
context_menu.append("Copy", &frame, |_| println!("Copy"));
context_menu.append("Paste", &frame, |_| println!("Paste"));
context_menu.popup_at_cursor(frame.hwnd());
```

**Typical workflow**

1. `Menu::new("&File")` for each drop-down (the `&` marks the
   mnemonic — `Alt+F` opens it).
2. `menu.append(label, &frame, |id| { … })` for each item. Use
   `append_with_shortcut(label, accelerator, &frame, …)` to show a
   right-aligned `Ctrl+S` hint.
3. Use `append_check_item` for toggles, `append_radio_item` for radio
   groups, `append_separator` for visual dividers.
4. `MenuBar::new()`, then `menu_bar.append(file_menu)`, etc.
5. `frame.set_menu_bar(menu_bar)` to attach the strip to the frame.
6. `menu.popup_at_cursor(frame.hwnd())` to show a menu as a popup
   (e.g. inside a right-click handler).

**Shortcuts and mnemonics**

- `&` in a label underlines the next character for `Alt`-key navigation
  (`&File` → `Alt+F`). Use `&&` to render a literal `&`.
- `append_with_shortcut` formats the visible text as `"{label}\t{acc}"`
  — the tab is the Win32 marker for right-aligned text inside a menu,
  so `Ctrl+S` shows up at the right edge.
- `menu_bar.update_item_shortcut(id, new_acc)` updates the shortcut
  in place; the label is rebuilt and `ModifyMenuW` re-paints the row.

## Win32 notes

- `Menu::new` uses **`CreatePopupMenu`** (a menu is *always* a popup;
  promotion to a drop-down happens when `MenuBar::append` adds it via
  `AppendMenuW(..., MF_POPUP, hmenu, ...)`).
- `MenuBar::new` uses **`CreateMenu`** (a plain menu bar handle).
- Local constants (not exported by `windows-sys 0.59`):
  - `MF_UNCHECKED_LOCAL = 0x0000`
  - `MF_RADIOCHECK_LOCAL = 0x0000_0200` (the radio dot glyph)
- The `&` character in a label is the **mnemonic marker** (`&File` underlines
  `F` and binds <kbd>Alt+F</kbd>). Use `&&` to render a literal `&`.
- `popup_at_cursor` uses **`SetForegroundWindow`** + **`TrackPopupMenu`** +
  **`PostMessageW(WM_NULL)`**. The `WM_NULL` post is the documented fix for
  the menu not closing when the user clicks outside it.

## Tests

7 unit tests, all about the shortcut update path:

- `Menu::update_item_shortcut` — applies new shortcut, label rebuilds, `false` for unknown id.
- `Menu::update_item_shortcut_no_change` — `None` shortcut clears the label suffix.
- `Menu::update_item_shortcut_separator` — separator rows are a no-op.
- `MenuBar::update_item_shortcut` — walks submenus, first match wins, returns `false` if missing.

## Cross-references

- [`accelerator.md`](accelerator.md) — the `Accelerator` type stored in
  `MenuItem.shortcut`. The `Frame::replace_accelerator` method calls
  `MenuBar::update_item_shortcut` to keep menu hints in sync.
- [`frame.md`](frame.md) — `Frame::set_menu_bar(&MenuBar)` attaches the bar.
- [`popup_menu.md`](popup_menu.md) — thin wrapper that delegates to `Menu`.
- [`prelude.md`](prelude.md) — re-exports the public types.
