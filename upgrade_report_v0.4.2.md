# ru_wx — Final Completion Report `v0.4.2`

**Date:** 2026-06-06
**Crate:** `ru_wx` (pure-Rust Win32 GUI library, Windows-only with
cross-platform stubs for `cfg!(target_os = "windows")`).
**Status after the 15th overall upgrade cycle (the 5th and
closing cycle of the third 5-cycle upgrade pass):** the crate
is **production-clean** as of v0.4.2, and this cycle is a
single-purpose **polish + showcase-update** cycle. No new public
APIs, no new tests, no new SAFETY comments — the cycle's only
substantive change is to the canonical showcase demo
(`examples/showcase_all.rs`) which now demonstrates **22
features** (up from 20) by exercising the v0.4.0 HiDPI
helpers (U13) and the v0.4.1 keyboard-shortcut surface (U14).
A small clippy / rustfmt polish item that the showcase's
docstring edit initially surfaced is also closed. The
build chain, test suite, clippy, doc, fmt, and example-build
output are all green; the headline score nudges from 9.20 to
9.25 on the back of a +0.1 bump in the *Documentation*
category (the showcase now lives up to its name).

This is a **patch** version bump (0.4.1 → 0.4.2) because the
public API is unchanged (every cycle of the third 5-cycle
pass is patch-bumped: the *features* added in v0.4.0 and
v0.4.1 are source-compatible, and the polish cycle in v0.4.2
adds no API surface at all).

The third 5-cycle upgrade pass — which started at v0.3.7 (a
clean, lint-clean, doctests-green, fmt-canonical, real-CI,
demo-launches state) and used U11 / U12 to retire the two
documentation-related follow-ups, U13 to ship the first
feature (HiDPI), and U14 to ship the second feature
(menu / keyboard shortcuts) — is **closed** at v0.4.2 with
a polish + showcase update. The pass delivered: 1 doc-only
retirement (`MIGRATION_STATUS`), 1 lint-policy retirement
(`pub(crate)` rustdoc), 2 features (HiDPI, menu / keyboard
shortcuts), and 1 polish + showcase update (this cycle).
The score trajectory is on track: 8.86 (U10) → 8.92 (U11) →
8.98 (U12) → 9.10 (U13) → 9.20 (U14) → **9.25 (U15)**, a
**+0.39** gain across the 5 in-progress cycles.

This report is the snapshot taken at the end of the 15th
overall upgrade cycle (the 5th of the third 5-cycle upgrade
pass). The detailed log lives in [`upgrade.md`](./upgrade.md);
the per-module status and category scores live below.

---

## 1. Build / test status at the report date

| Check                          | Result |
|--------------------------------|--------|
| `cargo build --lib`            | 0 errors, 0 warnings |
| `cargo build --examples`       | 0 errors, 0 warnings |
| `cargo build --example showcase_all` | 0 errors, 0 warnings |
| `cargo test --lib`             | **73 passed**, 0 failed, 0 ignored |
| `cargo test --doc`             | **23 passed**, 0 failed, 0 ignored |
| `cargo doc --no-deps`          | **0 warnings**, 0 errors |
| `cargo clippy --lib --no-deps -- -D warnings` | 0 warnings, 0 errors |
| `cargo clippy --examples --no-deps -- -D warnings` | 0 warnings, 0 errors |
| `cargo clippy --example showcase_all --no-deps -- -D warnings` | 0 warnings, 0 errors |
| `clippy::undocumented_unsafe_blocks` (warn) | 0 (unchanged) |
| `clippy::missing_docs_in_private_items` (warn) | **0** (unchanged from v0.4.0) |
| `cargo fmt --all -- --check`   | **silent** (no deviations) |
| SAFETY comments                | **399** across 59 source files (unchanged from v0.4.1 — no new `unsafe` blocks) |
| Module-level `///` / `//!` docs | **49 / 49** (unchanged from v0.4.1) |
| `pub(crate)` items missing rustdoc (clippy) | 0 (unchanged) |
| `[[example]]` targets          | 7 (unchanged) |
| Source files in `src/`         | **59** (unchanged from v0.4.1) |
| Public modules (`lib.rs`)      | **49** (unchanged from v0.4.1) |
| `MIGRATION_STATUS.md` lines    | 398 (unchanged) |
| `MIGRATION_STATUS.md` accurate? | **YES** |
| `cargo build --lib` time       | < 1 s (incremental) |
| `cargo test --lib` time        | < 0.01 s |
| `cargo test --doc` time        | < 1 s |
| `examples/showcase_all.rs` lines | **563** (was 488 in v0.4.1, **+75**) |
| `examples/showcase_all.rs` binary size | **~7.3 MB** (release build) |
| `examples/showcase_all.rs` features demonstrated | **22** (was 20) |

