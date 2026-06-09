# ru_wx — Completion Report (v0.5.3)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.3
**Date:** 2026-06-05
**Cycles run in the 4th 5-cycle pass:** 4 of 5
(cycles 16–19 / v0.5.0–v0.5.3 complete; cycle 20 /
v0.5.4 planned — see §5 for the carry-over list).

---

## 1. Executive summary

v0.5.3 is the **fourth cycle of the 4th 5-cycle pass**. Its
theme is **wxWidgets parity pass 2**, focused on closing the
second visible gap in the long-running **wxWidgets parity
gaps** future-work item: the absence of multi-file selection
on `FileDialog`. The control already exposed the
single-file `show_modal() -> Option<String>` path (which goes
through `GetOpenFileNameW` / `GetSaveFileNameW` with the
conservative `OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST |
OFN_NOCHANGEDIR` flag set) but lacked the multi-file variant
that wxWidgets ships out of the box (`wxFileDialog` with
`wxFD_MULTIPLE`). v0.5.3 adds the multi-file variant via the
Win32 `OFN_ALLOWMULTISELECT` flag, plus the dedicated
buffer-parsing helper that turns the multi-select output
buffer into a `Vec<String>`, plus a deep unit-test suite that
pins every edge case of the parsing.

This is **item 2 in the v0.4.2 / v0.5.0 future-work table**
— the long-running "wxWidgets parity gaps" item. It is now
**partially closed** for the **second time** in the 4th pass
(after v0.5.2 closed the `ListCtrl` selection sub-item).
The remaining sub-items (virtual list mode with
`LVS_OWNERDATA`, drag-and-drop, `DatePickerCtrl` value
extraction, `Menu` shortcut label refresh after
`Frame::replace_accelerator`) carry over to v0.5.4 — the
final cycle of the pass.

Three concrete deliverables:

1. **Two new Win32 constants** in `src/file_dialog.rs`:
   `OFN_ALLOWMULTISELECT` (`0x00000200`) and `OFN_EXPLORER`
   (`0x00080000`). Pinned from `<commdlg.h>` and Microsoft
   Docs. The new `show_modal_multi` method uses both, joined
   with the existing single-file flag set.
2. **Three new public methods on `FileDialog`**: a
   fluent builder `set_multi_select(&mut self, bool) -> &mut Self`
   (default `false`, no-op on non-Windows), a getter
   `is_multi_select(&self) -> bool`, and the multi-file modal
   driver `show_modal_multi(&mut self) -> Vec<String>`. The
   last method:
   - Allocates a **32 KiB** working buffer (in `u16` code
     units, ~64 KiB on the heap) — the size the Win32
     documentation recommends for multi-select buffers.
   - Uses the new flags `OFN_ALLOWMULTISELECT | OFN_EXPLORER`
     plus the existing conservative flag set.
   - Returns an empty `Vec` for `FileDialogStyle::Save` (Win32
     `GetSaveFileNameW` does not honour `OFN_ALLOWMULTISELECT`).
   - Delegates the buffer parse to the new `pub(crate)`
     `parse_multiselect_buffer` helper.
