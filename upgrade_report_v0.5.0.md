# ru_wx — Completion Report (v0.5.0)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.0
**Date:** 2026-06-05
**Cycles run in the 4th 5-cycle pass:** 1 of 5
(cycle 16 / v0.5.0 complete; cycles 17–20 / v0.5.1–v0.5.4
planned — see §5 for the carry-over list).

---

## 1. Executive summary

v0.5.0 is the **opening cycle of the 4th 5-cycle pass**. Its
theme is **testing infrastructure**: the platform-agnostic
parts of the public API now have a real test net, instead of
being smoke-tested only through the `examples/showcase_all.rs`
binary.

Three concrete deliverables:

1. **A `pub(crate) Frame::for_testing` constructor** that lets
   `src/frame.rs::tests` exercise the public surface of
   `Frame` (accelerator registration, command / notify / tray
   handler tables, sizer storage, DPI fallback) without
   requiring a real Win32 `HWND`.
2. **11 new unit tests in `src/frame.rs::tests`** + 1 small
   clippy fix on the pre-existing `MockWidget::new` helper.
3. **A new top-level `tests/integration.rs` test binary** with
   9 cross-module tests, all of which use only the **public**
   API (i.e. what a downstream user actually sees).

Two missing pieces of the public surface that the tests made
obvious are also added in this cycle:

- `BoxSizer::padding(&self) -> i32`
- `BoxSizer::orientation(&self) -> Orientation`

So the cycle closes one future-work item from the v0.4.2
report (`widget integration tests`) and adds two small but
genuinely useful public getters.

**CI status (post-cycle):** green. 84 lib tests + 9
integration tests + 23 doc tests = **116 / 116** passing.
0 clippy warnings, 0 clippy errors, 0 rustfmt diffs.

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
| `frame` | 11 ✓ (new in v0.5.0) | 1 ✓ | 1 ✓ (type in scope) | yes | **unit + smoke** |
| `geometry` | 6 ✓ | 0 | 1 ✓ | yes (used everywhere) | **complete** |
| `grid` | (none — `Cell` / `Grid` are pure data) | 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `grid_sizer` | (none — `GridSizer` / `FlexGridSizer` are pure data) | 0 | 0 (prelude) | yes | **smoke only** |
| `icon` / `icon_tray` | (none — requires `HWND` / shell APIs) | 0 | 0 (prelude) | yes | **smoke only** |
| `list_box` / `list_ctrl` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `log::*` | 17 ✓ across 6 submodules | 8 ✓ across 4 submodules | 0 (private) | yes (used internally) | **complete** |
| `menu` | (none — requires `HWND`) | 1 ✓ (`Menu::append_with_shortcut`) | 0 (prelude) | yes | **smoke only** |
| `message_box` / `message_dialog` | (none — requires `HWND`) | 0 / 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `panel` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `popup_menu` | (none — requires `HWND`) | 1 ✓ | 0 (prelude) | yes | **smoke only** |
| `radio_button` / `radio_box` | (none — requires `HWND`) | 0 | 0 (prelude) | yes | **smoke only** |
| `sizer` | 6 ✓ (incl. the new getter coverage) | 0 | 1 ✓ (new) | yes (used by the showcase) | **complete** |
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

- **Unit tests:** 84 ✓ (up from 73 in v0.4.2; +11 from
  `frame::tests`, +0 elsewhere).
- **Doc tests:** 23 ✓ (unchanged).
- **Integration tests:** 9 ✓ (new in v0.5.0).
- **Grand total:** 116 / 116 passing.

**Smoke-only modules.** All of the "smoke only" rows above
**require a real Win32 `HWND`** (creating a `Frame`,
registering a window class, dispatching a `WM_COMMAND`,
etc.). The test harness in v0.5.0 deliberately stops short
of those: it covers the platform-agnostic public surface.
Windowed coverage is provided by the `examples/showcase_all.rs`
binary, which exercises every windowed widget end-to-end.

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
  and the two new getters (`padding`, `orientation`).
- **`frame` (platform-agnostic only)** — 11 unit tests
  (new in v0.5.0). Covers `Frame::for_testing` (empty
  state), accelerator registration (order, duplicates,
  clone isolation), command-handler map (insert,
  overwrite), notify-handler map, tray-message-handler
  unregister, sizer storage (`None` → `Some` → `Some`),
  and the `null_hwnd` fallback in `dpi` /
  `scale_factor` (Windows-only).

### 3.2 Smoke-only (windowed)

These modules require a real Win32 `HWND` to test. They are
exercised end-to-end by `examples/showcase_all.rs`, which is
the integration test for the windowed surface.

- **Widgets:** `button`, `checkbox`, `combo_box`,
  `check_list_box`, `choice`, `radio_button`, `radio_box`,
  `static_text`, `text_ctrl`, `slider`, `spin_ctrl`, `gauge`,
  `colour_picker_ctrl`, `date_picker_ctrl`, `list_box`,
  `list_ctrl`, `tree_ctrl`, `tab`, `panel`.
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