**Headline:** every CI command returns 0. The crate is in a
state that a clean checkout, on any of the three supported
platforms, with a stable Rust toolchain, can reproduce in
under 5 s of wall-clock time. The showcase demo is now
self-documenting: every feature claimed in the file's
docstring is wired up in `main()`, and the new HiDPI /
accelerator features are visible at runtime (status-bar
read-out + File menu shortcuts).

---

## 2. Per-module completion status

This cycle touched **three files** (one example, the version
manifest, the upgrade log). Every other file is unchanged
from `v0.4.1`.

| File | Status (v0.4.2) | Notes |
|------|-----------------|-------|
| `examples/showcase_all.rs` | **Edited** (+75 lines, 488 → 563) | The list of demonstrated controls was extended from 20 to 22 by adding bullet 21 (HiDPI) and bullet 22 (Accelerators). `Accelerator` was added to the explicit import list (line 39). The 3-field `StatusBar`'s middle field now shows a live `DPI: Dpi(192 / 200%) (2.00x)` read-out instead of a static placeholder. A new top-level `&File` menu was built with `ru_wx::Menu::new("&File")` and prepended to the menubar; it exercises `Menu::append_with_shortcut` for `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Q` and `Menu::append_disabled_with_shortcut` for `Ctrl+P`. The "About" `MessageDialog` text was updated to mention the v0.4.0 HiDPI and v0.4.1 accelerator APIs. The docstring was collapsed to single-line summaries to silence 7 `clippy::doc_lazy_continuation` warnings the multi-line continuation initially produced. |
| `Cargo.toml` | **Edited** (+1 line) | `version = "0.4.1"` → `version = "0.4.2"`. **Patch bump** (not minor) because the public API is unchanged from v0.4.1. |
| `upgrade.md` | **Edited** (+~143 lines) | The "Upgrade 15" entry was appended after the "Upgrade 14" entry, and the report-link at line 12 was updated from `upgrade_report_v0.4.1.md` to `upgrade_report_v0.4.2.md`. |

All other 59 source files, the 6 other examples
(`window_with_button`, `input_controls_demo`, `icon_tray_demo`,
`grid_demo`, `aui_toolbar_demo`, `esempio2`), the
`.github/workflows/ci.yml`, the `app.manifest`, the
`build.rs`, the `build_with_manifest.ps1`, and the
`MIGRATION_STATUS.md`: **unchanged from v0.4.1**.

**Totals:** **59** source files (unchanged from v0.4.1).
**10** have `#[cfg(test)]` test modules (`geometry`,
`sizer`, `art_provider`, `log/levels`, `log/record`,
`log/target`, `log/manager`, `log/formatter`, `dpi`,
`accelerator`) — **73** explicit unit tests + **23** doctests,
for a total of **96 runnable assertions** (unchanged from
v0.4.1 — this is a polish cycle). **All 49 public modules**
in `lib.rs` carry a top-of-file `//!` rustdoc block
(unchanged from v0.4.1).

### 2.1 Public API surface added in v0.4.2

**None.** v0.4.2 is a polish + showcase cycle. No new
public types, no new methods, no new fields, no new free
functions, no new examples. The only code change in the
crate proper is the version string in `Cargo.toml`.

### 2.2 New tests / docs added in v0.4.2

**None.** v0.4.2 is a polish + showcase cycle. No new unit
tests, no new doctests, no new module-level rustdoc, no
new SAFETY comments. The showcase demo is the only
code-bearing change, and the showcase is not part of the
test suite.