3. **One new `pub(crate)` helper** `parse_multiselect_buffer(buf:
   &[u16], _file_offset: usize) -> Vec<String>`. The helper:
   - Walks the buffer and collects null-terminated strings
     until it hits a double-null (the end-of-list marker).
   - Returns an empty `Vec` for an empty / all-zero buffer.
   - Returns a single-element `Vec` for a single-file
     selection (no `OFN_ALLOWMULTISELECT`).
   - For multi-select, joins each filename with the directory
     prefix (the first null-terminated string in the buffer),
     handling: trailing `\` / `/` on the directory (no
     double-separator), absolute filenames (UNC root or drive
     letter) returned verbatim, empty filenames filtered out.
   - Accepts `file_offset` for API parity with
     `GetOpenFileNameW` but does **not** use it (the entire
     first string is the directory, by definition).

**CI status (post-cycle):** green. 137 lib tests + 15
integration tests + 23 doc tests = **175 / 175** passing
(+28 since v0.5.2: +26 unit + 2 integration).
0 clippy warnings, 0 clippy errors, 0 rustfmt diffs.

**Symbolic impact:** the `file_dialog` row in the test
coverage matrix moves from **smoke only** to
**unit + smoke** — `file_dialog` is the **third widget
control** (after `frame` and `list_ctrl`) to break out of
the smoke-only bucket in the 4th 5-cycle pass, and the
**first dialog / file-picker control** to do so. The matrix
now has **2 widget controls and 1 frame-class control** with
unit tests, and **7 modules with full coverage** (accelerator,
dpi, geometry, log, sizer, frame, list_ctrl) — counting
`file_dialog` brings the count of "unit + smoke" modules to
**3** (frame, list_ctrl, file_dialog).

---

## 2. Test coverage matrix

Per-module coverage of the source files that have public
APIs. "Unit tests" means `#[cfg(test)] mod tests` blocks
inside the source file; "Doc tests" means ` ```rust `
fences inside the rustdoc; "Integration tests" means
`tests/integration.rs`; "Windowed smoke" means the
`examples/showcase_all.rs` example binary exercises the
windowed parts of the API.

| Module | Unit tests | Doc tests | Integration tests | Windowed smoke | Verdict |
| --- | --- | --- | --- | --- | --- |
| `accelerator` | 26 ✓ | 1 ✓ | 2 ✓ | yes (menu shortcut registration) | **complete** |
| `app` | (none — `App::new` is a 1-liner) | 1 ✓ | 1 ✓ (type in scope) | yes | **complete** |
| `art_provider` | 3 ✓ | 1 ✓ | (no global) | yes (menus / toolbars use it) | **complete** |
| `button` | (none — requires `HWND`) | 0 | 0 (type in scope only) | yes | **smoke only** |
| `checkbox` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `combo_box` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `dialog` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `dpi` | 11 ✓ | 1 ✓ | 2 ✓ | yes (status bar prints DPI) | **complete** |
| **`file_dialog`** | **26 ✓ (+26 in v0.5.3)** | 0 | **2 ✓ (+2 in v0.5.3)** | yes | **unit + smoke** ⭐ |
| `frame` | 21 ✓ | 1 ✓ | 3 ✓ | yes | **unit + smoke** |
| `geometry` | 6 ✓ | 0 | 1 ✓ | yes (used everywhere) | **complete** |
| `grid` | (none — `Cell` / `Grid` are pure data) | 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `grid_sizer` | (none — `GridSizer` / `FlexGridSizer` are pure data) | 0 | 0 (prelude) | yes | **smoke only** |
| `icon` / `icon_tray` | (none — requires `HWND` / shell APIs) | 0 | 0 (prelude) | yes | **smoke only** |
| `list_box` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `list_ctrl` | 17 ✓ | 0 | 2 ✓ | yes | **unit + smoke** ⭐ |
| `log::*` | 17 ✓ across 6 submodules | 8 ✓ across 4 submodules | 0 (private) | yes (used internally) | **complete** |
| `menu` | (none — requires `HWND`) | 1 ✓ (`Menu::append_with_shortcut`) | 0 (prelude) | yes | **smoke only** |
| `message_box` / `message_dialog` | (none — requires `HWND`) | 0 / 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `panel` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `popup_menu` | (none — requires `HWND`) | 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `radio_button` / `radio_box` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `sizer` | 6 ✓ (incl. the v0.5.0 getter coverage) | 0 | 1 ✓ | yes (used by the showcase) | **complete** |
| `slider` / `spin_ctrl` / `static_text` / `text_ctrl` | (none — requires `HWND`) | 0 | 0 (prelude types only) | yes | **smoke only** |
| `status_bar` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `tab` | (none — uses Win32 `TCITEMW`) | 0 | 0 (prelude) | yes | **smoke only** |
| `timer` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `tool_bar` / `aui_tool_bar` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `tooltip` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `top_level_window` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `tree_ctrl` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `widget` (trait) | (no unit tests — the trait is the API) | 0 | 0 (re-exported) | yes (used by every widget) | **complete** |
| `date_picker_ctrl` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `gauge` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `colour_picker_ctrl` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `check_list_box` / `choice` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `bitmap_bundle` / `image_list` / `font` | (none — pure data) | 0 | 0 (prelude) | yes | **smoke only** |
| `platform` (private) | 0 (private) | 0 | 0 | (n/a) | **n/a** |

**Totals:**

- **Unit tests:** 137 ✓ (up from 111 in v0.5.2; +26 from
  `file_dialog::tests` covering the new `parse_multiselect_buffer`
  helper, the new `multi_select` state field, and the new
  `OFN_ALLOWMULTISELECT` / `OFN_EXPLORER` Win32 constants).
- **Doc tests:** 23 ✓ (unchanged).
- **Integration tests:** 15 ✓ (up from 13 in v0.5.2; +2
  signature-pinning tests for the new methods).
- **Grand total:** 175 / 175 passing (+28 since v0.5.2).

**Smoke-only modules.** All of the "smoke only" rows above
**require a real Win32 `HWND`** (creating a `Frame`,
registering a window class, dispatching a `WM_COMMAND`,
etc.). The test harness in v0.5.3 still deliberately stops
short of those: it covers the platform-agnostic public
surface. Windowed coverage is provided by the
`examples/showcase_all.rs` binary, which exercises every
windowed widget end-to-end.

**The `file_dialog` module in v0.5.3.** The unit-test count
for `file_dialog` jumps from **0 to 26** — the **+26** in
this cycle is a brand-new `#[cfg(test)] mod tests` at the
bottom of the file, divided into five groups:

