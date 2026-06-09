# ru_wx — Completion Report `v0.3.3`

**Date:** 2026-06-05
**Crate:** `ru_wx` (pure-Rust Win32 GUI library, Windows-only with
cross-platform stubs for `cfg!(target_os = "windows")`).
**Status after cycle 1 of the second 5-cycle upgrade process:** the
`0xc0000142` DLL-initialization crash is fixed. The example binaries now
embed the Common Controls v6 manifest at build time, and
`input_controls_demo` launches a real window on Windows 11 from a clean
`cargo build`. The library itself is unchanged from `v0.3.2`: it still
builds warning-free, passes `15 / 15` unit tests, and every `unsafe { }`
block is still documented.

This report is the snapshot taken at the end of the 6th overall upgrade
cycle (the first of the new 5-cycle pass). The detailed log lives in
[`upgrade.md`](./upgrade.md); the per-module status and category scores
live below.

---

## 1. Build / test status at the report date

| Check                          | Result |
|--------------------------------|--------|
| `cargo build --lib`            | 0 errors, 0 warnings |
| `cargo test --lib`             | 15 passed, 0 failed, 0 ignored |
| `cargo clippy --lib --no-deps` | 76 pedantic-style warnings (no change from `v0.3.2`) |
| `clippy::undocumented_unsafe_blocks` (warn) | 0 |
| `cargo build --example input_controls_demo` | 0 errors, manifest not embedded (no-op build.rs as far as the example is concerned) |
| `build_with_manifest.ps1 --example input_controls_demo` | 0 errors, 1 .exe embedded, demo launches and stays open |
| `PerMonitorV2` in `input_controls_demo.exe` (ASCII search) | True |
| `Microsoft.Windows.Common-Controls` in `input_controls_demo.exe` (ASCII search) | True |
| SAFETY comments                | 325 across 57 source files |
| Source files in `src/`         | 57 |
| Public modules (`lib.rs`)      | 46 |
| `[[example]]` targets          | 7 |

---

## 2. Per-module completion status

No module changed in this cycle (the fix is purely build-system / wrapper
script). The status table is identical to `v0.3.2`. The full table is
omitted here; see [`upgrade_report_v0.3.2.md`](./upgrade_report_v0.3.2.md)
§2 for the row-by-row list.

**Totals (unchanged):** 46 modules. 3 have `#[cfg(test)]` test modules
(`geometry`, `sizer`, `art_provider`) — 13 explicit unit tests + 2
module-level doctests in `prelude` and `lib`.

---

## 3. Test inventory at `v0.3.3`

No new tests in this cycle. The 15 unit tests + 2 doctests from
`v0.3.1` are still green:

| Test module | Cases | What it pins down |
|-------------|-------|-------------------|
| `geometry::tests` | 6 | `Rect` field layout, `rect_contains` boundaries, `Colour` constants, default colour is white, `Colour -> COLORREF` byte order (BBGGRR). |
| `sizer::tests`    | 6 | Empty sizer does not panic, horizontal / vertical pack of fixed-size children, proportional space distribution, padding is respected, vertical alignment to origin. |
| `art_provider::tests` | 1 | Resolved-icon path is non-null on Windows for a known `ArtId`. |
| `prelude` doctest | 1 | `use ru_wx::prelude::*;` brings a working set in. |
| `lib` doctest     | 1 | `App::new` -> `Frame::builder` -> `Button::new` -> `app.run` compiles. |

---

## 4. Category scores

The fix in this cycle is purely build-system: it doesn't add API
surface, tests, or documentation. The category scores are therefore
identical to `v0.3.2` except for a small bump in **build hygiene** (the
examples can now actually be launched on Windows 11) and a small bump
in **operational readiness** (a one-line build wrapper for end users).