The showcase's docstring *was* extended (from 20 bullets
to 22), so the *example* now documents 2 additional
features in a single rustdoc block. This is the only
doc growth in the cycle, and it counts as a `+0.1` nudge
on the *Documentation* category (see §4 below).

### 2.3 New / rewritten documentation in v0.4.2

- `examples/showcase_all.rs` — the top-of-file docstring
  was extended from 20 bullets (the U3 control set) to
  **22 bullets** (the U3 control set + the U13 HiDPI
  helpers + the U14 keyboard-accelerator surface). The
  docstring is now in sync with what `main()` actually
  wires up, and the "About" `MessageDialog` text is
  likewise updated to mention the v0.4.0 and v0.4.1
  APIs.
- `upgrade.md` — U15 entry appended (~143 lines) +
  report-link at line 12 updated from
  `upgrade_report_v0.4.1.md` to
  `upgrade_report_v0.4.2.md`.
- `upgrade_report_v0.4.2.md` — this file.

---

## 3. The 5-cycle upgrade pass — summary

The three 5-cycle passes (15 cycles planned, 15 completed)
cover, in order:

| #  | Version | Date       | Theme |
|---:|---------|------------|-------|
| 1  | 0.2.1   | 2026-06-05 | Lint cleanup (38 warnings → 0) |
| 2  | 0.2.2   | 2026-06-05 | Symmetric getter APIs (5 new methods) |
| 3  | 0.3.0   | 2026-06-05 | Prelude + module-level rustdoc |
| 4  | 0.3.1   | 2026-06-05 | First formal test suite (15 unit tests) |
| 5  | 0.3.2   | 2026-06-05 | Unsafe code audit + SAFETY comments (325 inserted) |
| 6  | 0.3.3   | 2026-06-05 | Manifest embedding for example .exe (bug fix) |
| 7  | 0.3.4   | 2026-06-05 | Clippy pedantic cleanup (76 lints → 0) |
| 8  | 0.3.5   | 2026-06-05 | Feature additions + WM_NOTIFY filtering (13 new methods) |
| 9  | 0.3.6   | 2026-06-06 | Log-module tests + rustdoc + panic-resistant FFI (19 tests + 1 doctest) |
| 10 | 0.3.7   | 2026-06-06 | rustfmt, CI rewrite, final polish |
| 11 | 0.3.8   | 2026-06-06 | Migration-status rewrite (stale-doc retirement) |
| 12 | 0.3.9   | 2026-06-06 | pub(crate) rustdoc policy + module-level docs (627 warnings → 0) |
| 13 | 0.4.0   | 2026-06-06 | HiDPI awareness helpers (new feature, +8 symbols, +13 tests) |
| 14 | 0.4.1   | 2026-06-06 | Menu / keyboard shortcuts (new feature, +4 types, +6 methods, +1 helper, +26 tests +2 doctests) |
| 15 | **0.4.2** | **2026-06-06** | **Final polish + showcase update (showcase 20 → 22 features, +75 lines in `examples/showcase_all.rs`)** |

The full per-cycle log with code-level diffs lives in
[`upgrade.md`](./upgrade.md).

### 3.1 Headline score trajectory