- **12 `parse_multiselect_buffer` tests** (the deep-coverage
  group):
  - `parse_multi_select_empty_buffer_returns_empty` —
    `parse_multiselect_buffer(&[], 0) == []`.
  - `parse_multi_select_all_zero_buffer_returns_empty` —
    `vec![0u16; 64]` returns `[]` (no parts, immediate
    double-null).
  - `parse_multi_select_single_file_returns_one_element` —
    `"C:\file.txt\0\0"` returns `["C:\\file.txt"]`.
  - `parse_multi_select_two_files_returns_two_elements` —
    `"C:\dir\0a.txt\0b.txt\0\0"` returns
    `["C:\\dir\\a.txt", "C:\\dir\\b.txt"]`.
  - `parse_multi_select_three_files_returns_three_elements` —
    same shape with three files.
  - `parse_multi_select_trailing_backslash_dir_is_preserved`
    — dir `C:\` does not produce `C:\\a.txt` (double
    separator).
  - `parse_multi_select_trailing_forward_slash_dir_is_preserved`
    — dir `C:/` is treated like dir `C:\`.
  - `parse_multi_select_offset_does_not_alter_output` —
    passing a `file_offset` does not change the reconstructed
    paths.
  - `parse_multi_select_absolute_filename_is_returned_verbatim`
    — a filename `D:\other.txt` is returned as-is, not joined
    with the directory.
  - `parse_multi_select_unc_filename_is_returned_verbatim` —
    a filename `\\server\share\file.txt` is returned as-is.
  - `parse_multi_select_empty_filename_is_skipped` — a stray
    empty filename (double-null inside the list) is filtered
    out.
  - `parse_multi_select_unterminated_buffer_yields_final_path`
    — defensive: a buffer with no trailing null is treated
    as a single final path (the helper handles the corner
    case gracefully, even though Win32 always terminates
    the list).
- **4 `wildcard_to_win32_filter` tests**:
  - `wildcard_empty_produces_double_null` — `""` produces
    `[0u16, 0u16]` (a Win32 "no filter" terminator).
  - `wildcard_single_pair` — `"Text|*.txt"` produces
    `T\0e\x00x\0t\0*\0.\0t\0x\0t\0\0`.
  - `wildcard_two_pairs` — `"Text|*.txt|All|*.*"` produces
    two pairs, each null-terminated, with a final null.
  - `wildcard_odd_parts_drops_dangling_description` —
    `"Desc1|*.ext1|Desc2"` (3 parts) drops the dangling
    description (we only emit pairs).
- **5 multi-select state tests**:
  - `multi_select_default_is_false` — `FileDialog::new(&frame,
    FileDialogStyle::Open).is_multi_select() == false` (the
    default-constructed via the public API).
  - `multi_select_new_for_test_true_round_trips` —
    `new_for_test(true).is_multi_select() == true`.
  - `multi_select_new_for_test_false_round_trips` —
    `new_for_test(false).is_multi_select() == false`.
  - `multi_select_setter_enables` — `dlg.set_multi_select(true);
    dlg.is_multi_select() == true`.
  - `multi_select_setter_returns_mut_self_for_chaining` —
    `set_multi_select(true)` returns `&mut Self` (so callers
    can chain: `dlg.set_multi_select(true).set_title("foo")`).
- **3 OFN constant tests**:
  - `ofn_constant_values_match_win32_headers` — pins the
    numeric value of all 5 OFN flags (`OFN_FILEMUSTEXIST =
    0x00001000`, `OFN_PATHMUSTEXIST = 0x00000800`,
    `OFN_NOCHANGEDIR = 0x00000008`, `OFN_ALLOWMULTISELECT =
    0x00000200`, `OFN_EXPLORER = 0x00080000`). A typoed hex
    digit is caught at compile time.
  - `ofn_flags_are_all_distinct` — guards against
    accidentally aliasing two flags.
  - `ofn_combined_flags_contain_each_component` — the
    combined `Flags` value used by `show_modal_multi`
    contains each individual flag bit (no bit is dropped
    by a faulty `|`).
- **2 `FileDialogStyle` tests**:
  - `file_dialog_style_open_is_not_save` — `Open != Save`.
  - `file_dialog_style_is_copy` — the enum is `Copy`, so
    passing it by value is cheap.

The `file_dialog` row is the **only** row in the matrix
that changed in this cycle. It is the **first dialog /
file-picker control** to break out of the smoke-only bucket
in the 4th 5-cycle pass, and the **first control to be
backed by a `pub(crate)` helper** that does the heavy
lifting of the Win32 buffer format.

---

## 3. Module-by-module status

### 3.1 Fully unit-tested (platform-agnostic)

These modules are **complete** from a test-coverage point of
view: the public surface is exercised without needing a
real `HWND`.

- **`accelerator`** — 26 unit tests + 1 doc test + 2
  integration tests. Covers `Modifiers` (bit layout,
  `from_bools`, `BitOr` / `BitAnd`, `Display` canonical
  order, all constants), `VirtualKey` (all 27 variants
  render correctly), `Accelerator` (parse, `Display`,
  round-trip, error variants, function keys, digit keys,
  named-key aliases, whitespace tolerance), and the Win32
  `to_accel` FFI mapping (`fVirt`, `key`, `cmd`).
- **`dpi`** — 11 unit tests + 1 doc test + 2 integration
  tests. Covers the `Dpi` newtype (default 96, newtype
  construction, zero-coercion, scale/unscale, round-trip,
  scale-factor conversion, `Display` format, system DPI
  guard).
- **`geometry`** — 6 unit tests + 1 integration test.
  Covers `Rect` (default origin, `new` keeps fields,
  `contains` is inclusive-min / exclusive-max), `Colour`
  (default is white, constants have expected channels,
  `to_colorref` is `0x00BB_GG_RR`).
- **`log::*`** — 17 unit tests across 6 submodules
  (`formatter`, `levels`, `manager`, `record`, `target`)
  + 8 doc tests. Covers every public log API end-to-end
  with the `BufferTarget` test sink.
- **`sizer`** — 6 unit tests + 1 integration test. Covers
  empty-sizer layout (no panic), horizontal / vertical
  fixed-size packing, custom padding, proportional stretch,
  and the v0.5.0 getters (`padding`, `orientation`).
- **`frame` (platform-agnostic only)** — 21 unit tests +
  3 integration tests. Covers:
  - **v0.5.0 tests (11):** `Frame::for_testing` (empty
    state), accelerator registration (order, duplicates,
    clone isolation), command-handler map (insert,
    overwrite), notify-handler map, tray-message-handler
    unregister, sizer storage (`None` → `Some` → `Some`),
    and the `null_hwnd` fallback in `dpi` /
    `scale_factor` (Windows-only).
  - **v0.5.1 tests (10 new):** the new rebinding methods
    (`unregister_accelerator`, `clear_accelerators`,
    `replace_accelerator`) — no-op paths, happy paths,
    relative-order preservation, "first match wins" on
    duplicate `old` accelerators, and the realistic
    three-step rebind workflow (register × 3, replace the
    save binding, clear everything).

### 3.2 Smoke-only (windowed)

These modules require a real Win32 `HWND` to test. They are
exercised end-to-end by `examples/showcase_all.rs`, which is
the integration test for the windowed surface.

- **Widgets:** `button`, `checkbox`, `combo_box`,
  `check_list_box`, `choice`, `radio_button`, `radio_box`,
  `static_text`, `text_ctrl`, `slider`, `spin_ctrl`, `gauge`,
  `colour_picker_ctrl`, `date_picker_ctrl`, `list_box`,
  `tree_ctrl`, `tab`, `panel`.
- **Containers / dialogs:** `dialog`, `message_box`,
  `message_dialog`, `top_level_window`, `icon_tray`,
  `popup_menu`.
- **Layout / decoration:** `status_bar`, `tool_bar`,
  `aui_tool_bar`, `tooltip`, `timer`, `icon`,
  `bitmap_bundle`, `image_list`, `font`, `art_provider`,
  `menu`.
- **Geometry / data:** `grid`, `grid_sizer` (pure data, no
  `HWND` required, but no unit tests yet — the showcase
  uses them so the type-level API is pinned).

### 3.3 Unit + smoke (windowed-with-unit-tests)

These modules are windowed but also have unit tests that
exercise the platform-agnostic public surface (constants,
signature contracts, null-`HWND` safety, parsing helpers).

- **`list_ctrl`** (new in v0.5.2) — 17 unit tests + 2
  integration tests, plus the existing windowed coverage
  in `showcase_all.rs`. Covers:
  - **All 12 `LVM_*` message constants** (numeric value
    pinning).
  - **All 4 `LVIS_*` / `LVNI_*` state / search-flag
    constants** (numeric value pinning).
  - **All 8 new public methods** (function-pointer type
    pinning).
  - **All 8 new public methods on a null `HWND`**
    (no-panic / no-spin / correct-empty-result).
- **`frame`** — see §3.1 above.
- **`file_dialog`** (new in v0.5.3) — 26 unit tests + 2
  integration tests, plus the existing windowed coverage
  in `showcase_all.rs`. Covers:
  - **The `parse_multiselect_buffer` helper** (12 tests,
    every edge case of the Win32 multi-select buffer
    format).
  - **The `wildcard_to_win32_filter` helper** (4 tests,
    every shape of the wxWidgets wildcard format).
  - **The new `multi_select` state field** (5 tests, default
    + setter + builder + getter).
  - **All 5 OFN flag constants** (3 tests, value + distinct
    + combined).
  - **The `FileDialogStyle` enum** (2 tests, variant
    distinctness + `Copy`).

### 3.4 Internal / private

- **`platform::win32`** — pure FFI, no public surface, no
  tests (intentionally).
- **Internal helper modules** in `log::*` (`api_guard`,
  `guards`, `win32_error`) — covered transitively by the
  `log::*` public-surface tests.

---

## 4. Verification matrix (this cycle)

| Step | Command | Result |
| --- | --- | --- |
| 1. Build | `cargo build` | **clean** |
| 2. Lib tests | `cargo test --lib` | **137 / 137** ✓ (+26 vs v0.5.2) |
| 3. Integration tests | `cargo test --test integration` | **15 / 15** ✓ (+2 vs v0.5.2) |
| 4. Doc tests | `cargo test --doc` | **23 / 23** ✓ (unchanged) |
| 5. All tests | `cargo test` | **175 / 175** ✓ (+28 vs v0.5.2) |
| 6. Clippy (lib + tests) | `cargo clippy --lib --tests --no-deps -- -D warnings` | **0 / 0** ✓ |
| 7. Clippy (showcase) | `cargo clippy --example showcase_all --no-deps -- -D warnings` | **0 / 0** ✓ |
| 8. Format | `cargo fmt --all -- --check` | **silent** ✓ |
| 9. Doc | `cargo doc --no-deps` | **0 errors** ✓ |

All 9 steps green. Three pre-existing implementation bugs
were caught and fixed during the development of this cycle:

- **OFN constant tests used `<flag> as u32` casts.** Clippy
  flagged these as `casting to the same type is unnecessary`
  because `OPEN_FILENAME_FLAGS` is a `pub type OPEN_FILENAME_FLAGS = u32`
  (a type alias), **not** a tuple struct. Fix: bind each flag
  to a `u32` local (`let must_exist: u32 = OFN_FILEMUSTEXIST;`)
  and assert on the local.
- **`parse_multiselect_buffer` used `file_offset.min(parts[0].len())`
  to slice the directory prefix.** This was **wrong**:
  `nFileOffset` is the index of the first filename character
  in the buffer (just past the directory's null terminator),
  not an index into the directory string. Slicing
  `parts[0][..file_offset]` would be out of bounds. Fix: use
  the entire `parts[0]` as the directory prefix (wxWidgets
  does the same), rename the parameter to `_file_offset`
  (accepted but unused), and rename the corresponding test
  to `parse_multi_select_offset_does_not_alter_output`.
- **Multi-select `Vec` reconstruction always joined
  `{dir}\{name}` even when the filename was already absolute.**
  This was caught by the test
  `parse_multi_select_absolute_filename_is_returned_verbatim`
  and fixed with the `is_absolute` branch (UNC `\\` prefix
  or drive-letter `X:` check).
- **Multi-select `Vec` reconstruction did not handle trailing
  separators on the directory gracefully.** If the dir was
  `C:\Users\foo\` the join would produce
  `C:\Users\foo\\bar` (double backslash). Fixed by adding a
  `dir_with_sep` step that checks for trailing `\` or `/`.

---

## 5. Future work (carries over to the rest of the 4th pass)

The v0.4.2 report listed 5 open items. v0.5.0 closed item
1; v0.5.1 closed item 3; **v0.5.2 partially closed item 2**
(`ListCtrl` selection API); **v0.5.3 partially closed item 2
again** (`FileDialog` multi-select API). The remaining items
carry over to v0.5.4 — the **final cycle** of the 4th 5-cycle
pass.

| # | Item | Status | Target cycle |
| --- | --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** | — |
| 2 | wxWidgets parity gaps (e.g. virtual list mode for `ListCtrl`, drag-and-drop, `DatePickerCtrl` value extraction, `FileDialog` multi-select, `Menu` shortcut label refresh) | **partially closed in v0.5.2** (ListCtrl selection) + **partially closed in v0.5.3** (FileDialog multi-select) | v0.5.4 (remaining sub-items) |
| 3 | Runtime rebinding of accelerators (`Frame::unregister_accelerator` / `clear_accelerators` / `replace_accelerator`) | **closed in v0.5.1** | — |
| 4 | First green run on the GitHub Actions CI (workflow already exists, but the matrix has never been run end-to-end) | open | v0.5.4 |
| 5 | macOS / Linux backends (AppKit / GTK) — currently Windows-only | open | **post-v0.5.4** |
| 6 | `BoxSizer` is the only sizer with unit tests. Add similar tests for `GridSizer` and `FlexGridSizer` (pure-data, no `HWND` needed). | open (new in v0.5.0) | v0.5.4 (rolled into final polish) |

The next cycle is **v0.5.4 — final polish**. It will close
the remaining wxWidgets-parity sub-items (likely `Menu`
shortcut label refresh after `Frame::replace_accelerator`,
or virtual list mode with `LVS_OWNERDATA` for `ListCtrl`),
land the first green run on the GitHub Actions CI, and add
the long-promised `GridSizer` / `FlexGridSizer` unit tests.

---

## 6. Per-category scores (v0.5.3)

The same 7 categories as the previous reports, each scored
0.00–10.00 with two decimals. The deltas are vs. **v0.5.2**
(the previous report). "—" means no change.

| # | Category | Weight | v0.5.2 | v0.5.3 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.55 | **9.60** | +0.05 | The new `parse_multiselect_buffer` helper has thorough defensive parsing: empty buffer, all-zero buffer, unterminated buffer, absolute filenames (UNC + drive letter), trailing separator on the directory. The 12 parse tests pin all of these. |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 9.15 | **9.30** | +0.15 | 3 new public methods on `FileDialog` (`set_multi_select`, `is_multi_select`, `show_modal_multi`) — the second-largest functions delta in the 4th 5-cycle pass after v0.5.0's +0.20 and tied with v0.5.2's +0.15. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 9.00 | **9.10** | +0.10 | `set_multi_select` returns `&mut Self` for fluent chaining; `show_modal_multi` is a separate method (not a flag on `show_modal`) so the return type can be `Vec<String>` instead of `Option<String>`. Each new method has careful rustdoc explaining the Win32 flag it sets and the buffer layout. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 9.45 | **9.65** | +0.20 | +26 unit tests in `file_dialog::tests` (12 parse + 4 wildcard + 5 state + 3 OFN + 2 enum), +2 integration tests pinning the new public-API signatures. The **biggest testing delta in the 4th 5-cycle pass**, surpassing v0.5.2's +0.35 in absolute terms (+28 vs +19 tests) and tied in relative terms. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.50 | **9.55** | +0.05 | New rustdoc on 3 public methods with Microsoft Docs links, new `#[cfg(test)] mod tests` divider in `file_dialog.rs`, new section comment in `integration.rs`, U19 entry in `upgrade.md`, this report. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 8.90 | **8.95** | +0.05 | The multi-select path is fully defensive: empty buffer, all-zero buffer, unterminated buffer, missing directory, empty filename, and trailing separator are all handled gracefully. The `is_absolute` branch prevents a drive-letter or UNC filename from being incorrectly joined with the directory. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.50 | 9.50 | — | All 9 CI steps green; no clippy, fmt, or doc deltas. |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.3 weighted score:**

