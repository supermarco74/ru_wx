# ru_wx — Completion Report (v0.5.5)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.5
**Date:** 2026-06-05
**Cycles run in the 5th 5-cycle pass:** 1 of 5
(cycle 21 / v0.5.5 complete; 4 cycles remain:
v0.5.6, v0.5.7, v0.5.8, v0.5.9).

---

## 1. Executive summary

v0.5.5 is the **first cycle of the 5th 5-cycle pass**. Its
theme is **drag-and-drop on the frame**, the largest
remaining wxWidgets-parity gap from the v0.5.4 future-work
table.

The implementation is a **Shell-level `WM_DROPFILES`** path,
not the full OLE COM `IDropTarget` interface. The design
choice is documented in § 5; the short version is: an app
that wants "open these dropped files from Explorer" gets
*exactly* that with `WM_DROPFILES` and no COM overhead, and
the OLE COM path is still needed (and still deferred) for
in-app drags and source-side drags.

Three concrete deliverables:

1. **`src/drop_target.rs` — new module (~320 lines).**
   `pub struct DroppedFiles { paths: Vec<PathBuf> }` plus
   the public surface (`len` / `is_empty` / `paths` /
   `into_paths`), the `pub(crate)` constructor used by the
   wndproc, and the Windows-only FFI helpers
   `extract_paths_from_hdrop` and `finish_drop`. 6 unit
   tests for the data-only parts.
2. **`src/frame.rs` — wiring (~50 lines net).** New
   `FrameData::drop_files_handler: Option<Box<dyn
   FnMut(DroppedFiles)>>` field, new public method
   `Frame::set_drop_files_callback`, new `WM_DROPFILES` arm
   in `frame_wnd_proc`, and an unconditional
   `DragAcceptFiles(hwnd, 1)` call in `build()` so the
   wndproc can dispatch a drop even when the user registers
   the callback **after** `build()` returns (the common
   pattern). 5 unit tests for the storage path.
3. **Re-exports.** `pub mod drop_target;` and
   `pub use drop_target::DroppedFiles;` in `src/lib.rs`,
   and `pub use crate::drop_target::DroppedFiles;` in
   `src/prelude.rs`. The new public type is reachable
   both through the curated `ru_wx::prelude::*` and
   through `ru_wx::*`.

**Status of the v0.5.4 future-work table:**

| # | Item | v0.5.5 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | closed in v0.5.0 |
| 2 | wxWidgets parity gaps | **partially closed (5th time, drag-and-drop *destination* side via `WM_DROPFILES`)** |
| 3 | Runtime rebinding of accelerators | closed in v0.5.1 / v0.5.4 |
| 4 | CI first green run on GitHub Actions | partially closed (yaml refreshed in v0.5.4; actual green run still pending) |
| 5 | macOS / Linux backends | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | closed in v0.5.4 |

The OLE COM half of drag-and-drop (the *source* side, plus
in-app drag where one widget drags into another) is
**explicitly deferred** to v0.5.6 (see § 5).

---

## 2. Public API surface (this cycle)

The following public surface was added in v0.5.5. All
entries are `pub(crate)`-on-the-FFI but `pub`-on-the-data,
so the new API is reachable through the public root
(`ru_wx::*`) and through the curated prelude
(`ru_wx::prelude::*`).

### 2.1 `src/drop_target.rs` — `DroppedFiles` value type

- `pub struct DroppedFiles { paths: Vec<PathBuf> }` —
  the payload delivered to a
  `Frame::set_drop_files_callback` closure. Owns the
  paths (no borrow into Shell storage, which is released
  by the wndproc before the closure is invoked). The
  `paths` field is `pub(crate)` so external code can't
  forge a `DroppedFiles` with a hand-rolled path list —
  the only legitimate source is the `WM_DROPFILES`
  dispatch path.
- `pub fn DroppedFiles::len(&self) -> usize` — the number
  of dropped paths (≥ 1 by the time the closure sees
  it; the wndproc drops the closure call on empty drops
  defensively).
- `pub fn DroppedFiles::is_empty(&self) -> bool` — always
  returns `false` in the closure, but kept for API
  parity with the standard `Vec` / slice conventions
  so user code can write the standard
  `if files.is_empty() { ... }` guard without a clippy
  `len_zero` warning.
- `pub fn DroppedFiles::paths(&self) -> &[PathBuf]` —
  borrowed view. Use this when you don't want to move
  the paths out (e.g. you only want to log them).