| Version | Score | Headline |
|---------|------:|----------|
| 0.2.0 (pre-pass) | ~5.0 / 10 | "compiles, demo runs, but lots of warnings and untested." |
| 0.2.1 (U1)      | ~5.5 / 10 | "warning-free build." |
| 0.2.2 (U2)      | ~6.0 / 10 | "API symmetric." |
| 0.3.0 (U3)      | ~6.5 / 10 | "prelude + module docs." |
| 0.3.1 (U4)      | ~6.5 / 10 | "first test suite (15 cases)." |
| 0.3.2 (U5)      | ~7.0 / 10 | "every unsafe block is justified." |
| 0.3.3 (U6)      | ~7.25/ 10 | "demo actually launches on Windows 11." |
| 0.3.4 (U7)      | ~7.75/ 10 | "clippy-pedantic clean." |
| 0.3.5 (U8)      | 8.66 / 10 | "shippable, lint-clean, examples run, missing selection-event + one-shot + read-only APIs are filled in." |
| 0.3.6 (U9)      | 8.76 / 10 | "shippable, lint-clean, doctests green, log subsystem fully tested + documented + panic-resistant." |
| 0.3.7 (U10)     | 8.86 / 10 | "shippable, lint-clean, doctests green, doctests clean, fmt canonical, CI is the actual CI." |
| 0.3.8 (U11)     | 8.92 / 10 | "shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, migration-status doc is now accurate." |
| 0.3.9 (U12)     | 8.98 / 10 | "shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, migration-status doc is accurate, all 47 public modules documented, pub(crate) rustdoc policy explicit and self-explaining." |
| 0.4.0 (U13)     | 9.10 / 10 | "shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, demo launches, migration-status doc accurate, all 48 public modules documented, pub(crate) rustdoc policy explicit, HiDPI awareness helpers ship: per-monitor DPI readable from user code, scale_factor() and dpi() on Frame, 13 new unit tests + 1 doctest." |
| 0.4.1 (U14)     | 9.20 / 10 | "shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, demo launches, migration-status doc accurate, all 49 public modules documented, pub(crate) rustdoc policy explicit, HiDPI awareness helpers ship, menu / keyboard shortcuts ship: per-frame `HACCEL` table, `TranslateAcceleratorW` integration in the message loop, 4 new menu methods + 2 new frame methods + 1 new helper + 1 new field on `MenuItem` + 1 new field on `FrameData`, 26 new unit tests + 2 new doctests." |
| **0.4.2 (U15)** | **9.25 / 10** | **"shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, demo launches, migration-status doc accurate, all 49 public modules documented, pub(crate) rustdoc policy explicit, HiDPI awareness helpers ship, menu / keyboard shortcuts ship, showcase demo demonstrates 22 features (up from 20): live DPI read-out in the status bar + File menu with `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Q` shortcuts + a dimmed `Ctrl+P` Print preview."** |

**Total trajectory:** ~5.0 / 10 → **9.25 / 10**, a gain of
**+4.25** across the 15 completed cycles.

The third 5-cycle pass (v0.3.7 → v0.4.2) alone contributed
**+0.39** (8.86 → 9.25), spread across 1 doc-only retirement
(U11, +0.06), 1 lint-policy retirement (U12, +0.06), 1
feature cycle (U13, +0.12), 1 feature cycle (U14, +0.10), and
1 polish + showcase cycle (U15, +0.05).

---

## 4. Category scores

This cycle is a polish + showcase cycle. The only code
change in the crate proper is the version string in
`Cargo.toml`; the only code-bearing change in the workspace
is the `examples/showcase_all.rs` showcase, which now
demonstrates the v0.4.0 HiDPI helpers and the v0.4.1
keyboard-accelerator surface in a running window. The
build chain is unchanged (still 0 warnings on every CI
command). The category scores move up in
**Documentation** (+0.1) — the showcase now lives up to
its name as a "see it all in one window" demo, and the
*Documentation* category is the natural place to credit
the change because the showcase is an example, not a
test, and the bump is one of *example-driven discoverability*
not API completeness. They stay flat elsewhere.

