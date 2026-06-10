# `tooltip.rs` — `ToolTip` (tooltips_class32 wrapper)

A wrapper around the Win32 **`tooltips_class32`** common control. Tooltips
are the small yellow popups that show on hover. The implementation
caches **one tooltip control per top-level window** so every hover-able
child of a frame can share the same HWND.

## Purpose

- A `ToolTip` is a *logical* binding between a `WidgetRef` (the hover
  target) and a piece of text.
- The first time a tooltip is bound to a widget, a *physical* tooltip
  control (HWND) is created as a child of the widget's top-level
  ancestor, and added to a global registry keyed by that top-level.
- Subsequent bindings in the same frame re-use the existing tooltip
  HWND and just add a new tool entry (`TTM_ADDTOOL`).
- A static `ToolTip::enable(true/false)` switch broadcasts
  `TTM_ACTIVATE` to every created tooltip control to globally
  enable/disable hover help.

## Public type

```rust
pub struct ToolTip { /* Rc<RefCell<ToolTipInner>> */ }
```

## Public API

| Method | Purpose |
|---|---|
| `new(text) -> Self` | Create a logical tooltip with the given text. |
| `text(&self) -> String` | Read the current text. |
| `set_text(&self, text)` | Replace the text (sends `TTM_UPDATETIPTEXT`). |
| `attach(&self, target: &WidgetRef)` | Bind the tooltip to `target` (creates / re-uses the per-top-level control and adds a tool entry). |
| `enable(enabled)` (static) | Globally enable or disable every tooltip. |
| `detach(target: &WidgetRef)` (static) | Remove the tool entry bound to `target`. |

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Bind a tooltip to any widget (button, panel, …) with a single call.
let tip = ToolTip::new("Click to save the document");
tip.attach(&button);

// 2. Change the text later in place (no re-attach needed).
tip.set_text("Save (Ctrl+S)");

// 3. Read the current text.
let t: String = tip.text();

// 4. Toggle all tooltips globally (e.g. for a "Show help" menu item).
ToolTip::enable(false);   // hide every tooltip in every frame
ToolTip::enable(true);    // show them again

// 5. Detach a specific tooltip (e.g. on widget destruction).
ToolTip::detach(&button);
```

The first `attach` call for a frame allocates **one** `tooltips_class32` HWND per top-level window and caches it in a global registry. Subsequent `attach` calls in the same frame just send `TTM_ADDTOOL` against the existing control, so the overhead of adding tooltips is one per top-level window, not one per widget.

## Win32 notes

- Window class **`tooltips_class32`**, styles `TTS_ALWAYSTIP | TTS_NOPREFIX`.
  - `TTS_ALWAYSTIP = 0x0000_0001` — show even when the owning window is
    inactive (typical for toolbars).
  - `TTS_NOPREFIX = 0x0000_0002` — don't strip the `&` mnemonic marker.
- Local **`TOOLINFOW`** struct (32 bytes) is defined in this file
  because `windows-sys 0.59` does not export a stable-layout version of
  it. The fields match the Win32 header (hence `non_snake_case`).
- Per-tool flags: `TTF_IDISHWND = 0x0000_0001` (the `uId` is a child HWND,
  not a 32-bit id) and `TTF_SUBCLASS = 0x0000_0010` (let the tooltip
  control subclass the target to track mouse events).
- Per-control messages:
  - `TTM_ACTIVATE = WM_USER + 1` — toggle globally on/off.
  - `TTM_ADDTOOL = WM_USER + 4` — add a tool entry.
  - `TTM_DELTOOL = WM_USER + 5` — remove a tool entry.
  - `TTM_UPDATETIPTEXT = WM_USER + 12` — change a tool's text in place.

## Per-top-level caching

The implementation walks the widget tree with
**`GetAncestor(target, GA_ROOT)`** to find the top-level frame, then
**`FindWindowExW`** in a `OnceLock<Mutex<Vec<HWND>>>` cache to find or
create a tooltip control for that root. This means:

- One `tooltips_class32` HWND per top-level window, not per tooltip.
- All `attach` calls in the same frame share the same physical control.
- `ToolTip::enable(true/false)` walks the cache and posts
  `TTM_ACTIVATE` to each entry.

## `attach` flow

1. Resolve the target's HWND via `WidgetRef::native_handle()`.
2. Find the top-level via `GetAncestor(hwnd, GA_ROOT)`.
3. Look up an existing tooltip HWND in the global cache; if none,
   `CreateWindowExW(... "tooltips_class32" ...)` and insert into the cache.
4. Fill a `TOOLINFOW { cbSize, uFlags = TTF_IDISHWND | TTF_SUBCLASS,
   uId = hwnd as usize, lpszText = LPSTR_TEXTCALLBACK }` and send
   `TTM_ADDTOOL` so the tooltip's parent sub-classes our target for
   mouse tracking.

## Re-entrancy and Drop

`ToolTip` does not currently register a per-message WndProc handler — the
tooltip control itself is a system common control and handles mouse
tracking internally via the `TTF_SUBCLASS` flag. `Drop` does not
explicitly remove the tool entry; the control cleans up when its parent
top-level is destroyed.

## Cross-references

- [`widget.md`](widget.md) — the `Widget` trait's `native_handle()` and
  the `WidgetRef` newtype used as the `attach` argument.
- [`frame.md`](../window/frame.md) — the top-level window whose HWND the cache is
  keyed by (`GetAncestor(..., GA_ROOT)`).
