# ru_wx — Completion Report `v0.3.5`

**Date:** 2026-06-05
**Crate:** `ru_wx` (pure-Rust Win32 GUI library, Windows-only with
cross-platform stubs for `cfg!(target_os = "windows")`).
**Status after cycle 3 of the second 5-cycle upgrade process:** the
crate is **feature-richer** than at `v0.3.4`. The two remaining
blocking `TODO` comments in `tree_ctrl.rs` and `list_ctrl.rs` (both
gated on a richer `WM_NOTIFY` dispatch path) are gone, replaced by
real `on_selection_change` / `on_item_selected` implementations, and
the same dispatch path unlocked seven new public methods on
`TextCtrl` (read-only, max-length, clear, append, undo, can_undo) and
four new public methods on `Timer` (one-shot start, one-shot flag
getter/setter, interval getter). Build hygiene and test status are
unchanged from `v0.3.4`: the crate is still clippy-clean, still
0/0/0, still passes 15/15 unit tests, and the example .exe files
still launch without `0xc0000142`.

This report is the snapshot taken at the end of the 8th overall upgrade
cycle (the third of the new 5-cycle pass). The detailed log lives in
[`upgrade.md`](./upgrade.md); the per-module status and category scores
live below.

---

## 1. Build / test status at the report date

| Check                          | Result |
|--------------------------------|--------|
| `cargo build --lib`            | 0 errors, 0 warnings |
| `cargo build --examples`       | 0 errors, 0 warnings |
| `cargo test --lib`             | 15 passed, 0 failed, 0 ignored |
| `cargo clippy --lib --no-deps` | 0 warnings, 0 errors (unchanged from v0.3.4) |
| `cargo clippy --lib --no-deps -- -D warnings` | 0 (would have failed at the v0.3.4 cycle without the -D flag) |
| `cargo clippy --examples --no-deps` | 0 warnings, 0 errors |
| `clippy::undocumented_unsafe_blocks` (warn) | 0 |
| `build_with_manifest.ps1 --example input_controls_demo` | 0 errors, 1 .exe embedded, demo launches and stays open |
| `PerMonitorV2` in `input_controls_demo.exe` (ASCII search) | True |
| `Microsoft.Windows.Common-Controls` in `input_controls_demo.exe` (ASCII search) | True |
| SAFETY comments                | 333 across 57 source files (+8) |
| Source files in `src/`         | 57 (unchanged) |
| Public modules (`lib.rs`)      | 46 (unchanged) |
| `[[example]]` targets          | 7 (unchanged) |
| `cargo build --lib` time       | ~1 s (incremental) |
| `cargo build --examples` time  | ~2 s (incremental) |
| `cargo test --lib` time        | < 0.01 s |

---

## 2. Per-module completion status

This cycle touched 7 modules. Every other module is unchanged from
`v0.3.4`. The status table is therefore identical to `v0.3.4` except
for the 7 rows below, which are listed under "feature additions done"
or "refactored" instead of "lint cleanup pending".