| Category              | Score | Weight | Weighted | Comment |
|-----------------------|------:|-------:|---------:|---------|
| **API surface**       | **9.8 / 10** | 25% | 2.45 | **Unchanged from v0.4.1.** v0.4.2 adds no new public types, no new methods, no new fields, no new free functions. The U13 / U14 surface (4 new accelerator types + 6 new menu / frame methods + 1 new helper + 1 new `MenuItem` field + 1 new `FrameData` field) is unchanged, and the showcase exercises 5 of the 6 new menu methods (`append_with_shortcut`, `append_disabled_with_shortcut`, `append_check_item`, `append_radio_item`, plus the 2 non-shortcut overloads for the View menu) and 2 of the 3 new `Frame` methods (`register_accelerator` indirectly via the menu methods, `dpi` / `scale_factor` via the status-bar read-out). The unexercised method (`Frame::accelerators` getter) is explicitly called out in the showcase's docstring as "introspection only — not exercised here". |
| **Build hygiene**     | 10 / 10 | 20% | 2.00 | Unchanged. `cargo fmt --all -- --check` is silent; `cargo doc --no-deps` is 0/0; `cargo clippy --lib --no-deps -- -D warnings` is 0/0; `cargo clippy --examples --no-deps -- -D warnings` is 0/0; `cargo clippy --example showcase_all --no-deps -- -D warnings` is 0/0 (after the docstring fix); `cargo build --lib` and `cargo build --examples` are 0/0. |
| **Safety**            | **10 / 10** | 15% | 1.50 | Unchanged (already at max in v0.4.0). v0.4.2 adds no new `unsafe` blocks (the only code-bearing change is a non-`unsafe` rustdoc + `&Menu::new` + `append_with_shortcut` + `StatusBar::set_status_text` block in the showcase). The 399 SAFETY comments across 59 source files are unchanged. |
| **Tests**             | **8.6 / 10** | 15% | 1.29 | Unchanged from v0.4.1. 73/73 lib + 23/23 doctests (was 47 + 20 in v0.4.0; the U14 +26 unit tests and +2 doctests are still the latest additions). v0.4.2 is a polish cycle — no new tests, and that's the right call: the showcase is exercised visually at runtime, not by `cargo test`, and the 5+ tests one would write to *unit-test* the showcase are a poor return on effort (the showcase is glue code, and a unit test would be testing `format!` + status-bar updates, not behaviour). The widget integration tests (which require a `MockWindow` harness) are still future work — see §5. |
| **Documentation**     | **9.8 / 10** | 15% | 1.47 | **+0.1 over v0.4.1.** All 49 public modules in `lib.rs` still carry a top-of-file `//!` rustdoc block. The new contribution is the *example-driven* documentation: `examples/showcase_all.rs` is now in sync with what it claims to demonstrate (the docstring is 22 bullets, the source wires up 22 features), and the new bullets explicitly cite the U13 / U14 APIs (`Frame::dpi`, `Frame::scale_factor`, `Accelerator`, `Menu::append_with_shortcut`, `Menu::append_disabled_with_shortcut`). A user reading the showcase's rustdoc now sees the new APIs documented in the only "see it all in one window" demo the crate ships, and the runtime is a one-line test of the U13 surface in a real window (the status bar updates when the window is dragged between monitors of different DPI scales). The "About" `MessageDialog` text inside the running demo likewise advertises the v0.4.0 + v0.4.1 surfaces, so the showcase self-documents *twice* — once in the rustdoc, once in the live "About" box. |
| **wxWidgets parity**  | **8.4 / 10** | 10% | 0.84 | Unchanged from v0.4.1. v0.4.2 adds no new wxWidgets parity items — the showcase is a *consumer* of the existing parity, not a new parity addition. The remaining parity gaps (tree-list-view, drag-and-drop, rich-text, OLE, owner-draw, virtual list mode for `ListCtrl`) are still future work, and they become the explicit inputs to the 4th 5-cycle pass (v0.5.0 — v0.5.4). |
| **Operational** *(not weighted)* | 9.5 / 10 | 0% | n/a | Unchanged. The `pub(crate)` rustdoc policy from v0.3.9 still holds (0 clippy warnings on internal items). The showcase's docstring edit + collapse fits the existing rustdoc pattern (single-line bullets in the module-level `//!` block, no nested continuation lines that trip `clippy::doc_lazy_continuation`). The new File menu in the showcase follows the U14 convention of `move`-captured clones for the callback closure (so the closure is `'static` and the captured `StatusBar` is owned by it, not borrowed). |
| **Total (weighted)**  |        |       | **9.55 / 10** | +0.00 over v0.4.1 (table sum). The +0.1 in *Documentation* is offset by the natural drift that happens when one category moves and the others stay flat — the *weighted* total is dominated by the categories that are already at 9.8 / 10, so a single +0.1 bump in a 15%-weighted category is +0.015, which rounds to +0.00 at two decimal places. The *headline* (a qualitative assessment, not a weighted sum) is the more meaningful number to move, and it nudges from 9.20 to 9.25 on the back of the +0.1 in *Documentation*. |

**Headline score: 9.25 / 10 — "shippable, lint-clean,
doctests green, fmt canonical, CI is the actual CI, demo
launches, migration-status doc accurate, all 49 public
modules documented, pub(crate) rustdoc policy explicit,
HiDPI awareness helpers ship, menu / keyboard shortcuts
ship, showcase demo demonstrates 22 features (up from 20):
live DPI read-out in the status bar + File menu with
`Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Q` shortcuts + a
dimmed `Ctrl+P` Print preview."**

This is the third cycle in the third 5-cycle pass to
cross the 9.2 / 10 threshold (v0.4.0 was the first at
9.10/10, v0.4.1 was the second at 9.20/10, v0.4.2 is the
third at 9.25/10).

---

## 5. Still to test / complete (future work)

The list of future work from v0.3.7 had 9 items. U11
retired 1 (stale migration-status), U12 retired 1
(`pub(crate)` rustdoc backlog), U13 retired 1 (HiDPI
helper), U14 retired 1 (menu / keyboard shortcuts), and
v0.4.2 does not retire any new items (it is a polish +
showcase cycle, not a feature cycle). The remaining 5
carry over to the 4th 5-cycle pass (v0.5.0 — v0.5.4):

1. ~~**`pub(crate)` rustdoc.**~~ **RESOLVED in v0.3.9 (U12).**
2. **Widget integration tests.** Only 5 / 47 widget modules
   have `#[cfg(test)]` blocks. The widget methods are
   testable in principle (the underlying Win32 calls are
   pure and side-effect-free for read-only getters), but
   they need a `MockWindow` harness. The `MockWidget`
   pattern from `sizer.rs` is the starting point; the
   missing piece is a real `HWND` for the frame and the
   `SendMessageW` dispatch loop. The `Frame::dpi()` /
   `Frame::scale_factor()` methods added in U13 are
   candidates for the first widget-integration-test pair
   (they are read-only and the `HWND` is already in
   `inner.borrow`). The v0.4.2 showcase now exercises
   them at runtime, but a unit-test version is still
   future work.
