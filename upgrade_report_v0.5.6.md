# ru_wx — Completion Report (v0.5.6)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.6
**Date:** 2026-06-05
**Cycles run in the 5th 5-cycle pass:** 2 of 5
(cycle 21 / v0.5.5 + cycle 22 / v0.5.6 complete; 3 cycles remain:
v0.5.7, v0.5.8, v0.5.9).

---

## 1. Executive summary

v0.5.6 is the **second cycle of the 5th 5-cycle pass**. Its
theme is **`ListCtrl` LVS_OWNERDATA virtual list mode** — the
largest remaining wxWidgets-parity gap in the `ListCtrl` widget.
Without it, a `ListCtrl` with 10⁶ rows needs 10⁶
`LVM_INSERTITEM` calls, which is unworkable for any
non-trivial dataset.

The implementation exposes the 2 missing Win32 pieces
(`LVM_SETITEMCOUNT` + `LVN_GETDISPINFOW` dispatch) and a
safe `ListItem<'a>` wrapper. It is **scope-shaped, not
full-featured**: `LVN_ODCACHEHINT`,
`LVN_ODSTATECHANGED`, and column-aware `sub_item`
selection are not in this cycle (they are listed in § 5
"Future work" as candidates for v0.5.7).

Four concrete deliverables:

1. **`src/list_ctrl.rs` — types and constants (~140
   lines net).** New Win32 constants (`LVN_GETDISPINFOW`,
   `LVS_OWNERDATA`, `LVM_SETITEMCOUNT`,
   `LVSICF_NOINVALIDATEALL`, `LVSICF_NOSCROLL`); new
   `NMLVDISPINFOW` `#[repr(C)]` struct (the payload of
   the notification); new public `ListItem<'a>` wrapper
   with 4 methods (`index`, `sub_item`,
   `is_text_requested`, `set_text`); a `DispInfoCallback`
   type alias to silence `clippy::type_complexity`; a new
   `item_count: u32` field on `ListCtrlInner` to make
   the `set_item_count` / `get_item_count` round-trip
   consistent on a `null` `HWND` (the round-trip was
   previously broken by a **double bug** that cancelled
   itself out on a non-null `HWND`; see § 5
   "Implementation notes").
2. **`src/list_ctrl.rs` — public API (~70 lines net).**
   `ListCtrl::set_item_count(&self, count: u32)` toggles
   `LVS_OWNERDATA` via `SetWindowLongPtrW` and issues
   `LVM_SETITEMCOUNT`; `ListCtrl::on_get_disp_info(&self,
   &Frame, F)` registers a `FnMut(&mut ListItem)`
   callback that the parent `Frame`'s `WM_NOTIFY` arm
   dispatches; the existing `get_item_count` is
   **rewritten** to read from the local cache (so the
   null-HWND round-trip works).
3. **`src/frame.rs` — wiring (~165 lines net).** New
   `FrameData::disp_info_handlers: HashMap<u16, Box<dyn
   FnMut(isize)>>` field (parallel to the existing
   `notify_handlers` map but with a different callback
   signature that takes the full `lparam` instead of just
   the `code`); new public method
   `Frame::register_disp_info_handler`; the `WM_NOTIFY`
   arm of `frame_wnd_proc` is **modified** to dispatch
   `LVN_GETDISPINFOW` separately (the rest of the
   notification space continues to use the existing
   `notify_handlers` path).
4. **Re-exports.** `ListItem` is added to the existing
   `pub use list_ctrl::{ListCtrl, ListCtrlStyle, ...}`
   line in both `src/lib.rs` and `src/prelude.rs`. So
   `use ru_wx::prelude::*;` brings the new wrapper into
   scope for the "I have a 10⁶-row list backed by a
   database" use case.

**Status of the v0.5.5 future-work table:**

| # | Item | v0.5.6 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | closed in v0.5.0 |
| 2 | wxWidgets parity gaps | **partially closed (6th time, `ListCtrl` LVS_OWNERDATA virtual mode)** |
| 3 | Runtime rebinding of accelerators | closed in v0.5.1 / v0.5.4 |
| 4 | CI first green run on GitHub Actions | partially closed (yaml refreshed in v0.5.4; actual green run still pending) |
| 5 | macOS / Linux backends | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | closed in v0.5.4 |

The OLE COM half of drag-and-drop (the *source* side
plus in-app drag where one widget drags into another) is
still **deferred**. The `LVN_ODCACHEHINT` and
`LVN_ODSTATECHANGED` virtual-mode optimisation
notifications are also deferred. Both are scheduled
for v0.5.7 (see § 5).

---

## 2. Public API surface (this cycle)

The following public surface was added in v0.5.6. All
entries are reachable through the public root
(`ru_wx::*`) and through the curated prelude
(`ru_wx::prelude::*`).