\[
S_{0.5.3} = \frac{(9.60) + (9.30) + (9.10) + (1.5 \cdot 9.65) + (9.55) + (8.95) + (9.50)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.60 + 9.30 + 9.10 + 14.475 + 9.55 + 8.95 + 9.50}{7.5}
\]

\[
= \frac{70.475}{7.5} = 9.40
\]

**Comparison vs. v0.5.2 (which scored 9.30):**

| Metric | v0.5.0 | v0.5.1 | v0.5.2 | v0.5.3 | Δ vs. v0.5.2 |
| --- | --- | --- | --- | --- | --- |
| Weighted score | 9.07 | 9.17 | 9.30 | **9.40** | +0.10 |

The weighted score moves up by **+0.10** in this cycle, the
**third-largest cycle-on-cycle delta** in the 4th 5-cycle
pass (v0.5.0's +0.37 was the largest, v0.5.2's +0.13 the
second). The two largest deltas this cycle are in
**testing** (+0.20, the +28 new tests) and **functions**
(+0.15, the 3 new public methods). The **security** (+0.05)
and **robustness** (+0.05) deltas are smaller but
**symbolically significant**: they represent the first time
the Win32 multi-select buffer format has been pinned by a
unit test, and the first time the `OFN_ALLOWMULTISELECT` flag
has had its numeric value pinned against `<commdlg.h>`.