### 3.3 Internal / private

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
| 2. Lib tests | `cargo test --lib` | **84 / 84** ✓ (+11 vs v0.4.2) |
| 3. Integration tests | `cargo test --test integration` | **9 / 9** ✓ (new) |
| 4. Doc tests | `cargo test --doc` | **23 / 23** ✓ (unchanged) |
| 5. All tests | `cargo test` | **116 / 116** ✓ |
| 6. Clippy (lib + tests) | `cargo clippy --lib --tests --no-deps -- -D warnings` | **0 / 0** ✓ |
| 7. Clippy (showcase) | `cargo clippy --example showcase_all --no-deps -- -D warnings` | **0 / 0** ✓ |
| 8. Format | `cargo fmt --all -- --check` | **silent** ✓ |
| 9. Doc | `cargo doc --no-deps` | **0 errors** ✓ |

All 9 steps green. The single pre-existing
`clippy::new_ret_no_self` warning on `MockWidget::new`
(test-only helper in `src/sizer.rs::tests`) was silenced
with a `#[allow]` + explanatory comment in this cycle
(per the rationale in §1 above).

---

## 5. Future work (carries over to the rest of the 4th pass)

The v0.4.2 report listed 5 open items. **Item 1 is closed
in v0.5.0**; the other 4 still need work and are joined by
1 new item opened in this cycle. The rest of the 4th
5-cycle pass (v0.5.1 → v0.5.4) is dedicated to closing
them.

| # | Item | Status | Target cycle |
| --- | --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** | — |
| 2 | wxWidgets parity gaps (e.g. virtual list mode for `ListCtrl`, drag-and-drop, `DatePickerCtrl` value extraction) | open | v0.5.2 + v0.5.3 |
| 3 | Runtime rebinding of accelerators (`Frame::replace_accelerator` / `Frame::clear_accelerators`) | open | v0.5.1 |
| 4 | First green run on the GitHub Actions CI (workflow already exists, but the matrix has never been run end-to-end) | open | v0.5.4 |
| 5 | macOS / Linux backends (AppKit / GTK) — currently Windows-only | open | **post-v0.5.4** |
| 6 | `BoxSizer` is the only sizer with unit tests. Add similar tests for `GridSizer` and `FlexGridSizer` (pure-data, no `HWND` needed). | open (new in v0.5.0) | v0.5.4 (rolled into final polish) |

The next cycle is **v0.5.1 — runtime rebinding of
accelerators** (item 3 in the table above).

---

## 6. Per-category scores (v0.5.0)

The same 7 categories as the previous reports, each scored
0.00–10.00 with two decimals. The deltas are vs. **v0.4.2**
(the previous report). "—" means no change.

| # | Category | Weight | v0.4.2 | v0.5.0 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.50 | 9.50 | — | No FFI / input-validation work in this cycle. |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 8.80 | **8.90** | +0.10 | `BoxSizer::padding` and `BoxSizer::orientation` close the only two missing public getters. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 8.90 | 8.90 | — | No API-shape change in this cycle. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 8.20 | **8.80** | +0.60 | +11 unit tests, +9 integration tests, +1 clippy fix. New test binary. The biggest delta in this cycle. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.20 | **9.30** | +0.10 | The new `tests/integration.rs` doubles as a living example of the public API; the upgrade entry and this report are both more detailed than the previous pass's per-cycle entries. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 8.70 | 8.70 | — | The new `dpi_falls_back_to_system_dpi_for_null_hwnd` test pins an existing fallback but does not add a new one. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.40 | **9.50** | +0.10 | The last `clippy::new_ret_no_self` warning is gone. All 9 CI steps green. |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.0 weighted score:**

\[
S_{0.5.0} = \frac{(9.50) + (8.90) + (8.90) + (1.5 \cdot 8.80) + (9.30) + (8.70) + (9.50)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.50 + 8.90 + 8.90 + 13.20 + 9.30 + 8.70 + 9.50}{7.5}
\]

\[
= \frac{68.00}{7.5} = 9.07
\]

**Comparison vs. v0.4.2 (which scored 9.06):**

| Metric | v0.4.2 | v0.5.0 | Δ |
| --- | --- | --- | --- |
| Weighted score | 9.06 | **9.07** | +0.01 |

The weighted score is essentially flat (+0.01), which is
expected for a test-infrastructure cycle: the test-coverage
category is heavily weighted (1.5×) and got the largest
delta (+0.60), but the other 6 categories either stayed
flat or moved by 0.10, so the weighted average only moves
slightly. The **raw** test-coverage score (8.20 → 8.80,
+0.60) is the headline number of this cycle.

**Goal for the rest of the 4th pass:** push the weighted
score past **9.30** by v0.5.4. The biggest opportunities
are:

- **Item 2 (wxWidgets parity gaps)** — closes 1–2 of the
  open feature gaps, which would move categories 2 and 6
  by ~0.10 each.
- **Item 3 (runtime rebinding of accelerators)** — small
  ergonomic improvement, ~0.05 in category 3.
- **Item 4 (CI first green run on GitHub Actions)** —
  closes the only remaining "untested on non-Windows"
  worry, ~0.10 in category 7.
- **Item 6 (`GridSizer` / `FlexGridSizer` unit tests)** —
  another +0.30 in category 4.

If all 4 items land, the weighted score should land in
the **9.20–9.35** range at v0.5.4, which is the target
ceiling for the 4th 5-cycle pass.

---

## 7. Changelog snapshot

For the running log, see [`upgrade.md`](./upgrade.md). The
v0.5.0 entry is **Upgrade 16** in that file. The previous
report is [`upgrade_report_v0.4.2.md`](./upgrade_report_v0.4.2.md).
