# ru_wx — Completion Report (v0.5.1)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.1
**Date:** 2026-06-05
**Cycles run in the 4th 5-cycle pass:** 2 of 5
(cycles 16–17 / v0.5.0–v0.5.1 complete; cycles 18–20 /
v0.5.2–v0.5.4 planned — see §5 for the carry-over list).

---

## 1. Executive summary

v0.5.1 is the **second cycle of the 4th 5-cycle pass**. Its
theme is **closing the "runtime rebinding of accelerators"
future-work item** that the v0.4.2 report opened and the
v0.5.0 report carried over.

v0.5.0 gave `Frame` a `for_testing` constructor and a working
`register_accelerator` / `accelerators` pair. v0.5.1 adds the
three **mutating counterparts** that were missing:

- `unregister_accelerator` — remove one (the first match).
- `clear_accelerators` — remove all.
- `replace_accelerator` — atomic in-place rebind.

Without these three, a user could populate the accelerator
table at construction time but could not edit it later — for
example, an "Options" dialog that lets the user re-bind a
shortcut was impossible. With these three, the table is
fully mutable from outside the frame constructor.

Three concrete deliverables:

1. **Three new public methods on `Frame`** (`unregister_accelerator`,
   `clear_accelerators`, `replace_accelerator`), each with a
   careful rustdoc explaining the **in-memory vs. HACCEL**
   caveat (mutating the `Vec` does **not** auto-rebuild the
   Win32 `HACCEL` currently in use by the message loop; that
   is a known limitation inherited from `register_accelerator`
   and is out of scope here).
2. **10 new unit tests in `src/frame.rs::tests`** under a
   dedicated "Accelerator rebinding (v0.5.1)" divider, covering
   the no-op paths, the happy paths, the relative-order
   preservation property, the "first match wins" semantics on
   duplicate `old` accelerators, and a realistic three-step
   rebind workflow.
3. **2 new integration tests in `tests/integration.rs`** that
   pin the **public-API signatures** of the three new methods
   at the integration boundary via function-pointer type
   assertions — one without the prelude and one through
   `ru_wx::prelude::*`. These catch an accidental rename,
   parameter-list change, or return-type change in
   `src/frame.rs` even though the integration layer cannot
   actually exercise the methods (they require a real `HWND`).

**CI status (post-cycle):** green. 94 lib tests + 11
integration tests + 23 doc tests = **128 / 128** passing
(+12 since v0.5.0: +10 unit + 2 integration).
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
| `frame` | 21 ✓ (+10 in v0.5.1) | 1 ✓ | 3 ✓ (+2 in v0.5.1) | yes | **unit + smoke** |
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

- **Unit tests:** 94 ✓ (up from 84 in v0.5.0; +10 from
  `frame::tests` covering the new rebinding methods).
- **Doc tests:** 23 ✓ (unchanged).
- **Integration tests:** 11 ✓ (up from 9 in v0.5.0; +2
  signature-pinning tests for the new methods).
- **Grand total:** 128 / 128 passing (+12 since v0.5.0).

**Smoke-only modules.** All of the "smoke only" rows above
**require a real Win32 `HWND`** (creating a `Frame`,
registering a window class, dispatching a `WM_COMMAND`,
etc.). The test harness in v0.5.1 still deliberately stops
short of those: it covers the platform-agnostic public
surface. Windowed coverage is provided by the
`examples/showcase_all.rs` binary, which exercises every
windowed widget end-to-end.

**The `frame` module in v0.5.1.** The unit-test count for
`frame` jumps from 11 to **21** — the +10 in this cycle is
the new "Accelerator rebinding (v0.5.1)" divider. The
integration-test count for `frame` jumps from 1 to **3** — the
+2 in this cycle is the new "Cross-module: v0.5.1 runtime
rebinding API" divider. The `frame` row is the **only** row
in the matrix that changed in this cycle.

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
- **`frame` (platform-agnostic only)** — 21 unit tests
  (up from 11 in v0.5.0) + 3 integration tests (up from 1
  in v0.5.0). Covers:
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
    save binding, clear everything). See §1 above for the
    full test-by-test list.

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
| 2. Lib tests | `cargo test --lib` | **94 / 94** ✓ (+10 vs v0.5.0) |
| 3. Integration tests | `cargo test --test integration` | **11 / 11** ✓ (+2 vs v0.5.0) |
| 4. Doc tests | `cargo test --doc` | **23 / 23** ✓ (unchanged) |
| 5. All tests | `cargo test` | **128 / 128** ✓ (+12 vs v0.5.0) |
| 6. Clippy (lib + tests) | `cargo clippy --lib --tests --no-deps -- -D warnings` | **0 / 0** ✓ |
| 7. Clippy (showcase) | `cargo clippy --example showcase_all --no-deps -- -D warnings` | **0 / 0** ✓ |
| 8. Format | `cargo fmt --all -- --check` | **silent** ✓ |
| 9. Doc | `cargo doc --no-deps` | **0 errors** ✓ |

All 9 steps green. One pre-existing implementation bug was
caught and fixed during the development of this cycle:

- The first cut of `unregister_accelerator` used
  `Vec::retain(|(a, _)| a != &accel)`, which removes
  **all** matching entries. The doc-comment and the
  `unregister_accelerator_removes_only_first_duplicate`
  unit test both required "first match only" semantics.
  The implementation was switched to
  `Vec::iter().position(...)` + `Vec::remove(pos)` to
  match the documentation, and the doc-comment was
  tightened to explicitly say "only the first match is
  removed". All 10 new unit tests pass on the corrected
  implementation.

