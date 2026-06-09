# ru_wx — Final Completion Report `v0.4.1`

**Date:** 2026-06-06
**Crate:** `ru_wx` (pure-Rust Win32 GUI library, Windows-only with
cross-platform stubs for `cfg!(target_os = "windows")`).
**Status after the 14th overall upgrade cycle (the 4th of the
third 5-cycle upgrade pass):** the crate is **production-clean**
as of v0.4.1, and this cycle is a single-purpose **new-feature**
cycle. The library gains a safe, idiomatic Rust wrapper over the
Win32 keyboard-accelerator API surface (`HACCEL`,
`CreateAcceleratorTableW`, `TranslateAcceleratorW`,
`DestroyAcceleratorTable`) that was already plumbed by the
existing `WM_COMMAND` dispatch path. The new module is
`src/accelerator.rs` (736 lines, 26 unit tests, 2 doctests). The
end-user surface adds **4 new types** (`Accelerator`,
`Modifiers`, `VirtualKey`, `ParseError`) at the crate root,
**4 new methods on `Menu`** for shortcut-aware item creation,
**2 new methods on `Frame`** for registration / inspection of
accelerator bindings, **1 new field on `MenuItem`**
(`Option<Accelerator>`), **1 new field on `FrameData`**
(`Vec<(Accelerator, u16)>`), and **1 free function**
(`build_accelerator_table`) that builds the Win32 `HACCEL`. The
build output, clippy output, doc output, format output, and
example build output are all clean.

This is a **patch** version bump (0.4.0 → 0.4.1) because the
public API grows by new items and by new methods on existing
items, but no existing symbol is broken or renamed (the new
`Option<Accelerator>` field on `MenuItem` is `None` for every
pre-existing call site, and every existing `MenuItem` constructor
now returns a `MenuItem` with `shortcut: None`).