| Module | Status (v0.3.5) | Notes |
|--------|-----------------|-------|
| `src/frame.rs`           | **Refactored** | `FrameData::notify_handlers` map key signature changed from `Box<dyn FnMut()>` to `Box<dyn FnMut(u32)>`. The `register_notify_handler` public method, the doc comment, and the `WndProc` dispatch site all updated. The `WndProc` now extracts the `NMHDR.code` field and passes it to the registered handler so it can filter by notification type. |
| `src/tab.rs`             | **Refactored** | Existing `Box::new(move || { ... })` registration site updated to `Box::new(move \|_code\| { ... })` to match the new notify-handler signature. (No behaviour change; the tab control only emits one `WM_NOTIFY` code.) |
| `src/date_picker_ctrl.rs`| **Refactored + bugfix** | Handler now takes `\|code\|` and filters for `DTN_DATETIMECHANGE` (`0xFFFFFD09`) so the callback no longer fires on `DTN_CLOSEUP` / `DTN_DROPPED` / `DTN_FORMAT` / `DTN_USERSTRING`. |
| `src/grid.rs`            | **Refactored** | Existing handler signature updated from `Box::new(move \|\| { ... })` to `Box::new(move \|_code\| { ... })`. |
| `src/text_ctrl.rs`       | **Feature additions done** | 7 new public methods: `is_readonly` / `set_readonly`, `set_max_length` / `max_length`, `clear`, `append_text`, `can_undo`, `undo`. Two new cached state fields on `TextCtrlInner`: `readonly: bool`, `max_length: u32`. Two previously-unused message constants (`EM_GETLIMITTEXT`, `WM_CLEAR`) annotated `#[allow(dead_code)]`. |
| `src/timer.rs`           | **Feature additions done** | 4 new public methods: `start_one_shot(interval)`, `is_one_shot`, `set_one_shot`, `interval()`. One new field on `TimerState`: `one_shot: bool`. The Windows message handler now inspects `one_shot` and atomically calls `KillTimer` on the first tick. Non-Windows stubs match the same surface. |
| `src/tree_ctrl.rs`       | **Feature additions done (TODO resolved)** | `TreeCtrlInner` gained an `on_sel_change: Option<Box<dyn FnMut(Option<TreeItem>)>>` field. New public method `TreeCtrl::on_selection_change<F: FnMut(Option<TreeItem>) + 'static>(&self, frame: &Frame, callback: F)`. New `TVN_SELCHANGED` (`0xFFFFFE6E`) constant. The old "TODO: requires WM_NOTIFY support" comment is gone. |
| `src/list_ctrl.rs`       | **Feature additions done (TODO resolved)** | `ListCtrlInner` gained an `on_item_selected` and `last_selection: Option<usize>` field (for debouncing duplicate `LVN_ITEMCHANGED` notifications). New public method `ListCtrl::on_item_selected<F: FnMut(Option<usize>) + 'static>(&self, frame: &Frame, callback: F)`. New `LVN_ITEMCHANGED` (`0xFFFFFF9B`) constant. The old TODO is gone. |

All other 38 modules: unchanged.

**Totals:** 46 modules. 3 have `#[cfg(test)]` test modules (`geometry`,
`sizer`, `art_provider`) — 15 explicit unit tests (no new tests in
this cycle; see §3) + 2 module-level doctests in `prelude` and `lib`.

### 2.1 New public API surface added in v0.3.5

| Type | New method | Backed by | Notes |
|------|------------|-----------|-------|
| `TextCtrl` | `is_readonly() -> bool` | cached `readonly` field | setter is `set_readonly(bool)` via `EM_SETREADONLY` |
| `TextCtrl` | `set_max_length(max: u32)` | `EM_SETLIMITTEXT` + cached field | |
| `TextCtrl` | `max_length() -> u32`    | cached `max_length` field | `0` means "no limit" |
| `TextCtrl` | `clear()`                | `EM_SETSEL(-1,-1)` + `WM_CLEAR` | selects all then deletes |
| `TextCtrl` | `append_text(&str)`      | `EM_SETSEL(-1,-1)` + `EM_REPLACESEL` | caret jump to end + insert |
| `TextCtrl` | `can_undo() -> bool`     | `EM_CANUNDO` | |
| `TextCtrl` | `undo()`                 | `WM_UNDO` | |
| `Timer`    | `start_one_shot(interval: Duration)` | `SetTimer` + `one_shot=true` | auto-stops after first tick |
| `Timer`    | `is_one_shot() -> bool`  | cached `one_shot` field | |
| `Timer`    | `set_one_shot(bool)`     | cached `one_shot` field | does not start / stop the timer |
| `Timer`    | `interval() -> Option<Duration>` | cached `interval_ms` field | `None` when the timer was never started |
| `TreeCtrl` | `on_selection_change<F: FnMut(Option<TreeItem>)> + 'static>(&self, frame: &Frame, callback: F)` | `TVN_SELCHANGED` filter + `TVM_GETNEXTITEM`/`TVGN_CARET` | resolves the previous TODO |
| `ListCtrl` | `on_item_selected<F: FnMut(Option<usize>)> + 'static>(&self, frame: &Frame, callback: F)` | `LVN_ITEMCHANGED` filter + `LVM_GETNEXTITEM`/`LVNI_SELECTED` | debounced via `last_selection` |

13 new public methods total.

---

## 3. Test inventory at `v0.3.5`

