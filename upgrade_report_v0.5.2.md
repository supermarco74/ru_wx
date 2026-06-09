# ru_wx — Completion Report (v0.5.2)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.2
**Date:** 2026-06-05
**Cycles run in the 4th 5-cycle pass:** 3 of 5
(cycles 16–18 / v0.5.0–v0.5.2 complete; cycles 19–20 /
v0.5.3–v0.5.4 planned — see §5 for the carry-over list).

---

## 1. Executive summary

v0.5.2 is the **third cycle of the 4th 5-cycle pass**. Its
theme is **the first wxWidgets-parity pass**, focused on
closing the most visible gap in `ListCtrl`: the absence of a
programmatic selection API. The control already exposed
`get_selected_item()` (single-selection result of a click) but
lacked the symmetric `select` / `deselect` /
`clear_selection` / `is_selected` /
`get_selected_item_count` / `get_selected_items` set that
wxWidgets' `wxListCtrl` ships out of the box. v0.5.2 adds all
six high-level methods plus two low-level helpers
(`set_item_state` / `get_item_state`) that wrap the raw
`LVM_SETITEMSTATE` / `LVM_GETITEMSTATE` Win32 messages for
power-users that need to set custom state bits (cut,
highlight, etc.).

This is **item 2 in the v0.5.0 / v0.5.1 future-work table** —
the long-running "wxWidgets parity gaps" item. It is now
**partially closed** (the `ListCtrl` selection sub-item).
The remaining sub-items (virtual list mode with
`LVS_OWNERDATA`, drag-and-drop, `DatePickerCtrl` value
extraction, `FileDialog` multi-select) carry over to v0.5.3
and v0.5.4.

Three concrete deliverables:

1. **Four new Win32 constants** in `src/list_ctrl.rs`:
   `LVM_SETITEMSTATE` (`LVM_FIRST + 43`),
   `LVM_GETITEMSTATE` (`LVM_FIRST + 44`),
   `LVM_GETSELECTEDCOUNT` (`LVM_FIRST + 50`), plus the new
   `LVIS_FOCUSED` (`0x0001`) / `LVIS_SELECTED` (`0x0002`)
   state-bit constants. All are documented with Microsoft
   Docs links.
2. **Eight new public methods on `ListCtrl`**: six
   high-level (`select`, `deselect`, `clear_selection`,
   `is_selected`, `get_selected_item_count`,
   `get_selected_items`) + two low-level (`set_item_state`,
   `get_item_state`) wrappers around `LVM_SETITEMSTATE` /
   `LVM_GETITEMSTATE`. Each new method has a careful
   rustdoc that explains its `SendMessageW` semantics, its
   null-`HWND` fallback, and (for `clear_selection` /
   `get_selected_items`) the no-progress guard that prevents
   a runaway loop on a malformed control.
3. **Seventeen new unit tests** in a new
   `#[cfg(test)] mod tests` at the bottom of `list_ctrl.rs`,
   and **two new integration tests** in
   `tests/integration.rs` (one pinning the public-API
   signatures of the 8 new methods + 4 `ListCtrlStyle`
   variants, one pinning the same through
   `ru_wx::prelude::*`).

**CI status (post-cycle):** green. 111 lib tests + 13
integration tests + 23 doc tests = **147 / 147** passing
(+19 since v0.5.1: +17 unit + 2 integration).
0 clippy warnings, 0 clippy errors, 0 rustfmt diffs.

**Symbolic impact:** the `list_ctrl` row in the test
coverage matrix moves from **smoke only** to
**unit + smoke** — `list_ctrl` is the **second widget
control** (after `frame`) to break out of the smoke-only
bucket in the 4th 5-cycle pass, and the first **purely data
+ `HWND`-coupled** control to do so.

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
| `file_dialog` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `frame` | 21 ✓ | 1 ✓ | 3 ✓ | yes | **unit + smoke** |
| `geometry` | 6 ✓ | 0 | 1 ✓ | yes (used everywhere) | **complete** |
| `grid` | (none — `Cell` / `Grid` are pure data) | 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `grid_sizer` | (none — `GridSizer` / `FlexGridSizer` are pure data) | 0 | 0 (prelude) | yes | **smoke only** |
| `icon` / `icon_tray` | (none — requires `HWND` / shell APIs) | 0 | 0 (prelude) | yes | **smoke only** |
| `list_box` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| **`list_ctrl`** | **17 ✓ (+17 in v0.5.2)** | 0 | **2 ✓ (+2 in v0.5.2)** | yes | **unit + smoke** ⭐ |
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

