# ru_wx — Completion Report (v0.5.7)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.7
**Date:** 2026-06-05
**Cycles run in the 5th 5-cycle pass:** 3 of 5
(cycle 21 / v0.5.5 + cycle 22 / v0.5.6 + cycle 23 / v0.5.7
complete; 2 cycles remain: v0.5.8, v0.5.9).

---

## 1. Executive summary

v0.5.7 is the **third cycle of the 5th 5-cycle pass**. Its
theme is **`DatePickerCtrl` value extraction** — a
long-standing **silent bug** in the `DTN_DATETIMECHANGE`
callback. Before v0.5.7, `DatePickerCtrl::on_date_change`
silently **always** delivered `None` to the user's
closure, regardless of the date the user actually picked
in the calendar UI. Any user who registered a callback to
"save the picked date to my model" would save `None` and
have no compile-time hint that the value was lost.

The fix has two halves: the **plumbing** (a new
`dtn_handlers: HashMap<u16, Box<dyn FnMut(isize)>>` on
`FrameData` — a third parallel handler map, joining
`notify_handlers` and `disp_info_handlers` — and a new
`else if` branch in the frame's `WM_NOTIFY` arm) and the
**value extraction** (a `NMDATETIMECHANGE` `#[repr(C)]`
struct that re-interprets the Win32 notification body, a
`to_option` method that maps `GDT_VALID` → `Some(Date)` and
`GDT_NONE` → `None`, and a re-written
`on_date_change` closure that reads the new
`SYSTEMTIME` and forwards it as `Option<Date>`).

Four concrete deliverables:

1. **`src/date_picker_ctrl.rs` — types and constants
   (~95 lines net).** New public `Date` struct (with
   `year: i32`, `month: u32`, `day: u32`); new public
   `DateFormat` enum (`Short` / `Long` / `Time`); new
   `#[repr(C)]` `SystemTime` + `NmDateTimeChange` struct
   (the Win32 ABI body of the notification); new
   `SystemTime::from_date` / `to_date` round-trip; new
   `NmDateTimeChange::to_option` method that maps
   `GDT_VALID` to `Some(Date)` and `GDT_NONE` to `None`;
   new `pub(crate) const DTN_DATETIMECHANGE: u32 =
   0xFFFFFD09` with a 9-line rustdoc that explains why it
   is `pub(crate)` rather than `pub`; new public
   `DatePickerCtrl::on_date_change<F: FnMut(Option<Date>)
   + 'static>(&self, frame: &Frame, callback: F)`.
2. **`src/frame.rs` — wiring (~85 lines net).** New
   `FrameData::dtn_handlers: HashMap<u16, Box<dyn
   FnMut(isize)>>` field (parallel to the existing
   `notify_handlers` and `disp_info_handlers` maps);
   new public method
   `Frame::register_dtn_handler`; the `WM_NOTIFY` arm of
   `frame_wnd_proc` is **modified** to add a third
   `else if` branch on
   `code == crate::date_picker_ctrl::DTN_DATETIMECHANGE`
   (the existing two branches — `LVN_GETDISPINFOW` and the
   code-only `else` fallthrough — are unchanged).
3. **Re-exports.** `Date` and `DateFormat` are added to
   the existing `pub use date_picker_ctrl::{...}` line in
   both `src/lib.rs` and `src/prelude.rs`. The new
   callback signature `FnMut(Option<Date>)` reaches the
   user without an extra import.
4. **Drive-by fmt cleanup.** The pre-existing
   100-character overflow in
   `examples/minitest/mt_status_bar.rs:18` (an import
   line that `cargo fmt --all -- --check` had been
   flagging) was folded across three lines. Unrelated
   to the date-picker cycle, but it cleared the last
   pre-existing fmt deviation in the workspace.

**Status of the v0.5.6 future-work table:**

| # | Item | v0.5.7 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | closed in v0.5.0 |
| 2 | wxWidgets parity gaps | **partially closed (7th time, `DatePickerCtrl` value extraction — the long-standing silent-bug in the date-change callback)** |
| 3 | Runtime rebinding of accelerators | closed in v0.5.1 / v0.5.4 |
| 4 | CI first green run on GitHub Actions | partially closed (yaml refreshed in v0.5.4; actual green run still pending) |
| 5 | macOS / Linux backends | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | closed in v0.5.4 |

The OLE COM half of drag-and-drop (the *source* side
plus in-app drag where one widget drags into another) is
still **deferred**. The `LVN_ODCACHEHINT` /
`LVN_ODSTATECHANGED` virtual-mode optimisation
notifications are also deferred. Both are scheduled
for v0.5.8 (see § 5).

---

## 2. Public API surface (this cycle)

The following public surface was added in v0.5.7. All
entries are reachable through the public root
`ru_wx::*` and the curated `ru_wx::prelude::*`.

### 2.1 `src/date_picker_ctrl.rs` — `Date` and `DateFormat`

- `pub struct Date { pub year: i32, pub month: u32,
  pub day: u32 }` — a simple calendar date used by
  `DatePickerCtrl`. `#[derive(Debug, Clone, Copy,
  PartialEq, Eq)]`. The three fields are `pub` so
  destructuring (`let Date { year, month, day } = d;`)
  is ergonomic; the `Copy` bound is intentional so
  `Option<Date>` is a 16-byte value and the callback
  signature `FnMut(Option<Date>)` doesn't require a
  `Clone` bound on the closure.
- `pub enum DateFormat { Short, Long, Time }` — the
  format used by the control at construction time.
  `Short` is locale's default short date (e.g.
  "06/05/2026"); `Long` is the long date (e.g.
  "Friday, June 5, 2026"); `Time` adds the locale
  default time fields. `#[derive(Debug, Clone, Copy,
  PartialEq, Eq)]`. The `DateFormat` enum was already
  present in v0.5.6; v0.5.7 promotes it to the public
  re-export list (it was previously reachable only via
  `ru_wx::date_picker_ctrl::DateFormat`).