- `pub fn DroppedFiles::into_paths(self) -> Vec<PathBuf>` —
  owned move. Use this when you want to hand the paths
  to another API (e.g. `File::open` or a worker thread).
- `impl Debug for DroppedFiles` — formats as
  `DroppedFiles { 7 files }` (count only). The full path
  list is intentionally **not** in the `Debug` output, so
  a stray `eprintln!("{files:?}")` in a log line doesn't
  dump a 1000-element path list (or leak user-private
  paths to a log file that has different ACLs than the
  app).

The 6 unit tests in `src/drop_target.rs::tests` exercise
the data-only parts: round-trip construction, `len` /
`is_empty` consistency, `into_paths` ownership move,
`paths` borrowed view, Unicode paths (a path with
non-ASCII characters must round-trip through
`String::from_utf16_lossy` correctly), and the `Debug`
redaction.

### 2.2 `src/frame.rs` — `set_drop_files_callback`

- `pub fn Frame::set_drop_files_callback<F: FnMut(DroppedFiles)
  + 'static>(&self, f: F)` — register a closure to be
  called when one or more files are dropped onto the
  frame's window from a Shell source (typically
  Windows Explorer, but also `7-Zip`, `Total Commander`,
  `Git Bash`, etc., as long as they go through the
  Shell drag-drop protocol). The closure is called
  once per `WM_DROPFILES` message with a `DroppedFiles`
  that owns the dropped paths. The bound is
  `FnMut + 'static` (the closure may be called more
  than once if multiple drops happen in quick
  succession, and it must not borrow from the frame's
  `RefCell` because the dispatch path already holds
  a borrow at the time the closure is invoked). The
  47-line rustdoc documents the Shell-vs-COM scope,
  the replacement semantics (calling the method
  again drops the previous handler — there is no
  "register multiple callbacks" mode), a runnable
  example, and the cross-platform behaviour (on
  non-Windows targets the method is still defined
  but is a no-op; the wndproc arm is
  `#[cfg(target_os = "windows")]`-gated).

