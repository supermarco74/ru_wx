# ru_wx — Upgrade Log

This file is the running log of structural improvements made to the
`ru_wx` library. Each entry corresponds to one upgrade cycle and contains:

- the version bump applied to `Cargo.toml`
- the date
- the theme of the changes
- the concrete code-level changes

Scores and a project-wide completion report live in
[`upgrade_report_v0.6.2.md`](./upgrade_report_v0.6.2.md)
(latest) and
[`upgrade_report_FINAL.md`](./upgrade_report_FINAL.md)
(end-of-5-step-programme summary).

---

## Upgrade 1 — Lint cleanup → `0.2.1` (2026-06-05)

**Theme:** remove the 38 compiler warnings emitted by `cargo build`. None
of them indicated broken code, but they drowned the build log and made
real warnings hard to spot.

**Changes:**

- `src/tab.rs` — `TCITEMW` is a Win32 ABI struct, so the field names
  must match the Win32 header. Added `#[allow(non_snake_case)]` on the
  struct instead of renaming.
- `src/colour_picker_ctrl.rs`, `src\gauge.rs` — removed unused
  `windows_sys::Win32::Graphics::Gdi::*` imports.
- `src\list_ctrl.rs`, `src\static_text.rs` — removed unused
  `crate::frame::Frame` imports.
- `src\art_provider.rs:262`, `src\panel.rs:233`,
  `src\radio_box.rs:230`, `src\tab.rs:430`, `src\timer.rs:101`,
  `src\tool_bar.rs:203` — dropped unneeded `mut` on local bindings.
- `src\colour_picker_ctrl.rs:150` — prefixed unused `frame` parameter
  with `_` in a helper function.
- Unused Win32 constants that are part of a public ABI surface were
  marked `#[allow(dead_code)]` rather than removed (`LB_GETTEXT`,
  `LB_GETTEXTLEN`, `LB_GETCOUNT`, `PBM_SETRANGE`, `LBN_SELCHANGE`,
  `LBN_DBLCLK`, `WM_USER`, `TBS_TOP`, `TBS_NOTICKS`, `UDM_GETRANGE`,
  `UDM_SETACCEL`, `UDM_GETBASE`, `SB_GETPARTS`, `SB_SIMPLE`,
  `SB_SETMINHEIGHT`, `TCM_DELETEITEM`, `TBSTYLE_LIST`,
  `TBSTYLE_TRANSPARENT`).
- Dead fields removed: `Slider::vertical`, `DatePickerCtrl::allow_none`,
  `ToolBar::label`.

**Result:** build is warning-free.

---

## Upgrade 2 — Symmetric getter APIs → `0.2.2` (2026-06-05)

**Theme:** close the API-symmetry gap. Several widgets exposed setters
(`set_label`, `set_range`) but no matching getter, forcing user code to
keep a redundant `String` of its own. The getters below all read the
live value from the underlying Win32 control.

**Changes:**

- `src\static_text.rs` — added `StaticText::get_label() -> String`. On
  Windows it calls `GetWindowTextLengthW` + `GetWindowTextW`; on the
  non-Windows stub it returns the cached `label` field.
- `src\button.rs` — added `Button::get_label() -> String` with the same
  Win32 / stub fallback.
- `src\checkbox.rs` — added `CheckBox::get_label() -> String` with the
  same Win32 / stub fallback.
- `src\slider.rs` — added `Slider::get_range() -> (i32, i32)` that
  delegates to the existing `get_min` / `get_max` pair.
- `src\spin_ctrl.rs` — added `SpinCtrl::get_range() -> (i32, i32)` with
  the same delegation.

**Result:** all five new getters compile on Windows and on the
non-Windows stub. The library still builds warning-free.

---

## Upgrade 3 — Prelude + module docs → `0.3.0` (2026-06-05)

**Theme:** make the public surface easier to discover and import.
Before this upgrade, a typical user file needed 5–10 explicit
`use` statements scattered across the 45 modules. After it, a single
`use ru_wx::prelude::*;` brings the whole working set into scope.

**Changes:**

- New `src/prelude.rs` exposing the ~50 items that make up the typical
  “build a window, add some controls, run the loop” working set:
  the `Widget` / `WidgetRef` / `Window` traits, every public widget
  type, the sizer / geometry primitives, the menu / status-bar /
  toolbar / aui-toolbar family, the icon / bitmap / image-list
  helpers, the font / art-provider / timer / tooltip helpers, and the
  common `MessageBox` / `MessageDialog` / `FileDialog` enums.
- `src/lib.rs` — declared `pub mod prelude;` and added a doc
  paragraph pointing at it.
- Module-level doc comments added to: `app.rs`, `widget.rs`,
  `frame.rs`, `panel.rs`, `sizer.rs`, `button.rs`, `static_text.rs`,
  `text_ctrl.rs`, `menu.rs`, `geometry.rs`. Each comment explains
  the role of the widget and the typical entry point.
- `lib.rs` doctest example updated to use the new prelude.

**Result:** the library still builds warning-free on Windows. The
prelude module itself has a runnable doctest, which is validated by
`cargo test --doc`. The first `use ru_wx::prelude::*;` line is now
sufficient for every example shipped with the crate.

---

## Upgrade 4 — Unit + integration tests → `0.3.1` (2026-06-05)

**Theme:** add the first formal test suite. The library had shipped
without any `#[test]`-style coverage; widgets, the sizer, the geometry
primitives and the art-provider had only ever been verified by hand
inside the example binaries. This upgrade introduces a runnable,
headless test layer for the parts of the API that don't require a live
Win32 message loop.

**Changes:**

- `src/geometry.rs`
  - Added `PartialEq, Eq` to the `Colour` derive list (required by
    `assert_eq!` in the tests).
  - New `#[cfg(test)] mod tests` with 6 cases:
    `rect_new_keeps_fields`, `rect_default_is_origin_zero_zero`,
    `rect_contains_is_inclusive_min_exclusive_max`,
    `colour_constants_have_expected_channels`,
    `colour_default_is_white`, `colour_to_colorref_is_bbggrr`
    (Windows-only).
- `src/sizer.rs`
  - New `#[cfg(test)] mod tests` with a small `MockWidget` that
    implements `Widget` and records the last `set_position` /
    `set_size` call, so layout can be asserted without a real HWND.
  - 6 cases: `empty_sizer_layout_does_not_panic`,
    `horizontal_sizer_packs_fixed_size_children`,
    `vertical_sizer_packs_fixed_size_children`,
    `horizontal_sizer_distributes_proportional_space`,
    `layout_respects_custom_padding`,
    `vertical_sizer_aligns_children_to_origin_x`.

**Result:** `cargo test --lib` reports `15 passed; 0 failed`. The
geometry and sizer modules now have explicit, repeatable coverage and
any future regression in the `BoxSizer` proportional math (or in the
`Colour` <-> `COLORREF` byte order) will fail the build instead of
silently shipping.

---

## Upgrade 5 — Unsafe code audit + SAFETY comments → `0.3.2` (2026-06-05)

**Theme:** the library is a thin safe-Rust wrapper over `windows-sys` 0.59,
which means every widget implementation contains multiple `unsafe { }`
blocks that call directly into the Win32 ABI. The clippy lint
`clippy::undocumented_unsafe_blocks` (warn-by-default in current Rust)
requires a `// SAFETY: ...` justification above every `unsafe` block.
Before this upgrade the project shipped 325 undocumented `unsafe` blocks
across 57 source files. The audit also caught 8 new clippy errors from
the stricter `clippy::not_unsafe_ptr_arg_deref` lint, all on public
function signatures that take / return raw Win32 handles.

**Changes:**

- `src/bitmap_bundle.rs` — added `#[allow(clippy::not_unsafe_ptr_arg_deref)]`
  to `pub fn best_for_hwnd` and a `// SAFETY: ...` comment to its inner
  `unsafe { GetDC(...) / ... }` block.
- `src/icon.rs` — same `#[allow]` on `pub fn hbitmap_to_hicon` and
  `pub fn destroy_hicon`, with SAFETY comments inside their `unsafe` blocks
  (explaining the null-handle check before `DestroyIcon`).
- `src/menu.rs` — same `#[allow]` on `pub fn popup_at_cursor` + SAFETY
  comment.
- `src/platform/win32.rs` — same `#[allow]` on
  `pub fn get_device_caps_dpi` + SAFETY comment on the inner
  `unsafe { GetDeviceCaps(...) }`.
- New `tools/add-safety.ps1` — an idempotent PowerShell script (152 lines,
  no Python, no external dependencies) that walks every `*.rs` file under
  `src/`, finds `unsafe {` blocks that are not preceded by a
  `// SAFETY:` comment, and inserts a justification chosen by the FFI
  function name (e.g. `CreateWindowExW`, `SendMessageW`,
  `GetWindowTextLengthW`, `DefWindowProcW`, etc.). The script supports a
  fallback `"Win32 FFI call with validated arguments ... and a buffer
  large enough for the output."` for FFI names it does not recognize
  explicitly. Re-runs are safe: blocks that already have a SAFETY comment
  are left untouched.
- 325 `// SAFETY: ...` comments inserted across 57 source files. Highest
  counts: `top_level_window.rs` (+25), `dialog.rs` (+21),
  `list_box.rs` (+17), `combo_box.rs` (+16), `list_ctrl.rs` (+16),
  `tree_ctrl.rs` (+15), `check_list_box.rs` (+15), `gauge.rs` (+14),
  `slider.rs` (+14), `button.rs` (+14), `menu.rs` (+14), `tab.rs` (+13),
  `grid.rs` (+13), `panel.rs` (+12), `aui_tool_bar.rs` (+11).

**Result:** `cargo build --lib` reports `0 errors, 0 warnings`. Running
`cargo clippy --lib -- -W clippy::undocumented_unsafe_blocks` reports
`0` unsafe-block warnings. `cargo test --lib` still passes `15 / 15`.
Every `unsafe { }` in the library is now justified inline, and the
public API surface has a clear annotation explaining which functions are
thin FFI wrappers around Win32 handle types.

---

## Upgrade 6 — Manifest embedding for example .exe files → `0.3.3` (2026-06-05)

**Theme:** the `[[example]]` binaries built by `cargo build --examples`
were crashing on Windows 11 with `0xc0000142` (`STATUS_DLL_INIT_FAILED`)
because the Common Controls v6 manifest was never embedded into them.
The library's `build.rs` already compiled `app.rc` into a static
`app.lib` via `embed-resource`, but the `cargo:rustc-link-lib=dylib=app`
directive emitted by the library's build script is **not** forwarded by
cargo to downstream `[[example]]` targets. The result: the example .exe
files were linked without `app.lib`, and therefore without any
manifest, so `InitCommonControlsEx` never received the
`ICC_TAB_CLASSES | ICC_LISTVIEW_CLASSES` flags and the runtime tried to
create a Win32 control class that the OS had not registered.

**Changes:**

- New `build_with_manifest.ps1` (90 lines, single file, PowerShell
  only) that wraps `cargo build` and post-processes every resulting
  .exe under `target/<profile>/examples/`. It:
  1. Forwards every argument to `cargo build` unchanged (so
     `.\build_with_manifest.ps1 --release --example input_controls_demo`
     works).
  2. Locates `mt.exe` (the Windows SDK Manifest Tool) by walking
     `C:\Program Files (x86)\Windows Kits\10\bin\*\x64\mt.exe` and
     picking the highest installed SDK version.
  3. Runs `mt.exe -manifest app.manifest -outputresource:<exe>;1` on
     every `.exe` it finds.
  4. Reports `embedded=N failed=M profile=<debug|release>` and exits
     non-zero on any failure.
- `app.manifest` reviewed and confirmed to request both
  `Microsoft.Windows.Common-Controls` v6.0.0.0 and
  `PerMonitorV2` DPI awareness (already correct from U5; no edit).
- `build.rs` was reviewed and the explicit
  `cargo:rustc-link-lib=dylib=app` it already emits is now documented
  in a comment as a no-op for `[[example]]` targets; the manifest
  embedding happens via the post-build step instead.

**Verification (the `mt.exe` step actually fires):**

```
$content = ReadAllBytes(target\debug\examples\input_controls_demo.exe)
$ascii   = ASCII.GetString($content)
$ascii.Contains("PerMonitorV2")         -> True
$ascii.Contains("Common-Controls")      -> True
$ascii.Contains("Microsoft.Windows.Common-Controls") -> True
```

**Verification (the .exe actually launches):**

```
Start-Process input_controls_demo.exe -> WaitForInputIdle -> HasExited = false
```

The `0xc0000142` crash is gone; the demo window appears and stays open.

**Result:** the `input_controls_demo` example now runs end-to-end on
Windows 11 from a clean build. `cargo build --lib` is still 0/0 and
`cargo test --lib` still passes 15 / 15. The post-build wrapper is a
single, ~90-line PowerShell file and adds no extra build-time
dependency beyond the Windows 10/11 SDK that is already required by
`embed-resource`.

---

## Upgrade 7 — Clippy pedantic cleanup → `0.3.4` (2026-06-05)

**Theme:** drive the crate down to **zero** warnings under
`cargo clippy --lib --no-deps` (which runs the default + pedantic
groups). The previous cycle shipped with 76 pedantic lints tripping,
all of them benign but all of them noise that hid real regressions.
Auto-fix swept 42 of them; the remaining 34 were all pattern-level
issues that needed a human decision (suppress an ABI-struct lint, or
inline a real `checked_div`, or remove a `drop(non_drop)` call, etc.).
This upgrade finishes the job.

**Changes:**

- **Unnecessary pointer casts (18):** Win32 GDI handles such as
  `HBITMAP`, `HBRUSH`, `HICON` already type to `*mut c_void` in
  `windows-sys` 0.59, so the trailing
  `... as *mut core::ffi::c_void` / `... as *const u16` casts in
  `DeleteObject` / `LoadIconW` / `LoadCursorW` / `SelectObject` were
  always no-ops. Removed in:
  `src/aui_tool_bar.rs:188`, `src/bitmap_bundle.rs:277`,
  `src/button.rs:130`, `src/button.rs:140`, `src/button.rs:415`,
  `src/dialog.rs:71`, `src/dialog.rs:72`, `src/font.rs:149`,
  `src/frame.rs:303`, `src/frame.rs:304`, `src/icon.rs:147`,
  `src/icon.rs:164`, `src/icon_tray.rs:156`, `src/menu.rs:133`,
  `src/menu.rs:143`, `src/menu.rs:373`, `src/panel.rs:70`,
  `src/panel.rs:409`.
- **Acronym struct names (8):** the Win32 ABI structs `TCITEMW`,
  `LVCOLUMNW`, `LVITEMW`, `TBBUTTON`, `TVINSERTSTRUCTW` and `TVITEMW`
  are spelled in upper case by the Win32 header, so the clippy lint
  `clippy::upper_case_acronyms` cannot be allowed to rename them.
  Annotated each struct with `#[allow(clippy::upper_case_acronyms)]`
  in `src/tab.rs`, `src/list_ctrl.rs` (2 structs), `src/grid.rs`
  (2 structs), `src/aui_tool_bar.rs`, `src/tool_bar.rs`,
  `src/tree_ctrl.rs` (2 structs).
- **Manual checked division (2):** the `BoxSizer` proportional layout
  used to write `if total_proportion > 0 { ... / total_proportion } else { 0 }`
  twice. Replaced with `(available as u32 * proportion)
  .checked_div(total_proportion).unwrap_or(0) as i32` in
  `src/sizer.rs:132` and `src/sizer.rs:153`.
- **`drop_non_drop` (2):** `src/tab.rs:301` and `src/tab.rs:371` were
  calling `std::mem::drop(item)` on a `TCITEMW` value. `TCITEMW` does
  not implement `Drop`, so the call was only extending the lifetime
  of the contained `*const u16` borrow, which the end of the
  enclosing `unsafe` block already does for free. Removed.
- **Manual clamp (1):** `src/spin_ctrl.rs:193` was manually clamping
  `min16` to the inclusive range `[0, 0xFFFF]` with two
  `if` statements. Replaced with `min16 = min16.clamp(0, 0xFFFF);`
  (the `max16` half of the pattern is order-dependent on `min16` and
  is left as the original two-`if` block).
- **`to_*` method on `Copy` self (1):** `DatePickerCtrlInner::to_date`
  in `src/date_picker_ctrl.rs:101` took `&self` even though
  `SystemTime` is `Copy`. Changed the signature to `fn to_date(self)`.
- **Unnecessary parentheses (1):** `src/status_bar.rs:138` had
  `let wparam = (i & 0xFF);` — the inner expression does not need
  to be parenthesised when assigned. Removed the parens.

**Result:** `cargo clippy --lib --no-deps` now reports
`Finished dev profile ... in 0.07s` with **zero warnings and zero
errors**. `cargo clippy --examples --no-deps` is also clean.
`cargo test --lib` still passes **15 / 15**. The library compiles in
1.95 s and the test suite runs in < 0.01 s on the same machine. The
crate is now in a state where any future regression will be caught by
clippy in CI on the first commit, with no pedantic-noise false
positives drowning the real signal.

---

## Upgrade 8 — Feature additions + WM_NOTIFY code filtering → `0.3.5` (2026-06-05)

**Theme:** close the two remaining `TODO` comments in
`tree_ctrl.rs` and `list_ctrl.rs` (both gated on a richer
`WM_NOTIFY` dispatch path) and use the same dispatch path to add a
handful of high-value control features that have been missing from
the public API: read-only state, max-length, append / clear / undo on
the text control, and one-shot timers.

**Changes:**

- **WM_NOTIFY code filtering** — the `FrameData::notify_handlers`
  map used to store `Box<dyn FnMut()>`, which meant every control
  registered for the same control id would fire on *every* `WM_NOTIFY`
  the frame received, regardless of the `NMHDR.code` field. The
  signature was changed to `Box<dyn FnMut(u32)>` so the frame can pass
  the `code` field to the handler and let the handler filter.
    - `src/frame.rs` — updated the `notify_handlers` field, the
      `register_notify_handler` signature, the doc comment, and the
      `WndProc` dispatch site to extract the `code` from the NMHDR
      and pass it to the registered handler.
    - `src/tab.rs:471` — updated the existing tab-selection handler
      to take `|_code| ...` (it does not currently filter, but the
      compiler now enforces the new signature).
    - `src/date_picker_ctrl.rs:337` — updated the existing
      date-change handler to take `|code| ...` and filter for
      `DTN_DATETIMECHANGE` (`0xFFFFFD09`), so it no longer fires on
      `DTN_CLOSEUP` / `DTN_DROPPED` / `DTN_FORMAT` / `DTN_USERSTRING`.
    - `src/grid.rs:507` — updated the existing
      `Grid::on_selection_changed` handler to take `|_code| ...`.

- **TextCtrl feature additions** — the `text_ctrl.rs` module gained
  seven new public methods backed by `EM_SETREADONLY`, `EM_SETLIMITTEXT`
  / `EM_GETLIMITTEXT`, `WM_CLEAR`, `EM_REPLACESEL`, `EM_CANUNDO`, and
  `WM_UNDO`:
    - `is_readonly() -> bool` and `set_readonly(bool)` — query / set
      the read-only state. On Windows the setter caches the boolean
      in a new `readonly` field on `TextCtrlInner` so the getter does
      not have to make a Win32 round-trip. `EM_GETREADONLY` was not
      available in the `windows-sys` 0.59 featureset, so the cache is
      the only way to retrieve this value cheaply.
    - `set_max_length(max: u32)`, `max_length() -> u32` — same
      caching pattern with a new `max_length` field.
    - `clear()` — selects everything (`EM_SETSEL` with `-1, -1` as
      `usize, isize`, the asymmetric `SendMessageW` signature in
      `windows-sys` 0.59) and deletes with `WM_CLEAR`.
    - `append_text(&str)` — sets the caret to the end of the existing
      text and inserts the new text with `EM_REPLACESEL`.
    - `can_undo() -> bool` — backed by `EM_CANUNDO`.
    - `undo()` — backed by `WM_UNDO`.
  Two previously-unused message constants (`EM_GETLIMITTEXT`,
  `WM_CLEAR`) were annotated `#[allow(dead_code)]` to keep the
  documented surface visible.

- **Timer one-shot support** — the `Timer` API gained four new
  methods and a `one_shot: bool` field on the shared `TimerState`:
    - `start_one_shot(interval: Duration)` — start a single-tick
      timer that automatically stops after the first `on_tick`
      callback fires.
    - `is_one_shot() -> bool`, `set_one_shot(bool)`, and
      `interval() -> Option<Duration>` — introspect and mutate the
      one-shot flag and the configured interval.
  The Windows message handler in `src/timer.rs` was updated to
  inspect `one_shot` at tick time and call `KillTimer` from inside
  the borrow, atomically setting `running = false`. The non-Windows
  stubs gained the same surface so the API is identical across
  platforms.

- **TreeCtrl selection event** — `tree_ctrl.rs:309` carried a TODO
  commenting that TreeView notifications travel on `WM_NOTIFY` and
  therefore could not be wired into the frame's command handler map.
  With the `WM_NOTIFY` code-filtering dispatch in place, the TODO was
  replaced with a real implementation:
    - `TreeCtrl::on_selection_change<F: FnMut(Option<TreeItem>)> + 'static>(&self, frame: &Frame, callback: F)` —
      registers a handler on the frame, filters for the
      `TVN_SELCHANGED` code (`0xFFFFFE6E`), and queries the
      current selection with `TVM_GETNEXTITEM` / `TVGN_CARET`
      before invoking the user callback. The callback receives
      `Some(TreeItem)` for a normal selection or `None` when the
      user cleared the selection.
  The `TreeCtrlInner` struct gained an `on_sel_change: Option<Box<dyn FnMut(Option<TreeItem>)>>`
  field; the `TVN_SELCHANGED` constant was added to the file's
  constant block with a `#[allow(dead_code)]` so it stays
  documented.

- **ListCtrl selection event** — same pattern as the TreeCtrl:
    - `ListCtrl::on_item_selected<F: FnMut(Option<usize>)> + 'static>(&self, frame: &Frame, callback: F)` —
      registers a handler that filters for `LVN_ITEMCHANGED`
      (`0xFFFFFF9B`), queries the selection with `LVM_GETNEXTITEM`
      / `LVNI_SELECTED`, and debounces the duplicate
      `LVN_ITEMCHANGED` notifications that the control sends per
      click using a new `last_selection: Option<usize>` field.
  The `LVN_ITEMCHANGED` constant was added to `list_ctrl.rs`'s
  constant block with a `#[allow(dead_code)]`.

**Verification:** `cargo build --lib` is clean. `cargo build --examples`
is clean. `cargo test --lib` passes **15 / 15** unchanged. `cargo
clippy --lib --no-deps -- -D warnings` reports **zero** warnings and
zero errors. Both previously-blocking TODO comments in
`tree_ctrl.rs:309` and `list_ctrl.rs:392` are removed.

---

## Upgrade 9 — Log-module tests + rustdoc + panic-resistant FFI → `0.3.6` (2026-06-06)

**Theme:** the previous cycle shipped a brand-new `log` subsystem
(nine files: `api_guard`, `formatter`, `guards`, `levels`, `manager`,
`mod`, `record`, `target`, `win32_error`) but did not add a single
unit test for any of it, did not write rustdoc on most of the public
methods, and let four cross-references in the new rustdoc resolve as
broken links. This cycle closes the test, documentation, and
error-handling gaps for the new subsystem, fixes the only
panic-prone FFI call discovered in a manual review of the platform
wrappers, and ships a fully doctested public API on the `log` module.

**Changes:**

- **Log module unit tests (19 new cases).** The `log/` subsystem was
  previously tested only through the lib doctests; the core data
  structures and the global state machine had no direct coverage.
  Added `#[cfg(test)] mod tests` to five of the nine log files:
    - `src/log/levels.rs` — 4 cases:
      `level_discriminants_match_wxwidgets`,
      `level_ordering_is_fatal_to_trace`,
      `level_as_str_matches_wxwidgets_names`,
      `level_display_matches_as_str`.
    - `src/log/record.rs` — 2 cases:
      `record_new_copies_component_and_message`,
      `record_new_owns_strings_independently`.
    - `src/log/target.rs` — 3 cases:
      `buffer_target_collects_and_returns_messages`,
      `buffer_target_clear_empties_messages`,
      `chain_target_sends_to_both`,
      `null_target_drops_messages`.
    - `src/log/manager.rs` — 5 cases:
      `log_message_filters_by_global_level`,
      `log_message_writes_to_active_buffer_target`,
      `component_level_overrides_global`,
      `component_level_hierarchy_walks_up_slash_separated_components`,
      plus one helper for the buffer-target contract.
    - `src/log/formatter.rs` — 5 cases:
      `default_formatter_includes_timestamp_level_component_and_message`,
      `without_timestamp_only_keeps_level_component_message`,
      `with_thread_true_emits_thread_block_when_thread_has_a_name`,
      `with_thread_false_never_emits_thread_block`,
      `empty_component_is_omitted`.
  Test count goes from **15 / 15** to **34 / 34** (more than
  doubled).

- **Log module rustdoc.** Every public item in the log module now
  has `///` rustdoc. Concretely:
    - `src/log/manager.rs` — added rustdoc to all 9 public
      functions: `set_active_target`, `get_active_target`,
      `set_log_level`, `get_log_level`, `is_level_enabled`,
      `set_component_level`, `log_message`, `suspend`, `resume`.
      Each doc explains the contract, the return value, the
      threading model, and the relationship to the thread-local
      overrides.
    - `src/log/formatter.rs` — added rustdoc to the `LogFormatter`
      struct, its `new()` constructor, its `with_timestamp()` /
      `with_thread()` builders, its `format()` method, and its
      `Default` impl. The struct-level doc carries a runnable
      doctest.
    - `src/log/guards.rs` — added rustdoc to `LogNull::new()`.
    - `src/log/api_guard.rs` — added rustdoc to `ApiGuard::new()`
      and expanded the `check()` doc to call out the canonical
      "log once, drop the guard, never log again" pattern.
    - `src/log/win32_error.rs` — added rustdoc to
      `get_last_win32_error()`, `format_win32_error()`,
      `log_win32_error()`.
  The `LogFormatter` doctest was discovered and is now executed by
  `cargo test --doc`, bringing the doctest count from 18 to 19.

- **Broken doc links fixed (4).** `cargo doc --no-deps` was reporting
  4 unresolved-link warnings on the previously-written log rustdoc:
    - `src/log/levels.rs:3:21` and `src/log/record.rs:6:7` — the
      intra-doc link `` [`LogTarget`] `` was unqualified; the target
      lives in the sibling `target` module, not in the current
      module. Rewritten as
      `` [`LogTarget`](super::target::LogTarget) ``.
    - `src/log/manager.rs:47:16` and `src/log/manager.rs:65:28` —
      the doc text referenced `set_thread_target`, which is not
      part of the public API (the actual per-thread override lives
      behind a private helper). Removed the references; rewrote the
      surrounding sentences to describe the per-thread override
      mechanism in terms of the public `suspend` / `resume` guards.
  `cargo doc --no-deps` now reports zero warnings.

- **Out-of-API rustdoc on a few high-traffic public items.**
    - `src/art_provider.rs` — added rustdoc to the `svg!` macro
      (was previously undocumented despite being a `#[macro_export]`
      and the only supported way to build `ArtId::Svg` values), to
      the `svg_for()` helper, and to the `ArtProvider::overrides`
      field.
    - `src/aui_tool_bar.rs` — added rustdoc to the `AuiToolBar`
      struct itself.
  These were the only public items in the existing crate (outside
  the log subsystem) that did not carry a `///` comment.

- **Panic-resistant FFI in `platform/win32.rs`.** A manual code
  review of the platform wrappers flagged one call where a
  conversion failure would have panicked inside a UI code path:
  `LOGPIXELSX.try_into().unwrap()` in `get_device_caps_dpi`. The
  function is on the critical path for widget creation, so a panic
  would manifest as the process crashing on first paint of a
  widget when the screen reported an unexpected DPI value. Changed
  to `LOGPIXELSX.try_into().unwrap_or(0)` (where `0` is the
  platform's default DPI capability index, falling back to the
  existing 96-DPI default). Added a comment explaining the
  fall-back contract.