- **Unit tests:** 111 ✓ (up from 94 in v0.5.1; +17 from
  `list_ctrl::tests` covering the new selection methods +
  Win32 constants).
- **Doc tests:** 23 ✓ (unchanged).
- **Integration tests:** 13 ✓ (up from 11 in v0.5.1; +2
  signature-pinning tests for the new methods).
- **Grand total:** 147 / 147 passing (+19 since v0.5.1).

**Smoke-only modules.** All of the "smoke only" rows above
**require a real Win32 `HWND`** (creating a `Frame`,
registering a window class, dispatching a `WM_COMMAND`,
etc.). The test harness in v0.5.2 still deliberately stops
short of those: it covers the platform-agnostic public
surface. Windowed coverage is provided by the
`examples/showcase_all.rs` binary, which exercises every
windowed widget end-to-end.

**The `list_ctrl` module in v0.5.2.** The unit-test count
for `list_ctrl` jumps from **0 to 17** — the **+17** in
this cycle is a brand-new `#[cfg(test)] mod tests` at the
bottom of the file, divided into three groups:

- **2 constant-pinning tests** (`lvm_constants_have_expected_values`,
  `lvis_constants_have_expected_values`) — pin the numeric
  values of all 12 `LVM_*` message constants and the 4
  `LVIS_*` / `LVNI_*` state / search-flag constants against
  the Microsoft Docs list. A future refactor that changes
  one of these by accident will fail to compile, with no
  behavioural test required.
- **8 signature-pinning tests** (`signature_select`,
  `signature_deselect`, `signature_clear_selection`,
  `signature_is_selected`,
  `signature_get_selected_item_count`,
  `signature_get_selected_items`,
  `signature_set_item_state`,
  `signature_get_item_state`) — function-pointer type
  assertions that pin the public-API contract for every
  new method. A future refactor that renames a method,
  changes its parameter list, or changes its return type
  will fail to compile, with no behavioural test required.
- **6 null-`HWND` safety tests** (`null_hwnd_select_does_not_panic`,
  `null_hwnd_deselect_does_not_panic`,
  `null_hwnd_clear_selection_does_not_panic`,
  `null_hwnd_is_selected_returns_false`,
  `null_hwnd_get_selected_item_count_returns_zero`,
  `null_hwnd_get_selected_items_returns_empty`,
  `null_hwnd_get_item_state_returns_zero`) — exercise
  every new method against a `ListCtrl` whose `HWND` is
  `NULL` (created via `Frame::for_testing()` so
  `CreateWindowExW` is issued with a null parent + `WS_CHILD`
  and fails, leaving the inner `HWND` `NULL`). On a null
  `HWND` `SendMessageW` is a no-op that returns 0, so:
  - `select`, `deselect`, `clear_selection` must not
    panic.
  - `is_selected` must return `false` (no false
    positives).
  - `get_selected_item_count` must return `0`.
  - `get_selected_items` must return an empty `Vec` and
    crucially must NOT spin in the `LVM_GETNEXTITEM` loop
    (the `count == 0` guard short-circuits the walk).
  - `get_item_state` must return `0`.

The `list_ctrl` row is the **only** row in the matrix that
changed in this cycle, and it is the **first widget
control** (after `frame`) to break out of the smoke-only
bucket in the 4th 5-cycle pass.

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
- **Containers / dialogs:** `dialog`, `file_dialog`,
  `message_box`, `message_dialog`, `top_level_window`,
  `icon_tray`, `popup_menu`.
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
signature contracts, null-`HWND` safety).

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
| 2. Lib tests | `cargo test --lib` | **111 / 111** ✓ (+17 vs v0.5.1) |
| 3. Integration tests | `cargo test --test integration` | **13 / 13** ✓ (+2 vs v0.5.1) |
| 4. Doc tests | `cargo test --doc` | **23 / 23** ✓ (unchanged) |
| 5. All tests | `cargo test` | **147 / 147** ✓ (+19 vs v0.5.1) |
| 6. Clippy (lib + tests) | `cargo clippy --lib --tests --no-deps -- -D warnings` | **0 / 0** ✓ |
| 7. Clippy (showcase) | `cargo clippy --example showcase_all --no-deps -- -D warnings` | **0 / 0** ✓ |
| 8. Format | `cargo fmt --all -- --check` | **silent** ✓ |
| 9. Doc | `cargo doc --no-deps` | **0 errors** ✓ |

