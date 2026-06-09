# `tests/integration.rs` — cross-module integration tests

Top-level test file (lives next to `Cargo.toml`). Exercises the **public**
API only — anything `pub(crate)` is invisible from here, so this file is the
safety net that catches accidental leakage of internal items into rustdoc
output.

## Coverage

Each test comments pin a specific contract. The file is organised in
clearly-marked sections:

### Public re-exports

- `glob_import_brings_in_the_public_api` — `use ru_wx::*` must compile
  with `Accelerator`, `Modifiers`, `VirtualKey`, `ParseError`, `Dpi`,
  `SYSTEM_DPI`, `Rect`, `Colour`, `BoxSizer`, `Orientation`. If any
  re-export is removed, this fails to compile.
- `prelude_brings_in_the_everyday_api` — `use ru_wx::prelude::*` must
  include `App`, `Frame`, `Button`, `StaticText`, `Colour`. Pinned
  via `fn` pointer coercion and type references.

### Cross-module: Accelerator

- `accelerator_via_modifiers_and_virtualkey_matches_parse` — struct
  literal vs string parse must produce equal `Accelerator`.
- `accelerator_parse_display_round_trip` — for sample bindings
  (`Ctrl+S`, `F5`, `Alt+F4`, `Ctrl+Alt+Shift+Z`, `Escape`, `Ctrl+1`),
  `parse` ∘ `to_string` ∘ `parse` must equal the original.

### Cross-module: Dpi

- `dpi_scale_unscale_round_trip` — for DPIs `[96, 120, 144, 168, 192,
  240, 288, 384]` and logical values `[0, 50, 100, 250, 800, 1234]`,
  `dpi.unscale(dpi.scale(x)) == x`.
- `dpi_display_includes_value_and_percent` — pins format
  `"Dpi(96 / 100%)"`, `"Dpi(120 / 125%)"`, etc.

### Cross-module: Sizer (v0.5.0)

- `box_sizer_getters_reflect_constructor` — `padding()` and
  `orientation()` getters return the constructor's values; default
  padding is 5.

### Cross-module: Geometry

- `rect_contains_and_colorref_agree` — `Rect::contains` corner
  inclusion semantics; `Colour::to_colorref()` byte layout
  `0x00BB_GG_RR`:
  - pure red  `(0xFF,0,0,0)` → `0x0000_00FF`
  - pure green `(0,0xFF,0,0)` → `0x0000_FF00`
  - pure blue  `(0,0,0xFF,0)` → `0x00FF_0000`

### Cross-module: Modifiers (v0.5.0)

- `modifiers_constants_match_the_win32_fvirt_bits` — pins the bit
  layout that maps to the `fVirt` byte of Win32 `ACCEL`:
  - `Modifiers::CTRL.0  == 0x08` (FCONTROL)
  - `Modifiers::ALT.0   == 0x10` (FALT)
  - `Modifiers::SHIFT.0 == 0x04` (FSHIFT)
  - `Modifiers::NONE.0  == 0x00`
  - All three are pairwise disjoint.

### Cross-module: v0.5.1 Frame runtime-rebinding API

- `accelerator_rebinding_methods_have_expected_signatures` — pins
  function pointers for `Frame::unregister_accelerator`,
  `Frame::clear_accelerators`, `Frame::replace_accelerator`.
- `accelerator_rebinding_methods_are_reachable_through_the_prelude`
  — same signatures through `ru_wx::prelude::*`.

### Cross-module: v0.5.2 ListCtrl selection API

- `listctrl_selection_methods_have_expected_signatures` — pins:
  - `ListCtrl::select(&ListCtrl, usize)`
  - `ListCtrl::deselect(&ListCtrl, usize)`
  - `ListCtrl::clear_selection(&ListCtrl)`
  - `ListCtrl::is_selected(&ListCtrl, usize) -> bool`
  - `ListCtrl::get_selected_item_count(&ListCtrl) -> usize`
  - `ListCtrl::get_selected_items(&ListCtrl) -> Vec<usize>`
  - `ListCtrl::set_item_state(&ListCtrl, usize, u32, u32)`
  - `ListCtrl::get_item_state(&ListCtrl, usize) -> u32`
- `listctrl_selection_methods_are_reachable_through_the_prelude` —
  same via the prelude.
- `ListCtrlStyle` variants pinned: `Report`, `List`, `Icon`,
  `SmallIcon`.

### Cross-module: v0.5.3 FileDialog multi-select API

- `file_dialog_multi_select_methods_have_expected_signatures` — pins:
  - `FileDialog::set_multi_select(&mut FileDialog, bool) -> &mut FileDialog`
  - `FileDialog::is_multi_select(&FileDialog) -> bool`
  - `FileDialog::show_modal_multi(&mut FileDialog) -> Vec<String>`
  - `FileDialogStyle::Open` and `::Save` are distinct variants.
- `file_dialog_multi_select_is_reachable_through_the_prelude` —
  same via the prelude.

## Why no windowed tests

Real `Frame`, `Button`, painting, `WM_COMMAND` dispatch all require a
live `HWND`. Those are exercised by:
- The `Frame::for_testing` unit tests inside `src/frame.rs` (which is
  `pub(crate)` and therefore invisible from this file).
- The `examples/showcase_all.rs` binary that lives in the main
  `examples/` directory.

## Running

```
cargo test --test integration
```

Or as part of the full suite:

```
cargo test
```