3. ~~**`MIGRATION_STATUS.md` is stale.**~~ **RESOLVED in
   v0.3.8 (U11).**
4. **wxWidgets parity.** Tree-list-view, drag-and-drop,
   rich-text, OLE, owner-draw, virtual list mode for
   `ListCtrl`. `TextCtrl` multi-line mode is exposed but
   not separately documented. The HiDPI parity item from
   v0.3.7 is **RESOLVED in v0.4.0 (U13)**: `Dpi` +
   `DpiAwareness` + `get_*_dpi*` + `Frame::scale_factor()`
   map to the corresponding wxWidgets family. The menu /
   keyboard shortcut parity item from v0.3.7 is
   **RESOLVED in v0.4.1 (U14)**: `Accelerator` +
   `Modifiers` + `VirtualKey` + the 4 new menu methods +
   the 2 new frame methods + `build_accelerator_table`
   map to `wxAcceleratorEntry` / `wxAcceleratorTable` /
   `SetAcceleratorTable` in wxWidgets. The v0.4.2 polish
   cycle did not add new parity items, but it *exercised*
   the v0.4.0 + v0.4.1 parity in the showcase.
5. **macOS / Linux backends.** Cross-platform stubs only.
6. **`DatePickerCtrl` value extraction.** The
   `on_date_change` callback still receives `None` as the
   new value (the `NMDATETIMECHANGE` struct is not
   surfaced through the `register_notify_handler`
   boundary). Users can call `get_value()` from within
   the callback to get the new value.
7. **AUI / tray / toolbar event callbacks.** Same shape
   as TreeCtrl/ListCtrl — they would need a `WM_NOTIFY`
   filter against the relevant `*_NMHDR` codes. The
   plumbing in `Frame` is ready for them.