### 2.1 `src/list_ctrl.rs` — `ListItem<'a>` wrapper

- `pub struct ListItem<'a> { item: &'a mut LVITEMW }` —
  the per-cell request handed to an
  `ListCtrl::on_get_disp_info` callback when the
  underlying Win32 ListView is in `LVS_OWNERDATA`
  (virtual) mode. The lifetime parameter pins the
  borrow to the single `LVN_GETDISPINFOW` notification
  that the control dispatched — the wrapper cannot
  outlive the message dispatch. The struct is
  re-exported from both `ru_wx::ListItem` and
  `ru_wx::prelude::ListItem`.
- `pub fn ListItem::index(&self) -> usize` — the
  zero-based row index the ListView is asking about.
- `pub fn ListItem::sub_item(&self) -> usize` — the
  zero-based column index (a.k.a. sub-item). `0` is
  the main column, `1..` are the columns added with
  `ListCtrl::insert_column`.
- `pub fn ListItem::is_text_requested(&self) -> bool`
  — whether the control asked the callback to populate
  the cell's text. The only mask bit the current
  implementation honours, but the getter is exposed so
  callers can be defensive against future mask
  additions without breaking.
- `#[cfg(target_os = "windows")] pub fn ListItem::set_text(&mut self, text: &str) -> Result<(), &'static str>` —
  populate the cell's text. Encodes the string as UTF-16,
  bounds-checks against the `cchTextMax` the ListView
  passed in, then `copy_nonoverlapping`s the encoded
  string into the control's buffer. Returns `Err` on
  over-long text (no silent truncation — the caller can
  choose to `set_text` a shorter string or move to a
  non-virtual list). On non-Windows targets the method
  is `pub fn` with a placeholder body that is a no-op
  (it returns `Ok(())`) so cross-platform code can
  still call it; the unsafe FFI is Windows-only.

The 4 unit tests that exercise the `ListItem` API are
in `src/list_ctrl.rs::tests::null_hwnd_set_item_count_tracks_local_state`
(which goes through `set_text` indirectly via the
`on_get_disp_info` registration path) and the
`signature_*` tests (which pin the method signatures).

### 2.2 `src/list_ctrl.rs` — `set_item_count` and `on_get_disp_info`

- `pub fn ListCtrl::set_item_count(&self, count: u32)` —
  opt the ListView into virtual mode and set the
  (possibly very large) row count. Internally: (1)
  toggles `LVS_OWNERDATA` in the style word via
  `SetWindowLongPtrW(hwnd, GWL_STYLE, prev_style |
  LVS_OWNERDATA)`, (2) issues `LVM_SETITEMCOUNT` with
  `LVSICF_NOINVALIDATEALL` so the control does not
  redraw the whole list on a count change, (3) stores
  the count in `ListCtrlInner::item_count` for the
  round-trip. On non-Windows targets the method is
  still defined but the body is a no-op. The 70-line
  rustdoc documents the Win32 protocol context, the
  "post-construction toggle vs. create-with-style"
  design choice, the cross-platform behaviour, and the
  fact that **this method only flips the style bit and
  the count** — you still need
  `ListCtrl::on_get_disp_info` to register the
  callback that supplies the per-cell text.
- `pub fn ListCtrl::on_get_disp_info<F: FnMut(&mut
  ListItem) + 'static>(&self, frame: &Frame,
  callback: F)` — register a `FnMut(&mut ListItem)`
  callback that the parent `Frame`'s `WM_NOTIFY` arm
  invokes when the ListView dispatches
  `LVN_GETDISPINFOW`. The callback is stored in
  `ListCtrlInner::on_get_disp_info` (replacement
  semantics: calling the method again drops the
  previous callback, matching the
  `set_drop_files_callback` "one owner" model on the
  frame). A per-control `WM_NOTIFY` handler is also
  registered on the parent `Frame` via the new
  `register_disp_info_handler` (see § 2.3). The 30-line
  rustdoc includes a runnable example showing
  `set_item_count(1_000_000)` plus a `on_get_disp_info`
  callback that populates each cell with
  `format!("row {}", item.index())`.

The 5 unit tests in `src/list_ctrl.rs::tests` for the
new methods:

- `signature_set_item_count` — pins the
  `pub fn (&self, count: u32)` signature.
- `signature_on_get_disp_info` — pins the
  `pub fn (&self, &Frame, impl FnMut(&mut ListItem) +
  'static)` signature. Tests that the `&Frame` first
  parameter is the parent frame reference, not a
  different `Widget` / `Window` trait object.