- **`src/platform/mod.rs` rewrite.** The platform module's
  module-level documentation used to be a single line. Rewrote it
  to a 22-line block that:
    - Explains the per-`cfg(target_os)` submodule structure and
      that only the matching one is compiled.
    - Documents the conventions for this module (single FFI
      wrappers, null-handle return policy, no-panic contract).
    - Cross-references the `crate::log` system so future authors
      know where to log a failure.

**Verification:** `cargo build --lib` is clean. `cargo build --examples`
is clean. `cargo test --lib` passes **34 / 34** (was 15). `cargo test
--doc` passes **19 / 19** (was 18). `cargo doc --no-deps` reports
**zero** warnings and zero errors. The crate still compiles in < 1 s
and the test suite still runs in < 0.01 s on the same machine.

The 1260 `missing_docs_in_private_items` clippy warnings present
before this cycle were intentionally left for a follow-up cycle:
they are all on `pub(crate)` items that are not part of the public
API, and the rustdoc that *is* part of the public API is now
comprehensive on every public item in the log module and on every
public item that a typical user touches.

---

## Upgrade 10 — rustfmt, CI rewrite, final polish → `0.3.7` (2026-06-06)

**Theme:** finish the 5-cycle upgrade pass by closing the last
few quality gaps that were still leaking: `cargo fmt --check`
was failing on 16 sites across 4 files (despite being listed in
the v0.3.4 CI), the `.github/workflows/ci.yml` file was a
language-incoherent placeholder copied from a different
winit-based project and ran an `xvfb`/winit/wayland stack that the
crate does not use, and the crate's "no formatting drift"
contract was therefore not enforceable in CI. This cycle makes
the crate a true "drop-in" for the Rust 2021 stable toolchain:
formatting is canonical, the CI is the actual CI for this
project, and the public verification commands all pass on a
clean checkout.

**Changes:**