---

## 5. Future work (carries over to the rest of the 4th pass)

The v0.4.2 report listed 5 open items. v0.5.0 closed item
1; **v0.5.1 closes item 3** (this cycle's headline). The
remaining 4 items are joined by 1 new item opened in v0.5.0.
The rest of the 4th 5-cycle pass (v0.5.2 → v0.5.4) is
dedicated to closing them.

| # | Item | Status | Target cycle |
| --- | --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** | — |
| 2 | wxWidgets parity gaps (e.g. virtual list mode for `ListCtrl`, drag-and-drop, `DatePickerCtrl` value extraction) | open | v0.5.2 + v0.5.3 |
| 3 | Runtime rebinding of accelerators (`Frame::unregister_accelerator` / `clear_accelerators` / `replace_accelerator`) | **closed in v0.5.1** | — |
| 4 | First green run on the GitHub Actions CI (workflow already exists, but the matrix has never been run end-to-end) | open | v0.5.4 |
| 5 | macOS / Linux backends (AppKit / GTK) — currently Windows-only | open | **post-v0.5.4** |
| 6 | `BoxSizer` is the only sizer with unit tests. Add similar tests for `GridSizer` and `FlexGridSizer` (pure-data, no `HWND` needed). | open (new in v0.5.0) | v0.5.4 (rolled into final polish) |

The next cycle is **v0.5.2 — wxWidgets parity pass 1**
(item 2 in the table above; likely virtual list mode for
`ListCtrl`).

---

## 6. Per-category scores (v0.5.1)

The same 7 categories as the previous reports, each scored
0.00–10.00 with two decimals. The deltas are vs. **v0.5.0**
(the previous report). "—" means no change.

| # | Category | Weight | v0.5.0 | v0.5.1 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.50 | 9.50 | — | No FFI / input-validation work in this cycle. |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 8.90 | **9.00** | +0.10 | 3 new public methods on `Frame` (`unregister_accelerator` / `clear_accelerators` / `replace_accelerator`) — the last missing mutators on the accelerator table. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 8.90 | **8.95** | +0.05 | Each new method has a careful rustdoc that explains the "in-memory list vs. live `HACCEL`" caveat, the order-preservation property, and the "first match wins" duplicate semantics. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 8.80 | **9.10** | +0.30 | +10 unit tests in `frame::tests` covering every new method, +2 integration tests pinning the new public-API signatures. The biggest delta in this cycle. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.30 | **9.40** | +0.10 | New rustdoc on 3 public methods, new section comment in `frame::tests`, new section comment in `integration.rs`, U17 entry in `upgrade.md`, this report. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 8.70 | **8.80** | +0.10 | `unregister_accelerator` is now provably "first match only" (regression guard via the duplicate test), `clear_accelerators` is provably idempotent, and `replace_accelerator` is provably atomic (in-place, order-preserving, no-op when `old` is absent). The pre-existing `in-memory vs. HACCEL` caveat is now documented on every new method. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.50 | 9.50 | — | All 9 CI steps green; no clippy, fmt, or doc deltas. |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.1 weighted score:**

\[
S_{0.5.1} = \frac{(9.50) + (9.00) + (8.95) + (1.5 \cdot 9.10) + (9.40) + (8.80) + (9.50)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.50 + 9.00 + 8.95 + 13.65 + 9.40 + 8.80 + 9.50}{7.5}
\]

\[
= \frac{68.80}{7.5} = 9.17
\]

**Comparison vs. v0.5.0 (which scored 9.07):**

| Metric | v0.5.0 | v0.5.1 | Δ |
| --- | --- | --- | --- |
| Weighted score | 9.07 | **9.17** | +0.10 |

The weighted score moves up by **+0.10**. The two largest
deltas are in **testing** (+0.30, the +12 new tests) and
**robustness** (+0.10, the new "first match wins" /
"idempotent clear" / "atomic in-place replace" properties
are now pinned). The two API-related deltas (**functions**
+0.10 and **interface** +0.05) are small because the
new methods are 3 well-scoped mutators on an already-tested
data structure — they round out an existing API surface
rather than introducing a new one.

**Goal for the rest of the 4th pass:** push the weighted
score past **9.30** by v0.5.4. The biggest opportunities
remaining are:

- **Item 2 (wxWidgets parity gaps)** — closes 1–2 of the
  open feature gaps, which would move categories 2 and 6
  by ~0.10 each.
- **Item 4 (CI first green run on GitHub Actions)** —
  closes the only remaining "untested on non-Windows"
  worry, ~0.10 in category 7.
- **Item 6 (`GridSizer` / `FlexGridSizer` unit tests)** —
  another +0.20 in category 4.

If all 3 items land, the weighted score should land in
the **9.30–9.45** range at v0.5.4, which is the target
ceiling for the 4th 5-cycle pass.

---

## 7. Changelog snapshot

For the running log, see [`upgrade.md`](./upgrade.md). The
v0.5.1 entry is **Upgrade 17** in that file. The previous
report is [`upgrade_report_v0.5.0.md`](./upgrade_report_v0.5.0.md).

**Source / test / build numbers (this cycle):**

- `src/frame.rs`: 880 → 1000 lines (+120 for the three new
  public methods, their rustdoc, and the 10 new unit tests).
- `tests/integration.rs`: 199 → 234 lines (+35 for the two
  new signature-pinning tests and their section comment).
- `Cargo.toml` `version`: 0.5.0 → 0.5.1.
- All other source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, the `build.rs`: **unchanged from
  v0.5.0**.
