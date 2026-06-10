# `accelerator.rs` — `Accelerator`, `VirtualKey`, `Modifiers`

A focused module for declaring keyboard shortcuts ("accelerators" in
Win32 parlance) in a portable-ish shape. Includes a `"Ctrl+Shift+S"`
string parser, a `Display`-based canonical renderer, and a Win32
`ACCEL` conversion.

## Why a dedicated module

The accelerator is the only piece of UI that bridges two namespaces
that the rest of the crate doesn't share: the *user-facing key chord*
("Ctrl+Shift+P") and the *Win32 `ACCEL` table entry* passed to
`CreateAcceleratorTableW`. Centralising the parse / display / FFI
mapping in one module means the rest of the crate (`frame`,
`menu`, `popup_menu`, `tab`) only needs to call
`Accelerator::parse("Ctrl+S")` and `to_accel(command_id)`.

## Public types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(pub u8);   // CTRL = 0x08, ALT = 0x10, SHIFT = 0x04

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualKey {
    Char(char),                 // ASCII letter or digit, upper-cased
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Escape, Tab, Enter, Space, Backspace, Delete, Insert,
    Home, End, PageUp, PageDown,
    Left, Right, Up, Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Accelerator {
    pub key: VirtualKey,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    MissingKey,
    InvalidToken(String),
    DuplicateModifier(&'static str),
    InvalidChar,
}
```

## `Modifiers`

- `Modifiers::NONE = 0x00`, `CTRL = 0x08`, `ALT = 0x10`, `SHIFT = 0x04`.
- These bits match the Win32 `ACCEL.fVirt` constants
  (`FCONTROL=0x08`, `FALT=0x10`, `FSHIFT=0x04`) so a `Modifiers` value
  can be OR'd straight into an `ACCEL` entry.
- `from_bools(ctrl, alt, shift)` constructs a value from three booleans.
- `ctrl()`, `alt()`, `shift()`, `is_none()` are `const fn` bit inspectors.
- Implements `BitOr`, `BitOrAssign`, `BitAnd`, `Default`, and `Display`
  (renders in canonical `Ctrl+Alt+Shift+` order, no trailing `+` issue
  because the canonical trailing `+` is part of the modifier's render).

## `VirtualKey`

The enum deliberately covers only the keys useful in a GUI hotkey:
the alphanumeric range, the function-key row, and the navigation
cluster. Its `Display` impl matches the parser's accepted token
strings (`F5`, `Escape`, `PgUp`, ...).

## `Accelerator` — public API

| Method | Purpose |
|---|---|
| `new(key)` | Bare key, no modifiers. |
| `with_modifiers(key, modifiers)` | Explicit modifier set. |
| `parse(s) -> Result<Self, ParseError>` | Parse `"Ctrl+S"`, `"Alt+Shift+F4"`, `"F5"`, etc. |
| `display() -> String` | Render canonical form (delegates to `Display`). |

### String format

```text
Ctrl+S            # S with the Ctrl modifier
Ctrl+Shift+P      # P with Ctrl and Shift
F5                # function key F5, no modifier
Alt+F4            # F4 with the Alt modifier
Escape            # bare key, no modifier
```

- Modifiers: `Ctrl` / `Control` / `Ctl`, `Alt`, `Shift` (any case).
- Named-key aliases: `Esc` / `Escape`, `Return` / `Enter`, `Del` / `Delete`,
  `Ins` / `Insert`, `PgUp` / `PageUp` / `Prior`, `PgDn` / `PageDown` /
  `PgDwn` / `Next`, `Backspace` / `BS`, `Space` / `Spacebar`.
- Single ASCII letter or digit only (`a`..`z`, `A`..`Z`, `0`..`9`).
- Whitespace around tokens is permitted and ignored.
- Parser errors:
  - `Empty` — input is empty or whitespace only.
  - `MissingKey` — modifiers present but no key, or two non-modifier
    tokens.
  - `InvalidToken(s)` — token is not a known modifier and not a known
    key (e.g. `"Ctrl+Bar"`).
  - `DuplicateModifier(name)` — a modifier appears twice.
  - `InvalidChar` — non-ASCII letter/digit key.

## Win32 FFI (Windows only)

`Accelerator::to_accel(command) -> windows_sys::...::ACCEL` produces a
Win32 `ACCEL` entry:

```text
fVirt = FVIRTKEY | FNOINVERT | modifiers.0
key   = virtual_key_to_win32(key)   // VK_S, VK_F5, ...
cmd   = command                      // menu / window command id
```

- `FVIRTKEY = 0x01` — the `key` is a virtual-key code, not an ASCII char.
- `FNOINVERT = 0x02` — prevent the menu item being visually inverted
  when the accelerator fires (the modern "fire-and-forget" behaviour).
  `FNOINVERT` is a well-known `winuser.h` constant that
  `windows-sys 0.59` does **not** export; defined locally to keep the
  FFI surface self-contained.
- `virtual_key_to_win32` maps every `VirtualKey` variant to the
  corresponding `VK_*` constant.

## Tests

22 unit tests, organised into four groups:

- **Modifiers** (4) — bit constants are disjoint, `from_bools` round-trips
  over all 8 combinations, `|` accumulates, `Display` order is canonical.
- **VirtualKey** (1) — `Display` round-trips for keys the parser knows.
- **parse** (10) — plain letter, lower/upper case, `Ctrl+letter`,
  case-insensitive modifier, three modifiers, function key with/without
  modifier, named-key aliases, whitespace tolerance, digit key.
- **parse errors** (5) — empty input, modifier-only input, unknown token,
  duplicate modifier, two keys.
- **display round-trip** (3) — `parse→display→parse` is the identity for
  the canonical form (no modifier, simple, three modifiers).
- **Win32 FFI** (2) — `to_accel` produces `FVIRTKEY | FNOINVERT |
  FCONTROL` and the right `VK_*` for `Ctrl+S` and `F5`.

## Cross-references

- [`frame.md`](../window/frame.md) — `Frame::register_accelerator(acc, command_id)`
  adds a binding to the per-frame `HACCEL` table, translated with
  `TranslateAcceleratorW` in the message loop and dispatched as
  `WM_COMMAND` (reusing the menu callback table).
- [`menu.md`](../window/menu.md) — `Menu::append_with_shortcut` stores the
  `Accelerator` in `MenuItem.shortcut` and renders it as a
  right-aligned `\t` hint.
- [`prelude.md`](../prelude.md) — re-exports `Accelerator`, `VirtualKey`,
  `Modifiers` at the crate root.

## Quick start

A complete, copy-pasteable example covering the three common uses:
parsing a string, attaching a hotkey to a menu item, and registering a
frame-level accelerator that fires a command even when its menu isn't open.

```rust,no_run
use ru_wx::prelude::*;

fn wire_shortcuts(frame: &Frame) {
    // 1. Build a menu item with a parsed accelerator.
    let mut file = Menu::new("&File");
    let save_id = file.append_with_shortcut(
        "&Save",
        Accelerator::parse("Ctrl+S").unwrap(),   // -> VirtualKey::Char('S') + Modifiers::CTRL
        || println!("save fired"),
    );

    // 2. Construct an accelerator directly (no string parse).
    let open_shortcut = Accelerator::new(VirtualKey::Char('O'))
        .with_modifiers(Modifiers::CTRL);
    let open_id = file.append_with_shortcut("&Open...", open_shortcut, || {
        println!("open fired");
    });

    // 3. Register the same accelerator on the frame so it fires
    //    even when the menu is closed (TranslateAcceleratorW path).
    frame.register_accelerator(open_shortcut, open_id);

    // 4. Render the canonical form (useful for status bars / tooltips).
    println!("save: {}", save_id);                 // Display impl
    let display = Accelerator::parse("Alt+Shift+F4").unwrap().display();
    assert_eq!(display, "Alt+Shift+F4");

    // 5. Build a function-key accelerator with three modifiers.
    let quit = Accelerator::with_modifiers(
        VirtualKey::F4,
        Modifiers::from_bools(/*ctrl*/ true, /*alt*/ true, /*shift*/ true),
    );
    frame.register_accelerator(quit, /* quit id */ 9999);
}
```

**Typical workflow**

1. Pick the key chord. For a "user-typed" hotkey (settings file, command
   palette), parse it with `Accelerator::parse("Ctrl+Shift+P")`. For an
   internal builder, construct it directly with
   `Accelerator::with_modifiers(key, modifiers)`.
2. Attach the chord to a `Menu` item via
   `Menu::append_with_shortcut(label, accelerator, callback)`. The
   `Menu` stores the chord and renders it as a right-aligned hint.
3. Optionally call `frame.register_accelerator(accelerator, command_id)`
   to make the same hotkey work **even when the menu is closed** — the
   frame's WndProc does `TranslateAcceleratorW` and dispatches the
   command as `WM_COMMAND`, reusing the menu callback table.
4. For the Windows FFI path, `Accelerator::to_accel(command_id)` produces
   a `windows_sys::Win32::UI::WindowsAndMessaging::ACCEL` ready for
   `CreateAcceleratorTableW`.

**Notes**

- The `Modifiers` bit pattern (`CTRL=0x08`, `ALT=0x10`, `SHIFT=0x04`)
  matches Win32 `FCONTROL` / `FALT` / `FSHIFT`, so a `Modifiers` value
  drops straight into an `ACCEL` `fVirt` field.
- `Accelerator::parse` is strict: it rejects empty input, modifier-only
  input, duplicate modifiers, unknown tokens, and non-ASCII letter/digit
  keys. All errors are typed as `ParseError` variants.
- The parser is case-insensitive for modifier names and aliases
  (`Ctrl` / `Control` / `Ctl`, `Esc` / `Escape`, `PgUp` / `PageUp` /
  `Prior`, ...). The key portion must be a single ASCII letter or digit,
  upper- or lower-case.
- `Display` and `parse` round-trip in canonical form — a parsed
  accelerator's `to_string()` is always re-parseable.
- For *keys outside* the `VirtualKey` enum (e.g. OEM-specific keys),
  this module is not the right tool — build the `ACCEL` entry by hand.

## Example

```rust
use ru_wx::prelude::*;
use ru_wx::accelerator::Accelerator;

let mut file = Menu::new("&File");
let open_id = file.append_with_shortcut(
    "&Open...",
    Accelerator::parse("Ctrl+O").unwrap(),
    &frame,
    || println!("open!"),
);
frame.register_accelerator(
    Accelerator::parse("Ctrl+O").unwrap(),
    open_id,
);
```