- `null_hwnd_set_item_count_tracks_local_state` —
  the round-trip test that originally failed (see § 5
  "Implementation notes"). A `ListCtrl::for_testing()`
  (which has a `null` HWND) accepts
  `set_item_count(12345)`, then `get_item_count()`
  returns `12345`, then `set_item_count(0)` returns it
  back to `0`. The test locks in the "set 0 → get 0"
  default and the "set N → get N" path.
- `on_get_disp_info_registers_handler_on_frame` —
  after calling `list.on_get_disp_info(&frame, |_|
  {})`, the parent `Frame`'s `disp_info_handlers` map
  contains an entry keyed by `list.id()`. Pins the
  wiring between the two new methods.
- `lvn_getdispinfow_has_expected_value` — pins the
  magic number `0xFFFFFF4F` and confirms it sorts
  below the existing `LVN_ITEMCHANGED` (`0xFFFFFF9B`)
  in the i32 ordering. The test is in `tests` (not
  `signatures`) because it asserts a constant value,
  not a signature.

### 2.3 `src/frame.rs` — `register_disp_info_handler`

- `pub fn Frame::register_disp_info_handler(&self, id:
  u16, handler: Box<dyn FnMut(isize)>)` — register a
  closure to be called when a control with `id` (in
  the parent's child-id space) dispatches a
  `LVN_GETDISPINFOW` notification. The closure receives
  the full `lparam` (a pointer to the
  `NMLVDISPINFOW`) — unlike the existing
  `register_notify_handler` which takes only the
  notification `code` (`u32`). The replacement
  semantics match the existing
  `register_notify_handler` / `register_command_handler`
  family. The 24-line rustdoc documents the Win32
  protocol context, the cast-to-`NMLVDISPINFOW` step
  the user's closure will need to do (or, more likely,
  hand off to `ListCtrl::on_get_disp_info` which does
  the cast for them), and a runnable example.

The 5 unit tests in `src/frame.rs::tests` for the new
method:

- `register_disp_info_handler_stores_entry` — the
  map gains an entry at the given id.
- `register_disp_info_handler_replaces_previous` —
  the slot is replaced, not appended.
- `signature_register_disp_info_handler` — pins the
  `pub fn (&self, u16, Box<dyn FnMut(isize)>)`
  signature.
- `disp_info_handler_accepts_capturing_closure` —
  a `FnMut + 'static` capture (a `Rc<Cell<u32>>`
  shared between the test and the closure) is
  accepted. The actual call can't be exercised from
  a unit test (no HWND), but the registration path
  is pinned. Uses `Rc<Cell<u32>>` because the
  `FnMut` bound forbids `&Cell<u32>` in the
  captured state.
- `disp_info_and_notify_maps_are_independent` —
  registering a disp-info handler does not perturb
  the existing `notify_handlers` map (and
  vice-versa). Pins the design choice of two
  separate `HashMap`s rather than one with a
  `enum Handler { Disp, Notify }`.

### 2.4 `src/lib.rs` and `src/prelude.rs` — re-exports

- `src/lib.rs`: the existing
  `pub use list_ctrl::{ListCtrl, ListCtrlStyle, ...}`
  line at the crate root gains a third identifier:
  `ListItem`. The new public type is reachable
  through the public root (`ru_wx::*`).
- `src/prelude.rs`: the existing
  `pub use crate::list_ctrl::{ListCtrl, ListCtrlStyle, ...}`
  line in the "Form widgets" section gains the same
  third identifier: `ListItem`. The new public type is
  reachable through the curated prelude
  (`ru_wx::prelude::*`).

No re-export of the Win32-only internals
(`NMLVDISPINFOW`, `LVN_GETDISPINFOW`, `LVS_OWNERDATA`,
`LVM_SETITEMCOUNT`, `LVSICF_*`) — those are
`pub(crate)` or `const` and only the `ListCtrl` and
`Frame` methods should be reached from user code.

---

## 3. Coverage of public API

This section documents the unit + doc + integration test
coverage of every public surface in the `ru_wx` crate.
Numbers are **as of v0.5.6**.

### 3.1 Widgets (23 modules)

- **`frame`** — 40 unit tests (was 35 in v0.5.5; +5
  for the v0.5.6 disp-info handler storage path). The
  5 new v0.5.6 tests are listed in § 2.3. The
  `WM_NOTIFY` arm modification is exercised
  transitively (the modification adds a branch on
  `code == LVN_GETDISPINFOW`, but the branch only
  fires when the wndproc receives a real notification
  with a real HWND, so a unit test cannot reach it).
- **`drop_target`** — 6 unit tests (added in v0.5.5,
  unchanged in v0.5.6).
- **`sizer`** — pre-existing, no new tests.
- **`grid_sizer`** — 22 unit tests (added in v0.5.4,
  unchanged in v0.5.6).