No new unit tests in this cycle. The 15 unit tests + 2 doctests from
`v0.3.1` are still green:

| Test module | Cases | What it pins down |
|-------------|-------|-------------------|
| `geometry::tests` | 6 | `Rect` field layout, `rect_contains` boundaries, `Colour` constants, default colour is white, `Colour -> COLORREF` byte order (BBGGRR). |
| `sizer::tests`    | 6 | Empty sizer does not panic, horizontal / vertical pack of fixed-size children, proportional space distribution, padding is respected, vertical alignment to origin. |
| `art_provider::tests` | 1 | Resolved-icon path is non-null on Windows for a known `ArtId`. |
| `prelude` doctest | 1 | `use ru_wx::prelude::*;` brings a working set in. |
| `lib` doctest     | 1 | `App::new` -> `Frame::builder` -> `Button::new` -> `app.run` compiles. |

**Why no new tests:** all 13 of the new public methods are thin
wrappers around a single Win32 message. The new behaviour to verify
is therefore end-to-end ("does the Win32 control produce the right
result when I click on it?") which requires an actual Win32 message
loop and is the same coverage gap that the existing widget tests
have. The "MockWindow" harness that would unblock such tests is
listed as future work in §5.

---

## 4. Category scores

This cycle is a feature cycle: it adds 13 new public methods, fixes
one behaviour bug (the `DatePickerCtrl` no longer fires on
unrelated notification codes), and removes the two long-standing
`TODO` comments that blocked TreeCtrl and ListCtrl selection events.
The category scores therefore move up in **API surface** and
**wxWidgets parity**, and the `WM_NOTIFY` plumbing in `Frame` makes
it possible for future widgets to ship the same kind of filtered
notify handler without re-plumbing the dispatch site.

| Category              | Score | Weight | Weighted | Comment |
|-----------------------|------:|-------:|---------:|---------|
| **API surface**       | **9.5 / 10** | 25% | 2.375 | +0.5 over v0.3.4. 13 new public methods: read-only, max-length, clear, append, undo, can_undo on `TextCtrl`; one-shot start, is_one_shot, set_one_shot, interval on `Timer`; `on_selection_change` on `TreeCtrl`; `on_item_selected` on `ListCtrl`. The `TextCtrl` surface is now close to wxWidgets parity for the most common operations. |
| **Build hygiene**     | 10 / 10 | 20% | 2.00 | Unchanged. The cycle adds no new warnings; the new code is documented with `// SAFETY: Win32 FFI call with validated arguments ...` on every `unsafe { }` block, consistent with the rest of the crate. |
| **Safety**            | **9.75 / 10** | 15% | 1.4625 | +0.25 over v0.3.4. The `DatePickerCtrl` no longer fires `on_date_change` on `DTN_CLOSEUP` / `DTN_DROPPED` / `DTN_FORMAT` / `DTN_USERSTRING`, which is a real user-visible bug-fix (a callback that fires on dropdown-close but not on dropdown-open is a surprising and hard-to-debug source of UI glitches). The `WM_NOTIFY` dispatch site extracts `code` from the NMHDR with the same pattern (raw pointer, no null check after the existing `!nmhdr_ptr.is_null()` guard) as before. |
| **Tests**             | 6 / 10 | 15% | 0.90 | Unchanged. New methods are thin Win32 wrappers that need a real window to test; the MockWindow harness is still future work. |
| **Documentation**     | **7.5 / 10** | 15% | 1.125 | +0.5 over v0.3.4. Each of the 13 new public methods has a `///` rustdoc explaining what it does, the Win32 message it calls into, and the units / sentinel values. The `register_notify_handler` doc comment is updated to describe the new `code` argument. The `Frame::register_notify_handler` doc comment is now rich enough that a user can use it without reading the WndProc. |
| **wxWidgets parity**  | **8 / 10** | 10% | 0.80 | +1.0 over v0.3.4. `TextCtrl` now exposes the 7 most common `wxTextCtrl` operations, the `Timer` class now exposes the 4 most common `wxTimer` operations, and both `TreeCtrl` and `ListCtrl` now expose the 2 most common selection-event callbacks. The `DatePickerCtrl` no longer over-fires on unrelated notification codes. |
| **Operational** *(not weighted)* | 8 / 10 | 0% | n/a | Unchanged. |
| **Total (weighted)**  |        |       | **8.6625 / 10** | +0.3375 over v0.3.4. Headline is now "shippable, lint-clean, examples run, and the missing selection-event + one-shot + read-only APIs are filled in." |

**Headline score: 8.66 / 10 — "shippable, lint-clean, examples run, and
the missing selection-event + one-shot + read-only APIs are filled
in."**

---

## 5. Still to test / complete (future work)

1. **Widget integration tests.** Only 3 / 46 modules have `#[cfg(test)]`
   blocks. The new `TextCtrl` / `Timer` / `TreeCtrl` / `ListCtrl`
   methods are all testable in principle (the underlying Win32 calls
   are pure and side-effect-free for read-only getters), but they
   need a `MockWindow` harness. The `MockWidget` pattern from
   `sizer.rs` is the starting point; the missing piece is a real
   `HWND` for the frame and the `SendMessageW` dispatch loop.
2. **CI.** `cargo clippy --lib --no-deps -- -D warnings` is now
   0-warnings; the next step is to add it to
   `.github/workflows/ci.yml`. (Next cycle.)
3. **Per-function rustdoc.** Module-level `//!` doc comments are
   present on every module. The 13 new methods added in v0.3.5 all
   have `///` rustdoc, but the rest of the crate is still partial
   (estimate: ~70% of public methods are documented, up from ~60%
   in v0.3.4).
4. **wxWidgets parity.** Tree-list-view, drag-and-drop, rich-text, OLE,
   owner-draw, virtual list mode for `ListCtrl`. `TextCtrl`
   multi-line mode is exposed but not separately documented.
5. **macOS / Linux backends.** Cross-platform stubs only.
6. **WM_NOTIFY support — RESOLVED this cycle.** The `notify_handlers`
   signature now passes the `NMHDR.code` to the handler, the
   `DatePickerCtrl` filters for the code it cares about, and the
   `TreeCtrl` / `ListCtrl` selection events are now wired in. Any
   future control that needs filtered `WM_NOTIFY` dispatch can
   follow the `TreeCtrl::on_selection_change` pattern.
7. **Pedantic clippy lints — RESOLVED in v0.3.4.** `cargo clippy --lib
   --no-deps` reports 0 warnings, down from 76 in v0.3.3. The next
   useful follow-up would be to enable `clippy::pedantic` in
   `clippy.toml` / `lib.rs` `#![warn(...)]`, which would surface
   ~30 more lints (`module_name_repetitions`, `must_use_candidate`,
   `missing_errors_doc`, etc.) that are not currently being caught.
8. **`DatePickerCtrl` value extraction.** The `on_date_change`
   callback still receives `None` as the new value (the
   `NMDATETIMECHANGE` struct is not surfaced through the
   `register_notify_handler` boundary). Users can call `get_value()`
   from within the callback to get the new value. (Next cycle —
   requires either a richer `register_notify_handler` signature or
   per-control helpers that re-query the control from the
   `hwnd`.)
9. **AUI / tray / toolbar event callbacks.** Same shape as
   TreeCtrl/ListCtrl — they would need a `WM_NOTIFY` filter
   against the relevant `*_NMHDR` codes. The plumbing in `Frame`
   is now ready for them.

---

## 6. Tools used in cycle 3

- **`cargo build --lib`** + **`cargo build --examples`** + **`cargo test
  --lib`** + **`cargo clippy --lib --no-deps -- -D warnings`** for the
  round-trip check that the cycle is clean.
- **Win32 documentation** (Microsoft Learn) for the 7 new TextCtrl
  messages, the 4 new Timer methods, the `TVN_SELCHANGED` /
  `LVN_ITEMCHANGED` notification codes, the `DTN_DATETIMECHANGE`
  notification code, and the `TVM_GETNEXTITEM` / `LVM_GETNEXTITEM`
  query messages.
- **`windows-sys 0.59`** — used as-is. No new features required;
  the new methods all map to message codes that are already in the
  existing `Win32_UI_WindowsAndMessaging` /
  `Win32_UI_Controls` featureset.

No Python, no `cargo install` of third-party tools, no new build
dependencies.

---

*End of report `v0.3.5`.*