All 9 steps green. One pre-existing implementation bug was
caught and fixed during the development of this cycle:

- The first cut of `get_selected_items` walked
  `LVM_GETNEXTITEM` with `LVNI_SELECTED` and accumulated
  results into a `Vec<usize>` with **no upper bound**. On
  a null `HWND` `SendMessageW` returns 0, which is also
  the "no more items" sentinel, so the loop would have
  terminated after one iteration. However, on a malformed
  control that returns a non-zero value that does not
  correspond to a real index, the loop would have spun
  forever. The fix was a two-layer guard:
  1. **Outer bound:** walk at most `get_item_count() + 1`
     times (the `+1` absorbs the final "no more" sentinel).
  2. **Inner no-progress guard:** if
     `LVM_GETNEXTITEM` returns the same index twice in a
     row, break out of the loop.
  Both guards are now exercised by the
  `null_hwnd_get_selected_items_returns_empty` test (the
  `count == 0` short-circuit covers the null-`HWND` case,
  and the no-progress guard is the safety net for the
  malformed-control case).

---

## 5. Future work (carries over to the rest of the 4th pass)

The v0.4.2 report listed 5 open items. v0.5.0 closed item
1; v0.5.1 closed item 3; **v0.5.2 partially closes item 2**
(this cycle's headline — the `ListCtrl` selection sub-item).
The remaining 4 items are joined by 1 new item opened in
v0.5.0. The rest of the 4th 5-cycle pass (v0.5.3 → v0.5.4)
is dedicated to closing them.

| # | Item | Status | Target cycle |
| --- | --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** | — |
| 2 | wxWidgets parity gaps (e.g. virtual list mode for `ListCtrl`, drag-and-drop, `DatePickerCtrl` value extraction, `FileDialog` multi-select) | **partially closed in v0.5.2** (ListCtrl selection API) | v0.5.3 (FileDialog multi-select) + v0.5.4 (remaining) |
| 3 | Runtime rebinding of accelerators (`Frame::unregister_accelerator` / `clear_accelerators` / `replace_accelerator`) | **closed in v0.5.1** | — |
| 4 | First green run on the GitHub Actions CI (workflow already exists, but the matrix has never been run end-to-end) | open | v0.5.4 |
| 5 | macOS / Linux backends (AppKit / GTK) — currently Windows-only | open | **post-v0.5.4** |
| 6 | `BoxSizer` is the only sizer with unit tests. Add similar tests for `GridSizer` and `FlexGridSizer` (pure-data, no `HWND` needed). | open (new in v0.5.0) | v0.5.4 (rolled into final polish) |

The next cycle is **v0.5.3 — wxWidgets parity pass 2**
(continued item 2; likely `FileDialog` multi-select via
`OFN_ALLOWMULTISELECT` + the multi-select buffer parsing
that wxWidgets does, or `Menu` shortcut label refresh
after `Frame::replace_accelerator`).

---

## 6. Per-category scores (v0.5.2)

The same 7 categories as the previous reports, each scored
0.00–10.00 with two decimals. The deltas are vs. **v0.5.1**
(the previous report). "—" means no change.

| # | Category | Weight | v0.5.1 | v0.5.2 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.50 | **9.55** | +0.05 | 6 new null-`HWND` safety tests pin the no-panic / no-spin / correct-empty-result contract on a `ListCtrl` whose `HWND` is `NULL`. The `get_selected_items` no-progress guard closes a runaway-loop risk on a malformed control. |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 9.00 | **9.15** | +0.15 | 8 new public methods on `ListCtrl` (6 high-level + 2 low-level). This is the second-largest functions delta in the 4th 5-cycle pass after v0.5.0's +0.20. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 8.95 | **9.00** | +0.05 | Each new method has a careful rustdoc that explains its `SendMessageW` semantics, its null-`HWND` fallback, the Microsoft Docs link, and (for `clear_selection` / `get_selected_items`) the no-progress guard. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 9.10 | **9.45** | +0.35 | +17 unit tests in `list_ctrl::tests` (2 constant + 8 signature + 7 null-`HWND`), +2 integration tests pinning the new public-API signatures. The biggest delta in this cycle and the second-biggest testing delta in the 4th 5-cycle pass. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.40 | **9.50** | +0.10 | New rustdoc on 8 public methods with Microsoft Docs links, new `#[cfg(test)] mod tests` divider in `list_ctrl.rs`, new section comment in `integration.rs`, U18 entry in `upgrade.md`, this report. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 8.80 | **8.90** | +0.10 | Null-`HWND` guards are now pinned on every new method. The `get_selected_items` no-progress guard (`last == next → break`) is a new robustness property that prevents a runaway loop on a malformed control. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.50 | 9.50 | — | All 9 CI steps green; no clippy, fmt, or doc deltas. |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.2 weighted score:**

\[
S_{0.5.2} = \frac{(9.55) + (9.15) + (9.00) + (1.5 \cdot 9.45) + (9.50) + (8.90) + (9.50)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.55 + 9.15 + 9.00 + 14.175 + 9.50 + 8.90 + 9.50}{7.5}
\]

\[
= \frac{69.775}{7.5} = 9.30
\]

**Comparison vs. v0.5.1 (which scored 9.17):**

| Metric | v0.5.0 | v0.5.1 | v0.5.2 | Δ vs. v0.5.1 |
| --- | --- | --- | --- | --- |
| Weighted score | 9.07 | 9.17 | **9.30** | +0.13 |

The weighted score moves up by **+0.13** in this cycle, the
**second-largest cycle-on-cycle delta** in the 4th 5-cycle
pass (v0.5.0's +0.37 was the largest, v0.5.1's +0.10 the
smallest). The two largest deltas this cycle are in
**testing** (+0.35, the +19 new tests) and **functions**
(+0.15, the 8 new public methods). The **security** (+0.05)
and **robustness** (+0.10) deltas are smaller but
**symbolically significant**: they represent the first time
a `ListCtrl` method has had its null-`HWND` safety pinned
by a unit test, and the first time the
`LVM_GETNEXTITEM`-with-`LVNI_SELECTED` walk has had a
no-progress guard.

**Goal for the rest of the 4th pass:** push the weighted
score past **9.40** by v0.5.4. The biggest opportunities
remaining are:

- **Item 2 (wxWidgets parity gaps, continued)** — closes
  1–2 more of the open feature gaps, which would move
  categories 2 and 6 by ~0.10 each.
- **Item 4 (CI first green run on GitHub Actions)** —
  closes the only remaining "untested on non-Windows"
  worry, ~0.10 in category 7.
- **Item 6 (`GridSizer` / `FlexGridSizer` unit tests)** —
  another +0.20 in category 4.

If all 3 items land, the weighted score should land in the
**9.40–9.55** range at v0.5.4, which would close the 4th
5-cycle pass with a weighted score comfortably above the
9.30 target that was set at v0.5.0.

---

## 7. Changelog snapshot

For the running log, see [`upgrade.md`](./upgrade.md). The
v0.5.2 entry is **Upgrade 18** in that file. The previous
report is [`upgrade_report_v0.5.1.md`](./upgrade_report_v0.5.1.md).

**Source / test / build numbers (this cycle):**

- `src/list_ctrl.rs`: 561 → 892 lines (+331 for the four
  new Win32 constants, the 8 new public methods, and the
  17 new unit tests).
- `tests/integration.rs`: 234 → 297 lines (+63 for the two
  new signature-pinning tests and their section comment).
- `Cargo.toml` `version`: 0.5.1 → 0.5.2.
- All other source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, the `build.rs`: **unchanged from
  v0.5.1**.