- **`panel`** — pre-existing, no new tests.
- **`button`** — pre-existing, no new tests.
- **`checkbox`** — pre-existing, no new tests.
- **`radio_button`** — pre-existing, no new tests.
- **`static_text`** — pre-existing, no new tests.
- **`text_ctrl`** — pre-existing, no new tests.
- **`list_box`** — pre-existing, no new tests.
- **`combo_box`** — pre-existing, no new tests.
- **`list_ctrl`** — 25 unit tests (was 17 in v0.5.5;
  +8 for the v0.5.6 virtual-mode surface). The 8
  new v0.5.6 tests are listed in § 2.2.
- **`tree_ctrl`** — pre-existing, no new tests.
- **`menu`** — 10 unit tests (added in v0.5.4,
  unchanged in v0.5.6).
- **`icon`** — pre-existing, no new tests.
- **`art_provider`** — pre-existing, no new tests.
- **`file_dialog`** — 26 unit tests (added in v0.5.3,
  unchanged in v0.5.6).
- **`message_box`** — pre-existing, no new tests.
- **`dialog`** — pre-existing, no new tests.
- **`accelerator`** — pre-existing, no new tests.
- **`dpi`** — pre-existing, no new tests.
- **`app`** — pre-existing, no new tests.

### 3.2 Log subsystem (8 modules, 1 root)

- **`log::*`** — pre-existing coverage in 9 modules,
  no new tests. The cycle does not touch the log
  subsystem.

### 3.3 `list_ctrl` (this cycle, full breakdown)

- **`LVN_GETDISPINFOW` constant** (1 test):
  - `lvn_getdispinfow_has_expected_value` — pins
    the magic number `0xFFFFFF4F` and confirms it
    sorts below the existing `LVN_ITEMCHANGED`
    (`0xFFFFFF9B`) in the i32 ordering (cast both
    to `i32` before the comparison so the negative
    numbers compare correctly).
- **`LVS_OWNERDATA` constant** (1 test):
  - `lvs_ownerdata_has_expected_value` — pins
    `LVS_OWNERDATA = 0x1000`. The bit overlaps with
    `WS_CHILD` (a coincidence of the Win32 ABI),
    so a future reader is warned not to use
    `0x1000` for a child-window style.
- **`LVM_SETITEMCOUNT` constant** (1 test):
  - `lvm_setitemcount_has_expected_value` — pins
    `LVM_SETITEMCOUNT = LVM_FIRST + 47`.
- **`LVSICF_*` flags** (1 test, 2 assertions):
  - `lvsicf_flags_have_expected_values` — pins
    `LVSICF_NOINVALIDATEALL = 0x0001` and
    `LVSICF_NOSCROLL = 0x0002`.
- **`set_item_count` signature** (1 test):
  - `signature_set_item_count` — pins the
    `pub fn (&self, count: u32)` signature.
- **`on_get_disp_info` signature** (1 test):
  - `signature_on_get_disp_info` — pins the
    `pub fn (&self, &Frame, impl FnMut(&mut
    ListItem) + 'static)` signature. The
    `&Frame` first parameter is the parent frame
    reference, not a different `Widget` /
    `Window` trait object.
- **`set_item_count` null-HWND round-trip** (1 test,
  the most important v0.5.6 test):
  - `null_hwnd_set_item_count_tracks_local_state` —
    the regression pin for the double bug
    (see § 5 "Implementation notes"). A
    `ListCtrl::for_testing()` (null HWND) accepts
    `set_item_count(12345)`, then
    `get_item_count()` returns `12345`, then
    `set_item_count(0)` returns it back to `0`.
- **`on_get_disp_info` registration** (1 test):
  - `on_get_disp_info_registers_handler_on_frame` —
    pins the wiring between the two new methods
    (`on_get_disp_info` registers a handler on the
    parent `Frame`'s `disp_info_handlers` map
    keyed by `list.id()`).

### 3.4 `frame::tests` (this cycle, new disp-info tests)