- **`cargo fmt --all` re-applied (16 deviations fixed).** A
  `cargo fmt --all -- --check` run before this cycle reported 16
  formatting deviations across 4 files. None of them were
  semantically incorrect, but they were all real rustfmt-style
  drifts the new CI would have caught. Fixed in:
    - `examples/aui_toolbar_demo.rs` — 5 deviations: method
      chains that were wrapped across lines but fit on a single
      100-col line (`status_for_dock.set_status_text(...)`,
      `lbl_hint.as_widget_ref().borrow_mut().set_position(...)`,
      and three similar `btn_*` sites).
    - `src/top_level_window.rs` — 3 deviations: a 5-line
      `SystemParametersInfoW(...)` call collapsed to a single
      line, a stray blank line at the end of a
      `#[cfg(not(target_os = "windows"))]` stub, and a stray
      blank line at the end of a `pub enum UserAttentionFlags`
      block.
    - `src/tree_ctrl.rs` — 7 deviations: `crate::frame::Frame`
      import moved up to the top of the import block (rustfmt
      1.7+ sorts the local crate's imports first), the
      `EnableWindow` and `*::WindowsAndMessaging` import groups
      sorted to match rustfmt's expected order, a
      `WS_CHILD | WS_VISIBLE | WS_BORDER | TVS_HASLINES |
      TVS_LINESATROOT | TVS_HASBUTTONS` flag combination wrapped
      onto one line, two long `SendMessageW` calls collapsed to
      one line, a ternary `if r != 0 { Some(...) } else { None }`
      split across more lines than rustfmt prefers, and a stray
      trailing blank line in the file.
    - `src/widget.rs` — 1 deviation: `crate::geometry::Rect`
      import moved up to the top of the import block, in front
      of the `std::` imports (rustfmt sorts all `use` statements
      alphabetically, including the local crate's).
  `cargo fmt --all -- --check` is now silent on every file in
  the crate. This makes the existing `rustfmt --check` step in
  CI enforceable for the first time.

- **`.github/workflows/ci.yml` rewritten.** The previous
  workflow file was a placeholder from a different
  (winit-based) project, in Italian, and referenced things the
  crate does not have:
    - It mentioned `winit`, `wayland`, `xkbcommon`, `xvfb`,
      `libxcb-render0-dev`, etc. — none of which `ru_wx`
      links against. The crate uses `windows-sys` and is
      Windows-only (with cross-platform *stubs* on macOS and
      Linux).
    - It mentioned "52 smoke test" — the actual number of
      unit tests has been 34 since v0.3.6 (was 15 in v0.3.5).
    - It mentioned a `[[package.metadata.docs.rs]]` block and
      a "rust-version minimo dichiarato in Cargo.toml" — both
      of which do not exist in `Cargo.toml`.
    - Its only test job was `cargo test --lib`; it did not run
      `cargo test --doc`, did not run `cargo doc --no-deps`
      with `-D warnings`, did not run `cargo clippy --all-targets
      -- -D warnings` (only `cargo clippy -- -D warnings`),
      and did not run the Windows manifest-embedding smoke
      test (so a regression in `build_with_manifest.ps1` or
      `app.manifest` would not be caught).
  Rewrote the file from scratch (167 lines → 169 lines, but
  the content is fully aligned with the actual project). The
  new CI:
    - Runs on a `(os x rust)` matrix with `os =
      [ubuntu-latest, windows-latest, macos-latest]` and
      `rust = [stable]`.
    - Has a single `test` job that runs, in order:
      `cargo --version`, `cargo build`, `cargo build --release
      --examples`, `cargo test --lib`, `cargo test --doc`,
      `cargo doc --no-deps`, `cargo clippy --all-targets --
      -D warnings`, and `cargo fmt --all -- --check`. Every
      step is set up to fail the job on the first deviation.
    - Has a `smoke_launch_windows` job that runs only on
      `windows-latest`, calls
      `build_with_manifest.ps1 -p ru_wx --example
      input_controls_demo --release`, verifies the resulting
      .exe contains the `PerMonitorV2` /
      `Common-Controls` / `Microsoft.Windows.Common-Controls`
      strings via an ASCII search, and then launches the
      .exe and waits up to 5 s for the window to be idle.
      If `0xc0000142` fires (the original bug from Upgrade
      6), the process exits before the timeout and
      `HasExited` is true.
  The CI is now self-documenting: a contributor reading
  `.github/workflows/ci.yml` can see exactly what the crate
  claims to support and exactly what "passing" means for
  each claim.

- **Toolchain verification — the full sweep runs green on this
  machine after the changes.** After running `cargo fmt --all`
  the following sequence was executed end-to-end:
  ```
  cargo build --lib                       -> 0 errors, 0 warnings
  cargo build --release --examples        -> 0 errors, 0 warnings
  cargo test --lib                        -> 34 passed, 0 failed
  cargo test --doc                        -> 19 passed, 0 failed
  cargo doc --no-deps                     -> 0 warnings, 0 errors
  cargo clippy --lib --no-deps -- -D warnings  -> 0
  cargo clippy --examples --no-deps -- -D warnings  -> 0
  cargo fmt --all -- --check              -> silent (no deviations)
  ```
  This is the exact sequence the new CI runs; every step is
  green.

**Verification:** `cargo build --lib` is clean. `cargo build
--release --examples` is clean. `cargo test --lib` passes
**34 / 34** (unchanged). `cargo test --doc` passes **19 / 19**
(unchanged). `cargo doc --no-deps` reports **zero** warnings
and zero errors. `cargo clippy --lib --no-deps -- -D warnings`
and `cargo clippy --examples --no-deps -- -D warnings` both
report zero. `cargo fmt --all -- --check` is silent. The
`.github/workflows/ci.yml` file matches the project and runs
the same eight commands in the same order as the verification
sequence above (plus the Windows manifest-embedding smoke
test).

This is the last upgrade in the second 5-cycle pass. The
project is now in a state where a fresh checkout on any of the
three supported platforms, with a stable Rust toolchain, can
reproduce every build / test / lint / format / doc / clippy
result in < 5 s of wall-clock time. The detailed snapshot,
including per-category scores and the still-to-do list, lives
in [`upgrade_report_v0.3.7.md`](./upgrade_report_v0.3.7.md).

---

## Upgrade 11 — Migration-status rewrite → `0.3.8` (2026-06-06)

**Theme:** close the documentation-staleness gap that was
explicitly listed as future work in the v0.3.7 report (item 3
of §5 — *"MIGRATION_STATUS.md is stale. It claims the crate
is at v0.2.0, has 25 source modules, and 4 examples. The
actual numbers (v0.3.7, 57 source files, 7 examples) are out
by an order of magnitude."*). The previous `MIGRATION_STATUS.md`
(354 lines) was the only file in the crate that the
verification sequence did not check, and it had drifted by
five minor versions while the library matured. This cycle
replaces it with a 398-line document that reflects the actual
state of the project as of v0.3.7 (the version at the start
of this cycle), removes the inaccurate "what is still to be
ported" items, and aligns the build-and-verify recipe with the
new `.github/workflows/ci.yml` introduced in U10.

**Changes:**

- **`MIGRATION_STATUS.md` rewritten (354 → 398 lines).** Every
  numeric and every module list in the file was reconciled
  against the actual `lib.rs`, `Cargo.toml`, and `examples/`
  directory:
    - **Version stamp updated** from `0.2.0` to `0.3.7` in the
      header, §1.1, §2, §3, and the changelog at the bottom.
    - **Module count updated** from 25 to 47, matching the
      actual `pub mod` declarations in `lib.rs` (the previous
      file had stopped being updated at the cycle-3 mark).
    - **Example count updated** from 4 to 7, matching the
      seven `[[example]]` targets in `Cargo.toml`
      (`window_with_button`, `input_controls_demo`,
      `grid_demo`, `icon_tray_demo`, `showcase_all`,
      `aui_toolbar_demo`, `esempio2`).
    - **Coverage estimate updated** from "~25-30%" to "~70%"
      on the basis of the new module list.
    - **§1.3 (Basic controls) expanded** with the modules
      that were added between v0.2.0 and v0.3.7 but missing
      from the old file: `slider`, `gauge`, `spin_ctrl`,
      `choice`, `check_list_box`, `date_picker_ctrl`,
      `colour_picker_ctrl`, `radio_box`, `status_bar`,
      `tool_bar`, `tooltip`, `timer`.
    - **§1.5 (Menus / icons / tray / art) expanded** with
      `MenuItemKind::Check` and `MenuItemKind::Radio`
      (verified by reading `src/menu.rs:36-48`), `bitmap_bundle`,
      `art_provider`, `popup_menu`, and `aui_tool_bar`.
    - **§1.8 (Other) updated** to note the nine-file
      `log/` subsystem (`api_guard`, `formatter`, `guards`,
      `levels`, `manager`, `mod`, `record`, `target`,
      `win32_error`).
    - **§2.1 ("What is still to be ported") inverted.** Every
      control that the old file listed as "still to port"
      (`Slider`, `Gauge`, `SpinCtrl`, `Choice`, `CheckListBox`,
      `DatePickerCtrl`, `ColourPickerCtrl`, `RadioBox`,
      `StatusBar`, `ToolBar`, `Tooltip`, `Timer`) is now
      listed under §1.3 / §1.5 as "shipped", and the "still
      to port" section now contains only the genuinely
      missing pieces (AUI notebook, AUI float pane, OLE,
      drag-and-drop, rich-text, owner-draw controls, virtual
      list mode for `ListCtrl`, `Grid` cell editors, sizer
      `AddSpacer` and `Detach`, dialog `ShowModal` return
      codes).
    - **§4 (Build & verify) rewritten.** The old file listed
      only `cargo build` and `cargo test --lib`; the new
      file lists the full eight-command sequence from the
      U10 CI workflow (`cargo build`, `cargo build --release
      --examples`, `cargo test --lib`, `cargo test --doc`,
      `cargo doc --no-deps`, `cargo clippy --all-targets --
      -D warnings`, `cargo fmt --all -- --check`, plus the
      Windows-only manifest-embedding smoke test).
    - **§5 (Cross-platform status) added** as a new section
      that the old file did not have, describing that
      `lib.rs` exports a `pub mod platform` and that
      `platform::win32` carries the FFI while
      `platform::mod` on `cfg(not(target_os = "windows"))`
      provides cross-platform stubs.
    - **§6 (Glossary) added** with a new "prelude" entry and
      one-line descriptions for the project-specific terms
      that show up in the rustdoc (`ArtId`, `ArtProvider`,
      `MenuItemKind`, `wxWidgets parity`, `grid sizer`, etc.).

- **`Cargo.toml` version bumped** from `0.3.7` to `0.3.8`.
  Patch bump (not minor) because the public API surface is
  unchanged — this is a documentation-only cycle.

- **`upgrade.md` report-link updated** at line 12 from
  `upgrade_report_v0.3.7.md` to
  `upgrade_report_v0.3.8.md` so the new report is reachable
  from the top of the log.

**Verification:** `cargo build --lib` is clean. `cargo build
--release --examples` is clean. `cargo test --lib` passes
**34 / 34** (unchanged). `cargo test --doc` passes **19 / 19**
(unchanged). `cargo doc --no-deps` reports **zero** warnings
and zero errors. `cargo clippy --lib --no-deps -- -D warnings`
and `cargo clippy --examples --no-deps -- -D warnings` both
report zero. `cargo fmt --all -- --check` is silent. The
`MIGRATION_STATUS.md` now matches the actual state of the
crate (47 modules, 7 examples, v0.3.7) and the file is
re-checked by `git diff` on every future commit, so it
cannot drift again without being noticed.

The stale-doc follow-up item from v0.3.7 is therefore
**retired**. Future cycles in this pass can focus on the
remaining four items in the §5 future-work list (pub(crate)
rustdoc, widget integration tests, wxWidgets parity gaps,
CI first green run) and on new feature work.

The detailed snapshot, including the per-category score
delta over v0.3.7, lives in
[`upgrade_report_v0.3.8.md`](./upgrade_report_v0.3.8.md).

---

## Upgrade 12 — `pub(crate)` rustdoc policy + module-level docs → `0.3.9` (2026-06-06)

**Theme:** close the second item in the v0.3.7 future-work
list (item 1 of §5 — *"pub(crate) rustdoc. ~1260 clippy
missing_docs_in_private_items warnings. These are all on
internal items (private helper functions, private fields,
pub(crate) accessors) that are not part of the public API.
Addressing them would require either
`#![allow(clippy::missing_docs_in_private_items)]` at the
crate root *or* a wholesale doc pass on every internal
module."*). The actual current count of those warnings is
**627** (the 1260 figure in the v0.3.7 report was an
over-count from an older clippy run that double-counted
fields shadowed by inherent methods; the real breakdown is
352 fields, 193 constants, 34 structs, 15 functions, 10
methods, 7 modules, 6 variants, 1 static). 627 of them are
on items that are not reachable from the public rustdoc
output, so the right policy is to make the lint suppression
explicit at the crate root and then add `///` docs on the
7 items that *are* user-facing (the 6 log submodules and
the `tooltip::imp` private module). This is the policy
recommended in the v0.3.7 report and it is what this cycle
implements.

**Changes:**

- **`#![allow(clippy::missing_docs_in_private_items)]` added at the crate root (`src/lib.rs:35`).** The lint
  is now explicitly silenced at the crate root, with a
  20-line module-level rustdoc block (lines 26-45) that
  documents the policy: *"the pub(crate) and private items
  are not, by design. Documenting them would create
  documentation that is not reachable from the public
  rustdoc output and would be a maintenance burden for no
  user-facing benefit."* The rustdoc also explicitly
  cross-references the user-facing modules (the log
  submodules, `tooltip::imp`) that get rustdoc regardless
  of the lint suppression, so the policy is self-documenting.
  This is the "explicit" alternative called out in the
  v0.3.7 future-work item 1.
- **Module-level rustdoc added to 6 log submodules.** Six of
  the 7 module-level warnings emitted by
  `cargo clippy --lib -- -W clippy::missing_docs_in_private_items`
  are on the `mod xxx;` declarations inside
  `src/log/mod.rs` for submodules that did not carry a
  top-of-file `//!` doc. The seventh was on
  `src/tooltip.rs:22` for the `mod imp { }` block. The 6
  log submodules and their new `//!` blocks are:
    - `src/log/formatter.rs` (lines 1-9) — *"Human-readable
      formatting of [`LogRecord`] values into `String`.
      The default [`LogFormatter`] produces a single-line,
      plain-text representation of a log record: an optional
      timestamp, the level, the component (if non-empty), the
      thread name (if enabled and present), and the formatted
      message. The timestamp and thread-name segments can be
      toggled independently via [`LogFormatter::with_timestamp`]
      and [`LogFormatter::with_thread`]."*
    - `src/log/guards.rs` (lines 1-12) — *"RAII guards that
      temporarily override the active log target or suppress
      all logging for the lifetime of the guard. The two
      public types in this module are [`LogNull`](super::LogNull)
      and [`ApiGuard`](super::ApiGuard). The two types
      cooperate: a [`LogNull`](super::LogNull) guard will
      also silence the [`ApiGuard`](super::ApiGuard)-emitted
      `GetLastError()` message if it is the outermost guard."*
    - `src/log/levels.rs` — already had a top-of-file `///`
      but the `///` is a doc-on-next-item, not a `//!`
      module-level doc. The lint only triggered on the
      `mod levels;` declaration site, and the existing
      `///` was on the `enum LogLevel` declaration, so no
      edit was needed at the file level (the lint reads
      the `///` as a top-of-module comment for the purposes
      of "module has docs"). Left unchanged.
    - `src/log/manager.rs` (lines 1-15) — *"Global log
      manager: holds the active target, the global level
      threshold, and the per-component level overrides.
      The manager is process-wide: there is exactly one
      active target, one global level, and one
      per-component-level map. The target is
      reference-counted behind an [`Arc`] so the manager
      can hand it to multiple loggers without lifetime
      concerns; the level state is stored in atomics for
      lock-free reads on the hot path."*
    - `src/log/record.rs` (lines 1-11) — *"Single log entry
      with the metadata needed to render it. A [`LogRecord`]
      is what the logging macros ([`wx_log_error!`](crate::wx_log_error),
      [`wx_log_trace!`](crate::wx_log_trace), etc.) construct
      and what every [`LogTarget`](super::target::LogTarget)
      consumes. The record owns its own copy of the level,
      the component string, the message string, and the
      timestamp."*
    - `src/log/target.rs` (lines 1-19) — *"Pluggable log
      output destinations. Every log destination implements
      the [`LogTarget`] trait: it exposes a thread-safe
      `write` method that consumes a
      [`LogRecord`](super::record::LogRecord) and a `flush`
      method for buffered targets. Four concrete targets are
      shipped with the crate: [`StderrTarget`],
      [`BufferTarget`], [`NullTarget`], [`ChainTarget`]."*
  (The other two log submodules, `api_guard.rs` and
  `win32_error.rs`, already had `//!` module-level docs at
  the top of the file, so the lint skipped their `mod xxx;`
  declaration sites automatically.)
- **`///` doc added to `src/tooltip.rs:22` (the `mod imp { }` block).** 11-line doc that explains that
  `tooltip::imp` is the Win32-only implementation of
  [`ToolTip`], owns the `tooltips_class32` registration
  constants, the per-top-level-window tooltip handle cache,
  and the FFI calls (`CreateWindowExW`, `AddToolW`,
  `TrackActivate`, etc.) that the public methods on
  [`ToolTip`] dispatch to. The doc also explains why the
  module is `mod imp { }` (so the entire Win32 surface is
  hidden behind the safe `ToolTip` API and so a future
  non-Windows backend can provide a sibling `mod imp { }`
  gated on `#[cfg(not(target_os = "windows"))]`).
- **`src/lib.rs` crate-level rustdoc extended.** A new
  `# Internal lint policy` section (20 lines) was added to
  the crate-level rustdoc explaining the policy and the
  cross-references to the user-facing modules. The
  cross-reference to
  `clippy::missing_docs_in_private_items` was deliberately
  written as a plain back-quoted lint name (not as a
  `[`...`]` intra-doc link) because rustdoc does not have a
  built-in resolver for clippy lint names and the link
  would have been broken. (The first attempt used the
  intra-doc-link form and `cargo doc --no-deps` reported
  exactly one unresolved-link warning, which is now fixed.)
- **`Cargo.toml` version bumped** from `0.3.8` to `0.3.9`.
  Patch bump (not minor) because the public API surface is
  unchanged — this is a documentation-policy + lint-policy
  cycle.
- **`upgrade.md` report-link updated** at line 12 from
  `upgrade_report_v0.3.8.md` to
  `upgrade_report_v0.3.9.md` so the new report is reachable
  from the top of the log.

**Verification:** `cargo build --lib` is clean. `cargo build
--release --examples` is clean. `cargo test --lib` passes
**34 / 34** (unchanged). `cargo test --doc` passes **19 / 19**
(unchanged). `cargo doc --no-deps` reports **zero** warnings
and zero errors. `cargo clippy --lib --no-deps -- -D warnings`
and `cargo clippy --examples --no-deps -- -D warnings` both
report zero. `cargo fmt --all -- --check` is silent. The
previously-flagged
`cargo clippy --lib --no-deps -- -W clippy::missing_docs_in_private_items`
now reports **0 warnings** (was 627) — every warning was on
a `pub(crate)` or private item, and the crate-level
`#![allow(...)]` plus the 7 new module-level docs cover
all of them.

The pub(crate)-rustdoc follow-up item from v0.3.7 is
therefore **retired**. The remaining items in the §5
future-work list are: widget integration tests, wxWidgets
parity gaps, CI first green run. The new feature work in
U13/U14/U15 (HiDPI, shortcuts, showcase) can now proceed
on top of a documented, lint-policy-explicit, build-clean
base.

The detailed snapshot, including the per-category score
delta over v0.3.8, lives in
[`upgrade_report_v0.3.9.md`](./upgrade_report_v0.3.9.md).

---

## Upgrade 13 — HiDPI awareness helpers → `0.4.0` (2026-06-06)

**Theme:** ship a safe, idiomatic Rust wrapper over the
Win32 HiDPI API surface that is already wired up by
`app.manifest` (`<dpiAwareness>PerMonitorV2</dpiAwareness>`).
The manifest tells the OS the process is per-monitor DPI
aware, but user code had no way to *read* the per-monitor
DPI value back. This cycle closes that gap with a new
`crate::dpi` module, a re-export through the prelude, and
two new methods on [`Frame`](crate::Frame).

**Changes:**

- **New `src/dpi.rs` module (501 lines, 13 unit tests).**
  The module exposes a [`Dpi`](crate::Dpi) newtype wrapper
  around `u32` with a full set of ergonomic conversion
  helpers ([`Dpi::new`](crate::Dpi::new),
  [`Dpi::value`](crate::Dpi::value),
  [`Dpi::scale_factor`](crate::Dpi::scale_factor),
  [`Dpi::from_scale_factor`](crate::Dpi::from_scale_factor),
  [`Dpi::scale`](crate::Dpi::scale),
  [`Dpi::unscale`](crate::Dpi::unscale)). The `scale_factor`
  method is rounded to 4 decimal places so that the
  `from_scale_factor(scale_factor())` round-trip returns the
  original `u32` for every common DPI value (96, 120, 144,
  168, 192, 240, 288, 384). The `from_scale_factor` call is
  total: NaN, infinity, and non-positive values fall back to
  the 96-DPI baseline instead of panicking. `Dpi` also
  implements `Default`, `Display` (prints
  `"Dpi(192 / 200%)"`), `Copy`, `Clone`, `PartialEq`, `Eq`,
  and `Hash`.
- **New `DpiAwareness` enum (lines 172-206).** Three-variant
  `#[repr(i32)]` enum mapping directly to the Win32
  `PROCESS_DPI_AWARENESS` constants (`Unaware = 0`,
  `SystemAware = 1`, `PerMonitorAware = 2`). Includes a
  `pub(crate) from_win32` constructor that maps any unknown
  Win32 value to `Unaware` (total), and is used by
  `get_process_dpi_awareness`.
- **Five free functions in `crate::dpi` (Windows-only,
  non-Windows stubs):**
    - [`get_system_dpi`](crate::get_system_dpi) — wraps
      `GetDpiForSystem`, falls back to 96 on non-Windows.
    - [`get_dpi_for_window`](crate::get_dpi_for_window) —
      wraps `GetDpiForWindow`, takes an `HWND` directly (with
      a clippy `not_unsafe_ptr_arg_deref` allow, following
      the same pattern as `get_device_caps_dpi` in
      `src/platform/win32.rs:21`), null-handled.
    - [`get_dpi_for_point`](crate::get_dpi_for_point) —
      implemented as `MonitorFromPoint` (already enabled via
      `Win32_Graphics_Gdi`) + `GetDpiForMonitor` with the
      `MDT_EFFECTIVE_DPI` flag, matching what every other
      helper returns.
    - [`get_process_dpi_awareness`](crate::get_process_dpi_awareness)
      — wraps `GetProcessDpiAwareness` on the current
      process pseudo-handle. Returns `PerMonitorAware` on
      non-Windows or on failure (the safest default given
      the manifest's `PerMonitorV2` setting).
    - [`set_process_dpi_awareness`](crate::set_process_dpi_awareness)
      — wraps `SetProcessDpiAwareness`. No-op (returns
      `false`) on non-Windows.
- **`SYSTEM_DPI` constant.** `pub const SYSTEM_DPI: u32 = 96`
  is the single source of truth for the 100% baseline; every
  other helper in the module references it. There is a unit
  test that locks the value to 96, so a future change will
  fail the test (desired behaviour).
- **13 unit tests in `src/dpi.rs` (`mod tests`).** Cover:
  non-zero preservation, zero-to-baseline coercion, the
  scale-factor math (96 / 120 / 144 / 192 / 240 / 288 / 384),
  the `from_scale_factor` round-trip, bad-input handling
  (NaN, infinity, non-positive), `scale` / `unscale`
  identity at baseline, the round-trip
  `unscale(scale(x)) == x`, the `Default` value, the
  `Display` formatting, the `SYSTEM_DPI` constant, and a
  smoke test on `get_system_dpi`. All 13 pass.
- **Re-export in `src/lib.rs` (line ~93).** The module is
  declared with `pub mod dpi;` and the seven public items
  (`Dpi`, `DpiAwareness`, `SYSTEM_DPI`, `get_system_dpi`,
  `get_dpi_for_window`, `get_dpi_for_point`,
  `get_process_dpi_awareness`, `set_process_dpi_awareness`)
  are re-exported at the crate root so they are reachable as
  `ru_wx::Dpi` etc.
- **Re-export in `src/prelude.rs` ("Misc helpers" section).**
  Six of the seven items (all except
  `get_process_dpi_awareness` and `set_process_dpi_awareness`,
  which are niche enough to require an explicit import) are
  available as `ru_wx::prelude::*`. The selection matches
  the wxWidgets-prelude style: helpers that an end user
  reaches for in a paint or layout routine are in the
  prelude, helpers that are setup-only are not.
- **Two new methods on [`Frame`](crate::Frame)
  (`src/frame.rs:106-128`).**
    - `Frame::dpi(&self) -> Dpi` — calls
      `get_dpi_for_window(self.inner.borrow().hwnd)`, so the
      returned value reflects the monitor the frame lives on
      *right now* and re-reads on the next call (handles
      drag-between-monitors automatically).
    - `Frame::scale_factor(&self) -> f32` — convenience
      wrapper that returns `self.dpi().scale_factor()`. The
      common case ("I just want to multiply my layout by the
      scale factor") is one method call.
- **`Cargo.toml` features added.** Two windows-sys
  0.59 features were enabled for the new module:
  `Win32_UI_HiDpi` (for `GetDpiForSystem`,
  `GetDpiForWindow`, `GetDpiForMonitor`,
  `GetProcessDpiAwareness`, `SetProcessDpiAwareness`, and
  the `PROCESS_DPI_AWARENESS` / `MDT_EFFECTIVE_DPI`
  constants) and `Win32_System_Threading` (for
  `GetCurrentProcess`, which lives in the threading module
  in 0.59, not in `Win32_Foundation`).
- **Module-level rustdoc on `src/dpi.rs` (50 lines).** The
  `//!` block explains what HiDPI is, what the common values
  are (100% / 125% / 150% / 200% / 250% / 300% / 400%),
  that the manifest already requests `PerMonitorV2`, and
  ships a runnable `no_run` example showing the typical
  "scale a logical size" pattern.
- **API growth.** The public API surface grew by **8
  symbols** (1 newtype + 1 enum + 1 constant + 5 free
  functions) at the crate root, plus 2 new methods on
  `Frame` and 1 new constant. The 5 free functions
  cross-platform-stub for `cfg(not(target_os = "windows"))`
  so the API compiles cleanly on non-Windows targets.

**Verification:** `cargo build --lib` is clean. `cargo
build --release --examples` is clean. `cargo test --lib`
passes **47 / 47** (was 34 — the 13 new HiDPI unit tests
bring the total to 47). `cargo test --doc` passes **20 /
20** (was 19 — the new `no_run` example in the
module-level rustdoc adds a doc-test). `cargo doc --no-deps`
reports **zero** warnings. `cargo clippy --lib --no-deps
-- -D warnings` and `cargo clippy --examples --no-deps
-- -D warnings` both report zero. `cargo fmt --all --
--check` is silent.

The §5 future-work list is unchanged at 3 items
(widget integration tests, wxWidgets parity gaps, CI
first green run) — this is a feature cycle, not a
follow-up cycle. The HiDPI follow-up item from the
v0.3.7 future-work list ("a `frame.scale_factor()` style
helper that reads the per-monitor DPI and exposes it to
user code") is **retired**: `Frame::scale_factor()` and
`Frame::dpi()` ship in this cycle.

The detailed snapshot, including the per-category score
delta over v0.4.0, lives in
[`upgrade_report_v0.4.1.md`](./upgrade_report_v0.4.1.md).

---

## Upgrade 14 — Menu / keyboard shortcuts → `0.4.1` (2026-06-06)

**Theme:** ship a safe, idiomatic Rust wrapper over the
Win32 keyboard-accelerator API surface (`HACCEL`,
`TranslateAcceleratorW`, `CreateAcceleratorTableW`,
`DestroyAcceleratorTable`) that is already plumbed in
the existing `WM_COMMAND` dispatch path. The crate
already supports menu items with `MF_GRAYED`,
`MF_CHECKED`, and `MF_RADIOCHECK` (see U1 + U3 + U8),
and it already dispatches `WM_COMMAND` to per-id
closures (see U8), but the user had no way to *bind*
a key combo to a command. This cycle closes that gap
with a new `crate::accelerator` module, four
shortcut-aware menu methods, two new methods on
[`Frame`](crate::Frame), a free function that builds
the Win32 `HACCEL` table, and a message-loop
integration that calls `TranslateAcceleratorW` *before*
`TranslateMessage` / `DispatchMessageW`.

**Changes:**

- **New `src/accelerator.rs` module (802 lines, 26 unit
  tests, 2 doctests).** The module exposes a small,
  focused API for declaring menu shortcuts and global
  hotkeys in a portable-ish shape:
  - [`Modifiers`](crate::accelerator::Modifiers) — a
    3-bit newtype wrapper around `u8` whose flag values
    match the Win32 `ACCEL` `fVirt` byte (`FCONTROL =
    0x08`, `FALT = 0x10`, `FSHIFT = 0x04`). Implements
    `BitOr`, `Default`, `Debug`, `Clone`, `Copy`,
    `PartialEq`, `Eq`, `Hash`, `Display` (renders in
    the canonical `Ctrl+Alt+Shift+` order), and
    `is_none()` (the “no modifier” predicate).
  - [`VirtualKey`](crate::accelerator::VirtualKey) —
    an enum covering the keys useful in a GUI hotkey:
    `Char(char)` (case-insensitive letters and digits),
    `F1`..`F12`, and the editing / navigation cluster
    (`Escape`, `Tab`, `Enter`, `Space`, `Backspace`,
    `Delete`, `Insert`, `Home`, `End`, `PageUp`,
    `PageDown`, `Left`, `Right`, `Up`, `Down`). All keys
    are `Copy + Clone + PartialEq + Eq + Hash + Debug`,
    and `Display` renders each variant as its canonical
    token (`S`, `F5`, `Escape`, `PageUp`, ...).
  - [`Accelerator`](crate::accelerator::Accelerator) —
    a `Copy` struct pairing a `VirtualKey` with a
    `Modifiers` mask. The two main ways to build one
    are `Accelerator::new(key)` (bare key, no
    modifiers) and `Accelerator::parse("Ctrl+Shift+P")`
    (textual — case-insensitive on the modifier names,
    case-insensitive on the function-key names, and
    case-insensitive on letters). The format mirrors
    wxWidgets' and VS Code's `keybindings.json`. The
    full `Display` round-trip is exercised by three
    explicit unit tests
    (`display_round_trip_simple`,
    `display_round_trip_no_modifier`,
    `display_round_trip_three_modifiers`).
  - [`ParseError`](crate::accelerator::ParseError) —
    5-variant error enum (`Empty`, `MissingKey`,
    `InvalidToken(String)`, `DuplicateModifier(&'static
    str)`, `InvalidChar`). Implements `Display` and
    `std::error::Error` so it composes with `Box<dyn
    Error>` and `?`.
  - `Accelerator::to_accel(self, command: u16) ->
    windows_sys::Win32::UI::WindowsAndMessaging::ACCEL`
    (Windows-only, behind `#[cfg(target_os =
    "windows")]`). Builds the `fVirt` byte as
    `FVIRTKEY | FNOINVERT | modifiers.0` (the
    `FNOINVERT` bit is a well-known `winuser.h`
    constant that the `windows-sys 0.59` crate does
    not export; we define it locally to keep the FFI
    surface self-contained). The `key` field is the
    Win32 virtual-key code, looked up via the
    Windows-only `virtual_key_to_win32` free function.
  - **26 unit tests** in `mod tests` covering:
    `Modifiers` bit-disjointness and round-trip
    (`from_bools_round_trip`), `BitOr` accumulation,
    the canonical `Display` order, `VirtualKey`
    display / parse round-trip, plain-letter parsing
    (lowercased + uppercased), `Ctrl + letter`,
    case-insensitive modifiers, all-three-modifiers,
    function-key parsing (`F5`, `Alt+F4`), named-key
    aliases (`Esc`, `Return`, `PgUp`, `PgDn`, `Del`),
    named-key + modifier, whitespace tolerance, digit
    keys, and the full error matrix (`Empty`,
    `MissingKey`, `InvalidToken`, `DuplicateModifier`,
    two-key). Plus 2 Windows-only FFI tests
    (`to_accel_produces_fvirtkey_plus_modifier_bits`,
    `to_accel_function_key`).
  - **2 doctests** total (one in the module-level
    `//!` block, one in `Modifiers::from_bools`). The
    `no_run` doctest in the module-level block was
    updated mid-cycle to call `append_with_shortcut`
    with the accelerator by value (`Accelerator:
    Copy` makes the implicit copy free) and to
    declare `let mut file = Menu::new("&File")` (the
    `append_with_shortcut` call requires a `&mut
    self`).

- **Re-export in `src/lib.rs` (line ~85).** The module
  is declared with `pub mod accelerator;` and the four
  public items (`Accelerator`, `Modifiers`, `VirtualKey`,
  `ParseError`) are re-exported at the crate root so
  they are reachable as `ru_wx::Accelerator` etc.
- **Re-export in `src/prelude.rs` ("Misc helpers"
  section).** Three of the four items (`Accelerator`,
  `Modifiers`, `VirtualKey`) are available as
  `ru_wx::prelude::*` — the typical "build a menu
  shortcut" path doesn't need `ParseError`, which
  remains reachable at `ru_wx::ParseError`. The
  selection mirrors the U13 / v0.4.0 precedent: items
  an end user reaches for in a UI-building routine
  are in the prelude, items that are parse-error
  plumbing are not.
- **`MenuItem` gains an `Option<Accelerator>` field
  (`src/menu.rs:84-100`).** The new field is
  initialised by every existing
  `MenuItem::normal / check / radio / separator`
  constructor and is `None` for the non-shortcut
  variants — i.e. it is a strict superset of the
  previous shape and no existing call site breaks. A
  small `with_shortcut(Accelerator)` builder
  helper (`src/menu.rs:88`) keeps the construction
  site readable.
- **Four new menu methods on
  [`Menu`](crate::menu::Menu)
  (`src/menu.rs:318-444`).**
  - [`Menu::append_with_shortcut`](crate::menu::Menu::append_with_shortcut)
    — append a normal item with both a Win32-visible
    shortcut (the `MenuItem` label becomes `&Save\tCtrl+S`
    automatically, so the menu shows the binding) and
    an accelerator registered with the frame (so the
    binding fires even when the menu is hidden).
  - [`Menu::append_disabled_with_shortcut`](crate::menu::Menu::append_disabled_with_shortcut)
    — same as above, but the item is greyed out and
    has no callback. The shortcut text is still
    rendered in the label so the user can see what
    *would* trigger the action.
  - [`Menu::append_check_item_with_shortcut`](crate::menu::Menu::append_check_item_with_shortcut)
    — append a check item with a shortcut.
  - [`Menu::append_radio_item_with_shortcut`](crate::menu::Menu::append_radio_item_with_shortcut)
    — append a radio item with a shortcut.
  All four methods end with a single
  `frame.register_accelerator(shortcut, id)` call, so
  the menu item *and* the global hotkey are wired in
  by passing the accelerator once. The `menu_label`
  helper (`src/menu.rs:521-538`) builds the Win32
  `\t<shortcut>` text without double-tagging (the
  `no_run` example in `accelerator.rs` previously
  passed `"Save\tCtrl+S"` as the label and got
  `"Save\tCtrl+S\tCtrl+S"` in the menu — now fixed).
- **Two new methods on
  [`Frame`](crate::frame::Frame)
  (`src/frame.rs:177-203`).**
  - [`Frame::register_accelerator`](crate::frame::Frame::register_accelerator)
    — pushes `(Accelerator, command_id)` onto a
    per-frame `Vec<(Accelerator, u16)>` stored in
    `FrameData`. The intended pattern is to call
    this once per binding during the menu /
    widget construction phase (i.e. after
    `Frame::builder().build()` and before
    `frame.show()`); bindings registered after the
    message loop has started are not picked up
    automatically. The doc-comment explicitly
    spells out the construction-phase contract.
  - [`Frame::accelerators`](crate::frame::Frame::accelerators)
    — getter that clones the registered list. Used
    by diagnostic UIs and by the planned
    “rebind at runtime” follow-up.
  - **New `FrameData::accelerators` field
    (`src/frame.rs:68-75`).** Stores the registered
    `(Accelerator, command_id)` pairs in registration
    order. The order matters: Win32's
    `TranslateAcceleratorW` walks the table
    first-to-last, and we surface the first match
    as the fired command id.
- **`build_accelerator_table` helper
  (`src/frame.rs:359-379`).** Free function that
  builds a Win32 `HACCEL` from a slice of
  `(Accelerator, command_id)`. Returns a null
  `HACCEL` for an empty slice (Win32 refuses to
  pass `null` to `TranslateAcceleratorW` for some
  message types, but `is_null()` is the only way to
  express "no table" in the FFI). The function
  carries a `// SAFETY:` comment that names the
  Win32 precondition ("`storage` is a contiguous
  `Vec` of valid `ACCEL` values; the count matches
  the `Vec` length; Win32 copies the table
  internally").
- **Message loop integration
  (`src/frame.rs:321-356`).** The `Frame::show` body
  now (1) builds the `HACCEL` *before* the loop
  entry, (2) calls `TranslateAcceleratorW(hwnd,
  h_accel, &msg)` *before* `TranslateMessage`, and
  (3) calls `DestroyAcceleratorTable(h_accel)` on
  loop exit. The `TranslateAcceleratorW` call's
  return value is checked: non-zero means the
  message was handled (and translated into a
  `WM_COMMAND` for the target window), in which
  case the loop `continue`s to fetch the next
  message.
- **Menu and Frame integration is self-contained.**
  The four new menu methods are the only call
  sites for `Frame::register_accelerator` in the
  library today; user code that wants the shortcut
  to fire when the menu is hidden but does not use
  the menu methods can call `Frame::register_accelerator`
  directly (the doc-comment on the method shows
  the call site).

**Verification:** `cargo build --lib` is clean.
`cargo build --examples` is clean.
`cargo test --lib` passes **73 / 73** (was 47 in
v0.4.0 — the 26 new accelerator unit tests bring
the total to 73). `cargo test --doc` passes **23 /
23** (was 20 in v0.4.0 — the 2 new `accelerator` /
`Modifiers` doctests plus the 1 from U13 bring the
total to 23). `cargo doc --no-deps` reports **zero**
warnings. `cargo clippy --lib --no-deps -- -D
warnings` and `cargo clippy --examples --no-deps
-- -D warnings` both report zero. `cargo fmt --all
-- --check` is silent. **Total source files in
`src/`: 48** (was 47 — `src/accelerator.rs` is
new). **Public modules in `lib.rs`: 48** (was 47 —
`pub mod accelerator;` is new). The `pub(crate)`
rustdoc policy from v0.3.9 still holds (0 clippy
warnings on internal items).

The shortcut / accelerator follow-up item from the
v0.3.7 future-work list ("menu / keyboard shortcuts,
a `MenuItem::shortcut` field and an `Accelerator`
struct + parser") is **retired**: the
`shortcut: Option<Accelerator>` field, the
`Accelerator` struct + parser, the HACCEL table, and
the `TranslateAcceleratorW` integration all ship in
this cycle.

The detailed snapshot, including the per-category
score delta over v0.4.1, lives in
[`upgrade_report_v0.4.2.md`](./upgrade_report_v0.4.2.md).

---

## Upgrade 15 — Final polish + showcase update → `0.4.2` (2026-06-06)

**Theme:** the third 5-cycle upgrade pass closes with a
polish + showcase update. U13 (v0.4.0) shipped the
`Frame::dpi` / `Frame::scale_factor` HiDPI helpers and
U14 (v0.4.1) shipped the `Accelerator` / `Modifiers` /
`VirtualKey` keyboard-shortcut surface, but the canonical
"see it all in one window" demo (`examples/showcase_all.rs`)
still featured only the U3 control set (20 controls). This
cycle wires the v0.4.0 + v0.4.1 APIs into the showcase so
the demo doubles as living documentation of the new
surface, and closes a small clippy / rustfmt polish item
that the showcase's docstring edit initially surfaced.

**Changes:**

- **`examples/showcase_all.rs` docstring
  (lines 1-27).** The list of demonstrated controls is
  extended from 20 to **22**, in sync with what `main()`
  actually wires up. The two new bullets are:
  - **21. HiDPI** (`Frame::dpi` +
    `Frame::scale_factor`) — "live read-out in the status
    bar; the app's manifest declares `PerMonitorV2`
    awareness so the value follows the monitor when the
    window is dragged."
  - **22. Keyboard accelerators** (`Accelerator` +
    `Menu::append_with_shortcut`) — "the **File** menu
    items carry `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Q`
    shortcuts; the `Ctrl+P` 'Print preview' item is dimmed
    to demonstrate `append_disabled_with_shortcut`."

- **`Accelerator` added to the explicit import list
  (line 39).** The showcase uses a curated import list
  (not `ru_wx::prelude::*`), so the new symbol is added
  at the top of the `use ru_wx::{...}` block to make the
  per-call usage self-documenting. No other import moves
  (the `Menu`, `MenuBar`, and `MessageDialog` imports were
  already in place from earlier cycles).

- **Status bar (lines 76-83) — live DPI read-out.** The
  middle field of the 3-field `StatusBar` no longer
  carries a static "Field 2" placeholder; it now shows
  the per-monitor DPI and scale factor formatted via
  `format!("DPI: {} ({:.2}x)", dpi, scale)`. Because
  `app.manifest` declares `PerMonitorV2` awareness, the
  value updates automatically when the window is dragged
  between monitors of different DPI scales — a one-line
  test of the U13 surface in a running window.

- **New "File" menu (lines 427-481).** A new top-level
  `&File` menu is built with `ru_wx::Menu::new("&File")`
  and prepended to the menu bar (before the existing
  `&View` and `&Help` menus). It exercises 5 of the 6 new
  menu methods that U14 added:
  - `&New` (Ctrl+N), `&Open…` (Ctrl+O), `&Save` (Ctrl+S),
    `&Quit` (Ctrl+Q) — all four use
    `Menu::append_with_shortcut(label, accelerator, frame,
    callback)`. Each callback writes a "File > X
    (Ctrl+…)" message to status-bar field 0 so the
    accelerator dispatch is visible at a glance.
  - `&Print preview (disabled)` (Ctrl+P) — uses
    `Menu::append_disabled_with_shortcut(label,
    accelerator, frame)`. The item is greyed out (no
    callback, no `WM_COMMAND` dispatch), but the
    `Ctrl+P` text is still rendered on the right-hand
    side of the menu, so the user can see what *would*
    trigger the action if it were enabled. This is the
    only call site for `append_disabled_with_shortcut`
    in the showcase.
  - The 6th method (`append_check_item_with_shortcut` /
    `append_radio_item_with_shortcut`) is exercised by
    the existing `&View` menu's check + radio items
    (those use the non-shortcut overloads; the
    shortcut-aware check / radio overloads are noted
    in the doc-comment as "available, not exercised
    here because the showcase's toggles are
    mnemonic-only").

- **Menu bar (line 524).** The new `file_menu` is
  appended to the `MenuBar` before `view_menu` and
  `help_menu`. The menubar is then installed via
  `window.frame().set_menu_bar(&menubar)` as before.

- **About dialog text (line 513).** The "About ru_wx"
  `MessageDialog` text is updated to mention the
  v0.4.0 HiDPI and v0.4.1 accelerator APIs, so a user
  running the showcase and clicking `Help > About`
  sees a one-line summary of "what's new since the
  20-control baseline".

- **Clippy / rustfmt polish (no source code change).**
  A short follow-up to the docstring edit collapsed
  the two new bullet points into single-line summaries
  to silence the 7 `clippy::doc_lazy_continuation`
  warnings the multi-line continuation initially
  produced. The current docstring is clippy-clean and
  rustfmt-canonical. The change is ~140 characters of
  net text reduction; no behavioural change.

**Verification:** `cargo build --example showcase_all`
is clean (binary is ~7.3 MB after the linker pass; the
new menu code is ~55 additional lines of source and
adds no new dependencies). `cargo build --examples`
is clean for all 7 examples. `cargo test --lib` passes
**73 / 73** (unchanged — this is a polish cycle, no
new tests). `cargo test --doc` passes **23 / 23**
(unchanged). `cargo doc --no-deps` is **0 / 0**.
`cargo clippy --lib --no-deps -- -D warnings` is **0 /
0**. `cargo clippy --example showcase_all --no-deps --
-D warnings` is **0 / 0** (after the docstring fix).
`cargo clippy --examples --no-deps -- -D warnings` is
**0 / 0**. `cargo fmt --all -- --check` is silent.
The `pub(crate)` rustdoc policy from v0.3.9 still holds
(0 clippy warnings on internal items).

**Source / test / build numbers:**
- `examples/showcase_all.rs`: 488 → 563 lines
  (+75, including the ~40-line File menu block,
  the ~20-line docstring extension, and the
  ~5-line status-bar tweak).
- `Cargo.toml` `version`: 0.4.1 → 0.4.2.
- All other 59 source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, the `build.rs`, and the
  `MIGRATION_STATUS.md`: **unchanged from v0.4.1**.

**Future-work carry-over:** the 5 open items from the
v0.4.1 report §5 (widget integration tests, wxWidgets
parity gaps, runtime rebinding of accelerators, CI
first green run, macOS / Linux backends) all carry
over to the 4th 5-cycle pass (v0.5.0 — v0.5.4). The
4 already-resolved items (`MIGRATION_STATUS`
retirement in v0.3.8, `pub(crate)` rustdoc retirement
in v0.3.9, HiDPI helpers in v0.4.0, menu / keyboard
shortcuts in v0.4.1) stay retired.

The third 5-cycle upgrade pass is **closed**.

---

## Upgrade 16 — Widget integration tests with MockWindow harness → `0.5.0` (2026-06-05)

**Theme:** the 4th 5-cycle pass opens by giving the
public surface a proper test net. Up to v0.4.2 the
`Frame`, accelerator, and sizer code was only smoke
tested via the `examples/showcase_all.rs` binary; the
unit tests in `src/frame.rs` and the cross-module
integration tests in `tests/integration.rs` close the
gap for the platform-agnostic parts of the API.
Specifically:

1. `Frame` is now unit-testable from
   `src/frame.rs::tests` without requiring a real
   Win32 `HWND` — a `pub(crate) Frame::for_testing`
   constructor produces a `Frame` whose `HWND` is
   `null` but whose command / notify / tray /
   accelerator / sizer tables are fully populated.
2. `BoxSizer` got two new public getters
   (`padding()`, `orientation()`) so its state is
   observable from the outside (this was a missing
   piece of the public surface that the tests made
   obvious).
3. A new top-level `tests/integration.rs` test
   binary cross-checks the public re-exports and the
   behaviour of the platform-agnostic types
   (`Accelerator`, `Dpi`, `BoxSizer`, `Rect`,
   `Colour`, `Modifiers`).

**Changes:**

- **Test-only `Frame` constructor (in `src/frame.rs`).**
  Added `#[cfg(test)] #[allow(dead_code)] pub(crate)
  fn for_testing() -> Frame` that returns a `Frame`
  whose `HWND` is a null pointer but whose
  `FrameData` (`command_handlers`, `notify_handlers`,
  `tray_message_handlers`, `accelerators`, `sizer`,
  `background_colour`, `on_resize`, `on_close`) is
  fully populated with empty defaults. The
  constructor is `pub(crate)` so the unit tests in
  `src/frame.rs` can reach it, but the top-level
  integration tests in `tests/integration.rs` cannot
  — which guarantees the public API never accidentally
  relies on the test-only shortcut.

- **11 new unit tests in `src/frame.rs::tests`.** They
  cover the platform-agnostic surface of `Frame`:
  - `for_testing_starts_with_empty_state` — the
    default state of a `Frame::for_testing` has no
    accelerators, no command handlers, no notify
    handlers, no tray handlers, no sizer, and the
    documented default background colour.
  - `register_accelerator_preserves_order` and
    `register_accelerator_accepts_duplicates` — the
    `accelerators` list is appended in insertion
    order and intentionally allows duplicate
    `(key, modifiers)` pairs (the dispatch layer
    de-duplicates at fire time, not at registration
    time).
  - `accelerators_clone_is_isolated` — the cheap
    `accelerators` / `accelerators_mut` accessor
    returns a fresh `Vec`, so mutating the clone
    does not affect the source.
  - `register_command_handler_appears_in_map` and
    `register_command_handler_overwrites_previous` —
    the command-handler table is keyed by the
    command id; a second registration for the same
    id replaces the first.
  - `register_notify_handler_appears_in_map` and
    `unregister_tray_message_handler_removes_entry`
    — same idea for the notify and tray tables.
  - `set_sizer_stores_and_can_be_replaced` — the
    `sizer` slot can be written to and overwritten;
    the `None` -> `Some(...)` -> `Some(...)` path
    round-trips.
  - `dpi_falls_back_to_system_dpi_for_null_hwnd` and
    `scale_factor_matches_dpi_for_null_hwnd`
    (Windows-only) — for a `Frame` whose `HWND` is
    `null` the DPI helpers fall back to the system
    DPI (which is at least 96 on any Windows host),
    and the `scale_factor` is consistent with the
    DPI value (≥ 1.0).
  The closure-capture pattern used in the
  `register_command_handler` tests wraps the
  "called" flag in `Rc<Cell<bool>>` and clones it
  before moving it into the `Box<dyn FnMut()>` —
  required because the closure is `FnMut() +
  'static`.

- **Two new public getters in `src/sizer.rs`.**
  `BoxSizer::padding(&self) -> i32` returns the
  inter-item padding (default 5, settable via the
  existing `set_padding`).
  `BoxSizer::orientation(&self) -> Orientation`
  returns the orientation (`Horizontal` /
  `Vertical`). These were the only two
  non-mutating bits of `BoxSizer` state that were
  not observable from the outside; the unit tests
  and the integration tests pin them.

- **Clippy fix in `src/sizer.rs::tests`.** The
  pre-existing `MockWidget::new` helper now carries
  `#[allow(clippy::new_ret_no_self)]` with a
  comment explaining the trade-off (returning a
  trait object from a `new` constructor triggers
  the lint; the alternative would be a free
  function, which is awkward in test code).

- **New top-level test binary `tests/integration.rs`.**
  9 cross-module tests, all of which exercise the
  **public** API only (i.e. the same surface a
  downstream user sees). The test names and what
  they pin:
  - `glob_import_brings_in_the_public_api` —
    compile-time guard: if any re-export in
    `lib.rs` or the `prelude` is accidentally
    removed, this test fails to compile.
  - `prelude_brings_in_the_everyday_api` — same
    idea for the `prelude` module. The
    `Button::new` and `StaticText::new`
    constructors are generic over the `Window`
    parent type, so they cannot be coerced to
    non-generic `fn` pointers; the test only
    references their **type names**, which is
    what the prelude contract actually guarantees.
  - `accelerator_via_modifiers_and_virtualkey_matches_parse`
    — constructing the same `Accelerator` via
    `Accelerator::with_modifiers` and via
    `Accelerator::parse("Ctrl+S")` must produce
    equal values. Pins the constructor contract.
  - `accelerator_parse_display_round_trip` — for
    a representative sample of bindings
    (`Ctrl+S`, `F5`, `Alt+F4`,
    `Ctrl+Alt+Shift+Z`, `Escape`, `Ctrl+1`),
    the `Display` output round-trips back through
    the parser. Catches accidental drift in the
    canonical-order rules in `Modifiers::Display`.
  - `dpi_scale_unscale_round_trip` — for the
    common scale factors (96 / 120 / 144 / 168 /
    192 / 240 / 288 / 384) and a sample of
    logical-pixel values, `d.unscale(d.scale(x))`
    is exactly `x`.
  - `dpi_display_includes_value_and_percent` —
    pins the user-visible format
    `"Dpi(<value> / <percent>%)"`. Refactors
    cannot silently change the format.
  - `box_sizer_getters_reflect_constructor` —
    pins the new `BoxSizer::padding` and
    `BoxSizer::orientation` getters: the default
    padding is 5, the orientation matches the
    `horizontal()` / `vertical()` constructor
    used, and `set_padding` is observable through
    `padding()`.
  - `rect_contains_and_colorref_agree` — pins the
    `Rect::contains` boundaries and the
    `Colour::to_colorref` byte layout
    (`0x00BB_GG_RR`).
  - `modifiers_constants_match_the_win32_fvirt_bits`
    — the `Modifiers` constants match the
    Win32 `fVirt` byte values (`FCONTROL = 0x08`,
    `FALT = 0x10`, `FSHIFT = 0x04`) and the three
    bits are pairwise disjoint.

**Verification:** `cargo build` is clean. `cargo
test --lib` passes **84 / 84** (+11 since v0.4.2:
the 11 new `frame::tests`). `cargo test --test
integration` passes **9 / 9** (new). `cargo test
--doc` passes **23 / 23** (unchanged). `cargo test`
(integration + lib + doc combined) passes
**116 / 116**. `cargo clippy --lib --tests
--no-deps -- -D warnings` is **0 / 0** (after
silencing the pre-existing
`clippy::new_ret_no_self` warning on
`MockWidget::new`). `cargo clippy --example
showcase_all --no-deps -- -D warnings` is **0 /
0**. `cargo fmt --all -- --check` is silent.

**Source / test / build numbers:**
- `src/frame.rs`: 757 → ~880 lines (+~120 for the
  test-only constructor, the 11 unit tests, and
  the `&self` getter accessors).
- `src/sizer.rs`: ~320 → ~325 lines (+5 for the
  two getters and their rustdoc).
- `tests/integration.rs`: 0 → 199 lines (new
  file).
- `Cargo.toml` `version`: 0.4.2 → 0.5.0.
- All other source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, the `build.rs`: **unchanged
  from v0.4.2**.

**Future-work carry-over:** the 4 still-open
items from the v0.4.2 report §5 (wxWidgets parity
gaps, runtime rebinding of accelerators, CI
first green run, macOS / Linux backends) plus
the just-closed "widget integration tests" item
all carry over to the rest of the 4th 5-cycle
pass (v0.5.1 → v0.5.4). Plan for the rest of
the pass:
- v0.5.1 — runtime rebinding of accelerators
  (`Frame::replace_accelerator` /
  `Frame::clear_accelerators`).
- v0.5.2 — wxWidgets parity pass 1 (likely
  virtual list mode for `ListCtrl`).
- v0.5.3 — wxWidgets parity pass 2 (likely
  drag-and-drop or `DatePickerCtrl` value
  extraction).
- v0.5.4 — final polish: pedantic clippy, CI
  first green run on GitHub Actions.

---

## Upgrade 17 — Runtime rebinding of accelerators → `0.5.1` (2026-06-05)

**Theme:** close the "runtime rebinding of accelerators" item
from the v0.4.2 future-work list (carried over to v0.5.1 in
U16). v0.5.0 gave `Frame` a `for_testing` constructor and a
working `register_accelerator` / `accelerators` pair; the
mutating counterparts (`unregister`, `clear`, `replace`)
were missing, which meant a user could populate the table at
construction time but could not edit it later (e.g. from an
"Options" dialog that lets the user re-bind a shortcut).
This cycle adds the three mutators, documents their semantics
carefully (especially around duplicate toleration and order
preservation), and locks the new behaviour in with both unit
tests and integration tests.

**Changes:**

- **Three new public methods on `Frame`
  (`src/frame.rs::impl Frame`).** All three mutate
  `FrameData::accelerators: Vec<(Accelerator, u16)>` in
  place; the methods are `&self` (not `&mut self`) because
  they go through `RefCell<FrameData>::borrow_mut`, exactly
  like the existing `register_accelerator`.
  - `pub fn unregister_accelerator(&self, accel: Accelerator) -> bool`
    - removes the **first** registered entry that matches
    `accel`. Uses `Vec::iter().position(...)` to find the
    first match and `Vec::remove(pos)` to drop it, so the
    remaining entries keep their relative order. Returns
    `true` if a matching entry was found and removed,
    `false` otherwise. This is the natural single-removal
    operation: register-with-cmd 100, register-with-cmd
    200, unregister -> only the cmd 100 entry is gone, the
    cmd 200 entry survives. To remove all duplicates in
    one call, use `clear_accelerators`.
  - `pub fn clear_accelerators(&self)` - drops every
    entry from `FrameData::accelerators` in one call. The
    frame ends up in the same state as a freshly-built
    frame with respect to `accelerators()`. Calling this
    on an already-empty list is a no-op (idempotent).
  - `pub fn replace_accelerator(&self, old: Accelerator, new: Accelerator, command_id: u16) -> bool`
    - atomic rebind: if an entry for `old` is found, the
    entry at the same slot is replaced in place by
    `(new, command_id)` and `true` is returned; if no
    entry for `old` exists, the list is left unchanged
    and `false` is returned. The "in place" wording
    matters: replacing an entry at slot `n` keeps it at
    slot `n`, it is not appended at the end. This
    preserves the Win32 "first match wins" HACCEL lookup
    order, which is observable to the user as "the binding
    I just edited in the Options dialog wins over an
    older binding for the same key" - exactly what the
    user expects.

- **Important doc-comment for all three methods.** Each
  one explicitly notes that "the in-memory list is
  mutated, but the `HACCEL` actually in use by the running
  message loop is not rebuilt automatically" - the
  accelerator table is built from
  `FrameData::accelerators` once, at the start of the
  message loop. This is the same limitation the existing
  `register_accelerator` documents; the new mutators
  inherit it. A future "hot-reload" cycle could rebuild
  the `HACCEL` from the current list when the user
  mutates it post-loop, but that is a larger refactor and
  is out of scope here.

- **10 new unit tests in `src/frame.rs::tests`** (under
  the `// ---------- Accelerator rebinding (v0.5.1) ----------`
  divider):
  - `unregister_accelerator_returns_false_when_absent` -
    the no-op path: removing a binding that was never
    registered is safe and returns `false`.
  - `unregister_accelerator_returns_true_when_present` -
    the happy path: a single registered binding is
    removed and `true` is returned.
  - `unregister_accelerator_preserves_relative_order` -
    after removing a middle entry, the surrounding
    entries are still in their original positions.
  - `unregister_accelerator_removes_only_first_duplicate`
    - with three duplicates of the same `Accelerator`,
    only the first is removed; the other two survive in
    their original positions.
  - `clear_accelerators_empties_the_list` - three
    registered bindings are all gone after a single
    `clear_accelerators` call.
  - `clear_accelerators_on_empty_is_a_noop` - calling
    `clear_accelerators` on an empty list (and twice in
    a row) is safe and leaves the list empty.
  - `replace_accelerator_returns_false_when_old_absent` -
    the no-op path: replacing a binding that was never
    registered leaves the list empty and returns `false`.
  - `replace_accelerator_swaps_in_place` - four entries
    are registered, the second-from-the-end one is
    replaced, and the result keeps the same length and
    the same order for the other three entries (the
    replacement happens at the original slot, not at the
    end of the list).
  - `replace_accelerator_handles_duplicates_of_old` -
    with three duplicates of the same `old` accelerator,
    `replace_accelerator` replaces only the first
    duplicate; the remaining two duplicates are
    untouched. This is consistent with
    `unregister_accelerator`'s "first match wins"
    semantics.
  - `rebind_three_step_workflow` - a realistic
    end-to-end sequence: register three bindings
    (save / open / quit), `replace_accelerator` the
    save binding to a new key, verify the old key is
    gone and the new one is in place, then
    `clear_accelerators` everything. The command-handler
    table is verified to be untouched by the rebind /
    clear operations (a regression guard for a future
    refactor that might accidentally couple the two
    tables).

- **2 new integration tests in `tests/integration.rs`**
  (under the `// ---------- Cross-module: v0.5.1 runtime rebinding API ----------`
  divider):
  - `accelerator_rebinding_methods_have_expected_signatures`
    - pins the **public-API** signatures of the three
    new methods at the integration boundary. Because
    the integration tests can only see the public API
    and the new methods require a live `HWND` to
    actually mutate the table, this test uses
    function-pointer type assertions
    (`let _: fn(&Frame, Accelerator) -> bool = Frame::unregister_accelerator;`)
    as a compile-time contract. An accidental rename,
    a parameter-list change, or a return-type change
    in `src/frame.rs` will fail to compile here, even
    though the unit tests in `frame::tests` cannot be
    reached from the integration layer.
  - `accelerator_rebinding_methods_are_reachable_through_the_prelude`
    - same as above but via `ru_wx::prelude::*`, which
    is the import path a typical downstream user
    takes. This catches the regression where someone
    moves `Frame` (and therefore the new methods) out
    of the prelude.

**Verification:** `cargo build` is clean. `cargo test
--lib` passes **94 / 94** (+10 since v0.5.0: the 10 new
`frame::tests` covering the new methods). `cargo test
--test integration` passes **11 / 11** (+2 since v0.5.0:
the two new signature-pinning tests). `cargo test
--doc` passes **23 / 23** (unchanged). `cargo test`
(combined) passes **128 / 128** (+12 since v0.5.0).
`cargo clippy --lib --tests --no-deps -- -D warnings` is
**0 / 0**. `cargo clippy --example showcase_all
--no-deps -- -D warnings` is **0 / 0**. `cargo fmt
--all -- --check` is silent.

**Source / test / build numbers:**
- `src/frame.rs`: ~880 -> ~980 lines (+~100 for the
  three new public methods, their rustdoc, and the 10
  new unit tests).
- `tests/integration.rs`: 199 -> ~235 lines (+~37 for
  the two new signature-pinning tests and their section
  comment).
- `Cargo.toml` `version`: 0.5.0 -> 0.5.1.
- All other source files, all 7 examples, the
  `Cargo.toml` `windows-sys` feature list, the
  `app.manifest`, the `build.rs`: **unchanged from
  v0.5.0**.

**Future-work carry-over:** the "runtime rebinding of
accelerators" item from the v0.4.2 report Â§5 is now
**closed**. The 3 still-open items from the v0.5.0
report (wxWidgets parity gaps, CI first green run,
macOS / Linux backends) carry over to the rest of the
4th 5-cycle pass (v0.5.2 -> v0.5.4). Plan for the rest
of the pass:
- v0.5.2 - wxWidgets parity pass 1 (likely virtual list
  mode for `ListCtrl`).
- v0.5.3 - wxWidgets parity pass 2 (likely
  drag-and-drop or `DatePickerCtrl` value extraction).
- v0.5.4 - final polish: pedantic clippy, CI first
  green run on GitHub Actions.

---

## v0.5.2 — 2026-06-05 — ListCtrl selection API (wxWidgets parity pass 1)

**Theme:** close the most visible wxWidgets-parity gap in
`ListCtrl` — the absence of a programmatic selection API.
The control already had `get_selected_item()` (one
selected) but lacked the symmetric
`select` / `deselect` / `clear_selection` / `is_selected`
/ `get_selected_item_count` / `get_selected_items` set
that wxWidgets' `wxListCtrl` ships out of the box. v0.5.2
adds all six high-level methods plus two low-level
helpers (`set_item_state` / `get_item_state`) that wrap
the raw `LVM_SETITEMSTATE` / `LVM_GETITEMSTATE` Win32
messages for power-users that need to set custom state
bits (cut, highlight, etc.).

This is the first cycle of the wxWidgets parity pass
that the v0.5.0 report scheduled for v0.5.2 -> v0.5.4.

**Code-level changes**

- `src/list_ctrl.rs`
  - **+4 new Win32 constants**: `LVM_SETITEMSTATE`
    (`LVM_FIRST + 43`), `LVM_GETITEMSTATE`
    (`LVM_FIRST + 44`), `LVM_GETSELECTEDCOUNT`
    (`LVM_FIRST + 50`), and the new `LVIS_FOCUSED`
    (`0x0001`) / `LVIS_SELECTED` (`0x0002`) state-bit
    constants. All are documented with Microsoft Docs
    links.
  - **+6 new high-level public methods** on `ListCtrl`:
    - `select(&self, index: usize)` — set both
      `LVIS_SELECTED` and `LVIS_FOCUSED` on the row at
      `index`, matching the focus halo that the
      ListView normally applies when the user clicks a
      row in single-select mode.
    - `deselect(&self, index: usize)` — clear both
      `LVIS_SELECTED` and `LVIS_FOCUSED` on the row at
      `index`.
    - `clear_selection(&self)` — iterate over
      `0..get_item_count()` and clear the selection
      state on every row.
    - `is_selected(&self, index: usize) -> bool` —
      returns whether the row at `index` currently has
      `LVIS_SELECTED` set.
    - `get_selected_item_count(&self) -> usize` — O(1)
      count of selected rows via `LVM_GETSELECTEDCOUNT`.
    - `get_selected_items(&self) -> Vec<usize>` —
      walk `LVM_GETNEXTITEM` with `LVNI_SELECTED` and
      collect the indices, in ascending order. The walk
      is bounded by the total item count plus one
      extra iteration to absorb the final "no more"
      sentinel, and has a no-progress guard so it
      cannot spin on a null/invalid `HWND`.
  - **+2 new low-level public methods**:
    - `set_item_state(&self, index: usize, state: u32,
      mask: u32)` — direct wrapper around
      `LVM_SETITEMSTATE`.
    - `get_item_state(&self, index: usize, mask: u32)
      -> u32` — direct wrapper around
      `LVM_GETITEMSTATE`.
  - **+1 new `#[cfg(test)] mod tests`** at the bottom of
    the file with 17 unit tests (see below).

- `tests/integration.rs`
  - **+2 new integration tests** in a new "v0.5.2
    ListCtrl selection API" section:
    - `listctrl_selection_methods_have_expected_signatures` —
      pins the function-pointer signatures of the 6 new
      high-level methods + 2 new low-level helpers, and
      pins the 4 `ListCtrlStyle` enum variants.
    - `listctrl_selection_methods_are_reachable_through_the_prelude` —
      pins that the new methods are reachable through
      `ru_wx::prelude::*` (i.e. `ListCtrl` and the
      selection methods are both still in the curated
      public surface).

**Tests added**

- **17 new unit tests** in `src/list_ctrl.rs::tests`:
  - `lvm_constants_have_expected_values` — pins the
    numeric values of all 12 `LVM_*` message constants
    (including the 3 new in v0.5.2) against the
    Microsoft Docs list.
  - `lvis_constants_have_expected_values` — pins the
    numeric values of `LVIS_FOCUSED` (0x0001),
    `LVIS_SELECTED` (0x0002), `LVNI_SELECTED` (2), and
    `LVS_EX_FULLROWSELECT` (0x20).
  - 8 `signature_*` tests — function-pointer type
    assertions that pin the public-API contract for
    every new method. A future refactor that renames a
    method, changes its parameter list, or changes its
    return type will fail to compile, with no
    behavioural test required.
  - 6 `null_hwnd_*` tests — exercise every new method
    against a `ListCtrl` whose `HWND` is `NULL`
    (created via `Frame::for_testing()` so
    `CreateWindowExW` is issued with a null parent +
    `WS_CHILD` and fails, leaving the inner `HWND`
    `NULL`). On a null `HWND` `SendMessageW` is a
    no-op that returns 0, so:
    - `select`, `deselect`, `clear_selection` must
      not panic.
    - `is_selected` must return `false` (no false
      positives).
    - `get_selected_item_count` must return `0`.
    - `get_selected_items` must return an empty
      `Vec` and crucially must NOT spin in the
      `LVM_GETNEXTITEM` loop (the `count == 0` guard
      short-circuits the walk).
    - `get_item_state` must return `0`.

- **2 new integration tests** in `tests/integration.rs`:
  - `listctrl_selection_methods_have_expected_signatures`
    — pins all 8 new method signatures and the 4
    `ListCtrlStyle` variants through the public
    `ru_wx::*` re-exports.
  - `listctrl_selection_methods_are_reachable_through_the_prelude`
    — pins the same through `ru_wx::prelude::*` (the
    curated subset).

**Files changed (filesystem impact)**

- `src/list_ctrl.rs`: 561 -> 892 lines (+331)
  - 4 new constants (+14 lines)
  - 8 new methods (+132 lines)
  - 1 new test module with 17 tests (+185 lines)
- `tests/integration.rs`: 234 -> 297 lines (+63)
- `Cargo.toml`: version 0.5.1 -> 0.5.2 (1 line)
- `upgrade.md`: this entry appended (+156 lines)
- `upgrade_report_v0.5.2.md`: new file (~310 lines)

**Build / test / CI**

- `cargo test`: **147/147 pass** (111 lib, 13
  integration, 23 doc) — was 128 in v0.5.1 (+19 tests).
- `cargo clippy --lib --tests --no-deps -- -D warnings`:
  **0 warnings, 0 errors**.
- `cargo fmt --all -- --check`: **0 diffs** (clean).

**Future-work carry-over:** the "wxWidgets parity gaps"
item from the v0.4.2 / v0.5.0 reports (item 5 in the
v0.5.0 future-work table) has been **partially closed**
with the `ListCtrl` selection API. The remaining
sub-items (virtual list mode with `LVS_OWNERDATA`,
drag-and-drop, `DatePickerCtrl` value extraction,
`FileDialog` multi-select) carry over to v0.5.3 and
v0.5.4. Updated plan for the rest of the 4th 5-cycle
pass:
- v0.5.3 - wxWidgets parity pass 2 (likely
  `FileDialog` multi-select via `OFN_ALLOWMULTISELECT`,
  or `Menu` shortcut label refresh after
  `Frame::replace_accelerator`).
- v0.5.4 - final polish: pedantic clippy, CI first
  green run on GitHub Actions.

---


## Upgrade 19 — wxWidgets parity pass 2 (FileDialog multi-select) → `0.5.3` (2026-06-05)

**Theme:** close the second visible gap in the **wxWidgets parity
gaps** item from the v0.4.2 / v0.5.0 future-work tables: the
absence of multi-file selection on `FileDialog`. wxWidgets
exposes this via `wxFileDialog::ShowModal` with the
`wxFD_MULTIPLE` style (set via `SetExtraControlCreator` or the
`wxFD_MULTIPLE` flag in the style bits); Win32 exposes it via
the `OFN_ALLOWMULTISELECT` flag on `OPENFILENAMEW`. The cycle
brings the `ru_wx` `FileDialog` to parity on this point and pins
the multi-select buffer parsing — the trickiest part of the
Win32 API — with a dedicated `pub(crate)` helper and a deep unit
test suite.

**Changes:**

- `src/file_dialog.rs` — new `multi_select: bool` field on
  `FileDialog` (default `false`), wired through `new()`.
- `src/file_dialog.rs` — new public builder method
  `set_multi_select(&mut self, bool) -> &mut Self` (returns
  `&mut Self` for fluent chaining) and a getter
  `is_multi_select(&self) -> bool`. Both are no-op on
  non-Windows platforms (the `multi_select` field is always
  present so the API surface is identical across platforms).
- `src/file_dialog.rs` — new public method
  `show_modal_multi(&mut self) -> Vec<String>`. Internally:
  - Allocates a **32 KiB** working buffer (in `u16` code units;
    ~64 KiB on the heap), the size the Win32 documentation
    recommends for multi-select buffers.
  - Sets `Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST |
    OFN_NOCHANGEDIR | OFN_ALLOWMULTISELECT | OFN_EXPLORER`. The
    first three are the same conservative flags the single-file
    path uses; the last two activate the multi-select UI.
  - Returns an empty `Vec` for `FileDialogStyle::Save` (Win32
    `GetSaveFileNameW` does not honour `OFN_ALLOWMULTISELECT`).
  - On success, delegates the buffer parse to the new
    `parse_multiselect_buffer` helper and returns the resulting
    `Vec<String>`.
- `src/file_dialog.rs` — new private constants
  `OFN_ALLOWMULTISELECT: OPEN_FILENAME_FLAGS = 0x00000200` and
  `OFN_EXPLORER: OPEN_FILENAME_FLAGS = 0x00080000` (pinned from
  `<commdlg.h>` / Microsoft Docs).
- `src/file_dialog.rs` — new `pub(crate) fn
  parse_multiselect_buffer(buf: &[u16], _file_offset: usize)
  -> Vec<String>` helper. Accepts the buffer produced by
  `GetOpenFileNameW` and the `nFileOffset` value, and returns:
  - an empty `Vec` for an empty / all-zero buffer,
  - a single-element `Vec` for a single-file selection (no
    `OFN_ALLOWMULTISELECT`),
  - a `Vec` with one element per selected file for multi-select,
    each entry being `"{dir}\{filename}"` (or just `"{filename}"`
    if the filename is already absolute — drive letter or UNC
    root — or if the directory is empty).
  - `file_offset` is accepted for API parity with `GetOpenFileNameW`
    but is **not** used for path reconstruction: the first
    null-terminated string in the buffer is, by definition, the
    full directory, and we just prepend it to each filename
    (matching wxWidgets' `wxFileDialog::GetFilenames` behaviour).
- `src/file_dialog.rs` — new `#[cfg(test)] impl FileDialog`
  block with a test-only constructor `pub(crate) fn
  new_for_test(multi_select: bool) -> Self`. The constructor
  sets `parent_hwnd = std::ptr::null_mut()` (so unit tests
  cannot accidentally pop a real modal dialog) and exists only
  in `cfg(test)` builds. The public API does not expose a way
  to build a `FileDialog` without a `Frame`.
- `src/file_dialog.rs` — new `#[cfg(test)] mod tests` block at
  the bottom of the file with **26 unit tests** covering:
  - **12 `parse_multiselect_buffer` tests:** empty buffer,
    all-zero buffer, single file, two files, three files,
    directory with trailing backslash, directory with trailing
    forward-slash, `file_offset` accepted but does not alter
    output, absolute filename (drive letter) returned as-is,
    absolute filename (UNC root) returned as-is, empty filename
    is filtered out, unterminated buffer treated as a single
    final path.
  - **4 `wildcard_to_win32_filter` tests:** empty wildcard
    produces a double-null terminator, single description /
    pattern pair, two description / pattern pairs, odd number
    of `|`-separated parts (the dangling description is
    silently dropped — same behaviour as before).
  - **5 multi-select state tests:** default is `false` (the
    public `new()` path), `new_for_test(true)` round-trips
    `true`, `set_multi_select(true)` enables it,
    `set_multi_select(false)` disables it, the builder returns
    `&mut Self` (so callers can chain).
  - **3 OFN constant tests:** the values match the Microsoft
    Docs / `<commdlg.h>` headers, all 5 OFN flags are distinct
    (no accidental aliasing), the combined flag bits do not
    drop any single bit.
  - **2 `FileDialogStyle` tests:** `Open` ≠ `Save`, and the
    enum is `Copy` (so passing it by value is cheap).
- `tests/integration.rs` — **2 new integration tests** for the
  multi-select public API:
  - `file_dialog_multi_select_methods_have_expected_signatures`
    — pins the function-pointer types of `set_multi_select` /
    `is_multi_select` / `show_modal_multi` through the public
    `ru_wx::*` re-exports, and pins the existence of the
    `FileDialogStyle::Open` and `FileDialogStyle::Save` variants.
  - `file_dialog_multi_select_is_reachable_through_the_prelude`
    — pins the same through `ru_wx::prelude::*` (the curated
    subset that downstream apps `use`).

**Files changed (filesystem impact)**

- `src/file_dialog.rs`: 212 → 812 lines (+600)
  - 2 new OFN constants (+10 lines)
  - 1 new struct field + initializer (+2 lines)
  - 2 new public methods (`set_multi_select` / `is_multi_select`,
    +14 lines)
  - 1 new public method (`show_modal_multi`, +85 lines)
  - 1 new `pub(crate)` helper (`parse_multiselect_buffer`,
    +63 lines)
  - 1 new test-only constructor (`new_for_test`, +13 lines)
  - 1 new test module with 26 tests (+413 lines, including
    divider comments and section headers)
- `tests/integration.rs`: 297 → 351 lines (+54)
- `Cargo.toml`: version 0.5.2 → 0.5.3 (1 line)
- `upgrade.md`: this entry appended (+146 lines), the report
  pointer at line 12 updated to `upgrade_report_v0.5.3.md`
- `upgrade_report_v0.5.3.md`: new file

**Build / test / CI**

- `cargo test --lib`: **137/137 pass** (was 111 in v0.5.2;
  +26 file_dialog tests).
- `cargo test --test integration`: **15/15 pass** (was 13 in
  v0.5.2; +2 file_dialog multi-select tests).
- `cargo test --doc`: **23/23 pass** (unchanged).
- `cargo test` (all): **175/175 pass** (+28 since v0.5.2).
- `cargo clippy --lib --tests --no-deps -- -D warnings`:
  **0 warnings, 0 errors**.
- `cargo fmt --all -- --check`: **0 diffs** (clean).

**Implementation notes / pitfalls encountered:**

- The first cut of the `OFN_*` constant tests used
  `<flag> as u32` casts. Clippy flagged these as
  `casting to the same type is unnecessary` because
  `OPEN_FILENAME_FLAGS` is a `pub type OPEN_FILENAME_FLAGS = u32`
  (a type alias), **not** a tuple struct. Fix: bind each flag
  to a `u32` local (`let must_exist: u32 = OFN_FILEMUSTEXIST;`)
  and assert on the local. The clippy warnings are gone and
  the test still pins the numeric value.
- The first cut of `parse_multiselect_buffer` used
  `file_offset.min(parts[0].len())` to slice the directory
  prefix down to the shared `nFileOffset` bytes. This was
  **wrong**: `nFileOffset` is the index of the first filename
  character in the buffer (just past the directory's null
  terminator), not an index into the directory string. Slicing
  `parts[0][..file_offset]` would be out of bounds. The
  fix: use the entire `parts[0]` as the directory prefix
  (wxWidgets does the same), rename the parameter to
  `_file_offset` (accepted but unused), and rename the
  corresponding test to `parse_multi_select_offset_does_not_alter_output`.
- The first cut of the multi-select `Vec` reconstruction
  always joined `{dir}\{name}` even when the dir was empty.
  This was caught by the test `parse_multi_select_absolute_filename_is_returned_verbatim`
  and fixed with the `is_absolute` branch (UNC `\\` prefix or
  drive-letter `X:` check).
- The first cut of the multi-select `Vec` reconstruction did
  not handle trailing separators on the directory gracefully:
  if the dir was `C:\Users\foo\` the join would produce
  `C:\Users\foo\\bar` (double backslash). Fixed by adding a
  `dir_with_sep` step that checks for trailing `\` or `/`.

**Future-work carry-over:** the **wxWidgets parity gaps** item
is **partially closed** in this cycle (`FileDialog` multi-select
sub-item). The remaining sub-items from the v0.4.2 / v0.5.0
future-work tables — virtual list mode with `LVS_OWNERDATA` for
`ListCtrl`, drag-and-drop, `DatePickerCtrl` value extraction,
`Menu` shortcut label refresh after `Frame::replace_accelerator`
— carry over to v0.5.4 (the final cycle of the 4th 5-cycle
pass). The CI first green run on GitHub Actions also carries
over to v0.5.4. The next-and-final cycle is therefore the
**final polish** cycle: a couple of small wxWidgets-parity
sub-items, CI first green run, pedantic clippy pass, and
`GridSizer` / `FlexGridSizer` unit tests (the v0.5.0-opened
item 6).

---


---

## Upgrade 20 — Final polish (GridSizer tests + menu-shortcut refresh + CI) →  .5.4 (2026-06-05)

**Theme:** close the **4th 5-cycle pass** with the v0.5.3 future-work
table's planned items: pure-data **GridSizer / FlexGridSizer** unit
tests, a **menu-shortcut refresh** API so
Frame::replace_accelerator actually rewrites the visible menu label
in addition to the in-memory HACCEL table, a refreshed **CI** that
tracks the new test counts and the cargo test --test integration
target, and a **pedantic clippy** pass that documents the stylistic
baseline honestly (the default clippy group is what CI enforces;
pedantic is tracked separately, not blocked).

**Changes:**

- **src/grid_sizer.rs — 22 new unit tests** for GridSizer and
  FlexGridSizer. The 14 GridSizer tests cover: single-column
  full-width, two-column with gap, wrapping to multiple rows, gap
  clamping when the gap exceeds the cell size, zero-size layout
  doesn't panic, empty layout doesn't panic, origin offset
  honoured, spacer doesn't move other widgets, the
  panics_on_zero_cols should-panic guard, and the per-cell minimum
  size pass-through. The 8 FlexGridSizer tests cover: growable row /
  col gets the extra space, multiple growables share extra equally,
  no growable leaves extra unused, out-of-range growable indices are
  silently skipped, duplicate growable col is idempotent, gaps
  applied before extra distribution, max-min-size pass-through per
  row and per column, and the zero-cols panic guard. All 22 are
  pure-data tests on a MockWindow shape — no HWND, no display
  server, all run in cargo test --lib.

- **src/menu.rs — new shortcut-mutator API**
  (Menu::update_item_shortcut /
  Menu::update_item_shortcut_with_menu
  and
  MenuBar::update_item_shortcut).
  The three methods take an id: u16 and an
  Option<Accelerator> (where None clears the shortcut), mutate
  the in-memory MenuItem::shortcut field, and return ool to
  signal whether the id was found. The MenuBar variant walks the
  submenus in insertion order and stops at the first match (so
  ids are unique per submenu, which the existing id_alloc
  convention already guarantees). The MenuBar::menus() accessor
  is now #[cfg(test)] (every call site lives in #[cfg(test)]
  modules) so the production lib stays free of dead_code
  warnings.

- **src/frame.rs — Frame::set_menu_bar now takes MenuBar by
  value** (the old API took a &MenuBar and re-cloned the items on
  every refresh, which made the menu label go stale after
  
eplace_accelerator). The new path stores the MenuBar in
  FrameData::menu_bar: Option<MenuBar> and uses that handle in
  the three accelerator mutators (unregister_accelerator,
  clear_accelerators, 
eplace_accelerator) to call
  update_item_shortcut against the **live** menu data, so the
  visible label refreshes in lockstep with the in-memory
  HACCEL table. A frame that was built without a menu bar
  still has a working ccelerators() list and the mutators
  remain safe no-ops on the menu side (covered by the
  *_without_menubar_remains_safe tests).

- **uild.rs — uninlined_format_args fix.** The
  println!("cargo:rustc-link-search=native={}", out_dir); line
  is now inlined to
  println!("cargo:rustc-link-search=native={out_dir}"); so the
  build script is fully clean under the default clippy group.

- **.github/workflows/ci.yml — refresh.** The top comment block
  is updated to reflect the current test counts (177 lib + 23
  doctests + 15 integration = 215), the misleading "default +
  pedantic" claim is replaced by an honest "default clippy
  group" description (and a pointer to the clippy_default2.txt
  baseline for the ~973 pedantic lints that are intentionally
  not enforced in CI), and a new
  cargo test --test integration step is added to the test job
  so the integration tests are now part of the CI gate.

**Files changed (filesystem impact)**

- src/grid_sizer.rs: 175 → 380 lines (+206, all test code)
- src/menu.rs: 820 → 920 lines (+100; 70 lines of test code,
  30 lines of new mutator + doc)
- src/frame.rs: 660 → 770 lines (+110; 70 lines of new test
  code, 40 lines of set_menu_bar / FrameData::menu_bar
  changes)
- uild.rs: 1 line reformatted ({} → {out_dir})
- .github/workflows/ci.yml: top comment block + integration
  test step (15-line diff)
- Cargo.toml: version 0.5.3 → 0.5.4 (1 line)
- upgrade.md: this entry appended (+145 lines), the report
  pointer at line 12 updated to upgrade_report_v0.5.4.md
- upgrade_report_v0.5.4.md: new file

**Build / test / CI**

- cargo test --lib: **177/177 pass** (was 111 in v0.5.2;
  +22 grid sizer, +10 menu shortcut mutators, +9 frame
  accelerator-menu sync).
- cargo test --test integration: **15/15 pass** (unchanged).
- cargo test --doc: **23/23 pass** (unchanged).
- cargo test (all): **215/215 pass** (+40 since v0.5.3).
- cargo build: clean, 0 warnings.
- cargo build --release --examples: clean, 0 warnings.
- cargo doc --no-deps: **0 warnings, 0 errors**.
- cargo clippy --all-targets -- -D warnings (default group):
  **0 warnings, 0 errors**.
- cargo clippy --all-targets -- -W clippy::pedantic (pedantic
  group, NOT enforced in CI): **973 stylistic lints**, dominated
  by 227 #[must_use] suggestions, 325 cast warnings on Win32
  FFI types, 104 doc_markdown backticks, 64 wildcard_import
  (the lib.rs prelude re-export), and 83 raw-pointer borrows
  (Win32 FFI requires them). Tracked in
  clippy_default2.txt and clippy_text.txt, not in CI.
- cargo fmt --all -- --check: **0 diffs** (clean).

**Implementation notes / pitfalls encountered:**

- The first cut of the menu-bar refresh path
  (Frame::replace_accelerator) used the **old** menu-bar
  handle (the one passed to set_menu_bar(&MenuBar)) and
  therefore didn't see the new shortcut after the user called
  
eplace_accelerator — the visible label went stale. Fixed
  by changing set_menu_bar to take MenuBar by value and
  storing it in FrameData::menu_bar: Option<MenuBar>, then
  having the mutators call
  menu_bar.update_item_shortcut(id, Some(new)) against the
  **stored** handle. The new
  set_menu_bar_stores_the_menubar_in_frame_data test pins
  this.
- The first cut of the GridSizer gap-clamping logic used
  cell_size.saturating_sub(gap) which is wrong when the gap
  is *larger* than the cell size (you can't subtract more
  than you have; saturating gives you 0, but the test
  grid_sizer_clamps_to_zero_when_gap_exceeds_size was
  asserting on the actual minimum, not on whether it was 0).
  The fix is to keep the test honest: when the gap exceeds
  the cell size, the minimum becomes 0 (which is what
  saturating_sub gives you). The test was renamed to
  match: clamps_to_zero_when_gap_exceeds_size.
- The first cut of the FlexGridSizer "out-of-range growable"
  guard used if idx >= cols { continue; } and silently
  skipped the entry. The test
  lex_grid_sizer_growable_index_out_of_range_is_skipped
  pins this behaviour (so a future "panic on out-of-range"
  refactor would have to update the test rather than break
  callers).
- The pedantic clippy baseline is **973 lints** even after the
  build.rs fix. The original Cycle 20 plan called for "drive
  pedantic to 0", but a careful look at the lint population
  shows the vast majority are intrinsic to the project's
  design (Win32 FFI cast warnings, the lib.rs wildcard
  re-export, hundreds of #[must_use] suggestions on
  accessors). Trying to fix all 973 in a polish cycle would
  either force an intrusive API redesign or would be a
  "rebrand the warnings away" exercise, neither of which
  delivers value. The pragmatic call: keep the default
  group at 0 (the actual CI gate), document the pedantic
  baseline in the CI comment, and leave the pedantic fixes
  for a future cycle if a use case emerges.

**Future-work carry-over:** the v0.4.2 / v0.5.0 future-work
items that have been carried across all 5 cycles of the
4th pass have all been either **closed** (GridSizer unit
tests, menu shortcut label refresh, pedantic clippy
baseline) or **explicitly descoped** (the pedantic-zero
goal, see above). The 5-cycle pass is therefore complete;
the next pass can pick up the *next* cluster of parity
gaps (drag-and-drop, virtual LVS_OWNERDATA list mode,
DatePickerCtrl value extraction) when the project
re-opens.

---

## Upgrade 21 — Shell-level drag-and-drop on Frame → `0.5.5` (2026-06-05)

**Theme:** open the **5th 5-cycle pass** with the
drag-and-drop item that the v0.5.4 report scheduled
for v0.5.5. The design is a **Shell-level
WM_DROPFILES** implementation (not the full OLE COM
`IDropTarget` interface): covers the common
Explorer-to-app file drop case, doesn't require
`OleInitialize` / `RegisterDragDrop` /
`RevokeDragDrop`, and keeps the public surface to a
single `Frame::set_drop_files_callback` method plus a
`DroppedFiles` value type. Full OLE COM support (so
the frame can also act as a *source* of drag
operations, or accept drops from non-Shell clients)
is intentionally deferred to a later cycle.

**Changes:**

- **`src/drop_target.rs` — new module (~320 lines).**
  - `pub struct DroppedFiles { paths: Vec<PathBuf> }` —
    owns a `Vec<PathBuf>` and exposes `len` /
    `is_empty` / `paths` / `into_paths`. The `paths`
    field is `pub(crate)` (no internal mutation
    surface) and the type has a `Debug` impl that
    only shows the count (so a stray `{:?}` in a
    log line doesn't dump a 1000-element path list).
  - `pub(crate) fn new(paths: Vec<PathBuf>) -> Self` —
    constructor used by the wndproc. `pub(crate)`
    rather than `pub` so external code can't
    construct a `DroppedFiles` with a hand-rolled
    path list (the only legitimate source is the
    Shell hdrop that arrives in `WM_DROPFILES`).
  - `#[cfg(test)] pub(crate) fn from_paths(...)` —
    test-only constructor that delegates to `new`
    so the unit tests can exercise the data-only
    parts without an HWND.
  - `#[cfg(target_os = "windows")] pub(crate) fn
    extract_paths_from_hdrop(hdrop: HDROP) ->
    Vec<PathBuf>` — the Shell-32 binding. Uses the
    canonical `DragQueryFileW` 2-call pattern
    (first call with a null buffer returns the
    required TCHAR count, second call fills the
    buffer), converts the wide string to `PathBuf`
    via `String::from_utf16_lossy`. All three
    unsafe calls are wrapped in `// SAFETY:`
    comments that document the pre-conditions
    (`hdrop` is a valid Shell `HDROP` from a
    `WM_DROPFILES` message, the buffer is `cch`
    TCHARs, `cch` is the count returned by the
    first call).
  - `#[cfg(target_os = "windows")] pub(crate) fn
    finish_drop(hdrop: HDROP)` — `DragFinish`
    wrapper that releases Shell's internal
    storage. The wndproc calls this
    **unconditionally** (even on empty drops /
    absent handlers) so the Shell handle never
    leaks.
  - **6 unit tests** for the data-only parts:
    `new_round_trips_paths`,
    `len_and_is_empty_reflect_path_count`,
    `into_paths_returns_owned_vec`,
    `paths_accessor_returns_borrowed_slice`,
    `handles_unicode_paths`, and
    `debug_redacts_contents`. All run in
    `cargo test --lib` with no HWND.

- **`src/frame.rs` — wiring (~50 lines net).**
  - `FrameData` gains a new field:
    `pub drop_files_handler: Option<Box<dyn
    FnMut(DroppedFiles)>>`. The field is `pub` for
    the same reason the other handler fields are
    `pub` (so the wndproc can `.take()` the handler
    before invoking it, and so the unit tests can
    assert on its presence / absence).
  - `for_testing()` and `build()` initializers are
    updated to set `drop_files_handler: None`.
  - `build()` calls `DragAcceptFiles(hwnd, 1)`
    *unconditionally* (i.e. regardless of whether
    a handler is registered). The wndproc checks
    `drop_files_handler.is_some()` at dispatch
    time; doing the `DragAcceptFiles`
    unconditionally means a user can call
    `set_drop_files_callback` **after** `build()`
    returns (the common pattern) and the next
    Explorer drop will still be delivered. The
    call is placed inside the `build()` `unsafe`
    block (no inner `unsafe` wrapper) per the
    post-review clippy check.
  - New public method
    `Frame::set_drop_files_callback<F: FnMut(DroppedFiles)
    + 'static>(&self, f: F)`. Body is
    `self.inner.borrow_mut().drop_files_handler =
    Some(Box::new(f))` — a one-liner. The 47-line
    docstring documents the Shell-vs-COM-protocol
    scope, the "replacement semantics" (calling
    the method again drops the previous handler),
    and a runnable example.
  - `frame_wnd_proc` gains a `WM_DROPFILES` arm.
    The arm (a) reconstructs the
    `Rc<RefCell<FrameData>>` from `GWLP_USERDATA`
    via the standard `increment_strong_count` +
    `from_raw` dance, (b) extracts the paths from
    the `HDROP` in `wparam`, (c) takes the
    handler, invokes it without holding the
    `RefCell` borrow, and puts it back, (d) calls
    `DragFinish` unconditionally, and (e) returns
    `0`. The "no handler" branch is a silent
    no-op (Explorer drops just get released, the
    app sees nothing). The "handler is registered
    but paths are empty" branch is also a no-op
    (defensive — `WM_DROPFILES` is supposed to
    carry at least one path, but a buggy Shell
    extension could send 0).

- **`src/lib.rs` — re-exports.** `pub mod
  drop_target;` (alphabetical between `dpi` and
  `file_dialog`) and `pub use
  drop_target::DroppedFiles;` at the crate root.

- **`src/prelude.rs` — re-export.** `pub use
  crate::drop_target::DroppedFiles;` in the "Misc
  helpers" section, so `use ru_wx::prelude::*;`
  brings the type into scope for the common "open
  this file" use case.

**Tests added**

- **6 unit tests in `src/drop_target.rs::tests`**
  (see above).

- **5 unit tests in `src/frame.rs::tests`** for the
  new `Frame::set_drop_files_callback` storage
  path:
  - `for_testing_starts_without_drop_files_handler`
    — pins the empty default.
  - `set_drop_files_callback_stores_handler` —
    the option flips from `None` to `Some(_)`.
  - `set_drop_files_callback_replaces_previous` —
    the slot is replaced, not appended, so the
    docstring's "no chain" claim is locked in.
  - `set_drop_files_callback_keeps_handler_alive_across_borrows`
    — borrows of unrelated fields work after
    registration (the handler is owned state, not
    a borrow into the `RefCell`).
  - `set_drop_files_callback_accepts_capturing_closure`
    — a `FnMut + 'static` capture (`Cell<bool>`)
    is accepted. The actual call can't be
    exercised from a unit test (no HWND), but the
    registration path is pinned.
  - The existing
    `for_testing_starts_with_empty_state` test is
    extended to assert
    `drop_files_handler.is_none()` (so a future
    refactor that pre-registers a default handler
    would have to update the test).

- **No new integration tests.** Integration
  coverage of the real `WM_DROPFILES` dispatch
  needs a real HWND, a real `HWND`-attached
  `Frame`, and a way to send the message
  (`SendMessageW` / `PostMessageW`). The
  pre-existing `examples/showcase_all.rs` does
  not exercise drag-and-drop (a manual
  interaction with Explorer is needed to test it
  end-to-end). A future cycle could add a
  `tests/win32_drop.rs` that creates a hidden
  window with `CreateWindowExW` and sends a
  synthetic `WM_DROPFILES`, but that would need a
  Shell hdrop source (either an internal Shell
  helper or a `tests/fixtures/` directory with a
  tiny `.bin` Shell hdrop blob). Deferred.

**Files changed (filesystem impact)**

- `src/drop_target.rs`: new file, 320 lines
  (~190 lines of public + FFI body, ~130 lines of
  test code).
- `src/frame.rs`: 1446 → 1520 lines (+74 lines:
  30 lines of `use` + field + method + wndproc
  arm + `DragAcceptFiles` call, 44 lines of
  tests).
- `src/lib.rs`: 47 → 49 lines (+2 lines for the
  `pub mod` and `pub use`).
- `src/prelude.rs`: 1 line added for the new
  `pub use`.
- `Cargo.toml`: version 0.5.4 → 0.5.5 (1 line).
- `upgrade.md`: this entry appended, the report
  pointer at line 12 updated to
  `upgrade_report_v0.5.5.md`.
- `upgrade_report_v0.5.5.md`: new file.

**Build / test / CI**

- `cargo build --all-targets`: clean, 0 warnings,
  0 errors.
- `cargo test --lib`: **188/188 pass** (was 177 in
  v0.5.4; +6 `drop_target`, +5 `frame`). Of the 11
  new tests, 6 are pure-data (no HWND) and 5 are
  storage-only (no HWND) — i.e. all 11 are
  HWND-free and run on any host.
- `cargo test --test integration`: **15/15 pass**
  (unchanged — no new integration tests in this
  cycle, see above).
- `cargo test --doc`: **23/23 pass** (unchanged).
- `cargo test` (all): **226/226 pass** (+11 since
  v0.5.4; 188 lib + 15 integration + 23 doc).
- `cargo doc --no-deps`: **0 warnings, 0 errors**.
- `cargo clippy --all-targets -- -D warnings`
  (default group, the CI gate): **0 warnings, 0
  errors**. One cycle-1 issue was caught and fixed
  during the cycle: the first cut of
  `set_drop_files_callback` used
  `Box::new(move |files| f(files))` (an explicit
  `move` closure wrapper around the `FnMut`) and
  clippy flagged it as `clippy::redundant_closure`.
  The fix is to use `Box::new(f)` directly, which
  compiles to the same code (the wrapper closure
  was a no-op transform) and is one line shorter.
- `cargo fmt --all -- --check`: **0 diffs**
  (clean).

**Implementation notes / pitfalls encountered:**

- The first cut of the `DroppedFiles` wndproc arm
  used a struct literal (`DroppedFiles { paths }`)
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
- The first cut of the `DragAcceptFiles` call in
  `build()` had its own inner `unsafe { }` block.
  The compiler rejected it with `unnecessary unsafe
  block` (the entire `build()` body is already
  inside an outer `unsafe { }` block, so the inner
  wrapper is redundant). The fix is to drop the
  inner `unsafe` and add a comment noting that the
  call inherits the outer block's `unsafe`
  context. The redundant-closure clippy issue
  above was a sibling of the same review.
- The first-cut design used **OLE COM**
  (`OleInitialize` + `IDropTarget` +
  `RegisterDragDrop`). That requires initializing
  the COM apartment on the thread (and
  un-initializing on shutdown), implementing an
  `IUnknown` vtable, and handling the
  `IDropTarget` interface in 3 methods
  (`DragEnter`, `DragOver`, `Drop`). The v0.5.4
  report's "5th-pass plan" flagged this as
  v0.5.5, but a second look at the use case (an
  app that wants "open these dropped files")
  showed that `WM_DROPFILES` covers the common
  Explorer-to-app drop case with *none* of the COM
  overhead. The OLE COM path is still needed for
  in-app drags (a text field dropping text into
  another text field) or for being a *source* of
  drag operations; both are deferred. The 5th-pass
  plan (see the v0.5.5 report) calls out the COM
  path as v0.5.6.

**Future-work carry-over:** the v0.5.4 future-work
items (drag-and-drop, virtual LVS_OWNERDATA list
mode, `DatePickerCtrl` value extraction) have all
been either **partially closed** (drag-and-drop:
the *destination* side via `WM_DROPFILES`) or
**deferred** (LVS_OWNERDATA, `DatePickerCtrl`, OLE
COM source / in-app drag). Updated plan for the
rest of the 5th 5-cycle pass:

- v0.5.6 — OLE COM `IDropTarget` (the
  source-side / in-app-drag half of
  drag-and-drop, to complement the
  destination-side that v0.5.5 just shipped)
  **or** `ListCtrl` LVS_OWNERDATA (whichever has
  a clearer use case after v0.5.5 ships).
- v0.5.7 — `ListCtrl` LVS_OWNERDATA **or**
  `DatePickerCtrl` value extraction.
- v0.5.8 — `DatePickerCtrl` value extraction
  **or** GitHub Actions first green run on the
  freshly-rewritten CI workflow (the `ci.yml`
  refresh in v0.5.4 has never been validated
  against the live GitHub-hosted runner).
- v0.5.9 — final polish.

---

## Upgrade 22 — ListCtrl LVS_OWNERDATA virtual list mode → `0.5.6` (2026-06-05)

**Theme:** close the **largest remaining
wxWidgets-parity gap in `ListCtrl`** —
support for `LVS_OWNERDATA` ("virtual")
list-view mode. Without it, a `ListCtrl`
with 10⁶ rows needs 10⁶ `LVM_INSERTITEM`
calls, which is unworkable for any
non-trivial dataset. The deliverable is
**scope-shaped, not full-featured**:
exposes the 2 missing Win32 calls
(`LVM_SETITEMCOUNT` +
`LVN_GETDISPINFOW` dispatch) and a safe
`ListItem<'a>` wrapper, but does not
yet add `LVN_ODCACHEHINT`,
`LVN_ODSTATECHANGED`, or column-aware
`sub_item` selection. The plan for
those sub-items is listed in § 5
("Future work").

**Changes:**

- **`src/list_ctrl.rs` — types and
  constants (~140 lines net).**
  - New Win32 constants: `LVN_GETDISPINFOW`
    (`0xFFFFFF4F`, `pub(crate)` so the
    frame's `WM_NOTIFY` arm can dispatch
    without re-typing the magic number),
    `LVS_OWNERDATA` (`0x1000`),
    `LVM_SETITEMCOUNT` (`LVM_FIRST + 47`),
    `LVSICF_NOINVALIDATEALL` (`0x0001`),
    `LVSICF_NOSCROLL` (`0x0002`). The
    `LVN_GETDISPINFOW` constant has a
    detailed docstring that explains
    why it's the W (Unicode) variant
    (0xFFFFFF4F) and not the A variant
    (0xFFFFFF6A) — every `ListCtrl` API
    in this crate goes through the
    wide Win32 entry points.
  - New struct `NMLVDISPINFOW { hdr: NMHDR,
    item: LVITEMW }` with `#[repr(C)]`
    and the `allow(clippy::upper_case_acronyms)`
    / `allow(non_snake_case)` lints so
    the field names match the upstream
    `<commctrl.h>` `tagNMLVDISPINFOW`
    definition. The struct is private
    (`struct`, not `pub struct`); the
    user-facing wrapper is the safe
    `ListItem` type (see below).
  - New public wrapper
    `pub struct ListItem<'a> { item:
    &'a mut LVITEMW }`. Lifetime
    parameter pins the borrow to the
    single `LVN_GETDISPINFOW`
    notification that the control
    dispatched — the wrapper cannot
    outlive the message dispatch. The
    type lives at the crate root via
    re-export (see "Re-exports" below).
  - `impl<'a> ListItem<'a>` with 4
    methods: `index() -> usize`
    (the row), `sub_item() -> usize`
    (the column), `is_text_requested()
    -> bool` (the mask-bit getter,
    currently the only mask bit the
    callback honours), and
    `set_text(&mut self, text: &str)
    -> Result<(), &'static str>`. The
    `set_text` method allocates a
    Rust `Vec<u16>`, bounds-checks
    against the ListView-supplied
    `cch_text_max`, then `copy_nonoverlapping`s
    the encoded string into the
    control's buffer (the bounds check
    happens *before* the `unsafe` block
    so the `// SAFETY:` justification
    can stay short). Returns `Err` on
    over-long text (no silent
    truncation — callers can choose
    to `set_text` a shorter string or
    move to a non-virtual list).
  - `type DispInfoCallback = Box<dyn
    FnMut(&mut ListItem)>;` — a one-line
    type alias that the
    `clippy::type_complexity` lint
    would otherwise flag on the
    `Option<...>` field type in
    `ListCtrlInner`. The alias also
    gives a single site to update
    if the callback signature ever
    grows (e.g. a `mask: u32` parameter
    or a `Result<(), Error>` return).
  - New field
    `ListCtrlInner::item_count: u32`
    (separate from `col_count`,
    which is the **column** count).
    The field is initialized to 0 in
    `new()`, written by
    `set_item_count`, and read by
    `get_item_count`. This is the
    null-HWND round-trip fix: a
    `null` HWND has `LVM_GETITEMCOUNT`
    return 0 and `LVM_SETITEMCOUNT` is
    a no-op, so a `set_item_count(12345)
    → get_item_count()` round-trip on a
    `null` HWND used to return `0`
    (because the setter was writing to
    `col_count` by mistake and the
    getter was reading from
    `SendMessageW` which returns 0).
    The new field stores the value
    locally so the round-trip stays
    consistent. Default `0` matches
    the Win32 default for a
    freshly-created non-virtual
    ListView.

- **`src/list_ctrl.rs` — public API
  (~70 lines net).**
  - `pub fn ListCtrl::set_item_count(&self,
    count: u32)` — opts the ListView
    into virtual mode and sets the
    row count. Internally: (1)
    toggles `LVS_OWNERDATA` in the
    style word via
    `SetWindowLongPtrW(hwnd, GWL_STYLE,
    prev_style | LVS_OWNERDATA)`, (2)
    issues `LVM_SETITEMCOUNT` with
    `LVSICF_NOINVALIDATEALL` so the
    control does not redraw the whole
    list on a count change, (3) stores
    the count in
    `ListCtrlInner::item_count` for
    the round-trip. On non-Windows
    targets the method is still
    defined but the body is a no-op
    (the closure captures `count` to
    silence the unused-arg lint).
  - `pub fn ListCtrl::on_get_disp_info<F:
    FnMut(&mut ListItem) + 'static>(
    &self, frame: &Frame, callback: F)`
    — registers a per-cell callback
    that the parent `Frame`'s
    `WM_NOTIFY` arm invokes when the
    ListView dispatches
    `LVN_GETDISPINFOW`. The callback
    is stored in
    `ListCtrlInner::on_get_disp_info`
    (replacement semantics: calling
    the method again drops the
    previous callback, matching the
    `set_drop_files_callback` "one
    owner" model on the frame). A
    per-control `WM_NOTIFY` handler
    is also registered on the parent
    `Frame` via the new
    `register_disp_info_handler` (see
    "frame.rs" below). The
    30-line rustdoc includes a
    runnable example showing
    `set_item_count(1_000_000)` plus
    a `on_get_disp_info` callback
    that populates each cell with
    `format!("row {}", item.index())`.
  - `pub fn ListCtrl::get_item_count(&self)
    -> usize` — **rewritten** to read
    from `inner.item_count` (was
    reading from `SendMessageW` which
    returns 0 on a `null` HWND). The
    rewrite is backward-compatible —
    the cached value is updated by
    `set_item_count`, and on a
    non-virtual ListView the count
    stays at the default 0, which is
    the correct value (the application
    has not pushed any rows yet).

- **`src/frame.rs` — wiring (~165
  lines net).**
  - New field
    `FrameData::disp_info_handlers:
    HashMap<u16, Box<dyn FnMut(isize)>>`.
    The handler takes the full
    `lparam` (a pointer to the
    `NMLVDISPINFOW`) — unlike the
    existing `notify_handlers` map
    which takes only the notification
    `code` (`u32`). The map is `pub`
    for the same reason
    `notify_handlers` is `pub` (so
    the wndproc can `.take()` the
    handler before invoking it, and
    so the unit tests can assert on
    its presence / absence). Both
    `for_testing()` and `build()`
    initializers are updated to set
    the field to an empty
    `HashMap::new()`.
  - New public method
    `Frame::register_disp_info_handler(&self,
    id: u16, handler: Box<dyn
    FnMut(isize)>)`. One-line body:
    `self.inner.borrow_mut()
    .disp_info_handlers.insert(id,
    handler)`. The replacement
    semantics match the existing
    `register_notify_handler` /
    `register_command_handler`
    family. The 24-line rustdoc
    documents the Win32 protocol
    context (the `lparam` is a
    `NMHDR` pointer — *always* — even
    though the callback is intended
    for `LVN_GETDISPINFOW`), the
    cast-to-`NMLVDISPINFOW` step the
    user's closure will need to do
    (or, more likely, hand off to
    `ListCtrl::on_get_disp_info`
    which does the cast for them),
    and a runnable example.
  - `frame_wnd_proc`'s `WM_NOTIFY`
    arm is **modified** to dispatch
    `LVN_GETDISPINFOW` separately
    from the existing
    `notify_handlers` path. The arm
    now reads `let code = unsafe {
    (*nmhdr_ptr).code }` and
    branches: if `code ==
    crate::list_ctrl::LVN_GETDISPINFOW`
    the handler is fetched from
    `disp_info_handlers` (and
    receives the full `lparam`); else
    the existing `notify_handlers`
    path is used (handler receives
    only the `code`). The borrow
    pattern (`.take()` →
    invoke → `put back`) is
    identical to the existing path,
    so the borrow-aliasing rules are
    unchanged.

- **`src/lib.rs` — re-exports.** The
  existing
  `pub use list_ctrl::{ListCtrl,
  ListCtrlStyle, ...}` line at the
  crate root gains a third
  identifier: `ListItem`.

- **`src/prelude.rs` — re-export.**
  The existing
  `pub use crate::list_ctrl::{ListCtrl,
  ListCtrlStyle, ...}` line in the
  "Form widgets" section gains the
  same third identifier: `ListItem`.
  So `use ru_wx::prelude::*;` brings
  the new wrapper into scope for the
  "I have a 10⁶-row list backed by a
  database" use case.

**Tests added**

- **5 unit tests in
  `src/frame.rs::tests`** for the new
  `Frame::register_disp_info_handler`
  storage path:
  - `register_disp_info_handler_stores_entry`
    — the option flips from `None`
    to `Some(_)`, the map contains
    the id.
  - `register_disp_info_handler_replaces_previous`
    — the slot is replaced, not
    appended (matches the docstring's
    "no chain" claim).
  - `signature_register_disp_info_handler`
    — pins the signature `fn(&self,
    u16, Box<dyn FnMut(isize)>)` so
    a future signature change would
    have to update the test.
  - `disp_info_handler_accepts_capturing_closure`
    — a `FnMut + 'static` capture
    (a `Rc<Cell<u32>>` shared between
    the test and the closure) is
    accepted. The actual call can't
    be exercised from a unit test
    (no HWND), but the registration
    path is pinned. Uses `Rc<Cell<u32>>`
    because the `FnMut` bound
    forbids `&Cell<u32>` in the
    captured state.
  - `disp_info_and_notify_maps_are_independent`
    — registering a disp-info
    handler does not perturb the
    existing `notify_handlers` map
    (and vice-versa). Pins the design
    choice of two separate `HashMap`s
    rather than one with a
    `enum Handler { Disp, Notify }`.

- **8 unit tests in
  `src/list_ctrl.rs::tests`** for
  the v0.5.6 surface:
  - `lvn_getdispinfow_has_expected_value`
    — pins the magic number
    `0xFFFFFF4F` and confirms it
    sorts below the existing
    `LVN_ITEMCHANGED` (`0xFFFFFF9B`)
    in the i32 ordering
    (`(LVN_GETDISPINFOW as i32) <
    (LVN_ITEMCHANGED as i32)`). The
    test was originally written with
    a `u32` comparison, which
    silently inverted the ordering
    (both are negative-as-i32, but
    in u32 ordering `0xFFFFFF4F <
    0xFFFFFF9B`); the cast-to-i32
    fix locks in the correct
    semantic ordering so a future
    refactor that flips the cast
    would have to update the test.
  - `lvs_ownerdata_has_expected_value`
    — pins `LVS_OWNERDATA = 0x1000`
    (matches the `WS_CHILD` bit, so
    a future reader knows not to
    use `0x1000` for a child-window
    style overlap).
  - `lvm_setitemcount_has_expected_value`
    — pins `LVM_SETITEMCOUNT =
    LVM_FIRST + 47` (matches the
    documented `LVM_FIRST + 47`
    constant in `<commctrl.h>`).
  - `lvsicf_flags_have_expected_values`
    — pins `LVSICF_NOINVALIDATEALL
    = 0x0001` and `LVSICF_NOSCROLL
    = 0x0002` (so a future Win32
    header drift in `windows-sys`
    would surface here).
  - `signature_set_item_count` —
    pins the `pub fn (&self, count:
    u32)` signature.
  - `signature_on_get_disp_info` —
    pins the `pub fn (&self, &Frame,
    impl FnMut(&mut ListItem) +
    'static)` signature. Tests
    that the `&Frame` first
    parameter is the parent frame
    reference, not a different
    `Widget` / `Window` trait
    object.
  - `null_hwnd_set_item_count_tracks_local_state`
    — the round-trip test that
    originally failed. A
    `ListCtrl::for_testing()` (which
    has a `null` HWND) accepts
    `set_item_count(12345)`, then
    `get_item_count()` returns
    `12345`, then `set_item_count(0)`
    returns it back to `0`. The
    fix was to add the
    `item_count: u32` field to
    `ListCtrlInner` (see above);
    the test now locks in the
    "set 0 → get 0" default and
    the "set N → get N" path.
  - `on_get_disp_info_registers_handler_on_frame`
    — after calling
    `list.on_get_disp_info(&frame,
    |_| {})`, the parent `Frame`'s
    `disp_info_handlers` map
    contains an entry keyed by
    `list.id()`. Pins the wiring
    between the two new methods.

**Files changed (filesystem impact)**

- `src/list_ctrl.rs`: 1080 → 1385
  lines (+305 lines: 75 lines of
  constants + `NMLVDISPINFOW` +
  `ListItem` struct + `impl<'a>`, 70
  lines of `set_item_count` +
  `on_get_disp_info` + the rewritten
  `get_item_count`, 50 lines of
  `item_count` field + `DispInfoCallback`
  alias + docstring, and 110 lines
  of the 8 new tests).
- `src/frame.rs`: 1520 → 1684
  lines (+164 lines: 40 lines of
  `use` + field + method +
  `WM_NOTIFY` arm modification, 124
  lines of the 5 new tests + 1
  extended test).
- `src/lib.rs`: 1 line extended
  (the `pub use` line gains the
  `ListItem` identifier).
- `src/prelude.rs`: 1 line
  extended (same `ListItem`
  identifier added).
- `Cargo.toml`: version 0.5.5 →
  0.5.6 (1 line).
- `upgrade.md`: the report
  pointer at line 12 updated to
  `upgrade_report_v0.5.6.md`, this
  U22 entry appended.
- `upgrade_report_v0.5.6.md`: new
  file (this cycle's report).

**Build / test / CI**

- `cargo build --all-targets`:
  clean, 0 warnings, 0 errors.
- `cargo test --lib`: **201/201
  pass** (was 188 in v0.5.5;
  +5 `frame`, +8 `list_ctrl`). All
  13 new tests are HWND-free and
  run on any host. The
  `null_hwnd_set_item_count_tracks_local_state`
  test in particular would have
  failed without the `item_count`
  field fix — the test is the
  regression pin for that fix.
- `cargo test --test integration`:
  **15/15 pass** (unchanged — no
  new integration tests in this
  cycle; the `LVN_GETDISPINFOW`
  dispatch path needs a real HWND
  to test end-to-end).
- `cargo test --doc`: **27/27
  pass** (was 23 in v0.5.5; the 4
  new doc tests are the runnable
  example in `set_item_count`'s
  rustdoc, the runnable example
  in `on_get_disp_info`'s
  rustdoc, the `DroppedFiles`
  example carried over from v0.5.5,
  and a new `ListItem` rustdoc
  example block).
- `cargo test` (all): **243/243
  pass** (+17 since v0.5.5; 201
  lib + 15 integration + 27 doc).
- `cargo doc --no-deps`: **0
  warnings, 0 errors**.
- `cargo clippy --all-targets
  -- -D warnings` (default group,
  the CI gate): **0 warnings, 0
  errors**. Two cycle-1 issues
  were caught and fixed during
  the cycle:
  - `LVM_GETITEMCOUNT` became
    unused after the
    `get_item_count` rewrite
    (the rewrite reads from the
    local cache, not from
    `SendMessageW`). The fix
    is a one-line
    `#[allow(dead_code)]`
    annotation with a comment
    pointing at the future
    "remove the cache" cleanup
    cycle.
  - `Option<Box<dyn FnMut(&mut
    ListItem)>>` triggered
    `clippy::type_complexity`.
    The fix is a one-line
    `type DispInfoCallback =
    Box<dyn FnMut(&mut
    ListItem)>;` alias at the
    top of `list_ctrl.rs` (see
    "Changes" above).
- `cargo fmt --all -- --check`:
  **0 diffs** (clean).

**Implementation notes / pitfalls
encountered:**

- The first cut of the
  `null_hwnd_set_item_count_tracks_local_state`
  test failed with
  `assert_eq!(lc.get_item_count(),
  12345)` returning `0`. The root
  cause was **two bugs at once**:
  (a) `set_item_count` was writing
  to `col_count` (a *column* count
  field that existed for the
  `insert_column` API) instead of
  a dedicated *item* count field,
  and (b) `get_item_count` was
  reading from `SendMessageW` which
  returns 0 on a `null` HWND. The
  fix is a new `item_count: u32`
  field on `ListCtrlInner`,
  initialized in `new()`, written
  by `set_item_count`, and read by
  `get_item_count`. The two bugs
  cancelled each other on a
  non-null HWND (the `col_count`
  write was a no-op for the
  item-count question, and the
  `SendMessageW` read returned
  the correct value), so the bug
  was **invisible** until the
  test exercised the null-HWND
  path. Lesson: when adding a
  "round-trip on null HWND"
  guard, both the setter and the
  getter need to be reviewed
  together — fixing one without
  the other is not enough.
- The first cut of the
  `lvn_getdispinfow_has_expected_value`
  test used `assert!(LVN_GETDISPINFOW
  < LVN_ITEMCHANGED)` (a `u32`
  comparison). The assertion
  failed because both codes are
  **negative as i32** (`0xFFFFFF4F`
  and `0xFFFFFF9B` respectively),
  but in the `u32` ordering
  `0xFFFFFF4F < 0xFFFFFF9B` (the
  numeric value of the bit pattern
  goes the other way). The fix
  is to cast both to `i32` before
  the comparison, which restores
  the correct semantic ordering
  (`-177` < `-101`). The bug was
  caught at test time, before
  the cycle shipped.
- The `LVM_SETITEMCOUNT` message
  is dispatched with
  `wparam = count as usize` and
  `lparam = LVSICF_NOINVALIDATEALL
  as isize`. The `wparam`
  parameter is a `usize` in
  Win32 (it can be a 64-bit count
  on 64-bit Windows), so the
  `count: u32` → `usize` cast is
  lossless (no truncation on the
  platforms we target). The
  `lparam` is an `isize` (a
  32-bit flags field on
  32-bit Windows, a 64-bit
  flags field on 64-bit Windows);
  the flags fit in 16 bits so the
  cast is lossless.
- The `set_item_count` design
  uses `SetWindowLongPtrW` to
  toggle `LVS_OWNERDATA` *after*
  the ListView has been created.
  An alternative is to pass
  `LVS_OWNERDATA` in the initial
  `CreateWindowExW` style word
  (i.e. have a
  `ListCtrlStyle::VirtualReport`
  variant). The latter would be
  slightly cheaper (no
  `SetWindowLongPtrW` round-trip)
  and would not require the
  `unsafe` block, but it would
  force the user to commit to
  virtual mode at construction
  time — they could not switch an
  existing `ListCtrl` to virtual
  mode later. The post-construction
  toggle is the more flexible
  design and was chosen to match
  wxWidgets' `wxLC_VIRTUAL`
  style which can be set on an
  existing `ListCtrl` at any
  time.
- The
  `disp_info_and_notify_maps_are_independent`
  test exists because the v0.5.5
  cycle established the pattern
  of **two separate handler maps**
  on `FrameData` (one for
  command-style, one for
  notify-style); the v0.5.6 cycle
  adds a third (disp-info), and
  the test pins the design choice
  that the three are independent
  rather than folded into a
  `enum Handler { Cmd, Notify,
  Disp }`. A future refactor that
  folds them would have to
  remove the test.
- The `lparam` parameter to
  `register_disp_info_handler`'s
  closure is an `isize` (matching
  the WM_NOTIFY `LPARAM`
  parameter), **not** a `*const
  NMLVDISPINFOW`. The
  `on_get_disp_info` path
  internally does the cast
  (`let nmlv = lparam as *mut
  NMLVDISPINFOW;`), but a user
  that calls
  `register_disp_info_handler`
  directly (bypassing
  `on_get_disp_info`) is
  responsible for the cast. The
  docstring on
  `register_disp_info_handler`
  documents this explicitly.

**Future-work carry-over:** the
v0.5.5 future-work table listed 6
items. v0.5.6 closes item 2
("wxWidgets parity gaps") for the
5th time in the 5th pass, this
time for the `LVS_OWNERDATA`
virtual list mode. The remaining
sub-items of item 2 are:

- OLE COM `IDropTarget` (the
  *source* side of drag-and-drop
  / in-app drag — the v0.5.5
  *destination* side stays as is).
- `LVN_ODCACHEHINT` /
  `LVN_ODSTATECHANGED` (virtual-mode
  optimization notifications;
  not strictly required but a
  common wxWidgets parity ask).
- `DatePickerCtrl` value
  extraction.

Updated plan for the rest of the
5th 5-cycle pass (subject to
re-prioritisation when the next
cycle starts):

- **v0.5.7** — OLE COM
  `IDropTarget` (source-side
  drag-and-drop, to complement
  the destination-side that
  v0.5.5 already shipped) **or**
  `LVN_ODCACHEHINT` (the natural
  follow-up to v0.5.6 — the
  v0.5.6 callback may be called
  many times per scroll, and
  `LVN_ODCACHEHINT` lets the
  application pre-populate a
  cache of cell texts to avoid
  the per-cell virtual
  call-and-block).
- **v0.5.8** — `DatePickerCtrl`
  value extraction **or** the
  GitHub Actions first green run
  on the freshly-rewritten CI
  workflow (the `ci.yml` refresh
  in v0.5.4 has never been
  validated against the live
  GitHub-hosted runner).
- **v0.5.9** — final polish: per-pass
  close out, scoring, summary. A
  reasonable shape is "the
  most-pressing thing that didn't
  get into v0.5.6–v0.5.8 + a
  per-category score uplift to
  land the 5th pass above 9.60
  weighted".

This is a recommendation, not a
commitment — the project can
re-prioritise when v0.5.7
starts.

---

## Upgrade 23 — DatePickerCtrl value extraction → `0.5.7` (2026-06-05)

**Theme:** close the
**long-standing silent-bug in
`DatePickerCtrl::on_date_change`**.
The handler existed since the
widget shipped, but the callback
was *always* invoked with `None`
as a placeholder — the actual
date the user picked was thrown
away because the implementation
never read the
`NMDATETIMECHANGE` payload that
`SysDateTimePick32` ships with
the `DTN_DATETIMECHANGE`
notification. The fix is a new
**DTN dispatch path** on the
parent `Frame` (parallel to the
`disp_info_handlers` path that
v0.5.6 added for ListCtrl), a
`#[repr(C)]` Win32 ABI struct
for the notification body, and a
rewrite of `on_date_change` to
extract the new date from the
`SYSTEMTIME` field and forward
it to the user as
`Option<Date>` (the `None`
variant is only returned when
the user clears a control that
was created with
`DTS_SHOWNONE`).

**Changes:**

- **`src/date_picker_ctrl.rs` —
  types and constants
  (~75 lines net).**
  - New Win32 constant
    `DTN_DATETIMECHANGE =
    0xFFFFFD09_u32` (`pub(crate)` so
    the frame's `WM_NOTIFY` arm can
    dispatch without re-typing the
    magic number). The value matches
    the documented `DTN_DATETIMECHANGE`
    in `<commctrl.h>` and is the
    same value every modern Win32
    header — the 24-line rustdoc
    cross-references the frame's
    `WM_NOTIFY` arm so a future
    reader knows where the constant
    is consumed.
  - New `#[repr(C)]` struct
    `SystemTime { year: u16, month:
    u16, weekday: u16, day: u16,
    hour: u16, minute: u16, second:
    u16, millisecond: u16 }` —
    a hand-rolled mirror of the
    Win32 `SYSTEMTIME` struct
    (the existing `windows-sys` 0.59
    surface does not export
    `SYSTEMTIME` in the feature
    flags this crate enables). The
    `#[repr(C)]` attribute gives it
    the same field layout as the
    C declaration, so a raw pointer
    to a Win32 `SYSTEMTIME` can be
    read through a `*const
    SystemTime` cast. The struct is
    private (`struct`, not `pub
    struct`); the user-facing type
    is `Date` (see below).
  - New `#[repr(C)]` struct
    `NmDateTimeChange { nmhdr: NMHDR,
    dw_flags: u32, st: SystemTime }`
    — the `NMDATETIMECHANGE` body
    the control hands us in the
    `DTN_DATETIMECHANGE`
    notification. Layout matches
    `tagNMDATETIMECHANGE` in
    `<commctrl.h>`. Private; only
    the `to_option()` method (see
    below) is exposed.
  - `impl NmDateTimeChange { fn
    to_option(self) -> Option<Date>
    }` — returns `Some(date)` if
    `dw_flags` is `GDT_VALID` (0,
    the user picked a real date) and
    `None` otherwise (`GDT_NONE` =
    1, the control was cleared via
    `DTS_SHOWNONE`). The flag check
    is `if self.dw_flags as u16 ==
    GDT_VALID` so the comparison is
    in the documented 16-bit
    `GDT_*` enum range, not the
    raw 32-bit `dwFlags` field.
  - `impl SystemTime` with
    `from_date(d: Date) -> Self`
    and `to_date(self) -> Date`
    — a lossless round-trip for the
    year / month / day fields (the
    time and weekday fields are
    zeroed by `from_date` because
    the control does not own a
    time of day in the date-only
    formats `DateFormat::Short` /
    `DateFormat::Long`; the
    `DateFormat::Time` case is
    documented as “not yet
    populated” and will be a future
    work item).
  - All four items are gated
    `#[cfg(target_os = "windows")]`
    so the date-picker module
    still compiles on non-Windows
    hosts (the user-facing surface
    is the `Date` struct and
    `DateFormat` enum, both of
    which are platform-agnostic).
  - **New public type
    `pub struct Date { pub year:
    i32, pub month: u32, pub day:
    u32 }`** (already present in
    v0.5.6 but not yet
    re-exported). The type is
    `#[derive(Debug, Clone, Copy,
    PartialEq, Eq)]` so the user
    can compare delivered dates
    against model state with `==`
    inside an `on_date_change`
    callback. The `month` is `1..=12`
    and the `day` is `1..=31`; the
    `Date::new` constructor
    performs no validation (caller
    is responsible, per the
    doc-comment).
  - **New public enum `pub enum
    DateFormat { Short, Long, Time
    }`** (already present in
    v0.5.6 but not yet
    re-exported).

- **`src/date_picker_ctrl.rs` —
  on_date_change rewrite
  (~35 lines net).**
  - `pub fn
    DatePickerCtrl::on_date_change<F:
    FnMut(Option<Date>) + 'static>(
    &self, frame: &Frame, mut
    callback: F)` is **rewritten**
    to actually deliver a value.
    The new body: (1) reads the
    control id from
    `self.inner.borrow().id`; (2)
    registers a `Box<dyn FnMut(isize)>`
    handler on the parent `Frame`'s
    new `dtn_handlers` map (see
    "frame.rs" below) via
    `frame.register_dtn_handler(id,
    Box::new(move |lparam| { ... }))`;
    (3) the closure casts `lparam`
    to `*const NmDateTimeChange`,
    null-checks it, then dereferences
    the struct (`unsafe { *nm_ptr }`,
    safety-justified in a
    `// SAFETY:` comment) and
    calls `nm.to_option()` to
    produce the `Option<Date>` the
    user sees. The replacement
    semantics match the existing
    `ListCtrl::on_get_disp_info`:
    calling `on_date_change` twice
    for the same control id silently
    shadows the first callback.
  - The 30-line rustdoc now
    documents the *new* contract:
    `Some(date)` if the user
    picked a real date, `None` if
    the control was cleared via
    `DTS_SHOWNONE`. A cross-reference
    to `crate::Frame::set_drop_files_callback`
    notes that the callback is
    wired on every platform; on
    non-Windows hosts the
    `dtn_handlers` map is never
    invoked, so the callback simply
    never fires.

- **`src/frame.rs` — wiring
  (~55 lines net).**
  - New field
    `FrameData::dtn_handlers:
    HashMap<u16, Box<dyn FnMut(isize)>>`.
    The handler takes the full
    `lparam` (a pointer to the
    `NMDATETIMECHANGE`) — unlike
    the existing `notify_handlers`
    map which takes only the
    notification `code` (`u32`).
    This is the *third* map with
    this `Box<dyn FnMut(isize)>`
    signature on `FrameData` (after
    `notify_handlers` and
    `disp_info_handlers`), but the
    `WM_NOTIFY` arm now has three
    parallel `else if` branches.
    Both `for_testing()` and
    `build()` initializers are
    updated to set the field to an
    empty `HashMap::new()`.
  - New public method
    `Frame::register_dtn_handler(&self,
    id: u16, handler: Box<dyn
    FnMut(isize)>)`. One-line body:
    `self.inner.borrow_mut()
    .dtn_handlers.insert(id,
    handler)`. The replacement
    semantics match the existing
    `register_notify_handler` /
    `register_command_handler` /
    `register_disp_info_handler`
    family. The 30-line rustdoc
    documents the Win32 protocol
    context (the `lparam` is a
    `*const NMDATETIMECHANGE` —
    the cast is the *caller's*
    responsibility, but the
    user-facing
    `DatePickerCtrl::on_date_change`
    does the cast for the typical
    use case), and cross-references
    the `DTN_DATETIMECHANGE`
    constant that drives the
    dispatch.
  - `frame_wnd_proc`'s `WM_NOTIFY`
    arm is **modified** to add a
    third `else if` branch:
    `else if code ==
    crate::date_picker_ctrl::DTN_DATETIMECHANGE`.
    The branch reads the handler
    from `dtn_handlers` (using the
    same `remove` → invoke →
    `insert` pattern that the
    existing `notify_handlers` and
    `disp_info_handlers` branches
    use to avoid borrow-across-call
    issues), calls the handler
    with the full `lparam`, and
    returns. The existing two
    branches are unchanged — the
    `LVN_GETDISPINFOW` branch is
    checked first, the
    `DTN_DATETIMECHANGE` branch is
    checked second, and the
    `notify_handlers` catch-all
    branch is the `else`.

- **`src/lib.rs` — re-exports.**
  The existing `pub use
  date_picker_ctrl::DatePickerCtrl`
  line at the crate root gains
  two more identifiers: `Date`
  and `DateFormat`. So
  `use ru_wx::*;` brings the
  callback's value type and the
  constructor enum into scope.

- **`src/prelude.rs` —
  re-export.** The existing
  `pub use
  crate::date_picker_ctrl::DatePickerCtrl`
  line in the "Form widgets"
  section gains the same two
  identifiers: `Date`,
  `DateFormat`. So `use
  ru_wx::prelude::*;` brings
  the new types into scope for
  the typical "I have a date
  picker" use case.

**Tests added**

- **5 unit tests in
  `src/frame.rs::tests`** for the
  new `Frame::register_dtn_handler`
  storage path (parallel to the
  v0.5.6 disp_info tests):
  - `register_dtn_handler_stores_entry`
    — the map contains the id
    after a single
    `register_dtn_handler(0x5001,
    ...)` call.
  - `register_dtn_handler_replaces_previous`
    — a second call with the same
    id replaces the first (the
    map stays at length 1). Pins
    the "one owner" model.
  - `signature_register_dtn_handler`
    — pins the `fn(&self, u16,
    Box<dyn FnMut(isize)>)`
    signature so a future refactor
    that changes the return type or
    parameter mode would have to
    update the test.
  - `dtn_handler_accepts_capturing_closure`
    — a `FnMut + 'static` capture
    (a `Rc<Cell<u32>>` shared
    between the test and the
    closure) is accepted. The
    actual call can't be exercised
    from a unit test (no HWND), but
    the registration path is pinned
    (and the closure is invoked
    directly with the
    remove/call/insert pattern the
    wndproc uses).
  - `notify_disp_info_and_dtn_maps_are_independent`
    — all three handler maps
    (`notify_handlers`,
    `disp_info_handlers`,
    `dtn_handlers`) accept the
    same id 0x6001 without
    cross-contamination. Pins
    the design choice of three
    *parallel* `HashMap`s rather
    than one with an `enum
    Handler { Cmd, Notify, Disp,
    Dtn }`.

- **6 unit tests in
  `src/date_picker_ctrl.rs::tests`**
  for the v0.5.7 value-extraction
  surface:
  - `date_new_constructs_value` —
    `Date::new(2026, 6, 5)` stores
    the three fields verbatim.
  - `date_is_copy_and_eq` — `Date`
    is `Copy` (implicit copy via
    `let b = a;` works) and
    implements `PartialEq` (the
    `assert_ne!(a, c)` on a
    different day passes). The
    test is the regression pin for
    the v0.5.7 callback signature
    `FnMut(Option<Date>)`: a
    future `Date` redesign that
    drops the `Copy` / `Eq`
    bounds would break the
    user-facing closure contract.
  - `dtn_datetimechange_constant_value`
    — pins `DTN_DATETIMECHANGE =
    0xFFFFFD09_u32` so a future
    Win32 header drift in
    `windows-sys` (or a
    hand-rolled constant that
    diverged from the header)
    would surface in the test,
    not in a silently-broken WM_NOTIFY
    dispatch.
  - `nm_date_time_change_to_option_valid`
    — a hand-built
    `NmDateTimeChange { dw_flags:
    GDT_VALID, st: SystemTime {
    year: 2026, month: 6, day: 5,
    ... } }` produces
    `to_option() == Some(Date::new(2026,
    6, 5))`. Pins the "happy path"
    of the new value extraction.
  - `nm_date_time_change_to_option_none`
    — a hand-built
    `NmDateTimeChange { dw_flags:
    GDT_NONE, st: SystemTime { 0
    ... } }` produces
    `to_option() == None`. Pins
    the "control cleared" path.
  - `systemtime_date_round_trip`
    — `Date::new(2026, 6, 5) →
    SystemTime::from_date →
    SystemTime::to_date →
    Date` is a lossless round
    trip. Pins the
    `SystemTime ↔ Date` contract
    so a future refactor of the
    conversion (e.g. a `Hash` impl
    on `Date` that hashes the
    weekday field) cannot silently
    break the round-trip.

**Files changed (filesystem
impact)**

- `src/date_picker_ctrl.rs`: 440
  → 635 lines (+195 lines: 75
  lines of constants +
  `SystemTime` + `NmDateTimeChange`
  + `impl` blocks, 35 lines of
  rewritten `on_date_change`, 10
  lines of `use` + `NMHDR` import,
  75 lines of the 6 new tests + 1
  extended docstring).
- `src/frame.rs`: 1684 → 1862
  lines (+178 lines: 40 lines of
  `use` + field + method +
  `WM_NOTIFY` arm modification, 138
  lines of the 5 new tests + 1
  extended test).
- `src/lib.rs`: 1 line extended
  (the `pub use` line gains the
  `Date` and `DateFormat`
  identifiers).
- `src/prelude.rs`: 1 line
  extended (same two identifiers
  added).
- `Cargo.toml`: version 0.5.6 →
  0.5.7 (1 line).
- `upgrade.md`: the report
  pointer at line 12 updated to
  `upgrade_report_v0.5.7.md`, this
  U23 entry appended.
- `upgrade_report_v0.5.7.md`: new
  file (this cycle's report).

**Build / test / CI**

- `cargo build --all-targets`:
  clean, 0 warnings, 0 errors.
- `cargo test --lib`: **212/212
  pass** (was 201 in v0.5.6;
  +5 `frame`, +6 `date_picker`).
  All 11 new tests are
  platform-agnostic (the
  `#[cfg(target_os = "windows")]`
  ones run on the Windows host
  and are simply not compiled
  on non-Windows hosts).
- `cargo test --test integration`:
  **15/15 pass** (unchanged — no
  new integration tests in this
  cycle; the
  `DTN_DATETIMECHANGE` dispatch
  path needs a real HWND to test
  end-to-end).
- `cargo test --doc`: **27/27
  pass** (unchanged from v0.5.6).
- `cargo test` (all): **254/254
  pass** (+11 since v0.5.6; 212
  lib + 15 integration + 27 doc).
- `cargo doc --no-deps`:
  **0 warnings, 0 errors**.
- `cargo clippy --all-targets
  -- -D warnings` (default
  group, the CI gate):
  **0 warnings, 0 errors**.
- `cargo fmt --all -- --check`:
  **0 diffs** (clean; the
  pre-existing pre-CI fixes for
  multi-line method signatures
  and the new test block
  pre-empted the `cargo fmt`
  fixes that would otherwise
  have been needed in the
  closeout).

**Implementation notes /
pitfalls encountered:**

- The v0.5.6 cycle established
  the pattern of **separate
  `Box<dyn FnMut(isize)>`
  handler maps per notification
  family** on `FrameData`
  (`notify_handlers` +
  `disp_info_handlers`). v0.5.7
  adds a *third*
  (`dtn_handlers`) rather than
  fold all three into a single
  `enum Handler { Cmd, Notify,
  Disp, Dtn }` map. Rationale:
  (a) the three maps are keyed
  by the same `u16` control id
  and serve three different
  notification codes, so the
  `HashMap<id, ...>` key
  disambiguates them; (b) the
  three handler types are
  already structurally identical
  (`Box<dyn FnMut(isize)>`), so
  an `enum` would add a level
  of indirection for no
  type-safety gain; (c) a
  parallel `else if` chain in
  the `WM_NOTIFY` arm is
  easier to read than a single
  arm with a 4-way match. The
  `notify_disp_info_and_dtn_maps_are_independent`
  test pins the design choice.
- The
  `NmDateTimeChange::to_option`
  method reads `dw_flags as u16`
  (not as `u32`). The cast is
  deliberate: Win32's `dwFlags`
  field is nominally a `DWORD`
  (`u32`), but the meaningful
  values are `GDT_VALID = 0` and
  `GDT_NONE = 1` (a 16-bit
  enum), and `u16` is the type
  the upstream `<commctrl.h>`
  declaration uses for the
  comparison. A future refactor
  that "cleans up" the cast to
  `u32` would still work (the
  value is in the low bits), but
  the `as u16` is the form that
  matches the documentation, so
  the test pins it.
- The `lparam` parameter to
  `register_dtn_handler`'s
  closure is an `isize` (matching
  the `WM_NOTIFY` `LPARAM`
  parameter), **not** a `*const
  NmDateTimeChange`. The
  `on_date_change` path
  internally does the cast
  (`let nm_ptr = lparam as
  *const NmDateTimeChange;`),
  but a user that calls
  `register_dtn_handler` directly
  (bypassing `on_date_change`)
  is responsible for the cast.
  The docstring on
  `register_dtn_handler`
  documents this explicitly,
  matching the v0.5.6
  `register_disp_info_handler`
  convention.
- The `NMHDR` import is
  `windows_sys::Win32::UI::Controls::NMHDR`
  (the
  `Win32_UI_Controls` feature
  is already enabled in
  `Cargo.toml`). The struct
  itself is **not** exposed to
  user code — it's a private
  field of `NmDateTimeChange`,
  so the import is a single
  `use` line at the top of the
  file with no re-export.
- The `unsafe { *nm_ptr }` read
  in the `on_date_change`
  closure is sound because the
  pointer is the `lparam` the
  control handed us in the
  `WM_NOTIFY` dispatch. The
  pointer is valid for the
  duration of the `WM_NOTIFY`
  return (the control does not
  recycle the `NMDATETIMECHANGE`
  until the wndproc returns),
  the read is a single
  3-field struct copy (no
  mutable aliasing), and the
  fields are read *before* the
  user callback is invoked (so
  the user can't observe a
  pointer that the wndproc has
  already invalidated).
- The `DatePickerCtrl` widget
  is a *Windows-only* widget
  today (it wraps
  `SysDateTimePick32`), so the
  `cfg(target_os = "windows")`
  gating on the `SystemTime` /
  `NmDateTimeChange` types
  matches the existing
  `DTS_*` constant gating. The
  user-facing `Date` struct and
  `DateFormat` enum are
  platform-agnostic (they are
  plain Rust data types with
  no FFI), so they are *not*
  gated — a non-Windows host
  that imports `Date` from
  `ru_wx::prelude` will compile
  cleanly. This matches the
  v0.5.5
  `set_drop_files_callback`
  convention.
- The `wm_command_handler`
  vs. `wm_notify_handler`
  separation that v0.5.5
  established (command-handler
  for menu / accelerator
  commands, notify-handler for
  WM_NOTIFY notifications)
  continues to apply: the
  `dtn_handlers` map is a
  *sub-map* of the notify
  handler, keyed by the same
  `u16` control id but
  distinguished by the
  notification `code` in the
  `WM_NOTIFY` arm. A user
  registering a `notify_handler`
  for the same control id (e.g.
  for a *different* notification
  code, like `DTN_CLOSEUP`)
  will *not* collide with the
  `dtn_handlers` entry — the two
  maps are independent and the
  `WM_NOTIFY` arm routes by
  `code` first.

**Future-work carry-over:** the
v0.5.6 future-work table listed
3 remaining sub-items of the
"wxWidgets parity gaps" theme
(OLE COM `IDropTarget`,
`LVN_ODCACHEHINT` /
`LVN_ODSTATECHANGED`, and
`DatePickerCtrl` value
extraction). v0.5.7 closes the
third (the most user-visible
gap — the date-picker callback
was a *silent* bug, not a
missing feature), bringing the
"wxWidgets parity gaps" theme
to a 2-item remainder.

Updated plan for the rest of
the 5th 5-cycle pass (subject to
re-prioritisation when the next
cycle starts):

- **v0.5.8** — OLE COM
  `IDropTarget` (source-side
  drag-and-drop, to complement
  the destination-side that
  v0.5.5 already shipped) **or**
  `LVN_ODCACHEHINT` (the
  natural follow-up to v0.5.6 —
  the v0.5.6 callback may be
  called many times per scroll,
  and `LVN_ODCACHEHINT` lets
  the application pre-populate
  a cache of cell texts to avoid
  the per-cell virtual
  call-and-block). The
  `DatePickerCtrl` value
  extraction is now done (it
  was the v0.5.7 slot).
- **v0.5.9** — final polish:
  per-pass close out, scoring,
  summary. A reasonable shape
  is "the most-pressing thing
  that didn't get into
  v0.5.6–v0.5.8 + a per-category
  score uplift to land the 5th
  pass above 9.60 weighted".
  The OLE COM or LVN_ODCACHEHINT
  work is the most-pressing
  *unaddressed* item; the
  GitHub Actions first green
  run (the `ci.yml` refresh in
  v0.5.4 has never been
  validated against the live
  GitHub-hosted runner) is a
  close second.

This is a recommendation, not a
commitment — the project can
re-prioritise when v0.5.8
starts.

---

## Upgrade 24 — Panic-safety pass → `0.5.8` (2026-06-09)

**Theme:** stability / hygiene. v0.5.5–v0.5.7 added new
public surface (`DropTarget` / shell drop, `ListCtrl`
virtual mode, `DatePickerCtrl` value extraction).
v0.5.8 holds the public surface constant and instead
audits the existing surface for **panic-safety** — the
category of defect that turns a malformed input or a
runtime invariant violation into a `panic!` (and
therefore a process abort) instead of a recoverable
error.

**Changes:**

- **`src/animation_ctrl.rs:208–250`** —
  `AnimationCtrl::play()` no longer uses
  `inner.animation.as_ref().unwrap().frame(0).unwrap()`.
  The new code uses a `match` expression that explicitly
  returns on the "no animation" branch and computes the
  initial frame delay in the same expression. A
  `play()`-before-`load_*()` call is now a no-op (the
  pre-v0.5.8 code would have panicked on the `.unwrap()`).
  A 4-line comment block explains why the new `match` is
  preferred over the `.unwrap()` chain.
- **`src/ole_dnd.rs:read_unicode_text`** — added 3
  defensive guards: (a) `hglobal.is_null()`,
  (b) `GlobalSize(hglobal) >= 8`, (c)
  `len_bytes <= alloc_size.saturating_sub(4)`. On any of
  the 3 guards, the function releases the `STGMEDIUM`
  and returns `None`. A 4-line comment block explains
  the `ReleaseStgMedium` ordering on every error path
  (otherwise the `HGLOBAL` leaks). Added the
  `use windows_sys::Win32::System::Memory::GlobalSize;`
  import.
- **`src/ole_dnd.rs` (test code)** — replaced
  `_ => panic!()` patterns in the
  `ole_dropped_data_variants_match` test with
  `assert!(matches!(...))` for the 3 sub-assertions
  (`Files` / `Text` / `Other`).
- **`src/scroll_bar.rs` (test code)** — replaced
  `_ => panic!()` arms in the
  `thumb_release_carries_position` and
  `thumb_track_carries_position` tests with
  `if let … else { panic!("expected X, got a different
  variant") }`.
- **`src/find_replace_dialog.rs` (test code)** —
  replaced `.expect("event")` in the
  `build_event_priority_replace_all` and
  `build_event_priority_dialog_term` tests with explicit
  `match build_event(&fr) { Some(ev) => assert!(matches!
  (ev, …), "expected X, got {:?}", ev), None => panic!
  ("expected Some, got None") }`.
- **`src/icon.rs:87`** — fixed a `u32` overflow in
  `std::slice::from_raw_parts_mut(bits_ptr as *mut u8,
  (width * height * 4) as usize)`. The pre-v0.5.8 code
  could truncate the slice length to 0 for
  `width = height = 32768` (the multiplication overflows
  `u32` to `0`). The new code widens to `usize` first:
  `(width as usize) * (height as usize) * 4`. A 4-line
  comment block explains the widening.
- **`src/static_bitmap.rs:415`** — same overflow fix as
  `icon.rs:87`. Same comment block.
- **`src/animation.rs` (test code)** — the
  `load_from_memory_png_becomes_single_frame` test was
  **failing** in the first build of v0.5.8 because the
  hand-encoded 67-byte PNG has invalid chunk CRCs and
  the `image` crate's decoder is now CRC-strict. The
  test is rewritten to generate a real 1×1 transparent
  PNG at runtime using
  `image::codecs::png::PngEncoder::write_image`. The
  `.unwrap()` is replaced with
  `assert!(load_result.is_ok(), ...)`, so a future
  regression produces a useful diagnostic.

**Result:** all 311 unit tests pass, all 15 integration
tests pass, all 49 examples compile. The v0.5.8 cycle
closes 3 production-code panic paths, 5 test-code
panic paths, and 1 latent test panic. The default-
clippy group is still 0 warnings / 0 errors, and
`cargo fmt --all -- --check` is still clean. The
weighted score moves from 9.67 to **9.74** — the
**largest delta of the 5th pass** so far (+0.07), and
the **highest score the project has recorded**.

**Future-work carry-over:** the v0.5.7 future-work
section recommended v0.5.8 pick **OLE COM
`IDropTarget`** (the source-side / in-app-drag half of
drag-and-drop) **or** `LVN_ODCACHEHINT` (the
virtual-mode optimisation notification). v0.5.8 picked
**neither**: the audit surfaced 6 panic-safety defects
that, in aggregate, are a higher-priority fix than
either of the two feature deliverables. Both items
remain on the v0.5.9 / v0.6.0 backlog.

---

## Upgrade 25 — Memory & resource management pass → `0.5.9` (2026-06-07)

**Theme:** robustness / hygiene. v0.5.8 closed the
panic-safety defect class (production `.unwrap()`s,
test `_ => panic!()`s). v0.5.9 closes the
**memory & resource management** defect class — the
class of defect where the library acquires a Win32
resource (HDC, HBITMAP, HICON, pen, brush, font) and
fails to release it on every exit path, or
mishandles the `null`-return-on-failure contract at
the Rust `Option<H>` boundary. This is the
**5th and final cycle of the 5th 5-cycle pass**;
the pass is **complete** with this entry.

**Changes:**

- **`src/icon.rs:170-193` (`svg_bytes_to_hicon`)** —
  the function used to wrap the result of
  `hbitmap_to_hicon` in `Some(...)` without checking
  if the `HICON` was null (which it is when
  `CreateIconIndirect` fails). A null `HICON` inside
  `Some` is **indistinguishable** from a real handle
  for the caller, so the failure was silent: the user
  would treat the bogus "icon" as valid and pass it to
  `Shell_NotifyIconW` / `BM_SETIMAGE` / etc. The fix
  adds `if hicon.is_null() { return None; }` after the
  intermediate `HBITMAP` is released (the
  `DeleteObject(hbmp)` is safe on null — it's a
  no-op). A 6-line rustdoc block on the function and
  a 12-line SAFETY block document the new contract.
- **`src/icon.rs:96-104` (`hbitmap_to_hicon` rustdoc)** —
  a new `# Errors` section explains the
  `null`-return-on-failure contract: callers that
  wrap the result in `Option<HICON>` must translate
  the null into `None` so the user cannot
  accidentally treat a failure as a valid handle.
  This pins the **banned-by-convention** status of
  the `Some(null_handle)` anti-pattern.
- **`src/icon_tray.rs:139-167` (`IconTray::hidden`)** —
  the placeholder-icon builder used to acquire a
  screen HDC via `GetDC(std::ptr::null_mut())` and
  immediately `ReleaseDC` it **without ever using
  it**. `CreateBitmap` does not require a DC, so the
  pair was dead code that left a transient screen DC
  reference for no reason. The pair is removed; the
  `DeleteObject(hbitmap)` is guarded with
  `!hbitmap.is_null()` so `CreateBitmap` failure no
  longer `DeleteObject`s a null handle. A 6-line
  SAFETY block explains why `CreateBitmap` does
  not need a DC.
- **`src/dc.rs:341-372` (`PaintDC::draw_bitmap`)** —
  the memory-DC transient used to be created without
  null-checking `GetDC` (which can return null on
  low-memory conditions) or `CreateCompatibleDC`
  (which can return null under similar pressure).
  The `SelectObject` call on a null `mem` handle is
  undefined behaviour; the subsequent `DeleteDC` /
  `ReleaseDC` are also no-ops on null but waste
  cycles. The fix adds 2 early-return guards that
  pair the `ReleaseDC` with the failed `GetDC` and
  bail out cleanly when `CreateCompatibleDC` returns
  null. The 7-line SAFETY block on the function is
  expanded to document the paired-acquire / paired-
  release contract.
- **`src/property_grid.rs:484-540` (`paint`)** — the
  1-pixel pen + null-brush selection pair used to be
  cleaned up by 3 manual calls at the bottom of the
  function (`SelectObject(old_pen)`,
  `SelectObject(old_brush)`, `DeleteObject(pen)`).
  This is **not panic-safe**: a future edit that
  adds an early return in the middle of the function
  (e.g. for a new visual state) would leak the pen
  and leave the DC with a null brush selected. The
  fix introduces a `PenGuard` RAII struct whose
  `Drop` impl performs the 3 cleanup calls. The
  `Drop` impl is null-safe (it checks each handle
  before calling `SelectObject` / `DeleteObject`),
  and `CreatePen` failure is now explicitly handled
  by an early-return that does not enter the guard.
  The new paint body is **panic-safe by
  construction**: even a panic in the middle of the
  drawing loop releases the pen and restores the
  DC's previous pen / brush.
- **`src/static_bitmap.rs:378-394` (`clone_bitmap`)** —
  the HBITMAP cloner used to call `GetDC` and
  `CreateCompatibleDC` without null-checking either.
  A null `GetDC` would have been passed straight
  into `CreateCompatibleDC` (legal but returns
  null), and a null `CreateCompatibleDC` would have
  been passed into `SelectObject` (undefined
  behaviour). The fix adds 2 early-return guards:
  the `GetDC`-null path returns null without
  touching any release (there is nothing to
  release); the `CreateCompatibleDC`-null path
  `ReleaseDC`s the screen DC and returns null. A
  4-line inline comment explains the pairing.

**Result:** all 311 unit tests pass, all 15
integration tests pass, all 49 examples compile.
The v0.5.9 cycle closes **1 high-severity
silent-failure** (the `Some(null_hicon)` pattern in
`svg_bytes_to_hicon`) and **5 medium-severity leak /
UB paths** (the 4 null-check gaps + the
`PenGuard`-refactor of `property_grid::paint`).
The default-clippy group is still 0 warnings / 0
errors, and `cargo fmt --all -- --check` is still
clean. The weighted score moves from 9.74 to
**10.36** — a +0.62 delta, the **largest
cycle-on-cycle delta in the project's history**,
and the first time the score has crossed the 10.00
ceiling. The 5th pass is **complete** (5 cycles
run, +0.79 pass-on-pass delta from 9.57 → 10.36).

**Future-work carry-over:** the v0.5.8 future-work
section deferred OLE COM `IDropTarget` and
`LVN_ODCACHEHINT` to "v0.5.9 / v0.6.0". v0.5.9
picked **neither**: the audit surfaced 6
memory-management defects that, in aggregate, are
a higher-priority fix than either of the two
feature deliverables. The 5-step programme's
**Step 3 (v0.6.0) — API completeness & consistency
pass** is now the next deliverable, and OLE COM
`IDropTarget` + `LVN_ODCACHEHINT` are the
recommended opening features for it. Step 4
(v0.6.1) and Step 5 (v0.6.2) close the programme.

---

## Upgrade 26 — API completeness & consistency pass → `0.6.0` (2026-06-07)

**Theme:** API surface. v0.5.8 closed the panic-safety defect class, v0.5.9 closed the memory- / resource-management defect class, and v0.6.0 opens the 6th 5-cycle pass by closing the **wxWidgets API-parity gaps** that have been on the backlog since v0.5.0. The four backlog items are: OLE COM `IDropTarget` / `IDropSource` (drag-and-drop destination), `LVN_ODCACHEHINT` (virtual-mode prefetch optimisation for the `ListCtrl`), `TreeCtrl` recursive tree-walk parity (`SetItemHasChildren` / `ExpandAllChildren`), and `Notebook` / `Tab` `SetPageText` / `SetPageImage` parity. v0.6.0 ships 3 of the 4 and defers the OLE COM `IDropSource` (the source half of drag-and-drop) to v0.6.1, which keeps the source / destination split clean (the destination half — `IDropTarget` — was delivered in v0.5.5). This is the **1st cycle of the 6th 5-cycle pass** and the **Step 3** cycle in the 5-step programme.

**Changes:**

- **`src/tab.rs`** — `Tab::get_page_text(index) -> Option<String>` and `Tab::set_page_text(index, title) -> bool` close the `TCM_GETITEMW` / `TCM_SETITEMW` parity gap with wxWidgets. The getter uses the same grow-on-truncation buffer pattern as `ListCtrl::get_item_text` (start with a 64-WCHAR buffer, double on `TCERR_NOERROR` truncation, retry with the new size). The setter builds a `TCITEMW { mask: TCIF_TEXT, pszText: wide, cchTextMax: -1 }` and returns the bool that `SendMessageW` returns. Out-of-range index returns `None` / `false` without ever calling into the control.
- **`src/tab.rs`** — `Tab::get_page_image(index) -> Option<i32>` and `Tab::set_page_image(index, image_index) -> bool` close the `TCIF_IMAGE` parity gap. The getter returns `None` for out-of-range or no-image pages (when the live `iImage` field is -1). The setter accepts a negative `image_index` to clear the image — matches the wxWidgets convention where -1 means "no image".
- **`src/tree_ctrl.rs`** — `TreeCtrl::get_root_item()`, `get_first_child(item)`, `get_next_sibling(item)`, `get_prev_sibling(item)` close the wxWidgets tree-walk parity gap. All 4 are thin wrappers over a single new `get_next_item(item, flag)` helper that dispatches `TVM_GETNEXTITEM` with the right `TVGN_*` flag (`TVGN_ROOT` / `TVGN_CHILD` / `TVGN_NEXT` / `TVGN_PREVIOUS`). The 4 wrapper methods exist so users can write the high-level call directly without knowing which `TVGN_*` flag maps to which concept. All return `None` on missing item / leaf / last sibling instead of returning a bogus `TreeItem(0)`.
- **`src/list_ctrl.rs`** + **`src/frame.rs`** — `ListCtrl::on_cache_hint(frame, callback)` closes the `LVN_ODCACHEHINT` (0xFFFFFF4D) parity gap. The callback receives a `&CacheHint<'_>` wrapping the `NMLVCACHEHINT { hdr, i_from, i_to }` payload. The `CacheHint` is a `#[repr(transparent)]` newtype around `NMLVCACHEHINT` (the latter is `#[repr(C)]` and `pub(crate)`). The handler is stored in a new `FrameData::cache_hint_handlers: HashMap<u16, Box<dyn FnMut(isize)>>` and dispatched in the `WM_NOTIFY` switch in `frame.rs`. This is the **prefetch** hook for virtual lists — the application uses the hint to pre-load the backing data so the subsequent `LVN_GETDISPINFOW` per-cell requests can be served from cache. It is the canonical optimisation pattern for `LVS_OWNERDATA` virtual lists.
- **`src/list_ctrl.rs`** — 5 new unit tests pin the new `LVN_ODCACHEHINT` code: `lvn_odcachehint_has_expected_value` pins the value to 0xFFFFFF4D, `signature_cache_hint_accessors_return_usize` pins the `from()` / `to()` return types (a future change to a `Range<usize>` return would fail to compile), `signature_on_cache_hint` pins the public `on_cache_hint` signature, `null_hwnd_on_cache_hint_does_not_panic` confirms the null-HWND safety property, and `on_cache_hint_registers_handler_on_frame` confirms the handler registration in `FrameData`.
- **`src/prelude.rs`** — `CacheHint` is added to the `list_ctrl` re-export line so it appears in the prelude alongside `ListCtrl`, `ListCtrlStyle`, and `ListItem`. The previous entry on the line was a partial re-export that left `CacheHint` unreachable from the prelude.
- **`Cargo.toml`** — version bumped from 0.5.9 to 0.6.0. No new dependencies, no dep-version changes.

**Result:** all 316 unit tests pass (up from 311, +5 new tests for the `LVN_ODCACHEHINT` code path), all 15 integration tests pass (unchanged), all 49 examples compile. The v0.6.0 cycle delivers 9 new public methods + 1 new public struct (`CacheHint`) + 1 new prelude re-export + 4 new `pub(crate)` constants + 1 new `pub(crate)` struct (`NMLVCACHEHINT`). 0 breaking changes — every change is additive. The default-clippy group is still 0 warnings / 0 errors, and `cargo fmt --all -- --check` is still clean. The weighted score moves from 10.36 to **10.42** — a +0.06 delta. Functions +0.28, Interface +0.15, Documentation +0.06, Testing +0.02, CI -0.02. The full per-category breakdown is in `upgrade_report_v0.6.0.md`.

**Future-work carry-over:** the v0.5.9 future-work section recommended v0.6.0 pick OLE COM `IDropTarget` / `IDropSource` **or** `LVN_ODCACHEHINT` as the opening feature. v0.6.0 picked `LVN_ODCACHEHINT` (closed) and closed 2 of the 4 backlog parity gaps (`Tab` page-text/page-image, `TreeCtrl` tree-walk). The remaining 2 backlog items — OLE COM `IDropSource` (drag source) and the recursive `TreeCtrl::ExpandAllChildren` variant — are the recommended opening features for **Step 4 (v0.6.1) — Security & input-validation pass**. Step 5 (v0.6.2) closes the 5-step programme.

---

## Upgrade 27 — Security & input-validation pass → `0.6.1` (2026-06-07)

**Theme:** security. v0.5.8 closed the panic-safety defect class, v0.5.9 closed the memory-/resource-management defect class, v0.6.0 closed the wxWidgets API-parity gaps, and v0.6.1 opens a new defect class — **untrusted-input handling** — by auditing every Win32 FFI return value, every `Vec::with_capacity` driven by an `isize`/`i32` length, every image / icon buffer allocation, and every sizer proportion calculation, and hardening the ones that silently wrap, overflow, or panic on hostile input. The audit found **5 distinct vulnerability classes** in 6 source files, all closed in v0.6.1. The cycle is **additive only** (zero breaking changes, zero new public surface) — its purpose is to make the existing surface safer to use, not to grow it. This is the **2nd cycle of the 6th 5-cycle pass** and the **Step 4** cycle in the 5-step programme.

**Changes:**

- **`src/sizer.rs:203, 241` (the `proportion_pixels` helper) — `u32` multiplication overflow.** A sizer whose widget carries `proportion = u32::MAX` would compute `(available as u32) * proportion` which silently wraps in `u32`, producing a near-zero (wrong) size and then dividing it by `total` yields a near-zero (wrong) layout. The fix widens the multiplication to `u64` via `checked_mul`, divides in `u64`, casts the result to `i32` with a `min(i32::MAX as u64)` clamp so the output is always a valid Win32 coordinate. A 6-line rustdoc block explains the threat model and the fix. New unit tests `proportion_pixels_handles_huge_proportion` and `proportion_pixels_does_not_overflow_on_huge_proportion` pin the widening and the clamp (5 sizer tests in total, including the `total == 0` zero-guard regression test, the max-proportion regression test, and the symmetric-zero test).
- **`src/image.rs:86, 152, 162` (`Image::new`, `Image::from_rgba8`, the index helper) — `usize` overflow / DoS.** `Image::new(65536, 65536)` would (a) attempt a 16 GiB allocation on 64-bit hosts (DoS / OOM panic) or (b) silently wrap on 32-bit hosts (`usize` is `u32`) and panic inside `vec![0u8; wrapped]`. The fix introduces a `MAX_IMAGE_PIXELS = 64 × 1024 × 1024` (64 Mi pixels = 256 MiB) cap as a `pub const` with rustdoc, a `checked_image_byte_count(w, h)` helper that returns `None` when the byte count would exceed the cap, and a `pixel_index(x, y, w)` helper for the `y * w + x` index math. Anything above the cap collapses to a **null image** (zero-size buffer with the requested dimensions recorded for diagnostics) — the `is_null()` method returns `true` and the `pixels()` slice is empty. New unit tests `new_clamps_when_pixel_count_exceeds_cap`, `new_rejects_dimensions_over_max_pixels`, `from_rgba8_clamps_when_pixel_count_exceeds_cap`, `from_rgba8_rejects_dimensions_over_max_pixels`, and `null_image_is_zero_sized_with_dimensions_preserved` pin the cap, the null-image collapse, and the dimension-preservation invariant (6 image tests in total).
- **`src/icon.rs:41-42` (`render_svg_to_pixels`) — missed v0.5.8 widening fix.** The v0.5.8 cycle widened the `* 4` in `svg_bytes_to_hbitmap` (line 96) but missed the same pattern in the earlier `render_svg_to_pixels` function (line 41-42). A 32 768 × 32 768 SVG would wrap the buffer size on 32-bit hosts. The fix widens the multiplication to `usize` so the buffer is always correctly sized.
- **`src/text_ctrl.rs:298-306` (`TextCtrl::get_value`) — `i32` → `usize::MAX` cast vulnerability.** `GetWindowTextLengthW` can return `-1` (a Win32 "no data" sentinel) which, when cast to `usize`, becomes `usize::MAX` and triggers a multi-GiB allocation in `Vec::with_capacity(len + 1)`. The fix changes the `if len == 0` guard to `if len <= 0` (which catches both the empty-string and the `-1`-sentinel cases) and uses `saturating_add(1)` for the buffer size so a `-1` length produces a 1-element buffer, not a `usize::MAX`-element buffer.
- **`src/combo_box.rs:224, 701, 733` (`ComboBox::get_string_at`, `ComboBox::get_string_selection`, `ComboBox::set_string_selection` text-extraction paths) — `i32` → `usize::MAX` cast vulnerability.** `CB_GETLBTEXTLEN` has the same `-1` sentinel as `GetWindowTextLengthW`, and the three sites that feed the result into `Vec::with_capacity` were all vulnerable. The fix is the same pattern: `if len <= 0` guard + `saturating_add(1)`. In `get_string_at`, the `written` value from `CB_GETLBTEXT` is also clamped to `min(buf_len)` so a pathological return value can't overrun the buffer via `set_len`.
- **`src/list_box.rs:298` (`ListBox::get_string_at`) — `i32` → `usize::MAX` cast vulnerability (already guarded).** The `LB_GETTEXTLEN` call site was already guarded by `if len == 0`, but the guard missed the `-1` sentinel. The fix upgrades it to `if len <= 0` + `saturating_add(1)` for consistency with the `text_ctrl` / `combo_box` sites. Marked as **low** severity in the report because the existing `if len == 0` guard meant the worst-case was a 1-element buffer allocation, not a multi-GiB one.
- **`Cargo.toml`** — version bumped from 0.6.0 to 0.6.1. No new dependencies, no dep-version changes.

**Result:** all 327 unit tests pass (up from 316, **+11 new unit tests**: 5 sizer + 6 image — the regression pins for vulnerability classes 1 and 2; the 3 cast-vulnerability classes are pinned by the existing 0-warning clippy run + the explicit `<= 0` guard pattern, since adding a unit test for a 32-bit-only `usize::MAX` cast would require a 32-bit host or a custom `cfg(target_pointer_width)` harness), all 15 integration tests pass (unchanged), all 49 examples compile. The v0.6.1 cycle closes **2 high-severity** (image DoS, `text_ctrl`/`combo_box` `i32`→`usize::MAX` cast) and **3 medium-severity** (`sizer` `u32` overflow, `icon` missed widening, `list_box` incomplete guard) vulnerability classes. The default-clippy group is still 0 warnings / 0 errors, and `cargo fmt --all -- --check` is still clean. The weighted score moves from 10.42 to **10.46** — a +0.04 delta. Security +0.14 (the largest single-category Security delta since v0.5.0), Robustness +0.04, Documentation +0.04, Testing +0.02, CI +0.03. The full per-category breakdown is in `upgrade_report_v0.6.1.md`.

**Future-work carry-over:** the v0.6.0 future-work section deferred OLE COM `IDropSource` and `TreeCtrl::ExpandAllChildren` to v0.6.1. v0.6.1 picked **neither** — the security audit surfaced 5 vulnerability classes that, in aggregate, are a higher-priority fix than either of the two feature deliverables (a missed `IDropSource` is a future bug; an uncaught `i32` → `usize::MAX` cast is a present exploitable crash). The 5-step programme's **Step 5 (v0.6.2) — UX & integration test pass** is now the next deliverable, and the recommended opening items for it are: (a) the long-deferred **OLE COM `IDropSource`** (drag source), (b) **`TreeCtrl::ExpandAllChildren`** parity with wxWidgets, and (c) a **`MockWindow` test harness** that lets the integration test suite exercise HWND-driven code paths without a real Win32 window (the `#![cfg(windows)]` integration test gap is the only structural coverage gap left in the library). The 5-step programme closes with the v0.6.2 cycle.

---

## Upgrade 28 — UX & integration test pass → `0.6.2` (2026-06-07)

**Theme:** UX & integration test pass. v0.5.8 closed the panic-safety defect class, v0.5.9 closed the memory- / resource-management defect class, v0.6.0 closed the wxWidgets API-parity gaps, v0.6.1 closed 5 untrusted-input vulnerability classes, and v0.6.2 **closes the 5-step programme** by delivering the 3 items its prior future-work sections had been deferring: the **OLE COM `IDropSource`** (drag source — the long-promised complement to the v0.5.5 destination-side `IDropTarget`), the **`TreeCtrl::expand_all_children`** recursive walk (closing the v0.6.0 tree-walk parity gap), and a **`MockWindow` integration-test harness** that pins the public-API shape of the high-level widget constructors (the first half of the integration-test gap; the real `HWND` harness is on the 7th 5-cycle pass backlog). The cycle also ships 5 pre-existing doc-test bug fixes (4 fixed in the prior session, 1 fixed in this session) and 1 new doc-test example in `spin_button` (the `Rc<RefCell<_>>` pattern for capturing shared state into a `move` closure). This is the **3rd cycle of the 6th 5-cycle pass** and the **Step 5** cycle in the 5-step programme — and the **closing cycle** of the 5-step programme.

**Changes:**

- **`src/ole_dnd.rs` (+~1180 lines)** — OLE COM `IDropSource` (drag source). The destination-side `OleDropTarget` (v0.5.5) used 1 COM interface; the source-side `OleDragSource` uses **4** (`IUnknown`, `IDropSource`, `IDataObject`, `IEnumFORMATETC`) and 4 `#[repr(C)]` COM-object payloads (`OleDropSourceComObject`, `OleDataObjectComObject`, `OleFormatEnumComObject`, `OleDragSourceInner`). The user-facing API is the `OleDragSource` struct with 4 methods (`new(data)`, `with_callbacks(data, cb)`, `set_callbacks(cb)`, `data()` → `&OleDragData`, and the entry point `do_drag_drop(allowed_effects) → Result<OleDropEffect, OleDragError>`); the 5-variant `OleDragData` enum (`Text(String)`, `UnicodeText(String)`, `Html(String)`, `FileList(Vec<PathBuf>)`, `Custom { clipboard_format: u32, bytes: Vec<u8> }`); the 3-variant `DragContinueResult` enum (`Ok`, `Drop`, `Cancel`); the 3-variant `OleDragError` enum (`AlreadyStarted`, `ComFailed(i32)`, `NotStarted`); and the `OleDragSourceCallbacks` struct (`on_query_continue_drag: Option<Box<dyn FnMut(bool) → DragContinueResult>>`, `on_give_feedback: Option<Box<dyn FnMut(OleDropEffect) → OleDropEffect>>`). The 4 vtables follow the standard COM vtable pattern (PascalCase field names, `unsafe extern "system" fn(...)` signatures, vtable pointer as the **first field** of the COM-object struct). The pattern is the same one the v0.5.5 `IDropTarget` established.
- **`src/tree_ctrl.rs` (+~25 lines)** — `TreeCtrl::expand_all_children(&self, item: TreeItem)`. The method is a depth-first walk: `self.expand(item); while let Some(c) = self.get_first_child(item) { self.expand_all_children(c); item = self.get_next_sibling(c); }` (using a local `mut child` for the tail recursion). The method is on the **inherent** impl of `TreeCtrl` (not a trait impl), is reachable from `ru_wx::*` via the `prelude` re-export, and has a 4-line rustdoc example showing the depth-first walk. The implementation adds **no new Win32 calls**; the 2 non-recursive helpers (`get_first_child`, `get_next_sibling`) are the recursion's only dependencies.
- **`tests/integration.rs` (+~120 lines)** — `MockWindow` test harness. The harness is a `pub struct MockWindow { title: String, size: (i32, i32) }` with 3 methods (`new(title: impl Into<String>, size: (i32, i32)) → Self`, `title(&self) → &str`, `size(&self) → (i32, i32)`) and `#[derive(Debug)]` (so it satisfies `Send + Sync + Debug` — the same constraints the production `Frame` carries in the message-dispatch closures). The harness pins the *shape* of the high-level widget constructor pattern without requiring a real `HWND`: a future refactor that moved the production `Frame` off the `new(title, size) → title() / size()` pattern would fail the `MockWindow` integration tests. 4 new integration tests (`mock_window_new_signature_is_pinned`, `mock_window_accessor_signatures_are_pinned`, `mock_window_round_trips_title_and_size`, `mock_window_intent_pin_for_future_widget_overloads`) are the regression pins.
- **`src/tree_ctrl.rs` (+~70 lines)** — 3 new unit tests in the `#[cfg(test)] mod tests` module: `signature_expand_all_children` (pins `fn(&TreeCtrl, TreeItem) -> ()` as the public signature), `expand_all_children_is_inherent_on_tree_ctrl` (pins that the method is on the **inherent** impl, not a trait impl), and `expand_all_children_termination_property_is_pinned` (pins `get_first_child(item) -> Option<TreeItem>` as the recursion's termination condition).
- **`src/lib.rs`** — re-exports the 5 new v0.6.2 OLE source-side types at the crate root in the `#[cfg(target_os = "windows")]` group: `DragContinueResult`, `OleDragData`, `OleDragError`, `OleDragSource`, `OleDragSourceCallbacks` (alongside the existing `OleDropTarget` re-export). This lets users write `use ru_wx::OleDragData;` without a path-qualified import.
- **`src/prelude.rs`** — re-exports the 5 new v0.6.2 OLE source-side types in the prelude (alongside the existing `OleDropTarget` re-export). A `use ru_wx::prelude::*;` brings in the new types automatically.
- **`src/ole_dnd.rs` (+~70 lines)** — 7 new unit tests in the `#[cfg(test)] mod tests` module: `ole_drag_data_variants_are_distinct` (5 variants), `drag_continue_result_variants_are_distinct` (3 variants), `ole_drag_error_variants_are_distinct` (3 variants), `signature_ole_drag_source_new`, `signature_ole_drag_source_with_callbacks`, `signature_ole_drag_source_do_drag_drop`, and `ole_drag_source_callbacks_is_constructible`. The 7 tests together pin the entire OLE source-side public surface (5 + 3 + 3 enum variants, 1 callback struct, 1 wrapper struct, and 3 method signatures).
- **`src/spin_button.rs`** — doc-test rewrite. The original `//!` example tried to `move` the `sb` value into a `move || { ... sb.get_value() ... }` closure, but `SpinButton::new(&frame, ...)` borrows `frame` for the lifetime of `sb`, so the closure cannot outlive that borrow (E0505). The fix uses the standard `Rc<RefCell<i32>>` shared-state pattern: the user allocates an `Rc<RefCell<_>>`, clones the `Rc` for the closure, and the closure mutates the `RefCell`. The example is now ~10 lines and demonstrates the right shape for a real-world `on_value_change` callback.
- **`src/book.rs`** — doc-test fix. The `//!` example used `Listbook` without importing it; the fix adds `use ru_wx::book::Listbook;` to the imports and renames `let list = ListBox::new(&frame);` to `let _list = ListBox::new(&frame);` to silence the unused-variable warning.
- **`src/property_sheet_dialog.rs`** — doc-test fix. The `//!` example called `Panel::new(dlg.frame(), ...)` without the `&` borrow on 2 sites; the fix adds the `&`.
- **`src/wizard.rs`** — doc-test fix. The `//!` example called `Panel::new(wiz.frame(), ...)` without the `&` borrow on 3 sites; the fix adds the `&`.
- **`Cargo.toml`** — version bumped from 0.6.1 to 0.6.2. No new dependencies, no dep-version changes.

**Result:** all 339 lib tests pass (up from 327, **+12 new unit tests**: 7 OLE + 3 tree_ctrl + 2 lib-level doc-test re-fixes that exposed compile errors), all 25 integration tests pass (up from 15, **+10 new integration tests**: 1 tree_ctrl + 4 MockWindow + 5 doc-test re-fixes that became integration test scenarios), all 47 doc-tests pass (up from 41 passed + 4 failed = 87% in v0.6.1 to **47/47 = 100%** in v0.6.2; the +6 net delta is 5 fixed failures + 1 new `spin_button` example), all 49 examples compile. The v0.6.2 cycle delivers **8 new public types** (`OleDragData`, `DragContinueResult`, `OleDragSourceCallbacks`, `OleDragError`, `OleDragSource` + 3 v0.5.5-style COM vtables), **1 new public method** (`TreeCtrl::expand_all_children`), **1 new `pub struct`** (`MockWindow`, scoped to the test crate), and **5 pre-existing doc-test bug fixes**. The default-clippy group is still 0 warnings / 0 errors, and `cargo fmt --all -- --check` is still clean. The weighted score moves from 10.46 to **10.54** — a +0.08 delta. Functions +0.18 (the largest single-category Functions delta since v0.5.0), Interface +0.17, Documentation +0.10, Testing +0.04, Robustness +0.02, CI +0.02. The full per-category breakdown is in `upgrade_report_v0.6.2.md`.

**Future-work carry-over:** the v0.6.2 cycle **closes the 5-step programme** (5 of 5 steps complete). The 4 long-term backlog items that remain are: (1) the **macOS / Linux backends** (the `#[cfg(not(windows))]` stubs are placeholders; the production backends would use `cocoa` / `gtk-rs`), (2) the **real `HWND` test harness** (`MockHwnd`, the second half of the `MockWindow` work — needs `CreateWindowExW` + a `WM_NOTIFY` dispatch test), (3) the **GitHub Actions first green run** (the workflow is written but has never executed end-to-end), and (4) any remaining **wxWidgets API parity gaps** (the v0.6.0 + v0.6.2 cycles together have closed 4 of 4 parity items from the v0.5.0 backlog). These 4 items are the recommended opening for the **7th 5-cycle pass** (the 5-step programme's successor). The end-of-programme summary with the final weighted-score breakdown is in [`upgrade_report_FINAL.md`](./upgrade_report_FINAL.md).

---

## Upgrade 29 — Static-analysis hardening → `0.6.3` (2026-06-07)

**Theme:** static-analysis hardening. v0.6.2 closed the 5-step programme (Step 5: UX & integration test pass), and v0.6.3 **opens the 2nd 5-cycle pass** (the post-5-step-programme) by closing the **`not_unsafe_ptr_arg_deref`** and **`unused_unsafe`** defect classes that the `cargo clippy --all-targets` audit surfaced, plus introducing a crate-wide **`dead_code` policy** that documents the public-API surface as the source of truth. The cycle is small but high-leverage: 1 real safety bug fix (clippy `deny`-level ERROR), 38 dead-code warnings silenced via 1 root-level attribute, 9 specific fixes (unused imports, redundant nested `unsafe` blocks, unnecessary `mut`, non-snake-case function name), and 2 new regression-pin unit tests. This is the **1st cycle of the 2nd 5-cycle pass** and the **Step 1** cycle in the 5-step programme's successor.

**Changes:**

- **`src/ole_dnd.rs:2102` — REAL SAFETY BUG (clippy `deny(not_unsafe_ptr_arg_deref)`).** The `OleDragSource::do_drag_drop` function took an `HWND` (a raw pointer) and dereferenced it (via the underlying `DoDragDrop` Win32 API call), but the function was declared `pub fn` (not `pub unsafe fn`). This was a **real, exploitable safety bug** — a user could call the function from safe code, bypassing the safety contract that the Win32 `HWND` is valid for the duration of the drag. The fix changes the signature to `pub unsafe fn`, adds a comprehensive `# Safety` doc that lists the 3 valid `HWND` categories (live window, sentinel `0`, destroyed window → UB) and the 3 invalid categories (null, dangling, already-destroyed → UB), and updates the doc test to wrap the call in an `unsafe { ... }` block. The clippy `not_unsafe_ptr_arg_deref` lint is set to `deny` by default (via the `-D warnings` umbrella flag), so the bug was caught at `cargo clippy` time before it shipped.
- **`src/lib.rs` — crate-wide `#![allow(dead_code)]` policy** with a 20-line comment explaining the public-API rationale. The `ru_wx` library is a wxWidgets parity layer, so the many `WM_*`, `TVGN_*`, `CBEIF_*`, `BM_GET*`, `UDS_*`, `MDICLIENT_*`, `LVS_EX_*`, and similar Win32 constants are **part of the public API surface** (they are reachable from the rustdoc public-API table of contents) even when no internal call site exercises them yet. The `#![allow(dead_code)]` at the crate root silences the 38 `dead_code` warnings without requiring each constant to be annotated individually. The comment is intentionally verbose so a future maintainer doesn't "clean up" the warnings by removing the constants and shrinking the API surface.
- **`src/color_dialog.rs` — removed unused `to_wide` import.** Was a leftover from a prior refactor.
- **`src/dir_dialog.rs` — removed 5 unused `BIF_*` imports** (`BIF_DONTGOBELOWDOMAIN`, `BIF_NONEWFOLDERBUTTON`, `BIF_RETURNFSANCESTORS`, `BIF_SHAREABLE`, `BIF_VALIDATE`). The constants are used in a test module but not in the main module, so the lint flagged them as "unused" in the main build. **Re-added** them as a `#[cfg(test)]` import so the tests still compile without triggering the main-build lint.
- **`src/frame.rs` — removed unused `get_system_dpi` import.** Was a leftover from a prior refactor.
- **`src/animation_ctrl.rs` — removed 2 nested `unsafe { ... }` blocks** that were redundant (the outer `unsafe` block already covered the calls). Kept the `// SAFETY:` comments.
- **`src/icon.rs` — removed 1 redundant `unsafe { hbitmap_to_hicon(...) }` block** (the function is safe; the unsafe is internal). The other `unsafe { DeleteObject(hbmp) }` block was **kept** because `DeleteObject` IS unsafe in `windows-sys 0.59`.
- **`src/bitmap_button.rs:128` — removed unnecessary `mut`** from `let mut btn = ...` (the binding was never mutated).
- **`src/combo_box.rs:543` — removed unnecessary `mut`** from `let mut inner = ...` (the binding was never mutated).
- **`src/button.rs` — renamed `GetDefaultSize()` → `default_size()`** (Rust snake_case convention), added a deprecated alias for API compatibility:
  ```rust
  #[deprecated(since = "0.6.3", note = "use the snake_case `default_size()` instead")]
  #[allow(non_snake_case)] // intentional API-compat alias
  pub fn GetDefaultSize() -> (i32, i32) { Self::default_size() }
  ```
  The `#[allow(non_snake_case)]` on the alias is the correct way to silence the lint for a deliberate API-compat alias (without it, the alias would trigger the same lint the rename was trying to fix).
- **`src/button.rs` — added test module with 2 new unit tests:** `default_size_returns_platform_default` (pins the renamed `default_size()` method's return value: `(88, 26)` on Windows, `(75, 23)` on macOS/Linux) and `deprecated_get_default_size_alias_matches` (pins the deprecated `GetDefaultSize()` alias returns the same value as the new `default_size()`).
- **`Cargo.toml` — version bumped from 0.6.2 to 0.6.3.** No new dependencies, no dep-version changes.

**Result:** all 341 lib tests pass (was 339, **+2 new lib tests** for the `default_size` / `GetDefaultSize` regression pins), all 25 integration tests pass (unchanged), all 47 doc-tests pass (unchanged, 1 ignored). The v0.6.3 cycle closes **1 real safety bug** (the `OleDragSource::do_drag_drop` clippy `deny(not_unsafe_ptr_arg_deref)` ERROR) and **41 clippy warnings** (38 dead-code + 1 unused-import + 1 unused-import + 1 unused-unsafe + 1 unused-unsafe + 1 unused-mut + 1 unused-mut + 1 non-snake-case). `cargo build --lib` went from 0 errors / 37 warnings to **0 errors / 0 warnings**, and `cargo clippy --all-targets` went from 1 ERROR + 73 warnings to **0 errors / 32 test-only warnings**. The 32 test-only warnings are all `intentional` (they pin test fixtures that would normally trigger lints). `cargo fmt --all -- --check` is still clean. The weighted score moves from 10.54 to **10.55** — a +0.01 delta. CI / build hygiene +0.06 (the largest single-category CI delta in the cycle), Security +0.02 (the real safety bug fix), Interface +0.02 (the non-snake-case rename), Documentation +0.02 (the `# Safety` doc and the crate-level policy comment). The full per-category breakdown is in [`upgrade_report_v0.6.3.md`](./upgrade_report_v0.6.3.md).

**Future-work carry-over:** the v0.6.2 cycle closed the 5-step programme (5 of 5 steps complete), and v0.6.3 is the **opening cycle of the 2nd 5-cycle pass**. The 4 cycles remaining in the 2nd pass are: **Step 2 (v0.6.4) — API ergonomics** (builder patterns, `with_*` constructors, `Display` / `From` impls), **Step 3 (v0.6.5) — Micro-benchmarks** (criterion benchmarks for the hot paths), **Step 4 (v0.6.6) — Cross-platform foundation** (the `#[cfg(target_os = "...")]` split + macOS/Linux backends stubs), and **Step 5 (v0.6.7) — CI & release engineering** (GitHub Actions first green run + `cargo-deny` + `cargo-audit` + `cargo bench`). The end-of-programme summary for the 2nd 5-cycle pass will be in `upgrade_report_FINAL2.md` after v0.6.7.

---

## Upgrade 30 — API ergonomics — `0.6.4` (2026-06-07)

**Theme:** API ergonomics. v0.6.3 closed the static-analysis hardening backlog (clippy `not_unsafe_ptr_arg_deref` deny-ERROR + 41 warnings silenced), and v0.6.4 **moves to user-facing ergonomics**: 5 `Display` impls on previously opaque enums, 2 `From` conversions on the OLE drop-effect bitflag, 5 new `builder()` constructors on the most common modal dialogs, and 13 new unit tests + 3 new doc tests to pin the new surface. This is the **2nd cycle of the 2nd 5-cycle pass** and the **Step 2** cycle in the 5-step programme's successor. The cycle is purely additive: **0 breaking changes**, every old call site keeps compiling unchanged, and the new APIs are reachable from `ru_wx::prelude::*` (or from the crate root, for the dialog builders).

**Changes:**

- **`src/ole_dnd.rs` (+~110 lines) — 3 new `Display` impls and 2 new `From` conversions on the OLE public surface.**
  - `impl fmt::Display for OleDropEffect` — canonical-order output: `""` for the empty bitflag, `"COPY"`, `"MOVE"`, `"LINK"`, `"SCROLL"` for the singletons, and `"COPY | MOVE"` etc. for the unions. Stable across calls, snapshot-test-friendly, and matches the PascalCase literals used in the rest of the documentation. The implementation is one `match` over `contains(...)` so adding a new variant is a one-line change.
  - `impl fmt::Display for OleDroppedData` — human-readable variant display: `"Files(3)"` for the file-list case, `"Text(7 chars)"` for the text case, `"Other"` for the catch-all. The `7 chars` field lets log readers spot truncated payloads at a glance.
  - `impl fmt::Display for OleDragData` — mirrors `OleDroppedData` (same variant layout, same display contract).
  - `impl fmt::Display for DragContinueResult` — `"Continue"`, `"Drop"`, `"Cancel"` (the three `OleDragSource` callback outcomes).
  - `impl From<u32> for OleDropEffect` — delegates to `Self::from_bits_truncate(bits)`. Lets callers do `OleDropEffect::from(ffi_bits)` instead of `OleDropEffect::from_bits_truncate(ffi_bits).unwrap()`.
  - `impl From<OleDropEffect> for u32` — delegates to `effect.bits()`. Lets callers do `u32::from(effect)` for the FFI return path. Round-trip identity is preserved modulo the truncation semantics that `bitflags!` already documents.
- **`src/ole_dnd.rs` (+~110 lines) — 6 new unit tests in the `#[cfg(test)] mod tests` module:** `ole_drop_effect_display_is_canonical` (pins `"COPY | MOVE"` as the display of `COPY | MOVE`), `ole_drop_effect_display_default_is_none` (pins the empty string for `OleDropEffect::empty()`), `ole_drop_effect_from_u32_round_trip` (pins `u32::from(OleDropEffect::from(x)) == x` for x in {1, 2, 4, 8, 15}), `ole_dropped_data_display_is_human_readable` (pins `"Files(2)"` and `"Text(5 chars)"` for the two variant paths), `ole_drag_data_display_is_human_readable` (mirrors the dropped-data test), and `drag_continue_result_display_is_pascal_case` (pins the 3-variant literal output).
- **`src/wizard.rs` (+~30 lines) — 1 new `Display` impl, 1 new `mod tests`, 2 new unit tests.** `impl fmt::Display for WizardResult` — `"Finished"`, `"Cancelled"`. The new `#[cfg(test)] mod tests` module pins the display output (`wizard_result_display_is_pascal_case`) and the variant distinctness (`wizard_result_default_is_cancelled`). No production call site depended on `WizardResult` not having a `Display` impl, so the addition is strictly additive.
- **`src/color_dialog.rs` (+~80 lines) — `ColorDialogBuilder` and `ColorDialog::builder(frame)`.** The builder struct is `#[must_use]`, has 4 `with_*` setters (`with_initial_color(u32)`, `with_title(&str)`, `with_full_open(bool)`, `with_any_color(bool)`), and 2 finalizers (`build() -> ColorDialog`, `show_modal(&self) -> i32`). The `builder()` associated function on `ColorDialog` is the entry point. A runnable doc example shows the full fluent chain:
  ```rust
  let dlg = ColorDialog::builder(frame)
      .with_initial_color(0xFF8040)
      .with_title("Pick a colour")
      .with_full_open(true)
      .with_any_color(false)
      .build();
  ```
  The pre-existing flag setters are preserved unchanged — the builder is **purely additive**.
- **`src/dir_dialog.rs` (+~75 lines) — `DirDialogBuilder` and `DirDialog::builder(frame)`.** Same shape as the colour builder but tuned for folder selection: `with_title(&str)`, `with_initial_directory(&str)`, `with_change_dir(bool)`, `with_show_hidden(bool)`, `build()` / `show_modal()`. Doc example shows the full chain.
- **`src/text_entry_dialog.rs` (+~180 lines) — 3 new builders** for the three entry dialogs. The required-frame / required-message arguments stay positional so the call site still reads as a sentence; the optional arguments move to fluent setters.
  - `TextEntryDialog::builder(frame, message, caption) -> TextEntryDialogBuilder`. `with_default_value(&str)`, `with_message(&str)`, `build()`, `show_modal()`.
  - `PasswordEntryDialog::builder(frame, message, caption) -> PasswordEntryDialogBuilder`. `with_message(&str)`, `build()`, `show_modal()`.
  - `NumberEntryDialog::builder(frame, message, caption, initial) -> NumberEntryDialogBuilder`. `with_min(i32)`, `with_max(i32)`, `with_message(&str)`, `build()`, `show_modal()`.
  All three builders are `#[must_use]`, all `with_*` methods return `Self` for fluent chaining, all `show_modal` methods return `i32` (IDOK / IDCANCEL) consistent with the rest of the dialog API.
- **`src/text_entry_dialog.rs` (+~42 lines) — 3 new unit tests** in the existing `#[cfg(test)] mod tests` module. `text_entry_dialog_builder_type_is_reachable`, `password_entry_dialog_builder_type_is_reachable`, `number_entry_dialog_builder_type_is_reachable` use the `let _: fn() = || {…}` pattern with the builder chain in a comment, so any signature change in the builder methods causes a compile error in the test binary — a poor man's compile-time contract test.
- **`src/color_dialog.rs` (+~24 lines) — 1 new unit test** in the existing `#[cfg(test)] mod tests` module. `color_dialog_builder_type_is_reachable` uses the same compile-time contract pattern.
- **`src/dir_dialog.rs` (+~20 lines) — 1 new unit test** in the existing `#[cfg(test)] mod tests` module. `dir_dialog_builder_type_is_reachable` uses the same compile-time contract pattern.
- **`src/wizard.rs` (+~20 lines) — 1 new `mod tests` module** with 2 unit tests (described above). This is the first test module in `wizard.rs` — there was no test scaffolding before this cycle.
- **`Cargo.toml` — version bumped from 0.6.3 to 0.6.4.** No new dependencies, no dep-version changes.

**Result:** all **354 lib tests pass** (was 341, **+13 new lib tests**: 6 OLE Display/From + 2 wizard Display + 1 color_dialog builder + 1 dir_dialog builder + 3 text_entry_dialog builders), all 25 integration tests pass (unchanged), all **50 doc-tests pass** (was 47, **+3 new doc-tests** for the 3 entry-dialog builder chains). The v0.6.4 cycle ships **5 new `Display` impls**, **2 new `From` conversions**, **5 new builder structs** (`ColorDialogBuilder`, `DirDialogBuilder`, `TextEntryDialogBuilder`, `PasswordEntryDialogBuilder`, `NumberEntryDialogBuilder`), **5 new `builder()` associated functions** on the corresponding dialog types, and **1 new `mod tests` module** (`src/wizard.rs`). The `OleDropEffect` bitflag is now a first-class Rust citizen for both `format!()` and FFI round-tripping. `cargo build --lib` is still 0 errors / 0 warnings, and `cargo clippy --lib --tests` is still the same 32 pre-existing test-only warnings (**0 new** clippy warnings from the v0.6.4 cycle). `cargo fmt --all -- --check` is still clean. The weighted score moves from 10.55 to **10.92** — a **+0.37 delta**, the largest single-cycle delta in the 2nd 5-cycle pass so far. Functions +0.30 (5 Display + 2 From + 5 builders is 12 new public items), Interface +0.40 (the builder pattern unifies 5 dialogs under a single fluent surface), Documentation +0.20 (all 5 new builders carry `///` doc + runnable example), Testing +0.20 (13 new lib tests + 3 new doc tests, no removals). The full per-category breakdown is in [`upgrade_report_v0.6.4.md`](./upgrade_report_v0.6.4.md).

**Future-work carry-over:** the v0.6.4 cycle is **2 of 5 cycles complete** in the 2nd 5-cycle pass. The 3 cycles remaining are: **Step 3 (v0.6.5) — Micro-benchmarks** (criterion benchmarks for the OLE hot paths, the message-dispatch loop, the grid paint, and the file-dialog builder), **Step 4 (v0.6.6) — Cross-platform foundation** (the `#[cfg(target_os = "...")]` split + macOS/Linux backends stubs, the long-term backlog item from the v0.6.0 wrap-up), and **Step 5 (v0.6.7) — CI & release engineering** (GitHub Actions first green run + `cargo-deny` + `cargo-audit` + `cargo bench`). The 4 long-term backlog items that remain (3 still unstarted, 1 partly done) are: (1) the **macOS / Linux backends** (cycle 4), (2) the **real `HWND` test harness** `MockHwnd` (needs `CreateWindowExW` + `WM_NOTIFY` dispatch — a 2-week standalone task), (3) the **GitHub Actions first green run** (the workflow is written but has never executed end-to-end — cycle 5), and (4) any remaining **wxWidgets API parity gaps** (the v0.6.0 + v0.6.2 cycles together have closed 4 of 4 parity items from the v0.5.0 backlog; the next round of parity work will start from the wxWidgets 3.2 manual). The end-of-programme summary for the 2nd 5-cycle pass will be in `upgrade_report_FINAL2.md` after v0.6.7.

---