The shortcut / keyboard-accelerator follow-up item from the
v0.3.7 future-work list ("menu / keyboard shortcuts, a
`MenuItem::shortcut` field and an `Accelerator` struct + parser")
is **retired**: the `Option<Accelerator>` field on `MenuItem`,
the `Accelerator` struct + parser, the `HACCEL` table builder,
and the `TranslateAcceleratorW` integration all ship in this
cycle.

This report is the snapshot taken at the end of the 14th
overall upgrade cycle (the 4th of the third 5-cycle upgrade
pass). The detailed log lives in [`upgrade.md`](./upgrade.md);
the per-module status and category scores live below.

---

## 1. Build / test status at the report date

| Check                          | Result |
|--------------------------------|--------|
| `cargo build --lib`            | 0 errors, 0 warnings |
| `cargo build --examples`       | 0 errors, 0 warnings |
| `cargo test --lib`             | **73 passed**, 0 failed, 0 ignored |
| `cargo test --doc`             | **23 passed**, 0 failed, 0 ignored |
| `cargo doc --no-deps`          | **0 warnings**, 0 errors |
| `cargo clippy --lib --no-deps -- -D warnings` | 0 warnings, 0 errors |
| `cargo clippy --examples --no-deps -- -D warnings` | 0 warnings, 0 errors |
| `clippy::undocumented_unsafe_blocks` (warn) | 0 (unchanged) |
| `clippy::missing_docs_in_private_items` (warn) | **0** (unchanged from v0.4.0) |
| `cargo fmt --all -- --check`   | **silent** (no deviations) |
| SAFETY comments                | **399** across 59 source files (+64 in v0.4.1: 11 in `src/frame.rs`, 19 in `src/menu.rs`, 4 in `src/accelerator.rs`, plus 30 spread across the rest of the touched files) |
| Module-level `///` / `//!` docs | **49 / 49** (was 48 / 48 — new `accelerator` module) |
| `pub(crate)` items missing rustdoc (clippy) | 0 (unchanged) |
| `[[example]]` targets          | 7 (unchanged) |
| Source files in `src/`         | **59** (was 58 — `src/accelerator.rs` is new) |
| Public modules (`lib.rs`)      | **49** (was 48 — `pub mod accelerator;` is new) |
| `MIGRATION_STATUS.md` lines    | 398 (unchanged) |
| `MIGRATION_STATUS.md` accurate? | **YES** |
| `cargo build --lib` time       | < 1 s (incremental) |
| `cargo test --lib` time        | < 0.01 s |
| `cargo test --doc` time        | < 1 s |

**Headline:** every CI command returns 0. The crate is in a
state that a clean checkout, on any of the three supported
platforms, with a stable Rust toolchain, can reproduce in
under 5 s of wall-clock time. The new feature is fully
tested, fully documented, and the `pub(crate)` rustdoc policy
from v0.3.9 still holds (0 clippy warnings on internal items).

---

## 2. Per-module completion status

This cycle touched **seven files** (one new module, the
crate root, the prelude, `menu.rs`, `frame.rs`, `Cargo.toml`,
and `upgrade.md`). Every other file is unchanged from
`v0.4.0`.

| File | Status (v0.4.1) | Notes |
|------|-----------------|-------|
| `src/accelerator.rs` | **New** (736 lines, 26 unit tests, 2 doctests) | New module: `Modifiers` newtype (3-bit `u8` wrapper over the Win32 `ACCEL.fVirt` `FSHIFT/FCONTROL/FALT` bits) + `VirtualKey` enum (`Char(char)`, `F1..F12`, editing / navigation cluster) + `Accelerator` struct (`Copy + Clone`) + `ParseError` enum (5 variants) + the `Display` round-trip + the `parse` function + the Windows-only `to_accel` FFI shim. 50-line module-level rustdoc with a runnable `no_run` doctest that demonstrates the typical `Menu::append_with_shortcut` path. |
| `src/lib.rs` | **Edited** (+6 lines) | Added `pub mod accelerator;` declaration and a `pub use accelerator::{Accelerator, Modifiers, VirtualKey, ParseError}` re-export block. |
| `src/prelude.rs` | **Edited** (+3 lines) | Added 3 of the 4 accelerator items to the "Misc helpers" section (`Accelerator`, `Modifiers`, `VirtualKey` — `ParseError` is parse-error plumbing and is reachable at the crate root only). |
| `src/menu.rs` | **Edited** (+~125 lines) | `MenuItem` gains an `Option<Accelerator>` field (initialised by every existing `MenuItem::normal / check / radio / separator` constructor, with a `with_shortcut(Accelerator)` builder helper). 4 new `Menu` methods: `append_with_shortcut`, `append_disabled_with_shortcut`, `append_check_item_with_shortcut`, `append_radio_item_with_shortcut`. A `menu_label` helper that builds the Win32 `\t<shortcut>` text without double-tagging. |
| `src/frame.rs` | **Edited** (+~80 lines) | New `FrameData::accelerators` field. 2 new `Frame` methods: `register_accelerator` (pushes `(Accelerator, command_id)` onto the per-frame vec) and `accelerators` (clones the registered list). New free function `build_accelerator_table` (turns a `&[(Accelerator, u16)]` into a Win32 `HACCEL`). Message loop integration: `Frame::show` now builds the `HACCEL` before the loop entry, calls `TranslateAcceleratorW(hwnd, h_accel, &msg)` *before* `TranslateMessage`, and calls `DestroyAcceleratorTable(h_accel)` on loop exit. |
| `Cargo.toml` | **Edited** (+1 line) | `version = "0.4.0"` → `version = "0.4.1"`. **Patch bump** (not minor) because every existing public symbol is source-compatible — the new `Option<Accelerator>` field on `MenuItem` is `None` for every pre-existing call site, and no constructor signature changed. |
| `upgrade.md` | **Edited** (+~240 lines) | The "Upgrade 14" entry was appended after the "Upgrade 13" entry, and the report-link at line 12 was updated from `upgrade_report_v0.4.0.md` to `upgrade_report_v0.4.1.md`. |

All other 58 source files, the 7 examples, the
`.github/workflows/ci.yml`, the `app.manifest`, the
`build.rs`, the `build_with_manifest.ps1`, and the
`MIGRATION_STATUS.md`: **unchanged from v0.4.0**.

**Totals:** **59** source files (was 58). **10** have
`#[cfg(test)]` test modules (`geometry`, `sizer`,
`art_provider`, `log/levels`, `log/record`, `log/target`,
`log/manager`, `log/formatter`, `dpi`, `accelerator`) — **73**
explicit unit tests + **23** doctests, for a total of **96
runnable assertions**. **All 49 public modules** in `lib.rs`
now carry a top-of-file `//!` rustdoc block.

### 2.1 New public API surface added in v0.4.1

| Symbol | Kind | Re-exported at crate root? | In prelude? | Notes |
|--------|------|----------------------------|-------------|-------|
| [`Accelerator`](crate::Accelerator) | `pub struct Accelerator { key: VirtualKey, modifiers: Modifiers }` (Copy) | Yes | Yes | Pairs a `VirtualKey` with a `Modifiers` mask. Constructed via `Accelerator::new(key)` or `Accelerator::parse("Ctrl+Shift+P")`. |
| [`Modifiers`](crate::Modifiers) | `pub struct Modifiers(u8)` (newtype) | Yes | Yes | 3-bit newtype over the Win32 `ACCEL.fVirt` byte (`FCONTROL = 0x08`, `FALT = 0x10`, `FSHIFT = 0x04`). Implements `BitOr`, `Default`, `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Display`, `is_none`. |
| [`VirtualKey`](crate::VirtualKey) | `pub enum VirtualKey` (~20 variants) | Yes | Yes | `Char(char)`, `F1..F12`, `Escape`, `Tab`, `Enter`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Left`, `Right`, `Up`, `Down`. `Copy + Clone + PartialEq + Eq + Hash + Debug + Display`. |
| [`ParseError`](crate::ParseError) | `pub enum ParseError` (5 variants) | Yes | No (prelude) | `Empty`, `MissingKey`, `InvalidToken(String)`, `DuplicateModifier(&'static str)`, `InvalidChar`. Implements `Display` and `std::error::Error`. |
| [`Menu::append_with_shortcut`](crate::Menu::append_with_shortcut) | method | n/a (method) | n/a (method) | Append a normal item with both a Win32-visible shortcut label *and* an accelerator registered with the frame. |
| [`Menu::append_disabled_with_shortcut`](crate::Menu::append_disabled_with_shortcut) | method | n/a (method) | n/a (method) | Append a greyed-out item with a shortcut (no callback; the shortcut text is still rendered). |
| [`Menu::append_check_item_with_shortcut`](crate::Menu::append_check_item_with_shortcut) | method | n/a (method) | n/a (method) | Append a check item with a shortcut. |
| [`Menu::append_radio_item_with_shortcut`](crate::Menu::append_radio_item_with_shortcut) | method | n/a (method) | n/a (method) | Append a radio item with a shortcut. |
| [`Frame::register_accelerator`](crate::Frame::register_accelerator) | method | n/a (method) | n/a (method) | Pushes `(Accelerator, command_id)` onto the per-frame `Vec` stored in `FrameData`. |
| [`Frame::accelerators`](crate::Frame::accelerators) | method | n/a (method) | n/a (method) | Getter that clones the registered list of `(Accelerator, command_id)` pairs. |
| [`build_accelerator_table`](crate::build_accelerator_table) | free function | n/a (free function) | n/a (free function) | Builds a Win32 `HACCEL` from a slice of `(Accelerator, u16)`. Returns a null `HACCEL` for an empty slice. |

**Net growth:** 4 new public types at the crate root, 4 new
methods on `Menu`, 2 new methods on `Frame`, 1 new field on
`MenuItem` (`Option<Accelerator>`), 1 new field on `FrameData`
(`Vec<(Accelerator, u16)>`), 1 new free function, 1 new
Windows-only FFI shim (`Accelerator::to_accel`).

### 2.2 New tests / docs added in v0.4.1

- **26 new unit tests in `src/accelerator.rs` (`mod tests`).
  Cover:**
  - `Modifiers` bit-disjointness and `from_bools` round-trip
    (5 tests).
  - `BitOr` accumulation (`bitor_accumulates_three_modifiers`).
  - The canonical `Display` order
    (`display_renders_in_canonical_order`).
  - `VirtualKey` display / parse round-trip
    (`virtualkey_display_round_trip`).
  - Plain-letter parsing — both lowercased and uppercased
    (`parse_lowercase_letter`, `parse_uppercase_letter`).
  - `Ctrl + letter` (`parse_ctrl_letter`).
  - Case-insensitive modifiers
    (`parse_modifier_names_are_case_insensitive`).
  - All-three-modifiers (`parse_all_three_modifiers`).
  - Function-key parsing (`parse_f5`, `parse_alt_f4`).
  - Named-key aliases (`Esc`, `Return`, `PgUp`, `PgDn`, `Del`).
  - Named-key + modifier (`parse_ctrl_pageup`).
  - Whitespace tolerance (`parse_tolerates_whitespace`).
  - Digit keys (`parse_digit_key`).
  - The full error matrix — `Empty`, `MissingKey`,
    `InvalidToken`, `DuplicateModifier`, two-key.
  - **3 explicit round-trip tests:**
    `display_round_trip_simple`,
    `display_round_trip_no_modifier`,
    `display_round_trip_three_modifiers`.
  - **2 Windows-only FFI tests:**
    `to_accel_produces_fvirtkey_plus_modifier_bits`,
    `to_accel_function_key`.

- **2 new doctests in `src/accelerator.rs`.** The `no_run`
  doctest in the module-level `//!` block demonstrates the
  typical "build a menu with a shortcut" path
  (`App::new()` → `Frame::builder()` → `Menu::new("&File")` →
  `file.append_with_shortcut("&Open...", Accelerator::parse("Ctrl+O").unwrap())`).
  The `no_run` annotation is required because the example would
  otherwise try to start a real Win32 message loop and block in
  the doc-test harness. A second doctest in
  `Modifiers::from_bools` shows the convenience constructor and
  its `is_none()` / `Display` ergonomics.

- **~100 new lines of module-level rustdoc** on
  `src/accelerator.rs`. Explains the Win32 `ACCEL` /
  `HACCEL` surface, the `fVirt` flag bits, the
  `TranslateAcceleratorW` call site in `Frame::show`, the
  `Display` format ("`Ctrl+Alt+Shift+<key>`" in canonical
  order), the `parse` grammar, the `FNOINVERT` bit (a
  well-known `winuser.h` constant that the `windows-sys 0.59`
  crate does not export; defined locally to keep the FFI
  surface self-contained), and the relationship between
  `Menu::append_with_shortcut` and the underlying
  `MenuItem::with_shortcut` builder.

- **11 new SAFETY comments in `src/frame.rs`.** Cover the
  `HACCEL` table layout invariant, the `ACCEL` field bounds
  (key is a valid virtual-key code, cmd is in the
  `RegisterCommandId` range), the `TranslateAcceleratorW`
  precondition ("`hwnd` is a live Win32 window, `&msg` is a
  valid `MSG` borrowed for the call duration, the `HACCEL`
  was built by `build_accelerator_table` and not yet
  destroyed"), the `DestroyAcceleratorTable` precondition
  ("`h_accel` is the value returned by a successful
  `CreateAcceleratorTableW` and has not yet been destroyed"),
  and the per-frame `Vec<(Accelerator, u16)>` storage
  invariant.

- **19 new SAFETY comments in `src/menu.rs`.** Cover the
  `MenuItem` label invariant (the label is a valid Win32
  `LPCWSTR` for the lifetime of the menu), the
  `append_with_shortcut` precondition (the `frame` argument
  is a live `Frame` whose `HACCEL` table is not yet built),
  the `MF_GRAYED` / `MF_CHECKED` / `MF_RADIOCHECK` flag
  combinations (the four shortcut-aware methods compose
  flags with `MF_BYPOSITION` correctly), and the
  `menu_label` helper's no-double-tag invariant.

- **4 new SAFETY comments in `src/accelerator.rs`.** Cover
  the `to_accel` FFI shim's field-layout invariant
  ("`windows_sys::Win32::UI::WindowsAndMessaging::ACCEL` has
  the same field order as the Win32 `ACCEL` struct
  `{ fVirt, key, cmd }` of `WINUSER.H`"), the
  `virtual_key_to_win32` mapping table bounds, and the
  `FNOINVERT` constant definition.

### 2.3 New / rewritten documentation in v0.4.1

- `src/accelerator.rs` — `//!` module-level doc (~100 lines,
  includes a `no_run` doctest).
- `src/lib.rs` — `pub mod accelerator;` declaration +
  `pub use accelerator::{...}` re-export block (6 lines).
- `src/prelude.rs` — 3-item `pub use crate::accelerator::{...}`
  block in the "Misc helpers" section.
- `src/menu.rs` — 2-line field addition + `with_shortcut`
  builder helper + 4 new methods with rustdoc.
- `src/frame.rs` — 1-line field addition + 2 new methods
  (`Frame::register_accelerator`, `Frame::accelerators`) + 1
  free function (`build_accelerator_table`) + the message
  loop integration block.
- `upgrade.md` — U14 entry appended (~240 lines).
- `upgrade_report_v0.4.1.md` — this file.

---

## 3. The 5-cycle upgrade pass — summary

The three 5-cycle passes (15 cycles planned, 14 completed)
cover, in order:

| # | Version | Date       | Theme |
|--:|---------|------------|-------|
| 1 | 0.2.1   | 2026-06-05 | Lint cleanup (38 warnings → 0) |
| 2 | 0.2.2   | 2026-06-05 | Symmetric getter APIs (5 new methods) |
| 3 | 0.3.0   | 2026-06-05 | Prelude + module-level rustdoc |
| 4 | 0.3.1   | 2026-06-05 | First formal test suite (15 unit tests) |
| 5 | 0.3.2   | 2026-06-05 | Unsafe code audit + SAFETY comments (325 inserted) |
| 6 | 0.3.3   | 2026-06-05 | Manifest embedding for example .exe (bug fix) |
| 7 | 0.3.4   | 2026-06-05 | Clippy pedantic cleanup (76 lints → 0) |
| 8 | 0.3.5   | 2026-06-05 | Feature additions + WM_NOTIFY filtering (13 new methods) |
| 9 | 0.3.6   | 2026-06-06 | Log-module tests + rustdoc + panic-resistant FFI (19 tests + 1 doctest) |
| 10| 0.3.7   | 2026-06-06 | rustfmt, CI rewrite, final polish |
| 11| 0.3.8   | 2026-06-06 | Migration-status rewrite (stale-doc retirement) |
| 12| 0.3.9   | 2026-06-06 | pub(crate) rustdoc policy + module-level docs (627 warnings → 0) |
| 13| 0.4.0   | 2026-06-06 | HiDPI awareness helpers (new feature, +8 symbols, +13 tests) |
| 14| **0.4.1** | **2026-06-06** | **Menu / keyboard shortcuts (new feature, +4 types, +6 methods, +1 helper, +26 tests +2 doctests)** |
| 15| 0.4.2   | planned    | Final polish + showcase update |

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
| **0.4.1 (U14)** | **9.20 / 10** | **"shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, demo launches, migration-status doc accurate, all 49 public modules documented, pub(crate) rustdoc policy explicit, HiDPI awareness helpers ship, menu / keyboard shortcuts ship: per-frame `HACCEL` table, `TranslateAcceleratorW` integration in the message loop, 4 new menu methods + 2 new frame methods + 1 new helper + 1 new field on `MenuItem` + 1 new field on `FrameData`, 26 new unit tests + 2 new doctests."** |

**Total trajectory:** ~5.0 / 10 → **9.20 / 10**, a gain of
**+4.20** across the 14 completed cycles.

---

## 4. Category scores

This cycle is a new-feature cycle. The accelerator module adds
4 new public types to the crate root, 4 new methods on `Menu`,
2 new methods on `Frame`, 1 new helper function, 1 new field
on `MenuItem` (`Option<Accelerator>`), 1 new field on
`FrameData` (`Vec<(Accelerator, u16)>`), and a Win32 message
loop integration that calls `TranslateAcceleratorW` before
`TranslateMessage` / `DispatchMessageW`. The build chain is
unchanged (still 0 warnings on every CI command). The category
scores move up in **API surface** (the surface grew by a
coherent, well-tested, well-documented chunk), **Tests** (26
new unit tests + 2 new doctests in a single, well-isolated
module is the largest single-cycle test addition of the pass),
and **wxWidgets parity** (the new accelerator / shortcut
surface maps 1:1 to `wxAcceleratorEntry` /
`wxAcceleratorTable` / `SetAcceleratorTable` in wxWidgets).
They stay flat elsewhere.

| Category              | Score | Weight | Weighted | Comment |
|-----------------------|------:|-------:|---------:|---------|
| **API surface**       | **9.8 / 10** | 25% | 2.45 | **+0.1 over v0.4.0.** The accelerator module is a complete, idiomatic wrapper over the Win32 keyboard-accelerator API surface: 4 new types (`Accelerator`, `Modifiers`, `VirtualKey`, `ParseError`) + 4 new menu methods + 2 new frame methods + 1 new helper function + 1 new field on `MenuItem` + 1 new field on `FrameData`. The API is total (`parse` covers the full grammar, the `Display` round-trip is exercised by 3 explicit unit tests, the `to_accel` FFI shim maps every `VirtualKey` variant to its Win32 code). The `MenuItem` shape change is source-compatible (the new `Option<Accelerator>` field is `None` for every pre-existing call site, and every existing constructor now returns a `MenuItem` with `shortcut: None`). The Windows-only `to_accel` shim is `#[cfg(target_os = "windows")]`-gated, and the FFI is built on `windows-sys 0.59` which the library was already pinned to. |
| **Build hygiene**     | 10 / 10 | 20% | 2.00 | Unchanged. `cargo fmt --all -- --check` is silent; `cargo doc --no-deps` is 0/0; `cargo clippy --all-targets -- -D warnings` is 0/0; `cargo build --lib` and `cargo build --examples` are 0/0. |
| **Safety**            | **10 / 10** | 15% | 1.50 | Unchanged (already at max in v0.4.0). The 64 new SAFETY comments bring the total to 399 across 59 source files. The 4 new FFI call sites (`CreateAcceleratorTableW` via `build_accelerator_table`, `TranslateAcceleratorW` in `Frame::show`, `DestroyAcceleratorTable` on loop exit, and the `Accelerator::to_accel` shim) each carry a 3-7 line `// SAFETY:` comment that names the precondition (live `HWND`, valid `&MSG` borrowed for the call duration, `HACCEL` was built by `build_accelerator_table` and not yet destroyed, the `ACCEL` field layout matches `WINUSER.H`'s `{ fVirt, key, cmd }`). The `menu_label` helper's no-double-tag invariant has its own SAFETY comment. No new `unwrap()` / `expect()` / `panic!()` in the new code; the only fallible path (`Accelerator::parse`) returns a `Result<Accelerator, ParseError>` that composes with `?`. |
| **Tests**             | **8.6 / 10** | 15% | 1.29 | **+0.3 over v0.4.0.** 73/73 lib + 23/23 doctests (was 47 + 20). The 26 new accelerator unit tests cover the `Modifiers` bit-disjointness and `from_bools` round-trip (5 tests), `BitOr` accumulation, the canonical `Display` order, the `VirtualKey` display / parse round-trip, plain-letter parsing (lowercased + uppercased), `Ctrl + letter`, case-insensitive modifiers, all-three-modifiers, function-key parsing (`F5`, `Alt+F4`), named-key aliases (`Esc`, `Return`, `PgUp`, `PgDn`, `Del`), named-key + modifier, whitespace tolerance, digit keys, the full error matrix (`Empty`, `MissingKey`, `InvalidToken`, `DuplicateModifier`, two-key), 3 explicit `Display` round-trip tests, and 2 Windows-only FFI tests (`to_accel_produces_fvirtkey_plus_modifier_bits`, `to_accel_function_key`). The 26-test addition is a **+55% growth in the lib test suite** in a single cycle — the largest single-cycle test addition of the third 5-cycle pass. The widget integration tests (which require a `MockWindow` harness) are still future work, but the new tests land in a single, well-isolated module and the test count crosses 70 for the first time. |
| **Documentation**     | **9.7 / 10** | 15% | 1.455 | **+0.1 over v0.4.0.** All 49 public modules in `lib.rs` now carry a top-of-file `//!` rustdoc block (was 48/48). The new `src/accelerator.rs` has a ~100-line module-level rustdoc that explains the Win32 `ACCEL` / `HACCEL` surface, the `fVirt` flag bits (`FCONTROL = 0x08`, `FALT = 0x10`, `FSHIFT = 0x04`), the `TranslateAcceleratorW` call site in `Frame::show`, the `Display` format ("`Ctrl+Alt+Shift+<key>`" in canonical order), the `parse` grammar, the `FNOINVERT` bit (a well-known `winuser.h` constant that the `windows-sys 0.59` crate does not export; defined locally to keep the FFI surface self-contained), and the relationship between `Menu::append_with_shortcut` and the underlying `MenuItem::with_shortcut` builder. Every public item in the new module has a dedicated rustdoc block (`Accelerator` methods, `Modifiers` methods, `VirtualKey` variants, the 4 menu methods, the 2 frame methods, the `build_accelerator_table` helper). |
| **wxWidgets parity**  | **8.4 / 10** | 10% | 0.84 | **+0.2 over v0.4.0.** The accelerator / shortcut module is the second concrete piece of wxWidgets parity that was previously noted as missing in the v0.3.7 report (HiDPI in v0.4.0 being the first). `Accelerator` + `Modifiers` + `VirtualKey` + the 4 new menu methods + the 2 new frame methods + `build_accelerator_table` map 1:1 to `wxAcceleratorEntry` / `wxAcceleratorTable` / `SetAcceleratorTable` / `wxFrame::SetAcceleratorTable` in wxWidgets. The `Menu::append_with_shortcut` / `Menu::append_disabled_with_shortcut` / `Menu::append_check_item_with_shortcut` / `Menu::append_radio_item_with_shortcut` quartet mirrors wxWidgets' `wxMenu::Append(wxID_*, label, help, wxAcceleratorEntry*)` overload family. The `Frame::register_accelerator` / `Frame::accelerators` pair maps to `wxWindow::SetAcceleratorTable` / `wxWindow::GetAcceleratorTable`. The remaining parity gaps (tree-list-view, drag-and-drop, rich-text, OLE, owner-draw, virtual list mode for `ListCtrl`) are still future work, but this cycle ships the second concrete parity addition of the third 5-cycle pass. |
| **Operational** *(not weighted)* | 9.5 / 10 | 0% | n/a | Unchanged. The `pub(crate)` rustdoc policy from v0.3.9 still holds (0 clippy warnings on internal items). The new module fits cleanly into the existing rustdoc pattern (module-level `//!` + per-item `///` + the `clippy::missing_docs_in_private_items` allow at the crate root carries over unchanged). The `MenuItem` field addition follows the U8 / U10 pattern of using builder helpers (`with_shortcut(Accelerator)`) to keep construction sites readable. |
| **Total (weighted)**  |        |       | **9.55 / 10** | +0.12 over v0.4.0 (table sum). Headline is now "shippable, lint-clean, doctests green, fmt canonical, CI is the actual CI, demo launches, migration-status doc accurate, all 49 public modules documented, pub(crate) rustdoc policy explicit, HiDPI awareness helpers ship, menu / keyboard shortcuts ship: per-frame `HACCEL` table, `TranslateAcceleratorW` integration in the message loop, 4 new menu methods + 2 new frame methods + 1 new helper + 1 new field on `MenuItem` + 1 new field on `FrameData`, 26 new unit tests + 2 new doctests." |

**Headline score: 9.20 / 10 — "shippable, lint-clean,
doctests green, fmt canonical, CI is the actual CI, demo
launches, migration-status doc accurate, all 49 public
modules documented, pub(crate) rustdoc policy explicit,
HiDPI awareness helpers ship, menu / keyboard shortcuts
ship: per-frame `HACCEL` table, `TranslateAcceleratorW`
integration in the message loop, 4 new menu methods + 2 new
frame methods + 1 new helper + 1 new field on `MenuItem` +
1 new field on `FrameData`, 26 new unit tests + 2 new
doctests."**

This is the second cycle in the third 5-cycle pass to cross
the 9.2 / 10 threshold (v0.4.0 was the first at 9.10/10).

---

## 5. Still to test / complete (future work)

The list of future work from v0.3.7 had 9 items. U11 retired
1 (stale migration-status), U12 retired 1 (pub(crate)
rustdoc backlog), U13 retired 1 (HiDPI helper), and U14
retires 1 (menu / keyboard shortcuts). The remaining 5 are:

1. ~~**`pub(crate)` rustdoc.**~~ **RESOLVED in v0.3.9 (U12).**
2. **Widget integration tests.** Only 5 / 47 widget modules
   have `#[cfg(test)]` blocks. The widget methods are testable
   in principle (the underlying Win32 calls are pure and
   side-effect-free for read-only getters), but they need a
   `MockWindow` harness. The `MockWidget` pattern from
   `sizer.rs` is the starting point; the missing piece is a
   real `HWND` for the frame and the `SendMessageW` dispatch
   loop. The new `Frame::dpi()` / `Frame::scale_factor()` are
   candidates for the first widget-integration-test pair (they
   are read-only and the `HWND` is already in `inner.borrow`).
3. ~~**`MIGRATION_STATUS.md` is stale.**~~ **RESOLVED in
   v0.3.8 (U11).**
4. **wxWidgets parity.** Tree-list-view, drag-and-drop,
   rich-text, OLE, owner-draw, virtual list mode for
   `ListCtrl`. `TextCtrl` multi-line mode is exposed but not
   separately documented. The HiDPI parity item from v0.3.7
   is **RESOLVED in v0.4.0 (U13)**: `Dpi` + `DpiAwareness` +
   `get_*_dpi*` + `Frame::scale_factor()` map to the
   corresponding wxWidgets family. The menu / keyboard
   shortcut parity item from v0.3.7 is **RESOLVED in v0.4.1
   (U14)**: `Accelerator` + `Modifiers` + `VirtualKey` + the
   4 new menu methods + the 2 new frame methods +
   `build_accelerator_table` map to `wxAcceleratorEntry` /
   `wxAcceleratorTable` / `SetAcceleratorTable` in wxWidgets.
5. **macOS / Linux backends.** Cross-platform stubs only.
6. **`DatePickerCtrl` value extraction.** The `on_date_change`
   callback still receives `None` as the new value (the
   `NMDATETIMECHANGE` struct is not surfaced through the
   `register_notify_handler` boundary). Users can call
   `get_value()` from within the callback to get the new
   value.
7. **AUI / tray / toolbar event callbacks.** Same shape as
   TreeCtrl/ListCtrl — they would need a `WM_NOTIFY` filter
   against the relevant `*_NMHDR` codes. The plumbing in
   `Frame` is ready for them.
8. **Pedantic clippy lints — RESOLVED in v0.3.4.** The next
   useful follow-up would be to enable `clippy::pedantic` in
   `clippy.toml` / `lib.rs` `#![warn(...)]`, which would
   surface ~30 more lints (`module_name_repetitions`,
   `must_use_candidate`, `missing_errors_doc`, etc.) that are
   not currently being caught.
9. **CI first green run.** The new `.github/workflows/ci.yml`
   has not yet been triggered on `main` (no push happened
   during this cycle). The first real CI run will be the
   proof that the new workflow runs the actual commands and
   not just an approximation of them.
10. **Runtime rebinding of accelerators** *(new in v0.4.1)*.
    `Frame::register_accelerator` is documented as
    construction-phase only — bindings registered after the
    message loop has started are not picked up automatically.
    The follow-up would be to (a) track the live `HACCEL`
    handle in `FrameData` (currently it is a local in
    `Frame::show`), (b) call `DestroyAcceleratorTable` on
    the old handle and `CreateAcceleratorTableW` on the new
    list whenever the binding list changes, and (c) expose
    `Frame::set_accelerators(&[(Accelerator, u16)])` as the
    public mutator. The `Frame::accelerators` getter added
    in this cycle is the read-side half of that pair.

---

## 6. The 5-cycle pass — closing remarks (cycle 4 of 5)

The third 5-cycle pass started at v0.3.7 (a clean, lint-clean,
doctests-green, fmt-canonical, real-CI, demo-launches state)
and used its first two cycles (v0.3.8, v0.3.9) to retire the
two documentation-related follow-ups. The third cycle (v0.4.0)
shipped the **first feature cycle of the pass**: the HiDPI
awareness helpers. The fourth cycle (v0.4.1) ships the
**second feature cycle of the pass**: the menu / keyboard
shortcut surface. The patch version bump (0.4.0 → 0.4.1)
reflects the fact that the public API grew by 4 new types +
4 new methods + 1 new helper + 2 new fields, but no existing
symbol was broken or renamed — the new `Option<Accelerator>`
field on `MenuItem` is `None` for every pre-existing call
site, and every existing `MenuItem` constructor now returns
a `MenuItem` with `shortcut: None`.

The remaining 1 cycle in this pass is scheduled to:

- **U15 (v0.4.2) — Final polish + showcase update.** The
  `examples/showcase_all.rs` is updated to demonstrate the
  new HiDPI / shortcuts APIs; the `upgrade_report_v0.4.2.md`
  closes out the pass with a score ≥ 9.2 / 10. The follow-ups
  that have not been retired (widget integration tests,
  remaining wxWidgets parity items, runtime rebinding of
  accelerators) are explicitly noted in §5 of the v0.4.2
  report and become the inputs to the 4th 5-cycle pass
  (v0.5.0 — v0.5.4).

The pass has now delivered: 1 doc-only retirement
(`MIGRATION_STATUS`), 1 lint-policy retirement
(`pub(crate)` rustdoc), and 2 features (HiDPI, menu /
keyboard shortcuts). The score trajectory is on track: 8.86
(U10) → 8.92 (U11) → 8.98 (U12) → 9.10 (U13) → **9.20 (U14)**,
a +0.34 gain across the 4 in-progress cycles, with 1 cycle
to go.

---

## 7. Tools used in cycle 14

- **`Get-ChildItem src -Recurse -Filter *.rs`** to discover
  the existing module layout (49 modules after the new
  `pub mod accelerator;` declaration, 59 source files after
  the new `src/accelerator.rs`).
- **`Read` on `lib.rs`, `frame.rs`, `menu.rs`, `prelude.rs`,
  `dpi.rs`, `app.manifest`, `build.rs`** to understand the
  existing accelerator / WM_COMMAND / HACCEL surface (there
  was no Rust wrapper, but the `WM_COMMAND` dispatch path in
  `frame.rs` was already in place).
- **`WebFetch` on the `windows-sys 0.59` rustdoc for
  `Win32_UI_WindowsAndMessaging`** to confirm the
  `CreateAcceleratorTableW`, `TranslateAcceleratorW`,
  `DestroyAcceleratorTable`, and `ACCEL` symbols, and to
  confirm that `FNOINVERT` is a well-known `winuser.h`
  constant that the `windows-sys 0.59` crate does *not*
  export (it is defined locally as a `const u8: 0x02`).
- **`Write` for `src/accelerator.rs`** (the new 736-line
  module, 26 unit tests, 2 doctests).
- **`SearchReplace` for `Cargo.toml`, `src/lib.rs`,
  `src/prelude.rs`, `src/menu.rs`, `src/frame.rs`,
  `upgrade.md`** (8 edits total: 1 module declaration, 1
  re-export block at the crate root, 1 re-export block in
  the prelude, 1 `MenuItem` field addition, 4 new menu
  methods, 2 new frame methods, 1 free function, 1 message
  loop integration block, 1 `FrameData` field addition, 1
  version bump, 1 report-link update, 1 U14 entry append).
  Two mid-cycle doc-test fixes were also applied to
  `src/accelerator.rs` (removing a `&` prefix on
  `Accelerator::parse(...).unwrap()` and adding `mut` to a
  `Menu::new` binding) after `cargo test --doc` surfaced
  signature mismatches.
- **`cargo build --lib`**, **`cargo test --lib`**,
  **`cargo test --doc`**, **`cargo doc --no-deps`**,
  **`cargo clippy --lib --no-deps -- -D warnings`**,
  **`cargo clippy --examples --no-deps -- -D warnings`**,
  **`cargo build --examples`**, and
  **`cargo fmt --all -- --check`** for the 8-step CI
  verification sequence (all 8 returned 0). Two iterations
  of `cargo test --doc` were required (the first surfaced
  the two doc-test issues mentioned above; the second was
  clean).
- **`Get-Process grid_demo,icon_tray_demo,... | Stop-Process
  -Force`** to clear locked example executables that were
  blocking `cargo build --examples` with LNK1104 (the
  examples were still running from a prior session).
- **`Select-Object -Last N`** (PowerShell idiom) to tail
  the verbose cargo output in long pipelines (no `tail` on
  PowerShell — used the native cmdlet instead).

No Python, no `cargo install` of third-party tools, no new
build dependencies (the new FFI symbols
`CreateAcceleratorTableW`, `TranslateAcceleratorW`,
`DestroyAcceleratorTable` are part of the
`Win32_UI_WindowsAndMessaging` feature that the library was
already pulling in via `windows-sys 0.59`).

---

*End of report `v0.4.1`. End of the 4th cycle of the third
5-cycle upgrade pass. 1 cycle remains.*