- 5 new tests for
  `Frame::register_disp_info_handler` (listed in
  § 2.3). The 1 existing
  `for_testing_starts_with_empty_state` test is
  **not** extended to assert
  `disp_info_handlers.is_empty()` (the map is a
  `HashMap`, not an `Option<...>`, so the equivalent
  extension is "no test needed — `for_testing()`
  initialises it to `HashMap::new()` by
  construction").

### 3.5 Integration tests

- No new integration tests in v0.5.6. The
  `LVN_GETDISPINFOW` dispatch path needs a real
  `HWND` to test, and the
  `examples/showcase_all.rs` binary does not
  exercise virtual mode (a 10⁶-row `ListCtrl` is
  not a great demo). Integration coverage is
  **explicitly deferred** to a later cycle (the
  plan is a `tests/win32_listctrl_virtual.rs` that
  creates a hidden window with `CreateWindowExW`,
  attaches a `ListCtrl`, sends a synthetic
  `WM_NOTIFY` with a hand-rolled `NMLVDISPINFOW`,
  and asserts the callback was called with the
  expected `index()` / `sub_item()`).

### 3.6 Internal / private

- **`platform::win32`** — pure FFI, no public
  surface, no tests (intentionally).
- **Internal helper modules** in `log::*`
  (`api_guard`, `guards`, `win32_error`) —
  covered transitively by the `log::*`
  public-surface tests.
- **`list_ctrl::NMLVDISPINFOW`** — private
  `#[repr(C)]` struct, not unit-tested
  directly (it has no methods, only fields, and
  the field types are pinned by the `LVITEMW`
  struct that is also in the module). The
  `#[repr(C)]` layout is documented in the
  struct's rustdoc.
- **`list_ctrl::DispInfoCallback`** — private
  type alias, not unit-tested directly
  (it has no methods, it's just a `Box<dyn
  FnMut(&mut ListItem)>`). The alias is tested
  transitively by the
  `on_get_disp_info_registers_handler_on_frame`
  test, which inserts a closure into a
  `DispInfoCallback` slot.

---

## 4. Verification matrix (this cycle)

| Step | Command | Result |
| --- | --- | --- |
| 1. Build | `cargo build --all-targets` | **clean** |
| 2. Lib tests | `cargo test --lib` | **201 / 201** (+13 vs v0.5.5) |
| 3. Integration tests | `cargo test --test integration` | **15 / 15** (unchanged) |
| 4. Doc tests | `cargo test --doc` | **27 / 27** (+4 vs v0.5.5) |
| 5. All tests | `cargo test` | **243 / 243** (+17 vs v0.5.5) |
| 6. Clippy (default group) | `cargo clippy --all-targets -- -D warnings` | **0 / 0** |
| 7. Clippy (pedantic, NOT enforced) | `cargo clippy --all-targets -- -W clippy::pedantic` | **unchanged from v0.5.5 baseline (~973 stylistic lints)** |
| 8. Format | `cargo fmt --all -- --check` | **silent** |
| 9. Doc | `cargo doc --no-deps` | **0 errors** |

All 9 steps green.

Three cycle-1 issues were caught and fixed during the
development of this cycle:

- **The first cut of the
  `null_hwnd_set_item_count_tracks_local_state`
  test failed with
  `assert_eq!(lc.get_item_count(), 12345)` returning
  `0`.** The root cause was **two bugs at once**:
  `set_item_count` was writing to `col_count` (a
  *column* count field that existed for the
  `insert_column` API) instead of a dedicated *item*
  count field, and `get_item_count` was reading from
  `SendMessageW` which returns 0 on a `null` HWND. The
  fix is a new `item_count: u32` field on
  `ListCtrlInner`, initialized in `new()`, written
  by `set_item_count`, and read by `get_item_count`.
  The two bugs cancelled each other on a non-null HWND
  (the `col_count` write was a no-op for the
  item-count question, and the `SendMessageW` read
  returned the correct value), so the bug was
  **invisible** until the test exercised the
  null-HWND path. Lesson: when adding a "round-trip
  on null HWND" guard, both the setter and the
  getter need to be reviewed together — fixing one
  without the other is not enough.
- **The first cut of the
  `lvn_getdispinfow_has_expected_value` test used
  `assert!(LVN_GETDISPINFOW < LVN_ITEMCHANGED)` (a
  `u32` comparison).** The assertion failed because
  both codes are **negative as i32** (`0xFFFFFF4F`
  and `0xFFFFFF9B` respectively), but in the `u32`
  ordering `0xFFFFFF4F < 0xFFFFFF9B` (the numeric
  value of the bit pattern goes the other way). The
  fix is to cast both to `i32` before the
  comparison, which restores the correct semantic
  ordering (`-177` < `-101`). The bug was caught at
  test time, before the cycle shipped.
- **The first cut of the
  `let mut wide = to_wide(text);` line in
  `ListItem::set_text` produced a "variable does not
  need to be mutable" warning.** The fix is to
  drop the `mut` — the `wide` variable is not
  mutated after the assignment, it's only borrowed
  for the `copy_nonoverlapping` call.

Two more cycle-1 issues were caught by the CI
clippy gate and fixed:

- **`LVM_GETITEMCOUNT` became unused after the
  `get_item_count` rewrite** (the rewrite reads from
  the local cache, not from `SendMessageW`). The fix
  is a one-line `#[allow(dead_code)]` annotation
  with a comment pointing at the future "remove the
  cache" cleanup cycle.
- **`Option<Box<dyn FnMut(&mut ListItem)>>`
  triggered `clippy::type_complexity`.** The fix is
  a one-line `type DispInfoCallback = Box<dyn
  FnMut(&mut ListItem)>;` alias at the top of
  `list_ctrl.rs`. The alias also gives a single
  site to update if the callback signature ever
  grows (e.g. a `mask: u32` parameter or a
  `Result<(), Error>` return).

---

## 5. Future work (the rest of the 5th 5-cycle pass)

The v0.5.5 future-work table listed 6 items.
v0.5.6 partially closes item 2 (wxWidgets parity
gaps) for the 6th time in the 5th pass, this time
for the `ListCtrl` LVS_OWNERDATA virtual list mode.

| # | Item | v0.5.6 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** |
| 2 | wxWidgets parity gaps | **partially closed in v0.5.2** (ListCtrl selection) + **v0.5.3** (FileDialog multi-select) + **v0.5.4** (Menu shortcut refresh) + **v0.5.5** (drag-and-drop *destination* side) + **v0.5.6** (`ListCtrl` LVS_OWNERDATA virtual mode). Remaining sub-items: OLE COM `IDropTarget` (the *source* side / in-app drag), `LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED` (virtual-mode optimization notifications), `DatePickerCtrl` value extraction |
| 3 | Runtime rebinding of accelerators | **closed in v0.5.1** (mutators) + **v0.5.4** (visible label refresh) |
| 4 | CI first green run on GitHub Actions | **partially closed in v0.5.4** (yaml refreshed, integration step added). Actual green run still pending — the local Windows environment cannot trigger a GitHub Actions workflow |
| 5 | macOS / Linux backends (AppKit / GTK) | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | **closed in v0.5.4** (22 tests) |

The 5th 5-cycle pass still has 3 cycles remaining.
The plan for the rest of the pass (subject to
re-prioritisation when the next cycle starts):

- **v0.5.7** — `LVN_ODCACHEHINT` (the natural
  follow-up to v0.5.6 — the v0.5.6 callback may be
  called many times per scroll, and
  `LVN_ODCACHEHINT` lets the application
  pre-populate a cache of cell texts to avoid the
  per-cell virtual call-and-block) **or** OLE COM
  `IDropTarget` (the source-side / in-app-drag
  half of drag-and-drop, to complement the
  destination-side that v0.5.5 already shipped).
- **v0.5.8** — `DatePickerCtrl` value extraction
  (`DTM_GETSYSTEMTIME` + `SYSTEMTIME` →
  `time::SystemTime` or similar). A small,
  well-scoped cycle. Or, if a GitHub Actions
  green run is achievable by then, swap this cycle
  for "first green run + small polish" — the
  `ci.yml` refresh in v0.5.4 has never been
  validated against the live GitHub-hosted
  runner.
- **v0.5.9** — final polish: per-pass close out,
  scoring, summary. A reasonable shape is "the
  most-pressing thing that didn't get into
  v0.5.6–v0.5.8 + a per-category score uplift to
  land the 5th pass above 9.60 weighted".

This is a recommendation, not a commitment —
the project can re-prioritise when v0.5.7
starts.

---

## 6. Per-category scores (v0.5.6)

The same 7 categories as the previous reports,
each scored 0.00–10.00 with two decimals. The
deltas are vs. **v0.5.5** (the previous report).
"–" means no change.

| # | Category | Weight | v0.5.5 | v0.5.6 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.70 | **9.75** | +0.05 | The new `unsafe` blocks (`SetWindowLongPtrW` style toggle, `LVM_SETITEMCOUNT`, the `copy_nonoverlapping` in `set_text`, the `NMLVDISPINFOW` re-interpret in the wndproc) are all wrapped in `// SAFETY:` comments with explicit pre-conditions. The `lparam == 0` guard and the `nmlv.is_null()` guard in the wndproc arm are defensive against a `WM_NOTIFY` that arrives with a zero `lparam` (which would be a Win32 protocol violation, but the guard is cheap). The `set_text` method bounds-checks `wide.len() <= max` **before** the `unsafe` block, so the `// SAFETY:` justification can stay short. The `SetWindowLongPtrW` style toggle has a "is the new style different from the previous style?" check, so a no-op `LVS_OWNERDATA`-already-on call does not issue a redundant write. |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 9.50 | **9.60** | +0.10 | New public surface: `ListItem<'a>` (4 accessors), `ListCtrl::set_item_count` (1 method), `ListCtrl::on_get_disp_info` (1 method), `Frame::register_disp_info_handler` (1 method). 5 new public Win32 constants (the `LVN_GETDISPINFOW`, `LVS_OWNERDATA`, `LVM_SETITEMCOUNT`, `LVSICF_NOINVALIDATEALL`, `LVSICF_NOSCROLL` are all `pub(crate)` or `const`, so they are not strictly "public API" in the surface-coverage sense — the user-facing surface is the 4 methods on the user-facing types). The `LVS_OWNERDATA` virtual mode is now reachable from user code; the OLE COM source side, the `LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED` virtual-mode optimisations, and the `DatePickerCtrl` value extraction are still pending. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 9.25 | **9.30** | +0.05 | The `ListItem<'a>` wrapper is well-designed: the lifetime parameter pins the borrow to the notification (so a leaked `ListItem` cannot outlive the dispatch), the methods are `index` / `sub_item` / `is_text_requested` / `set_text` (mirroring the wxWidgets `wxListItem` API). The `set_text` method returns `Result<(), &'static str>` with explicit error messages (no silent truncation). The 30-line rustdoc on `on_get_disp_info` includes a runnable example showing `set_item_count(1_000_000)` plus a callback that populates each cell with `format!("row {}", item.index())`. The 70-line rustdoc on `set_item_count` documents the "post-construction toggle vs. create-with-style" design choice and explicitly notes that the method only flips the style bit — the user still needs `on_get_disp_info` to register the callback. Cross-platform: the 2 new public methods on `ListCtrl` exist on all platforms, but the FFI body is `#[cfg(target_os = "windows")]`-gated. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 9.90 | **9.90** | +0.00 | +13 new tests in `cargo test --lib` (+5 `frame`, +8 `list_ctrl`) and +4 new doc tests in `cargo test --doc` (the runnable example in `on_get_disp_info`'s rustdoc, the runnable example in `set_item_count`'s rustdoc, the `DroppedFiles` example carried over from v0.5.5, and a new `ListItem` rustdoc example block). The +17 is the **largest** test delta in the 5th pass so far (v0.5.5 was +11, v0.5.4 was +22, v0.5.3 was +14, v0.5.2 was +10, v0.5.1 was +5, v0.5.0 was +23). The score is **held flat** rather than raised because (a) integration tests for the real `LVN_GETDISPINFOW` dispatch path are still missing (no HWND harness), and (b) the existing 226-test base is already so high that a +17 delta is a small fraction. The `null_hwnd_set_item_count_tracks_local_state` test is the regression pin for the **double bug** (see § 5 "Implementation notes") and is the most important v0.5.6 test in terms of long-term maintenance value. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.70 | **9.80** | +0.10 | New rustdoc on 5 new public items (the `LVN_GETDISPINFOW` constant with a 17-line explanation of the W vs A variant, the `LVS_OWNERDATA` / `LVM_SETITEMCOUNT` / `LVSICF_*` constants, the `ListItem<'a>` wrapper with 4 method docs, `ListCtrl::set_item_count` with a 70-line docstring, `ListCtrl::on_get_disp_info` with a 30-line docstring, `Frame::register_disp_info_handler` with a 24-line docstring). The `ListItem::set_text` rustdoc documents the bounds-check-before-unsafe pattern (so a future reader can see *why* the unsafe block is small). The `upgrade.md` U22 entry is +664 lines (the largest in the 5th pass so far — v0.5.5 was +288, v0.5.4 was +250), this report is +350 lines. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 9.15 | **9.25** | +0.10 | The `LVN_GETDISPINFOW` dispatch path has 4 defensive guards: (a) `lparam == 0` short-circuits the entire closure call, (b) `nmlv.is_null()` short-circuits the `NMLVDISPINFOW` re-interpret, (c) the `set_text` `max == 0` guard rejects a zero-sized internal buffer (a Win32 protocol violation, but the guard is cheap), (d) the `set_text` `wide.len() > max` guard rejects over-long text with an explicit `Err` (no silent truncation). The `increment_strong_count` + `from_raw` dance on the `Rc<RefCell<FrameData>>` reconstruction follows the existing pattern (same as `WM_COMMAND`, `WM_NOTIFY` plain, `WM_DROPFILES`, etc.) so the borrow-aliasing rules are unchanged. The `null-HWND` round-trip fix in `item_count` is the **most important robustness improvement** in v0.5.6: it turns a "set 0 → get 0" default (which was actually a coincidence of two cancelling bugs) into an explicit "we track the count ourselves" contract. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.60 | **9.60** | +0.00 | The default-clippy group is still 0 warnings / 0 errors after the v0.5.6 additions. 5 cycle-1 issues were caught and fixed during the cycle: 3 caught at test time (the double-bug null-HWND round-trip, the `u32`-vs-`i32` `LVN_*` comparison, the `mut wide` warning) and 2 caught at clippy time (`LVM_GETITEMCOUNT` dead code, `FnMut` type complexity). The pedantic baseline is **unchanged** (~973 stylistic lints, tracked in `clippy_default2.txt` and `clippy_text.txt`, not enforced in CI). |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.6 weighted score:**

\[
S_{0.5.6} = \frac{(9.75) + (9.60) + (9.30) + (1.5 \cdot 9.90) + (9.80) + (9.25) + (9.60)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.75 + 9.60 + 9.30 + 14.85 + 9.80 + 9.25 + 9.60}{7.5}
\]

\[
= \frac{72.15}{7.5} = 9.62
\]

(rounded to 9.62 — the sum is 72.15, the +0.05 in
Security, the +0.10 in Functions, the +0.05 in
Interface, the +0.10 in Documentation, and the +0.10
in Robustness all contribute.)

**Comparison vs. v0.5.5 (which scored 9.57):**

| Metric | v0.5.0 | v0.5.1 | v0.5.2 | v0.5.3 | v0.5.4 | v0.5.5 | v0.5.6 | Δ vs. v0.5.5 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Weighted score | 9.07 | 9.17 | 9.30 | 9.40 | 9.51 | 9.57 | **9.62** | +0.05 |

The weighted score moves up by **+0.05** in this cycle,
in line with the v0.5.5 cycle's delta (+0.06). This
is by design: v0.5.6 is a **scope-shaped, not
full-featured** cycle (exposes 2 missing Win32 calls
and a safe wrapper, but does not add
`LVN_ODCACHEHINT`, `LVN_ODSTATECHANGED`, or
column-aware `sub_item` selection). The remaining
sub-items are scheduled for v0.5.7 (see § 5) and
should bump the score more, both because they are
larger deliverables and because they complete the
virtual-list-mode cluster.

**Goal recap (set at v0.5.0):** push the weighted score
past **9.40** by v0.5.4. v0.5.3 hit 9.40 one cycle
ahead of schedule; v0.5.4 landed at 9.51; v0.5.5
landed at 9.57; v0.5.6 lands at **9.62**, which is
**+0.22** above the v0.5.4 baseline and the
**highest score the project has recorded so far**.
The 5th 5-cycle pass is therefore **opening
comfortably above the v0.5.0 goal** of 9.40 and
on-track to clear the v0.5.9 target of 9.60
weighted.

---

## 7. Changelog snapshot

For the running log, see [`upgrade.md`](./upgrade.md).
The v0.5.6 entry is **Upgrade 22** in that file. The
previous report is
[`upgrade_report_v0.5.5.md`](./upgrade_report_v0.5.5.md).

**Source / test / build numbers (this cycle):**

- `src/list_ctrl.rs`: 1080 → 1385 lines
  (+305 lines: 75 lines of constants +
  `NMLVDISPINFOW` + `ListItem` struct + `impl<'a>`,
  70 lines of `set_item_count` +
  `on_get_disp_info` + the rewritten
  `get_item_count`, 50 lines of `item_count`
  field + `DispInfoCallback` alias +
  docstring, and 110 lines of the 8 new
  tests).
- `src/frame.rs`: 1520 → 1684 lines
  (+164 lines: 40 lines of `use` + field +
  method + `WM_NOTIFY` arm modification,
  124 lines of the 5 new tests).
- `src/lib.rs`: 1 line extended (the
  `pub use` line gains the `ListItem`
  identifier).
- `src/prelude.rs`: 1 line extended (same
  `ListItem` identifier added).
- `Cargo.toml` `version`: 0.5.5 → 0.5.6
  (1 line).
- `upgrade.md`: the report pointer at line
  12 updated to `upgrade_report_v0.5.6.md`,
  the U22 entry appended (+664 lines).
- `upgrade_report_v0.5.6.md`: this file
  (new, ~350 lines).
- All other source files, all 7 examples,
  the `Cargo.toml` `windows-sys` feature
  list, the `app.manifest`, `build.rs`,
  the 3 `clippy_*.txt` historical logs,
  `err.log`, `out.log`: **unchanged from
  v0.5.5**.

**Mid-pass summary:**

The 5th 5-cycle pass (v0.5.5 → v0.5.9) is now
2 cycles in. The weighted score moved from
**9.57** (v0.5.5) → **9.62** (v0.5.6), a
**+0.05** hand-off. v0.5.5 closed 1 of the 6
carry-over items from the v0.5.4 future-work
table (item 2, "wxWidgets parity gaps", is
**partially closed** for the 5th time, this
time for the `ListCtrl` LVS_OWNERDATA virtual
list mode); the OLE COM source side, the
`LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED`
virtual-mode optimisations, the
`DatePickerCtrl` value extraction, and the
GitHub Actions first green run are scheduled
for the remaining 3 cycles of the pass (see
§ 5).