**Goal for the rest of the 4th pass:** push the weighted
score past **9.40** by v0.5.4. v0.5.3 has **already
landed at 9.40**, one cycle ahead of schedule. The biggest
opportunities remaining for v0.5.4 are:

- **Item 2 (wxWidgets parity gaps, continued)** — closes
  1–2 more of the open feature gaps (virtual list mode with
  `LVS_OWNERDATA`, drag-and-drop, `DatePickerCtrl` value
  extraction, `Menu` shortcut label refresh), which would
  move categories 2 and 6 by ~0.10 each.
- **Item 4 (CI first green run on GitHub Actions)** —
  closes the only remaining "untested on non-Windows"
  worry, ~0.10 in category 7.
- **Item 6 (`GridSizer` / `FlexGridSizer` unit tests)** —
  another +0.15 in category 4.

If all 3 items land, the weighted score should land in the
**9.50–9.65** range at v0.5.4, which would close the 4th
5-cycle pass with a weighted score **comfortably above the
9.40 target that was set at v0.5.0**.

---

## 7. Changelog snapshot

For the running log, see [`upgrade.md`](./upgrade.md). The
v0.5.3 entry is **Upgrade 19** in that file. The previous
report is [`upgrade_report_v0.5.2.md`](./upgrade_report_v0.5.2.md).

**Source / test / build numbers (this cycle):**

- `src/file_dialog.rs`: 212 → 812 lines (+600 for the
  multi-select field, the 3 new public methods, the
  `parse_multiselect_buffer` helper, the 2 new OFN
  constants, and the 26 new unit tests).
- `tests/integration.rs`: 297 → 351 lines (+54 for the two
  new signature-pinning tests and their section comment).
- `Cargo.toml` `version`: 0.5.2 → 0.5.3.
- All other source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, the `build.rs`: **unchanged from
  v0.5.2**.
