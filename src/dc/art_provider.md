# `src/art_provider.rs` — Built-in SVG icon library + overrides

## Purpose
A static catalogue of 34 standard icon IDs (New, Open, Save, Undo, …) backed
by 24×24-viewBox SVGs, plus an `ArtProvider` type that lets the application
register custom SVGs and resolve any `ArtId` to a `BitmapBundle` at the
client's DPI. Mirrors wxWidgets' `wxArtProvider`.

## Key types

### `ArtId` (34 variants)
Application / file / status / navigation stock icons:
`New, Open, Save, SaveAs, Print, Cut, Copy, Paste, Undo, Redo, Find,
Replace, Delete, Add, Remove, Ok, Cancel, Apply, Close, Quit, About, Help,
Information, Warning, Error, Question, Folder, File, Home, Settings, Refresh,
Search, Star`. Each variant has a hand-written SVG body inside the `svg!`
macro.

### `ArtClient` (4 variants)
Surface type → default logical icon size in pixels:
- `Menu` → 16
- `ToolBar` → 24
- `Button` → 32
- `Dialog` → 48

```rust
impl ArtClient {
    pub fn default_size(self) -> u32;        // returns 16 / 24 / 32 / 48
}
```

### `ArtProvider`
- `overrides: HashMap<ArtId, Vec<u8>>` — user-registered SVGs that take
  precedence over the built-in catalogue.
- `register_svg(id, svg_bytes)` — install a custom SVG for `id`.
- `unregister(id)` — drop a custom SVG (built-in will be used again).
- `get_bitmap(id) -> Option<BitmapBundle>` — render at the
  client-determined sizes (queries the bound `ArtClient`, default `Menu`).
- `get_bitmap_with_size(id, size) -> Option<BitmapBundle>` — render at
  `[size, size + size/2, size*2]` (the three canonical HiDPI steps).

## Macros / helpers
- `svg! { xml }` — macro that wraps an inner XML body in
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">{body}</svg>`.
  Used once per `ArtId` to inline the icon's geometry.
- `svg_for(id: ArtId) -> &'static [u8]` — returns the built-in SVG bytes
  for `id`, or the override if one is registered.

## Public API
```rust
pub enum ArtId { New, Open, Save, SaveAs, Print, /* ... 30 more ... */ Star }
pub enum ArtClient { Menu, ToolBar, Button, Dialog }
impl ArtClient { pub fn default_size(self) -> u32; }

pub struct ArtProvider { /* private: overrides: HashMap<...> */ }
impl ArtProvider {
    pub fn new() -> Self;
    pub fn register_svg(&mut self, id: ArtId, svg: Vec<u8>);
    pub fn unregister(&mut self, id: ArtId);
    pub fn get_bitmap(&self, id: ArtId) -> Option<BitmapBundle>;
    pub fn get_bitmap_with_size(&self, id: ArtId, size: u32) -> Option<BitmapBundle>;
}
```

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. The crate ships a single global ArtProvider; obtain a reference via
//    `default_art_provider()` (re-exported from the prelude).
let prov = default_art_provider();

// 2. Look up any of the 34 built-in stock icons as a multi-resolution bundle.
if let Some(bundle) = prov.get_bitmap(ArtId::New) {
    // Bundle holds (16, 24, 32) for Menu client, (24, 36, 48) for ToolBar, etc.
    let icon = bundle.best_for_dpi(frame.dpi().value()).unwrap();
    // ... hand `icon.hbitmap` to a ToolBar, ImageList, frame icon, ...
}

// 3. Override one of the built-in icons with a custom SVG.
let mut prov = ArtProvider::new();
prov.register_svg(ArtId::New, br#"<rect width="24" height="24" fill="lime"/>"#.to_vec());
if let Some(bundle) = prov.get_bitmap_with_size(ArtId::New, 16) {
    // Bundle holds (16, 24, 32) — the 1.5× HiDPI step is 16 + 16/2 = 24.
    let h = bundle.best_for_dpi(96).unwrap();
}

// 4. Or add a brand-new icon (not in the built-in list):
prov.register_svg(ArtId::Star, include_bytes!("../assets/icons/star.svg").to_vec());
if let Some(bundle) = prov.get_bitmap(ArtId::Star) {
    // use it ...
}

// 5. Drop the override — the built-in SVG (or `None` if it was a brand-new
//    id) is used again:
prov.unregister(ArtId::New);

// 6. The 4 ArtClient surfaces pick the default logical size:
assert_eq!(ArtClient::Menu.default_size(),     16);
assert_eq!(ArtClient::ToolBar.default_size(), 24);
assert_eq!(ArtClient::Button.default_size(),  32);
assert_eq!(ArtClient::Dialog.default_size(),  48);
```

Each `ArtId` ships with a hand-written 24×24-viewBox SVG inlined via the `svg!` macro. `register_svg` lets you replace any of them or add a brand-new id (the built-in catalogue is just a starting point).

## Win32 / platform notes
- `get_bitmap` is the entry point used by `frame.rs` / `tool_bar.rs` /
  `aui_tool_bar.rs` when populating a control with stock icons.
- Resolution selection is handled by `BitmapBundle::best_for_dpi`, not
  here; this module only **builds** the bundle.
- HiDPI step sizes: `get_bitmap_with_size(id, 16)` returns
  `BitmapBundle` of `(16, 24, 32)` — the 1.5× step is `size + size/2`
  (integer rounding: `24` from `16 + 8`, not `25`).

## Tests (3)
- `default_size_for_menu_is_16` — `ArtClient::Menu.default_size() == 16`.
- `default_size_for_toolbar_is_24` — `ArtClient::ToolBar.default_size() == 24`.
- `art_id_svg_is_non_empty` — every variant returns at least one byte from
  `svg_for`.

## Cross-references
- `icon.rs` — `svg_bytes_to_hbitmap` does the actual rasterization.
- `bitmap_bundle.rs` — multi-resolution output container.
- `frame.rs` / `tool_bar.rs` / `aui_tool_bar.rs` — consumers of
  `ArtProvider::get_bitmap`.
- `lib.rs` / `prelude.rs` — the global `default_art_provider()` returns
  the single crate-wide instance.

## Example
```rust,no_run
use ru_wx::prelude::*;

let mut prov = ArtProvider::new();
prov.register_svg(ArtId::New, br#"<rect width="24" height="24" fill="lime"/>"#.to_vec());
if let Some(bundle) = prov.get_bitmap_with_size(ArtId::New, 16) {
    let h = bundle.best_for_dpi(96).unwrap(); // 16x16 on a 100% monitor
}
```