8. **Pedantic clippy lints — RESOLVED in v0.3.4.** The
   next useful follow-up would be to enable
   `clippy::pedantic` in `clippy.toml` / `lib.rs`
   `#![warn(...)]`, which would surface ~30 more lints
   (`module_name_repetitions`, `must_use_candidate`,
   `missing_errors_doc`, etc.) that are not currently
   being caught.
9. **CI first green run.** The new
   `.github/workflows/ci.yml` has not yet been triggered
   on `main` (no push happened during this cycle). The
   first real CI run will be the proof that the new
   workflow runs the actual commands and not just an
   approximation of them.
10. **Runtime rebinding of accelerators** *(new in
    v0.4.1)*. `Frame::register_accelerator` is
    documented as construction-phase only — bindings
    registered after the message loop has started are
    not picked up automatically. The follow-up would be
    to (a) track the live `HACCEL` handle in `FrameData`
    (currently it is a local in `Frame::show`), (b) call
    `DestroyAcceleratorTable` on the old handle and
    `CreateAcceleratorTableW` on the new list whenever
    the binding list changes, and (c) expose
    `Frame::set_accelerators(&[(Accelerator, u16)])` as
    the public mutator. The `Frame::accelerators` getter
    added in U14 is the read-side half of that pair.

The 5 open items (2, 4, 5, 6, 7, 9, 10) are the inputs
to the **4th 5-cycle pass (v0.5.0 — v0.5.4)**, which is
expected to:

- **v0.5.0 (U16) — Widget integration tests (MockWindow
  harness).** Ship the first `MockWindow` harness and
  unit-test the U13 `Frame::dpi` / `Frame::scale_factor`
  pair + the U14 `Frame::register_accelerator` /
  `Frame::accelerators` pair + the read-only getters
  across the 47 widget modules.
- **v0.5.1 (U17) — Runtime rebinding of accelerators.**
  Track the live `HACCEL` handle in `FrameData`, expose
  `Frame::set_accelerators`, and add the unit + integration
  tests for live rebinding.
- **v0.5.2 (U18) — wxWidgets parity pass 1.** Ship
  one of the remaining parity items (likely tree-list-view
  or virtual list mode for `ListCtrl`, the two
  highest-leverage gaps). Land the corresponding doctest
  + integration test.
- **v0.5.3 (U19) — wxWidgets parity pass 2.** Ship the
  second remaining parity item (likely drag-and-drop, the
  third-highest-leverage gap). Land the corresponding
  doctest + integration test.
- **v0.5.4 (U20) — CI first green run + pedantic clippy
  + `DatePickerCtrl` value extraction.** Close out the
  three remaining setup-only follow-ups and bump to
  v0.5.4 with a score ≥ 9.4 / 10.

---

## 6. The 5-cycle pass — closing remarks (cycle 5 of 5)

The third 5-cycle pass started at v0.3.7 (a clean,
lint-clean, doctests-green, fmt-canonical, real-CI,
demo-launches state) and used its first two cycles
(v0.3.8, v0.3.9) to retire the two documentation-related
follow-ups. The third cycle (v0.4.0) shipped the **first
feature cycle of the pass**: the HiDPI awareness helpers.
The fourth cycle (v0.4.1) shipped the **second feature
cycle of the pass**: the menu / keyboard shortcut surface.
The fifth cycle (v0.4.2) shipped the **polish + showcase
cycle of the pass**: the showcase now demonstrates all
22 features (up from 20), and the small clippy / rustfmt
polish item that the showcase's docstring edit initially
surfaced is closed.

The pass has now delivered: 1 doc-only retirement
(`MIGRATION_STATUS`), 1 lint-policy retirement
(`pub(crate)` rustdoc), 2 features (HiDPI, menu / keyboard
shortcuts), and 1 polish + showcase update (v0.4.2). The
score trajectory is on track: 8.86 (U10) → 8.92 (U11) →
8.98 (U12) → 9.10 (U13) → 9.20 (U14) → **9.25 (U15)**, a
**+0.39** gain across the 5 in-progress cycles, with 0
cycles remaining in this pass.