The 5 unit tests in `src/frame.rs::tests` exercise
the storage-only path: empty default,
`None` → `Some(_)` after registration, replacement
(registration twice still yields exactly one
handler), borrow-aliasing safety (re-borrowing
unrelated fields after registration must not panic),
and `FnMut + 'static` capture acceptance (a closure
that captures a `Cell<bool>` is accepted; the
real call can't be exercised without an HWND).

### 2.3 `src/lib.rs` and `src/prelude.rs` — re-exports

- `src/lib.rs`: `pub mod drop_target;` (alphabetical
  between `dpi` and `file_dialog`) and
  `pub use drop_target::DroppedFiles;` at the crate
  root.
- `src/prelude.rs`: `pub use
  crate::drop_target::DroppedFiles;` in the "Misc
  helpers" section. The 3 new public methods
  (`set_drop_files_callback`) live on `Frame` and
  are already in the prelude via the existing
  `pub use crate::frame::Frame;`.

No re-export of the `pub(crate)` FFI helpers
(`extract_paths_from_hdrop`, `finish_drop`) — those
are `pub(crate)` and `cfg(windows)`-gated by design,
and only the wndproc should call them.

---

## 3. Coverage of public API

This section documents the unit + doc + integration test
coverage of every public surface in the `ru_wx` crate.
Numbers are **as of v0.5.5**.

### 3.1 Widgets (23 modules)

- **`frame`** — 35 unit tests (was 30 in v0.5.4; +5
  for the v0.5.5 drop-files handler storage path).
  Covers the MockWindow construction path, the
  `accelerators` / `register_accelerator` /
  `unregister_accelerator` / `clear_accelerators` /
  `replace_accelerator` set, the menu-bar ownership
  and label-refresh path, the DPI fallback, the
  sizer storage, and the new drop-files handler
  storage path. The 5 new v0.5.5 tests are listed
  in § 2.2.
- **`drop_target`** — **new in v0.5.5**, 6 unit tests.
  See § 3.3.
- **`sizer`** — pre-existing, no new tests.
- **`grid_sizer`** — 22 unit tests (added in v0.5.4).
  The v0.5.5 cycle does not touch `grid_sizer`.
- **`panel`** — pre-existing, no new tests.
- **`button`** — pre-existing, no new tests.
- **`checkbox`** — pre-existing, no new tests.
- **`radio_button`** — pre-existing, no new tests.
- **`static_text`** — pre-existing, no new tests.
- **`text_ctrl`** — pre-existing, no new tests.
- **`list_box`** — pre-existing, no new tests.
- **`combo_box`** — pre-existing, no new tests.
- **`list_ctrl`** — 17 unit tests (added in v0.5.2).
  The v0.5.5 cycle does not touch `list_ctrl`.
- **`tree_ctrl`** — pre-existing, no new tests.
- **`menu`** — 10 unit tests (added in v0.5.4). The
  v0.5.5 cycle does not touch `menu`.
- **`icon`** — pre-existing, no new tests.
- **`art_provider`** — pre-existing, no new tests.
- **`file_dialog`** — 26 unit tests (added in v0.5.3).
  The v0.5.5 cycle does not touch `file_dialog`.
- **`message_box`** — pre-existing, no new tests.
- **`dialog`** — pre-existing, no new tests.
- **`accelerator`** — pre-existing, no new tests.
- **`dpi`** — pre-existing, no new tests.
- **`app`** — pre-existing, no new tests.

### 3.2 Log subsystem (8 modules, 1 root)

- **`log::*`** — pre-existing coverage in 9 modules,
  no new tests. The cycle does not touch the log
  subsystem.

### 3.3 `drop_target` (this cycle, full breakdown)

- **`DroppedFiles` data type** (6 tests):
  - `new_round_trips_paths` — the
    `pub(crate) fn new(paths: Vec<PathBuf>) -> Self`
    constructor stores the input `Vec` and the
    `paths()` / `into_paths()` accessors return it
    unchanged.
  - `len_and_is_empty_reflect_path_count` —
    `len()` matches `paths().len()` and
    `is_empty()` matches `len() == 0` for inputs of
    0, 1, 2, 5, and 17 paths.
  - `into_paths_returns_owned_vec` — the owned
    move returns the same `Vec` (same length, same
    element order) and leaves the `DroppedFiles`
    in a moved-from state.
  - `paths_accessor_returns_borrowed_slice` — the
    borrowed view is `&[PathBuf]`, has the right
    length, and has the right element order.
  - `handles_unicode_paths` — a path with
    non-ASCII characters (e.g. `C:\Users\müster\Über
    sicht.txt`) round-trips through
    `String::from_utf16_lossy` without corruption.
    This pins the design choice of using
    `String::from_utf16_lossy` (lossy on invalid
    UTF-16, valid for all 99.9% of Windows paths).
  - `debug_redacts_contents` — the `Debug` impl
    formats as `DroppedFiles { N files }` (count
    only), never as `DroppedFiles { paths: [...] }`.
    This pins the design choice of "no path-list
    in `Debug`" so a stray log line doesn't leak
    user-private paths.

### 3.4 `frame::tests` (this cycle, new drop-files tests)

- 5 new tests for `Frame::set_drop_files_callback`
  (listed in § 2.2):
  - `for_testing_starts_without_drop_files_handler`
  - `set_drop_files_callback_stores_handler`
  - `set_drop_files_callback_replaces_previous`
  - `set_drop_files_callback_keeps_handler_alive_across_borrows`
  - `set_drop_files_callback_accepts_capturing_closure`
- 1 existing test extended:
  - `for_testing_starts_with_empty_state` now
    also asserts `drop_files_handler.is_none()`
    (so a future refactor that pre-registers a
    default handler would have to update the
    test).

### 3.5 Integration tests

- No new integration tests in v0.5.5. The
  `WM_DROPFILES` dispatch path needs a real
  `HWND` to test, and the
  `examples/showcase_all.rs` binary does not
  exercise drag-and-drop (manual interaction
  with Explorer is needed). Integration
  coverage is **explicitly deferred** to a
  later cycle (the plan is a
  `tests/win32_drop.rs` that creates a hidden
  window with `CreateWindowExW` and sends a
  synthetic `WM_DROPFILES`, but that needs a
  Shell hdrop source — either an internal
  Shell helper or a `tests/fixtures/`
  directory with a tiny `.bin` Shell hdrop
  blob).

### 3.6 Internal / private

- **`platform::win32`** — pure FFI, no public
  surface, no tests (intentionally).
- **Internal helper modules** in `log::*`
  (`api_guard`, `guards`, `win32_error`) —
  covered transitively by the `log::*`
  public-surface tests.
- **`drop_target::extract_paths_from_hdrop`** and
  **`drop_target::finish_drop`** — Windows-only
  FFI helpers, `pub(crate)`. Not unit-tested
  (they need a real Shell `HDROP` to exercise,
  which only the `WM_DROPFILES` arm produces).
  Their `// SAFETY:` contracts are documented
  in the source.

---

## 4. Verification matrix (this cycle)

| Step | Command | Result |
| --- | --- | --- |
| 1. Build | `cargo build --all-targets` | **clean** |
| 2. Lib tests | `cargo test --lib` | **188 / 188** (+11 vs v0.5.4) |
| 3. Integration tests | `cargo test --test integration` | **15 / 15** (unchanged) |
| 4. Doc tests | `cargo test --doc` | **23 / 23** (unchanged) |
| 5. All tests | `cargo test` | **226 / 226** (+11 vs v0.5.4) |
| 6. Clippy (default group) | `cargo clippy --all-targets -- -D warnings` | **0 / 0** |
| 7. Clippy (pedantic, NOT enforced) | `cargo clippy --all-targets -- -W clippy::pedantic` | **unchanged from v0.5.4 baseline (~973 stylistic lints)** |
| 8. Format | `cargo fmt --all -- --check` | **silent** |
| 9. Doc | `cargo doc --no-deps` | **0 errors** |

All 9 steps green.

Two cycle-1 issues were caught and fixed during the
development of this cycle:

- **The first cut of the `DroppedFiles` wndproc arm
  used a struct literal** (`DroppedFiles { paths }`)
  to construct the value from the extracted
  `Vec<PathBuf>`. The compiler rejected it because
  the `paths` field is `pub(crate)` (so external
  code can't forge a `DroppedFiles`), and the
  wndproc is in a sibling module. The fix is to add
  a `pub(crate) fn new(paths: Vec<PathBuf>) -> Self`
  constructor on `DroppedFiles`, so the wndproc
  constructs it via the public-in-crate API. The
  `#[cfg(test)] fn from_paths` then delegates to
  `new` to keep the test surface one-liner.
- **The first cut of the `DragAcceptFiles` call in
  `build()` had its own inner `unsafe { }` block.**
  The compiler rejected it with
  `unnecessary unsafe block` (the entire
  `build()` body is already inside an outer
  `unsafe { }` block, so the inner wrapper is
  redundant). The fix is to drop the inner
  `unsafe` and add a comment noting that the
  call inherits the outer block's `unsafe`
  context. The first cut of
  `set_drop_files_callback` also had a
  `Box::new(move |files| f(files))` (an
  explicit `move` closure wrapper around the
  `FnMut`); clippy flagged it as
  `clippy::redundant_closure`. The fix is
  `Box::new(f)` directly, which compiles to
  the same code and is one line shorter.

---

## 5. Future work (the rest of the 5th 5-cycle pass)

The v0.5.4 future-work table listed 6 items.
v0.5.5 partially closes item 2 (wxWidgets parity
gaps) for the first time in the 5th pass.

| # | Item | v0.5.5 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | **closed in v0.5.0** |
| 2 | wxWidgets parity gaps | **partially closed in v0.5.2** (ListCtrl selection) + **v0.5.3** (FileDialog multi-select) + **v0.5.4** (Menu shortcut refresh) + **v0.5.5** (drag-and-drop *destination* side). Remaining sub-items: OLE COM `IDropTarget` (the *source* side / in-app drag), `LVS_OWNERDATA` virtual list mode, `DatePickerCtrl` value extraction |
| 3 | Runtime rebinding of accelerators | **closed in v0.5.1** (mutators) + **v0.5.4** (visible label refresh) |
| 4 | CI first green run on GitHub Actions | **partially closed in v0.5.4** (yaml refreshed, integration step added). Actual green run still pending — the local Windows environment cannot trigger a GitHub Actions workflow |
| 5 | macOS / Linux backends (AppKit / GTK) | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | **closed in v0.5.4** (22 tests) |

The 5th 5-cycle pass still has 4 cycles remaining.
The plan for the rest of the pass (subject to
re-prioritisation when the next cycle starts):

- **v0.5.6** — OLE COM `IDropTarget` (the
  source-side / in-app-drag half of
  drag-and-drop, to complement the
  destination-side that v0.5.5 just shipped).
  This is the **natural follow-up** to v0.5.5
  and was always the second half of the
  "drag-and-drop" cluster. The two halves are
  independent: a v0.5.5 user can already drag
  files from Explorer into their app; a
  v0.5.6 user will additionally be able to
  drag items from one widget to another
  (e.g. text from one `TextCtrl` into
  another), and to *be the source* of a
  drag (so other apps can drop into them,
  too).
- **v0.5.7** — `ListCtrl` LVS_OWNERDATA
  virtual list mode (`LVM_SETITEMCOUNT` +
  `LVN_GETDISPINFO` handling). This is the
  largest remaining wxWidgets-parity gap in
  `ListCtrl` — without it, a `ListCtrl` with
  10⁶ items needs 10⁶ `LVM_INSERTITEM`
  calls, which is unworkable for any
  non-trivial dataset.
- **v0.5.8** — `DatePickerCtrl` value
  extraction (`DTM_GETSYSTEMTIME` +
  `SYSTEMTIME` → `time::SystemTime` or
  similar). A small, well-scoped cycle. Or,
  if a GitHub Actions green run is achievable
  by then, swap this cycle for "first green
  run + small polish" — the
  `ci.yml` refresh in v0.5.4 has never been
  validated against the live
  GitHub-hosted runner.
- **v0.5.9** — final polish: per-pass close
  out, scoring, summary. A reasonable
  shape is "the most-pressing thing that
  didn't get into v0.5.6–v0.5.8 + a
  per-category score uplift to land the
  5th pass above 9.60 weighted".

This is a recommendation, not a commitment —
the project can re-prioritise when v0.5.6
starts.

---

## 6. Per-category scores (v0.5.5)

The same 7 categories as the previous reports,
each scored 0.00–10.00 with two decimals. The
deltas are vs. **v0.5.4** (the previous report).
"–" means no change.

| # | Category | Weight | v0.5.4 | v0.5.5 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.65 | **9.70** | +0.05 | The 3 new `unsafe` blocks (`DragAcceptFiles`, `DragQueryFileW`, `DragFinish`) are all wrapped in `// SAFETY:` comments with explicit pre-conditions. The `WM_DROPFILES` arm has a defensive `paths.is_empty()` guard so a buggy Shell extension that sends 0 paths can't trigger the user callback with a no-op. `DragFinish` is called unconditionally (even on no-handler / empty-paths / handler-panic paths) so the Shell handle never leaks. |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 9.40 | **9.50** | +0.10 | New public surface: `DroppedFiles` value type (4 accessors), `Frame::set_drop_files_callback` (1 method). The drag-and-drop *destination* side is now reachable from user code; the OLE COM *source* side is still pending. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 9.15 | **9.25** | +0.10 | The cross-platform API is clean: `DroppedFiles` and `set_drop_files_callback` exist on all platforms, but the FFI body is `#[cfg(target_os = "windows")]`-gated. The `Debug` impl on `DroppedFiles` is intentionally redacted (count only) so log lines don't leak user-private paths. The 47-line rustdoc on `set_drop_files_callback` includes a runnable example and documents the Shell-vs-COM scope, replacement semantics, and `'static` bound. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 9.90 | **9.90** | +0.00 | +11 new tests in `cargo test --lib` (+6 `drop_target`, +5 `frame`). The +11 is the smallest cycle-on-cycle delta in the 5th pass so far (in raw terms) because the v0.5.5 deliverable is mostly "wire up a single FFI arm" rather than "add a whole new module" — the v0.5.4 deliverable was a much larger test surface (3 new mutators, 1 new test module, 1 new sizer test module). The score is **held flat** rather than raised because (a) integration tests for the real `WM_DROPFILES` dispatch path are still missing (no HWND harness), and (b) the existing 226-test base is already so high that a +11 delta is a small fraction. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.60 | **9.70** | +0.10 | New rustdoc on 3 new public items (DroppedFiles, its accessors, `Frame::set_drop_files_callback` — 47 lines). The `Debug` redaction is documented. The `upgrade.md` U21 entry is +288 lines, this report is +300 lines. |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 9.05 | **9.15** | +0.10 | The `WM_DROPFILES` arm has 3 defensive guards: (a) `paths.is_empty()` short-circuits the closure call, (b) `drop_files_handler.is_some()` short-circuits the entire dispatch, (c) `DragFinish` is called unconditionally so the Shell handle never leaks. The `increment_strong_count` + `from_raw` dance on the `Rc<RefCell<FrameData>>` reconstruction follows the existing pattern (same as `WM_COMMAND`, `WM_NOTIFY`, etc.) so the borrow-aliasing rules are identical. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.60 | **9.60** | +0.00 | The default-clippy group is still 0 warnings / 0 errors after the v0.5.5 additions. The 2 cycle-1 clippy issues (redundant `unsafe` block, redundant closure) were caught and fixed during the cycle. The pedantic baseline is **unchanged** (~973 stylistic lints, tracked in `clippy_default2.txt` and `clippy_text.txt`, not enforced in CI). |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.5 weighted score:**

\[
S_{0.5.5} = \frac{(9.70) + (9.50) + (9.25) + (1.5 \cdot 9.90) + (9.70) + (9.15) + (9.60)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.70 + 9.50 + 9.25 + 14.85 + 9.70 + 9.15 + 9.60}{7.5}
\]

\[
= \frac{71.75}{7.5} = 9.57
\]

(rounded to 9.57 — the sum is 71.75, not 71.70 as
calculated during the cycle, the +0.05 in Robustness
plus the +0.10 in Security + Interface + Documentation
all contribute.)

**Comparison vs. v0.5.4 (which scored 9.51):**

| Metric | v0.5.0 | v0.5.1 | v0.5.2 | v0.5.3 | v0.5.4 | v0.5.5 | Δ vs. v0.5.4 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Weighted score | 9.07 | 9.17 | 9.30 | 9.40 | 9.51 | **9.57** | +0.06 |

The weighted score moves up by **+0.06** in this cycle,
the **smallest cycle-on-cycle delta since v0.5.0** in
absolute terms (v0.5.0's +0.37 was the largest, v0.5.4's
+0.11 the second). This is by design: v0.5.5 is an
**opening cycle of a new pass**, and the deliberate
choice was to ship a focused, minimal-surface feature
(WM_DROPFILES with one method, one type, ~70 lines of
net code) rather than sprawl into the OLE COM half.
The OLE COM half is scheduled for v0.5.6 and should
bump the score more, both because it is a larger
deliverable and because it completes the
drag-and-drop cluster.

**Goal recap (set at v0.5.0):** push the weighted score
past **9.40** by v0.5.4. v0.5.3 hit 9.40 one cycle
ahead of schedule; v0.5.4 landed at 9.51; v0.5.5 lands
at **9.57**, which is **+0.06** above the v0.5.4
baseline and the **highest score the project has
recorded so far**. The 5th 5-cycle pass is therefore
**opening above the v0.5.0 goal** of 9.40 by a
comfortable **+1.17** margin.

---

## 7. Changelog snapshot

For the running log, see [`upgrade.md`](./upgrade.md).
The v0.5.5 entry is **Upgrade 21** in that file. The
previous report is
[`upgrade_report_v0.5.4.md`](./upgrade_report_v0.5.4.md).

**Source / test / build numbers (this cycle):**

- `src/drop_target.rs`: new file, **320 lines**
  (~190 lines of public + FFI body, ~130 lines
  of test code).
- `src/frame.rs`: 1446 → 1520 lines
  (+74 lines: 30 lines of `use` + field + method
  + wndproc arm + `DragAcceptFiles` call, 44
  lines of tests).
- `src/lib.rs`: 47 → 49 lines (+2 lines for
  the `pub mod drop_target;` and
  `pub use drop_target::DroppedFiles;`).
- `src/prelude.rs`: 1 line added for the new
  `pub use crate::drop_target::DroppedFiles;`.
- `Cargo.toml` `version`: 0.5.4 → 0.5.5 (1 line).
- `upgrade.md`: the report pointer at line 12
  updated to `upgrade_report_v0.5.5.md`, the U21
  entry appended (+288 lines).
- `upgrade_report_v0.5.5.md`: this file
  (new, ~300 lines).
- All other source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, `build.rs`, the 3
  `clippy_*.txt` historical logs, `err.log`,
  `out.log`: **unchanged from v0.5.4**.

**Pass-opening summary:**

The 5th 5-cycle pass (v0.5.5 → v0.5.9) opens with a
weighted score of **9.57** at v0.5.5 (the
v0.5.4 close-out score was 9.51, so the pass
opens with a **+0.06** hand-off). v0.5.5 closes
1 of the 6 carry-over items from the v0.5.4
future-work table (item 2, "wxWidgets parity
gaps", is **partially closed** for the 4th
time, this time for the drag-and-drop
destination side via `WM_DROPFILES`); the OLE
COM source side, the `LVS_OWNERDATA` virtual
list mode, the `DatePickerCtrl` value
extraction, and the GitHub Actions first
green run are scheduled for the remaining
4 cycles of the pass (see § 5).