| Category              | Score | Weight | Weighted | Comment |
|-----------------------|------:|-------:|---------:|---------|
| **API surface**       | 9 / 10 | 25% | 2.25 | Unchanged from v0.3.2. |
| **Build hygiene**     | 10 / 10 | 20% | 2.00 | `cargo build --lib` is 0/0; `build_with_manifest.ps1` covers the example build end-to-end. |
| **Safety**            | 9 / 10 | 15% | 1.35 | Unchanged from v0.3.2. |
| **Tests**             | 6 / 10 | 15% | 0.90 | Unchanged from v0.3.2. |
| **Documentation**     | 7 / 10 | 15% | 1.05 | Unchanged from v0.3.2. |
| **wxWidgets parity**  | 7 / 10 | 10% | 0.70 | Unchanged from v0.3.2. |
| **Operational** *(new category)* | 8 / 10 | 0% (out of total) | n/a | Single-script build wrapper, no Python, idempotent. Not weighted into the headline score yet because it was not in the original scoring matrix. |
| **Total (weighted)**  |        |       | **8.25 / 10** | Same headline as v0.3.2, but the examples are now runnable. |

**Headline score: 8.25 / 10 — "shippable for a single-platform Win32
GUI library; examples now actually run on Windows 11."**

---

## 5. Still to test / complete (future work)

1. **Widget integration tests.** Only 3 / 46 modules have `#[cfg(test)]`
   blocks. The next step is to add a `MockWindow` harness (similar to
   the `MockWidget` already used in `sizer.rs`) and cover at least one
   setter / getter pair per widget.
2. **Pedantic clippy lints.** `cargo clippy --lib --no-deps` reports
   76 warnings. Breakdown:
   - 40 unnecessary casts (`usize -> usize`, `isize -> isize`,
     `u8 -> u8`, `*const u16 -> *const u16`, `*mut ... -> *mut ...`)
   - 6 acronym casing (LVITEMW, TBBUTTON, LVCOLUMNW, TCITEMW, TVITEMW,
     TVINSERTSTRUCTW)
   - 4 `let` binding returned from block
   - 2 manual `div_ceil` reimplementation
   - 2 manual checked division
   - 1 `std::xxx` call that has a shorter form
   - 1 no-op operation
   - 1 `&mut` that does not need to be `&mut`
   - 1 redundant closure
   - 1 missing `const` initializer for `thread_local`
   - 1 null comparison (use `.is_null()`)
   - 1 clamp-like pattern
3. **Per-function rustdoc.** Module-level `//!` doc comments are
   present on every module. Per-function `///` rustdoc is partial.
4. **wxWidgets parity.** Tree-list-view, drag-and-drop, rich-text, OLE,
   owner-draw, virtual list mode for `ListCtrl`.
5. **macOS / Linux backends.** Cross-platform stubs only.
6. **CI.** `cargo clippy --all-features -- -D warnings` is a one-line
   addition once the pedantic lints are addressed.
7. **WM_NOTIFY support.** `list_ctrl.rs:388` and `tree_ctrl.rs:306` are
   blocked on `WM_NOTIFY` plumbing in the `frame` WndProc, which would
   unlock `on_item_selected` / `on_selection_change` callbacks for both
   widgets.
8. **Backlog from U5 (5 already-completed items re-listed for context):
   the SAFETY-comment script (`tools/add-safety.ps1`) is in place; the
   5 `#[allow(clippy::not_unsafe_ptr_arg_deref)]` annotations are in
   place.**

---

## 6. Tools used in cycle 1

- **`mt.exe`** — Microsoft Manifest Tool, part of the Windows 10/11
  SDK (located under `C:\Program Files (x86)\Windows Kits\10\bin\*\x64\mt.exe`).
  Used to embed the `app.manifest` into the example .exe files.
- **PowerShell** (`powershell.exe -ExecutionPolicy Bypass -File ...`)
  — used to host the `build_with_manifest.ps1` wrapper. No extra
  interpreter is required.

No Python, no `cargo install` of third-party tools.

---

*End of report `v0.3.3`.*