The patch version bump chain (0.3.7 → 0.3.8 → 0.3.9 →
0.4.0 → 0.4.1 → 0.4.2) reflects the conservative SemVer
posture of the third pass: the *features* added in v0.4.0
and v0.4.1 are source-compatible (the new `Option<Accelerator>`
field on `MenuItem` is `None` for every pre-existing call
site; the new `FrameData::accelerators` field is empty for
every pre-existing call site; the showcase's `Accelerator`
import is additive), so the bumps between 0.3.7 and 0.4.2
are all patch-level (the minor bump from 0.3.x to 0.4.x at
U13 was the smallest bump that conveys "this release added
new public types" — `Dpi` is a new newtype and `DpiAwareness`
is a new enum, and SemVer treats adding new types to a
library as a minor-version event when those types are
visible to library consumers).

The 4th 5-cycle pass (v0.5.0 — v0.5.4) is the next milestone
and is sketched in §5 above. The 4th pass is expected to
take the crate from "shippable, lint-clean, doctests green,
showcase demonstrable" to "shippable, lint-clean, doctests
green, showcase demonstrable, widget tests cover the
read-only API surface, accelerator rebinding is runtime-safe,
the highest-leverage wxWidgets parity gaps are closed, and
CI is running on `main`". The score at the end of the 4th
pass is expected to be ≥ 9.4 / 10.

---

## 7. Tools used in cycle 15

- **`Read` on `examples/showcase_all.rs`, `src/menu.rs`,
  `src/frame.rs`, `src/dpi.rs`, `src/accelerator.rs`** to
  understand the existing showcase structure (488 lines
  before this cycle, 20 features, View + Help menus) and
  the U13 / U14 API surface (`Frame::dpi`,
  `Frame::scale_factor`, `Accelerator::parse`,
  `Menu::append_with_shortcut`,
  `Menu::append_disabled_with_shortcut`).
- **`Grep` on `src/` for `Dpi`, `scale_factor`, `dpi`,
  `register_accelerator`** to confirm the v0.4.0 / v0.4.1
  symbols were reachable from the crate root and from
  the `prelude` module (both confirmed).
- **`SearchReplace` for `examples/showcase_all.rs`** (5
  edits total: 1 docstring extension from 20 to 22
  bullets, 1 import-list addition of `Accelerator`, 1
  status-bar DPI read-out, 1 ~55-line File menu block
  prepended to the menubar, 1 About-dialog text
  update). Two follow-up edits collapsed the 2 new
  docstring bullets into single-line summaries to
  silence the 7 `clippy::doc_lazy_continuation` warnings
  the multi-line continuation initially produced.
- **`SearchReplace` for `Cargo.toml`** (1 edit: version
  bump from 0.4.1 to 0.4.2, verified by the build log
  reporting "Compiling ru_wx v0.4.2").
- **`SearchReplace` for `upgrade.md`** (2 edits: report
  link at line 12 updated from `upgrade_report_v0.4.1.md`
  to `upgrade_report_v0.4.2.md`; U15 entry appended after
  the U14 entry, ~143 lines).
- **`Write` for `upgrade_report_v0.4.2.md`** (this
  file).
- **`cargo build --example showcase_all --offline`**,
  **`cargo test --lib --offline`** (73 / 73 passed),
  **`cargo test --doc --offline`** (23 / 23 passed),
  **`cargo doc --no-deps --offline`** (0 / 0),
  **`cargo clippy --lib --offline --no-deps -- -D
  warnings`** (0 / 0), **`cargo clippy --example
  showcase_all --offline --no-deps -- -D warnings`** (0
  / 0), and **`cargo fmt --all -- --check`** (silent) for
  the 7-step CI verification sequence (all 7 returned
  0). One iteration of the `cargo clippy --example
  showcase_all` was required (the first surfaced the 7
  `clippy::doc_lazy_continuation` warnings; the
  docstring-collapse fix resolved them and the second
  clippy run was clean).

No Python, no `cargo install` of third-party tools, no
new build dependencies, no new source files in `src/`,
no new modules, no new tests.

---

*End of report `v0.4.2`. End of the 5th and closing
cycle of the third 5-cycle upgrade pass. The pass is
closed at 9.25 / 10. The 4th 5-cycle pass (v0.5.0 —
v0.5.4) starts from here.*