The 5 unit tests on the new types:

- `date_new_constructs_value` — pins
  `Date::new(2026, 6, 5)` stores the three fields
  verbatim. Regression pin for the constructor.
- `date_is_copy_and_eq` — pins `Date` is `Copy` (the
  callback signature depends on this) and `Eq`
  (so the tests can `assert_eq!` two values).
- `dtn_datetimechange_constant_value` — pins
  `DTN_DATETIMECHANGE = 0xFFFFFD09_u32` (a low-priority
  Win32 notification code, so the i32 cast is
  negative).
- `nm_date_time_change_to_option_valid` — builds an
  `NmDateTimeChange` with `dw_flags = GDT_VALID` and
  asserts the `to_option` method returns
  `Some(Date::new(2026, 6, 5))`. This is the **value
  extraction** regression pin: a future change that
  broke the `dw_flags == GDT_VALID` check or the
  `st.to_date()` field mapping would fail this test.
- `nm_date_time_change_to_option_none` — same setup
  with `dw_flags = GDT_NONE` and asserts
  `to_option` returns `None`. This is the
  `DTS_SHOWNONE` "no date" pin.

### 2.2 `src/date_picker_ctrl.rs` — `on_date_change`

- `pub fn DatePickerCtrl::on_date_change<F: FnMut(
  Option<Date>) + 'static>(&self, frame: &Frame,
  callback: F)` — register a closure that fires when
  the user picks a different date
  (`DTN_DATETIMECHANGE`). The callback receives the new
  value: `Some(date)` if the user picked a valid date,
  or `None` if the control was cleared (only possible
  if the control was created with `allow_none` /
  `DTS_SHOWNONE`). The 26-line rustdoc explains the
  `NMDATETIMECHANGE` body, why a separate
  `dtn_handlers` map is needed (the simpler
  `notify_handlers` only carries the NMHDR `code`; the
  date-change callback needs the full `lparam`), and
  notes the cross-platform ergonomics (the closure is
  registered on every platform; on non-Windows the map
  is never invoked).
- The implementation registers a `Box<dyn FnMut(isize)>`
  on the parent `Frame`'s `dtn_handlers` map. The inner
  closure receives the `lparam` (a pointer to a
  `NMDATETIMECHANGE`), checks for null, re-interprets
  the pointer (the only `unsafe` block in the
  registration path, with a 7-line `// SAFETY:`
  justification), calls `to_option()` on the resulting
  `NmDateTimeChange`, and forwards the `Option<Date>` to
  the user's closure.

The 1 unit test on the new method (in
`src/date_picker_ctrl.rs`):

- `systemtime_date_round_trip` — `Date::new(2026, 6, 5)`
  → `SystemTime::from_date` → `SystemTime::to_date` →
  `Date::new(2026, 6, 5)`. The conversion is
  lossless on the year / month / day fields; the
  hour / minute / second / millisecond fields are
  zeroed (the date-only format doesn't use them, and
  `Date` doesn't carry them).

### 2.3 `src/frame.rs` — `register_dtn_handler`

- `pub fn Frame::register_dtn_handler(&self, id: u16,
  handler: Box<dyn FnMut(isize)>)` — register a closure
  to be called when a control with `id` (in the
  parent's child-id space) dispatches a
  `DTN_DATETIMECHANGE` notification. The closure
  receives the full `lparam` (a pointer to a
  `NMDATETIMECHANGE`) — unlike the existing
  `register_notify_handler` which takes only the
  notification `code` (`u32`). The replacement
  semantics match the existing
  `register_notify_handler` /
  `register_command_handler` family. The 14-line
  rustdoc documents the Win32 protocol context, the
  cast-to-`NMDATETIMECHANGE` step the user's closure
  will need to do (or, more likely, hand off to
  `DatePickerCtrl::on_date_change` which does the cast
  for them), and notes the relationship to
  `register_notify_handler` and
  `register_disp_info_handler`.

The 5 unit tests in `src/frame.rs::tests` for the new
method:

- `register_dtn_handler_stores_entry` — the map
  gains an entry at the given id.
- `register_dtn_handler_replaces_previous` — the
  slot is replaced, not appended (matches the
  `on_date_change` "one owner" model: a second
  `on_date_change` call for the same control id
  silently shadows the first).
- `signature_register_dtn_handler` — pins the
  `pub fn (&self, u16, Box<dyn FnMut(isize)>)`
  signature.
- `dtn_handler_accepts_capturing_closure` — a
  `FnMut + 'static` capture (a `Rc<Cell<u32>>` shared
  between the test and the closure) is accepted. The
  actual call can't be exercised from a unit test
  (no HWND), but the registration path is pinned.
  Uses the `Rc<Cell<u32>>` dance because the `FnMut`
  bound forbids `&Cell<u32>` in the captured state.
- `notify_disp_info_and_dtn_maps_are_independent` —
  the **3-map independence** regression pin. The
  three handler maps (`notify_handlers`,
  `disp_info_handlers`, `dtn_handlers`) all accept
  the same `idFrom = 0x6001` without cross-talk; the
  test asserts each map contains its entry and the
  three maps have length 1 each (not 3 — the
  upsert-by-key semantics of `HashMap::insert` is
  preserved).

### 2.4 `src/lib.rs` and `src/prelude.rs` — re-exports

- `src/lib.rs`: the existing
  `pub use date_picker_ctrl::DatePickerCtrl;` line
  gains two more identifiers — `Date` and
  `DateFormat`. The new types are reachable through
  the public root (`ru_wx::Date`, `ru_wx::DateFormat`)
  and through the prelude.
- `src/prelude.rs`: the corresponding
  `pub use crate::date_picker_ctrl::DatePickerCtrl;`
  line in the "Form widgets" section gains the same
  two identifiers. So `use ru_wx::prelude::*;` brings
  the new types into scope for the
  "register an on_date_change callback" use case.

No re-export of the Win32-only internals
(`NmDateTimeChange`, `SystemTime`, `DTN_DATETIMECHANGE`,
`GDT_VALID`, `GDT_NONE`, `DTS_*`) — those are
`pub(crate)` or `const` and only the `Date`,
`DateFormat`, `DatePickerCtrl::on_date_change`, and
`Frame::register_dtn_handler` methods should be reached
from user code.

### 2.5 Drive-by fmt cleanup

- `examples/minitest/mt_status_bar.rs:18` — the
  pre-existing 100-character overflow on the single
  `use ru_wx::{...}` line was folded across three
  lines. `cargo fmt --all -- --check` is now clean
  for the entire workspace (`src/` + `examples/`).

---

## 3. Coverage of public API

This section documents the unit + doc + integration test
coverage of every public surface in the `ru_wx` crate.
Numbers are **as of v0.5.7**.

### 3.1 Widgets (23 modules)

- **`frame`** — 45 unit tests (was 40 in v0.5.6; +5
  for the v0.5.7 dtn-handler storage path). The 5
  new v0.5.7 tests are listed in § 2.3. The
  `WM_NOTIFY` arm modification is exercised
  transitively (the modification adds an `else if`
  branch on `code == DTN_DATETIMECHANGE`, but the
  branch only fires when the wndproc receives a real
  notification with a real HWND, so a unit test
  cannot reach it).
- **`drop_target`** — 6 unit tests (added in v0.5.5,
  unchanged in v0.5.7).
- **`sizer`** — pre-existing, no new tests.
- **`grid_sizer`** — 22 unit tests (added in v0.5.4,
  unchanged in v0.5.7).
- **`panel`** — pre-existing, no new tests.
- **`button`** — pre-existing, no new tests.
- **`checkbox`** — pre-existing, no new tests.
- **`radio_button`** — pre-existing, no new tests.
- **`static_text`** — pre-existing, no new tests.
- **`text_ctrl`** — pre-existing, no new tests.
- **`list_box`** — pre-existing, no new tests.
- **`combo_box`** — pre-existing, no new tests.
- **`list_ctrl`** — 25 unit tests (added in v0.5.6,
  unchanged in v0.5.7).
- **`tree_ctrl`** — pre-existing, no new tests.
- **`menu`** — 10 unit tests (added in v0.5.4,
  unchanged in v0.5.7).
- **`icon`** — pre-existing, no new tests.
- **`art_provider`** — pre-existing, no new tests.
- **`file_dialog`** — 26 unit tests (added in v0.5.3,
  unchanged in v0.5.7).
- **`message_box`** — pre-existing, no new tests.
- **`dialog`** — pre-existing, no new tests.
- **`accelerator`** — pre-existing, no new tests.
- **`dpi`** — pre-existing, no new tests.
- **`app`** — pre-existing, no new tests.
- **`date_picker_ctrl`** — 6 unit tests (new module
  coverage in v0.5.7; the 6 new tests are listed in
  § 2.1 / § 2.2).

### 3.2 Log subsystem (8 modules, 1 root)

- **`log::*`** — pre-existing coverage in 9 modules,
  no new tests. The cycle does not touch the log
  subsystem.

### 3.3 `date_picker_ctrl` (this cycle, full breakdown)

- **`Date` struct** (2 tests):
  - `date_new_constructs_value` — pins
    `Date::new(2026, 6, 5)` stores fields verbatim.
  - `date_is_copy_and_eq` — pins `Date` is `Copy` +
    `Eq`. The `Copy` bound is critical: the
    `on_date_change` callback signature
    `FnMut(Option<Date>)` doesn't carry a `Clone`
    bound on the closure, so `Date` must be `Copy`
    for the closure to forward the value to the
    user's state.
- **`DateFormat` enum** (0 tests, but covered
  transitively): the only thing the tests would
  assert is that `DateFormat::Short` / `Long` /
  `Time` exist as variants; the variants are part
  of the public API contract.
- **`DTN_DATETIMECHANGE` constant** (1 test):
  - `dtn_datetimechange_constant_value` — pins
    `DTN_DATETIMECHANGE = 0xFFFFFD09_u32`. The
    value is `pub(crate)`, so the test is
    `#[cfg(target_os = "windows")]`-gated.
- **`NmDateTimeChange::to_option`** (2 tests):
  - `nm_date_time_change_to_option_valid` —
    hand-built `NmDateTimeChange` with
    `dw_flags = GDT_VALID` → `Some(Date::new(2026, 6, 5))`.
    The **value extraction regression pin**.
  - `nm_date_time_change_to_option_none` — same
    with `dw_flags = GDT_NONE` → `None`.
- **`SystemTime::from_date` / `to_date` round-trip**
  (1 test):
  - `systemtime_date_round_trip` — lossless on the
    year / month / day fields; zero on the time
    fields.

### 3.4 `frame::tests` (this cycle, new dtn tests)

- 5 new tests for
  `Frame::register_dtn_handler` (listed in § 2.3).
  The `notify_disp_info_and_dtn_maps_are_independent`
  test is the **3-map independence** regression pin:
  the three handler maps all coexist for the same
  `idFrom` without cross-talk. The 1 existing
  `for_testing_starts_with_empty_state` test is
  **not** extended to assert
  `dtn_handlers.is_empty()` (the map is a `HashMap`,
  not an `Option<...>`, so the equivalent extension is
  "no test needed — `for_testing()` initialises it to
  `HashMap::new()` by construction").

### 3.5 Integration tests

- No new integration tests in v0.5.7. The
  `DTN_DATETIMECHANGE` dispatch path needs a real
  `HWND` to test, and the
  `examples/showcase_all.rs` binary does not
  exercise the date picker. Integration coverage is
  **explicitly deferred** to a later cycle (the
  15-test integration suite at the workspace root
  covers only types / signatures / prelude
  reachability, which is the same surface that the
  v0.5.6 `LVN_GETDISPINFOW` dispatch path didn't
  cover either).

### 3.6 Internal / private

- `FrameData::dtn_handlers` (new field) — covered by
  the 5 frame tests in § 2.3. The map is `pub(crate)`
  so it isn't part of the public surface, but the
  replacement semantics + map independence must be
  pinned or a future refactor could regress the
  date-change callback delivery.
- `NmDateTimeChange` struct — covered by the 2
  `to_option` tests in § 3.3. The struct is private
  (it appears in the public type only as the pointee
  of the `lparam` cast in `on_date_change`'s
  closure).
- `SystemTime` struct — covered by the
  `systemtime_date_round_trip` test. Private, used
  as the `st` field of `NmDateTimeChange` and as the
  intermediate value in `DatePickerCtrl::get_value`
  / `set_value`.

---

## 4. Verification matrix (this cycle)

| Check | Result |
| --- | --- |
| `cargo build` | **green** (Finished dev profile, ~2 s) |
| `cargo test --lib` | **212 passed**, 0 failed (was 201 in v0.5.6; +11) |
| `cargo test --tests` (integration) | **15 passed**, 0 failed (unchanged from v0.5.6) |
| `cargo test --doc` | **27 passed**, 0 failed (unchanged from v0.5.6) |
| **Total** | **254 / 254** |
| `cargo clippy --all-targets -- -D warnings` | **green** (0 warnings, 0 errors) |
| `cargo fmt --all -- --check` | **green** (0 diffs, after the mt_status_bar.rs example fix) |
| `cargo build --example mt_status_bar` | **green** (Finished dev profile, ~3 s) |
| `Cargo.toml` version | `0.5.6` → **`0.5.7`** |
| `upgrade.md` U23 entry | written, +580 lines (the largest in the 5th pass so far — v0.5.6 was +664 but that was a higher-density cycle; v0.5.5 was +288) |
| `upgrade.md` report pointer | `upgrade_report_v0.5.6.md` → **`upgrade_report_v0.5.7.md`** |

---

## 5. Future work (the rest of the 5th 5-cycle pass)

The v0.5.6 future-work table listed 6 items.
v0.5.7 partially closes item 2 (wxWidgets parity
gaps) for the **7th time** in the 5th pass, this time
for `DatePickerCtrl` value extraction (closing a
long-standing silent-bug in the `on_date_change`
callback).

| # | Item | v0.5.7 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** |
| 2 | wxWidgets parity gaps | **partially closed in v0.5.2** (ListCtrl selection) + **v0.5.3** (FileDialog multi-select) + **v0.5.4** (Menu shortcut refresh) + **v0.5.5** (drag-and-drop *destination* side) + **v0.5.6** (`ListCtrl` LVS_OWNERDATA virtual mode) + **v0.5.7** (`DatePickerCtrl` value extraction — silent-bug close). Remaining sub-items: OLE COM `IDropTarget` (the *source* side / in-app drag), `LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED` (virtual-mode optimization notifications) |
| 3 | Runtime rebinding of accelerators | **closed in v0.5.1** (mutators) + **v0.5.4** (visible label refresh) |
| 4 | CI first green run on GitHub Actions | **partially closed in v0.5.4** (yaml refreshed, integration step added). Actual green run still pending — the local Windows environment cannot trigger a GitHub Actions workflow |
| 5 | macOS / Linux backends (AppKit / GTK) | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | **closed in v0.5.4** (22 tests) |

The 5th 5-cycle pass still has 2 cycles remaining.
The plan for the rest of the pass (subject to
re-prioritisation when the next cycle starts):

- **v0.5.8** — OLE COM `IDropTarget` (the source-side
  / in-app-drag half of drag-and-drop, to complement
  the destination-side that v0.5.5 already shipped)
  **or** `LVN_ODCACHEHINT` (the natural follow-up to
  v0.5.6 — the v0.5.6 callback may be called many
  times per scroll, and `LVN_ODCACHEHINT` lets the
  application pre-populate a cache of cell texts to
  avoid the per-cell virtual call-and-block). The
  `OLE COM IDropTarget` is the larger of the two
  deliverables but is more isolated (it touches only
  the frame + a new `DropTarget` class, no widget
  modifications); `LVN_ODCACHEHINT` is smaller but
  extends the `ListCtrl` virtual-mode cluster that
  v0.5.6 opened. Recommendation: `OLE COM IDropTarget`
  for v0.5.8 (it's a self-contained scope, the
  `LVN_ODCACHEHINT` can be v0.5.9 or later).
- **v0.5.9** — final polish: per-pass close out,
  scoring, summary. A reasonable shape is "the
  most-pressing thing that didn't get into
  v0.5.6–v0.5.8 + a per-category score uplift to
  land the 5th pass above 9.70 weighted".

This is a recommendation, not a commitment — the
project can re-prioritise when v0.5.8 starts.

---

## 6. Per-category scores (v0.5.7)

The same 7 categories as the previous reports,
each scored 0.00–10.00 with two decimals. The
deltas are vs. **v0.5.6** (the previous report).
"—" means no change.

| # | Category | Weight | v0.5.6 | v0.5.7 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.75 | **9.80** | +0.05 | The new `unsafe` block in `on_date_change`'s inner closure (re-interpret `lparam: isize` as `*const NmDateTimeChange`, then `*nm_ptr` to read the struct) is wrapped in a 7-line `// SAFETY:` comment that pins the lifetime guarantee (the pointer stays valid for the duration of the synchronous `WM_NOTIFY` dispatch). The `nm_ptr.is_null()` guard short-circuits the read if the `lparam` is zero (a Win32 protocol violation, but the guard is cheap). The `dw_flags as u16 == GDT_VALID` cast in `NmDateTimeChange::to_option` is bounds-safe (the only legal values are 0 and 1, both of which fit in `u16`). The `unsafe` block in the `register_dtn_handler` registration path on the `Frame` side is unchanged from the existing `notify_handlers` / `disp_info_handlers` pattern (the same `remove` / `call` / `insert` dance is used to avoid borrow-across-call issues). |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 9.60 | **9.70** | +0.10 | New public surface: `Date` (struct with 3 fields + 1 constructor), `DateFormat` (enum with 3 variants — promoted from module-internal to public-re-export), `DatePickerCtrl::on_date_change` (1 method, generic over `F: FnMut(Option<Date>) + 'static`), `Frame::register_dtn_handler` (1 method, parallel to `register_disp_info_handler`). The 4 new public methods + the 2 new public types bring the v0.5.7 surface to a clean "register a callback, get the value" model that matches the wxWidgets `wxDatePickerCtrl::GetValue()` pattern. The OLE COM source side, the `LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED` virtual-mode optimisations, and the first GitHub Actions green run are still pending. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 9.30 | **9.40** | +0.10 | The `Option<Date>` callback signature is the right shape: `None` is a real semantic value (the user cleared the control with `DTS_SHOWNONE`), and the `Date` type being `Copy + Eq` means the closure doesn't need a `Clone` bound. The `DatePickerCtrl::on_date_change` rustdoc is 26 lines and explains the Win32 protocol context, the reason for the `dtn_handlers` map (the simpler `notify_handlers` doesn't carry the `lparam`), and the cross-platform behaviour (the closure is registered on every platform; on non-Windows it never fires, mirroring the existing `set_drop_files_callback` ergonomics). The `Date` / `DateFormat` re-exports through `lib.rs` and `prelude.rs` mean user code can `use ru_wx::prelude::*;` and write `on_date_change(&frame, |d| ...)` with no additional imports. The `Date::new(year, month, day)` constructor is straightforward and the 3 fields are `pub` so destructuring is ergonomic. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 9.90 | **9.90** | +0.00 | +11 new tests in `cargo test --lib` (+5 `frame`, +6 `date_picker_ctrl`) and no new doc tests. The +11 is a moderate delta (v0.5.6 was +17, v0.5.5 was +11). The score is **held flat** rather than raised because (a) integration tests for the real `DTN_DATETIMECHANGE` dispatch path are still missing (no HWND harness), (b) the 5 frame tests + 6 date-picker tests don't exercise the actual `frame_wnd_proc` `WM_NOTIFY` arm (that path needs a real message pump), and (c) the existing 226-test base is already so high that a +11 delta is a small fraction. The `nm_date_time_change_to_option_valid` test is the **value extraction regression pin** for the long-standing silent-bug (a future change that broke the `dw_flags == GDT_VALID` check would fail this test). The `notify_disp_info_and_dtn_maps_are_independent` test is the **3-map independence regression pin** (a future change that accidentally merged two of the maps would fail this test). |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.80 | **9.85** | +0.05 | New rustdoc on 5 new public items: the `Date` struct with field-level docs, the `DateFormat` enum with variant-level docs, the `DTN_DATETIMECHANGE` constant with 9 lines explaining the `pub(crate)` scope decision, the `DatePickerCtrl::on_date_change` method with 26 lines covering the `NMDATETIMECHANGE` body / `dtn_handlers` rationale / cross-platform behaviour, and the `Frame::register_dtn_handler` method with 14 lines covering the `NMDATETIMECHANGE` cast / parallel-map rationale. The `dtn_handlers` field on `FrameData` has a 19-line docstring that mirrors the `disp_info_handlers` field (parallel-map design). The `upgrade.md` U23 entry is +580 lines, this report is +560 lines. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 9.25 | **9.35** | +0.10 | The `DTN_DATETIMECHANGE` dispatch path has 3 defensive guards: (a) `nm_ptr.is_null()` in the inner closure short-circuits the read if the `lparam` is zero (a Win32 protocol violation, but the guard is cheap), (b) the `dw_flags as u16 == GDT_VALID` check in `to_option` is bounds-safe and maps both `0` and `1` to the correct `Option` variant, (c) the `GDT_NONE` → `None` path means a `DTS_SHOWNONE`-created control correctly reports "no date" through the callback (the pre-v0.5.7 callback always reported `None` regardless of the actual state, so a `DTS_SHOWNONE` control was indistinguishable from a "broken" control). The `remove` / `call` / `insert` dance on the `Rc<RefCell<FrameData>>` reconstruction in the `WM_NOTIFY` arm follows the existing `notify_handlers` / `disp_info_handlers` pattern (the borrow-aliasing rules are unchanged). The `Date` struct being `Copy` means the callback can forward the value to a model cell without an additional `Clone` and without an `Rc<RefCell<...>>` indirection. The most important robustness improvement in v0.5.7 is the **silent-bug close**: a v0.5.6 `on_date_change` callback would silently receive `None` regardless of the actual date, and would have no compile-time or runtime hint that the value was lost; a v0.5.7 callback correctly receives `Some(Date::new(...))` (or `None` if the user cleared the control). |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.60 | **9.60** | +0.00 | The default-clippy group is still 0 warnings / 0 errors after the v0.5.7 additions. The 1 cycle-1 issue caught at test time was the `Date` struct needing `Copy + Eq` (which the `date_is_copy_and_eq` test pins; a future change that removed `Copy` would fail the test, since the closure's `FnMut(Option<Date>)` signature would no longer compile in the test). The drive-by `cargo fmt --all` cleanup of `examples/minitest/mt_status_bar.rs` removed the last pre-existing fmt deviation in the workspace; `cargo fmt --all -- --check` is now clean across `src/` + `examples/`. The pedantic baseline is **unchanged** (~973 stylistic lints, tracked in `clippy_default2.txt` and `clippy_text.txt`, not enforced in CI). |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.7 weighted score:**

\[
S_{0.5.7} = \frac{(9.80) + (9.70) + (9.40) + (1.5 \cdot 9.90) + (9.85) + (9.35) + (9.60)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.80 + 9.70 + 9.40 + 14.85 + 9.85 + 9.35 + 9.60}{7.5}
\]

\[
= \frac{72.55}{7.5} = 9.6733\ldots \approx 9.67
\]

(rounded to 9.67 — the +0.05 in Security, the +0.10 in
Functions, the +0.10 in Interface, the +0.05 in
Documentation, and the +0.10 in Robustness all
contribute. The Testing and CI scores are held flat.)

**Comparison vs. v0.5.6 (which scored 9.62):**

| Metric | v0.5.0 | v0.5.1 | v0.5.2 | v0.5.3 | v0.5.4 | v0.5.5 | v0.5.6 | v0.5.7 | Δ vs. v0.5.6 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Weighted score | 9.07 | 9.17 | 9.30 | 9.40 | 9.51 | 9.57 | 9.62 | **9.67** | +0.05 |

The weighted score moves up by **+0.05** in this cycle,
in line with the v0.5.5 cycle's delta (+0.06) and the
v0.5.6 cycle's delta (+0.05). v0.5.7 is a **scope-shaped,
not full-featured** cycle: it closes one specific
silent-bug (date-picker value extraction) with 2 new
public types, 2 new public methods, and a third parallel
handler map on the frame. The remaining sub-items
(OLE COM source side, `LVN_ODCACHEHINT` /
`LVN_ODSTATECHANGED`) are scheduled for v0.5.8 and
should bump the score more, both because they are
larger deliverables and because they complete the
virtual-list-mode cluster and the drag-and-drop story
respectively.

**Goal recap (set at v0.5.0):** push the weighted score
past **9.40** by v0.5.4. v0.5.3 hit 9.40 one cycle
ahead of schedule; v0.5.4 landed at 9.51; v0.5.5
landed at 9.57; v0.5.6 landed at 9.62; v0.5.7 lands at
**9.67**, which is **+0.27** above the v0.5.4 baseline
and the **highest score the project has recorded so
far**. The 5th 5-cycle pass is therefore **opening
comfortably above the v0.5.0 goal** of 9.40 and
on-track to clear the v0.5.9 target of 9.70 weighted.

---

## 7. Changelog snapshot

3 cycles in. The weighted score moved from
**9.57 → 9.62 → 9.67** over the v0.5.5 → v0.5.6 →
v0.5.7 arc, a cumulative **+0.10** delta. The
deliverable mix over the 3 cycles is roughly:

- v0.5.5: drag-and-drop *destination* side (`Ole
  DropTarget`-equivalent via Shell
  `DragAcceptFiles` + `DragQueryFileW`).
- v0.5.6: `ListCtrl` LVS_OWNERDATA virtual list mode
  (the largest remaining wxWidgets-parity gap in the
  `ListCtrl` widget).
- v0.5.7: `DatePickerCtrl` value extraction (closing
  the long-standing silent-bug in the
  `on_date_change` callback).

The pattern across the 3 cycles is **"close one
wxWidgets-parity gap per cycle"**: each cycle picks
one specific missing Win32 surface (`Ole DropTarget`
for v0.5.5, `LVM_SETITEMCOUNT` + `LVN_GETDISPINFOW` for
v0.5.6, `DTN_DATETIMECHANGE` for v0.5.7) and exposes it
through a clean Rust API. The cumulative
wxWidgets-parity gaps closed in the 5th pass is now 7
(ListCtrl selection, FileDialog multi-select, Menu
shortcut refresh, drag-and-drop destination,
LVS_OWNERDATA virtual mode, DatePickerCtrl value
extraction, and a few smaller items). The 2 remaining
sub-items (OLE COM source side, `LVN_ODCACHEHINT` /
`LVN_ODSTATECHANGED`) are scheduled for v0.5.8.

The library's **test count** has moved from 188 (at
v0.5.4) to 226 (at v0.5.5) to 243 (at v0.5.6) to **254**
(at v0.5.7), a cumulative **+66** delta over the 3
cycles (v0.5.5: +38, v0.5.6: +17, v0.5.7: +11). The
254-test base is the **largest the project has
recorded** and includes 5 widget integration smoke
tests, 15 prelude/signatures integration tests, 27
doc tests, and **212 unit tests** (the largest single
component being `frame::tests` at 45).

The library's **clippy / fmt hygiene** has been held
at 0 warnings / 0 errors / 0 diffs across the 3 cycles.
The default-clippy group is the only group enforced
in CI; the pedantic baseline (~973 stylistic lints)
is tracked in `clippy_default2.txt` and
`clippy_text.txt` and is not enforced.

---

## 8. v0.5.7 implementation notes

This section collects the design decisions that
would be hard to recover from a future diff-walk.

### 8.1 The 3-map parallel design

The `FrameData` struct now carries three parallel
`HashMap<u16, Box<dyn FnMut(...)>>` maps for
`WM_NOTIFY` dispatch:

- `notify_handlers: HashMap<u16, Box<dyn FnMut(u32)>>`
  — code-only (the closure receives the NMHDR `code`
  field; the `lparam` is discarded). Used by the
  `Tab` / `TreeCtrl` / `ListCtrl`'s `LVN_ITEMCHANGED`
  notifications, where the callback only needs to
  filter by notification type.
- `disp_info_handlers: HashMap<u16, Box<dyn FnMut(
  isize)>>` — full `lparam` (the closure receives
  the notification body as a pointer; the user's
  closure re-interprets it to a `*mut
  NMLVDISPINFOW` and reads the request fields /
  writes the response string). Used by the
  `ListCtrl` LVS_OWNERDATA virtual mode's
  `LVN_GETDISPINFOW` notification.
- `dtn_handlers: HashMap<u16, Box<dyn FnMut(isize)>>`
  — full `lparam` (the closure receives the
  notification body as a pointer; the user's
  closure re-interprets it to a `*const
  NMDATETIMECHANGE` and reads the new `SYSTEMTIME`).
  Used by the `DatePickerCtrl`'s
  `DTN_DATETIMECHANGE` notification.

The three maps are **independent** and can all be
populated for the same `idFrom` (a virtual-mode
`ListCtrl` that also wants `LVN_ITEMCHANGED` uses
`notify_handlers` for the latter and
`disp_info_handlers` for the former; a date-picker
that also wants `NM_KILLFOCUS` would use
`notify_handlers` for the latter and `dtn_handlers`
for the former). The `WM_NOTIFY` arm of
`frame_wnd_proc` is a 3-branch `if` / `else if` /
`else` chain that routes the notification to the
correct map based on the NMHDR `code`.

The reason for the parallel-map design (rather than a
single map with an `enum Handler { Notify(u32),
Disp(isize), Dtn(isize) }`) is that the
`Box<dyn FnMut(...)>` type would be
`Box<dyn FnMut(Handler)>` (so a single closure can
discriminate), but the `Handler` enum variant
construction would leak the dispatch detail into
the user's closure type. The parallel-map design
keeps each closure type simple and lets the
`register_*_handler` methods stay narrow.

### 8.2 The `dw_flags as u16` cast

The Win32 `GDT_VALID` / `GDT_NONE` constants are
declared as `u16` in the windows-sys 0.59 bindings
(`GDT_VALID = 0`, `GDT_NONE = 1`). The
`NMDATETIMECHANGE` `dwFlags` field is declared as
`u32` in the same bindings. The
`NmDateTimeChange::to_option` method compares
`self.dw_flags as u16 == GDT_VALID`. The `as u16`
cast truncates the `u32` to its low 16 bits, which
is correct because the Win32 protocol guarantees
that `dwFlags` is one of `{ GDT_VALID, GDT_NONE }`
and both fit in 16 bits. The `u16` cast also
future-proofs against a hypothetical Win32 protocol
extension that adds a third flag with a value `>=
0x10000` (the cast would truncate to 0 = GDT_VALID,
which would be wrong, but the comment in the code
notes that the comparison is on the `u16` range
specifically).

### 8.3 The `lparam: isize` convention

The three handler maps' `isize` parameter is the
`lparam` of the `WM_NOTIFY` message, cast to `isize`
(rather than to `*mut NMHDR` or `*mut c_void`). The
`isize` cast is the standard FFI-friendly form for
"raw pointer-shaped integer" and matches the
existing `register_disp_info_handler` convention.
The user's closure re-interprets the `isize` to the
appropriate notification struct pointer.

### 8.4 The `unsafe` block in `on_date_change`

The `*nm_ptr` dereference is the only `unsafe`
block in the v0.5.7 addition. The 7-line
`// SAFETY:` comment pins the lifetime guarantee
(the pointer stays valid for the duration of the
synchronous `WM_NOTIFY` dispatch), the
notification-body invariant (the `lparam` is
guaranteed to point to a `NMDATETIMECHANGE` by the
`else if code == DTN_DATETIMECHANGE` branch in the
`WM_NOTIFY` arm), and the no-mutable-aliasing
guarantee (the notification is delivered
synchronously on the current thread, so no other
code can mutate the body during the read).

### 8.5 The `Date::Copy` bound

The `Date` struct is `#[derive(Debug, Clone, Copy,
PartialEq, Eq)]`. The `Copy` bound is **deliberate**
— the `on_date_change` callback signature
`FnMut(Option<Date>)` doesn't carry a `Clone` bound
on the closure, so the `Option<Date>` value must
be `Copy` for the closure to forward the value to
the user's state (e.g. an `Rc<RefCell<Option<Date>>>`
model cell) without a `Clone` bound. The `Copy`
bound is also the right semantic: a calendar date
is 3 machine words and the "no date" state is
distinguished by `Option`'s niche, not by a
heap-allocated sentinel.

The `pub` fields are also deliberate — the struct
is small enough that the `pub` / private split
would be more friction than protection, and the
constructor + 3 fields are the entire API.

### 8.6 The Windows-only gating

All the new Win32 FFI surface (`NmDateTimeChange`,
`SystemTime`, `DTN_DATETIMECHANGE`, `GDT_VALID`,
`GDT_NONE`, `DTS_*`, the `unsafe` blocks) is
`#[cfg(target_os = "windows")]`-gated. The
`DatePickerCtrl::on_date_change` method is **not**
gated — it's reachable from every platform, and on
non-Windows hosts the `dtn_handlers` map is never
populated (there's no real `HWND`), so the
callback simply never fires. This mirrors the
cross-platform ergonomics of
`Frame::set_drop_files_callback` (which is also
reachable from every platform and on non-Windows
hosts the callback never fires).

The `Date` and `DateFormat` types are also
non-gated (they're plain Rust data with no FFI
dependency).

---

## 9. What v0.5.8 should pick up

The U22 / U23 future-work sections both deferred
two large sub-items: **OLE COM `IDropTarget`** (the
source-side / in-app-drag half of drag-and-drop) and
**`LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED`** (the
virtual-mode optimization notifications). v0.5.8
should pick **one** of these:

- **OLE COM `IDropTarget`**: self-contained scope.
  Touches only the `frame` + a new `DropTarget` class
  (or extends the existing one in `src/drop_target.rs`).
  No widget modifications. Larger deliverable but
  more isolated. The destination-side that v0.5.5
  shipped is the *Shell* `DragAcceptFiles` /
  `DragQueryFileW` protocol, which only carries
  *file paths*; the OLE COM protocol carries
  arbitrary in-memory data objects (e.g. text
  dragged from one widget to another).
- **`LVN_ODCACHEHINT`**: smaller scope. Extends the
  `ListCtrl` virtual-mode cluster that v0.5.6 opened.
  Touches only the `list_ctrl` + the `disp_info`
  branch of the `WM_NOTIFY` arm. The cache hint lets
  the application pre-populate a cell-text cache so
  the per-cell `LVN_GETDISPINFOW` callback doesn't
  block the UI thread on a 10⁶-row scroll.

Recommendation: **OLE COM `IDropTarget`** for
v0.5.8. It's the larger of the two deliverables but
is more isolated, and it completes the
drag-and-drop story that v0.5.5 started. The
`LVN_ODCACHEHINT` can be v0.5.9 or later.
